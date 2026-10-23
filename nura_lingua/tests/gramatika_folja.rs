//! Matja e zgjedhimit kundrejt arit: Wiktionary (kaikki) dhe UniMorph.
//! Ari lexohet vetëm këtu; moduli nuk e sheh kurrë.
//! Rregullat ndërtohen duke parë vetëm gabimet e gjysmës `Dev`; `Test` vetëm raportohet.

use nura_lingua::gramatika::folja::{conjugate_lemma, irregular_table, nonfinite_lemma, verb_specific_classes, Folja};
use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::tipet::{split_of, Form, Mood, NonFinite, Number, Person, Pos, Split, Tense, VerbCell, Voice};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
const KAIKKI: &str = "raw/kaikki_en/kaikki_en-sq.tsv";
const UNIMORPH: &str = "raw/uniparser/unimorph-sqi.tsv";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Cell(VerbCell),
    Nf(NonFinite, Voice),
}

type Gold = BTreeMap<(String, Key), BTreeSet<String>>;

fn person(s: &str) -> Option<Person> {
    match s {
        "first-person" | "1" => Some(Person::P1),
        "second-person" | "2" => Some(Person::P2),
        "third-person" | "3" => Some(Person::P3),
        _ => None,
    }
}

fn number(s: &str) -> Option<Number> {
    match s {
        "singular" | "SG" => Some(Number::Sing),
        "plural" | "PL" => Some(Number::Plur),
        _ => None,
    }
}

fn mood(s: &str) -> Option<Mood> {
    match s {
        "indicative" => Some(Mood::Ind),
        "subjunctive" => Some(Mood::Sub),
        "conditional" => Some(Mood::Cnd),
        "admirative" => Some(Mood::Adm),
        "optative" => Some(Mood::Opt),
        "imperative" => Some(Mood::Imp),
        _ => None,
    }
}

/// Etiketat e Wiktionary-t → qeliza; vetëm shablloni kryesor i zgjedhimit.
fn kaikki_key(tags: &str, voice: Voice) -> Option<Key> {
    match tags {
        "participle" => return Some(Key::Nf(NonFinite::Participle, voice)),
        "gerund" => return Some(Key::Nf(NonFinite::Gerund, voice)),
        "infinitive" => return Some(Key::Nf(NonFinite::Infinitive, voice)),
        _ => {}
    }
    let t: BTreeSet<&str> = tags.split(',').collect();
    let p: Vec<Person> = t.iter().filter_map(|x| person(x)).collect();
    let n: Vec<Number> = t.iter().filter_map(|x| number(x)).collect();
    let m: Vec<Mood> = t.iter().filter_map(|x| mood(x)).collect();
    let rest: Vec<&str> = t.iter().copied().filter(|x| person(x).is_none() && number(x).is_none() && mood(x).is_none()).collect();
    let (p, n, m) = match (p.as_slice(), n.as_slice(), m.as_slice()) {
        ([p], [n], [m]) => (*p, *n, *m),
        _ => return None,
    };
    let tense = match (m, rest.join(",").as_str()) {
        (Mood::Ind, "present") | (Mood::Sub, "present") | (Mood::Adm, "present") | (Mood::Opt, "present") | (Mood::Imp, "present") => Tense::Pres,
        (Mood::Ind, "imperfect") | (Mood::Sub, "imperfect") | (Mood::Adm, "imperfect") => Tense::Imperf,
        (Mood::Ind, "aorist") => Tense::Aor,
        (Mood::Ind, "perfect") | (Mood::Sub, "perfect") | (Mood::Adm, "perfect") | (Mood::Opt, "perfect") => Tense::Perf,
        (Mood::Ind, "past,perfect") | (Mood::Sub, "past,perfect") | (Mood::Adm, "past,perfect") => Tense::Pluperf,
        (Mood::Ind, "future") => Tense::Fut,
        (Mood::Ind, "future,perfect") => Tense::FutPerf,
        (Mood::Cnd, "imperfect") => Tense::Pres,
        (Mood::Cnd, "past,perfect") => Tense::Perf,
        _ => return None,
    };
    Some(Key::Cell(VerbCell { mood: m, tense, person: p, number: n, voice }))
}

