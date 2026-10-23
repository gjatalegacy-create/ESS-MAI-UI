//! Matja e përemrit kundër dëshmive të jashtme: tabelat e Wiktionary-t (pron, det),
//! fjalëformat PRON/DET dhe grupet e klitikëve të UD-së, leksemat e Wikidata-s.
//! Ndarja zhvillim/provë sipas lemës; te nyja e përparme sipas lemës së fjalës që ajo
//! prin, sepse lema e nyjës është vetë trajta (i, e, të, së). Rregullat u ndreqën duke
//! parë vetëm Dev.
//! Kolona «precision» pranon tekstin dhe variantet; «first» vetëm tekstin e parë, atë
//! që Nura do të thoshte. Klitiku dhe trajta e plotë maten veç: zgjedhja mes tyre është
//! sintaksore, prandaj prova e merr nga klasa e trajtës (më, të, e, i, na, ju, u) dhe nga
//! parafjala në pemë; moduli duhet të japë qelizën e duhur.

use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::gramatika::peremri::*;
use nura_lingua::gramatika::tipet::*;
use std::collections::{BTreeMap, BTreeSet, HashSet};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Sub {
    Personal,
    Clitic,
    Cluster,
    Demonstrative,
    Possessive,
    Interrogative,
    Indefinite,
    Reflexive,
    Article,
    Invariant,
    Unknown,
}

#[derive(Debug)]
struct Item {
    source: &'static str,
    lemma: String,
    /// Lema e ndarjes zhvillim/provë: te nyja, lema e fjalës që ajo prin.
    split_key: String,
    sub: Sub,
    gold: Vec<String>,
    preds: Vec<Form>,
    /// Krahasohet vetëm fjala e fundit e trajtës: UD e ndan nyjën si fjalë më vete.
    strip: bool,
    closed: bool,
    key: String,
}

fn tail(s: &str, strip: bool) -> String {
    match strip {
        true => s.rsplit(' ').next().unwrap_or(s).to_string(),
        false => s.to_string(),
    }
}

impl Item {
    fn texts(&self) -> Vec<String> {
        self.preds
            .iter()
            .flat_map(|f| std::iter::once(&f.text).chain(f.variants.iter()).map(|t| tail(t, self.strip)))
            .collect()
    }
    fn first_text(&self) -> Option<String> {
        self.preds.first().map(|f| tail(&f.text, self.strip))
    }
    fn answered(&self) -> bool {
        !self.preds.is_empty()
    }
    fn correct(&self) -> bool {
        self.texts().iter().any(|t| self.gold.contains(t))
    }
    fn first(&self) -> bool {
        self.first_text().is_some_and(|t| self.gold.contains(&t))
    }
    /// Jehonë: dëshmia, lema dhe trajta e parë janë e njëjta fjalë.
    fn echo(&self) -> bool {
        self.gold.contains(&self.lemma) && self.first_text().as_deref() == Some(self.lemma.as_str())
    }
    /// Qeliza e Wiktionary-t pa nyjën «të» që moduli e jep («tanët» kundër «të tanët»).
    fn gold_drops_article(&self) -> bool {
        self.source == "kaikki" && self.preds.first().is_some_and(|f| self.gold.iter().any(|g| f.text == format!("të {g}")))
    }
    fn split(&self) -> Split {
        split_of(&self.split_key)
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct M {
    total: usize,
    answered: usize,
    correct: usize,
    first: usize,
}

impl M {
    fn precision(&self) -> f64 {
        ratio(self.correct, self.answered)
    }
    fn coverage(&self) -> f64 {
        ratio(self.answered, self.total)
    }
    /// Kufiri i poshtëm Wilson 95% i saktësisë:
    /// (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n), z = 1,96, n = të përgjigjurat.
    fn wilson(&self) -> f64 {
        let n = self.answered as f64;
        let z = 1.96f64;
        let p = self.precision();
        match self.answered {
            0 => 0.0,
            _ => (p + z * z / (2.0 * n) - z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / (1.0 + z * z / n),
        }
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    match b {
        0 => 0.0,
        _ => a as f64 / b as f64,
    }
}

fn measure_with<'a>(items: impl Iterator<Item = &'a Item>, ok: impl Fn(&Item) -> bool) -> M {
    let mut m = M::default();
    for it in items {
        m.total += 1;
        m.answered += it.answered() as usize;
        m.correct += (it.answered() && ok(it)) as usize;
        m.first += (it.answered() && it.first()) as usize;
    }
    m
}

fn measure<'a>(items: impl Iterator<Item = &'a Item>) -> M {
    measure_with(items, Item::correct)
}

fn row(label: &str, split: &str, m: &M) -> String {
    format!(
        "{label:<44} {split:<4} {:>6} {:>8} {:>7} {:>9.4} {:>9.4} {:>9.4} {:>9.4}",
        m.total,
        m.answered,
        m.correct,
        m.precision(),
        m.coverage(),
        m.wilson(),
        ratio(m.first, m.answered)
    )
}

fn split_name(s: Option<Split>) -> &'static str {
    match s {
        Some(Split::Dev) => "Dev",
        Some(Split::Test) => "Test",
        None => "All",
    }
}

fn in_split(it: &Item, s: Option<Split>) -> bool {
    s.map_or(true, |s| it.split() == s)
}

/// Përgjigje vetëm kur të gjitha rrugët japin të njëjtën fjalë (ose fjalë të fundit).
fn agree_tail(forms: Vec<Option<Form>>, strip: bool) -> Option<Form> {
    let all: Option<Vec<Form>> = forms.into_iter().collect();
    let all = all?;
    let first = all.first()?.clone();
    let t0 = tail(&first.text, strip);
    all.iter().all(|f| tail(&f.text, strip) == t0).then_some(first)
}

fn genders_or_both(g: Option<Gender>) -> Vec<Gender> {
    match g {
        Some(g) => vec![g],
        None => vec![Gender::Masc, Gender::Fem],
    }
}

fn numbers_or_both(n: Option<Number>) -> Vec<Number> {
    match n {
        Some(n) => vec![n],
        None => vec![Number::Sing, Number::Plur],
    }
}

fn family(lex: &Lexicon, lemma: &str) -> Sub {
    let pro = lex.entries(lemma).iter().filter(|l| l.standard && l.flags.iter().any(|f| f == "PRO"));
    let declinable = pro.clone().any(|l| l.paradigms.iter().any(|p| p != "unchangeable"));
    let fixed = pro.clone().any(|l| l.paradigms.iter().all(|p| p == "unchangeable"));
    match lemma {
        "unë" | "ti" | "ne" | "ju" | "ai" | "ajo" | "ata" | "ato" => Sub::Personal,
        "ky" | "kjo" | "këta" | "këto" => Sub::Demonstrative,
        "kush" | "cili" | "cila" | "i cili" | "e cila" | "çfarë" | "ç'" | "që" => Sub::Interrogative,
        "vete" | "vetja" | "vetvete" | "vetvetja" => Sub::Reflexive,
        "vet" | "i vet" | "veti" | "i veti" => Sub::Possessive,
        "disa" | "gjithë" | "të gjithë" | "të gjitha" => Sub::Indefinite,
        _ => match (owner_of(lemma).is_some(), declinable, fixed) {
            (true, _, _) => Sub::Possessive,
            (false, true, _) => Sub::Indefinite,
            (false, false, true) => Sub::Invariant,
            (false, false, false) => Sub::Unknown,
        },
    }
}

fn generic_sub(lex: &Lexicon, lemma: &str, pred: &Option<Form>) -> Sub {
    match pred.as_ref().map(|f| f.rule.starts_with("P-I1")) {
        Some(true) => Sub::Invariant,
        _ => family(lex, lemma),
    }
}

/// Një emër farefisnie për secilën gjini, për qelizat e tabelës së Wiktionary-t
/// (im atë, ime motër); gjinia asnjanëse nuk ka emër të tillë.
fn kin_of(g: Gender) -> &'static str {
    match g {
        Gender::Masc => "atë",
        Gender::Fem => "motër",
        Gender::Neut => "",
    }
}

