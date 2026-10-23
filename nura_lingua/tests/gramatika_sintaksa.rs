//! Matja mbi pemët UD (STAF train/dev/test, TSA test): fjalitë e arta janë të
//! rregullta, prandaj çdo `Disagree` është alarm i rremë; ngacmimet e tipareve
//! duhet të kapen. Ndarja zhvillim/provë sipas `split_of(sent_id)`. Çdo matje bëhet
//! dy herë: vetëm me shënimin (`check`) dhe me leksikun si dëshmitar (`check_with`).
//! Shuma e rreshtave të arta numëron vendet strukturore të rregullit (dhe çdo vend tjetër
//! ku rregulli dha gjetje), jo vetëm gjetjet: vendet pa gjetje ulin mbulimin.

use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::sintaksa::{
    check, check_custom, check_with, parse_conllu_ids, Finding, Sentence, Verdict, ADJECTIVE_ARTICLE, CLITIC_DOUBLING,
    CLITIC_PLACEMENT, DETERMINER_NOUN, GENITIVE_ARTICLE, NEGATION, NOUN_ADJECTIVE, RULES, SUBJECT_VERB, TIME_NOUNS,
};
use nura_lingua::gramatika::tipet::{split_of, ud_feat, Split};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
const FILES: [&str; 4] = [
    "raw/ud_sq_staf/sq_staf-ud-train.conllu",
    "raw/ud_sq_staf/sq_staf-ud-dev.conllu",
    "raw/ud_sq_staf/sq_staf-ud-test.conllu",
    "raw/ud_sq_tsa/sq_tsa-ud-test.conllu",
];

type Gold = Vec<(String, Split, Sentence)>;

fn gold() -> Gold {
    FILES
        .iter()
        .flat_map(|f| parse_conllu_ids(&std::fs::read_to_string(format!("{DATA}/{f}")).unwrap()))
        .map(|(id, s)| (id.clone(), split_of(&id), s))
        .collect()
}

#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    total: usize,
    answered: usize,
    correct: usize,
}