fn usable(form: &str) -> bool {
    !form.is_empty() && form != "-" && !form.contains('—')
}

/// Numrat e sipërm shënojnë trajta dialektore («³preh»).
fn dialect_mark(ch: char) -> bool {
    "¹²³⁴⁵⁶⁷⁸⁹⁰".contains(ch)
}

/// «X (Y, ³Z)»: X dhe Y janë trajta ari (Y zë vendin e fjalës së fundit të X);
/// trajtat me numër të sipërm (dialektore) bien.
fn gold_forms(raw: &str) -> Vec<String> {
    let (main, rest) = match raw.split_once(" (") {
        Some((m, r)) => (m, r.strip_suffix(')')),
        None => (raw, None),
    };
    let lead = main.rsplit_once(' ').map(|(a, _)| format!("{a} ")).unwrap_or_default();
    let alts = rest.into_iter().flat_map(|r| r.split(", ")).filter(|a| !a.chars().any(dialect_mark)).map(|a| format!("{lead}{a}"));
    std::iter::once(main.to_string()).chain(alts).filter(|f| usable(f) && !f.chars().any(dialect_mark)).collect()
}

fn load_kaikki() -> Gold {
    let text = std::fs::read_to_string(Path::new(DATA).join(KAIKKI)).unwrap();
    let mut gold = Gold::new();
    let mut lemma: Option<String> = None;
    let mut block: Vec<(String, String)> = Vec::new();
    let flush = |gold: &mut Gold, lemma: &Option<String>, block: &mut Vec<(String, String)>| {
        let voice = match block.iter().any(|(f, _)| f.starts_with("duke u ") || f.starts_with("për t'u ")) {
            true => Voice::Mid,
            false => Voice::Act,
        };
        for (raw, tags) in block.drain(..) {
            for form in gold_forms(&raw) {
                let key = lemma.as_ref().and_then(|l| kaikki_key(&tags, voice).map(|k| (l.clone(), k)));
                gold.entry_or(key, form);
            }
        }
    };
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match c.as_slice() {
            ["E", word, pos, ..] => {
                flush(&mut gold, &lemma, &mut block);
                lemma = (*pos == "verb").then(|| word.to_string());
            }
            ["F", form, tags, source, ..] => {
                let src_ok = *source == "conjugation" || *source == "inflection";
                let restart = *tags == "participle" && block.iter().any(|(_, t)| t != "participle");
                match (src_ok, restart) {
                    (true, true) => {
                        flush(&mut gold, &lemma, &mut block);
                        block.push((form.to_string(), tags.to_string()));
                    }
                    (true, false) => block.push((form.to_string(), tags.to_string())),
                    (false, _) => {}
                }
            }
            _ => {}
        }
    }
    flush(&mut gold, &lemma, &mut block);
    gold
}

trait EntryOr {
    fn entry_or(&mut self, key: Option<(String, Key)>, form: String);
}

impl EntryOr for Gold {
    fn entry_or(&mut self, key: Option<(String, Key)>, form: String) {
        match key {
            Some(k) => {
                self.entry(k).or_default().insert(form);
            }
            None => {}
        }
    }
}