// Dëshmia e Wiktionary-t: tabelat e lakimit të përemrave dhe përcaktorëve.

struct Entry {
    lemma: String,
    gender: String,
    canonical: String,
    rows: Vec<(String, String, String)>,
}

fn kaikki_entries() -> Vec<Entry> {
    let text = std::fs::read_to_string(format!("{DATA}/raw/kaikki_en/kaikki_en-sq.tsv")).unwrap();
    let mut out: Vec<Entry> = Vec::new();
    let mut keep = false;
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        let get = |i: usize| c.get(i).copied().unwrap_or("").trim().to_string();
        match c.first().copied() {
            Some("E") => {
                keep = matches!(c.get(2).copied(), Some("pron") | Some("det"));
                match keep {
                    true => out.push(Entry { lemma: get(1), gender: get(3), canonical: get(4), rows: Vec::new() }),
                    false => {}
                }
            }
            Some("F") => match (keep, out.last_mut()) {
                (true, Some(e)) => e.rows.push((get(1), get(2), get(3))),
                _ => {}
            },
            _ => {}
        }
    }
    out
}

#[derive(Default)]
struct Tags {
    cases: Vec<Case>,
    genders: Vec<Gender>,
    number: Option<Number>,
    skip: bool,
}

fn parse_tags(s: &str) -> Tags {
    let mut t = Tags::default();
    for tag in s.split(',') {
        match tag {
            "nominative" => t.cases.push(Case::Nom),
            "accusative" => t.cases.push(Case::Acc),
            "dative" => t.cases.push(Case::Dat),
            "ablative" => t.cases.push(Case::Abl),
            "genitive" => t.cases.push(Case::Gen),
            "masculine" => t.genders.push(Gender::Masc),
            "feminine" => t.genders.push(Gender::Fem),
            "singular" => t.number = Some(Number::Sing),
            "plural" => t.number = Some(Number::Plur),
            "alternative" | "canonical" | "clitic" => t.skip = true,
            _ => {}
        }
    }
    t.genders.sort();
    t
}

