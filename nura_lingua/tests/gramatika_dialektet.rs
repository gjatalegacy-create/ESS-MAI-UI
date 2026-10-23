//! Matja e dialekteve kundrejt arit: Wiktionary (trajta alternative gege, toske, arbëreshe me
//! lemën standarde) dhe rreshtat jostandardë të uniparser-it me «lexref». Ndarja sipas lemës
//! standarde; rregullat ndërtohen vetëm mbi `Dev`, saktësia raportohet edhe për `Test`.
//! Ari negativ: trajta standarde të lakuara (Wiktionary, UD), ku çdo përgjigje tjetër nga lema
//! e vet është gabim.

use nura_lingua::gramatika::dialektet::{
    fold, in_irregular, infinitive_to_gheg, irregular_len, scored, to_gheg, to_standard, Candidate, Normalizer, CLUSTER_MB, CLUSTER_ND, DIPHTHONG,
    F_H, MIN_N, MIN_WILSON, MONOPHTHONG, NASAL, NASAL_FINAL, NASAL_LOW, NASAL_NJ, NJI, NORTH_PALATAL, O_A, PART_SHORT, PART_UE, PART_UEM, PART_UN,
    PART_YE, PROTHESIS, ROTACISM, ROTACISM_SCHWA, SCHWA, SCHWA_A, SPELLING, SPELLING_Z, TABLE, TOSK_SCHWA, UNROUNDING, VELAR, VERB_NJ, WEAK,
};
use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::tipet::{split_of, Pos, Split};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");

/// Rregullat e drejtimit dialekt → standard, për tabelën e qëndrueshmërisë.
const RULES: [&str; 28] = [
    ROTACISM,
    ROTACISM_SCHWA,
    NASAL,
    NASAL_LOW,
    NASAL_FINAL,
    NASAL_NJ,
    DIPHTHONG,
    MONOPHTHONG,
    PART_UN,
    PART_UEM,
    PART_UE,
    PART_SHORT,
    PART_YE,
    SCHWA,
    SCHWA_A,
    CLUSTER_MB,
    CLUSTER_ND,
    F_H,
    O_A,
    UNROUNDING,
    VELAR,
    NORTH_PALATAL,
    NJI,
    VERB_NJ,
    TOSK_SCHWA,
    SPELLING,
    SPELLING_Z,
    PROTHESIS,
];

/// Rregullat që nuk gjykohen me pragun mbi `Dev`: tabela e mbyllur dhe pjesoret, që i dëshmon
/// klasa e foljes në leksik.
const EXEMPT: [&str; 6] = [TABLE, PART_UN, PART_UEM, PART_UE, PART_SHORT, PART_YE];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    source: &'static str,
    variant: String,
    standard: String,
}

/// Vetëm shkronja latine (me shenjat kombinuese): pa fraza, prapashtesa a shkrim grek.
fn latin_word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| (c.is_alphabetic() && (c < '\u{370}' || ('\u{1e00}'..='\u{1eff}').contains(&c))) || ('\u{300}'..='\u{36f}').contains(&c))
}

fn kaikki() -> String {
    std::fs::read_to_string(Path::new(DATA).join("raw/kaikki_en/kaikki_en-sq.tsv")).unwrap()
}

