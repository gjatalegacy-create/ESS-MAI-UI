//! Matja e lakimit të emrit kundrejt arit: Wiktionary (kaikki) dhe UniMorph.
//! Ari përdoret vetëm këtu; rregullat ndërtohen duke parë vetëm gjysmën `Dev`.

use nura_lingua::gramatika::emri::{decline_lemma, decline_shape, noun_lexeme, paradigm, CELLS, GENDER_HOLD};
use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::tipet::{split_of, Case, Definite, Form, NounCell, Number, Pos, Split};
use std::collections::BTreeMap;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");

type Table = BTreeMap<NounCell, Vec<String>>;
type Gold = BTreeMap<String, Table>;

fn word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphabetic() && c.is_lowercase())
}

fn pick<T: Copy>(tags: &[&str], options: &[(&str, T)]) -> Option<T> {
    options.iter().find(|(k, _)| tags.contains(k)).map(|(_, v)| *v)
}

fn wikt_cell(tags: &str) -> Option<NounCell> {
    let t: Vec<&str> = tags.split(',').collect();
    let case = pick(&t, &[("nominative", Case::Nom), ("accusative", Case::Acc), ("dative", Case::Dat), ("ablative", Case::Abl)])?;
    let number = pick(&t, &[("singular", Number::Sing), ("plural", Number::Plur)])?;
    let definite = pick(&t, &[("indefinite", Definite::Ind), ("definite", Definite::Def)])?;
    (t.len() == 3).then_some(NounCell { case, number, definite })
}

fn push(gold: &mut Gold, lemma: &str, cell: NounCell, form: &str) {
    let forms = gold.entry(lemma.to_string()).or_default().entry(cell).or_default();
    match forms.iter().any(|f| f == form) {
        true => {}
        false => forms.push(form.to_string()),
    }
}

/// Tabelat e lakimit të Wiktionary-t: rreshtat F me burim «declension» të hyrjeve «noun».
fn wiktionary() -> Gold {
    let text = std::fs::read_to_string(format!("{DATA}/raw/kaikki_en/kaikki_en-sq.tsv")).unwrap();
    let mut gold = Gold::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match (c.first().copied(), c.len() >= 4) {
            (Some("E"), true) => cur = (c[2] == "noun" && word(c[1])).then(|| c[1].to_string()),
            (Some("F"), true) => {
                let hit = cur.as_ref().filter(|_| c[3] == "declension" && word(c[1])).zip(wikt_cell(c[2]));
                let _ = hit.map(|(l, cell)| push(&mut gold, &l.clone(), cell, c[1]));
            }
            (Some("E"), false) => cur = None,
            _ => {}
        }
    }
    gold
}

/// UniMorph pa rreshtat GEN: ata janë kopje e DAT dhe moduli jep për gjinoren trajtën e
/// dhanores, ndaj do të numëroheshin dy herë.
fn unimorph() -> Gold {
    let text = std::fs::read_to_string(format!("{DATA}/raw/uniparser/unimorph-sqi.tsv")).unwrap();
    let mut gold = Gold::new();
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        let t: Vec<&str> = c.get(2).map(|s| s.split(';').collect()).unwrap_or_default();
        let case = pick(&t, &[("NOM", Case::Nom), ("DAT", Case::Dat), ("ACC", Case::Acc), ("ABL", Case::Abl)]);
        let number = pick(&t, &[("SG", Number::Sing), ("PL", Number::Plur)]);
        let definite = pick(&t, &[("NDEF", Definite::Ind), ("DEF", Definite::Def)]);
        let ok = c.len() == 3 && t.first() == Some(&"N") && word(c[0]) && word(c[1]);
        let cell = case.zip(number).zip(definite).filter(|_| ok).map(|((case, number), definite)| NounCell { case, number, definite });
        let _ = cell.map(|cell| push(&mut gold, c[0], cell, c[1]));
    }
    gold
}

const LETTERS: &str = "abcçdeëfghijklmnopqrstuvxyz";
const VOWELS: &str = "aeëiouy";