/// «kë (kënd)» → kë, kënd; «(a)tij» → atij, tij.
fn expand(form: &str) -> Vec<String> {
    let f = form.to_lowercase();
    match (f.split_once(" ("), f.strip_prefix('(').and_then(|r| r.split_once(')'))) {
        (Some((a, b)), _) => vec![a.trim().to_string(), b.trim_end_matches(')').trim().to_string()],
        (None, Some((opt, rest))) => vec![format!("{opt}{rest}"), rest.to_string()],
        (None, None) => vec![f],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Attr,
    Kin,
    Subst,
}

/// Gjinia dhe numri që hyrja jep në fushën e saj të gjinisë.
fn raw_gender(g: &str) -> (Option<Gender>, Option<Number>) {
    match g {
        "m" => (Some(Gender::Masc), None),
        "f" => (Some(Gender::Fem), None),
        "m-p" => (Some(Gender::Masc), Some(Number::Plur)),
        "f-p" => (Some(Gender::Fem), Some(Number::Plur)),
        "n-s" => (Some(Gender::Neut), Some(Number::Sing)),
        _ => (None, None),
    }
}

/// Rruga e parashikimit merret nga vetë hyrja: pronari nga `owner_of`, trajta pa emër
/// kur lema është emërorja e tabelës pa emër, lidhori nga trajta kanonike («i cili»);
/// çdo lemë tjetër kalon nëpër `inflect`.
fn kaikki_pred(
    lex: &Lexicon,
    e: &Entry,
    kind: Kind,
    case: Case,
    genders: &[Gender],
    number: Option<Number>,
) -> (Sub, Option<Form>) {
    let l = e.lemma.as_str();
    let over_g = |f: &dyn Fn(Gender) -> Option<Form>| match genders {
        [g] => f(*g),
        _ => agree_tail(vec![f(Gender::Masc), f(Gender::Fem)], false),
    };
    match owner_of(l) {
        Some((op, on, og)) => {
            let subst_lemma = [Gender::Masc, Gender::Fem].iter().any(|g| {
                possessive_substantive(op, on, og, *g, Number::Sing, Case::Nom)
                    .is_some_and(|f| tail(&f.text, true) == l)
            });
            let kind = match subst_lemma {
                true => Kind::Subst,
                false => kind,
            };
            let p = match kind {
                Kind::Attr => number.and_then(|n| over_g(&|g| possessive(op, on, og, g, n, case, Definite::Def))),
                Kind::Subst => number.and_then(|n| over_g(&|g| possessive_substantive(op, on, og, g, n, case))),
                Kind::Kin => match number {
                    Some(Number::Plur) => None,
                    _ => over_g(&|g| possessive_kinship(op, on, kin_of(g), case)),
                },
            };
            (Sub::Possessive, p)
        }
        None => {
            let canonical = e.canonical.as_str();
            let relative = canonical.starts_with("i ") || canonical.starts_with("e ");
            let lemma = Some(canonical).filter(|c| relative && is_closed(c)).unwrap_or(l);
            let (rg, rn) = raw_gender(&e.gender);
            let q = |g: Option<Gender>| peremri_query(g, number.or(rn), Some(case), None);
            let p = match genders {
                [] => inflect(lex, lemma, &q(rg)),
                _ => over_g(&|g| inflect(lex, lemma, &q(Some(g)))),
            };
            (generic_sub(lex, lemma, &p), p)
        }
    }
}

fn peremri_query(g: Option<Gender>, n: Option<Number>, c: Option<Case>, d: Option<Definite>) -> Query {
    Query { gender: g, number: n, case: c, definite: d }
}

fn kaikki_items(lex: &Lexicon) -> Vec<Item> {
    let mut out = Vec::new();
    for e in kaikki_entries() {
        let first = e.rows.iter().find(|r| r.2 == "declension" && r.0 != "Term?").map(|r| r.1.clone());
        let mut table = 0usize;
        let mut seen = false;
        let mut cells: BTreeMap<(usize, Case, Vec<Gender>, Option<Number>), BTreeSet<String>> = BTreeMap::new();
        for (form, tags, source) in &e.rows {
            let decl = source == "declension";
            let t = match (decl, Some(tags) == first.as_ref()) {
                (true, true) => {
                    table += seen as usize;
                    seen = true;
                    table
                }
                (true, false) => table,
                (false, _) => 0,
            };
            let tg = parse_tags(tags);
            let usable = !tg.skip && form != "Term?" && !form.contains('~');
            for c in tg.cases.iter().filter(|_| usable) {
                cells.entry((t, *c, tg.genders.clone(), tg.number)).or_default().extend(expand(form));
            }
        }
        let has_plural: HashSet<usize> =
            cells.keys().filter(|k| k.3 == Some(Number::Plur)).map(|k| k.0).collect();
        for ((t, case, genders, number), gold) in &cells {
            let kind = match (*t, has_plural.contains(t)) {
                (0, _) => Kind::Attr,
                (_, true) => Kind::Subst,
                (_, false) => Kind::Kin,
            };
            let (sub, pred) = kaikki_pred(lex, &e, kind, *case, genders, *number);
            out.push(Item {
                source: "kaikki",
                lemma: e.lemma.clone(),
                split_key: e.lemma.clone(),
                sub,
                gold: gold.iter().cloned().collect(),
                preds: pred.into_iter().collect(),
                strip: false,
                closed: is_closed(&e.lemma) || matches!(sub, Sub::Personal | Sub::Demonstrative | Sub::Possessive),
                key: format!("{} t{} {:?} {:?} {:?} {:?}", e.lemma, t, kind, case, genders, number),
            });
        }
    }
    out
}

// Dëshmia e UD-së: fjalëformat PRON/DET dhe grupet e klitikëve (fjalët shumëpjesëshe).

#[derive(Debug, Clone)]
struct Tok {
    id: usize,
    form: String,
    lemma: String,
    upos: String,
    feats: String,
    head: usize,
    deprel: String,
}

fn feat<'a>(t: &'a Tok, k: &str) -> Option<&'a str> {
    ud_feat(&t.feats, k)
}

fn ud_person(s: &str) -> Option<Person> {
    match s {
        "1" => Some(Person::P1),
        "2" => Some(Person::P2),
        "3" => Some(Person::P3),
        _ => None,
    }
}

fn ud_definite(s: &str) -> Option<Definite> {
    match s {
        "Def" => Some(Definite::Def),
        "Ind" => Some(Definite::Ind),
        _ => None,
    }
}

