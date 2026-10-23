//! Importuesi i hyrjeve shqipe të Wiktionary-t anglisht (Wiktextract / kaikki.org).
//!
//! Lexon TSV-në kompakte që shkruan `nura_fetch --only kaikki_en` (formati është
//! përshkruar te `nura_fetch/src/src_kaikki_en.rs`):
//!
//! ```text
//! E ⇥ fjala ⇥ pos ⇥ gjinia ⇥ forma_kanonike ⇥ nr_etimologjie
//! F ⇥ forma ⇥ etiketat ⇥ burimi_i_formës
//! G ⇥ n ⇥ etiketat ⇥ glosa_angleze
//! O ⇥ n ⇥ form_of|alt_of ⇥ objektivi ⇥ etiketat
//! R ⇥ n ⇥ REL ⇥ fjala ⇥ etiketat ⇥ lang_code ⇥ shënim_anglisht
//! ```
//!
//! Çfarë krijohet:
//! - nyje `Word(sq, lemë)`: lema me shkronja të vogla; te mbiemrat pa nyjën
//!   (`bukur` + `article=i` te `Rel::Lex`);
//! - `Rel::Lex` (vetë-lak): pos, gender, article, trans, med, flags;
//! - `MorphEntry` për çdo formë të lakuar me tipare UD të nxjerra nga etiketat e
//!   Wiktextract (vetëm etiketat e njohura; forma me etiketë të panjohur nuk merret);
//! - `Rel::Sem` (SYNONYM, ANTONYM, HYPERNYM, HYPONYM, DERIVED_FROM, RELATED,
//!   COORDINATE, FORM_OF) VETËM kur skaji tjetër është fjalë shqipe (hyrje e kaikki
//!   ose nyje `Word(sq)` ekzistuese);
//! - glosat ANGLISHT si nyje `Sense{source: KaikkiEn, synset: "kaikki:<fjala>:<pos>:<n>"}`
//!   me `Text{lang: En}` dhe skaj `HasSense` — kurrë si tekst shqip.
//!
//! Çfarë NUK merret (rregulla të deklaruara):
//! - qelizat mbushëse të tabelave si forma (`is_word_form`): «-», «Term?», «{{{1}}}a»,
//!   «i/e/të/së», «(për)kula», «sg.», fjalët angleze të kokave («form», «indefinite» …),
//!   dhe çdo qelizë e një tabele që Wiktextract s'e lexoi (qeliza pa etiketa);
//! - glosat «<etiketa gramatikore> of <fjala>» pa `form_of` të strukturuar nuk bëhen
//!   kuptime angleze: bëhen FORM_OF (`via=gloss`);
//! - marrëdhëniet e kuptimeve jo-leksikore (shkurtim/alt-of/form-of) — ato i përkasin
//!   fjalës objektiv, jo homografit (p.sh. «nën» 'under' ≠ «nën» = «nëntor»);
//! - hyrjet në shkrim jo-latin (arvanitisht me germa greke).
//!
//! Atributi `sense` te `Rel::Sem` emërton kuptimin e hyrjes ku Wiktionary e liston
//! marrëdhënien; te DERIVED_FROM (e prejardhura → baza) ky është kuptimi i BAZËS.
//!
//! Stil: pa if/else — vetëm match.

use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::{Morph, MorphEntry};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const TSV: &str = "raw/kaikki_en/kaikki_en-sq.tsv";

const SRC_ATTR: &str = "wiktionary_en";

#[derive(Debug, Default, Clone)]
struct Entry {
    word: String,
    lemma: String,
    pos: String,
    gender: String,
    canonical: String,
    forms: Vec<(String, String, String)>,
    glosses: Vec<(usize, String, String)>,
    links: Vec<(usize, String, String, String)>,
    rels: Vec<(usize, String, String, String, String)>,
}

fn split_tags(t: &str) -> Vec<&str> {
    t.split(',').map(str::trim).filter(|x| !x.is_empty()).collect()
}

/// Heq nyjën e mbiemrit/emrit nga fillimi i një vargu me shumë fjalë.
fn strip_article(s: &str) -> (&str, Option<&'static str>) {
    match (s.strip_prefix("i "), s.strip_prefix("e "), s.strip_prefix("të "), s.strip_prefix("së ")) {
        (Some(r), _, _, _) => (r, Some("i")),
        (_, Some(r), _, _) => (r, Some("e")),
        (_, _, Some(r), _) => (r, Some("të")),
        (_, _, _, Some(r)) => (r, Some("së")),
        _ => (s, None),
    }
}

/// Wiktionary shënon theksin me akut/grav te koka e fjalës (p.sh. «áti», «praními»);
/// drejtshkrimi shqip nuk i ka këto shenja. Rregull i deklaruar: hiqen vetëm
/// theksat (á é í ó ú ý à è ì ò ù dhe U+0301/U+0300); ë dhe ç mbeten.
pub fn destress(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{0301}' | '\u{0300}'))
        .map(|c| match c {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            'ý' => 'y',
            'Á' | 'À' => 'A',
            'É' | 'È' => 'E',
            'Í' | 'Ì' => 'I',
            'Ó' | 'Ò' => 'O',
            'Ú' | 'Ù' => 'U',
            'Ý' => 'Y',
            o => o,
        })
        .collect()
}

/// Shkrim jo-latin (greqisht, cirilik): p.sh. hyrjet arvanitase «αρbε̰ρ». Janë shqip,
/// por jo në drejtshkrimin standard; nuk bëhen nyje `Word(sq)` dhe as forma.
pub fn non_latin(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}' | '\u{0400}'..='\u{04FF}'))
}

/// A është vargu një formë fjale e vërtetë? Tabelat e Wiktionary-t kanë qeliza
/// mbushëse që NUK janë fjalë shqipe: «-»/«—» (veta që s'ekziston, p.sh. urdhërorja
/// veta 1/3), «Term?» (kokë tabele), mbetje shablloni «{{{1}}}a», «i/e/të/së»,
/// «(për)kula», «(a)tij», «sg.», «pl.», «Eh!», «*kantkë», «2». Rregulli: duhet të ketë
/// shkronjë, asnjë shifër, asnjë nga `? / ( ) [ ] { } ! . * = < > | , ; : "`,
/// të mos nisë/mbarojë me vizë dhe të jetë në shkrim latin.
pub fn is_word_form(s: &str) -> bool {
    let t = s.trim();
    let has_letter = t.chars().any(char::is_alphabetic);
    let bad_char = t.chars().any(|c| {
        c.is_ascii_digit() || matches!(c, '?' | '/' | '(' | ')' | '[' | ']' | '{' | '}' | '!' | '.' | '*' | '=' | '<' | '>' | '|' | ',' | ';' | ':' | '"')
    });
    let dash_edge = t.starts_with(['-', '‐', '–', '—']) || t.ends_with(['-', '‐', '–', '—']);
    let header = HEADER_WORDS.contains(&t.to_lowercase().as_str());
    has_letter && !bad_char && !dash_edge && !header && !non_latin(t)
}

/// Fjalë angleze kokash tabele që Wiktextract i nxjerr ndonjëherë si «forma»
/// (p.sh. «Teuta» → «form», «don» → «forms»). Asnjëra s'është fjalë shqipe
/// («male» = shumësi i «mal» NUK është këtu).
const HEADER_WORDS: [&str; 16] = [
    "form", "forms", "definite", "indefinite", "singular", "plural", "nominative", "accusative", "genitive", "dative",
    "ablative", "vocative", "masculine", "feminine", "neuter", "female",
];

/// Fjalët gramatikore angleze që shfaqen në glosat «<etiketa> of <fjala>».
const GRAMMAR_WORDS: [&str; 39] = [
    "definite", "indefinite", "nominative", "accusative", "genitive", "dative", "ablative", "vocative", "singular",
    "plural", "and", "first-person", "second-person", "third-person", "simple", "perfect", "imperfect", "present",
    "past", "future", "indicative", "imperative", "subjunctive", "optative", "admirative", "conditional", "aorist",
    "participle", "gerund", "infinitive", "active", "passive", "mediopassive", "nonactive", "non-active", "masculine",
    "feminine", "neuter", "jussive",
];