/// Etiketat e UniMorph-it → qeliza («PRF;PRF» i vetës II shumës = «PRF;PST»).
fn unimorph_key(tags: &str, voice: Voice) -> Option<Key> {
    match tags {
        "V;NFIN" => return Some(Key::Nf(NonFinite::Infinitive, voice)),
        "V;V.PTCP" => return Some(Key::Nf(NonFinite::Participle, voice)),
        "V;V.PTCP;PRS" => return Some(Key::Nf(NonFinite::Gerund, voice)),
        _ => {}
    }
    let parts: Vec<&str> = tags.split(';').collect();
    let (p, n, rest) = match parts.as_slice() {
        ["V", p, n, rest @ ..] => (person(p)?, number(n)?, rest.join(";")),
        _ => return None,
    };
    let (m, t) = match rest.as_str() {
        "IND;PRS" => (Mood::Ind, Tense::Pres),
        "IND;IPFV" => (Mood::Ind, Tense::Imperf),
        "IND;PST" => (Mood::Ind, Tense::Aor),
        "IND;PRF" => (Mood::Ind, Tense::Perf),
        // UniMorph-i shënon me «PRF;PST» «pata + pjesore» te veprorja, «isha + pjesore» te joveprorja.
        "IND;PRF;PST" | "IND;PRF;PRF" => match voice {
            Voice::Act => (Mood::Ind, Tense::PastAnterior),
            Voice::Mid => (Mood::Ind, Tense::Pluperf),
        },
        "IND;FUT" => (Mood::Ind, Tense::Fut),
        "IND;PRF;FUT" => (Mood::Ind, Tense::FutPerf),
        "SBJV;PRS" => (Mood::Sub, Tense::Pres),
        "SBJV;PST" => (Mood::Sub, Tense::Imperf),
        "SBJV;PRF" => (Mood::Sub, Tense::Perf),
        "SBJV;PRF;PST" | "SBJV;PRF;PRF" => (Mood::Sub, Tense::Pluperf),
        "COND;PRS" => (Mood::Cnd, Tense::Pres),
        "COND;PRF" => (Mood::Cnd, Tense::Perf),
        "ADM;PRS" => (Mood::Adm, Tense::Pres),
        "ADM;IPFV" => (Mood::Adm, Tense::Imperf),
        "ADM;PRF" => (Mood::Adm, Tense::Perf),
        "ADM;PRF;PST" | "ADM;PRF;PRF" => (Mood::Adm, Tense::Pluperf),
        "OPT;PRS" => (Mood::Opt, Tense::Pres),
        "OPT;PRF" => (Mood::Opt, Tense::Perf),
        "IMP" => (Mood::Imp, Tense::Pres),
        _ => return None,
    };
    Some(Key::Cell(VerbCell { mood: m, tense: t, person: p, number: n, voice }))
}

fn load_unimorph() -> Gold {
    let text = std::fs::read_to_string(Path::new(DATA).join(UNIMORPH)).unwrap();
    let rows: Vec<(&str, &str, &str)> = text
        .lines()
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            match c.as_slice() {
                [lemma, form, tags, ..] => tags.starts_with("V;").then_some((*lemma, *form, *tags)),
                _ => None,
            }
        })
        .collect();
    // Zëri i lemës: «-em» me vetën III njëjës në «-et» është joveprore.
    let middle: BTreeSet<&str> = rows.iter().filter(|(l, f, t)| *t == "V;3;SG;IND;PRS" && l.ends_with("em") && f.ends_with("et")).map(|(l, _, _)| *l).collect();
    let mut gold = Gold::new();
    for (lemma, form, tags) in rows {
        let voice = match middle.contains(lemma) {
            true => Voice::Mid,
            false => Voice::Act,
        };
        let key = unimorph_key(tags, voice).filter(|_| usable(form)).map(|k| (lemma.to_string(), k));
        gold.entry_or(key, form.to_string());
    }
    gold
}

#[derive(Default, Clone, Copy)]
struct Tally {
    total: usize,
    answered: usize,
    correct: usize,
    strict: usize,
}

