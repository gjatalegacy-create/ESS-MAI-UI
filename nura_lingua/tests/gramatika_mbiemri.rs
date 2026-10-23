//! Matja e mbiemrit dhe e nyjës kundrejt arit: Wiktionary (trajtat e mbiemrit) dhe UD (nyja).
//! Ari lexohet vetëm këtu; rregullat ndërtohen duke parë vetëm gjysmën `Dev`, `Test` vetëm raportohet.

use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::mbiemri::{article, citation, AdjLex, Linking};
use nura_lingua::gramatika::tipet::{split_of, ud_case, ud_feat, ud_gender, ud_number, Case, Definite, Gender, Number, Pos, Split};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
const KAIKKI: &str = "raw/kaikki_en/kaikki_en-sq.tsv";
const UD: &[&str] = &[
    "raw/ud_sq_staf/sq_staf-ud-train.conllu",
    "raw/ud_sq_staf/sq_staf-ud-dev.conllu",
    "raw/ud_sq_staf/sq_staf-ud-test.conllu",
    "raw/ud_sq_tsa/sq_tsa-ud-test.conllu",
];

/// Sa mospërputhje `Dev` shtypen për punën me rregullat.
const SHOW_DEV_MISSES: usize = 0;

#[derive(Debug, Default, Clone, Copy)]
struct M {
    total: usize,
    answered: usize,
    correct: usize,
}

impl M {
    fn add(&mut self, answered: bool, correct: bool) {
        self.total += 1;
        self.answered += answered as usize;
        self.correct += (answered && correct) as usize;
    }

    fn precision(&self) -> f64 {
        ratio(self.correct, self.answered)
    }

    fn coverage(&self) -> f64 {
        ratio(self.answered, self.total)
    }

    fn wilson(&self) -> f64 {
        wilson_lower(self.correct, self.answered)
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    match b {
        0 => 0.0,
        _ => a as f64 / b as f64,
    }
}

/// Kufiri i poshtëm Wilson 95%: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n), z = 1.96.
fn wilson_lower(correct: usize, n: usize) -> f64 {
    let z = 1.96f64;
    match n {
        0 => 0.0,
        _ => {
            let nf = n as f64;
            let p = correct as f64 / nf;
            (p + z * z / (2.0 * nf) - z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt()) / (1.0 + z * z / nf)
        }
    }
}

fn split_name(s: Split) -> &'static str {
    match s {
        Split::Dev => "Dev",
        Split::Test => "Test",
    }
}

#[derive(Default)]
struct Table {
    rows: BTreeMap<(usize, String, &'static str), M>,
}

impl Table {
    fn add(&mut self, order: usize, name: &str, split: Split, answered: bool, correct: bool) {
        self.rows.entry((order, name.to_string(), split_name(split))).or_default().add(answered, correct);
    }

    fn get(&self, order: usize, name: &str, split: &'static str) -> M {
        self.rows.get(&(order, name.to_string(), split)).copied().unwrap_or_default()
    }

    fn print(&self) {
        eprintln!("| metric | split | total | answered | correct | precision | coverage | wilson95_low |");
        eprintln!("|---|---|---|---|---|---|---|---|");
        for ((_, name, split), m) in &self.rows {
            eprintln!(
                "| {name} | {split} | {} | {} | {} | {:.4} | {:.4} | {:.4} |",
                m.total,
                m.answered,
                m.correct,
                m.precision(),
                m.coverage(),
                m.wilson()
            );
        }
    }
}

/// Theksi i fjalorit (á, é ...) nuk është drejtshkrim shqip: hiqet nga ari.
fn plain(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '\u{301}')
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' => 'u',
            'ý' => 'y',
            o => o,
        })
        .collect()
}

/// Trajta pa nyjë dhe a kishte nyjë.
fn bare(s: &str) -> (bool, &str) {
    ["i ", "e ", "të "]
        .iter()
        .find_map(|a| s.strip_prefix(a))
        .map(|rest| (true, rest))
        .unwrap_or((false, s))
}