type Sentence = (Vec<Tok>, Vec<(usize, usize, String)>);

fn ud_sentences() -> Vec<Sentence> {
    let files = [
        "raw/ud_sq_staf/sq_staf-ud-train.conllu",
        "raw/ud_sq_staf/sq_staf-ud-dev.conllu",
        "raw/ud_sq_staf/sq_staf-ud-test.conllu",
        "raw/ud_sq_tsa/sq_tsa-ud-test.conllu",
    ];
    let mut out = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(format!("{DATA}/{f}")).unwrap();
        for block in text.split("\n\n").map(str::trim).filter(|b| !b.is_empty()) {
            let mut toks = Vec::new();
            let mut mwt = Vec::new();
            for line in block.lines().filter(|l| !l.starts_with('#')) {
                let c: Vec<&str> = line.split('\t').collect();
                match (c.len() == 10, c.first().and_then(|id| id.split_once('-'))) {
                    (true, Some((a, b))) => mwt.push((a.parse().unwrap(), b.parse().unwrap(), c[1].to_lowercase())),
                    (true, None) => match c[0].parse::<usize>() {
                        Ok(id) => toks.push(Tok {
                            id,
                            form: c[1].to_lowercase().replace('’', "'"),
                            lemma: c[2].to_lowercase().replace('’', "'"),
                            upos: c[3].to_string(),
                            feats: c[5].to_string(),
                            head: c[6].parse().unwrap_or(0),
                            deprel: c[7].to_string(),
                        }),
                        Err(_) => {}
                    },
                    (false, _) => {}
                }
            }
            out.push((toks, mwt));
        }
    }
    out
}

/// Veta, numri dhe gjinia që mbart lema e përemrit vetor në UD.
fn ident(lemma: &str) -> Option<(Option<Person>, Option<Number>, Option<Gender>)> {
    let (p1, p2, p3) = (Some(Person::P1), Some(Person::P2), Some(Person::P3));
    let (s, p) = (Some(Number::Sing), Some(Number::Plur));
    let (m, f) = (Some(Gender::Masc), Some(Gender::Fem));
    match lemma {
        "unë" | "mua" | "meje" | "më" => Some((p1, s, None)),
        "ti" | "ty" | "teje" => Some((p2, s, None)),
        "ne" | "neve" | "nesh" | "na" => Some((p1, p, None)),
        "ju" | "juve" | "jush" => Some((p2, p, None)),
        "ai" | "atij" | "atë" => Some((p3, s, m)),
        "ajo" | "asaj" => Some((p3, s, f)),
        "ata" | "atyre" => Some((p3, p, m)),
        "ato" => Some((p3, p, f)),
        "e" | "i" => Some((p3, None, None)),
        "u" => Some((p3, p, None)),
        "të" => Some((None, s, None)),
        _ => None,
    }
}

fn near(lemma: &str) -> Option<(Option<Gender>, Option<Number>)> {
    let (s, p) = (Some(Number::Sing), Some(Number::Plur));
    let (m, f) = (Some(Gender::Masc), Some(Gender::Fem));
    match lemma {
        "ky" | "këtij" => Some((m, s)),
        "kjo" | "kësaj" => Some((f, s)),
        "këta" => Some((m, p)),
        "këto" => Some((f, p)),
        "këtë" => Some((None, s)),
        "këtyre" => Some((None, p)),
        _ => None,
    }
}

fn far(lemma: &str) -> Option<(Option<Gender>, Option<Number>)> {
    let (s, p) = (Some(Number::Sing), Some(Number::Plur));
    let (m, f) = (Some(Gender::Masc), Some(Gender::Fem));
    match lemma {
        "ai" | "atij" => Some((m, s)),
        "ajo" | "asaj" => Some((f, s)),
        "ata" => Some((m, p)),
        "ato" => Some((f, p)),
        "atë" => Some((None, s)),
        "atyre" => Some((None, p)),
        _ => None,
    }
}

struct Probe {
    sub: Sub,
    preds: Vec<Form>,
    strip: bool,
    closed: bool,
    ctx: String,
    /// Dëshmia e plotë kur UD e ndan nyjën e pronorit si fjalë më vete («e tij»).
    gold: Option<String>,
    split_key: Option<String>,
    /// Nyja e mbiemrit të emërzuar ose kallëzuesor nuk i përket përemrit: jashtë matjes.
    scope: bool,
}

fn probe(p: Option<Form>, sub: Sub, strip: bool, closed: bool, ctx: String) -> Probe {
    Probe { sub, preds: p.into_iter().collect(), strip, closed, ctx, gold: None, split_key: None, scope: true }
}

fn is_article_lemma(l: &str) -> bool {
    matches!(l, "i" | "e" | "të" | "së")
}