impl Tally {
    fn add(&mut self, answer: Option<&Form>, gold: &BTreeSet<String>) {
        self.total += 1;
        match answer {
            Some(f) => {
                self.answered += 1;
                self.correct += usize::from(gold.iter().any(|g| f.accepts(g)));
                self.strict += usize::from(gold.contains(&f.text));
            }
            None => {}
        }
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

/// Kufiri i poshtëm Wilson 95%: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n), z = 1,96.
fn wilson_lower(correct: usize, n: usize) -> f64 {
    let z = 1.96_f64;
    match n {
        0 => 0.0,
        _ => {
            let nf = n as f64;
            let p = correct as f64 / nf;
            (p + z * z / (2.0 * nf) - z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt()) / (1.0 + z * z / nf)
        }
    }
}

fn line(name: &str, t: &Tally) -> String {
    format!(
        "{:<34} {:>7} {:>8} {:>7} {:>7} {:>7.4} {:>7.4} {:>8.4} {:>7.4} {:>8.4}",
        name,
        t.total,
        t.answered,
        t.correct,
        t.strict,
        t.precision(),
        ratio(t.answered, t.total),
        wilson_lower(t.correct, t.answered),
        ratio(t.strict, t.answered),
        wilson_lower(t.strict, t.answered)
    )
}

fn header() -> String {
    format!("{:<34} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>8} {:>7} {:>8}", "metric", "total", "answered", "correct", "strict", "prec", "cov", "wilson95", "strictP", "wilsonS")
}

fn answer(f: &Folja, lemma: &str, key: Key) -> Option<Form> {
    match key {
        Key::Cell(c) => f.conjugate(lemma, c),
        Key::Nf(nf, v) => f.nonfinite(lemma, nf, v),
    }
}

/// Qelizat e një fjale: kohët e thjeshta, urdhërorja, pjesorja (pa ndihmëse e pjesëza).
fn synthetic(key: Key) -> bool {
    match key {
        Key::Cell(c) => matches!(
            (c.voice, c.mood, c.tense),
            (Voice::Act, Mood::Ind, Tense::Pres | Tense::Imperf | Tense::Aor)
                | (Voice::Act, Mood::Adm, Tense::Pres | Tense::Imperf)
                | (Voice::Act, Mood::Opt, Tense::Pres)
                | (_, Mood::Imp, Tense::Pres)
                | (Voice::Mid, Mood::Ind, Tense::Pres | Tense::Imperf)
        ),
        Key::Nf(nf, v) => nf == NonFinite::Participle && v == Voice::Act,
    }
}

/// Gjendja e një leme në Test: a u përgjigj, a pati gabim (me variante / vetëm teksti).
#[derive(Default, Clone, Copy)]
struct LemmaRow {
    answered: bool,
    wrong: bool,
    wrong_strict: bool,
}

fn group(key: Key) -> String {
    match key {
        Key::Cell(c) => format!("{:?} {:?} {:?}", c.voice, c.mood, c.tense),
        Key::Nf(nf, v) => format!("{v:?} {nf:?}"),
    }
}

struct Run {
    dev: Tally,
    test: Tally,
    test_no_irregular: Tally,
    test_no_own: Tally,
    test_synthetic: Tally,
    test_no_twins: Tally,
    lemmas: BTreeMap<String, LemmaRow>,
    groups: BTreeMap<String, Tally>,
}

impl Run {
    /// Saktësia sipas lemave: lema është e pastër kur asnjë qelizë e saj e përgjigjur s'është gabim.
    fn by_lemma(&self) -> Tally {
        let rows = self.lemmas.values();
        Tally {
            total: self.lemmas.len(),
            answered: rows.clone().filter(|r| r.answered).count(),
            correct: rows.clone().filter(|r| r.answered && !r.wrong).count(),
            strict: rows.filter(|r| r.answered && !r.wrong_strict).count(),
        }
    }
}

/// Lemat e klasave me mbaresa të një foljeje të vetme (sipas leksemave të vetë lemës).
fn own_class_lemma(lx: &Lexicon, lemma: &str, own: &[&str]) -> bool {
    lx.entries(lemma).iter().filter(|l| l.pos == Pos::Verb).flat_map(|l| l.paradigms.iter()).any(|p| own.contains(&p.trim_end_matches("-1")))
}

fn evaluate(lx: &Lexicon, f: &Folja, gold: &Gold, dump: Option<&str>) -> Run {
    let (irregular, _) = irregular_table();
    let (own, _) = verb_specific_classes();
    let dev_cells: BTreeSet<(Key, &BTreeSet<String>)> = gold.iter().filter(|((l, _), _)| split_of(l) == Split::Dev).map(|((_, k), v)| (*k, v)).collect();
    let mut run = Run {
        dev: Tally::default(),
        test: Tally::default(),
        test_no_irregular: Tally::default(),
        test_no_own: Tally::default(),
        test_synthetic: Tally::default(),
        test_no_twins: Tally::default(),
        lemmas: BTreeMap::new(),
        groups: BTreeMap::new(),
    };
    for ((lemma, key), forms) in gold {
        let a = answer(f, lemma, *key);
        match split_of(lemma) {
            Split::Dev => {
                run.dev.add(a.as_ref(), forms);
                let wrong = a.as_ref().filter(|f| !forms.iter().any(|g| f.accepts(g)));
                match wrong.zip(dump) {
                    Some((f, tag)) => {
                        let class = lx.entries(lemma).iter().find(|l| l.pos == Pos::Verb).map(|l| format!("{}:{}", l.paradigms.join(","), l.stems.join("|"))).unwrap_or_default();
                        eprintln!("DEV-MISS {tag}\t{lemma}\t{}\t{}\t{:?}\t{}", group(*key), f.text, forms, class);
                    }
                    None => {}
                }
                match (a.is_none(), dump) {
                    (true, Some(tag)) => {
                        let class = lx.entries(lemma).iter().filter(|l| l.pos == Pos::Verb).map(|l| format!("{}[{}]:{}", l.paradigms.join(","), l.standard, l.stems.join("|"))).collect::<Vec<_>>().join(" ; ");
                        eprintln!("DEV-NONE {tag}\t{lemma}\t{}\t{:?}\t{}", group(*key), forms, class);
                    }
                    _ => {}
                }
            }
            Split::Test => {
                run.test.add(a.as_ref(), forms);
                run.groups.entry(group(*key)).or_default().add(a.as_ref(), forms);
                let irr = irregular.contains(&lemma.as_str());
                let own_class = irr || own_class_lemma(lx, lemma, &own);
                let twin = dev_cells.contains(&(*key, forms));
                let picks = [(!irr, &mut run.test_no_irregular), (!own_class, &mut run.test_no_own), (synthetic(*key), &mut run.test_synthetic), (!twin, &mut run.test_no_twins)];
                picks.into_iter().filter(|(take, _)| *take).for_each(|(_, tally)| tally.add(a.as_ref(), forms));
                let row = run.lemmas.entry(lemma.clone()).or_default();
                row.answered |= a.is_some();
                row.wrong |= a.as_ref().is_some_and(|f| !forms.iter().any(|g| f.accepts(g)));
                row.wrong_strict |= a.as_ref().is_some_and(|f| !forms.contains(&f.text));
            }
        }
    }
    run
}

/// Rregullat e vetës III njëjës të `nura_lingua::verb` (pa UD), për pajtueshmërinë.
mod nura_verb {
    use nura_lingua::gramatika::leksiku::{Lexeme, Lexicon};
    use nura_lingua::gramatika::tipet::Pos;

    const IRREGULAR: &[(&str, &str)] = &[
        ("jam", "është"), ("kam", "ka"), ("them", "thotë"), ("dua", "do"), ("vij", "vjen"), ("jap", "jep"), ("bie", "bie"),
        ("di", "di"), ("lë", "lë"), ("vë", "vë"), ("zë", "zë"), ("rri", "rri"), ("iki", "ikën"), ("eci", "ecën"), ("hyj", "hyn"),
        ("ha", "ha"), ("pi", "pi"), ("fle", "fle"), ("shoh", "sheh"), ("flas", "flet"), ("dal", "del"), ("marr", "merr"),
        ("vdes", "vdes"), ("vras", "vret"), ("pres", "pret"), ("shes", "shet"), ("ngas", "nget"), ("njoh", "njeh"), ("bëj", "bën"),
        ("mbaj", "mban"), ("shkoj", "shkon"), ("sjell", "sjell"), ("ndjek", "ndjek"), ("hedh", "hedh"), ("djeg", "djeg"),
        ("bredh", "bredh"), ("mbyll", "mbyll"), ("hap", "hap"), ("heq", "heq"), ("vesh", "vesh"), ("rris", "rrit"), ("pyes", "pyet"),
        ("mbes", "mbetet"), ("fryj", "fryn"),
    ];

    enum Rule {
        Stem0N,
        Stem1,
        Stem0,
    }

    fn class_rule(par: &str) -> Option<Rule> {
        match par.trim_end_matches("-1") {
            "lexoj" | "laj" | "kthej" | "brej" | "ndiej" | "shkruaj" | "shkruej" | "arrij" => Some(Rule::Stem0N),
            "pres-wait" | "flas" | "shoh" | "dal" | "marr" | "jap" | "vras" | "pres" | "shes" | "vdes" | "ngas" => Some(Rule::Stem1),
            "hap" | "mat" | "beh" | "heq" | "vjel" | "zhvesh" | "ec" | "ik" => Some(Rule::Stem0),
            _ => None,
        }
    }

    fn stems(raw: &[String]) -> Vec<String> {
        raw.iter().map(|s| s.split("//").next().unwrap_or("").trim_matches('.').to_string()).filter(|s| !s.is_empty() && s != "NOSTEM").collect()
    }

    fn mediopassive(lemma: &str) -> Option<String> {
        lemma.strip_suffix("em").map(|stem| format!("{stem}et"))
    }

    /// Si në importin e nura_lingua: rreshti i parë std, ose jostandard me lemë standarde (lexref).
    fn first_edge<'l>(lx: &'l Lexicon, lemma: &str) -> Option<&'l Lexeme> {
        lx.entries(lemma).iter().find(|l| l.pos == Pos::Verb && (l.standard || l.lexref.as_deref().is_some_and(|r| r != lemma)))
    }

