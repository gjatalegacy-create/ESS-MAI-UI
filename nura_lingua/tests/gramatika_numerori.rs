//! Matja e numërorëve me ar të jashtëm: Wiktionary (kaikki) dhe UD; vetitë e kthimit veçmas.
//! Ari lexohet vetëm këtu; moduli nuk e lexon kurrë. Rregullat ndërtohen vetëm me `Dev`.
//! Çelësi i ndarjes Dev/Test është vlera e numrit, si lemë e përbashkët e të gjitha shkrimeve të tij,
//! i njëjtë në çdo metrikë; kur ari nuk jep vlerë (UD jashtë Wiktionary-t), lema.

use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::numerori::*;
use nura_lingua::gramatika::tipet::{split_of, ud_feat, ud_gender, Gender, Pos, Split};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
const UD: [&str; 4] = [
    "raw/ud_sq_staf/sq_staf-ud-train.conllu",
    "raw/ud_sq_staf/sq_staf-ud-dev.conllu",
    "raw/ud_sq_staf/sq_staf-ud-test.conllu",
    "raw/ud_sq_tsa/sq_tsa-ud-test.conllu",
];

/// Rregullat e modulit që përgjigjen nga një tabelë e deklaruar.
const TABLE_RULES: [&str; 4] = [R_READ_BARE, R_ORD_FIRST, R_ORD_BARE, R_FRACTION_BARE];