fn ud_probe(lex: &Lexicon, t: &Tok, sent: &[Tok]) -> Probe {
    let lemma = t.lemma.as_str();
    let case = feat(t, "Case").and_then(ud_case);
    let gender = feat(t, "Gender").and_then(ud_gender);
    let number = feat(t, "Number").and_then(ud_number);
    let person = feat(t, "Person").and_then(ud_person);
    let definite = feat(t, "Definite").and_then(ud_definite);
    let pron_type = feat(t, "PronType").unwrap_or("");
    let poss = feat(t, "Poss") == Some("Yes");
    let by_id = |id: usize| sent.iter().find(|x| x.id == id);
    let head = by_id(t.head);
    let nominal = |x: &&Tok| matches!(x.upos.as_str(), "NOUN" | "PROPN" | "PRON");
    let is_article = t.upos == "DET" && (pron_type == "Art" || is_article_lemma(lemma));
    let owner = owner_of(lemma).or_else(|| {
        poss.then(|| ident(lemma))
            .flatten()
            .and_then(|(p, n, g)| Some((p?, n?, g.filter(|_| p == Some(Person::P3) && n == Some(Number::Sing)))))
    });
    let far_dem = far(lemma).filter(|_| pron_type == "Dem");
    let pers = ident(lemma).filter(|_| t.upos == "PRON");
    match (is_article, owner, near(lemma), far_dem, pers) {
        (true, ..) => {
            // Nyja pas një nyje tjetër (të së vërtetës, e së cilës) i përket vetë emrit pas saj;
            // nyja pa emër prijës (është i mirë, të mirët) është nyja e lirë.
            let after_article =
                by_id(t.id.wrapping_sub(1)).is_some_and(|p| p.upos == "DET" && is_article_lemma(&p.lemma));
            let gov = head.and_then(|h| by_id(h.head)).filter(nominal).filter(|_| !after_article);
            let head_pronoun = head.is_some_and(|h| matches!(h.upos.as_str(), "PRON" | "DET"));
            let ctrl = match after_article {
                true => head.filter(nominal),
                false => gov,
            };
            let case = ctrl.and_then(|x| feat(x, "Case")).and_then(ud_case).or(case);
            let gender = ctrl.and_then(|x| feat(x, "Gender")).and_then(ud_gender).or(gender);
            let number = ctrl.and_then(|x| feat(x, "Number")).and_then(ud_number).or(number);
            let gdef = gov.and_then(|g| feat(g, "Definite")).and_then(ud_definite);
            let adjacent = gov.map(|g| g.id + 1 == t.id);
            let ctx = format!("{:?}/{:?}/{}/{:?}{:?}{:?}", gdef, adjacent, after_article, case, gender, number);
            let options: Vec<(Definite, bool)> = match (gdef, adjacent) {
                (Some(Definite::Ind), _) => vec![(Definite::Ind, false)],
                (Some(Definite::Def), Some(a)) => vec![(Definite::Def, a)],
                _ => vec![(Definite::Def, true), (Definite::Def, false), (Definite::Ind, false)],
            };
            let p = case.and_then(|c| {
                let mut all = Vec::new();
                for g in genders_or_both(gender) {
                    for n in numbers_or_both(number) {
                        match gov {
                            None => all.push(free_article(g, n, c)),
                            Some(_) => {
                                for (d, a) in &options {
                                    all.push(article(g, n, c, *d, *a));
                                }
                            }
                        }
                    }
                }
                agree_tail(all, false)
            });
            Probe {
                split_key: head.map(|h| h.lemma.clone()),
                scope: gov.is_some() || head_pronoun,
                ..probe(p, Sub::Article, false, true, ctx)
            }
        }
        (false, Some((op, on, og)), ..) => {
            let hdef = head.filter(nominal).and_then(|h| feat(h, "Definite")).and_then(ud_definite);
            // Pronori para emrit të vet është ndërtimi me emër farefisnie (im atë, jot motër).
            let prenominal = head.filter(nominal).is_some_and(|h| h.id > t.id);
            let kin = head.map_or("", |h| h.lemma.as_str());
            // Nyja e pronorit, kur UD e ndan si fjalë më vete, matet bashkë me të: «e tij», «të tanët».
            let art = by_id(t.id.wrapping_sub(1)).filter(|a| {
                a.head == t.id && matches!(a.upos.as_str(), "DET" | "PART") && is_article_lemma(&a.lemma)
            });
            let strip = art.is_none();
            // Pronori përshtatet me emrin e zotëruar: tiparet e tij merren nga emri prijës.
            // Te tij/saj/tyre UD shënon shpesh gjininë e pronarit dhe rasën e vetë përemrit.
            let possessed = head.filter(nominal);
            let case = possessed.and_then(|x| feat(x, "Case")).and_then(ud_case).or(case);
            let own_gender = gender.filter(|_| op != Person::P3);
            let gender = possessed.and_then(|x| feat(x, "Gender")).and_then(ud_gender).or(own_gender);
            let number = possessed.and_then(|x| feat(x, "Number")).and_then(ud_number).or(number);
            let ctx = format!(
                "{:?}/{}/{}/{:?}{:?}{:?}",
                hdef,
                prenominal,
                art.map_or("", |a| a.form.as_str()),
                case,
                gender,
                number
            );
            let p = case.and_then(|c| {
                let mut all = Vec::new();
                for g in genders_or_both(gender) {
                    for n in numbers_or_both(number) {
                        match (prenominal, definite) {
                            (true, _) => all.push(possessive_kinship(op, on, kin, c).filter(|_| n == Number::Sing)),
                            (false, Some(Definite::Def)) => {
                                all.push(possessive_substantive(op, on, og, g, n, c))
                            }
                            (false, _) => {
                                for d in hdef.map_or(vec![Definite::Def, Definite::Ind], |d| vec![d]) {
                                    all.push(possessive(op, on, og, g, n, c, d));
                                }
                            }
                        }
                    }
                }
                agree_tail(all, strip)
            });
            Probe {
                gold: art.map(|a| format!("{} {}", a.form, t.form)),
                ..probe(p, Sub::Possessive, strip, true, ctx)
            }
        }
        (false, None, Some((g0, n0)), ..) => {
            let p = number.or(n0).zip(case).and_then(|(n, c)| demonstrative(Deixis::Near, gender.or(g0), n, c));
            probe(p, Sub::Demonstrative, false, true, String::new())
        }
        (false, None, None, Some((g0, n0)), _) => {
            let p = number.or(n0).zip(case).and_then(|(n, c)| demonstrative(Deixis::Far, gender.or(g0), n, c));
            probe(p, Sub::Demonstrative, false, true, String::new())
        }
        (false, None, None, None, Some((ip, inum, ig))) => {
            let after_prep = sent.iter().any(|x| x.head == t.id && x.deprel == "case");
            let short = matches!(t.form.as_str(), "më" | "të" | "e" | "i" | "na" | "ju" | "u")
                && !after_prep
                && matches!(case, Some(Case::Dat | Case::Acc));
            let p = match (person.or(ip), number.or(inum), case) {
                (Some(p), Some(n), Some(c)) => match (short, after_prep) {
                    (true, _) => clitic(p, n, c),
                    (false, true) => prepositional(p, n, gender.or(ig), c),
                    (false, false) => personal(p, n, gender.or(ig), c),
                },
                _ => None,
            };
            let sub = match short {
                true => Sub::Clitic,
                false => Sub::Personal,
            };
            probe(p, sub, false, true, format!("{short}/{after_prep}"))
        }
        (false, None, None, None, None) => {
            let rel = matches!(lemma, "cili" | "cila") && pron_type == "Rel";
            let l2 = match (rel, lemma) {
                (true, "cili") => "i cili",
                (true, _) => "e cila",
                (false, l) => l,
            };
            let q = Query { gender, number, case, definite };
            // «vet» si pronor vetvetor: shquarja e emrit prijës merret nga pema.
            let hdef = head.filter(nominal).and_then(|h| feat(h, "Definite")).and_then(ud_definite);
            let p = match (l2, case) {
                ("vet", Some(c)) => {
                    let mut all = Vec::new();
                    for g in genders_or_both(gender) {
                        for n in numbers_or_both(number) {
                            for d in hdef.map_or(vec![Definite::Def, Definite::Ind], |d| vec![d]) {
                                all.push(own(g, n, c, d));
                            }
                        }
                    }
                    agree_tail(all, true)
                }
                _ => inflect(lex, l2, &q),
            };
            let sub = generic_sub(lex, lemma, &p);
            probe(p, sub, true, is_closed(l2), String::new())
        }
    }
}