    /// Defekti i dëshmuar i `verb.rs` kur jep trajtë tjetër nga `folja`.
    pub fn defect(lx: &Lexicon, lemma: &str, theirs: &str) -> Option<&'static str> {
        let edge = first_edge(lx, lemma);
        let irregular = IRREGULAR.iter().any(|(l, _)| *l == lemma);
        let nonstandard_edge = edge.is_some_and(|l| !l.standard);
        let em_rule = lemma.strip_suffix("em").is_some_and(|s| theirs == format!("{s}et"));
        let v0 = edge.and_then(|l| l.stems.first()).and_then(|s0| s0.split("//").next()).map(|v| v.trim_matches('.'));
        let stem_variant = v0.is_some_and(|v| !v.is_empty() && !lemma.starts_with(v) && theirs.starts_with(v));
        match (irregular, nonstandard_edge, em_rule, stem_variant) {
            (true, _, _, _) => Some("D4 tabela IRREGULAR e verb.rs kundër klasës së leksikut"),
            (_, true, _, _) => Some("D3 verb.rs lexon rreshtin jostandard të lemës"),
            (_, _, true, _) => Some("D2 rregulli «-em → -et» mbi folje veprore në «-em»"),
            (_, _, _, true) => Some("D1 tema e parë e rreshtit të parë nuk e prodhon lemën"),
            _ => None,
        }
    }

    fn by_paradigm(lx: &Lexicon, lemma: &str) -> Option<String> {
        let lex = first_edge(lx, lemma)?;
        let par = lex.paradigms.join(",");
        let st = stems(&lex.stems);
        let first = st.first().cloned()?;
        match (mediopassive(lemma), class_rule(&par)) {
            (Some(m), _) => Some(m),
            (None, Some(Rule::Stem0N)) => Some(format!("{first}n")),
            (None, Some(Rule::Stem1)) => st.get(1).cloned(),
            (None, Some(Rule::Stem0)) => match (lemma.ends_with('s'), st.get(1).filter(|s| s.ends_with('t'))) {
                (true, Some(s)) => Some(s.clone()),
                _ => Some(first),
            },
            (None, None) => None,
        }
    }

    fn by_shape(lemma: &str) -> Option<String> {
        let j = lemma.strip_suffix('j').filter(|s| s.ends_with(['o', 'e', 'a', 'i', 'y', 'u']));
        match (mediopassive(lemma), j) {
            (Some(m), _) => Some(m),
            (None, Some(stem)) => Some(format!("{stem}n")),
            (None, None) => None,
        }
    }

    pub fn rule_3sg(lx: &Lexicon, lemma: &str) -> Option<String> {
        IRREGULAR.iter().find(|(l, _)| *l == lemma).map(|(_, f)| f.to_string()).or_else(|| by_paradigm(lx, lemma)).or_else(|| by_shape(lemma))
    }
}