/// Theksi i shënuar hiqet (gjárpri → gjarpri).
fn unstress(s: &str) -> String {
    let plain = |c: char| match c {
        'á' | 'à' => 'a',
        'é' | 'è' => 'e',
        'í' | 'ì' => 'i',
        'ó' | 'ò' => 'o',
        'ú' | 'ù' => 'u',
        'ý' => 'y',
        other => other,
    };
    s.chars().map(plain).collect()
}

/// Ari i pastruar: pa theks, vetëm shkronja të alfabetit, me shkronjën e parë të lemës, pa
/// mbetjet e shabllonit me dy sibilante para «-sh» (blerëssh); qelizat që mbeten bosh hiqen.
fn cleaned(gold: &Gold) -> Gold {
    let sibilant = ["ssh", "zsh", "shsh", "zhsh"];
    let keep = |lemma: &str, f: &str| {
        let head = f.chars().next() == lemma.chars().next();
        let alphabet = f.chars().all(|c| LETTERS.contains(c));
        let template = sibilant.iter().any(|t| f.ends_with(t) || f.ends_with(&format!("{t}it")));
        head && alphabet && !template
    };
    let mut out = Gold::new();
    for (lemma, table) in gold {
        let l = unstress(lemma);
        for (cell, forms) in table {
            for f in forms.iter().map(|f| unstress(f)).filter(|f| keep(&l, f)) {
                push(&mut out, &l, *cell, &f);
            }
        }
    }
    out
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct M {
    total: usize,
    answered: usize,
    correct: usize,
}

impl M {
    fn add(&mut self, answer: Option<&Form>, gold: &[String]) {
        self.total += 1;
        let _ = answer.map(|f| {
            self.answered += 1;
            self.correct += usize::from(gold.contains(&f.text));
        });
    }

    fn plus(self, o: M) -> M {
        M { total: self.total + o.total, answered: self.answered + o.answered, correct: self.correct + o.correct }
    }

    fn precision(&self) -> f64 {
        self.correct as f64 / (self.answered.max(1)) as f64
    }

    fn coverage(&self) -> f64 {
        self.answered as f64 / (self.total.max(1)) as f64
    }

    /// Kufiri i poshtëm Wilson 95%: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n).
    fn wilson(&self) -> f64 {
        let n = self.answered as f64;
        let z = 1.96f64;
        let p = self.precision();
        match self.answered {
            0 => 0.0,
            _ => (p + z * z / (2.0 * n) - z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / (1.0 + z * z / n),
        }
    }

    fn row(&self, name: &str) -> String {
        format!(
            "{:<46} {:>7} {:>7} {:>7} {:>9.4} {:>9.4} {:>9.4}",
            name,
            self.total,
            self.answered,
            self.correct,
            self.precision(),
            self.coverage(),
            self.wilson()
        )
    }
}

fn header() -> String {
    format!("{:<46} {:>7} {:>7} {:>7} {:>9} {:>9} {:>9}", "metric", "total", "answer", "correct", "precision", "coverage", "wilson95")
}

fn split_name(s: Split) -> &'static str {
    match s {
        Split::Dev => "Dev",
        Split::Test => "Test",
    }
}

fn cell_name(c: &NounCell) -> String {
    format!("{:?}.{:?}.{:?}", c.case, c.definite, c.number)
}

/// Emërorja dhe kallëzorja e pashquar e njëjësit janë vetë lema: qeliza identike.
fn identity(c: &NounCell) -> bool {
    c.number == Number::Sing && c.definite == Definite::Ind && matches!(c.case, Case::Nom | Case::Acc)
}

/// Matja e një burimi ari: të gjitha qelizat, qelizat jo-identike, qeliza më vete, shumësi
/// sipas klasave të leksemës dhe paradigma e plotë për lemë.
struct Run {
    all: M,
    plain: M,
    cells: BTreeMap<NounCell, M>,
    classes: BTreeMap<String, M>,
    paradigm: M,
}

type Answer<'a> = &'a dyn Fn(&str, NounCell) -> Option<Form>;

fn run(gold: &Gold, split: Split, answer: Answer, keep: &dyn Fn(&str) -> bool, class: &dyn Fn(&str) -> String) -> Run {
    let mut r = Run { all: M::default(), plain: M::default(), cells: BTreeMap::new(), classes: BTreeMap::new(), paradigm: M::default() };
    for (lemma, table) in gold.iter().filter(|(l, _)| split_of(l) == split && keep(l)) {
        let mut lemma_answered = true;
        let mut lemma_correct = true;
        let name = class(lemma);
        for (cell, forms) in table {
            let a = answer(lemma, *cell);
            r.all.add(a.as_ref(), forms);
            let _ = (!identity(cell)).then(|| r.plain.add(a.as_ref(), forms));
            let _ = (cell.number == Number::Plur).then(|| r.classes.entry(name.clone()).or_default().add(a.as_ref(), forms));
            r.cells.entry(*cell).or_default().add(a.as_ref(), forms);
            lemma_answered &= a.is_some();
            lemma_correct &= a.as_ref().is_some_and(|f| forms.contains(&f.text));
        }
        r.paradigm.total += 1;
        r.paradigm.answered += usize::from(lemma_answered);
        r.paradigm.correct += usize::from(lemma_answered && lemma_correct);
    }
    r
}

/// Shuma e qelizave të një numri (njëjës ose shumës).
fn by_number(cells: &BTreeMap<NounCell, M>, number: Number) -> M {
    cells.iter().filter(|(c, _)| c.number == number).fold(M::default(), |a, (_, m)| a.plus(*m))
}

/// Matja kundrejt arit, Dev dhe Test; pragjet janë pak nën vlerën më të ulët të matur në të dyja.
#[test]
fn gold_measurement() {
    let lx = Lexicon::load(std::path::Path::new(DATA)).unwrap();
    let wikt = wiktionary();
    let clean = cleaned(&wikt);
    let um = unimorph();
    let by_lexicon = |l: &str, c: NounCell| decline_lemma(&lx, l, c);
    let by_shape = |l: &str, c: NounCell| decline_shape(l, c);
    let any = |_: &str| true;
    let unheld = |l: &str| !GENDER_HOLD.contains(&l);
    let missing = |l: &str| lx.entries(l).is_empty();
    let non_noun = |l: &str| !lx.entries(l).is_empty() && noun_lexeme(&lx, l).is_none();
    let class = |l: &str| noun_lexeme(&lx, l).map(|x| x.paradigms.join(",")).unwrap_or_else(|| "absent".to_string());

    let mut lines = vec![header()];
    let mut seen: BTreeMap<(&str, &str), M> = BTreeMap::new();
    let mut test_cells: BTreeMap<NounCell, M> = BTreeMap::new();
    let mut class_lines: Vec<String> = Vec::new();
    for split in [Split::Dev, Split::Test] {
        let s = split_name(split);
        let w = run(&wikt, split, &by_lexicon, &any, &class);
        let wc = run(&clean, split, &by_lexicon, &any, &class);
        let wh = run(&wikt, split, &by_lexicon, &unheld, &class);
        let u = run(&um, split, &by_lexicon, &any, &class);
        let sh = run(&wikt, split, &by_shape, &any, &class);
        let ms = run(&wikt, split, &by_lexicon, &missing, &class);
        let mc = run(&clean, split, &by_lexicon, &missing, &class);
        let nn = run(&wikt, split, &by_lexicon, &non_noun, &class);
        let rows = [
            ("wiktionary all cells", w.all),
            ("wiktionary non-identity cells", w.plain),
            ("wiktionary singular cells", by_number(&w.cells, Number::Sing)),
            ("wiktionary plural cells", by_number(&w.cells, Number::Plur)),
            ("wiktionary full paradigm/lemma", w.paradigm),
            ("wiktionary all cells, cleaned gold", wc.all),
            ("wiktionary non-identity, cleaned gold", wc.plain),
            ("wiktionary all cells, w/o GENDER_HOLD", wh.all),
            ("wiktionary non-identity, w/o GENDER_HOLD", wh.plain),
            ("unimorph all cells (no GEN)", u.all),
            ("unimorph full paradigm/lemma", u.paradigm),
            ("shape rules alone, all lemmas", sh.all),
            ("shape rules alone, non-identity", sh.plain),
            ("lemmas absent from lexicon", ms.all),
            ("lemmas absent from lexicon, cleaned gold", mc.all),
            ("lemmas lexicon has as non-noun", nn.all),
        ];
        for (name, m) in rows {
            lines.push(m.row(&format!("{s} {name}")));
            seen.insert((s, name), m);
        }
        let mut classes: Vec<(&String, &M)> = w.classes.iter().filter(|(_, m)| m.answered >= 40).collect();
        classes.sort_by_key(|(_, m)| std::cmp::Reverse(m.answered));
        class_lines.extend(classes.iter().map(|(k, m)| m.row(&format!("{s} plural cells, class {k}"))));
        let _ = (split == Split::Test).then(|| test_cells = w.cells.clone());
    }
    for (cell, m) in &test_cells {
        lines.push(m.row(&format!("Test wiktionary {}", cell_name(cell))));
    }
    lines.extend(class_lines);
    lines.push(format!("irregular table: {} entries (GENDER_HOLD, singular held; Test with/without above)", GENDER_HOLD.len()));
    eprintln!("\n{}", lines.join("\n"));

    let floor = |name: &str, precision: f64, coverage: f64| {
        for s in ["Dev", "Test"] {
            let m = seen[&(s, name)];
            assert!(m.precision() >= precision, "{s} {name}: precision {:.4} < {precision}", m.precision());
            assert!(m.coverage() >= coverage, "{s} {name}: coverage {:.4} < {coverage}", m.coverage());
        }
    };
    floor("wiktionary all cells", 0.97, 0.73);
    floor("wiktionary non-identity cells", 0.965, 0.70);
    floor("wiktionary singular cells", 0.98, 0.90);
    floor("wiktionary plural cells", 0.945, 0.55);
    floor("wiktionary full paradigm/lemma", 0.93, 0.58);
    floor("unimorph all cells (no GEN)", 0.97, 0.79);
    floor("shape rules alone, non-identity", 0.97, 0.40);
    floor("lemmas absent from lexicon", 0.955, 0.45);
    assert_eq!(seen[&("Dev", "lemmas lexicon has as non-noun")].answered, 0);
    assert_eq!(seen[&("Test", "lemmas lexicon has as non-noun")].answered, 0);
    for (cell, m) in &test_cells {
        let p = match cell.number {
            Number::Sing => 0.97,
            Number::Plur => 0.945,
        };
        assert!(m.precision() >= p, "{}: precision {:.4} < {p}", cell_name(cell), m.precision());
    }
}

/// Kufi temë-mbaresë që shqipja nuk e lejon, i lexuar nga fundi i trajtës së dhënë. `stems`
/// janë temat e leksemës: «-të», «-t», «-sh», «-shit» pas një teme që mbaron vetë me «-i»
/// (pashai → pashaitë, lei → leish) s'janë «-itë»/«-ish» pas zanores.
fn impossible(cell: &NounCell, s: &str, stems: &[String]) -> Option<&'static str> {
    let chars: Vec<char> = s.chars().collect();
    let at = |k: usize| chars.len().checked_sub(k).and_then(|i| chars.get(i)).copied();
    let vowel = |c: Option<char>| c.is_some_and(|c| VOWELS.contains(c));
    let cons = |c: Option<char>| c.is_some_and(|c| !VOWELS.contains(c));
    let ends = |p: &[&str]| p.iter().any(|x| s.ends_with(x));
    let before = |t: &str, set: &str| s.strip_suffix(t).and_then(|h| h.chars().last()).is_some_and(|c| set.contains(c));
    let bare = cell.definite == Definite::Ind && matches!(cell.case, Case::Nom | Case::Acc);
    let plural = cell.number == Number::Plur;
    let sg = cell.number == Number::Sing;
    let oblique = matches!(cell.case, Case::Dat | Case::Gen | Case::Abl);
    let glide = at(2) == Some('j') && vowel(at(3));
    let nom_def_sg = sg && cell.definite == Definite::Def && cell.case == Case::Nom;
    let stem_i = stems.iter().filter(|st| st.ends_with('i')).any(|st| s.strip_prefix(st.as_str()).is_some_and(|rest| ["t", "të", "sh", "shit"].contains(&rest)));
    let vowel_e = ["s", "n", "së", "në", "ve", "vet", "t", "sh", "shit"]
        .iter()
        .any(|t| s.strip_suffix(t).and_then(|h| h.strip_suffix('ë')).and_then(|h| h.chars().last()).is_some_and(|c| "aeiouy".contains(c)));
    let checks = [
        ("alphabet", !s.chars().all(|c| LETTERS.contains(c))),
        ("sibilant+sh", !bare && ends(&["ssh", "shsh", "zsh", "zhsh", "sshit", "shshit", "zshit", "zhshit"])),
        ("double t", !bare && ends(&["tt", "ttë"])),
        ("ë+të", !bare && s.ends_with("ëtë")),
        ("vowel+ë", !bare && vowel_e),
        ("vowel+e", sg && cell.definite == Definite::Ind && oblique && ends(&["ae", "oe", "ue"])),
        ("consonant+t", !bare && !nom_def_sg && s.ends_with('t') && cons(at(2)) && !glide && !ends(&["uart", "yert"])),
        ("consonant+së/në", !bare && ends(&["së", "në"]) && cons(at(3))),
        ("consonant+s/n", cell.definite == Definite::Def && !nom_def_sg && (s.ends_with('s') || s.ends_with('n')) && cons(at(2)) && !glide),
        ("plural -t after -i", plural && cell.definite == Definite::Def && s.ends_with("it") && !before("it", LETTERS)),
        ("plural vowel+i", plural && !bare && !stem_i && ["it", "itë", "ish", "ishit"].iter().any(|t| before(t, "aeëouy"))),
    ];
    checks.iter().find(|(_, hit)| *hit).map(|(name, _)| *name)
}