/// Fjalëformat e UD-së dhe numri i nyjave jashtë fushës së përemrit.
fn ud_items(lex: &Lexicon) -> (Vec<Item>, usize) {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut out_of_scope = 0usize;
    for (toks, mwts) in ud_sentences() {
        for t in toks.iter().filter(|t| matches!(t.upos.as_str(), "PRON" | "DET")) {
            let pr = ud_probe(lex, t, &toks);
            out_of_scope += (!pr.scope) as usize;
            let key = format!("{}|{}|{}|{}|{}", t.lemma, t.form, t.upos, t.feats, pr.ctx);
            match pr.scope && seen.insert(key.clone()) {
                true => out.push(Item {
                    source: "ud",
                    lemma: t.lemma.clone(),
                    split_key: pr.split_key.unwrap_or_else(|| t.lemma.clone()),
                    sub: pr.sub,
                    gold: vec![pr.gold.unwrap_or_else(|| t.form.clone())],
                    preds: pr.preds,
                    strip: pr.strip,
                    closed: pr.closed,
                    key,
                }),
                false => {}
            }
        }
        for (a, b, form) in &mwts {
            let parts: Vec<&Tok> = toks.iter().filter(|t| t.id >= *a && t.id <= *b).collect();
            let roles: Option<Vec<(Case, Person, Number)>> = parts
                .iter()
                .map(|t| {
                    (t.upos == "PRON").then_some(())?;
                    let c = feat(t, "Case").and_then(ud_case).filter(|c| matches!(c, Case::Dat | Case::Acc))?;
                    let id = ident(&t.lemma);
                    let p = feat(t, "Person").and_then(ud_person).or(id.and_then(|x| x.0))?;
                    let n = feat(t, "Number").and_then(ud_number).or(id.and_then(|x| x.1))?;
                    Some((c, p, n))
                })
                .collect();
            let roles = roles.filter(|r| r.len() >= 2);
            let cell = roles.and_then(|r| {
                let dat: Vec<(Person, Number)> = r.iter().filter(|x| x.0 == Case::Dat).map(|x| (x.1, x.2)).collect();
                let acc: Vec<(Person, Number)> = r.iter().filter(|x| x.0 == Case::Acc).map(|x| (x.1, x.2)).collect();
                (dat.len() <= 1 && acc.len() <= 1).then(|| (dat.first().copied(), acc.first().copied()))
            });
            match cell {
                Some((d, ac)) => {
                    let key = format!("mwt|{form}|{d:?}|{ac:?}");
                    match seen.insert(key.clone()) {
                        true => out.push(Item {
                            source: "ud",
                            lemma: form.clone(),
                            split_key: form.clone(),
                            sub: Sub::Cluster,
                            gold: vec![form.clone()],
                            preds: cluster(d, ac).into_iter().collect(),
                            strip: false,
                            closed: true,
                            key,
                        }),
                        false => {}
                    }
                }
                None => {}
            }
        }
    }
    (out, out_of_scope)
}

// Dëshmia e Wikidata-s: leksemat e përemrave.