/// Kufiri i poshtëm Wilson 95%: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n).
fn wilson_lower(correct: usize, n: usize) -> f64 {
    let z = 1.96f64;
    let nf = n as f64;
    match n {
        0 => 0.0,
        _ => {
            let p = correct as f64 / nf;
            (p + z * z / (2.0 * nf) - z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt()) / (1.0 + z * z / nf)
        }
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    match b {
        0 => 0.0,
        _ => a as f64 / b as f64,
    }
}

fn row(metric: &str, split: &str, t: Tally) -> String {
    format!(
        "| {:<42} | {:<4} | {:>5} | {:>8} | {:>7} | {:>9.4} | {:>8.4} | {:>8.4} |",
        metric,
        split,
        t.total,
        t.answered,
        t.correct,
        ratio(t.correct, t.answered),
        ratio(t.answered, t.total),
        wilson_lower(t.correct, t.answered)
    )
}

fn text_of(s: &Sentence) -> String {
    s.iter().map(|t| t.form.as_str()).collect::<Vec<_>>().join(" ")
}

fn dev_trace(kind: &str) -> bool {
    std::env::var("SINTAKSA_DEV").map(|v| v == kind).unwrap_or(false)
}

fn low(s: &str) -> String {
    s.to_lowercase()
}

const CLITIC_FORMS: [&str; 16] = ["më", "të", "e", "i", "na", "ju", "u", "ia", "ua", "ma", "ta", "jua", "ja", "iu", "m'", "t'"];

fn article_token(t: &nura_lingua::gramatika::sintaksa::Token) -> bool {
    matches!(t.upos.as_str(), "DET" | "PART") && matches!(low(&t.form).as_str(), "i" | "e" | "të" | "së")
}

fn clitic_token(t: &nura_lingua::gramatika::sintaksa::Token) -> bool {
    t.upos == "PRON" && CLITIC_FORMS.contains(&low(&t.form).as_str()) && ud_feat(&t.feats, "Case") != Some("Nom")
}

/// Vendet strukturore të çdo rregulli, të përcaktuara nga pema dhe tiparet, jo nga vendimi.
fn rule_sites(s: &Sentence, rule: &str) -> Vec<usize> {
    let get = |id: usize| s.iter().find(|t| t.id == id);
    let head_upos = |t: &nura_lingua::gramatika::sintaksa::Token| get(t.head).map(|h| h.upos.clone()).unwrap_or_default();
    s.iter()
        .filter(|t| match rule {
            SUBJECT_VERB => matches!(t.deprel.as_str(), "nsubj" | "nsubj:pass"),
            NOUN_ADJECTIVE => {
                t.upos == "ADJ"
                    && (t.deprel == "amod" || (t.deprel == "conj" && get(t.head).map_or(false, |h| h.upos == "ADJ" && h.deprel == "amod")))
            }
            ADJECTIVE_ARTICLE => {
                t.upos == "ADJ"
                    && s.iter().any(|k| k.head == t.id && k.id < t.id && article_token(k) && matches!(k.deprel.as_str(), "det:adj" | "det" | "case"))
            }
            GENITIVE_ARTICLE => {
                t.deprel == "nmod:poss" || t.deprel == "det:poss" || (t.deprel.starts_with("nmod") && ud_feat(&t.feats, "Case") == Some("Gen"))
            }
            DETERMINER_NOUN => {
                matches!(t.deprel.as_str(), "det" | "det:poss" | "nummod") && !article_token(t) && matches!(head_upos(t).as_str(), "NOUN" | "PROPN")
            }
            CLITIC_DOUBLING => {
                let verb_head = matches!(head_upos(t).as_str(), "VERB" | "AUX");
                let indirect = t.deprel == "iobj" && matches!(t.upos.as_str(), "NOUN" | "PROPN" | "PRON");
                let direct = t.deprel == "obj"
                    && ((matches!(t.upos.as_str(), "NOUN" | "PROPN") && ud_feat(&t.feats, "Definite") == Some("Def"))
                        || (t.upos == "PRON" && ud_feat(&t.feats, "PronType") == Some("Prs")));
                verb_head && !clitic_token(t) && (indirect || direct)
            }
            NEGATION => matches!(t.upos.as_str(), "PART" | "ADV") && matches!(low(&t.form).as_str(), "nuk" | "s'" | "s" | "mos"),
            CLITIC_PLACEMENT => clitic_token(t) && (matches!(t.deprel.as_str(), "obj" | "iobj") || t.deprel.starts_with("expl")),
            _ => false,
        })
        .map(|t| t.id)
        .collect()
}

/// Fjala që i përket vendit në `tokens` të gjetjes, sipas rregullit.
fn principal(rule: &str, f: &Finding) -> Option<usize> {
    match rule {
        NOUN_ADJECTIVE => f.tokens.get(1),
        ADJECTIVE_ARTICLE | GENITIVE_ARTICLE => f.tokens.last(),
        _ => f.tokens.first(),
    }
    .copied()
}

/// Alarmet e rreme për rregull, sipas vendeve: total = vende, answered = vende me Agree ose
/// Disagree, correct = vende me Agree dhe pa Disagree; saktësia = correct / answered.
fn false_alarms(data: &Gold, split: Split, rule: &str, run: &dyn Fn(&Sentence) -> Vec<Finding>) -> Tally {
    let mut t = Tally::default();
    for (id, _, s) in data.iter().filter(|(_, sp, _)| *sp == split) {
        let fs: Vec<Finding> = run(s).into_iter().filter(|f| f.rule == rule).collect();
        let mut sites = rule_sites(s, rule);
        sites.extend(fs.iter().filter_map(|f| principal(rule, f)));
        sites.sort_unstable();
        sites.dedup();
        for site in sites {
            let at: Vec<&Finding> = fs.iter().filter(|f| principal(rule, f) == Some(site)).collect();
            let disagree = at.iter().any(|f| f.verdict == Verdict::Disagree);
            let agree = at.iter().any(|f| f.verdict == Verdict::Agree);
            t.total += 1;
            t.answered += usize::from(agree || disagree);
            t.correct += usize::from(agree && !disagree);
            let shown: Vec<_> = at.iter().map(|f| (f.verdict, f.tokens.clone(), f.note.clone())).collect();
            let show = split == Split::Dev && ((disagree && dev_trace("fa")) || (!agree && !disagree && dev_trace("hold")));
            show.then(|| eprintln!("[{rule}] {id} site {site} {shown:?}\n    {}", text_of(s)));
        }
    }
    t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Perturbation {
    VerbNumber,
    AdjectiveGender,
    NounDefinite,
    Negation,
}

impl Perturbation {
    fn name(self) -> &'static str {
        match self {
            Perturbation::VerbNumber => "perturb verb Number -> subject-verb",
            Perturbation::AdjectiveGender => "perturb adj Gender -> noun-adjective",
            Perturbation::NounDefinite => "perturb noun Definite -> articles/det",
            Perturbation::Negation => "perturb nuk<->mos -> negation",
        }
    }

    fn targets(self) -> &'static [&'static str] {
        match self {
            Perturbation::VerbNumber => &[SUBJECT_VERB],
            Perturbation::AdjectiveGender => &[NOUN_ADJECTIVE],
            Perturbation::NounDefinite => &[ADJECTIVE_ARTICLE, GENITIVE_ARTICLE, DETERMINER_NOUN, CLITIC_DOUBLING],
            Perturbation::Negation => &[NEGATION],
        }
    }
}