fn wiktionary(text: &str) -> Vec<Item> {
    let mut head: Option<String> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match c.first().copied() {
            Some("E") => head = c.get(1).map(|s| s.to_string()),
            Some("F") => {
                let tags: Vec<&str> = c.get(2).copied().unwrap_or("").split(',').collect();
                let alt = tags.contains(&"alternative") && !tags.contains(&"Standard");
                let source = ["Gheg", "Tosk", "Arbëresh"].iter().find(|d| tags.contains(d)).map(|d| match *d {
                    "Gheg" => "wikt-gegë",
                    "Tosk" => "wikt-toskë",
                    _ => "wikt-arbëresh",
                });
                match (alt, source, head.as_ref(), c.get(1)) {
                    (true, Some(source), Some(h), Some(v)) => out.push(Item { source, variant: v.to_string(), standard: h.clone() }),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

fn uniparser() -> Vec<Item> {
    let text = std::fs::read_to_string(Path::new(DATA).join("raw/uniparser/uniparser-sq.tsv")).unwrap();
    text.lines()
        .map(|l| l.split('\t').collect::<Vec<&str>>())
        .filter(|c| c.len() == 18 && c[0] != "lex" && !c[0].starts_with('#'))
        .filter_map(|c| {
            let source = match c[10].trim() {
                "nonst" => Some("uni-nonst"),
                "gheg" => Some("uni-gegë"),
                _ => None,
            }?;
            let r = c[13].trim();
            (!r.is_empty() && r != "-").then(|| Item { source, variant: c[1].trim().to_string(), standard: r.to_string() })
        })
        .collect()
}

fn gold(text: &str) -> (Vec<Item>, usize) {
    let raw: BTreeSet<Item> = wiktionary(text).into_iter().chain(uniparser()).collect();
    let n = raw.len();
    let kept: Vec<Item> = raw.into_iter().filter(|i| latin_word(&i.variant) && latin_word(&i.standard) && fold(&i.variant) != fold(&i.standard)).collect();
    let dropped = n - kept.len();
    (kept, dropped)
}

/// Etiketat gramatikore të Wiktionary-t që shënojnë një trajtë të lakuar standarde.
const GRAMMATICAL: [&str; 41] = [
    "plural",
    "singular",
    "perfect",
    "indicative",
    "third-person",
    "second-person",
    "first-person",
    "definite",
    "present",
    "indefinite",
    "imperfect",
    "admirative",
    "subjunctive",
    "past",
    "accusative",
    "nominative",
    "conditional",
    "future",
    "optative",
    "dative",
    "ablative",
    "aorist",
    "imperative",
    "participle",
    "feminine",
    "masculine",
    "neuter",
    "infinitive",
    "gerund",
    "adjectival",
    "progressive",
    "future-i",
    "future-ii",
    "pluperfect",
    "past-ii",
    "past-i",
    "plural-only",
    "negative",
    "active",
    "passive",
    "genitive",
];

/// Trajtat standarde të lakuara të Wiktionary-t: (trajta, lema), pa lemat e leksikut.
fn standard_forms(text: &str, lex: &Lexicon, variants: &BTreeSet<String>) -> BTreeSet<(String, String)> {
    let mut head: Option<String> = None;
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match c.first().copied() {
            Some("E") => head = c.get(1).map(|s| s.to_string()),
            Some("F") => {
                let tags: Vec<&str> = c.get(2).copied().unwrap_or("").split(',').collect();
                let grammatical = !tags.is_empty() && tags.iter().all(|t| GRAMMATICAL.contains(t));
                match (grammatical, head.as_ref(), c.get(1)) {
                    (true, Some(h), Some(f)) => {
                        let keep = latin_word(f) && fold(f) != fold(h) && lex.entries(f).is_empty() && !variants.contains(&fold(f));
                        match keep {
                            true => {
                                out.insert((f.to_string(), h.clone()));
                            }
                            false => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

/// Fjalët e UD-së (trajta, lema) që nuk janë lema të leksikut, me numrin e shfaqjeve.
fn ud_tokens(lex: &Lexicon, variants: &BTreeSet<String>) -> BTreeMap<(String, String), usize> {
    let files = ["raw/ud_sq_staf/sq_staf-ud-train.conllu", "raw/ud_sq_staf/sq_staf-ud-dev.conllu", "raw/ud_sq_staf/sq_staf-ud-test.conllu", "raw/ud_sq_tsa/sq_tsa-ud-test.conllu"];
    let mut out: BTreeMap<(String, String), usize> = BTreeMap::new();
    for f in files {
        let text = std::fs::read_to_string(Path::new(DATA).join(f)).unwrap();
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let c: Vec<&str> = line.split('\t').collect();
            let token = c.len() == 10 && c[0].chars().all(|ch| ch.is_ascii_digit());
            match token {
                true => {
                    let (form, lemma) = (c[1], c[2]);
                    let keep = latin_word(form) && lex.entries(form).is_empty() && lex.entries(&form.to_lowercase()).is_empty() && !variants.contains(&fold(form));
                    match keep {
                        true => *out.entry((form.to_string(), lemma.to_string())).or_default() += 1,
                        false => {}
                    }
                }
                false => {}
            }
        }
    }
    out
}

/// Kufiri i poshtëm Wilson 95%: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n), z = 1.96.
fn wilson(correct: usize, n: usize) -> f64 {
    match n {
        0 => 0.0,
        _ => {
            let (z, nf) = (1.96f64, n as f64);
            let p = correct as f64 / nf;
            (p + z * z / (2.0 * nf) - z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt()) / (1.0 + z * z / nf)
        }
    }
}

/// Kufiri i sipërm Wilson 95% i një përqindjeje gabimesh: 1 − kufiri i poshtëm i së kundërtës.
fn wilson_upper(bad: usize, n: usize) -> f64 {
    match n {
        0 => 1.0,
        _ => 1.0 - wilson(n - bad, n),
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    total: usize,
    answered: usize,
    correct: usize,
    anywhere: usize,
}

impl Tally {
    fn add(&mut self, answered: bool, correct: bool, anywhere: bool) {
        self.total += 1;
        self.answered += usize::from(answered);
        self.correct += usize::from(correct);
        self.anywhere += usize::from(anywhere);
    }
    fn precision(&self) -> f64 {
        ratio(self.correct, self.answered)
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    match b {
        0 => 0.0,
        _ => a as f64 / b as f64,
    }
}

fn row(name: &str, t: &Tally) -> String {
    format!(
        "{:<34} {:>6} {:>6} {:>6} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3}",
        name,
        t.total,
        t.answered,
        t.correct,
        t.precision(),
        ratio(t.answered, t.total),
        wilson(t.correct, t.answered),
        ratio(t.correct, t.total),
        ratio(t.anywhere, t.total)
    )
}

fn header(cols: [&str; 9]) -> String {
    format!("{:<34} {:>6} {:>6} {:>6} {:>7} {:>7} {:>7} {:>7} {:>7}", cols[0], cols[1], cols[2], cols[3], cols[4], cols[5], cols[6], cols[7], cols[8])
}

fn split_name(s: Split) -> &'static str {
    match s {
        Split::Dev => "dev",
        Split::Test => "test",
    }
}

type Rows = BTreeMap<(String, &'static str), Tally>;

fn get(m: &Rows, k: &str, s: &'static str) -> Tally {
    m.get(&(k.to_string(), s)).copied().unwrap_or_default()
}

#[test]
fn dialect_gold() {
    let lex = Lexicon::load(Path::new(DATA)).unwrap();
    let nz = Normalizer::new(&lex);
    let text = kaikki();
    let (items, dropped) = gold(&text);
    let dump = std::env::var("DIALEKTET_DEV").is_ok();

    // Ari i një varianti: lemat standarde të të njëjtës ndarje (nga çdo burim).
    let mut golds: BTreeMap<(String, &'static str), BTreeSet<String>> = BTreeMap::new();
    for i in &items {
        golds.entry((i.variant.clone(), split_name(split_of(&i.standard)))).or_default().insert(i.standard.to_lowercase());
    }
    // Gjykimi i leksikut: lema standarde që leksiku i shënon një lemë ari jostandarde (potkua → patkua)
    // ose vetë variantit (shuell → shuall). Raportohet veçmas; rreshtat kryesorë nuk e përdorin.
    let lexref = |w: &str| -> Option<String> { lex.entries(w).iter().filter(|l| !l.standard).find_map(|l| l.lexref.clone()) };

    let variants: BTreeSet<String> = items.iter().map(|i| i.variant.clone()).collect();
    let answers: BTreeMap<String, Vec<Candidate>> = variants.iter().map(|v| (v.clone(), nz.to_standard(v))).collect();
    let everything: BTreeMap<String, Vec<(u32, Candidate)>> = variants.iter().map(|v| (v.clone(), scored(v, &lex))).collect();
    let pairs: BTreeSet<(String, String)> = items.iter().map(|i| (i.variant.clone(), i.standard.clone())).collect();

    let mut norm: Rows = BTreeMap::new();
    let mut broad: Rows = BTreeMap::new();
    let mut table_free: Rows = BTreeMap::new();
    let mut judged: Rows = BTreeMap::new();

    for i in &items {
        let split = split_name(split_of(&i.standard));
        let g = golds.get(&(i.variant.clone(), split)).cloned().unwrap_or_default();
        let cands = answers.get(&i.variant).cloned().unwrap_or_default();
        let top_ok = cands.first().map(|c| g.contains(&c.text.to_lowercase())).unwrap_or(false);
        let any_ok = cands.iter().any(|c| g.contains(&c.text.to_lowercase()));
        norm.entry((i.source.to_string(), split)).or_default().add(!cands.is_empty(), top_ok, any_ok);
    }
    for (variant, standard) in &pairs {
        let split = split_name(split_of(standard));
        let g = golds.get(&(variant.clone(), split)).cloned().unwrap_or_default();
        let hit = |t: &str| g.contains(&t.to_lowercase());
        let cands = answers.get(variant).cloned().unwrap_or_default();
        let top_ok = cands.first().map(|c| hit(&c.text)).unwrap_or(false);
        let any_ok = cands.iter().any(|c| hit(&c.text));
        norm.entry(("të gjitha".to_string(), split)).or_default().add(!cands.is_empty(), top_ok, any_ok);
        match in_irregular(variant) || in_irregular(standard) {
            true => table_free.entry(("me tabelë".to_string(), split)).or_default().add(!cands.is_empty(), top_ok, any_ok),
            false => table_free.entry(("pa tabelë".to_string(), split)).or_default().add(!cands.is_empty(), top_ok, any_ok),
        }
        let extra: BTreeSet<String> = g.iter().filter_map(|s| lexref(s)).chain(lexref(variant)).map(|s| s.to_lowercase()).collect();
        let judged_ok = cands.first().map(|c| hit(&c.text) || extra.contains(&c.text.to_lowercase())).unwrap_or(false);
        judged.entry(("të gjitha, gjykim leksiku".to_string(), split)).or_default().add(!cands.is_empty(), judged_ok, judged_ok);

        let all = everything.get(variant).cloned().unwrap_or_default();
        let b_top = all.first().map(|(_, c)| hit(&c.text)).unwrap_or(false);
        let b_any = all.iter().any(|(_, c)| hit(&c.text));
        broad.entry(("scored, të gjitha".to_string(), split)).or_default().add(!all.is_empty(), b_top, b_any);

        match (dump, split, top_ok) {
            (true, "dev", false) => eprintln!(
                "DEV {variant} -> {:?} | {} | {}",
                g,
                cands.first().map(|c| c.text.as_str()).unwrap_or("-"),
                all.iter().take(3).map(|(k, c)| format!("{} {k} {:?}", c.text, c.rules)).collect::<Vec<_>>().join(" ; ")
            ),
            _ => {}
        }
    }

    // Qëndrueshmëria e rregullave, vetëm mbi `Dev`: përgjigjet që do jepte çdo rregull po të ishte
    // i qëndrueshëm (`to_standard_by` me çdo rregull të pranuar).
    let mut by_rule: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for (variant, standard) in pairs.iter().filter(|(_, s)| split_of(s) == Split::Dev) {
        let g = golds.get(&(variant.clone(), "dev")).cloned().unwrap_or_default();
        let c = nz.to_standard_by(variant, &|_| true);
        match c.first().and_then(|c| c.rules.first().map(|r| (*r, g.contains(&c.text.to_lowercase())))) {
            Some((r, ok)) => {
                let e = by_rule.entry(r).or_default();
                e.0 += 1;
                e.1 += usize::from(ok);
                match (dump, ok) {
                    (true, false) => eprintln!("RULE-ERR {variant} -> {} [{r}] gold {standard}", c.first().map(|c| c.text.as_str()).unwrap_or("-")),
                    _ => {}
                }
            }
            None => {}
        }
    }
    eprintln!("\nQËNDRUESHMËRIA E RREGULLAVE mbi dev (secili rregull i lejuar): i qëndrueshëm kur n ≥ {MIN_N} dhe wilson ≥ {MIN_WILSON}");
    eprintln!("{:<62} {:>4} {:>5} {:>7} {:>7}  {}", "rregulli", "n", "sakt", "prec", "wilson", "gjendja");
    let mut misjudged: Vec<String> = Vec::new();
    for r in RULES.iter().chain([TABLE].iter()) {
        let (n, ok) = by_rule.get(r).copied().unwrap_or_default();
        let status = match WEAK.contains(r) {
            true => "i dobët",
            false => "i qëndrueshëm",
        };
        eprintln!("{:<62} {:>4} {:>5} {:>7.3} {:>7.3}  {}", r, n, ok, ratio(ok, n), wilson(ok, n), status);
        let meets = n >= MIN_N && wilson(ok, n) >= MIN_WILSON;
        match !EXEMPT.contains(r) && meets == WEAK.contains(r) {
            true => misjudged.push(format!("{r}: n {n}, sakt {ok}, wilson {:.3}", wilson(ok, n))),
            false => {}
        }
    }

    let h = header(["metrika / burimi / ndarja", "total", "përgj", "sakt", "prec@1", "mbul", "wilson", "rec@1", "rec@k"]);
    eprintln!("\nNORMALIZIMI to_standard (dialekt → lemë standarde); hequr {dropped} çifte jo-fjalë ose të njëjta; «të gjitha» pa përsëritje (variant, lemë)");
    eprintln!("{h}");
    for ((src, split), t) in &norm {
        eprintln!("{}", row(&format!("{src} {split}"), t));
    }
    eprintln!("\nTabela e parregullt: {} hyrje; çiftet me dhe pa fjalët e tabelës:", irregular_len());
    for ((src, split), t) in &table_free {
        eprintln!("{}", row(&format!("{src} {split}"), t));
    }
    eprintln!("\nGJYKIM LEKSIKU: pranohet edhe lexref-i i leksikut për lemën e arit ose për variantin (vetëm informacion):");
    for ((src, split), t) in &judged {
        eprintln!("{}", row(&format!("{src} {split}"), t));
    }
    eprintln!("\nKRAHASIM: scored pa politikën e mbajtjes (edhe rregullat e dobëta dhe zinxhirët):");
    for ((src, split), t) in &broad {
        eprintln!("{}", row(&format!("{src} {split}"), t));
    }

    // Ari negativ: trajta standarde të lakuara; çdo përgjigje që s'është lema e vet është gabim.
    let forms = standard_forms(&text, &lex, &variants);
    let ud = ud_tokens(&lex, &variants);
    let mut neg: BTreeMap<(String, &'static str), (usize, usize, usize)> = BTreeMap::new();
    let mut seen: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    let mut examples: Vec<String> = Vec::new();
    let mut judge = |form: &str, lemma: &str, weight: usize, key: &str, seen: &mut BTreeMap<String, Vec<Candidate>>| {
        let c = seen.entry(form.to_string()).or_insert_with(|| nz.to_standard(form)).clone();
        let split = split_name(split_of(lemma));
        let wrong = c.first().map(|c| fold(&c.text) != fold(lemma)).unwrap_or(false);
        let e = neg.entry((key.to_string(), split)).or_default();
        e.0 += weight;
        e.1 += weight * usize::from(!c.is_empty());
        e.2 += weight * usize::from(wrong);
        match (wrong, split, examples.len() < 40) {
            (true, "dev", true) => examples.push(format!("{form}→{}", c.first().map(|c| c.text.as_str()).unwrap_or("-"))),
            _ => {}
        }
    };
    for (f, l) in &forms {
        judge(f, l, 1, "wikt trajta standarde", &mut seen);
    }
    for ((f, l), k) in &ud {
        judge(f, l, *k, "UD fjalë (shfaqje)", &mut seen);
    }
    eprintln!("\nARI NEGATIV: trajta standarde të lakuara që nuk janë lema të leksikut; gabim = përgjigje ≠ lema e vet");
    eprintln!("{:<34} {:>7} {:>6} {:>6} {:>8} {:>9}", "burimi / ndarja", "total", "përgj", "gabim", "gabim%", "wilson↑");
    for ((src, split), (n, answered, wrong)) in &neg {
        eprintln!("{:<34} {:>7} {:>6} {:>6} {:>8.4} {:>9.4}", format!("{src} {split}"), n, answered, wrong, ratio(*wrong, *n), wilson_upper(*wrong, *n));
    }
    match dump {
        true => eprintln!("NEG-DEV {}", examples.join(" ")),
        false => {}
    }

    // Prodhimi standard → gegë: a e përmban lista trajtën gege të arit, dhe sa nga trajtat e
    // prodhuara janë të dëshmuara si variante gege të së njëjtës fjalë.
    let mut gheg_of: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for i in items.iter().filter(|i| i.source.ends_with("gegë")) {
        gheg_of.entry(i.standard.clone()).or_default().insert(fold(&i.variant));
    }
    let mut gen: Rows = BTreeMap::new();
    let mut gen_forms: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    let mut gen_sizes: Vec<usize> = Vec::new();
    for i in items.iter().filter(|i| i.source.ends_with("gegë")) {
        let split = split_name(split_of(&i.standard));
        let g = to_gheg(&i.standard);
        gen_sizes.push(g.len());
        let found = g.contains(&fold(&i.variant));
        gen.entry((i.source.to_string(), split)).or_default().add(!g.is_empty(), found, found);
        gen.entry(("gegë gjithsej".to_string(), split)).or_default().add(!g.is_empty(), found, found);
    }
    for (standard, known) in &gheg_of {
        let split = split_name(split_of(standard));
        let g = to_gheg(standard);
        let e = gen_forms.entry(split).or_default();
        e.0 += g.len();
        e.1 += g.iter().filter(|x| known.contains(*x)).count();
    }
    let mean = ratio(gen_sizes.iter().sum(), gen_sizes.len());
    eprintln!("\nPRODHIMI to_gheg (standard → gegë, pa renditje); mesatarisht {mean:.1} trajta për fjalë");
    eprintln!("{}", header(["metrika / burimi / ndarja", "total", "jo-bosh", "gjetur", "gjet|jb", "jb/tot", "wilson", "rec", "rec"]));
    for ((src, split), t) in &gen {
        eprintln!("{}", row(&format!("{src} {split}"), t));
    }
    for (split, (n, known)) in &gen_forms {
        eprintln!("trajta të prodhuara {split}: {n}, të dëshmuara si gege në ar: {known} (saktësia e trajtës {:.3}, wilson {:.3})", ratio(*known, *n), wilson(*known, *n));
    }

    // Paskajorja: konsistenca e dy drejtimeve mbi pjesoret standarde të leksikut, për çdo trajtë
    // gege të prodhuar (për të X → me X' → për të X). Nuk është saktësi kundrejt arit.
    let participles: BTreeSet<String> = lex
        .iter()
        .filter(|l| l.standard && l.pos == Pos::Adj && ["uar", "ur", "yer", "ier"].iter().any(|e| l.lemma.ends_with(e)))
        .map(|l| l.lemma.clone())
        .collect();
    let mut round: Rows = BTreeMap::new();
    for p in &participles {
        let standard = format!("për të {p}");
        for g in infinitive_to_gheg(&standard) {
            let back = nz.infinitive_to_standard(&g).map(|c| c.text);
            let ok = back.as_deref() == Some(standard.as_str());
            round.entry(("paskajore, çdo trajtë gege".to_string(), split_name(split_of(p)))).or_default().add(back.is_some(), ok, ok);
        }
    }
    eprintln!("\nPASKAJORJA, konsistencë (jo saktësi): për të X → me X' → për të X, mbi {} pjesore standarde të leksikut", participles.len());
    eprintln!("{h}");
    for ((src, split), t) in &round {
        eprintln!("{}", row(&format!("{src} {split}"), t));
    }

    assert!(misjudged.is_empty(), "WEAK nuk përputhet me pragun mbi dev: {misjudged:?}");
    let all_test = get(&norm, "të gjitha", "test");
    let uni_gheg_test = get(&norm, "uni-gegë", "test");
    let wikt_gheg_test = get(&norm, "wikt-gegë", "test");
    let free_test = get(&table_free, "pa tabelë", "test");
    let gen_test = get(&gen, "gegë gjithsej", "test");
    let gen_uni_test = get(&gen, "uni-gegë", "test");
    let round_test = get(&round, "paskajore, çdo trajtë gege", "test");
    let (wn, _, ww) = neg.get(&("wikt trajta standarde".to_string(), "test")).copied().unwrap_or_default();
    let (un, _, uw) = neg.get(&("UD fjalë (shfaqje)".to_string(), "test")).copied().unwrap_or_default();
    assert!(all_test.total > 600, "ari duhet të ngarkohet");
    assert!(all_test.precision() >= 0.93, "prec@1 test {:.3}", all_test.precision());
    assert!(free_test.precision() >= 0.93, "prec@1 test pa tabelë {:.3}", free_test.precision());
    assert!(uni_gheg_test.precision() >= 0.97, "uni-gegë prec@1 test {:.3}", uni_gheg_test.precision());
    assert!(wikt_gheg_test.precision() >= 0.88, "wikt-gegë prec@1 test {:.3}", wikt_gheg_test.precision());
    assert!(ratio(all_test.correct, all_test.total) >= 0.12, "rec@1 test");
    assert!(wn > 10_000 && un > 500, "ari negativ duhet të ngarkohet");
    assert!(ratio(ww, wn) <= 0.001, "gabime mbi trajtat standarde të Wiktionary-t {ww}/{wn}");
    assert!(ratio(uw, un) <= 0.005, "gabime mbi fjalët e UD-së {uw}/{un}");
    assert!(ratio(gen_test.correct, gen_test.total) >= 0.30, "prodhimi test");
    assert!(ratio(gen_uni_test.correct, gen_uni_test.total) >= 0.68, "prodhimi uni-gegë test");
    assert!(round_test.precision() >= 0.99, "paskajorja test {:.3}", round_test.precision());
}

/// Funksionet pa tregues (`to_standard`) dhe normalizuesi me tregues japin të njëjtën përgjigje.
#[test]
fn scan_and_index_agree() {
    let lex = Lexicon::load(Path::new(DATA)).unwrap();
    let nz = Normalizer::new(&lex);
    let text = kaikki();
    let (items, _) = gold(&text);
    let variants: BTreeSet<String> = items.iter().map(|i| i.variant.clone()).collect();
    let probes = ["shtëpisë", "mikun", "kthyem", "tim", "dhanë", "Kinës", "vulën", "hapë", "pa", "asht"];
    for v in variants.iter().map(String::as_str).chain(probes) {
        assert_eq!(to_standard(v, &lex), nz.to_standard(v), "{v}");
    }
}
