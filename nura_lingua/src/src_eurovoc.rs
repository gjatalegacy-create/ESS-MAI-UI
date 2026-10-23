//! Importuesi i EuroVoc-ut shqip (tezaurusi i BE-së, skema 100141).
//!
//! TSV (nga nura_fetch):
//! `koncepti ⇥ prefLabel_sq ⇥ altLabel_sq(|) ⇥ broader ⇥ related ⇥ prefLabel_en ⇥ altLabel_en(|)`
//! (kolona e shtatë mund të mungojë në TSV më të vjetra; atëherë filtri përdor vetëm prefLabel_en).
//! - Çdo etiketë shqipe bëhet nyje `Word` (fjalë ose frazë), lema me shkronja të vogla;
//!   përcaktuesi në kllapa në fund («raport (dokument)») hiqet nga lema dhe ruhet si atribut.
//! - altLabel → prefLabel: `Sem("EUROVOC_UF")` (JO SYNONYM). Në EuroVoc altLabel është term
//!   «use for» (UF): ndonjëherë sinonim, shpesh term më i ngushtë i postuar te përshkruesi
//!   («tramvaj» → «mjete mbi shina»). Lidhja nuk paraqitet si sinonim.
//! - broader → `Sem("BROADER")` (JO HYPERNYM: «broader» i EuroVoc nuk është gjithmonë «është një lloj»).
//! - related → `Sem("RELATED")`.
//! - Çdo skaj mban `("domain","EU-admin")`, konceptin dhe etiketën angleze vetëm si urë (`en`).
//! - Anglishtja nuk paraqitet si shqip. Një etiketë «shqipe» hidhet kur:
//!   (a) prefLabel me disa fjalë që është e njëjtë me një etiketë angleze të të njëjtit koncept;
//!   (b) çdo etiketë që ka fjalë shërbimi angleze me shkronja të vogla
//!       (and, of, the, for, with, from, by) — «Marrëveshja AND» (akronim) mbetet;
//!   (c) altLabel me disa fjalë e njëjtë me një etiketë angleze të të njëjtit koncept
//!       («European Union investment»); një fjalë e vetme e njëjtë me anglishten NUK hidhet
//!       nga ky rregull, sepse shqipja i ka këto fjalë («hotel», «film», «opera», «proton»);
//!   (d) altLabel vetëm ASCII, asnjë fjalë e saj në prefLabel-et shqipe, dhe:
//!       me disa fjalë — çdo fjalë ose në fjalorin e etiketave angleze të EuroVoc ose me formë
//!       tipike angleze, të paktën një në fjalorin anglisht («natural reserve», «sounding balloon»);
//!       me një fjalë — në fjalorin anglisht DHE me formë tipike angleze (-ing, -tion, -sion,
//!       -ness, -ship, -ity, -ism, ose me shkronjën «w», që shqipja nuk e ka) («fishing»).
//!   Emrat e përveçëm (çdo fjalë me shkronjë të madhe: «Sri Lanka», «Grana Padano») nuk
//!   hidhen nga (a), (c), (d): ata përdoren të njëjtë në shqip.
use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;
use std::collections::{HashMap, HashSet};

pub const TSV: &str = "raw/eurovoc/eurovoc-sq.tsv";

/// Emri i marrëdhënies altLabel → prefLabel («use for» i tezaurusit, jo sinonim i konfirmuar).
pub const UF: &str = "EUROVOC_UF";

/// Fjalë shërbimi angleze që nuk janë fjalë shqipe.
const EN_FUNCTION: [&str; 7] = ["and", "of", "the", "for", "with", "from", "by"];

struct Row<'a> {
    id: &'a str,
    pref: &'a str,
    alts: Vec<&'a str>,
    broader: Vec<&'a str>,
    related: Vec<&'a str>,
    en: &'a str,
    en_alts: Vec<&'a str>,
}