/// Rresht ari i shëndoshë drejtshkrimisht: pa shifra, jo një shkronjë, pa «ëe» në fund.
fn sound(s: &str) -> bool {
    let (_, b) = bare(s);
    !b.chars().any(|c| c.is_ascii_digit()) && b.chars().count() >= 2 && !b.ends_with("ëe")
}

fn cell_of(tags: &str) -> Option<(Gender, Number, &'static str)> {
    match tags {
        "feminine" => Some((Gender::Fem, Number::Sing, "feminine")),
        "masculine,plural" => Some((Gender::Masc, Number::Plur, "masculine plural")),
        "feminine,plural" => Some((Gender::Fem, Number::Plur, "feminine plural")),
        _ => None,
    }
}

/// Një hyrje mbiemri e Wiktionary-t me qelizat e saj.
#[derive(Default)]
struct Entry {
    lemma: String,
    canonical: Vec<String>,
    cells: Vec<(Gender, Number, &'static str, String)>,
}

impl Entry {
    /// Hyrja që ka vetëm femëroren «lemë + e», pa trajtë kanonike: vlera e paracaktuar e
    /// shabllonit, jo trajtë e shkruar nga redaktori.
    fn template_default(&self) -> bool {
        let only = self.cells.len() == 1;
        let default = self.cells.first().map(|(g, n, _, f)| *g == Gender::Fem && *n == Number::Sing && *f == format!("{}e", self.lemma)).unwrap_or(false);
        self.canonical.is_empty() && only && default
    }