/// Kufiri i poshtëm Wilson 95%: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n).
fn wilson_lower(correct: u64, n: u64, z: f64) -> f64 {
    match n {
        0 => 0.0,
        _ => {
            let nf = n as f64;
            let p = correct as f64 / nf;
            let z2 = z * z;
            (p + z2 / (2.0 * nf) - z * (p * (1.0 - p) / nf + z2 / (4.0 * nf * nf)).sqrt()) / (1.0 + z2 / nf)
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    total: u64,
    answered: u64,
    correct: u64,
}

impl Tally {
    fn add(&mut self, answered: bool, correct: bool) {
        self.total += 1;
        self.answered += u64::from(answered);
        self.correct += u64::from(answered && correct);
    }

    fn precision(&self) -> f64 {
        match self.answered {
            0 => 0.0,
            a => self.correct as f64 / a as f64,
        }
    }

    fn coverage(&self) -> f64 {
        match self.total {
            0 => 0.0,
            t => self.answered as f64 / t as f64,
        }
    }

    fn wilson(&self) -> f64 {
        wilson_lower(self.correct, self.answered, 1.96)
    }
}

/// Një metrikë: Dev, Test dhe Test pa njësitë që prekin ndonjë tabelë të deklaruar.
#[derive(Debug, Default, Clone, Copy)]
struct Metric {
    dev: Tally,
    test: Tally,
    test_regular: Tally,
}

impl Metric {
    fn add(&mut self, split: Split, table: bool, answered: bool, correct: bool) {
        match split {
            Split::Dev => self.dev.add(answered, correct),
            Split::Test => {
                self.test.add(answered, correct);
                match table {
                    true => {}
                    false => self.test_regular.add(answered, correct),
                }
            }
        }
    }
}

fn split_value(v: u64) -> Split {
    split_of(&v.to_string())
}

/// Pa asnjë përgjigje saktësia dhe kufiri Wilson nuk kanë kuptim: shkruhen «n/a».
fn row(name: &str, split: &str, t: &Tally) -> String {
    let defined = |x: f64| match t.answered {
        0 => "n/a".to_string(),
        _ => format!("{x:.4}"),
    };
    format!(
        "{:<26} {:<14} {:>8} {:>8} {:>8} {:>9} {:>9.4} {:>9}",
        name,
        split,
        t.total,
        t.answered,
        t.correct,
        defined(t.precision()),
        t.coverage(),
        defined(t.wilson())
    )
}

fn table(metrics: &[(&str, Metric)]) -> String {
    let head = format!(
        "{:<26} {:<14} {:>8} {:>8} {:>8} {:>9} {:>9} {:>9}",
        "metric", "split", "total", "answered", "correct", "precision", "coverage", "wilson95"
    );
    let rows: Vec<String> = metrics
        .iter()
        .flat_map(|(n, m)| [row(n, "Dev", &m.dev), row(n, "Test", &m.test), row(n, "Test-no-table", &m.test_regular)])
        .collect();
    [vec![head], rows].concat().join("\n")
}

/// Themelori i n kalon nga tabela `IRREGULAR` (njëzet, tridhjetë, dyzet).
fn has_irregular(n: u64) -> bool {
    cardinal(n, Gender::Masc).is_some_and(|c| c.split(' ').any(|w| IRREGULAR.iter().any(|(_, t, _)| *t == w)))
}

/// Njësia prek një tabelë kur rregulli i përgjigjes është rregull tabele, ose kur vlera
/// (e përgjigjes; pa përgjigje, e arit) kalon nga `IRREGULAR`.
fn touches_table(answer: Option<Reading>, gold: &[u64]) -> bool {
    match answer {
        Some(r) => TABLE_RULES.contains(&r.rule) || has_irregular(r.value),
        None => gold.iter().any(|v| has_irregular(*v)),
    }
}

fn english_small(w: &str) -> Option<u64> {
    const SMALL: [&str; 20] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen",
        "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    ];
    const TENS: [&str; 8] = ["twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    SMALL
        .iter()
        .position(|s| *s == w)
        .map(|i| i as u64)
        .or_else(|| TENS.iter().position(|s| *s == w).map(|i| 20 + 10 * i as u64))
        .or_else(|| (w == "a").then_some(1))
}

fn english_words(g: &str) -> Option<u64> {
    let toks: Vec<&str> = g.split(|c: char| c.is_whitespace() || c == '-').filter(|t| !t.is_empty() && *t != "and").collect();
    (!toks.is_empty()).then_some(())?;
    let mut total = 0u64;
    let mut cur = 0u64;
    for t in toks {
        match t {
            "hundred" => cur = cur.max(1) * 100,
            "thousand" => {
                total += cur.max(1) * 1_000;
                cur = 0;
            }
            "million" => {
                total += cur.max(1) * 1_000_000;
                cur = 0;
            }
            _ => cur += english_small(t)?,
        }
    }
    Some(total + cur)
}

/// Vlera e një glose angleze që është vetëm numër: «twenty-one», «one hundred», «1941».
fn english_number(gloss: &str) -> Option<u64> {
    let g = gloss.split('(').next().unwrap_or("").trim().trim_end_matches(['.', ',', ';']).to_lowercase();
    (!g.is_empty() && !g.starts_with('-')).then_some(())?;
    g.parse::<u64>().ok().or_else(|| english_words(&g))
}

fn english_ordinal(gloss: &str) -> Option<u64> {
    const ORD: [&str; 20] = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "eleventh", "twelfth",
        "thirteenth", "fourteenth", "fifteenth", "sixteenth", "seventeenth", "eighteenth", "nineteenth", "twentieth",
    ];
    let g = gloss.trim().to_lowercase();
    ORD.iter().position(|o| *o == g).map(|i| i as u64 + 1).or_else(|| match g.as_str() {
        "hundredth" => Some(100),
        "thousandth" => Some(1_000),
        _ => None,
    })
}

#[derive(Debug, Default)]
struct Entry {
    word: String,
    pos: String,
    canonical: String,
    glosses: Vec<(String, String)>,
    forms: Vec<(String, String)>,
    related: Vec<(String, String)>,
}

fn kaikki() -> Vec<Entry> {
    let text = std::fs::read_to_string(Path::new(DATA).join("raw/kaikki_en/kaikki_en-sq.tsv")).unwrap();
    let mut out: Vec<Entry> = Vec::new();
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        let f = |i: usize| c.get(i).copied().unwrap_or("").to_string();
        let kind = c.first().copied().unwrap_or("");
        match (kind, out.last_mut()) {
            ("E", _) => out.push(Entry { word: f(1), pos: f(2), canonical: f(4), ..Default::default() }),
            ("G", Some(e)) => e.glosses.push((f(2), f(3))),
            ("F", Some(e)) => e.forms.push((f(1), f(2))),
            ("R", Some(e)) => e.related.push((f(2), f(3))),
            _ => {}
        }
    }
    out
}

fn values(e: &Entry) -> BTreeSet<u64> {
    e.glosses.iter().filter_map(|(_, g)| english_number(g)).collect()
}

/// Çelësi që bashkon shkrimin e ndarë me atë të bashkuar: pa hapësira dhe pa «ë».
fn key(w: &str) -> String {
    w.chars().filter(|c| *c != ' ' && *c != 'ë').collect()
}

/// Temat e mundshme të një fjale rreshtore të arit: pa «-të», pa «-ë», pa «-t» pas «-r».
fn gold_ordinal_stems(w: &str) -> Vec<String> {
    let te = w.strip_suffix("të").map(|b| [b.to_string(), format!("{b}ë")]).into_iter().flatten();
    let e = w.strip_suffix('ë').map(str::to_string);
    let t = w.strip_suffix('t').filter(|b| b.ends_with('r')).map(str::to_string);
    te.chain(e).chain(t).collect()
}

struct Gold {
    words: BTreeMap<String, BTreeSet<u64>>,
    alternatives: BTreeMap<String, BTreeSet<u64>>,
    ordinals: Vec<(String, u64, Gender, String)>,
}

fn gold(entries: &[Entry]) -> Gold {
    let nums: Vec<&Entry> = entries.iter().filter(|e| e.pos == "num" && !values(e).is_empty()).collect();
    let mut words: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut by_key: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for e in &nums {
        words.entry(e.word.clone()).or_default().extend(values(e));
        by_key.entry(key(&e.word)).or_default().extend(values(e));
    }
    for e in &nums {
        for (rel, w) in &e.related {
            let hit = by_key.get(&key(w)).filter(|_| rel == "RELATED").cloned();
            match hit {
                Some(v) => words.entry(w.clone()).or_default().extend(v),
                None => {}
            }
        }
    }
    let mut alternatives: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for e in &nums {
        for (form, tags) in &e.forms {
            match tags.contains("alternative") && !words.contains_key(form) {
                true => alternatives.entry(form.clone()).or_default().extend(values(e)),
                false => {}
            }
        }
    }
    let num_words: BTreeSet<&str> = entries.iter().filter(|e| e.pos == "num").map(|e| e.word.as_str()).collect();
    let mut ordinals: Vec<(String, u64, Gender, String)> = Vec::new();
    for e in entries.iter().filter(|e| e.pos == "adj") {
        let n = match e.glosses.iter().find_map(|(_, g)| english_ordinal(g)) {
            Some(n) => n,
            None => continue,
        };
        let canonical = match e.canonical.is_empty() {
            true => format!("i {}", e.word),
            false => e.canonical.clone(),
        };
        ordinals.push((e.word.clone(), n, Gender::Masc, canonical));
        for (form, tags) in &e.forms {
            let fem_sg = tags.split(',').any(|t| t == "feminine") && !tags.contains("plural");
            match fem_sg {
                true => ordinals.push((e.word.clone(), n, Gender::Fem, form.clone())),
                false => {}
            }
        }
    }
    let mut numeric: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.pos == "num" || e.pos == "noun") {
        let vs = values(e);
        match vs.is_empty() {
            true => {}
            false => numeric.entry(key(&e.word)).or_default().extend(vs),
        }
    }
    for e in entries {
        for (rel, w) in &e.related {
            let link = (rel == "RELATED" || rel == "DERIVED")
                && !w.contains(' ')
                && (w.ends_with("të") || w.ends_with("rt"))
                && !num_words.contains(w.as_str())
                && !words.contains_key(w)
                && !ordinals.iter().any(|(l, _, _, _)| l == w);
            let vs: BTreeSet<u64> =
                gold_ordinal_stems(w).iter().filter_map(|s| numeric.get(&key(s))).flatten().copied().collect();
            let single = (vs.len() == 1).then(|| vs.iter().next().copied()).flatten();
            match (link, single) {
                (true, Some(n)) => ordinals.push((w.clone(), n, Gender::Masc, format!("i {w}"))),
                _ => {}
            }
        }
    }
    Gold { words, alternatives, ordinals }
}