/// Glosë që përshkruan vetëm një formë gramatikore pa fushë `form_of` të strukturuar,
/// p.sh. «definite nominative singular and indefinite dative/ablative singular of kalë»
/// ose «Indefinite plural of ka (“ox”).». Kthen (fjala_objektiv, etiketat_si_në_glosë).
/// Glosat me kuptim pas objektivit («participle of vdes: dead») NUK përputhen.
pub fn gloss_form_of(gloss: &str) -> Option<(String, String)> {
    let g = gloss.trim();
    let i = g.rfind(" of ")?;
    let (pre, rest) = (&g[..i], g[i + 4..].trim());
    let words: Vec<String> = pre.split([' ', '/']).filter(|w| !w.is_empty()).map(str::to_lowercase).collect();
    let all_grammar = !words.is_empty() && words.iter().all(|w| GRAMMAR_WORDS.contains(&w.as_str())) && words.iter().any(|w| w != "and");
    let mut parts = rest.splitn(2, ' ');
    let target = parts.next().unwrap_or("").trim_end_matches(['.', ',', ';']).to_string();
    let tail = parts.next().unwrap_or("").trim().trim_end_matches('.').trim();
    // Pas objektivit lejohet vetëm një përkthim në kllapa: «(“ox”)».
    let tail_ok = tail.is_empty() || (tail.starts_with('(') && tail.ends_with(')') && !tail.contains(':'));
    match (all_grammar, tail_ok, target.is_empty()) {
        (true, true, false) => Some((target, pre.trim().to_string())),
        _ => None,
    }
}

fn lemma_of(word: &str, pos: &str) -> String {
    let w = destress(word.trim()).to_lowercase();
    match pos {
        "adj" => strip_article(&w).0.to_string(),
        _ => w,
    }
}

/// Morfemat (prapashtesa, parashtesa, shkronja) nuk janë fjalë.
fn is_morpheme(e: &Entry) -> bool {
    matches!(e.pos.as_str(), "suffix" | "prefix" | "interfix" | "character" | "infix" | "circumfix")
        || e.word.starts_with('-')
        || e.word.ends_with('-')
        || e.lemma.is_empty()
}

fn pos_of(p: &str) -> Option<Pos> {
    match p {
        "noun" | "name" => Some(Pos::Noun),
        "verb" => Some(Pos::Verb),
        "adj" => Some(Pos::Adj),
        "adv" => Some(Pos::Adv),
        _ => None,
    }
}

fn upos_of(p: &str) -> Option<&'static str> {
    match p {
        "noun" => Some("NOUN"),
        "name" => Some("PROPN"),
        "verb" => Some("VERB"),
        "adj" => Some("ADJ"),
        "adv" => Some("ADV"),
        "pron" => Some("PRON"),
        "det" => Some("DET"),
        "num" => Some("NUM"),
        _ => None,
    }
}

/// Gjinia nga shablloni i kokës (`m`, `f`, `n`, `mf`, `f-p`, …) dhe, kur mungon,
/// nga etiketat e kuptimeve (masculine/feminine/neuter). Kthen (gjinia, shumës_vetëm).
fn gender_of(raw: &str, sense_tags: &[&str]) -> (Option<&'static str>, bool) {
    let mut m = false;
    let mut f = false;
    let mut n = false;
    let mut pl = false;
    for g in raw.split(',').map(str::trim) {
        match g {
            "m" | "m-s" => m = true,
            "f" | "f-s" => f = true,
            "n" | "n-s" => n = true,
            "mf" | "mfbysense" | "m-f" => {
                m = true;
                f = true;
            }
            "m-p" | "mpl" | "mp" => {
                m = true;
                pl = true;
            }
            "f-p" | "fpl" | "fp" => {
                f = true;
                pl = true;
            }
            "n-p" | "npl" => {
                n = true;
                pl = true;
            }
            "p" | "pl" => pl = true,
            _ => {}
        }
    }
    let from_head = m || f || n;
    let (m, f, n) = match from_head {
        true => (m, f, n),
        false => (
            sense_tags.contains(&"masculine"),
            sense_tags.contains(&"feminine"),
            sense_tags.contains(&"neuter"),
        ),
    };
    let g = match (m, f, n) {
        (true, false, false) => Some("m"),
        (false, true, false) => Some("f"),
        (false, false, true) => Some("n"),
        (true, true, false) => Some("mf"),
        _ => None,
    };
    (g, pl)
}

fn gender_feat(g: Option<&str>) -> Option<&'static str> {
    match g {
        Some("m") => Some("Masc"),
        Some("f") => Some("Fem"),
        Some("n") => Some("Neut"),
        _ => None,
    }
}

/// Etiketë Wiktextract → tipar UD. `None` = etiketë e panjohur (forma nuk merret).
fn tag_feat(tag: &str, verbal: bool) -> Option<(&'static str, &'static str)> {
    match (tag, verbal) {
        ("singular", _) => Some(("Number", "Sing")),
        ("plural", _) | ("plural-only", false) => Some(("Number", "Plur")),
        ("nominative", false) => Some(("Case", "Nom")),
        ("accusative", false) => Some(("Case", "Acc")),
        ("genitive", false) => Some(("Case", "Gen")),
        ("dative", false) => Some(("Case", "Dat")),
        ("ablative", false) => Some(("Case", "Abl")),
        ("vocative", false) => Some(("Case", "Voc")),
        ("definite", false) => Some(("Definite", "Def")),
        ("indefinite", false) => Some(("Definite", "Ind")),
        ("masculine", false) => Some(("Gender", "Masc")),
        ("feminine", false) => Some(("Gender", "Fem")),
        ("neuter", false) => Some(("Gender", "Neut")),
        ("first-person", true) => Some(("Person", "1")),
        ("second-person", true) => Some(("Person", "2")),
        ("third-person", true) => Some(("Person", "3")),
        ("indicative", true) => Some(("Mood", "Ind")),
        ("subjunctive", true) => Some(("Mood", "Sub")),
        ("admirative", true) => Some(("Mood", "Adm")),
        ("optative", true) => Some(("Mood", "Opt")),
        ("imperative", true) => Some(("Mood", "Imp")),
        ("conditional", true) => Some(("Mood", "Cnd")),
        ("present", true) => Some(("Tense", "Pres")),
        ("imperfect", true) => Some(("Tense", "Imp")),
        ("aorist", true) => Some(("Tense", "Past")),
        ("future", true) => Some(("Tense", "Fut")),
        ("participle", true) => Some(("VerbForm", "Part")),
        ("infinitive", true) => Some(("VerbForm", "Inf")),
        ("gerund", true) => Some(("VerbForm", "Ger")),
        ("active", true) => Some(("Voice", "Act")),
        ("mediopassive", true) => Some(("Voice", "Mid")),
        ("passive", true) => Some(("Voice", "Pass")),
        ("negative", true) => Some(("Polarity", "Neg")),
        _ => None,
    }
}

/// Etiketat → një ose më shumë vargje tiparesh UD (p.sh. "accusative,nominative"
/// jep dy hyrje). `head_form` = forma nga koka e fjalës (pa rasë: emërore).
fn feats_of(tags: &[&str], upos: &str, gender: Option<&str>, pl_only: bool, head_form: bool) -> Option<Vec<String>> {
    let verbal = upos == "VERB";
    let nounish = matches!(upos, "NOUN" | "PROPN");
    let mut by: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    for t in tags.iter().filter(|t| **t != "form-of") {
        let (k, v) = tag_feat(t, verbal)?;
        let vals = by.entry(k).or_default();
        match vals.contains(&v) {
            true => {}
            false => vals.push(v),
        }
    }
    match (nounish, by.contains_key("Gender"), gender_feat(gender)) {
        (true, false, Some(g)) => {
            by.insert("Gender", vec![g]);
        }
        _ => {}
    }
    match (nounish && head_form, by.contains_key("Case")) {
        (true, false) => {
            by.insert("Case", vec!["Nom"]);
        }
        _ => {}
    }
    match (nounish && head_form, by.contains_key("Definite")) {
        (true, false) => {
            by.insert("Definite", vec!["Ind"]);
        }
        _ => {}
    }
    match (nounish, by.contains_key("Number")) {
        (true, false) => {
            by.insert("Number", vec![match pl_only {
                true => "Plur",
                false => "Sing",
            }]);
        }
        _ => {}
    }
    match (verbal, by.contains_key("Mood"), by.contains_key("VerbForm")) {
        (true, true, false) => {
            by.insert("VerbForm", vec!["Fin"]);
        }
        _ => {}
    }
    match by.is_empty() {
        true => return None,
        false => {}
    }
    // Prodhimi kartezian i vlerave (renditja e çelësave sipas UD: alfabetike).
    let mut out: Vec<Vec<String>> = vec![vec![]];
    for (k, vals) in &by {
        out = out
            .into_iter()
            .flat_map(|prefix| {
                vals.iter().map(move |v| {
                    let mut p = prefix.clone();
                    p.push(format!("{k}={v}"));
                    p
                })
            })
            .collect();
    }
    Some(out.into_iter().map(|p| p.join("|")).collect())
}