    /// A shfaq hyrja nyjë: te trajta kanonike ose te qelizat; None kur s'ka asnjë trajtë.
    fn articulated(&self) -> Option<bool> {
        let forms: Vec<&str> = self.canonical.iter().map(String::as_str).chain(self.cells.iter().map(|(_, _, _, f)| f.as_str())).collect();
        (!forms.is_empty()).then(|| forms.iter().any(|f| bare(f).0))
    }
}

fn wiktionary() -> Vec<Entry> {
    let text = std::fs::read_to_string(Path::new(DATA).join(KAIKKI)).unwrap();
    let mut out: Vec<Entry> = Vec::new();
    let mut current: Option<Entry> = None;
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match c.first().copied() {
            Some("E") => {
                out.extend(current.take());
                current = (c.get(2).copied() == Some("adj")).then(|| Entry { lemma: c.get(1).copied().unwrap_or("").to_string(), ..Entry::default() });
            }
            Some("F") => {
                let tags = c.get(2).copied().unwrap_or("");
                let form = c.get(1).map(|f| plain(f.trim())).filter(|f| !f.is_empty() && f != "-");
                match (current.as_mut(), cell_of(tags), form) {
                    (Some(e), Some((g, n, name)), Some(form)) => {
                        let cell = (g, n, name, form);
                        match e.cells.contains(&cell) {
                            true => {}
                            false => e.cells.push(cell),
                        }
                    }
                    (Some(e), None, Some(form)) => match tags.split(',').any(|t| t == "canonical") {
                        true => e.canonical.push(form),
                        false => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out.extend(current);
    out
}

/// Trajtat e arit për një qelizë; `editor`: të paktën një trajtë e shëndoshë nga hyrje që
/// nuk është vlera e paracaktuar e shabllonit.
#[derive(Default)]
struct Gold {
    forms: BTreeSet<String>,
    editor: BTreeSet<String>,
}

type Cells = BTreeMap<(String, Gender, Number, &'static str), Gold>;

fn gold_cells(entries: &[Entry]) -> Cells {
    let mut cells = Cells::new();
    for e in entries {
        let editor = !e.template_default();
        for (g, n, name, form) in &e.cells {
            let gold = cells.entry((e.lemma.clone(), *g, *n, *name)).or_default();
            gold.forms.insert(form.clone());
            match editor && sound(form) {
                true => {
                    gold.editor.insert(form.clone());
                }
                false => {}
            }
        }
    }
    cells
}

struct Tok {
    id: usize,
    form: String,
    lemma: String,
    upos: String,
    feats: String,
    head: usize,
}

fn sentences() -> Vec<Vec<Tok>> {
    let mut out = Vec::new();
    for f in UD {
        let text = std::fs::read_to_string(Path::new(DATA).join(f)).unwrap();
        let mut cur: Vec<Tok> = Vec::new();
        for line in text.lines() {
            let c: Vec<&str> = line.split('\t').collect();
            match (line.trim().is_empty(), c.len() == 10) {
                (true, _) => out.push(std::mem::take(&mut cur)),
                (false, true) => {
                    let id = c[0].parse::<usize>().ok();
                    let head = c[6].parse::<usize>().ok();
                    match (id, head) {
                        (Some(id), Some(head)) => cur.push(Tok {
                            id,
                            form: c[1].to_lowercase(),
                            lemma: c[2].to_string(),
                            upos: c[3].to_string(),
                            feats: c[5].to_string(),
                            head,
                        }),
                        _ => {}
                    }
                }
                (false, false) => {}
            }
        }
        out.push(cur);
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

fn feat<'a>(t: &'a Tok, k: &str) -> Option<&'a str> {
    ud_feat(&t.feats, k)
}

fn gnc(t: &Tok) -> (Option<Gender>, Option<Number>, Option<Case>) {
    (feat(t, "Gender").and_then(ud_gender), feat(t, "Number").and_then(ud_number), feat(t, "Case").and_then(ud_case))
}

fn nominal(t: &Tok) -> bool {
    matches!(t.upos.as_str(), "NOUN" | "PROPN" | "PRON")
}

/// Shquarsia e emrit: pa tipar te përemri = e pashquar; pa tipar te emri = e panjohur.
fn definite(h: &Tok) -> Option<Definite> {
    match (feat(h, "Definite"), h.upos.as_str()) {
        (Some("Def"), _) => Some(Definite::Def),
        (Some("Ind"), _) => Some(Definite::Ind),
        (_, "PRON") => Some(Definite::Ind),
        _ => None,
    }
}

/// Nyja e parashikuar për mbiemrin `t` me nyjën `d`: tiparet e emrit që përcakton, ose të
/// mbiemrit vetë kur s'ka emër (kallëzuesor: emërore e pashquar, nyjë larg).
fn predict_adj(s: &[Tok], t: &Tok, d: &Tok) -> Option<&'static str> {
    let head = s.iter().find(|h| h.id == t.head).filter(|h| nominal(h));
    let (ag, an, ac) = gnc(t);
    match head {
        Some(h) => {
            let (hg, hn, hc) = gnc(h);
            let context = match (t.id > h.id, d.id == h.id + 1) {
                (false, _) => Linking::Prenominal,
                (true, true) => Linking::Contact,
                (true, false) => Linking::Distant,
            };
            let def = match context {
                Linking::Prenominal => Some(Definite::Def),
                _ => definite(h),
            };
            article(hg.or(ag)?, hn.or(an)?, hc.or(ac)?, def?, context)
        }
        None => article(ag?, an?, ac?, Definite::Ind, Linking::Distant),
    }
}

/// Nyja e parashikuar për gjinoren `t`, nga tiparet e emrit zotërues para saj.
fn predict_gen(s: &[Tok], t: &Tok, d: &Tok) -> Option<&'static str> {
    let h = s.iter().find(|h| h.id == t.head).filter(|h| nominal(h) && h.id < t.id)?;
    let (hg, hn, hc) = gnc(h);
    let context = match d.id == h.id + 1 {
        true => Linking::Contact,
        false => Linking::Distant,
    };
    article(hg?, hn?, hc?, definite(h)?, context)
}

const LINKERS: &[&str] = &["i", "e", "të", "së"];

/// Lemat `Test` që u panë gjatë shkrimit të rregullave R-NY2 (tema e shumësit, zanorja lidhëse «a»)
/// dhe R-NY5 (i pandryshueshëm me parashtesë): rreshtat e tyre maten edhe veçmas, pa to.
const SEEN_IN_RULE_WRITING: &[&str] = &["breshkamadh", "dashakeq", "pafund"];

const R_FEM: usize = 1;
const R_ALL: usize = 4;
const R_SOUND: usize = 5;
const R_SOUND_OUT: usize = 6;
const R_EDITOR: usize = 7;
const R_EDITOR_OUT: usize = 8;
const R_CIT: usize = 9;
const R_CIT_EDITOR: usize = 10;
const R_CIT_HELD_OUT: usize = 11;
const R_CIT_EDITOR_HELD_OUT: usize = 12;
const R_ART: usize = 13;
const R_ART_EDITOR: usize = 14;
const R_ART_HELD_OUT: usize = 15;
const R_ART_EDITOR_HELD_OUT: usize = 16;
const R_FAMILY: usize = 17;
const R_UD_ADJ: usize = 20;
const R_UD_GEN: usize = 21;
const R_UD_GEN_ART: usize = 22;

const N_ALL: &str = "wikt form: all three cells";
const N_SOUND: &str = "wikt form: all three cells, orthographically sound rows";
const N_SOUND_OUT: &str = "wikt form: rows left out of the sound subset";
const N_EDITOR: &str = "wikt form: all three cells, editor-entered entries";
const N_EDITOR_OUT: &str = "wikt form: rows left out of the editor subset";
const N_CIT: &str = "wikt citation (article + form)";
const N_CIT_EDITOR: &str = "wikt citation (article + form), editor-entered entries";
const N_CIT_HELD_OUT: &str = "wikt citation, without the 3 Test lemmas seen in rule writing";
const N_CIT_EDITOR_HELD_OUT: &str = "wikt citation, editor-entered, without the 3 Test lemmas seen in rule writing";
const N_ART: &str = "wikt articulation (lemma takes i/e/të)";
const N_ART_EDITOR: &str = "wikt articulation, editor-entered entries";
const N_ART_HELD_OUT: &str = "wikt articulation, without the 3 Test lemmas seen in rule writing";
const N_ART_EDITOR_HELD_OUT: &str = "wikt articulation, editor-entered, without the 3 Test lemmas seen in rule writing";
const N_UD_ADJ: &str = "UD article of ADJ (DET PronType=Art)";
const N_UD_GEN: &str = "UD linker of genitive (DET i/e/të/së)";
const N_UD_GEN_ART: &str = "UD linker of genitive, DET PronType=Art only";

fn wiktionary_rows(lexicon: &Lexicon, table: &mut Table) {
    let entries = wiktionary();
    let cells = gold_cells(&entries);
    let mut shown = 0usize;
    for ((lemma, g, n, name), gold) in &cells {
        let split = split_of(lemma);
        let adj = AdjLex::resolve(lexicon, lemma);
        let form = adj.form(*g, *n);
        let answered = form.is_some();
        let bare_gold: BTreeSet<&str> = gold.forms.iter().map(|f| bare(f).1).collect();
        let ok = form.as_ref().map(|f| bare_gold.contains(f.text.as_str())).unwrap_or(false);
        let order = match *name {
            "feminine" => R_FEM,
            "masculine plural" => R_FEM + 1,
            _ => R_FEM + 2,
        };
        table.add(order, &format!("wikt form: {name}"), split, answered, ok);
        table.add(R_ALL, N_ALL, split, answered, ok);
        let sound_gold: BTreeSet<&str> = gold.forms.iter().filter(|f| sound(f)).map(|f| bare(f).1).collect();
        match sound_gold.is_empty() {
            true => table.add(R_SOUND_OUT, N_SOUND_OUT, split, answered, ok),
            false => {
                let ok = form.as_ref().map(|f| sound_gold.contains(f.text.as_str())).unwrap_or(false);
                table.add(R_SOUND, N_SOUND, split, answered, ok);
            }
        }
        let held_out = !SEEN_IN_RULE_WRITING.contains(&lemma.as_str());
        let cit = citation(&adj, *g, *n);
        let ok_cit = cit.as_ref().map(|c| gold.forms.contains(c)).unwrap_or(false);
        table.add(R_CIT, N_CIT, split, cit.is_some(), ok_cit);
        match held_out {
            true => table.add(R_CIT_HELD_OUT, N_CIT_HELD_OUT, split, cit.is_some(), ok_cit),
            false => {}
        }
        match gold.editor.is_empty() {
            true => table.add(R_EDITOR_OUT, N_EDITOR_OUT, split, answered, ok),
            false => {
                let ok = form.as_ref().map(|f| gold.editor.iter().any(|g| bare(g).1 == f.text)).unwrap_or(false);
                let ok_cit = cit.as_ref().map(|c| gold.editor.contains(c)).unwrap_or(false);
                table.add(R_EDITOR, N_EDITOR, split, answered, ok);
                table.add(R_CIT_EDITOR, N_CIT_EDITOR, split, cit.is_some(), ok_cit);
                match held_out {
                    true => table.add(R_CIT_EDITOR_HELD_OUT, N_CIT_EDITOR_HELD_OUT, split, cit.is_some(), ok_cit),
                    false => {}
                }
                let family = form.as_ref().map(|f| f.rule.split([' ', ':']).next().unwrap_or("")).unwrap_or("none");
                table.add(R_FAMILY, &format!("wikt form by rule family {family}, editor-entered"), split, answered, ok);
            }
        }
        let show = split == Split::Dev && shown < SHOW_DEV_MISSES && (answered && !ok || cit.is_some() && !ok_cit);
        match show {
            true => {
                shown += 1;
                eprintln!("DEV miss {lemma} {name}: gold {:?} editor {:?} form {:?} citation {:?}", gold.forms, gold.editor, form.map(|f| (f.text, f.rule)), cit);
            }
            false => {}
        }
    }

    let mut art: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for e in &entries {
        match e.articulated() {
            Some(takes) => {
                let a = art.entry(e.lemma.clone()).or_insert((false, false));
                a.0 |= takes;
                a.1 |= !e.template_default();
            }
            None => {}
        }
    }
    for (lemma, (takes, editor)) in &art {
        let split = split_of(lemma);
        let a = AdjLex::resolve(lexicon, lemma).articulation;
        let ok = a.map(|a| a.takes == *takes).unwrap_or(false);
        let held_out = !SEEN_IN_RULE_WRITING.contains(&lemma.as_str());
        table.add(R_ART, N_ART, split, a.is_some(), ok);
        match (editor, held_out) {
            (true, true) => {
                table.add(R_ART_EDITOR, N_ART_EDITOR, split, a.is_some(), ok);
                table.add(R_ART_HELD_OUT, N_ART_HELD_OUT, split, a.is_some(), ok);
                table.add(R_ART_EDITOR_HELD_OUT, N_ART_EDITOR_HELD_OUT, split, a.is_some(), ok);
            }
            (true, false) => table.add(R_ART_EDITOR, N_ART_EDITOR, split, a.is_some(), ok),
            (false, true) => table.add(R_ART_HELD_OUT, N_ART_HELD_OUT, split, a.is_some(), ok),
            (false, false) => {}
        }
    }
}

fn ud_rows(lexicon: &Lexicon, table: &mut Table) {
    for s in sentences() {
        for t in &s {
            let adj = t.upos == "ADJ";
            let gen = matches!(t.upos.as_str(), "NOUN" | "PROPN") && feat(t, "Case") == Some("Gen");
            let linker = |d: &&Tok| feat(d, "PronType") == Some("Art") || (gen && feat(d, "PronType").is_none() && LINKERS.contains(&d.form.as_str()));
            let dets: Vec<&Tok> = s.iter().filter(|d| d.head == t.id && d.upos == "DET" && d.id < t.id).filter(linker).collect();
            // Te gjinorja e mbiemrit të emërzuar (të së vërtetës, e së keqes) lema është mbiemër i leksikut:
            // nyja menjëherë para tij është e mbiemrit vetë, lidhësja është e para. Përndryshe merret
            // nyja më pranë fjalës (lidhëza «e» para «të vendit» nuk është lidhëse).
            let substantivized = gen && lexicon.standard(&t.lemma.to_lowercase(), Pos::Adj).is_some();
            let det = match substantivized {
                true => dets.iter().min_by_key(|d| d.id),
                false => dets.iter().max_by_key(|d| d.id),
            }
            .copied();
            let split = split_of(&t.lemma);
            match (adj, gen, det) {
                (true, _, Some(d)) => {
                    let p = predict_adj(&s, t, d);
                    table.add(R_UD_ADJ, N_UD_ADJ, split, p.is_some(), p == Some(d.form.as_str()));
                }
                (_, true, Some(d)) => {
                    let p = predict_gen(&s, t, d);
                    let ok = p == Some(d.form.as_str());
                    table.add(R_UD_GEN, N_UD_GEN, split, p.is_some(), ok);
                    match feat(d, "PronType") == Some("Art") {
                        true => table.add(R_UD_GEN_ART, N_UD_GEN_ART, split, p.is_some(), ok),
                        false => {}
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn gold_measurement() {
    let lexicon = Lexicon::load(Path::new(DATA)).unwrap();
    let mut table = Table::default();
    wiktionary_rows(&lexicon, &mut table);
    ud_rows(&lexicon, &mut table);
    table.print();
    eprintln!("irregular table entries: 0 (forms come from the lexicon stems and the class table)");

    let test = |order: usize, name: &str| table.get(order, name, "Test");
    let floors: &[(usize, &str, f64, f64)] = &[
        (R_ALL, N_ALL, 0.905, 0.83),
        (R_SOUND, N_SOUND, 0.935, 0.84),
        (R_EDITOR, N_EDITOR, 0.955, 0.915),
        (R_CIT, N_CIT, 0.895, 0.805),
        (R_CIT_EDITOR, N_CIT_EDITOR, 0.935, 0.90),
        (R_CIT_HELD_OUT, N_CIT_HELD_OUT, 0.895, 0.805),
        (R_CIT_EDITOR_HELD_OUT, N_CIT_EDITOR_HELD_OUT, 0.93, 0.90),
        (R_ART, N_ART, 0.92, 0.69),
        (R_ART_EDITOR, N_ART_EDITOR, 0.955, 0.72),
        (R_ART_HELD_OUT, N_ART_HELD_OUT, 0.915, 0.685),
        (R_ART_EDITOR_HELD_OUT, N_ART_EDITOR_HELD_OUT, 0.955, 0.72),
        (R_UD_ADJ, N_UD_ADJ, 0.88, 0.97),
        (R_UD_GEN, N_UD_GEN, 0.875, 0.97),
    ];
    for (order, name, precision, coverage) in floors {
        let m = test(*order, name);
        eprintln!("floor {name}: precision {:.4} >= {precision}, coverage {:.4} >= {coverage}", m.precision(), m.coverage());
        assert!(m.precision() >= *precision, "{name}: precision {:.4} under floor {precision}", m.precision());
        assert!(m.coverage() >= *coverage, "{name}: coverage {:.4} under floor {coverage}", m.coverage());
    }
}