#[test]
fn gold_measurement() {
    let lx = Lexicon::load(Path::new(DATA)).unwrap();
    let dump = std::env::var("FOLJA_DEV").ok();
    let kaikki = load_kaikki();
    let unimorph = load_unimorph();
    let mut both = kaikki.clone();
    for (k, v) in &unimorph {
        both.entry(k.clone()).or_default().extend(v.iter().cloned());
    }
    let lemmas = |g: &Gold| g.keys().map(|(l, _)| l.clone()).collect::<BTreeSet<String>>().len();
    eprintln!("gold: wiktionary {} cells / {} verbs; unimorph {} cells / {} verbs", kaikki.len(), lemmas(&kaikki), unimorph.len(), lemmas(&unimorph));

    let folja = Folja::new(&lx);
    let rk = evaluate(&lx, &folja, &kaikki, dump.as_deref().filter(|d| *d == "k" || *d == "all").map(|_| "wikt"));
    let ru = evaluate(&lx, &folja, &unimorph, dump.as_deref().filter(|d| *d == "u" || *d == "all").map(|_| "um"));
    let rb = evaluate(&lx, &folja, &both, None);
    let (irr, irr_size) = irregular_table();
    let (own, own_size) = verb_specific_classes();

    eprintln!("\n{}", header());
    eprintln!("{}", line("wiktionary dev", &rk.dev));
    eprintln!("{}", line("wiktionary test", &rk.test));
    eprintln!("{}", line("wiktionary test w/o irregular", &rk.test_no_irregular));
    eprintln!("{}", line("unimorph dev", &ru.dev));
    eprintln!("{}", line("unimorph test", &ru.test));
    eprintln!("{}", line("unimorph test w/o irregular", &ru.test_no_irregular));
    eprintln!("{}", line("combined dev", &rb.dev));
    eprintln!("{}", line("combined test", &rb.test));
    eprintln!("{}", line("combined test w/o irregular", &rb.test_no_irregular));
    eprintln!("{}", line("combined test w/o irr+own classes", &rb.test_no_own));
    eprintln!("{}", line("combined test w/o dev twins", &rb.test_no_twins));
    eprintln!("{}", line("combined test synthetic cells", &rb.test_synthetic));
    eprintln!("{}", line("combined test by lemma", &rb.by_lemma()));
    eprintln!("irregular table: {} lemmas ({}), {} forms", irr.len(), irr.join(", "), irr_size);
    eprintln!("verb-specific ending sets: {} classes ({}), {} endings", own.len(), own.join(", "), own_size);
    eprintln!("prec = correct/answered (text or accepted variant); strictP = text alone (headline); wilson95/wilsonS = Wilson 95% lower bound of prec/strictP");
    eprintln!("by lemma: total = test lemmas, answered = lemmas with >= 1 answer, correct/strict = answered lemmas with no wrong cell; Wilson over lemmas");
    eprintln!("\nper voice+mood+tense group, combined test:\n{}", header());
    for (g, t) in &rb.groups {
        eprintln!("{}", line(g, t));
    }

    let cov = |t: &Tally| ratio(t.answered, t.total);
    let strict = |t: &Tally| ratio(t.strict, t.answered);
    let lemmas = rb.by_lemma();
    assert!(rk.test.precision() >= 0.984, "wiktionary test precision");
    assert!(ru.test.precision() >= 0.977, "unimorph test precision");
    assert!(rb.test.precision() >= 0.984, "combined test precision");
    assert!(rb.test_no_irregular.precision() >= 0.984, "combined test precision w/o irregular");
    assert!(rb.test_no_own.precision() >= 0.984, "combined test precision w/o verb-specific classes");
    assert!(wilson_lower(rb.test.correct, rb.test.answered) >= 0.984, "combined test wilson");
    assert!(strict(&rb.test) >= 0.975 && wilson_lower(rb.test.strict, rb.test.answered) >= 0.974, "combined test, standard text alone");
    assert!(rb.test_synthetic.precision() >= 0.981, "combined test, one-word cells");
    assert!(lemmas.precision() >= 0.92, "combined test, lemmas without any wrong cell");
    assert!(cov(&rk.test) >= 0.855 && cov(&ru.test) >= 0.948 && cov(&rb.test) >= 0.855, "test coverage");
    assert!(rb.dev.precision() >= 0.986, "combined dev precision");
}