fn parse(text: &str, rep: &mut ImportReport) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    let mut bad = 0usize;
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        let get = |i: usize| c.get(i).copied().unwrap_or("").to_string();
        let n = c.get(1).and_then(|x| x.parse::<usize>().ok());
        match (c.first().copied(), out.last_mut(), n) {
            (Some("E"), _, _) => out.push(Entry {
                word: get(1),
                lemma: lemma_of(&get(1), &get(2)),
                pos: get(2),
                gender: get(3),
                canonical: get(4),
                ..Entry::default()
            }),
            (Some("F"), Some(e), _) => e.forms.push((get(1), get(2), get(3))),
            (Some("G"), Some(e), Some(n)) => e.glosses.push((n, get(2), get(3))),
            (Some("O"), Some(e), Some(n)) => e.links.push((n, get(2), get(3), get(4))),
            (Some("R"), Some(e), Some(n)) => e.rels.push((n, get(2), get(3), get(4), get(5))),
            (Some(x), _, _) => match x.starts_with('#') || x.is_empty() {
                true => {}
                false => bad += 1,
            },
            (None, _, _) => {}
        }
    }
    match bad {
        0 => {}
        b => rep.note(format!("kaikki_en: {b} rreshta TSV të panjohur u anashkaluan")),
    }
    out
}

#[derive(Default)]
struct Counts {
    entries: usize,
    morphemes: usize,
    lex: usize,
    senses: usize,
    morph_table: usize,
    morph_head: usize,
    morph_form_of: usize,
    forms_unmapped: usize,
    forms_multiword: usize,
    head_superseded: usize,
    destressed: usize,
    form_of: usize,
    alt_of: usize,
    sem: BTreeMap<&'static str, usize>,
    not_albanian: usize,
    foreign_lang: usize,
    inflected_skipped: usize,
    self_loops: usize,
    non_latin: usize,
    forms_junk: usize,
    gloss_form_of: usize,
    rel_nonlexical: usize,
    /// Hyrje pa urë angleze sepse nyja-alias «i X»/«e X» ka kuptim shqip të konfirmuar.
    alias_sq: usize,
}

/// Numrat e kuptimeve JO-leksikore të një hyrjeje: form-of / alt-of (përfshirë
/// shkurtimet «abbreviation of …») dhe glosat «<etiketa> of <fjala>».
/// Marrëdhëniet e këtyre kuptimeve nuk i përkasin fjalës homografe (p.sh. «nën»
/// 'under' ≠ «nën» = shkurtim i «nëntor»), prandaj nuk importohen.
fn nonlexical_senses(e: &Entry) -> HashSet<usize> {
    e.glosses
        .iter()
        .filter(|(_, t, gl)| {
            let ts = split_tags(t);
            ts.contains(&"form-of") || ts.contains(&"alt-of") || gloss_form_of(gl).is_some()
        })
        .map(|(n, _, _)| *n)
        .collect()
}

/// Lidhjet form-of të nxjerra nga glosa, VETËM kur kuptimi s'ka rresht `O` të strukturuar.
fn gloss_links(e: &Entry) -> Vec<(usize, String, String)> {
    let structured: HashSet<usize> = e.links.iter().map(|(n, _, _, _)| *n).collect();
    e.glosses
        .iter()
        .filter(|(n, _, _)| !structured.contains(n))
        .filter_map(|(n, _, gl)| gloss_form_of(gl).map(|(t, tags)| (*n, t, tags)))
        .collect()
}

fn sem_name(r: &str) -> Option<&'static str> {
    match r {
        "SYNONYM" => Some("SYNONYM"),
        "ANTONYM" => Some("ANTONYM"),
        "HYPERNYM" => Some("HYPERNYM"),
        "HYPONYM" => Some("HYPONYM"),
        "DERIVED" => Some("DERIVED_FROM"),
        "RELATED" => Some("RELATED"),
        "COORDINATE" => Some("COORDINATE"),
        _ => None,
    }
}

/// Fjala shqipe (nyje Word(sq) ekzistuese) për një varg objektiv; provohet edhe pa nyjë.
fn albanian_target(g: &Graph, raw: &str) -> Option<(u32, String)> {
    let t = destress(raw.trim()).to_lowercase();
    let stripped = strip_article(&t).0.to_string();
    [t.clone(), stripped]
        .into_iter()
        .find_map(|c| g.get(&word_key(Lang::Sq, &c)).map(|id| (id, c)))
}

/// Kuptim shqip (AlbaNet, BabelNet ose zgjerim lokal), jo urë angleze.
fn albanian_sense(e: &Edge) -> bool {
    matches!(e.source, Source::OmwAls | Source::BabelNet53 | Source::LocalExtension)
}