fn parse(text: &str) -> Vec<Row<'_>> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            let col = |i: usize| c.get(i).map(|s| s.trim()).unwrap_or("");
            let list = |i: usize, sep: char| -> Vec<&str> {
                c.get(i).map(|s| s.split(sep).map(|x| x.trim()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
            };
            match (col(0).is_empty(), col(1).is_empty()) {
                (false, false) => Some(Row {
                    id: col(0),
                    pref: col(1),
                    alts: list(2, '|'),
                    broader: list(3, ' '),
                    related: list(4, ' '),
                    en: col(5),
                    en_alts: list(6, '|'),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Lema: shkronja të vogla, hapësira të njësuara, pa përcaktuesin në kllapa në fund.
/// Kthen (lema, përcaktuesi).
pub fn lemma_of(label: &str) -> (String, Option<String>) {
    let norm = norm(label);
    let qual = norm
        .strip_suffix(')')
        .and_then(|s| s.rsplit_once(" ("))
        .map(|(head, q)| (head.trim().to_string(), q.trim().to_string()));
    match qual {
        Some((head, q)) => match (head.is_empty(), q.is_empty()) {
            (false, false) => (head, Some(q)),
            _ => (norm, None),
        },
        None => (norm, None),
    }
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Fjalët e një etikete (shkronja të vogla, të ndara te çdo shenjë jo-shkronjë).
fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|ch: char| !ch.is_alphabetic())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Etiketat angleze të një koncepti (prefLabel + altLabel), të normalizuara.
fn en_labels(r: &Row) -> HashSet<String> {
    std::iter::once(r.en).chain(r.en_alts.iter().copied()).filter(|s| !s.is_empty()).map(norm).collect()
}

/// Fjalë shërbimi angleze të shkruara me shkronja të vogla (jo akronime si «AND»).
fn has_en_function_word(label: &str) -> bool {
    label
        .split(|ch: char| !ch.is_alphabetic())
        .any(|t| EN_FUNCTION.contains(&t))
}

/// Emër i përveçëm: çdo fjalë fillon me shkronjë të madhe.
fn proper_name(label: &str) -> bool {
    let words: Vec<&str> = label.split(|ch: char| ch.is_whitespace() || ch == '-').filter(|w| !w.is_empty()).collect();
    !words.is_empty() && words.iter().all(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
}

/// Formë tipike angleze për një fjalë të vetme (shqipja nuk ka «w» dhe këto mbaresa).
fn english_shape(t: &str) -> bool {
    t.contains('w') || ["ing", "tion", "sion", "ness", "ship", "ity", "ism"].iter().any(|s| t.len() > s.len() + 2 && t.ends_with(s))
}

/// (a)+(b): prefLabel që është anglisht e papërkthyer.
fn english_pref(r: &Row, en: &HashSet<String>) -> bool {
    let n = norm(r.pref);
    (n.contains(' ') && en.contains(&n) && !proper_name(r.pref)) || has_en_function_word(r.pref)
}

/// (b)+(c)+(d): altLabel që është anglisht e papërkthyer.
fn english_alt(alt: &str, en: &HashSet<String>, en_vocab: &HashSet<String>, sq_vocab: &HashSet<String>) -> bool {
    let toks = tokens(alt);
    let candidate = alt.is_ascii() && !toks.is_empty() && !toks.iter().any(|t| sq_vocab.contains(t));
    let english_words = candidate
        && toks.iter().any(|t| en_vocab.contains(t))
        && match toks.len() {
            1 => english_shape(&toks[0]),
            _ => toks.iter().all(|t| en_vocab.contains(t) || english_shape(t)),
        };
    let same_as_en = toks.len() > 1 && en.contains(&norm(alt));
    has_en_function_word(alt) || (!proper_name(alt) && (same_as_en || english_words))
}

pub fn import(g: &mut Graph, _morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let rows = parse(text);
    let en_of: Vec<HashSet<String>> = rows.iter().map(en_labels).collect();
    // Fjalori i etiketave angleze (të gjitha konceptet).
    let en_vocab: HashSet<String> = rows
        .iter()
        .flat_map(|r| std::iter::once(r.en).chain(r.en_alts.iter().copied()))
        .flat_map(tokens)
        .collect();
    let pref_ok: Vec<bool> = rows.iter().zip(&en_of).map(|(r, en)| !english_pref(r, en)).collect();
    // Fjalori shqip: fjalët e prefLabel-eve shqipe të pranuara.
    let sq_vocab: HashSet<String> =
        rows.iter().zip(&pref_ok).filter(|(_, ok)| **ok).flat_map(|(r, _)| tokens(r.pref)).collect();
    // Kalimi i parë: koncepti → nyja e fjalës së prefLabel-it shqip.
    let mut word_of: HashMap<&str, u32> = HashMap::new();
    let mut qual_of: HashMap<&str, String> = HashMap::new();
    let mut skipped_pref: Vec<&str> = Vec::new();
    for (r, ok) in rows.iter().zip(&pref_ok) {
        match ok {
            false => skipped_pref.push(r.pref),
            true => {
                let (lemma, q) = lemma_of(r.pref);
                match lemma.is_empty() {
                    true => {}
                    false => {
                        let w = g.ensure_word(Lang::Sq, &lemma);
                        g.add_label(w, Text { lang: Lang::Sq, text: r.pref.to_string(), source: Source::EuroVoc });
                        word_of.insert(r.id, w);
                        match q {
                            Some(q) => {
                                qual_of.insert(r.id, q);
                            }
                            None => {}
                        }
                    }
                }
            }
        }
    }
    let attrs = |id: &str, en: &str, qual: Option<&String>, extra: Option<(&str, &str)>| -> Vec<(String, String)> {
        let mut a = vec![("domain".to_string(), "EU-admin".to_string()), ("concept".to_string(), format!("eurovoc:{id}"))];
        match qual {
            Some(q) => a.push(("qualifier".into(), q.clone())),
            None => {}
        }
        match en.is_empty() {
            true => {}
            false => a.push(("en".into(), en.to_string())),
        }
        match extra {
            Some((k, v)) => a.push((k.to_string(), v.to_string())),
            None => {}
        }
        a
    };
    let (mut uf, mut broader, mut related, mut missing) = (0usize, 0usize, 0usize, 0usize);
    let mut skipped_alt: Vec<&str> = Vec::new();
    let edge = |g: &mut Graph, from: u32, to: u32, rel: &str, attrs: Vec<(String, String)>| -> bool {
        match from == to {
            true => false,
            false => {
                let before = g.edges.len();
                g.add_edge(Edge {
                    from,
                    to,
                    rel: Rel::Sem(rel.to_string()),
                    source: Source::EuroVoc,
                    lang: Some(Lang::Sq),
                    method: Method::Published,
                    attrs,
                });
                g.edges.len() > before
            }
        }
    };
    for (r, en) in rows.iter().zip(&en_of) {
        let w = match word_of.get(r.id) {
            Some(w) => *w,
            None => continue,
        };
        let q = qual_of.get(r.id);
        for alt in &r.alts {
            let (lemma, aq) = lemma_of(alt);
            match (lemma.is_empty(), english_alt(alt, en, &en_vocab, &sq_vocab)) {
                (true, _) => continue,
                (false, true) => {
                    skipped_alt.push(alt);
                    continue;
                }
                (false, false) => {}
            }
            let a = g.ensure_word(Lang::Sq, &lemma);
            g.add_label(a, Text { lang: Lang::Sq, text: alt.to_string(), source: Source::EuroVoc });
            let mut at = attrs(r.id, r.en, q, aq.as_deref().map(|x| ("alt_qualifier", x)));
            at.push(("thesaurus".into(), "UF: synonym or narrower term, not a confirmed synonym".into()));
            match edge(g, a, w, UF, at) {
                true => uf += 1,
                false => {}
            }
        }
        for b in &r.broader {
            match word_of.get(b) {
                Some(t) => match edge(g, w, *t, "BROADER", attrs(r.id, r.en, q, Some(("target", &format!("eurovoc:{b}"))))) {
                    true => broader += 1,
                    false => {}
                },
                None => missing += 1,
            }
        }
        for b in &r.related {
            match word_of.get(b) {
                Some(t) => match edge(g, w, *t, "RELATED", attrs(r.id, r.en, q, Some(("target", &format!("eurovoc:{b}"))))) {
                    true => related += 1,
                    false => {}
                },
                None => missing += 1,
            }
        }
    }
    let sample = |v: &[&str]| v.iter().take(12).map(|s| format!("«{s}»")).collect::<Vec<_>>().join(", ");
    rep.note(format!(
        "eurovoc: {} koncepte me etiketë shqipe; {} nyje-fjalë të lidhura; {uf} {UF} (altLabel→prefLabel, jo sinonim), {broader} BROADER, {related} RELATED; {} prefLabel dhe {} altLabel anglisht të anashkaluara (p.sh. {}; {}); {missing} lidhje drejt koncepteve pa etiketë shqipe",
        rows.len(),
        word_of.len(),
        skipped_pref.len(),
        skipped_alt.len(),
        sample(&skipped_pref),
        sample(&skipped_alt)
    ));
    uf + broader + related
}

#[cfg(test)]
mod tests {
    use super::*;

    // Rreshta realë nga EuroVoc 4.24 (formati i nura_fetch), të shkurtuar.
    const FIX: &str = "# eurovoc-sq.tsv — test\n\
10\ttregti e brendshme\t\t2449\t1806\tdomestic trade\t\n\
1000\tfinancim\tfinancim i Bashkimit Europian\t2449\t\tfinancing\tEU financing\n\
2449\ttregti\t\t\t\ttrade\t\n\
1806\tkonkurrencë\t\t\t10\tcompetition\t\n\
1\tArhus (konte)\t\t337\t\tÅrhus (county)\t\n\
777\tsoil conditioning\t\t2449\t\tsoil conditioning\t\n\
5465\trezervë natyrore\tnatural reserve|park natyror\t\t\tnature reserve\tnatural reserve|wildlife park\n\
6203\tvjedhje e identitetit\tfishing|vjedhje identiteti\t\t\tidentity theft\tphishing|identity fraud\n\
4500\tpeshkim\t\t\t\tfishing\t\n\
8000\tMaqedonia Lindore and Traka\t\t\t\tEastern Macedonia and Thrace\t\n\
3000\tmjete mbi shina\ttramvaj\t\t\trail vehicle\ttram\n\
c_ea3f5ed2\taerostat\tbalonë me ajër të ngrohtë|sounding balloon\t\t\taerostat\thot-air balloon\n\
1404\tindustri hoteliere\thotel\t\t\thotel industry\thotel\n\
bad line without tabs\n";

    fn sem(g: &Graph, from: &str, rel: &str, to: &str) -> bool {
        let f = g.get(&word_key(Lang::Sq, from)).unwrap();
        let t = g.get(&word_key(Lang::Sq, to)).unwrap();
        g.out_edges(f).any(|(_, e)| e.to == t && e.rel == Rel::Sem(rel.into()))
    }

    fn run(text: &str) -> (Graph, usize) {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let n = import(&mut g, &mut m, text, &mut r);
        (g, n)
    }

    #[test]
    fn imports_relations() {
        let (g, n) = run(FIX);
        assert!(sem(&g, "financim i bashkimit europian", UF, "financim"));
        assert!(sem(&g, "tregti e brendshme", "BROADER", "tregti"));
        assert!(sem(&g, "tregti e brendshme", "RELATED", "konkurrencë"));
        assert!(sem(&g, "konkurrencë", "RELATED", "tregti e brendshme"));
        assert!(sem(&g, "financim", "BROADER", "tregti"));
        assert!(sem(&g, "park natyror", UF, "rezervë natyrore"));
        assert!(sem(&g, "vjedhje identiteti", UF, "vjedhje e identitetit"));
        // «Use for» nuk është sinonim: tramvaji nuk është sinonim i «mjete mbi shina».
        assert!(sem(&g, "tramvaj", UF, "mjete mbi shina"));
        assert!(!g.edges.iter().any(|e| e.rel == Rel::Sem("SYNONYM".into())));
        // EuroVoc broader nuk është HYPERNYM.
        assert!(!g.edges.iter().any(|e| e.rel == Rel::Sem("HYPERNYM".into())));
        // Kllapat → atribut, jo pjesë e lemës.
        assert!(g.get(&word_key(Lang::Sq, "arhus")).is_some());
        assert_eq!(n, 10);
        let e = g.edges.iter().find(|e| e.rel == Rel::Sem("BROADER".into())).unwrap();
        assert_eq!(e.attr("domain"), Some("EU-admin"));
        assert_eq!(e.source, Source::EuroVoc);
        assert_eq!(e.method, Method::Published);
    }

    #[test]
    fn english_never_becomes_albanian() {
        let (g, _) = run(FIX);
        // prefLabel = anglisht e papërkthyer.
        assert!(g.get(&word_key(Lang::Sq, "soil conditioning")).is_none());
        // altLabel = etiketë angleze e të njëjtit koncept.
        assert!(g.get(&word_key(Lang::Sq, "natural reserve")).is_none());
        // altLabel me fjalë vetëm angleze (fjalori anglisht), asnjë fjalë shqipe.
        assert!(g.get(&word_key(Lang::Sq, "fishing")).is_none());
        // prefLabel me fjalë shërbimi angleze.
        assert!(g.get(&word_key(Lang::Sq, "maqedonia lindore and traka")).is_none());
        // Fjalë me formë angleze jashtë fjalorit + fjalë e fjalorit anglisht.
        assert!(g.get(&word_key(Lang::Sq, "sounding balloon")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "balonë me ajër të ngrohtë")).is_some());
        // Fjala e vetme e njëjtë me anglishten, por fjalë shqipe, mbetet.
        assert!(g.get(&word_key(Lang::Sq, "hotel")).is_some());
        // Shqipja ASCII mbetet.
        assert!(g.get(&word_key(Lang::Sq, "tramvaj")).is_some());
        assert!(g.get(&word_key(Lang::Sq, "park natyror")).is_some());
    }

    #[test]
    fn six_column_tsv_still_works() {
        let (g, n) = run("1000\tfinancim\tfinancim i BE-së|financing of projects\t\t\tfinancing\n");
        assert_eq!(n, 1);
        assert!(sem(&g, "financim i be-së", UF, "financim"));
        assert!(g.get(&word_key(Lang::Sq, "financing of projects")).is_none());
    }

    #[test]
    fn lemma_rules() {
        assert_eq!(lemma_of("Arhus (konte)"), ("arhus".to_string(), Some("konte".to_string())));
        assert_eq!(lemma_of("fond  (BE)"), ("fond".to_string(), Some("be".to_string())));
        assert_eq!(lemma_of("organizatë(im) kulturor(e)"), ("organizatë(im) kulturor(e)".to_string(), None));
        assert_eq!(lemma_of("Finlanda"), ("finlanda".to_string(), None));
    }

    #[test]
    fn empty_and_partial() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, "", &mut r), 0);
        assert_eq!(import(&mut g, &mut m, "# vetëm koment\n\n42\n42\tfjalë\n", &mut r), 0);
        assert!(g.get(&word_key(Lang::Sq, "fjalë")).is_some());
    }
}