#[test]
fn present_third_person_agrees_with_nura_lingua_verb() {
    let lx = Lexicon::load(Path::new(DATA)).unwrap();
    let folja = Folja::new(&lx);
    let cell = VerbCell { mood: Mood::Ind, tense: Tense::Pres, person: Person::P3, number: Number::Sing, voice: Voice::Act };
    let lemmas: BTreeSet<&str> = lx.iter().filter(|l| l.pos == Pos::Verb && l.standard).map(|l| l.lemma.as_str()).collect();
    let mut both = 0;
    let mut agree = 0;
    let mut explained: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut unexplained: Vec<String> = Vec::new();
    for lemma in &lemmas {
        let mine = folja.conjugate(lemma, cell).map(|f| f.text);
        let theirs = nura_verb::rule_3sg(&lx, lemma);
        match (mine, theirs) {
            (Some(m), Some(t)) => {
                both += 1;
                let row = format!("{lemma}: folja={m} verb.rs={t}");
                match (m == t, nura_verb::defect(&lx, lemma, &t)) {
                    (true, _) => agree += 1,
                    (false, Some(d)) => explained.entry(d).or_default().push(row),
                    (false, None) => unexplained.push(row),
                }
            }
            _ => {}
        }
    }
    eprintln!("pres 3sg: {both} verbs answered by both; {agree} agree; {} differ by a documented verb.rs defect; {} unexplained", both - agree - unexplained.len(), unexplained.len());
    for (d, rows) in &explained {
        eprintln!("  {d}: {} ({})", rows.len(), rows.iter().take(6).cloned().collect::<Vec<_>>().join("; "));
    }
    for r in &unexplained {
        eprintln!("  UNEXPLAINED {r}");
    }
    assert!(unexplained.is_empty(), "e tashmja veta III njëjës duhet të pajtohet me nura_lingua::verb, përveç defekteve të dokumentuara");
}