fn wd_case(q: &str) -> Option<Case> {
    match q {
        "Q131105" => Some(Case::Nom),
        "Q146078" => Some(Case::Acc),
        "Q145599" => Some(Case::Dat),
        "Q156986" => Some(Case::Abl),
        "Q146233" => Some(Case::Gen),
        _ => None,
    }
}

fn wd_items(lex: &Lexicon) -> Vec<Item> {
    let text = std::fs::read_to_string(format!("{DATA}/raw/wd_lexemes/wd_lexemes-sq.tsv")).unwrap();
    let pronoun_cats = ["Q468801", "Q36224", "Q1050744", "Q576271", "Q953129", "Q1641446"];
    let lexemes: BTreeMap<String, String> = text
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|c| c.first() == Some(&"L") && c.get(3).is_some_and(|q| pronoun_cats.contains(q)))
        .map(|c| (c[1].to_string(), c[2].to_lowercase()))
        .collect();
    let mut cells: BTreeMap<(String, Case), BTreeSet<String>> = BTreeMap::new();
    for c in text.lines().map(|l| l.split('\t').collect::<Vec<_>>()).filter(|c| c.first() == Some(&"F")) {
        let lemma = c.get(1).and_then(|l| lexemes.get(*l));
        let case = c.get(4).and_then(|f| f.split(',').find_map(wd_case));
        match (lemma, case, c.get(3)) {
            (Some(l), Some(cs), Some(form)) => {
                cells.entry((l.clone(), cs)).or_default().insert(form.to_lowercase());
            }
            _ => {}
        }
    }
    cells
        .into_iter()
        .map(|((lemma, case), gold)| {
            let p = inflect(lex, &lemma, &Query { case: Some(case), ..Query::default() });
            Item {
                source: "wd",
                sub: family(lex, &lemma),
                closed: is_closed(&lemma),
                key: format!("{lemma} {case:?}"),
                split_key: lemma.clone(),
                lemma,
                gold: gold.into_iter().collect(),
                preds: p.into_iter().collect(),
                strip: false,
            }
        })
        .collect()
}

// Raporti dhe kufijtë e poshtëm.

const SUBS: [Sub; 11] = [
    Sub::Personal,
    Sub::Clitic,
    Sub::Cluster,
    Sub::Demonstrative,
    Sub::Possessive,
    Sub::Interrogative,
    Sub::Indefinite,
    Sub::Reflexive,
    Sub::Article,
    Sub::Invariant,
    Sub::Unknown,
];

const SPLITS: [Option<Split>; 2] = [Some(Split::Dev), Some(Split::Test)];