/// Kontroll mbi gjithë leksikun, pa ar: asnjë trajtë me kufi të pamundshëm dhe asnjë
/// përgjigje për fjalët që leksiku i njeh vetëm si pjesë të tjera ligjërate.
#[test]
fn lexicon_audit() {
    let lx = Lexicon::load(std::path::Path::new(DATA)).unwrap();
    let mut hits: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut forms = 0usize;
    for lex in lx.iter().filter(|l| l.pos == Pos::Noun) {
        let stems: Vec<String> = lex.clean_stems().into_iter().flatten().collect();
        for (cell, f) in paradigm(lex) {
            for s in std::iter::once(&f.text).chain(f.variants.iter()) {
                forms += 1;
                let _ = impossible(&cell, s, &stems).map(|p| hits.entry(p).or_default().push(format!("{} {} {s}", lex.lemma, cell_name(&cell))));
            }
        }
    }
    let mut lemmas: Vec<&str> = lx.iter().map(|l| l.lemma.as_str()).collect();
    lemmas.sort_unstable();
    lemmas.dedup();
    let others: Vec<&str> = lemmas.into_iter().filter(|l| noun_lexeme(&lx, l).is_none()).collect();
    let answered: Vec<&&str> = others.iter().filter(|l| CELLS.iter().any(|c| decline_lemma(&lx, l, *c).is_some())).collect();
    let summary: Vec<String> = hits.iter().map(|(p, v)| format!("{p}: {} {:?}", v.len(), v.iter().take(6).collect::<Vec<_>>())).collect();
    eprintln!(
        "\nlexicon audit: {forms} forms of noun lexemes, impossible boundaries {}\n{}\nnon-noun lexicon lemmas {}, answered {} {:?}",
        hits.values().map(Vec::len).sum::<usize>(),
        summary.join("\n"),
        others.len(),
        answered.len(),
        answered.iter().take(8).collect::<Vec<_>>()
    );
    assert!(hits.is_empty());
    assert!(answered.is_empty());
}