#[derive(Debug, Clone)]
struct Tok {
    id: String,
    form: String,
    lemma: String,
    upos: String,
    feats: String,
    head: String,
}

fn ud_sentences() -> Vec<Vec<Tok>> {
    let mut out: Vec<Vec<Tok>> = Vec::new();
    for file in UD {
        let text = std::fs::read_to_string(Path::new(DATA).join(file)).unwrap();
        let mut cur: Vec<Tok> = Vec::new();
        for line in text.lines() {
            let c: Vec<&str> = line.split('\t').collect();
            match (line.trim().is_empty(), c.len() == 10 && !line.starts_with('#')) {
                (true, _) => out.push(std::mem::take(&mut cur)),
                (false, true) => cur.push(Tok {
                    id: c[0].to_string(),
                    form: c[1].to_string(),
                    lemma: c[2].to_string(),
                    upos: c[3].to_string(),
                    feats: c[5].to_string(),
                    head: c[6].to_string(),
                }),
                _ => {}
            }
        }
        out.push(cur);
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

fn accepts_any(n: u64, form: &str, genders: &[Gender]) -> bool {
    genders.iter().any(|g| cardinal_form(n, *g).is_some_and(|f| f.accepts(form)))
}

fn min_value(vs: &BTreeSet<u64>) -> u64 {
    vs.iter().next().copied().unwrap_or(0)
}

/// Kthimi parse(cardinal(n)) == n në të dy gjinitë; numëron n-të që dështojnë.
fn roundtrip_failures(ns: impl Iterator<Item = u64>) -> (u64, u64) {
    let mut checked = 0u64;
    let mut failed = 0u64;
    for n in ns {
        let ok = [Gender::Masc, Gender::Fem].iter().all(|g| cardinal(n, *g).as_deref().and_then(parse) == Some(n));
        checked += 1;
        failed += u64::from(!ok);
    }
    (checked, failed)
}

#[test]
fn gold_metrics() {
    let entries = kaikki();
    let g = gold(&entries);
    let mut dev_misses: Vec<String> = Vec::new();
    let mut dev_holds: Vec<String> = Vec::new();

    let mut m_parse = Metric::default();
    let mut groups: BTreeMap<(u64, String), (bool, bool, bool)> = BTreeMap::new();
    for (w, vs) in &g.words {
        let r = reading(w);
        let ok = r.is_some_and(|r| vs.contains(&r.value));
        let gold_vs: Vec<u64> = vs.iter().copied().collect();
        let tab = touches_table(r, &gold_vs);
        let v = min_value(vs);
        m_parse.add(split_value(v), tab, r.is_some(), ok);
        let e = groups.entry((v, key(w))).or_insert((false, true, false));
        e.0 |= r.is_some();
        e.1 &= r.is_none() || ok;
        e.2 |= tab;
        match (split_value(v), r.is_some() && !ok, r.is_none()) {
            (Split::Dev, true, _) => dev_misses.push(format!("parse {w} → {r:?}, ari {vs:?}")),
            (Split::Dev, _, true) => dev_holds.push(format!("{w} {vs:?}")),
            _ => {}
        }
    }
    let mut m_parse_dedup = Metric::default();
    for ((v, _), (answered, correct, tab)) in &groups {
        m_parse_dedup.add(split_value(*v), *tab, *answered, *correct);
    }

    let mut m_alt = Metric::default();
    for (w, vs) in &g.alternatives {
        let r = reading(w);
        let ok = r.is_some_and(|r| vs.contains(&r.value));
        let gold_vs: Vec<u64> = vs.iter().copied().collect();
        m_alt.add(split_value(min_value(vs)), touches_table(r, &gold_vs), r.is_some(), ok);
        match (split_value(min_value(vs)), r.is_some() && !ok) {
            (Split::Dev, true) => dev_misses.push(format!("parse-alt {w} → {r:?}, ari {vs:?}")),
            _ => {}
        }
    }

    let mut by_value: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    for (w, vs) in &g.words {
        for v in vs {
            by_value.entry(*v).or_default().insert(w.clone());
        }
    }
    let mut m_card = Metric::default();
    for (n, ws) in &by_value {
        let forms: Vec<_> = [Gender::Masc, Gender::Fem].iter().filter_map(|gd| cardinal_form(*n, *gd)).collect();
        let ok = forms.iter().any(|f| ws.contains(&f.text) || f.variants.iter().any(|v| ws.contains(v)));
        let bare = |v: &String| BARE.iter().any(|(_, b, _)| b == v);
        let ok_plain = forms.iter().any(|f| ws.contains(&f.text) || f.variants.iter().any(|v| !bare(v) && ws.contains(v)));
        let tab = has_irregular(*n) || (ok && !ok_plain);
        m_card.add(split_value(*n), tab, !forms.is_empty(), ok);
        match (split_value(*n), !forms.is_empty() && !ok) {
            (Split::Dev, true) => dev_misses.push(format!("cardinal {n} → {:?}, ari {ws:?}", forms.first().map(|f| f.text.clone()))),
            _ => {}
        }
    }

    let mut m_ud = Metric::default();
    let mut ud_alpha = 0u64;
    let mut ud_number = 0u64;
    let mut ud_card = 0u64;
    let mut ud_three: BTreeMap<&str, u64> = BTreeMap::new();
    for s in ud_sentences() {
        for t in s.iter().filter(|t| t.upos == "NUM" && t.form.chars().any(char::is_alphabetic)) {
            let form = t.form.to_lowercase();
            let head_gender = s.iter().find(|h| h.id == t.head).and_then(|h| ud_feat(&h.feats, "Gender")).and_then(ud_gender);
            let genders: Vec<Gender> = head_gender.map(|gd| vec![gd]).unwrap_or_else(|| vec![Gender::Masc, Gender::Fem]);
            let gold_v: Vec<u64> = g.words.get(&t.lemma).map(|vs| vec![min_value(vs)]).unwrap_or_default();
            let split = gold_v.first().map(|v| split_value(*v)).unwrap_or_else(|| split_of(&t.lemma));
            let r = reading(&form);
            let ok = r.is_some_and(|r| accepts_any(r.value, &form, &genders));
            m_ud.add(split, touches_table(r, &gold_v), r.is_some(), ok);
            ud_alpha += 1;
            ud_number += u64::from(ud_feat(&t.feats, "Number").is_some());
            ud_card += u64::from(ud_feat(&t.feats, "NumType") == Some("Card"));
            let three = matches!(form.split(' ').last(), Some("tre" | "tri"));
            let label = match split {
                Split::Dev => "Dev",
                Split::Test => "Test",
            };
            *ud_three.entry(label).or_default() += u64::from(three);
            match (split, r.is_some() && !ok) {
                (Split::Dev, true) => dev_misses.push(format!("ud {form} → {r:?}, gjinia {head_gender:?}")),
                _ => {}
            }
        }
    }

    let ordinal_by_lemma: BTreeMap<String, u64> = g.ordinals.iter().map(|(l, n, _, _)| (l.clone(), *n)).collect();
    let mut m_ord = Metric::default();
    let mut m_ord_parse = Metric::default();
    let mut ord_values: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for (_, n, gd, form) in &g.ordinals {
        let split = split_value(*n);
        let label = match split {
            Split::Dev => "Dev",
            Split::Test => "Test",
        };
        ord_values.entry(label).or_default().insert(*n);
        let r = ordinal_reading(form);
        m_ord_parse.add(split, touches_table(r, &[*n]), r.is_some(), r.map(|r| r.value) == Some(*n));
        match (split, r.is_some_and(|r| r.value != *n)) {
            (Split::Dev, true) => dev_misses.push(format!("parse-ordinal {form} → {r:?}, ari {n}")),
            _ => {}
        }
        let got = ordinal_form(*n, *gd);
        let ok = got.as_ref().is_some_and(|f| f.accepts(form));
        let bare = |v: &String| BARE_ORDINAL.iter().any(|(_, b, _)| v.ends_with(&format!(" {b}")));
        let ok_plain = got.as_ref().is_some_and(|f| f.text == *form || f.variants.iter().any(|v| !bare(v) && v == form));
        let tab = *n == 1 || has_irregular(*n) || (ok && !ok_plain);
        m_ord.add(split, tab, got.is_some(), ok);
        match (split, got.is_some() && !ok) {
            (Split::Dev, true) => dev_misses.push(format!("ordinal {n} {gd:?} → {got:?}, ari {form}")),
            _ => {}
        }
    }
    for s in ud_sentences() {
        for t in s.iter().filter(|t| t.upos == "ADJ" && ud_feat(&t.feats, "Number") == Some("Sing")) {
            let n = ordinal_by_lemma.get(&t.lemma).copied();
            let gd = ud_feat(&t.feats, "Gender").and_then(ud_gender);
            match (n, gd) {
                (Some(n), Some(gd)) => {
                    let split = split_value(n);
                    let got = ordinal(n, gd).and_then(|o| o.split_once(' ').map(|(_, w)| w.to_string()));
                    let form = t.form.to_lowercase();
                    let ok = got.as_deref() == Some(form.as_str());
                    m_ord.add(split, n == 1 || has_irregular(n), got.is_some(), ok);
                    let r = ordinal_reading(&form);
                    m_ord_parse.add(split, touches_table(r, &[n]), r.is_some(), r.map(|r| r.value) == Some(n));
                    match (split, got.is_some() && !ok) {
                        (Split::Dev, true) => dev_misses.push(format!("ud-ordinal {form} → {got:?}")),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    let chunks: Vec<(u64, u64)> = (0..8u64).map(|i| (i * 125_000, (i + 1) * 125_000 + u64::from(i == 7))).collect();
    let (round, wide) = std::thread::scope(|sc| {
        let jobs: Vec<_> = chunks
            .iter()
            .map(|&(lo, hi)| {
                sc.spawn(move || {
                    let round = roundtrip_failures(lo..hi);
                    let wide = roundtrip_failures(((lo / 5)..(hi / 5)).map(|k| k.wrapping_mul(4_999_999_937) % (MAX + 1)));
                    (round, wide)
                })
            })
            .collect();
        jobs.into_iter().map(|j| j.join().unwrap()).fold(((0, 0), (0, 0)), |(r, w), ((rc, rf), (wc, wf))| {
            ((r.0 + rc, r.1 + rf), (w.0 + wc, w.1 + wf))
        })
    });

    let metrics = [
        ("parse/wiktionary", m_parse),
        ("parse/wiktionary-dedup", m_parse_dedup),
        ("parse/wiktionary-alt", m_alt),
        ("cardinal/wiktionary", m_card),
        ("parse+gender/ud-num", m_ud),
        ("ordinal/wiktionary+ud", m_ord),
        ("parse-ordinal/wikt+ud", m_ord_parse),
    ];
    let tables = IRREGULAR.len() + 1 + BARE.len() + BARE_ORDINAL.len();
    eprintln!("{}", table(&metrics));
    eprintln!(
        "tables: IRREGULAR {} + FIRST 1 + BARE {} + BARE_ORDINAL {} = {tables} entries",
        IRREGULAR.len(),
        BARE.len(),
        BARE_ORDINAL.len()
    );
    eprintln!("ordinal gold values: {ord_values:?}; lemmas {:?}", ordinal_by_lemma);
    eprintln!("ud NUM alphabetic {ud_alpha}: with Number {ud_number}, with NumType=Card {ud_card}; tre/tri last (gender test) {ud_three:?}");
    eprintln!("property roundtrip 0..=1e6 (each n once, Masc+Fem): checked {}, failed {}", round.0, round.1);
    eprintln!("property roundtrip sample..=MAX (each n once, Masc+Fem): checked {}, failed {}", wide.0, wide.1);
    eprintln!("dev HOLD (parse/wiktionary): {dev_holds:?}");
    eprintln!("dev mismatches: {}", dev_misses.len());
    for m in dev_misses.iter().take(40) {
        eprintln!("  {m}");
    }

    assert!(m_parse.test.precision() >= 0.99 && m_parse.test.coverage() >= 0.95 && m_parse.test.wilson() >= 0.93, "{:?}", m_parse.test);
    assert!(m_parse.dev.precision() >= 0.99 && m_parse.dev.coverage() >= 0.85, "{:?}", m_parse.dev);
    assert!(m_parse_dedup.test.precision() >= 0.99 && m_parse_dedup.test.wilson() >= 0.85, "{:?}", m_parse_dedup.test);
    assert!(m_alt.dev.correct == m_alt.dev.answered && m_alt.test.correct == m_alt.test.answered, "{:?}", m_alt);
    assert!(m_card.test.precision() >= 0.99 && m_card.test.coverage() >= 0.99 && m_card.test.wilson() >= 0.89, "{:?}", m_card.test);
    assert!(m_ud.test.precision() >= 0.99 && m_ud.test.coverage() >= 0.95 && m_ud.dev.precision() >= 0.99 && m_ud.dev.coverage() >= 0.70, "{:?}", m_ud);
    assert!(m_ord.test.precision() >= 0.99 && m_ord.test.coverage() >= 0.99 && m_ord.dev.precision() >= 0.99 && m_ord.dev.coverage() >= 0.99, "{:?}", m_ord);
    assert!(m_ord_parse.test.precision() >= 0.99 && m_ord_parse.test.coverage() >= 0.99 && m_ord_parse.dev.precision() >= 0.99, "{:?}", m_ord_parse);
    assert!(m_parse.test_regular.precision() >= 0.99 && m_parse.test_regular.coverage() >= 0.90 && m_card.test_regular.precision() >= 0.99);
    assert_eq!(round, (1_000_001, 0));
    assert_eq!(wide, (200_000, 0));
}

#[test]
fn witness_agreement() {
    let lx = Lexicon::load(Path::new(DATA)).unwrap();
    let mut t = Tally::default();
    let mut missing: Vec<String> = Vec::new();
    for n in 1..=1_000_000u64 {
        let got = ordinal_in(&lx, n, Gender::Masc);
        let attested = got.as_ref().is_some_and(|f| f.rule == R_ORD_WITNESS);
        t.add(got.is_some(), attested);
        match (got, attested) {
            (Some(f), false) => missing.push(f.text),
            _ => {}
        }
    }
    eprintln!(
        "witness (uniparser) ordinals 1..=1e6: answered {} attested {} ({:.4}); not in lexicon, first 20: {:?}",
        t.answered,
        t.correct,
        t.precision(),
        missing.iter().take(20).collect::<Vec<_>>()
    );
    assert!(t.answered >= 120 && t.precision() >= 0.99);

    let num: Vec<_> = lx.iter().filter(|l| l.pos == Pos::Num).collect();
    let std_lemmas: BTreeSet<&str> = num.iter().filter(|l| l.standard).map(|l| l.lemma.as_str()).collect();
    let atoms: Vec<String> = (1..=19u64).chain((2..=9).map(|t| t * 10)).filter_map(|n| cardinal(n, Gender::Masc)).collect();
    let unattested: Vec<&String> = atoms.iter().filter(|a| !std_lemmas.contains(a.as_str())).collect();
    let nonstandard: Vec<String> =
        num.iter().filter(|l| !l.standard).filter_map(|l| l.clean_stems().first().and_then(|s| s.first()).cloned()).collect();
    let misread: Vec<(&String, Option<u64>)> = nonstandard.iter().map(|w| (w, parse(w))).filter(|(_, v)| v.is_some()).collect();
    let noun = |lemma: &str| lx.standard(lemma, Pos::Noun);
    let plural = |lemma: &str| noun(lemma).map(|l| l.clean_stems()).and_then(|s| s.first().cloned()).unwrap_or_default();
    let scale_gender: Vec<(&str, Option<Gender>)> =
        ["mijë", "milion", "miliard"].iter().map(|w| (*w, noun(w).and_then(|l| l.gender_sg))).collect();
    let three_thousandth: Vec<bool> = ["trimijtë", "tremijtë"].iter().map(|w| lx.entries(w).iter().any(|l| l.pos == Pos::Adj && l.standard)).collect();
    eprintln!(
        "witness (uniparser) cardinal atoms: {}, not std NUM: {} {:?}; nonstandard NUM stems {:?}, read as numbers {:?}; plural milion {:?}, miliard {:?}; scale gender {:?}; std trimijtë/tremijtë {:?}",
        atoms.len(),
        unattested.len(),
        unattested,
        nonstandard,
        misread,
        plural("milion"),
        plural("miliard"),
        scale_gender,
        three_thousandth
    );
    assert!(unattested.is_empty() && misread.is_empty());
    assert_eq!(plural("milion"), vec!["milionë".to_string(), "miliona".to_string()]);
    assert_eq!(plural("miliard"), vec!["miliardë".to_string(), "miliarda".to_string()]);
    assert_eq!(scale_gender, vec![("mijë", Some(Gender::Fem)), ("milion", Some(Gender::Masc)), ("miliard", Some(Gender::Masc))]);
    assert_eq!(three_thousandth, vec![true, true]);
    for w in ["trezet", "katërzet", "pesëzet"] {
        assert!(lx.entries(w).is_empty(), "{w}");
    }
}