/// Vendet e ngacmimit, të përcaktuara nga struktura e pemës, jo nga vendimi i rregullit.
fn sites(s: &Sentence, p: Perturbation) -> Vec<usize> {
    let has_kid = |id: usize, rels: &[&str]| s.iter().any(|k| k.head == id && rels.contains(&k.deprel.as_str()));
    s.iter()
        .filter(|t| match p {
            Perturbation::VerbNumber => {
                matches!(t.upos.as_str(), "VERB" | "AUX")
                    && ud_feat(&t.feats, "Number").is_some()
                    && ud_feat(&t.feats, "Person").is_some()
                    && !matches!(low(&t.form).as_str(), "u" | "do" | "s'")
                    && (has_kid(t.id, &["nsubj", "nsubj:pass"])
                        || (matches!(t.deprel.as_str(), "aux" | "aux:pass" | "cop") && has_kid(t.head, &["nsubj", "nsubj:pass"])))
            }
            Perturbation::AdjectiveGender => {
                t.upos == "ADJ" && t.deprel == "amod" && matches!(ud_feat(&t.feats, "Gender"), Some("Masc") | Some("Fem"))
            }
            Perturbation::NounDefinite => {
                matches!(t.upos.as_str(), "NOUN" | "PROPN")
                    && ud_feat(&t.feats, "Definite").is_some()
                    && has_kid(t.id, &["amod", "nmod:poss", "det", "det:poss", "nummod"])
            }
            Perturbation::Negation => matches!(t.upos.as_str(), "PART" | "ADV") && matches!(low(&t.form).as_str(), "nuk" | "mos"),
        })
        .map(|t| t.id)
        .collect()
}