#[test]
fn gold_pronouns() {
    let lex = Lexicon::load(std::path::Path::new(DATA)).unwrap();
    let mut items = kaikki_items(&lex);
    let (ud, out_of_scope) = ud_items(&lex);
    items.extend(ud);
    items.extend(wd_items(&lex));

    eprintln!(
        "{:<44} {:<4} {:>6} {:>8} {:>7} {:>9} {:>9} {:>9} {:>9}",
        "metric", "split", "total", "answered", "correct", "precision", "coverage", "wilson95", "first"
    );
    let mut table: BTreeMap<String, M> = BTreeMap::new();
    let mut put = |label: String, key: String, sp: Option<Split>, m: M| {
        eprintln!("{}", row(&label, split_name(sp), &m));
        table.insert(format!("{key}/{}", split_name(sp)), m);
    };
    for src in ["kaikki", "ud", "wd", "all"] {
        for sp in SPLITS {
            let m = measure(items.iter().filter(|i| (src == "all" || i.source == src) && in_split(i, sp)));
            put(format!("source:{src}"), src.to_string(), sp, m);
        }
    }
    for sub in SUBS {
        for sp in [Some(Split::Dev), Some(Split::Test), None] {
            let m = measure(items.iter().filter(|i| i.sub == sub && in_split(i, sp)));
            put(format!("sub:{sub:?}"), format!("{sub:?}"), sp, m);
        }
    }
    for sp in SPLITS {
        let m = measure(items.iter().filter(|i| !i.closed && in_split(i, sp)));
        put("all without closed tables".to_string(), "open".to_string(), sp, m);
        let m = measure(items.iter().filter(|i| i.closed && in_split(i, sp)));
        put("all closed tables only".to_string(), "closed".to_string(), sp, m);
        let m = measure(items.iter().filter(|i| !i.echo() && in_split(i, sp)));
        put("all without echo items (gold=lemma=pred)".to_string(), "noecho".to_string(), sp, m);
        let m = measure(items.iter().filter(|i| !i.closed && !i.echo() && in_split(i, sp)));
        put("without closed tables, without echo".to_string(), "open-noecho".to_string(), sp, m);
    }
    for sp in SPLITS {
        let arts: Vec<&Item> = items.iter().filter(|i| i.sub == Sub::Article && in_split(i, sp)).collect();
        let hits = arts.iter().filter(|i| i.gold.iter().any(|g| g == "të")).count();
        let base = M { total: arts.len(), answered: arts.len(), correct: hits, first: hits };
        put("baseline: Article always «të»".to_string(), "baseline-te".to_string(), sp, base);
    }
    for sp in SPLITS {
        let kaikki = || items.iter().filter(|i| i.source == "kaikki" && in_split(i, sp));
        let m = measure_with(kaikki(), |i| i.correct() || i.gold_drops_article());
        put("kaikki, gold drops «të» counted correct".to_string(), "kaikki-art".to_string(), sp, m);
        eprintln!("  kaikki {} cells where the gold drops «të»: {}", split_name(sp), kaikki().filter(|i| i.gold_drops_article()).count());
    }
    eprintln!("UD articles out of scope (substantivized or predicative adjective, noun head): {out_of_scope}");
    for sub in SUBS {
        let m = measure(items.iter().filter(|i| i.sub == sub && i.split() == Split::Test));
        match m.answered < 10 {
            true => eprintln!("thin or no held-out evidence: sub:{sub:?} Test total {} answered {}", m.total, m.answered),
            false => {}
        }
    }
    eprintln!("no held-out evidence: clitic doubling (no gold annotates doubling; unit tests only)");
    eprintln!("closed-table cells answered without the lexicon: {}", closed_cells());

    let dev_miss: Vec<&Item> =
        items.iter().filter(|i| i.split() == Split::Dev && i.answered() && !i.correct()).collect();
    eprintln!("-- Dev mismatches: {}", dev_miss.len());
    for i in &dev_miss {
        eprintln!("  [{}] {:?} {} gold={:?} pred={:?}", i.source, i.sub, i.key, i.gold, i.texts());
    }
    let dev_open: Vec<&Item> = items.iter().filter(|i| i.split() == Split::Dev && !i.answered()).collect();
    eprintln!("-- Dev unanswered: {}", dev_open.len());
    for i in dev_open.iter().take(60) {
        eprintln!("  [{}] {:?} {} gold={:?}", i.source, i.sub, i.key, i.gold);
    }

    let p = |k: &str| table.get(k).map(|m| m.precision()).unwrap_or(0.0);
    let w = |k: &str| table.get(k).map(|m| m.wilson()).unwrap_or(0.0);
    let c = |k: &str| table.get(k).map(|m| m.coverage()).unwrap_or(0.0);
    let f = |k: &str| table.get(k).map(|m| ratio(m.first, m.answered)).unwrap_or(0.0);
    let lift = |k: &str| p(&format!("Article/{k}")) - p(&format!("baseline-te/{k}"));
    assert!(p("kaikki/Dev") >= 0.92 && p("kaikki/Test") >= 0.92, "Wiktionary");
    assert!(p("kaikki-art/Dev") >= 0.96 && p("kaikki-art/Test") >= 0.97, "Wiktionary pa nyjën «të»");
    assert!(p("ud/Dev") >= 0.86 && p("ud/Test") >= 0.85, "UD");
    assert!(p("all/Dev") >= 0.88 && p("all/Test") >= 0.88, "të gjitha");
    assert!(w("all/Test") >= 0.84 && c("all/Test") >= 0.79 && f("all/Test") >= 0.87, "Wilson, mbulimi, trajta e parë");
    assert!(p("noecho/Dev") >= 0.81 && p("noecho/Test") >= 0.84, "pa jehonat");
    assert!(p("Possessive/Dev") >= 0.84 && p("Possessive/Test") >= 0.88, "pronori");
    assert!(p("Interrogative/Dev") >= 0.96, "pyetësi");
    assert!(p("Demonstrative/Test") >= 0.85, "dëftori");
    assert!(p("Personal/Dev") >= 0.85 && p("Personal/Test") >= 0.90, "vetori");
    assert!(p("Clitic/Dev") >= 0.85 && p("Clitic/Test") >= 0.79, "klitiku");
    assert!(p("Article/Dev") >= 0.89 && p("Article/Test") >= 0.82, "nyja");
    assert!(lift("Dev") >= 0.30 && lift("Test") >= 0.30, "nyja mbi bazën «të»");
    assert!(p("Cluster/Test") >= 0.99, "grupet e klitikëve");
    assert!(p("open/Test") >= 0.97 && p("closed/Test") >= 0.86, "me dhe pa tabelë");
}

/// Numri i qelizave që tabelat e mbyllura i përgjigjen pa leksik.
fn closed_cells() -> usize {
    let persons = [Person::P1, Person::P2, Person::P3];
    let numbers = [Number::Sing, Number::Plur];
    let genders = [Gender::Masc, Gender::Fem];
    let cases = [Case::Nom, Case::Gen, Case::Dat, Case::Acc, Case::Abl];
    let mut cells: Vec<Option<Form>> = Vec::new();
    for p in persons {
        for nu in numbers {
            let owners: Vec<Option<Gender>> = match (p, nu) {
                (Person::P3, Number::Sing) => vec![Some(Gender::Masc), Some(Gender::Fem)],
                _ => vec![None],
            };
            let referents: Vec<Option<Gender>> = match p {
                Person::P3 => vec![Some(Gender::Masc), Some(Gender::Fem)],
                _ => vec![None],
            };
            for c in cases {
                cells.push(clitic(p, nu, c));
                cells.extend(referents.iter().map(|g| personal(p, nu, *g, c)));
                for g in genders {
                    cells.push(possessive_kinship(p, nu, kin_of(g), c));
                    for pn in numbers {
                        for og in &owners {
                            cells.push(possessive(p, nu, *og, g, pn, c, Definite::Def));
                            cells.push(possessive_substantive(p, nu, *og, g, pn, c));
                        }
                    }
                }
            }
            cells.extend(numbers.iter().map(|an| cluster(Some((p, nu)), Some((Person::P3, *an)))));
        }
    }
    for d in [Deixis::Near, Deixis::Far] {
        for nu in numbers {
            for g in genders {
                cells.extend(cases.iter().map(|c| demonstrative(d, Some(g), nu, *c)));
                cells.extend(cases.iter().map(|c| which(Some(g), nu, *c)));
            }
        }
    }
    cells.extend(cases.iter().map(|c| kush(*c)));
    cells.iter().filter(|c| c.is_some()).count()
}