/// Pjesët e një glose angleze për krahasim me etiketat e WordNet: «small, little» →
/// {small, little}; hiqen shënimet në kllapa dhe «to » e foljeve.
fn gloss_items(gloss: &str) -> Vec<String> {
    let mut depth = 0usize;
    let mut flat = String::new();
    for ch in gloss.chars() {
        match (ch, depth) {
            ('(', _) => depth += 1,
            (')', 0) => {}
            (')', _) => depth -= 1,
            (c, 0) => flat.push(c),
            _ => {}
        }
    }
    flat.split([',', ';'])
        .map(|p| {
            let t = p.trim().trim_end_matches('.').trim().to_lowercase();
            let verb = t.strip_prefix("to ").map(|r| r.trim().to_string());
            verb.unwrap_or(t)
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// A e ka nyja-alias me nyjë («i vogël», «e vogël») një kuptim shqip që i përket kësaj
/// fjale? Rregull i deklaruar: e njëjta pjesë ligjërate + të paktën një etiketë angleze
/// e kuptimit = një pjesë e glosës së Wiktionary-t. Kështu «i vogël» (small, little)
/// pranohet për «vogël», kurse «i ri» → «child» (emër) nuk pranohet për mbiemrin «ri».
fn alias_confirms(g: &Graph, lemma: &str, pos: Pos, items: &HashSet<String>) -> bool {
    ["i", "e"]
        .iter()
        .filter_map(|a| g.get(&word_key(Lang::Sq, &format!("{a} {lemma}"))))
        .any(|alias| {
            g.senses_of(alias).iter().any(|(sid, e)| {
                let node = g.node(*sid);
                let same_pos = match &node.kind {
                    NodeKind::Sense { pos: p, .. } => *p == pos,
                    _ => false,
                };
                albanian_sense(e)
                    && same_pos
                    && node.labels.iter().any(|t| t.lang == Lang::En && items.contains(&t.text.to_lowercase()))
            })
        })
}

fn sem_edge(from: u32, to: u32, rel: &str, attrs: Vec<(String, String)>) -> Edge {
    Edge { from, to, rel: Rel::Sem(rel.into()), source: Source::KaikkiEn, lang: Some(Lang::Sq), method: Method::Published, attrs }
}

#[allow(clippy::too_many_arguments)]
fn add_morph(
    morph: &mut Morph,
    seen: &mut HashSet<(String, String, String, String)>,
    lemma: &str,
    form: &str,
    upos: &str,
    feats: &str,
    sent_id: &str,
) -> bool {
    match seen.insert((lemma.to_string(), form.to_string(), upos.to_string(), feats.to_string())) {
        true => {
            morph.add(MorphEntry {
                lemma: lemma.to_string(),
                form: form.to_string(),
                upos: upos.to_string(),
                feats: feats.to_string(),
                sent_id: sent_id.to_string(),
                source: Source::KaikkiEn,
            });
            true
        }
        false => false,
    }
}

pub fn import(g: &mut Graph, morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let entries = parse(text, rep);
    let mut k = Counts::default();
    // Kalimi 1: nyjet e fjalëve për çdo hyrje (përfshirë format e lakuara si hyrje më vete).
    let mut ids: Vec<Option<u32>> = Vec::with_capacity(entries.len());
    for e in &entries {
        match (is_morpheme(e), non_latin(&e.word)) {
            (true, _) => {
                k.morphemes += 1;
                ids.push(None);
            }
            (false, true) => {
                k.non_latin += 1;
                ids.push(None);
            }
            (false, false) => {
                k.entries += 1;
                ids.push(Some(g.ensure_word(Lang::Sq, &e.lemma)));
            }
        }
    }
    // Format e lakuara të çdo leme (tabela + form_of) — për të mos i marrë si fjalë të prejardhura.
    let mut inflected: HashSet<(String, String)> = HashSet::new();
    let mut noun_gender: HashMap<String, (Option<&'static str>, bool)> = HashMap::new();
    for e in &entries {
        // Vetëm format e lakuara; format alternative/kanonike nuk llogariten.
        for (form, _, _) in e.forms.iter().filter(|(_, t, _)| {
            let ts = split_tags(t);
            !ts.contains(&"alternative") && !ts.contains(&"canonical") && !ts.contains(&"romanization")
        }).filter(|(f, _, _)| is_word_form(f)) {
            let f = destress(form).to_lowercase();
            inflected.insert((f.clone(), e.lemma.clone()));
            inflected.insert((strip_article(&f).0.to_string(), e.lemma.clone()));
        }
        for (_, kind, target, _) in &e.links {
            match kind.as_str() {
                "form_of" => {
                    inflected.insert((e.lemma.clone(), lemma_of(target, &e.pos)));
                }
                _ => {}
            }
        }
        for (_, target, _) in gloss_links(e) {
            inflected.insert((e.lemma.clone(), lemma_of(&target, &e.pos)));
        }
        match e.pos.as_str() {
            "noun" | "name" => {
                let st: Vec<&str> = e.glosses.iter().flat_map(|(_, t, _)| split_tags(t)).collect();
                let gd = gender_of(&e.gender, &st);
                noun_gender.entry(e.lemma.clone()).or_insert(gd);
            }
            _ => {}
        }
    }
    // Shqipja ka përparësi edhe te nyja-alias: AlbaNet i ka mbiemrat me nyjë («i vogël»).
    // Leme që «i X»/«e X» ka kuptim shqip të konfirmuar (`alias_confirms`) nuk merr ura angleze.
    let mut alias_sq: HashSet<String> = HashSet::new();
    for (e, id) in entries.iter().zip(ids.iter()) {
        match (id, pos_of(&e.pos)) {
            (Some(_), Some(p)) => {
                let nonlex = nonlexical_senses(e);
                let items: HashSet<String> = e
                    .glosses
                    .iter()
                    .filter(|(n, _, _)| !nonlex.contains(n))
                    .flat_map(|(_, _, gl)| gloss_items(gl))
                    .collect();
                match alias_confirms(g, &e.lemma, p, &items) {
                    true => {
                        alias_sq.insert(e.lemma.clone());
                    }
                    false => {}
                }
            }
            _ => {}
        }
    }
    let mut seen_morph: HashSet<(String, String, String, String)> = HashSet::new();
    let mut sense_no: HashMap<(String, String), usize> = HashMap::new();
    // Kalimi 2: gramatika, morfologjia e tabelave, kuptimet angleze, marrëdhëniet.
    for (e, id) in entries.iter().zip(ids.iter()) {
        let w = match id {
            Some(w) => *w,
            None => continue,
        };
        let nonlex = nonlexical_senses(e);
        let lexical: Vec<&(usize, String, String)> = e.glosses.iter().filter(|(n, _, _)| !nonlex.contains(n)).collect();
        let sense_tags: Vec<&str> = lexical.iter().flat_map(|(_, t, _)| split_tags(t)).collect();
        let (gender, pl_only) = match e.pos.as_str() {
            "noun" | "name" => gender_of(&e.gender, &sense_tags),
            _ => (None, false),
        };
        // ── Rel::Lex (vetëm për hyrjet me kuptim leksikor, jo për format e lakuara) ──
        match lexical.is_empty() {
            true => {}
            false => {
                let mut attrs: Vec<(String, String)> = Vec::new();
                let mut flags: Vec<String> = Vec::new();
                match pos_of(&e.pos) {
                    Some(p) => attrs.push(("pos".into(), p.code().into())),
                    None => flags.push(e.pos.clone()),
                }
                match gender {
                    Some(gd) => attrs.push(("gender".into(), gd.into())),
                    None => {}
                }
                let canon = destress(&e.canonical).to_lowercase();
                let (rest, art) = strip_article(&canon);
                match (e.pos.as_str(), art, rest == e.lemma) {
                    ("adj" | "noun", Some(a @ ("i" | "e")), true) => attrs.push(("article".into(), a.into())),
                    _ => {}
                }
                let vt = sense_tags.contains(&"transitive");
                let vi = sense_tags.contains(&"intransitive");
                match (e.pos.as_str(), vt, vi) {
                    ("verb", true, false) => attrs.push(("trans".into(), "vt".into())),
                    ("verb", false, true) => attrs.push(("trans".into(), "vi".into())),
                    ("verb", true, true) => flags.push("vt+vi".into()),
                    _ => {}
                }
                match (e.pos.as_str(), sense_tags.contains(&"mediopassive")) {
                    ("verb", true) => attrs.push(("med".into(), "yes".into())),
                    _ => {}
                }
                match (e.pos.as_str(), pl_only) {
                    ("name", _) => flags.push("propn".into()),
                    _ => {}
                }
                match pl_only {
                    true => flags.push("pl_tantum".into()),
                    false => {}
                }
                match flags.is_empty() {
                    true => {}
                    false => attrs.push(("flags".into(), flags.join(","))),
                }
                attrs.push(("src".into(), SRC_ATTR.into()));
                g.add_edge(Edge { from: w, to: w, rel: Rel::Lex, source: Source::KaikkiEn, lang: Some(Lang::Sq), method: Method::Published, attrs });
                k.lex += 1;
            }
        }
        // ── Kuptimet angleze (urë, të shënuara anglisht) ──
        let mut synset_of: HashMap<usize, String> = HashMap::new();
        // Shqipja ka përparësi: fjala që ka tashmë kuptim nga AlbaNet/BabelNet (ose zgjerim
        // lokal), vetë ose te nyja-alias «i X»/«e X» (e konfirmuar), nuk merr kuptime angleze
        // shtesë — ato vetëm do të shtonin paqartësi.
        let own_sq = g.senses_of(w).iter().any(|(_, e)| albanian_sense(e));
        let via_alias = alias_sq.contains(&e.lemma);
        match (own_sq, via_alias, lexical.is_empty()) {
            (false, true, false) => k.alias_sq += 1,
            _ => {}
        }
        let has_sq_sense = own_sq || via_alias;
        match (pos_of(&e.pos), has_sq_sense) {
            (Some(p), false) => {
                for (n, tags, gloss) in &lexical {
                    let counter = sense_no.entry((e.lemma.clone(), e.pos.clone())).or_insert(0);
                    *counter += 1;
                    let synset = format!("kaikki:{}:{}:{}", e.lemma, e.pos, counter);
                    let sid = g.ensure(sense_key(Source::KaikkiEn, &synset), NodeKind::Sense { source: Source::KaikkiEn, synset: synset.clone(), pos: p });
                    let t = Text { lang: Lang::En, text: gloss.clone(), source: Source::KaikkiEn };
                    g.add_label(sid, t.clone());
                    match g.nodes[sid as usize].glosses.contains(&t) {
                        true => {}
                        false => g.nodes[sid as usize].glosses.push(t),
                    }
                    let mut attrs = vec![("src".to_string(), SRC_ATTR.to_string())];
                    match tags.is_empty() {
                        true => {}
                        false => attrs.push(("tags".into(), tags.clone())),
                    }
                    g.add_edge(Edge { from: w, to: sid, rel: Rel::HasSense, source: Source::KaikkiEn, lang: Some(Lang::Sq), method: Method::Published, attrs });
                    synset_of.insert(*n, synset);
                    k.senses += 1;
                }
            }
            _ => {}
        }
        // ── Marrëdhëniet semantike ──
        for (n, rel, target, tags, lang) in &e.rels {
            let name = match sem_name(rel) {
                Some(x) => x,
                None => continue,
            };
            // Marrëdhënie e një kuptimi jo-leksikor (shkurtim/alt-of/form-of): nuk i përket homografit.
            match nonlex.contains(n) {
                true => {
                    k.rel_nonlexical += 1;
                    continue;
                }
                false => {}
            }
            match lang.as_str() {
                "" | "sq" => {}
                _ => {
                    k.foreign_lang += 1;
                    continue;
                }
            }
            let (to, tl) = match albanian_target(g, target) {
                Some(x) => x,
                None => {
                    k.not_albanian += 1;
                    continue;
                }
            };
            match (to == w, inflected.contains(&(tl.clone(), e.lemma.clone()))) {
                (true, _) => {
                    k.self_loops += 1;
                    continue;
                }
                (false, true) => {
                    k.inflected_skipped += 1;
                    continue;
                }
                _ => {}
            }
            let mut attrs: Vec<(String, String)> = Vec::new();
            match synset_of.get(n) {
                Some(s) => attrs.push(("sense".into(), s.clone())),
                None => {}
            }
            match tags.is_empty() {
                true => {}
                false => attrs.push(("tags".into(), tags.clone())),
            }
            attrs.push(("src".into(), SRC_ATTR.into()));
            // DERIVED: lista e fjalëve të prejardhura qëndron te BAZA → skaji nga e prejardhura te baza.
            // Kujdes: atributi `sense` emërton gjithmonë kuptimin e hyrjes ku qëndron lista në
            // Wiktionary; te DERIVED_FROM ky është kuptimi i BAZËS (skaji `to`), jo i fjalës së
            // prejardhur (p.sh. turpëroj → turp [sense=kaikki:turp:noun:1]).
            let (from, to) = match name {
                "DERIVED_FROM" => (to, w),
                _ => (w, to),
            };
            g.add_edge(sem_edge(from, to, name, attrs));
            *k.sem.entry(name).or_insert(0) += 1;
        }
    }
    // Kalimi M: morfologjia nga forms[]. Fillimisht tabelat (declension/conjugation) e
    // të gjitha hyrjeve, pastaj format e kokës VETËM për hyrjet pa tabelë: koka e
    // Wiktionary-t ka theks dhe, në disa hyrje, forma që nuk përputhen me tabelën.
    for head_phase in [false, true] {
        for (e, id) in entries.iter().zip(ids.iter()) {
            let upos = match (upos_of(&e.pos), id.is_some()) {
                (Some(u), true) => u,
                _ => continue,
            };
            // Tabelë e palexuar nga Wiktextract: qeliza pa asnjë etiketë (p.sh. «Afrim»: kokat
            // «nominative», «indefinite», «i/e/të/së» dalin si forma). Asnjë qelizë e saj nuk merret.
            let broken_table = e.forms.iter().any(|(_, t, src)| !src.is_empty() && split_tags(t).is_empty());
            // Tabelë e dëmtuar: ka qeliza me mbetje shablloni («{{{1}}}a», «Term?»). Qelizat e
            // shëndosha merren, por format e kokës NUK zëvendësohen nga kjo tabelë.
            let damaged_table = e.forms.iter().any(|(f, _, src)| !src.is_empty() && (f.contains("{{{") || f.contains('?')));
            let table_ok = !broken_table;
            let head_superseded = table_ok && !damaged_table && e.forms.iter().any(|(_, _, src)| !src.is_empty());
            let sense_tags: Vec<&str> = e
                .glosses
                .iter()
                .map(|(_, t, _)| split_tags(t))
                .filter(|ts| !ts.contains(&"form-of") && !ts.contains(&"alt-of"))
                .flatten()
                .collect();
            let (gender, pl_only) = match e.pos.as_str() {
                "noun" | "name" => gender_of(&e.gender, &sense_tags),
                _ => (None, false),
            };
            for (form, tags, src) in &e.forms {
                match (head_phase, src.is_empty(), table_ok, head_superseded) {
                    (false, false, false, _) => {
                        k.forms_junk += 1;
                        continue;
                    }
                    (false, false, true, _) => {}
                    (true, true, _, false) => {}
                    (true, true, _, true) => {
                        k.head_superseded += 1;
                        continue;
                    }
                    _ => continue,
                }
                let ts = split_tags(tags);
                let clean = destress(form.trim());
                match clean == form.trim() {
                    true => {}
                    false => k.destressed += 1,
                }
                // Nyja e përparme (i/e/të/së) është fjalë më vete në UD: «të bukura» → «bukura»,
                // «e marta» → «marta». Foljet nuk preken; as lemat që vetë nisin me nyjë.
                let surface = match (upos, strip_article(&e.lemma).1) {
                    ("VERB", _) | (_, Some(_)) => clean.clone(),
                    _ => strip_article(&clean).0.to_string(),
                };
                // Qelizat mbushëse të tabelës («-», «Term?», «{{{1}}}a», «i/e/të/së», «(për)kula»,
                // «sg.») nuk janë forma fjalësh shqipe: çdo fjalë e formës duhet të kalojë rregullin.
                match surface.split(' ').all(is_word_form) {
                    true => {}
                    false => {
                        k.forms_junk += 1;
                        continue;
                    }
                }
                match (surface.contains(' ') && !e.lemma.contains(' '), surface.is_empty()) {
                    (false, false) => {}
                    _ => {
                        k.forms_multiword += 1;
                        continue;
                    }
                }
                match feats_of(&ts, upos, gender, pl_only, head_phase) {
                    Some(list) => {
                        let sid = match head_phase {
                            true => "kaikki_en:head",
                            false => "kaikki_en:table",
                        };
                        for f in list {
                            match (add_morph(morph, &mut seen_morph, &e.lemma, &surface, upos, &f, sid), head_phase) {
                                (true, true) => k.morph_head += 1,
                                (true, false) => k.morph_table += 1,
                                _ => {}
                            }
                        }
                    }
                    None => k.forms_unmapped += 1,
                }
            }
        }
    }
    // Kalimi 3: form_of / alt_of → FORM_OF (+ morfologji nga etiketat e formës).
    for (e, id) in entries.iter().zip(ids.iter()) {
        let w = match id {
            Some(w) => *w,
            None => continue,
        };
        for (_, kind, target, tags) in &e.links {
            let (to, tl) = match albanian_target(g, target) {
                Some(x) => x,
                None => {
                    k.not_albanian += 1;
                    continue;
                }
            };
            match to == w {
                true => {
                    k.self_loops += 1;
                    continue;
                }
                false => {}
            }
            let rest: Vec<&str> = split_tags(tags).into_iter().filter(|t| *t != "form-of" && *t != "alt-of").collect();
            let mut attrs = vec![("kind".to_string(), kind.clone())];
            match rest.is_empty() {
                true => {}
                false => attrs.push(("tags".into(), rest.join(","))),
            }
            attrs.push(("src".into(), SRC_ATTR.into()));
            g.add_edge(sem_edge(w, to, "FORM_OF", attrs));
            match kind.as_str() {
                "alt_of" => k.alt_of += 1,
                _ => {
                    k.form_of += 1;
                    let (gd, pl) = noun_gender.get(&tl).copied().unwrap_or((None, false));
                    let feats = match (upos_of(&e.pos), rest.is_empty(), is_word_form(&e.lemma)) {
                        (Some(u), false, true) => feats_of(&rest, u, gd, pl, false).map(|f| (u, f)),
                        _ => None,
                    };
                    match feats {
                        Some((u, list)) => {
                            for f in list {
                                match add_morph(morph, &mut seen_morph, &tl, &e.lemma, u, &f, "kaikki_en:form_of") {
                                    true => k.morph_form_of += 1,
                                    false => {}
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
        }
        // Glosat «<etiketa> of <fjala>» pa `form_of` të strukturuar (p.sh. «kali»,
        // «qeshë»): bëhen FORM_OF (jo kuptime angleze). Pa morfologji: etiketat janë
        // tekst i lirë anglisht («simple perfect», «dative/ablative»), jo etiketa Wiktextract.
        for (_, target, gtags) in gloss_links(e) {
            let (to, _) = match albanian_target(g, &target) {
                Some(x) => x,
                None => {
                    k.not_albanian += 1;
                    continue;
                }
            };
            match to == w {
                true => {
                    k.self_loops += 1;
                    continue;
                }
                false => {}
            }
            let attrs = vec![
                ("kind".to_string(), "form_of".to_string()),
                ("via".to_string(), "gloss".to_string()),
                ("gloss_tags".to_string(), gtags),
                ("src".to_string(), SRC_ATTR.to_string()),
            ];
            g.add_edge(sem_edge(w, to, "FORM_OF", attrs));
            k.gloss_form_of += 1;
        }
    }
    let sem: Vec<String> = k.sem.iter().map(|(r, n)| format!("{r} {n}")).collect();
    rep.note(format!(
        "kaikki_en: {} hyrje (+{} morfema dhe {} hyrje në shkrim jo-latin të anashkaluara); {} skaje gramatikore (Lex); {} kuptime me glosë ANGLISHT (urë; {} hyrje pa urë sepse «i X»/«e X» ka kuptim shqip të konfirmuar); morfologji: {} forma tabele + {} forma koke (vetëm hyrjet pa tabelë; {} forma koke të zëvendësuara nga tabela) + {} nga form_of ({} qeliza mbushëse/jo-fjalë, {} forma me etiketë të panjohur dhe {} forma analitike/me shumë fjalë të pamarra; {} forma me theks të hequr); FORM_OF {} + alt_of {} + {} nga glosa «… of X»; marrëdhënie: {}; të pamarra: {} objektiva jo-shqip/të panjohur, {} në gjuhë tjetër, {} forma të lakuara të listuara si të prejardhura, {} të kuptimeve jo-leksikore (shkurtim/alt-of/form-of), {} vetë-lidhje",
        k.entries, k.morphemes, k.non_latin, k.lex, k.senses, k.alias_sq, k.morph_table, k.morph_head, k.head_superseded, k.morph_form_of, k.forms_junk, k.forms_unmapped, k.forms_multiword, k.destressed,
        k.form_of, k.alt_of, k.gloss_form_of, sem.join(", "), k.not_albanian, k.foreign_lang, k.inflected_skipped, k.rel_nonlexical, k.self_loops
    ));
    k.entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morph::FormOrigin;

    // TSV në formatin real që prodhon nura_fetch (nga hyrjet reale të kaikki.org, të shkurtuara).
    const FIX: &str = "# kaikki_en test\n\
E\tkalë\tnoun\tm\t\t\n\
F\tkuaj\tplural\t\n\
F\tkali\tdefinite\t\n\
F\tkalë\tindefinite,nominative,singular\tdeclension\n\
F\tkali\tdefinite,nominative,singular\tdeclension\n\
F\tkalin\taccusative,definite,singular\tdeclension\n\
F\tkuajt\taccusative,definite,nominative,plural\tdeclension\n\
F\tkal\talternative,Gheg\t\n\
R\t0\tDERIVED\tkalorës\t\t\t\n\
R\t0\tDERIVED\tkali\t\t\t\n\
G\t1\tmasculine\thorse\n\
R\t1\tSYNONYM\tat\t\t\t\n\
R\t1\tRELATED\thorse\t\ten\t\n\
E\tkalorës\tnoun\tm\t\t\n\
G\t1\t\trider, horseman\n\
E\tat\tnoun\tm\t\t1\n\
F\táti\tdefinite\t\n\
G\t1\t\tstallion\n\
E\tkali\tnoun\t\t\t\n\
G\t1\t\tdefinite nominative singular and indefinite dative/ablative singular of kalë\n\
E\tkalin\tnoun\t\t\t\n\
G\t1\taccusative,definite,form-of,singular\tdefinite accusative singular of kalë\n\
O\t1\tform_of\tkalë\taccusative,definite,form-of,singular\n\
E\tlule\tnoun\tf\t\t\n\
F\tlulja\tdefinite\t\n\
F\tlule\tplural\t\n\
G\t1\tfeminine\tflower\n\
E\tbukur\tadj\t\ti bukur\t\n\
F\ti bukur\tcanonical\t\n\
F\ttë bukura\tfeminine,plural\t\n\
G\t1\t\tbeautiful\n\
R\t1\tANTONYM\ti shëmtuar\t\t\t\n\
E\tshëmtuar\tadj\t\ti shëmtuar\t\n\
G\t1\t\tugly\n\
E\tpunoj\tverb\t\t\t\n\
F\tpunova\taorist\t\n\
F\tpunuar\tparticiple\t\n\
F\tpunoj\tfirst-person,indicative,present,singular\tconjugation\n\
F\tkam punuar\tfirst-person,indicative,perfect,singular\tconjugation\n\
G\t1\ttransitive\tto work\n\
E\tlexoj\tverb\t\t\t\n\
F\tlexova\taorist\t\n\
F\tlexuar\tparticiple\t\n\
G\t1\t\tto read\n\
E\tmartë\tnoun\tf\te martë\t\n\
F\te marta\tdefinite,nominative,singular\tdeclension\n\
G\t1\t\tTuesday\n\
E\tshpi\tnoun\tf\t\t\n\
G\t1\talt-of,alternative,colloquial,feminine\talternative form of shtëpi\n\
O\t1\talt_of\tshtëpi\talt-of,alternative,colloquial,feminine\n\
G\t2\tGheg,colloquial,feminine\tcarriage\n\
E\tshtëpi\tnoun\tf\t\t\n\
F\tshtëpia\tdefinite,nominative,singular\tdeclension\n\
G\t1\tfeminine\thouse\n\
E\t-im\tsuffix\t\t\t\n\
G\t1\t\tforms nouns\n\
E\tpi\tverb\t\t\t\n\
F\t-\tfirst-person,imperative,present,singular\tconjugation\n\
F\tpi\timperative,present,second-person,singular\tconjugation\n\
F\t-\timperative,present,singular,third-person\tconjugation\n\
F\tpini\timperative,plural,present,second-person\tconjugation\n\
G\t1\tGheg,Tosk,standard\tto drink, to suck\n\
E\tai\tpron\tm\t\t\n\
F\tatë\taccusative\t\n\
F\tTerm?\tnominative\tdeclension\n\
F\tTerm?\taccusative,clitic\tdeclension\n\
G\t1\tmasculine\the\n\
E\tmjezditë\tnoun\tf\t\t\n\
F\tmjezdita\tdefinite\t\n\
F\tmjezditë\tindefinite,nominative,singular\tdeclension\n\
F\t{{{1}}}a\tdefinite,nominative,singular\tdeclension\n\
F\tmjezditën\taccusative,definite,singular\tdeclension\n\
G\t1\tfeminine\tnoon, midday\n\
E\tAfrim\tname\tm\t\t\n\
F\tAfrime\tplural\t\n\
F\tAfrimi\tdefinite\t\n\
F\tnominative (emërore)\t\tdeclension\n\
F\ti/e/të/së\t\tdeclension\n\
F\tindefinite\tsingular\tdeclension\n\
F\tAfrim-i\t\tdeclension\n\
G\t1\t\ta male given name\n\
E\tkul\tverb\t\t\t\n\
F\t(për)kula\taorist\t\n\
F\t(për)kulur\tparticiple\t\n\
G\t1\t\tto bend, crouch, bow\n\
E\tTeuta\tname\tf\t\t\n\
F\tform\tdefinite\t\n\
G\t1\tfeminine\tname of the queen of the Illyrians\n\
E\tnën\tprep\t\t\t\n\
G\t1\twith-accusative\tunder\n\
E\tnën\tnoun\tm\t\t\n\
G\t1\tabbreviation,alt-of,masculine\tabbreviation of nëntor (“November”)\n\
O\t1\talt_of\tnëntor\tabbreviation,alt-of,masculine\n\
R\t1\tCOORDINATE\tkorr\t\t\t\n\
E\tnëntor\tnoun\tm\t\t\n\
G\t1\tmasculine\tNovember\n\
E\tkorr\tverb\t\t\t\n\
G\t1\ttransitive\tto harvest, reap\n\
E\tqe\tnoun\t\t\t\n\
G\t1\tform-of,indefinite,plural\tIndefinite plural of ka (“ox”).\n\
E\tka\tnoun\tm\t\t\n\
G\t1\tmasculine\tox\n\
E\tvdekur\tverb\t\t\t\n\
G\t1\tparticiple\tparticiple of vdes: dead\n\
E\tvdes\tverb\t\t\t\n\
G\t1\tintransitive\tto die\n\
E\tαρbε̰ρ\tnoun\tm\t\t\n\
F\tarbër\tromanization\t\n\
G\t1\tmasculine\tAlbanian/Arbëreshë\n";

    fn run() -> (Graph, Morph, ImportReport, usize) {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let n = import(&mut g, &mut m, FIX, &mut r);
        (g, m, r, n)
    }

    fn w(g: &Graph, l: &str) -> u32 {
        g.get(&word_key(Lang::Sq, l)).unwrap()
    }

    fn sem(g: &Graph, from: &str, rel: &str) -> Vec<String> {
        let id = w(g, from);
        g.out_edges(id)
            .filter(|(_, e)| e.rel == Rel::Sem(rel.into()))
            .map(|(_, e)| g.display_label(e.to).0)
            .collect()
    }

    fn lex(g: &Graph, l: &str) -> Vec<(String, String)> {
        let id = w(g, l);
        g.out_edges(id).filter(|(_, e)| e.rel == Rel::Lex).flat_map(|(_, e)| e.attrs.clone()).collect()
    }

    #[test]
    fn entries_and_morphemes() {
        let (g, _, r, n) = run();
        assert_eq!(n, 27);
        assert!(g.get(&word_key(Lang::Sq, "-im")).is_none());
        assert!(r.lines[0].contains("1 morfema"));
        // Arvanitisht me germa greke: nuk bëhet Word(sq).
        assert!(g.get(&word_key(Lang::Sq, "αρbε̰ρ")).is_none());
        assert!(r.lines[0].contains("1 hyrje në shkrim jo-latin"));
    }

    #[test]
    fn table_placeholders_are_not_word_forms() {
        let (_, m, _, _) = run();
        let all: Vec<&MorphEntry> = m.entries.iter().collect();
        for bad in ["-", "Term?", "{{{1}}}a", "i/e/të/së", "(për)kula", "(për)kulur", "form", "indefinite", "Afrim-i", "nominative (emërore)"] {
            assert!(!all.iter().any(|e| e.form == bad), "qelizë mbushëse e importuar: {bad}");
        }
        // Format e vërteta mbeten.
        let pi = m.entries_of("pi");
        assert!(pi.iter().any(|e| e.form == "pi" && e.feats == "Mood=Imp|Number=Sing|Person=2|Tense=Pres|VerbForm=Fin"));
        assert!(pi.iter().any(|e| e.form == "pini"));
        assert_eq!(pi.len(), 2);
        assert!(m.entries_of("ai").iter().any(|e| e.form == "atë"));
        assert_eq!(m.definite_nom_sg("mjezditë").text, "mjezdita");
        assert!(m.entries_of("mjezditë").iter().any(|e| e.form == "mjezditën"));
        // Tabela e palexuar e «Afrim» hidhet; format e kokës merren.
        let af = m.entries_of("afrim");
        assert!(af.iter().any(|e| e.form == "Afrimi" && e.sent_id == "kaikki_en:head"));
        assert!(!af.iter().any(|e| e.sent_id == "kaikki_en:table"));
        assert!(m.entries_of("kul").is_empty());
        assert!(m.entries_of("teuta").is_empty());
        assert!(is_word_form("AIDS-i"));
        assert!(is_word_form("s'ka"));
        assert!(is_word_form("male"));
        for bad in ["-", "—", "sg.", "Eh!", "*kantkë", "2", "-o", "nisira [1937]", "ε̰σ̈τε̰", "(a)tij", ""] {
            assert!(!is_word_form(bad), "{bad}");
        }
    }

    #[test]
    fn gloss_only_form_of_becomes_form_of_not_a_sense() {
        let (g, _, _, _) = run();
        assert!(g.senses_of(w(&g, "kali")).is_empty());
        assert!(lex(&g, "kali").is_empty());
        let e = g.out_edges(w(&g, "kali")).find(|(_, e)| e.rel == Rel::Sem("FORM_OF".into())).unwrap().1.clone();
        assert_eq!(g.display_label(e.to).0, "kalë");
        assert_eq!(e.attr("via"), Some("gloss"));
        assert_eq!(e.attr("gloss_tags"), Some("definite nominative singular and indefinite dative/ablative singular"));
        // Kuptim form-of pa rresht O: «Indefinite plural of ka (“ox”).»
        assert_eq!(sem(&g, "qe", "FORM_OF"), vec!["ka".to_string()]);
        assert!(g.senses_of(w(&g, "qe")).is_empty());
        // Glosë me kuptim pas objektivit mbetet kuptim.
        assert_eq!(g.senses_of(w(&g, "vdekur")).len(), 1);
        assert!(sem(&g, "vdekur", "FORM_OF").is_empty());
        assert_eq!(gloss_form_of("participle of vdes: dead"), None);
        assert_eq!(gloss_form_of("to (bring to an) end, finalize"), None);
        assert_eq!(gloss_form_of("abbreviation of nëntor (“November”)"), None);
        assert_eq!(
            gloss_form_of("first-person singular simple perfect indicative of jam"),
            Some(("jam".to_string(), "first-person singular simple perfect indicative".to_string()))
        );
    }

    #[test]
    fn relations_of_abbreviation_senses_do_not_land_on_the_homograph() {
        let (g, _, r, _) = run();
        // «nën» (shkurtim i «nëntor») COORDINATE «korr»: nuk lidhet te «nën» 'under'.
        assert!(sem(&g, "nën", "COORDINATE").is_empty());
        assert_eq!(sem(&g, "nën", "FORM_OF"), vec!["nëntor".to_string()]);
        assert!(r.lines[0].contains("1 të kuptimeve jo-leksikore"));
    }

    #[test]
    fn definite_nominative_comes_from_the_table() {
        let (_, m, _, _) = run();
        let r = m.definite_nom_sg("kalë");
        assert_eq!(r.text, "kali");
        assert!(matches!(r.origin, FormOrigin::Attested { source: Source::KaikkiEn, .. }));
        assert_eq!(m.definite_nom_sg("lule").text, "lulja");
        assert_eq!(m.definite_nom_sg("shtëpi").text, "shtëpia");
        assert_eq!(m.gender("kalë"), Some(crate::morph::Gender::Masc));
        assert_eq!(m.gender("lule"), Some(crate::morph::Gender::Fem));
        // Dy rasa në një rresht → dy hyrje.
        let kuajt: Vec<String> = m.entries_of("kalë").iter().filter(|e| e.form == "kuajt").map(|e| e.feats.clone()).collect();
        assert!(kuajt.contains(&"Case=Nom|Definite=Def|Gender=Masc|Number=Plur".to_string()));
        assert!(kuajt.contains(&"Case=Acc|Definite=Def|Gender=Masc|Number=Plur".to_string()));
        // Hyrja me tabelë: format e kokës nuk merren (tabela i ka; koka mund të ketë theks/gabime).
        assert!(!m.entries_of("kalë").iter().any(|e| e.sent_id == "kaikki_en:head"));
        // Hyrja pa tabelë: forma e kokës "lule" (plural) → emërore e pashquar shumës.
        assert!(m
            .entries_of("lule")
            .iter()
            .any(|e| e.form == "lule" && e.feats == "Case=Nom|Definite=Ind|Gender=Fem|Number=Plur" && e.sent_id == "kaikki_en:head"));
        // Theksi i Wiktionary-t hiqet: «áti» → «ati».
        assert_eq!(m.definite_nom_sg("at").text, "ati");
        // Emër me nyjë: «e marta» → token «marta» (si në UD), nyja te Lex.
        assert_eq!(m.definite_nom_sg("martë").text, "marta");
        // Forma alternative (Gheg) nuk është formë e lakuar.
        assert!(!m.entries_of("kalë").iter().any(|e| e.form == "kal"));
    }

    #[test]
    fn verbs_and_adjectives_morphology() {
        let (_, m, _, _) = run();
        let p = m.entries_of("punoj");
        assert!(p.iter().any(|e| e.form == "punoj" && e.feats == "Mood=Ind|Number=Sing|Person=1|Tense=Pres|VerbForm=Fin"));
        assert!(!p.iter().any(|e| e.form == "punova"), "koka zëvendësohet nga tabela");
        let l = m.entries_of("lexoj");
        assert!(l.iter().any(|e| e.form == "lexova" && e.feats == "Tense=Past"));
        assert!(l.iter().any(|e| e.form == "lexuar" && e.feats == "VerbForm=Part"));
        assert!(!p.iter().any(|e| e.form.contains(' ')), "format analitike nuk merren");
        let b = m.entries_of("bukur");
        assert!(b.iter().any(|e| e.form == "bukura" && e.upos == "ADJ" && e.feats == "Gender=Fem|Number=Plur"));
        assert!(!b.iter().any(|e| e.form.contains("i bukur")));
    }

    #[test]
    fn lex_attributes() {
        let (g, _, _, _) = run();
        let k = lex(&g, "kalë");
        assert!(k.contains(&("pos".into(), "n".into())));
        assert!(k.contains(&("gender".into(), "m".into())));
        let b = lex(&g, "bukur");
        assert!(b.contains(&("pos".into(), "a".into())));
        assert!(b.contains(&("article".into(), "i".into())));
        assert!(lex(&g, "martë").contains(&("article".into(), "e".into())));
        assert!(lex(&g, "punoj").contains(&("trans".into(), "vt".into())));
        // Formë e lakuar si hyrje: pa Lex.
        assert!(lex(&g, "kali").is_empty());
    }

    #[test]
    fn relations_only_between_albanian_words() {
        let (g, _, _, _) = run();
        assert_eq!(sem(&g, "kalë", "SYNONYM"), vec!["at".to_string()]);
        // "horse" (lang en) dhe objektivat e panjohur nuk krijohen.
        assert!(g.get(&word_key(Lang::Sq, "horse")).is_none());
        assert!(sem(&g, "kalë", "RELATED").is_empty());
        // DERIVED në bazë → skaj nga e prejardhura te baza; forma e lakuar "kali" nuk merret.
        assert_eq!(sem(&g, "kalorës", "DERIVED_FROM"), vec!["kalë".to_string()]);
        assert!(sem(&g, "kali", "DERIVED_FROM").is_empty());
        // Objektivi me nyjë ("i shëmtuar") gjendet si lemë pa nyjë.
        assert_eq!(sem(&g, "bukur", "ANTONYM"), vec!["shëmtuar".to_string()]);
        // FORM_OF dhe alt_of.
        assert_eq!(sem(&g, "kali", "FORM_OF"), vec!["kalë".to_string()]);
        assert_eq!(sem(&g, "shpi", "FORM_OF"), vec!["shtëpi".to_string()]);
        let e = g.out_edges(w(&g, "shpi")).find(|(_, e)| e.rel == Rel::Sem("FORM_OF".into())).unwrap().1.clone();
        assert_eq!(e.attr("kind"), Some("alt_of"));
        assert_eq!(e.attr("tags"), Some("alternative,colloquial,feminine"));
        assert_eq!(e.method, Method::Published);
        assert_eq!(e.lang, Some(Lang::Sq));
    }

    #[test]
    fn english_glosses_stay_english() {
        let (g, _, _, _) = run();
        let s = g.senses_of(w(&g, "kalë"));
        assert_eq!(s.len(), 1);
        let sid = s[0].0;
        assert_eq!(g.node(sid).key, "s:kaikki_en:kaikki:kalë:noun:1");
        assert_eq!(g.gloss_in(sid, Lang::En).map(|t| t.text.as_str()), Some("horse"));
        assert!(g.gloss_in(sid, Lang::Sq).is_none());
        assert!(g.label_in(sid, Lang::Sq).is_none());
        assert_eq!(g.display_label(sid), ("horse".to_string(), Lang::En));
        // Kuptimet form-of / alt-of nuk bëhen kuptime; "shpi" ruan vetëm "carriage".
        assert!(g.senses_of(w(&g, "kali")).is_empty());
        let sh = g.senses_of(w(&g, "shpi"));
        assert_eq!(sh.len(), 1);
        assert_eq!(sh[0].1.attr("tags"), Some("Gheg,colloquial,feminine"));
        // Sinonimi i lidhur me kuptimin përkatës.
        let e = g.out_edges(w(&g, "kalë")).find(|(_, e)| e.rel == Rel::Sem("SYNONYM".into())).unwrap().1.clone();
        assert_eq!(e.attr("sense"), Some("kaikki:kalë:noun:1"));
    }

    #[test]
    fn form_of_feeds_morphology_after_tables() {
        let (_, m, _, _) = run();
        assert!(m.entries_of("kalë").iter().any(|e| e.form == "kalin" && e.feats == "Case=Acc|Definite=Def|Gender=Masc|Number=Sing"));
        // Pa dyfishim: kali Def Nom Sing ekziston një herë.
        let n = m.entries_of("kalë").iter().filter(|e| e.form == "kali" && e.feats == "Case=Nom|Definite=Def|Gender=Masc|Number=Sing").count();
        assert_eq!(n, 1);
    }

    #[test]
    fn alias_with_confirmed_albanian_sense_blocks_the_english_bridge() {
        let mut g = Graph::new();
        // AlbaNet (synset-e reale): «i vogël» small/little (mbiemër); «i ri» child (emër);
        // «i vjetër» elder/older (mbiemër, por glosa e Wiktionary-t është «old»).
        for (alias, syn, pos, labels) in [
            ("i vogël", "01391351-a", Pos::Adj, vec!["small", "little"]),
            ("i ri", "09917593-n", Pos::Noun, vec!["child", "kid"]),
            ("i vjetër", "02100031-a", Pos::Adj, vec!["elder", "older"]),
        ] {
            let s = g.ensure(sense_key(Source::Pwn30, syn), NodeKind::Sense { source: Source::Pwn30, synset: syn.into(), pos });
            for l in labels {
                g.add_label(s, Text { lang: Lang::En, text: l.into(), source: Source::Pwn30 });
            }
            let a = g.ensure_word(Lang::Sq, alias);
            g.add_edge(Edge { from: a, to: s, rel: Rel::HasSense, source: Source::OmwAls, lang: Some(Lang::Sq), method: Method::OmwOffsetMap, attrs: vec![] });
        }
        let tsv = "E\tvogël\tadj\t\ti vogël\t\nG\t1\t\tsmall, little\nE\tri\tadj\t\ti ri\t\nG\t1\t\tyoung\nG\t2\t\tnew\nE\tvjetër\tadj\t\ti vjetër\t\nG\t1\t\told\n";
        let mut r = ImportReport::default();
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        let bridges = |g: &Graph, l: &str| g.senses_of(w(g, l)).iter().filter(|(_, e)| e.source == Source::KaikkiEn).count();
        // «i vogël» e konfirmon: pa urë angleze te «vogël».
        assert_eq!(bridges(&g, "vogël"), 0);
        // «i ri» (child, emër) dhe «i vjetër» (elder ≠ old) nuk konfirmohen: urat mbeten.
        assert_eq!(bridges(&g, "ri"), 2);
        assert_eq!(bridges(&g, "vjetër"), 1);
        assert!(r.lines[0].contains("1 hyrje pa urë"));
        assert_eq!(gloss_items("to bend, crouch (low), bow."), vec!["bend", "crouch", "bow"]);
    }

    #[test]
    fn feats_mapping_rejects_unknown_tags() {
        assert_eq!(feats_of(&["perfect", "singular"], "VERB", None, false, false), None);
        assert_eq!(
            feats_of(&["dative", "ablative", "indefinite", "plural-only"], "NOUN", Some("f"), true, false),
            Some(vec![
                "Case=Dat|Definite=Ind|Gender=Fem|Number=Plur".to_string(),
                "Case=Abl|Definite=Ind|Gender=Fem|Number=Plur".to_string()
            ])
        );
        assert_eq!(gender_of("f-p", &[]), (Some("f"), true));
        assert_eq!(gender_of("", &["masculine"]), (Some("m"), false));
        assert_eq!(gender_of("m,f", &[]), (Some("mf"), false));
        assert_eq!(destress("praními"), "pranimi");
        assert_eq!(destress("kalë çaj"), "kalë çaj");
    }
}