fn swap(feats: &str, key: &str, a: &str, b: &str) -> String {
    feats
        .split('|')
        .map(|kv| match kv.split_once('=') {
            Some((k, v)) => match (k == key, v == a, v == b) {
                (true, true, _) => format!("{k}={b}"),
                (true, false, true) => format!("{k}={a}"),
                _ => kv.to_string(),
            },
            None => kv.to_string(),
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn flip_negator(form: &str) -> String {
    match form {
        "nuk" => "mos".to_string(),
        "Nuk" => "Mos".to_string(),
        "NUK" => "MOS".to_string(),
        "mos" => "nuk".to_string(),
        "Mos" => "Nuk".to_string(),
        _ => "NUK".to_string(),
    }
}

fn perturb(s: &Sentence, p: Perturbation, id: usize) -> Sentence {
    s.iter()
        .cloned()
        .map(|mut t| {
            match (t.id == id, p) {
                (false, _) => (),
                (true, Perturbation::VerbNumber) => t.feats = swap(&t.feats, "Number", "Sing", "Plur"),
                (true, Perturbation::AdjectiveGender) => t.feats = swap(&t.feats, "Gender", "Masc", "Fem"),
                (true, Perturbation::NounDefinite) => t.feats = swap(&t.feats, "Definite", "Def", "Ind"),
                (true, Perturbation::Negation) => {
                    t.form = flip_negator(&t.form);
                    t.lemma = low(&t.form);
                }
            }
            t
        })
        .collect()
}

/// Kapja e ngacmimeve: total = vende, answered = rregulli dha vendim jo-Hold për vendin,
/// correct = rregulli dha një `Disagree` të ri, që fjalia e paprekur nuk e kishte (i njëjti
/// rregull, të njëjtat fjalë); recall = correct / total.
fn perturbations(data: &Gold, split: Split, p: Perturbation, run: &dyn Fn(&Sentence) -> Vec<Finding>) -> Tally {
    let mut t = Tally::default();
    for (id, _, s) in data.iter().filter(|(_, sp, _)| *sp == split) {
        let before: Vec<(&str, Vec<usize>)> = run(s)
            .into_iter()
            .filter(|f| p.targets().contains(&f.rule) && f.verdict == Verdict::Disagree)
            .map(|f| (f.rule, f.tokens))
            .collect();
        for site in sites(s, p) {
            let changed = perturb(s, p, site);
            let fs: Vec<_> = run(&changed).into_iter().filter(|f| p.targets().contains(&f.rule) && f.tokens.contains(&site)).collect();
            let answered = fs.iter().any(|f| f.verdict != Verdict::Hold);
            let caught = fs.iter().any(|f| f.verdict == Verdict::Disagree && !before.contains(&(f.rule, f.tokens.clone())));
            t.total += 1;
            t.answered += usize::from(answered);
            t.correct += usize::from(caught);
            let show = split == Split::Dev && !caught && dev_trace("miss");
            show.then(|| {
                eprintln!(
                    "[{}] {id} site {site} {:?}\n    {}",
                    p.name(),
                    fs.iter().map(|f| (f.rule, f.verdict, f.note.clone())).collect::<Vec<_>>(),
                    text_of(s)
                )
            });
        }
    }
    t
}

const PERTURBATIONS: [Perturbation; 4] = [Perturbation::VerbNumber, Perturbation::AdjectiveGender, Perturbation::NounDefinite, Perturbation::Negation];
const SPLITS: [(Split, &str); 2] = [(Split::Dev, "dev"), (Split::Test, "test")];

/// Synimi i kërkesës: alarmi i rremë për rregull jo më shumë se 3%.
const TARGET: f64 = 0.03;

/// Roje regresioni, pak përtej vlerave të matura (Dev dhe Test, të dy variantet). Nuk janë
/// synimi: rreshtat «target» tregojnë ku synimi 3% nuk arrihet.
const FALSE_ALARM_CEILING: [(&str, f64); 8] = [
    (SUBJECT_VERB, 0.05),
    (NOUN_ADJECTIVE, 0.03),
    (ADJECTIVE_ARTICLE, 0.02),
    (GENITIVE_ARTICLE, 0.01),
    (DETERMINER_NOUN, 0.02),
    (CLITIC_DOUBLING, 0.04),
    (NEGATION, 0.01),
    (CLITIC_PLACEMENT, 0.01),
];
const COVERAGE_FLOOR: [(&str, f64); 8] = [
    (SUBJECT_VERB, 0.74),
    (NOUN_ADJECTIVE, 0.78),
    (ADJECTIVE_ARTICLE, 0.65),
    (GENITIVE_ARTICLE, 0.62),
    (DETERMINER_NOUN, 0.62),
    (CLITIC_DOUBLING, 0.33),
    (NEGATION, 0.92),
    (CLITIC_PLACEMENT, 0.97),
];
const RECALL_FLOOR: [(Perturbation, f64); 4] = [
    (Perturbation::VerbNumber, 0.70),
    (Perturbation::AdjectiveGender, 0.83),
    (Perturbation::NounDefinite, 0.09),
    (Perturbation::Negation, 0.83),
];

/// Vendet e kundrinës që tabela e emrave të kohës i prek: vendimi ndryshon pa tabelë.
fn table_changes(data: &Gold, split: Split) -> usize {
    let verdicts = |s: &Sentence, table: &[&str]| -> Vec<(Vec<usize>, Verdict)> {
        check_custom(s, None, table).into_iter().filter(|f| f.rule == CLITIC_DOUBLING).map(|f| (f.tokens, f.verdict)).collect()
    };
    data.iter().filter(|(_, sp, s)| *sp == split && verdicts(s, &TIME_NOUNS) != verdicts(s, &[])).count()
}

#[test]
fn gold_metrics() {
    let data = gold();
    let lexicon = Lexicon::load(std::path::Path::new(DATA)).unwrap();
    let n_dev = data.iter().filter(|(_, s, _)| *s == Split::Dev).count();
    assert_eq!(data.len(), 260);
    let plain = |s: &Sentence| check(s);
    let lexical = |s: &Sentence| check_with(s, &lexicon);
    let variants: [(&str, &dyn Fn(&Sentence) -> Vec<Finding>); 2] = [("", &plain), (" +lex", &lexical)];
    let header = format!(
        "| {:<42} | {:<4} | {:>5} | {:>8} | {:>7} | {:>9} | {:>8} | {:>8} |",
        "metric", "split", "total", "answered", "correct", "precision", "coverage", "wilson95"
    );
    let mut lines = vec![
        format!("sentences: dev {n_dev}, test {}", data.len() - n_dev),
        header.clone(),
        "|".to_string() + &"-".repeat(header.len() - 2) + "|",
    ];
    let mut fa = Vec::new();
    for rule in RULES {
        for (tag, run) in variants {
            for (split, name) in SPLITS {
                let t = false_alarms(&data, split, rule, run);
                lines.push(row(&format!("gold {rule}{tag}"), name, t));
                fa.push((rule, format!("{rule}{tag}"), split, t));
            }
        }
    }
    let mut pt = Vec::new();
    for p in PERTURBATIONS {
        for (tag, run) in variants {
            for (split, name) in SPLITS {
                let t = perturbations(&data, split, p, run);
                lines.push(row(&format!("{}{tag}", p.name()), name, t));
                pt.push((p, format!("{}{tag}", p.name()), split, t));
            }
        }
    }
    lines.push("gold rows: total = structural sites; precision = Agree/(Agree+Disagree) per site; false-alarm rate = 1 - precision.".to_string());
    lines.push("perturbation rows: precision = caught/answered; caught = a new Disagree absent from the unperturbed run; recall = correct/total.".to_string());
    for (_, name, split, t) in &fa {
        let alarm = 1.0 - ratio(t.correct, t.answered);
        let met = match alarm <= TARGET {
            true => "met",
            false => "NOT met",
        };
        lines.push(format!("false-alarm {name:<24} {split:?}: {alarm:.4} ({} of {}) target<=0.03 {met}", t.answered - t.correct, t.answered));
    }
    for (_, name, split, t) in &pt {
        lines.push(format!("recall {name:<46} {split:?}: {:.4}", ratio(t.correct, t.total)));
    }
    let without = |s: &Sentence| check_custom(s, None, &[]);
    for (split, name) in SPLITS {
        let with_table = false_alarms(&data, split, CLITIC_DOUBLING, &plain);
        let no_table = false_alarms(&data, split, CLITIC_DOUBLING, &without);
        lines.push(format!(
            "table TIME_NOUNS ({} lemmas) {name}: sentences whose clitic-doubling verdicts change {}; precision {:.4} with ({} of {}), {:.4} without ({} of {})",
            TIME_NOUNS.len(),
            table_changes(&data, split),
            ratio(with_table.correct, with_table.answered),
            with_table.correct,
            with_table.answered,
            ratio(no_table.correct, no_table.answered),
            no_table.correct,
            no_table.answered
        ));
    }
    eprintln!("{}", lines.join("\n"));
    for (rule, name, split, t) in &fa {
        let alarm = 1.0 - ratio(t.correct, t.answered);
        let ceiling = FALSE_ALARM_CEILING.iter().find(|(r, _)| r == rule).map(|(_, c)| *c).unwrap();
        let floor = COVERAGE_FLOOR.iter().find(|(r, _)| r == rule).map(|(_, c)| *c).unwrap();
        assert!(alarm <= ceiling, "{name} {split:?}: false-alarm {alarm:.4} above {ceiling}");
        assert!(ratio(t.answered, t.total) >= floor, "{name} {split:?}: coverage below {floor}");
    }
    for (p, name, split, t) in &pt {
        let floor = RECALL_FLOOR.iter().find(|(q, _)| q == p).map(|(_, f)| *f).unwrap();
        assert!(ratio(t.correct, t.total) >= floor, "{name} {split:?}: recall below {floor}");
    }
}

/// Lexuesi CoNLL-U: fjalitë, id-të dhe tiparet e zhvendosura në kolonën XPOS.
#[test]
fn reader_counts() {
    let data = gold();
    assert_eq!(data.iter().filter(|(id, _, _)| id.starts_with("STAF")).count(), 200);
    assert_eq!(data.iter().filter(|(id, _, _)| id.starts_with("as-test")).count(), 60);
    let shifted = data.iter().flat_map(|(_, _, s)| s.iter()).filter(|t| t.feats.contains("Mood=") && t.upos == "SCONJ").count();
    assert!(shifted >= 1);
}

/// Asnjë rregull nuk bie përballë pemëve të dëmtuara: koka që mungojnë, sythe, tipare bosh,
/// rend i përmbysur, tekst CoNLL-U i copëtuar.
#[test]
fn robust_on_damaged_trees() {
    let data = gold();
    let lexicon = Lexicon::load(std::path::Path::new(DATA)).unwrap();
    let mut seen = 0usize;
    for (_, _, s) in &data {
        let n = s.len();
        let damaged: [Sentence; 4] = [
            s.iter().cloned().map(|mut t| {
                t.head += 50;
                t
            }).collect(),
            s.iter().cloned().map(|mut t| {
                t.head = t.id;
                t
            }).collect(),
            s.iter().cloned().map(|mut t| {
                t.feats.clear();
                t.form.clear();
                t
            }).collect(),
            s.iter().cloned().map(|mut t| {
                t.id = n + 1 - t.id.min(n);
                t
            }).collect(),
        ];
        for d in &damaged {
            seen += check(d).len() + check_with(d, &lexicon).len();
        }
    }
    let text = std::fs::read_to_string(format!("{DATA}/{}", FILES[3])).unwrap();
    let chopped: String = text.chars().filter(|c| *c != '\t').collect();
    assert!(parse_conllu_ids(&chopped).is_empty());
    let halves: Vec<String> = text.lines().map(|l| l.chars().take(l.chars().count() / 2).collect()).collect();
    let parsed = parse_conllu_ids(&halves.join("\n"));
    assert!(parsed.len() <= 60);
    assert!(seen > 0);
}