/// `Folja` (tregues i ndërtuar një herë) dhe funksionet e lira japin të njëjtën përgjigje.
#[test]
fn folja_index_matches_the_free_functions() {
    let lx = Lexicon::load(Path::new(DATA)).unwrap();
    let folja = Folja::new(&lx);
    let mids = [
        "lahem", "shihem", "ngrihem", "hiqem", "bëhem", "duhem", "thuhem", "jepem", "merrem", "gjendem", "flitem", "pritem", "vritem", "dilem", "njihem", "lihem", "vihem", "zihem", "blihem",
        "ruhem", "kthehem", "shkruhem", "hapem", "mbahem", "zgjidhem", "vishem", "fshihem", "digjem", "pihem", "hahem",
    ];
    let actives: Vec<&str> = lx.iter().filter(|l| l.pos == Pos::Verb && l.standard).map(|l| l.lemma.as_str()).collect::<BTreeSet<&str>>().into_iter().step_by(97).collect();
    let cells = [
        VerbCell { mood: Mood::Ind, tense: Tense::Pres, person: Person::P1, number: Number::Sing, voice: Voice::Mid },
        VerbCell { mood: Mood::Ind, tense: Tense::Aor, person: Person::P3, number: Number::Sing, voice: Voice::Mid },
        VerbCell { mood: Mood::Imp, tense: Tense::Pres, person: Person::P2, number: Number::Plur, voice: Voice::Mid },
        VerbCell { mood: Mood::Ind, tense: Tense::Aor, person: Person::P1, number: Number::Plur, voice: Voice::Act },
        VerbCell { mood: Mood::Imp, tense: Tense::Pres, person: Person::P2, number: Number::Plur, voice: Voice::Act },
    ];
    let mut compared = 0;
    let mut answered = 0;
    for lemma in mids.iter().copied().chain(actives.iter().copied()) {
        for c in cells {
            let fast = folja.conjugate(lemma, c);
            assert_eq!(conjugate_lemma(&lx, lemma, c), fast, "{lemma} {c:?}");
            compared += 1;
            answered += usize::from(fast.is_some());
        }
        assert_eq!(nonfinite_lemma(&lx, lemma, NonFinite::Gerund, Voice::Mid), folja.nonfinite(lemma, NonFinite::Gerund, Voice::Mid), "{lemma} gerund");
    }
    eprintln!("Folja vs free functions: {compared} cells compared, {answered} answered, all equal");
    assert!(answered > 100);
}
