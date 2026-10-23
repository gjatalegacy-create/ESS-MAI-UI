//! Sintaksa e shqipes standarde: përputhja dhe rregullsia e fjalisë.
//!
//! Hyrja është fjalia e shënuar sipas UD (trajta, lema, pjesa e ligjëratës,
//! tiparet, koka, lidhja). Çdo gjetje mban emrin e rregullit si dëshmi:
//! `Agree` kur tiparet përputhen, `Disagree` kur rregulli prishet, `Hold` kur
//! dëshmia nuk mjafton për vendim. Rendi i dëshmisë:
//! 1. trajta e fjalës drejtuese: kur tiparet e emrit kundërshtojnë mbaresën e vet,
//!    qeverisjen e rasës ose gjininë e leksikut (uniparser, kur jepet), shënimi nuk vlen
//!    si dëshmi;
//! 2. tiparet e shënuara të fjalëve të lidhura;
//! 3. trajta e fjalëve të mbyllura (nyjat, përemrat, trajtat e shkurtra, pjesëzat);
//! 4. rendi i fjalëve (kontakti i nyjës me emrin, vendi i trajtës së shkurtër).
//!
//! Nyja e përparme ndjek paradigmën `article` të uniparser-grammar-albanian
//! 2.3.0, MIT: «i, e, të, së» sipas gjinisë, numrit, rasës dhe kontaktit me
//! emrin e shquar.

use crate::gramatika::leksiku::Lexicon;
use crate::gramatika::tipet::{ud_case, ud_feat, ud_gender, ud_number, Case, Definite, Gender, Mood, Number, Person, Pos};

pub const SUBJECT_VERB: &str = "subject-verb";
pub const NOUN_ADJECTIVE: &str = "noun-adjective";
pub const ADJECTIVE_ARTICLE: &str = "adjective-article";
pub const GENITIVE_ARTICLE: &str = "genitive-article";
pub const DETERMINER_NOUN: &str = "determiner-noun";
pub const CLITIC_DOUBLING: &str = "clitic-doubling";
pub const NEGATION: &str = "negation";
pub const CLITIC_PLACEMENT: &str = "clitic-placement";

/// Rregullat, në rendin e kontrollit.
pub const RULES: [&str; 8] = [
    SUBJECT_VERB,
    NOUN_ADJECTIVE,
    ADJECTIVE_ARTICLE,
    GENITIVE_ARTICLE,
    DETERMINER_NOUN,
    CLITIC_DOUBLING,
    NEGATION,
    CLITIC_PLACEMENT,
];

/// Një fjalë e fjalisë, me shënimin UD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub id: usize,
    pub form: String,
    pub lemma: String,
    pub upos: String,
    /// Tiparet UD «A=b|C=d»; bosh kur s'ka.
    pub feats: String,
    pub head: usize,
    pub deprel: String,
}

pub type Sentence = Vec<Token>;

impl Token {
    pub fn new(id: usize, form: &str, lemma: &str, upos: &str, feats: &str, head: usize, deprel: &str) -> Token {
        Token {
            id,
            form: form.to_string(),
            lemma: lemma.to_string(),
            upos: upos.to_string(),
            feats: feats.to_string(),
            head,
            deprel: deprel.to_string(),
        }
    }

    pub fn feat(&self, key: &str) -> Option<&str> {
        ud_feat(&self.feats, key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Agree,
    Disagree,
    Hold,
}

/// Një vendim i një rregulli mbi fjalët `tokens` (id-të e fjalisë).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub tokens: Vec<usize>,
    pub verdict: Verdict,
    pub note: String,
}

fn finding(rule: &'static str, tokens: Vec<usize>, verdict: Verdict, note: impl Into<String>) -> Finding {
    Finding { rule, tokens, verdict, note: note.into() }
}

/// Hapat e dëshmisë në rend: vendos i pari që vlen.
fn decide(rule: &'static str, tokens: Vec<usize>, steps: Vec<(bool, Verdict, String)>) -> Finding {
    steps
        .into_iter()
        .find(|(applies, _, _)| *applies)
        .map(|(_, verdict, note)| finding(rule, tokens.clone(), verdict, note))
        .unwrap_or_else(|| finding(rule, tokens, Verdict::Hold, "asnjë dëshmi"))
}

// ---------------------------------------------------------------- CoNLL-U

/// Lexuesi i formatit CoNLL-U: fjalitë me `sent_id`. Fjalët shumëpjesëshe
/// («ia» = «i» + «e») merren sipas pjesëve; nyjet boshe kapërcehen.
pub fn parse_conllu_ids(text: &str) -> Vec<(String, Sentence)> {
    let mut out = Vec::new();
    let mut id = String::new();
    let mut cur: Sentence = Vec::new();
    for line in text.lines().map(|l| l.trim_end_matches('\r')) {
        match (line.trim().is_empty(), line.strip_prefix("# sent_id =")) {
            (true, _) => flush(&mut out, &mut id, &mut cur),
            (false, Some(s)) => id = s.trim().to_string(),
            (false, None) => cur.extend(token_line(line)),
        }
    }
    flush(&mut out, &mut id, &mut cur);
    out
}

/// Fjalitë e një teksti CoNLL-U.
pub fn parse_conllu(text: &str) -> Vec<Sentence> {
    parse_conllu_ids(text).into_iter().map(|(_, s)| s).collect()
}

fn flush(out: &mut Vec<(String, Sentence)>, id: &mut String, cur: &mut Sentence) {
    match cur.is_empty() {
        true => (),
        false => out.push((std::mem::take(id), std::mem::take(cur))),
    }
}

/// Rreshti i një fjale; kur kolona e tipareve është bosh dhe tiparet gjenden në
/// kolonën XPOS, merren prej andej.
fn token_line(line: &str) -> Option<Token> {
    let cols: Vec<&str> = line.split('\t').collect();
    match cols.as_slice() {
        [id, form, lemma, upos, xpos, feats, head, deprel, _, _] => {
            let id = id.parse::<usize>().ok()?;
            let feats = match (*feats, xpos.contains('=')) {
                ("_", true) => xpos,
                ("_", false) => "",
                (f, _) => f,
            };
            Some(Token::new(id, form, lemma, upos, feats, head.parse::<usize>().unwrap_or(0), deprel))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------- tiparet

fn low(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            '’' | '`' | '´' | 'ʼ' => '\'',
            c => c,
        })
        .collect()
}

fn number(t: &Token) -> Option<Number> {
    t.feat("Number").and_then(ud_number)
}

fn gender(t: &Token) -> Option<Gender> {
    t.feat("Gender").and_then(ud_gender)
}

fn case(t: &Token) -> Option<Case> {
    t.feat("Case").and_then(ud_case)
}

fn definite(t: &Token) -> Option<Definite> {
    match t.feat("Definite") {
        Some("Def") => Some(Definite::Def),
        Some("Ind") => Some(Definite::Ind),
        _ => None,
    }
}

fn person(t: &Token) -> Option<Person> {
    match t.feat("Person") {
        Some("1") => Some(Person::P1),
        Some("2") => Some(Person::P2),
        Some("3") => Some(Person::P3),
        _ => None,
    }
}

fn mood(t: &Token) -> Option<Mood> {
    match t.feat("Mood") {
        Some("Ind") => Some(Mood::Ind),
        Some("Sub") => Some(Mood::Sub),
        Some("Cnd") => Some(Mood::Cnd),
        Some("Adm") => Some(Mood::Adm),
        Some("Opt") => Some(Mood::Opt),
        Some("Imp") => Some(Mood::Imp),
        _ => None,
    }
}

fn is_nominal(t: &Token) -> bool {
    matches!(t.upos.as_str(), "NOUN" | "PROPN" | "PRON")
}

fn is_noun(t: &Token) -> bool {
    matches!(t.upos.as_str(), "NOUN" | "PROPN")
}

fn oblique(c: Case) -> bool {
    matches!(c, Case::Gen | Case::Dat | Case::Abl)
}

fn num_sq(n: Number) -> &'static str {
    match n {
        Number::Sing => "njëjës",
        Number::Plur => "shumës",
    }
}

fn gen_sq(g: Gender) -> &'static str {
    match g {
        Gender::Masc => "mashkullore",
        Gender::Fem => "femërore",
        Gender::Neut => "asnjanëse",
    }
}

fn per_sq(p: Person) -> &'static str {
    match p {
        Person::P1 => "veta e parë",
        Person::P2 => "veta e dytë",
        Person::P3 => "veta e tretë",
    }
}

// ---------------------------------------------------------------- dëshmia e trajtës

const NOM: &[Case] = &[Case::Nom];
const ACC: &[Case] = &[Case::Acc];
const NOM_ACC: &[Case] = &[Case::Nom, Case::Acc];
const OBL: &[Case] = &[Case::Gen, Case::Dat, Case::Abl];
const NON_NOM: &[Case] = &[Case::Acc, Case::Gen, Case::Dat, Case::Abl];
const ANY: &[Case] = &[];

const M: Option<Gender> = Some(Gender::Masc);
const F: Option<Gender> = Some(Gender::Fem);
const SG: Number = Number::Sing;
const PL: Number = Number::Plur;

type Cell = (Number, Option<Gender>, &'static [Case]);

/// Mbaresat e lakimit të shquar dhe qelizat që lejojnë (mbaresa më e gjatë fiton):
/// -ve/-vet zhdrejtë shumës; -it/-ut zhdrejtë njëjës mashkullore (edhe -it emërore/kallëzore
/// shumës: «njerëzit»); -s/-së zhdrejtë njëjës femërore; -t/-të emërore/kallëzore shumës;
/// -n/-në kallëzore njëjës; -a emërore njëjës femërore; -i/-u emërore njëjës mashkullore.
const DEFINITE_ENDINGS: &[(&str, &[Cell])] = &[
    ("vet", &[(PL, None, OBL)]),
    ("ve", &[(PL, None, OBL)]),
    ("it", &[(SG, M, OBL), (PL, None, NOM_ACC)]),
    ("ut", &[(SG, M, OBL)]),
    ("së", &[(SG, F, OBL)]),
    ("të", &[(PL, None, NOM_ACC)]),
    ("në", &[(SG, None, ACC)]),
    ("s", &[(SG, F, OBL)]),
    ("t", &[(PL, None, NOM_ACC)]),
    ("n", &[(SG, None, ACC)]),
    ("a", &[(SG, F, NOM)]),
    ("i", &[(SG, M, NOM)]),
    ("u", &[(SG, M, NOM)]),
];

/// Mbaresat e nyjës së prapme që i shtohen lemës (ose lemës pa -ë fundore).
const DEFINITE_SUFFIXES: [&str; 8] = ["n", "në", "s", "së", "t", "të", "it", "ut"];

/// Emri, shënimi i të cilit kundërshton mbaresën e vet, nuk vlen si dëshmi:
/// (a) emri i shquar (ose pa shënim shquarsie) me mbaresë të shquar të një qelize tjetër;
/// (b) femërori me lemë në -ë dhe trajtë në -e («radhë» → «radhe») është rasë e zhdrejtë
/// e pashquar, jo emërore a kallëzore;
/// (c) emri i shënuar i pashquar me trajtë = lema (pa -ë) + nyjë e prapme («joshjen», «borës»);
/// (d) emri i përbashkët i shënuar i shquar pa asnjë mbaresë të shquar («ide», «tryezë»).
fn contradicted(t: &Token) -> bool {
    let f = low(&t.form);
    let lemma = low(&t.lemma);
    let cells = DEFINITE_ENDINGS.iter().find(|(s, _)| f.ends_with(s)).map(|(_, c)| *c);
    let definite_clash = match (definite(t), number(t), case(t), cells) {
        (Some(Definite::Def) | None, Some(n), Some(c), Some(cells)) => !cells.iter().any(|(cn, cg, cc)| {
            *cn == n && cc.contains(&c) && cg.zip(gender(t)).map_or(true, |(want, got)| want == got)
        }),
        _ => false,
    };
    let oblique_form = lemma.strip_suffix('ë').map_or(false, |stem| f == format!("{stem}e"));
    let oblique_clash = oblique_form
        && definite(t) != Some(Definite::Def)
        && gender(t) != Some(Gender::Masc)
        && case(t).map_or(false, |c| !oblique(c));
    let definite_shape = [Some(lemma.as_str()), lemma.strip_suffix('ë')]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .any(|s| DEFINITE_SUFFIXES.iter().any(|e| f == format!("{s}{e}")));
    let indefinite_clash = definite(t) == Some(Definite::Ind) && definite_shape;
    let bare_clash = t.upos == "NOUN" && definite(t) == Some(Definite::Def) && cells.is_none();
    is_noun(t) && (definite_clash || oblique_clash || indefinite_clash || bare_clash)
}

/// Emrat e pashquar që e kanë shumësin të njëjtë me njëjësin: femërorët me -e, -i, -o
/// («lule», «shtëpi», «radio») dhe emrat e vepruesit me -ës, -ues («nxënës», «mësues»).
fn number_hidden(t: &Token) -> bool {
    let f = low(&t.form);
    let same = f == low(&t.lemma);
    let class = ["e", "i", "o", "ës", "ues", "yes"].iter().any(|s| f.ends_with(s)) && !f.ends_with('ë');
    definite(t) != Some(Definite::Def) && same && class
}

/// Numri që del në mbaresën e mbiemrit: -a femërore shumës («të mira», «të caktuara»),
/// -enj/-inj/-ij mashkullore shumës («të mëdhenj», «të rinj»); -or/-al/-ar/-ik/-iv/-ist
/// mashkullore njëjës, me -ë në shumës. Mbaresat e tjera (-e, -ë, -ur, -shëm) nuk e dallojnë.
fn adjective_number(form: &str) -> Option<Number> {
    const PLURAL: [&str; 10] = ["a", "enj", "inj", "ij", "orë", "alë", "arë", "ikë", "ivë", "istë"];
    const SINGULAR: [&str; 6] = ["or", "al", "ar", "ik", "iv", "ist"];
    match (PLURAL.iter().any(|s| form.ends_with(s)), SINGULAR.iter().any(|s| form.ends_with(s))) {
        (true, _) => Some(Number::Plur),
        (false, true) => Some(Number::Sing),
        (false, false) => None,
    }
}

/// Rasa që qeverisin parafjalët: emërore («nga», «te», «tek»), kallëzore («në», «me», «për»,
/// «mbi», «nën», «ndër», «pa», «nëpër»), rrjedhore (parafjalët e tjera të listës).
const PREPOSITIONS: &[(&str, Case)] = &[
    ("nga", Case::Nom),
    ("te", Case::Nom),
    ("tek", Case::Nom),
    ("në", Case::Acc),
    ("me", Case::Acc),
    ("për", Case::Acc),
    ("mbi", Case::Acc),
    ("nën", Case::Acc),
    ("ndër", Case::Acc),
    ("pa", Case::Acc),
    ("nëpër", Case::Acc),
    ("përmbi", Case::Acc),
    ("prej", Case::Abl),
    ("pas", Case::Abl),
    ("para", Case::Abl),
    ("përpara", Case::Abl),
    ("gjatë", Case::Abl),
    ("drejt", Case::Abl),
    ("afër", Case::Abl),
    ("pranë", Case::Abl),
    ("larg", Case::Abl),
    ("rreth", Case::Abl),
    ("sipas", Case::Abl),
    ("brenda", Case::Abl),
    ("jashtë", Case::Abl),
    ("përveç", Case::Abl),
    ("përmes", Case::Abl),
    ("nëpërmjet", Case::Abl),
    ("midis", Case::Abl),
    ("mes", Case::Abl),
    ("kundër", Case::Abl),
    ("përballë", Case::Abl),
    ("mbas", Case::Abl),
    ("krahas", Case::Abl),
];

/// Grupi i rasës për nyjat: emërore, kallëzore, e zhdrejtë (gjinore, dhanore, rrjedhore).
fn case_group(c: Case) -> u8 {
    match c {
        Case::Nom => 0,
        Case::Acc => 1,
        Case::Gen | Case::Dat | Case::Abl => 2,
        Case::Voc => 3,
    }
}

// ---------------------------------------------------------------- pamja e pemës

struct View<'a> {
    toks: &'a [Token],
    lex: Option<&'a Lexicon>,
    time_nouns: &'a [&'a str],
}

impl<'a> View<'a> {
    /// Leksiku e njeh lemën vetëm si mbiemër (emër i mbiemërzuar me nyjën e vet).
    fn lexical_adjective(&self, t: &Token) -> bool {
        self.lex.map_or(false, |lx| {
            let entries = lx.entries(&t.lemma);
            !entries.is_empty() && entries.iter().all(|l| l.pos == Pos::Adj)
        })
    }

    /// Shënimi i emrit nuk vlen si dëshmi: kundërshton mbaresën e vet ose gjininë që i jep
    /// leksiku për atë numër.
    fn unreliable(&self, t: &Token) -> bool {
        contradicted(t) || self.lexical_gender_clash(t)
    }

    fn lexical_gender_clash(&self, t: &Token) -> bool {
        let pos = match t.upos.as_str() {
            "NOUN" => Some(Pos::Noun),
            _ => None,
        };
        let lexical: Vec<Gender> = self
            .lex
            .zip(pos)
            .zip(number(t))
            .map(|((lx, pos), n)| lx.entries(&t.lemma).iter().filter(|l| l.pos == pos).filter_map(|l| l.gender(n)).collect())
            .unwrap_or_default();
        let single = lexical.first().filter(|g| lexical.iter().all(|x| x == *g));
        single.zip(gender(t)).map_or(false, |(want, got)| *want != got)
    }

    fn get(&self, id: usize) -> Option<&'a Token> {
        self.toks.iter().find(|t| t.id == id)
    }

    fn kids(&self, id: usize) -> impl Iterator<Item = &'a Token> + 'a {
        self.toks.iter().filter(move |t| t.head == id && t.id != id)
    }

    fn between(&self, a: usize, b: usize) -> impl Iterator<Item = &'a Token> + 'a {
        let (lo, hi) = (a.min(b), a.max(b));
        self.toks.iter().filter(move |t| t.id > lo && t.id < hi)
    }

    fn prev(&self, t: &Token) -> Option<&'a Token> {
        t.id.checked_sub(1).and_then(|i| self.get(i))
    }

    fn question(&self) -> bool {
        self.toks.iter().any(|t| t.upos == "PUNCT" && t.form.contains('?'))
    }
}

/// Rasa e shënuar e emrit kundërshton atë që kërkon parafjala e tij (ose kundrina e drejtë,
/// që është në kallëzore): shënimi i rasës nuk vlen si dëshmi.
fn governed_clash(v: &View, t: &Token) -> bool {
    let prep = v.kids(t.id).filter(|k| k.deprel == "case" && k.id < t.id).max_by_key(|k| k.id);
    let by_prep = prep.and_then(|p| PREPOSITIONS.iter().find(|(f, _)| *f == low(&p.form)).map(|(_, c)| *c));
    let by_role = (t.deprel == "obj").then_some(Case::Acc);
    by_prep.or(by_role).zip(case(t)).map_or(false, |(want, got)| case_group(want) != case_group(got))
}

/// Emri i bashkërenditur me emra të tjerë («një burrë dhe një grua»).
fn coordinated(v: &View, t: &Token) -> bool {
    v.kids(t.id).any(|k| k.deprel == "conj" && is_nominal(k))
}

const DISJUNCTIVE: [&str; 5] = ["ose", "apo", "as", "o", "qoftë"];

/// Bashkërenditja veçuese ose mohuese («ose», «apo», «as ... as»): lidhëza varet nga
/// gjymtyra e dytë ose, te «as ... as», edhe nga e para.
fn disjunct(v: &View, t: &Token, conj: &[&Token]) -> bool {
    let cc = |id: usize| v.kids(id).any(|k| k.deprel == "cc" && DISJUNCTIVE.contains(&low(&k.form).as_str()));
    !conj.is_empty() && (cc(t.id) || conj.iter().any(|c| cc(c.id)))
}

/// Numrat që lejon kundrina për trajtën e shkurtër: e bashkërenditur me «dhe» → shumës;
/// e bashkërenditur veçuese → njëjës ose shumës; ndryshe numri i saj.
fn object_numbers(v: &View, o: &Token) -> Vec<Number> {
    let conj: Vec<&Token> = v.kids(o.id).filter(|k| k.deprel == "conj" && is_nominal(k)).collect();
    match (conj.is_empty(), disjunct(v, o, &conj)) {
        (true, _) => number(o).into_iter().collect(),
        (false, true) => vec![Number::Sing, Number::Plur],
        (false, false) => vec![Number::Plur],
    }
}

/// Shoqëruesja «(bashkë) me» e kryefjalës.
fn comitative(v: &View, k: &Token) -> bool {
    v.kids(k.id).any(|c| c.deprel == "case" && low(&c.form) == "me")
}

/// Kryefjalë njëjës me gjinore shumës ose me shoqëruese («shumica e anëtarëve»,
/// «Beni me Anën»): lejon përputhjen sipas kuptimit.
fn quantified(v: &View, s: &Token) -> bool {
    v.kids(s.id).any(|k| (k.deprel.starts_with("nmod") && number(k) == Some(Number::Plur)) || comitative(v, k))
}

fn is_verbal_group(t: &Token) -> bool {
    matches!(t.deprel.as_str(), "aux" | "aux:pass" | "cop")
}

/// Trajta e shtjelluar që mban vetën dhe numrin: folja vetë ose ndihmësja/foljelidhorja.
fn finite_of<'a>(v: &View<'a>, h: &'a Token) -> Option<&'a Token> {
    std::iter::once(h).chain(v.kids(h.id).filter(|c| is_verbal_group(c))).find(|t| is_finite(t))
}

fn is_finite(t: &Token) -> bool {
    matches!(t.upos.as_str(), "VERB" | "AUX")
        && t.feat("VerbForm").map_or(true, |f| f == "Fin")
        && number(t).is_some()
        && person(t).is_some()
        && !matches!(low(&t.form).as_str(), "u" | "do" | "s'")
}

// ---------------------------------------------------------------- kryefjalë-folje

/// Përemrat vetorë dhe dëftorë si kryefjalë: veta dhe numri dalin nga trajta.
const PRONOUN_SUBJECTS: [(&str, Person, Number); 12] = [
    ("unë", Person::P1, Number::Sing),
    ("ti", Person::P2, Number::Sing),
    ("ai", Person::P3, Number::Sing),
    ("ajo", Person::P3, Number::Sing),
    ("ne", Person::P1, Number::Plur),
    ("ata", Person::P3, Number::Plur),
    ("ato", Person::P3, Number::Plur),
    ("ky", Person::P3, Number::Sing),
    ("kjo", Person::P3, Number::Sing),
    ("këta", Person::P3, Number::Plur),
    ("këto", Person::P3, Number::Plur),
    ("vetë", Person::P3, Number::Sing),
];

struct SubjectAgr {
    person: Person,
    number: Number,
    coordinated: bool,
    polite: bool,
}

/// Veta dhe numri i kryefjalës: kryefjala e bashkërenditur me «dhe» është shumës
/// dhe merr vetën më të ulët (1 < 2 < 3); «ju» e mirësjelljes është shumës. Mbiemrat e
/// emërzuar bashkërenditen si emrat («e mira dhe e keqja»).
fn subject_agr(v: &View, s: &Token) -> Result<SubjectAgr, &'static str> {
    let substantive = s.upos == "ADJ";
    let conj: Vec<&Token> = v.kids(s.id).filter(|c| c.deprel == "conj" && (is_nominal(c) || (substantive && c.upos == "ADJ"))).collect();
    let disjunct = disjunct(v, s, &conj);
    let closed = |t: &Token| match t.upos.as_str() {
        "PRON" | "DET" => PRONOUN_SUBJECTS.iter().find(|(f, _, _)| *f == low(&t.form)).map(|(_, p, n)| (*p, *n)),
        _ => None,
    };
    let own_person = |t: &Token| closed(t).map(|(p, _)| p).or(person(t)).unwrap_or(Person::P3);
    match (conj.is_empty(), disjunct) {
        (false, true) => Err("kryefjalë e bashkërenditur veçuese: përputhja ndjek gjymtyrën më të afërt"),
        (false, false) => Ok(SubjectAgr {
            person: conj.iter().map(|c| own_person(c)).fold(own_person(s), |a, b| a.min(b)),
            number: Number::Plur,
            coordinated: true,
            polite: false,
        }),
        (true, _) => match (low(&s.form).as_str(), closed(s).map(|(_, n)| n).or(number(s))) {
            ("ju", _) => Ok(SubjectAgr { person: Person::P2, number: Number::Plur, coordinated: false, polite: true }),
            (_, Some(n)) => Ok(SubjectAgr { person: own_person(s), number: n, coordinated: false, polite: false }),
            (_, None) => Err("kryefjala nuk e shfaq numrin"),
        },
    }
}

fn subject_verb(v: &View) -> Vec<Finding> {
    v.toks
        .iter()
        .filter(|s| s.deprel == "nsubj" || s.deprel == "nsubj:pass")
        .filter_map(|s| {
            let h = v.get(s.head)?;
            let fin = finite_of(v, h)?;
            let modal = matches!(low(&fin.form).as_str(), "mund" | "duhet" | "duhej");
            let complement = v
                .kids(fin.id)
                .chain(v.kids(h.id))
                .find(|k| matches!(k.deprel.as_str(), "xcomp" | "ccomp") && is_finite(k));
            Some(match (modal, complement) {
                (false, _) => judge_subject(v, s, h, fin),
                (true, Some(c)) => judge_subject(v, s, c, c),
                (true, None) => finding(SUBJECT_VERB, vec![s.id, fin.id], Verdict::Hold, "«mund/duhet» modale nuk lakohet sipas kryefjalës"),
            })
        })
        .collect()
}

/// Trajtat e vetës së tretë njëjës të «kam» ekzistenciale («ka», «kishte», «pati»).
const EXISTENTIAL_KA: [&str; 8] = ["ka", "kishte", "kish", "pati", "pat", "paska", "paskësh", "ketë"];

/// Foljet pavetore që marrin fjali plotësuese («Duket (se) ...», «Ndodh që ...»).
const IMPERSONAL: [&str; 9] = ["duket", "dukej", "duk", "ndodh", "ndodhi", "ndodhte", "mjafton", "mjaftonte", "qëllon"];

/// Folja përputhet me kryefjalën në vetë dhe numër. Nuk vendoset kur kryefjala është
/// fjali, në rasë jo emërore (përjetuesi i «më dhemb») ose me shënim që kundërshton
/// mbaresën. Nuk janë gabim: «ka» ekzistenciale me emër të pashquar shumës pas saj («Ka
/// njerëz»), folja pavetore me fjali plotësuese («Duket njerëzit kishin ...»), kryefjala
/// sasiore me gjinore shumës ose me shoqëruese «(bashkë) me», folja njëjës me kryefjalë të
/// bashkërenditur (përputhje me gjymtyrën më të afërt), foljelidhorja që përputhet me
/// kallëzuesoren, «kryefjala» para foljes që e dyfishon trajta e shkurtër kallëzore kur
/// folja ka një emër tjetër në emërore që përputhet me të (kundrinë e paravendosur: «sa
/// gishtërinjtë i ka nipi»).
/// «Mund/duhet» modale nuk lakohen: përputhet folja e lidhores pas tyre.
fn judge_subject(v: &View, s: &Token, h: &Token, fin: &Token) -> Finding {
    let ids = vec![s.id, fin.id];
    match subject_agr(v, s) {
        Err(why) => finding(SUBJECT_VERB, ids, Verdict::Hold, why),
        Ok(sa) => {
            let vn = number(fin);
            let vp = person(fin);
            let third_singular = vn == Some(Number::Sing) && vp.map_or(true, |p| p == Person::P3);
            let fits = vn == Some(sa.number) && vp.map_or(true, |p| p == sa.person);
            let oblique_subject = case(s).map_or(false, |c| c != Case::Nom);
            let clausal = matches!(s.upos.as_str(), "VERB" | "AUX");
            let existential = h.lemma == "kam"
                && EXISTENTIAL_KA.contains(&low(&fin.form).as_str())
                && s.id > fin.id
                && definite(s) != Some(Definite::Def)
                && third_singular
                && sa.number == Number::Plur;
            let impersonal = third_singular
                && IMPERSONAL.contains(&low(&fin.form).as_str())
                && v.kids(h.id).any(|k| matches!(k.deprel.as_str(), "ccomp" | "csubj" | "xcomp") && matches!(k.upos.as_str(), "VERB" | "AUX"));
            let notional = sa.number == Number::Sing && vn == Some(Number::Plur) && quantified(v, s);
            let doubled = v.kids(h.id).chain(v.kids(fin.id)).any(|k| is_clitic(k) && case(k) == Some(Case::Acc) && number(k) == Some(sa.number));
            let rival = v
                .kids(h.id)
                .any(|k| k.id != s.id && is_nominal(k) && case(k) == Some(Case::Nom) && vn.map_or(false, |n| number(k) == Some(n)));
            let object_like = sa.person == Person::P3 && s.id < fin.id && doubled && rival;
            let nearest = sa.coordinated && vn == Some(Number::Sing);
            let predicate = h.id != fin.id && is_nominal(h) && number(h) == vn;
            let tag = match (sa.coordinated, sa.polite) {
                (true, _) => " (e bashkërenditur, shumës)",
                (_, true) => " (shumësi i mirësjelljes)",
                _ => "",
            };
            decide(
                SUBJECT_VERB,
                ids,
                vec![
                    (clausal, Verdict::Hold, "kryefjalë fjali".to_string()),
                    (v.unreliable(s), Verdict::Hold, format!("shënimi i «{}» kundërshton mbaresën e vet", s.form)),
                    (oblique_subject, Verdict::Hold, "kryefjalë në rasë jo emërore: përjetues".to_string()),
                    (fits, Verdict::Agree, format!("«{}» përputhet me kryefjalën «{}»{}", fin.form, s.form, tag)),
                    (existential, Verdict::Hold, "«ka» ekzistenciale mbetet në vetën e tretë njëjës".to_string()),
                    (impersonal, Verdict::Hold, format!("«{}» pavetore me fjali plotësuese", fin.form)),
                    (
                        object_like,
                        Verdict::Hold,
                        format!("«{}» dyfishohet nga trajta e shkurtër kallëzore: kundrinë e paravendosur", s.form),
                    ),
                    (notional, Verdict::Hold, "kryefjalë sasiore me gjinore shumës: përputhje sipas kuptimit".to_string()),
                    (nearest, Verdict::Hold, "kryefjalë e bashkërenditur me folje njëjës: përputhje me gjymtyrën më të afërt".to_string()),
                    (predicate, Verdict::Hold, "foljelidhorja përputhet me kallëzuesoren".to_string()),
                    (
                        true,
                        Verdict::Disagree,
                        format!(
                            "«{}» është {} {}, kryefjala «{}» kërkon {} {}",
                            fin.form,
                            vp.map_or("", per_sq),
                            vn.map_or("", num_sq),
                            s.form,
                            per_sq(sa.person),
                            num_sq(sa.number)
                        ),
                    ),
                ],
            )
        }
    }
}

// ---------------------------------------------------------------- emër-mbiemër

fn adjective_head<'a>(v: &View<'a>, a: &'a Token) -> Option<&'a Token> {
    let h = v.get(a.head)?;
    match a.deprel.as_str() {
        "amod" => Some(h).filter(|h| is_noun(h)),
        "conj" => (h.upos == "ADJ" && h.deprel == "amod").then(|| v.get(h.head)).flatten().filter(|n| is_noun(n)),
        _ => None,
    }
}

fn noun_adjective(v: &View) -> Vec<Finding> {
    v.toks.iter().filter(|a| a.upos == "ADJ").filter_map(|a| adjective_head(v, a).map(|h| judge_adjective(v, a, h))).collect()
}

/// Mbiemri përputhet me emrin në gjini dhe numër. Mospërputhja në numër vlen vetëm kur
/// mbaresa e mbiemrit e dallon numrin; mospërputhja në gjini vlen gjithmonë, edhe kur
/// numri nuk duket. Pas emrave të bashkërenditur mbiemri del në shumës; gjinia vlerësohet
/// kur gjymtyrët kanë një gjini.
fn judge_adjective(v: &View, a: &Token, h: &Token) -> Finding {
    let ids = vec![h.id, a.id];
    let conj: Vec<&Token> = v.kids(h.id).filter(|k| k.deprel == "conj" && is_nominal(k)).collect();
    let joint = !conj.is_empty() && number(a) == Some(Number::Plur);
    let head_number = match joint {
        true => Some(Number::Plur),
        false => number(h),
    };
    let head_gender = gender(h).filter(|g| !joint || conj.iter().all(|c| gender(c) == Some(*g)));
    let n = number(a).zip(head_number);
    let g = gender(a).zip(head_gender);
    let bad_n = n.filter(|(x, y)| x != y);
    let bad_g = g.filter(|(x, y)| x != y);
    let visible = adjective_number(&low(&a.form));
    decide(
        NOUN_ADJECTIVE,
        ids,
        vec![
            (n.is_none() && g.is_none(), Verdict::Hold, "mungojnë gjinia dhe numri".to_string()),
            (v.unreliable(h), Verdict::Hold, format!("shënimi i «{}» kundërshton mbaresën e vet", h.form)),
            (
                bad_n.is_some() && visible.zip(number(a)).map_or(false, |(x, y)| x != y),
                Verdict::Hold,
                format!("shënimi i «{}» kundërshton mbaresën e vet", a.form),
            ),
            (
                bad_g.is_some(),
                Verdict::Disagree,
                bad_g.map(|(x, y)| format!("«{}» është {}, «{}» është {}", a.form, gen_sq(x), h.form, gen_sq(y))).unwrap_or_default(),
            ),
            (bad_n.is_some() && visible.is_none(), Verdict::Hold, format!("numri i «{}» nuk duket në mbaresë", a.form)),
            (
                bad_n.is_some(),
                Verdict::Disagree,
                bad_n.map(|(x, y)| format!("«{}» është {}, «{}» është {}", a.form, num_sq(x), h.form, num_sq(y))).unwrap_or_default(),
            ),
            (true, Verdict::Agree, format!("«{}» përputhet me «{}»", a.form, h.form)),
        ],
    )
}

// ---------------------------------------------------------------- nyja e përparme

/// Nyja e përparme sipas paradigmës `article` (uniparser 2.3.0, MIT).
/// `contact`: nyja vjen menjëherë pas emrit të shquar; ndryshe nyja është «e largët».
pub fn article(gender: Gender, number: Number, case: Case, contact: bool) -> Option<&'static str> {
    match (number, case, gender, contact) {
        (Number::Sing, Case::Nom, Gender::Masc, _) => Some("i"),
        (Number::Sing, Case::Nom, Gender::Fem, _) => Some("e"),
        (Number::Sing, Case::Acc, Gender::Masc | Gender::Fem, true) => Some("e"),
        (Number::Sing, Case::Acc, Gender::Masc | Gender::Fem, false) => Some("të"),
        (Number::Sing, Case::Gen | Case::Dat | Case::Abl, Gender::Fem, true) => Some("së"),
        (Number::Sing, Case::Gen | Case::Dat | Case::Abl, Gender::Masc | Gender::Fem, _) => Some("të"),
        (Number::Plur, Case::Nom | Case::Acc, _, true) => Some("e"),
        (Number::Plur, Case::Nom | Case::Acc, _, false) => Some("të"),
        (Number::Plur, Case::Gen | Case::Dat | Case::Abl, _, _) => Some("të"),
        _ => None,
    }
}

const ARTICLES: [&str; 4] = ["i", "e", "të", "së"];

/// Nyja e përparme: «i, e, të, së», e shënuar si përcaktor ose pjesëz.
fn is_article(t: &Token) -> bool {
    matches!(t.upos.as_str(), "DET" | "PART") && ARTICLES.contains(&low(&t.form).as_str())
}

fn article_of<'a>(v: &View<'a>, w: &Token) -> Option<&'a Token> {
    v.kids(w.id).find(|k| k.id < w.id && matches!(k.deprel.as_str(), "det:adj" | "det" | "case") && is_article(k))
}

/// Emri që drejton nyjën: koka, ose gjymtyra e bashkërenditur e kokës që qëndron
/// menjëherë para nyjës («frutat dhe perimet e rritura»); dhe a ka kontakt me të.
fn article_controller<'a>(v: &View<'a>, h: &'a Token, art: &Token) -> (&'a Token, bool) {
    let prev = v.prev(art);
    let conjunct = prev.filter(|p| p.id != h.id && is_noun(p) && p.deprel == "conj" && p.head == h.id);
    let ctrl = conjunct.unwrap_or(h);
    let contact = prev.map_or(false, |p| p.id == ctrl.id) && definite(ctrl) == Some(Definite::Def);
    (ctrl, contact)
}

/// Nyja e pritur pas emrit `ctrl`; `plural`: emra të bashkërenditur me mbiemër shumës.
fn expected_after_noun(v: &View, ctrl: &Token, contact: bool, own_case: Option<Case>, plural: bool) -> Result<&'static str, String> {
    let c = case(ctrl);
    let clash = c.zip(own_case).map_or(false, |(x, y)| case_group(x) != case_group(y)) || governed_clash(v, ctrl);
    let n = match plural {
        true => Some(Number::Plur),
        false => number(ctrl),
    };
    match (v.unreliable(ctrl), clash, gender(ctrl), n, c) {
        (true, _, _, _, _) => Err(format!("shënimi i «{}» kundërshton mbaresën e vet", ctrl.form)),
        (false, true, _, _, _) => Err(format!("rasa e shënuar e «{}» nuk përputhet me qeverisjen", ctrl.form)),
        (false, false, _, Some(Number::Plur), Some(c)) => article(Gender::Masc, Number::Plur, c, contact).ok_or("rasë pa nyjë".to_string()),
        (false, false, Some(g), Some(n), Some(c)) => article(g, n, c, contact).ok_or("rasë a gjini pa nyjë".to_string()),
        _ => Err(format!("«{}» nuk e shfaq gjininë, numrin ose rasën", ctrl.form)),
    }
}

fn judge_article(rule: &'static str, ids: Vec<usize>, art: &Token, expected: Result<&'static str, String>, whom: &str) -> Finding {
    let got = low(&art.form);
    match expected {
        Err(why) => finding(rule, ids, Verdict::Hold, why),
        Ok(e) => match e == got {
            true => finding(rule, ids, Verdict::Agree, format!("nyja «{}» e drejtë për «{}»", got, whom)),
            false => finding(rule, ids, Verdict::Disagree, format!("pritej nyja «{}», jo «{}», për «{}»", e, got, whom)),
        },
    }
}

/// Nyja e mbiemrit të nyjshëm: si cilësor, sipas emrit; si kallëzuesor, sipas kryefjalës
/// (njëjës mashkullor «i», njëjës femëror «e», shumës «të»).
fn adjective_article(v: &View) -> Vec<Finding> {
    v.toks
        .iter()
        .filter(|a| a.upos == "ADJ")
        .filter_map(|a| article_of(v, a).map(|art| (a, art)))
        .map(|(a, art)| {
            let attributive = adjective_head(v, a);
            let subject = v.kids(a.id).any(|k| k.deprel == "cop").then(|| v.kids(a.id).find(|k| k.deprel == "nsubj")).flatten();
            match (attributive, subject) {
                (Some(h), _) => {
                    let (ctrl, contact) = article_controller(v, h, art);
                    let plural = coordinated(v, h) && number(a) == Some(Number::Plur);
                    let expected = expected_after_noun(v, ctrl, contact, case(a), plural);
                    judge_article(ADJECTIVE_ARTICLE, vec![h.id, art.id, a.id], art, expected, &ctrl.form)
                }
                (None, Some(s)) => judge_article(ADJECTIVE_ARTICLE, vec![s.id, art.id, a.id], art, predicative_article(v, s, a), &s.form),
                (None, None) => finding(ADJECTIVE_ARTICLE, vec![art.id, a.id], Verdict::Hold, "mbiemër pa emër drejtues të dukshëm"),
            }
        })
        .collect()
}

/// Nyja e kallëzuesores sipas kryefjalës; kryefjala sasiore njëjës me foljelidhore shumës
/// («shumica e anëtarëve janë të lidhur») përputhet sipas kuptimit dhe nuk vendoset.
fn predicative_article(v: &View, s: &Token, a: &Token) -> Result<&'static str, String> {
    let clausal = matches!(s.upos.as_str(), "VERB" | "AUX");
    let cop_plural = v.kids(a.id).find(|k| k.deprel == "cop").and_then(number) == Some(Number::Plur);
    let agr = subject_agr(v, s).and_then(|sa| match clausal {
        true => Err("kryefjalë fjali"),
        false => Ok(sa),
    });
    match (agr, v.unreliable(s)) {
        (Err(why), _) => Err(why.to_string()),
        (Ok(_), true) => Err(format!("shënimi i «{}» kundërshton mbaresën e vet", s.form)),
        (Ok(sa), false) => match (sa.polite, sa.number, cop_plural && quantified(v, s), gender(s)) {
            (true, _, _, _) => Err("«ju» e mirësjelljes lejon kallëzuesoren njëjës".to_string()),
            (false, Number::Plur, _, _) => Ok("të"),
            (false, Number::Sing, true, _) => Err("kryefjalë sasiore me foljelidhore shumës: përputhje sipas kuptimit".to_string()),
            (false, Number::Sing, false, Some(g)) => article(g, Number::Sing, Case::Nom, false).ok_or("gjini pa nyjë".to_string()),
            (false, Number::Sing, false, None) => Err("kryefjala nuk e shfaq gjininë".to_string()),
        },
    }
}

/// Nyjat e gjinores. Kur para saj qëndrojnë dy nyja të saj («një pjesë të së vërtetës»),
/// e para është lidhorja dhe e dyta nyja e vetë mbiemrit të emërzuar. Kur gjinorja është
/// emër që leksiku nuk e njeh si mbiemër, e para mund të jetë lidhëza «e» («të huaj e të
/// vendit»): jepen të dyja leximet.
fn linker_of<'a>(v: &View<'a>, g: &Token) -> Option<(&'a Token, Option<&'a Token>)> {
    let first = article_of(v, g)?;
    let own = v
        .prev(g)
        .filter(|p| p.id != first.id && p.head == g.id && is_article(p) && v.prev(p).map_or(false, |q| q.id == first.id));
    let ambiguous = is_noun(g) && !v.lexical_adjective(g);
    Some((first, own.filter(|_| ambiguous)))
}

/// Nyja e gjinores (dhe e përemrave pronorë me nyjë) përputhet me emrin që zotërohet.
/// Kur dy lexime të vargut të nyjave japin vendime të ndryshme, nuk vendoset.
fn genitive_article(v: &View) -> Vec<Finding> {
    v.toks
        .iter()
        .filter(|g| g.deprel == "nmod:poss" || g.deprel == "det:poss" || (g.deprel.starts_with("nmod") && case(g) == Some(Case::Gen)))
        .filter_map(|g| linker_of(v, g).map(|arts| (g, arts)))
        .map(|(g, (art, alt))| match v.get(g.head).filter(|h| is_noun(h)) {
            Some(h) => {
                let judge = |a: &Token| {
                    let (ctrl, contact) = article_controller(v, h, a);
                    judge_article(GENITIVE_ARTICLE, vec![h.id, a.id, g.id], a, expected_after_noun(v, ctrl, contact, None, false), &ctrl.form)
                };
                let main = judge(art);
                let other = alt.map(judge);
                match other.filter(|o| o.verdict != main.verdict) {
                    None => main,
                    Some(_) => finding(GENITIVE_ARTICLE, main.tokens, Verdict::Hold, "varg nyjash: nyja lidhore nuk dallohet nga lidhëza «e»"),
                }
            }
            None => finding(GENITIVE_ARTICLE, vec![art.id, g.id], Verdict::Hold, "gjinore pa emër zotërues"),
        })
        .collect()
}

// ---------------------------------------------------------------- përcaktor-emër

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetKind {
    Demonstrative,
    Quantifier,
    Possessive,
    Numeral,
}

/// Përcaktorët e mbyllur: gjinia, numri, rasat e lejuara, shquarsia e kërkuar e emrit.
struct Det {
    form: &'static str,
    kind: DetKind,
    gender: Option<Gender>,
    number: Number,
    cases: &'static [Case],
}

const fn det(form: &'static str, kind: DetKind, gender: Option<Gender>, number: Number, cases: &'static [Case]) -> Det {
    Det { form, kind, gender, number, cases }
}

use DetKind::{Demonstrative as DEM, Numeral as NUMR, Possessive as POSS, Quantifier as QUANT};

/// Dëftorët dhe sasiorët para emrit kërkojnë emrin të pashquar («ky libër», «një libër»);
/// pronorët pas emrit kërkojnë emrin të shquar («libri im»).
const DETS: &[Det] = &[
    det("ky", DEM, M, SG, NOM),
    det("kjo", DEM, F, SG, NOM),
    det("këtë", DEM, None, SG, ACC),
    det("këtij", DEM, M, SG, OBL),
    det("kësaj", DEM, F, SG, OBL),
    det("këta", DEM, M, PL, NOM_ACC),
    det("këto", DEM, F, PL, NOM_ACC),
    det("këtyre", DEM, None, PL, OBL),
    det("ai", DEM, M, SG, NOM),
    det("ajo", DEM, F, SG, NOM),
    det("atë", DEM, None, SG, ACC),
    det("atij", DEM, M, SG, OBL),
    det("asaj", DEM, F, SG, OBL),
    det("ata", DEM, M, PL, NOM_ACC),
    det("ato", DEM, F, PL, NOM_ACC),
    det("atyre", DEM, None, PL, OBL),
    det("një", QUANT, None, SG, ANY),
    det("çdo", QUANT, None, SG, ANY),
    det("asnjë", QUANT, None, SG, ANY),
    det("ndonjë", QUANT, None, SG, ANY),
    det("im", POSS, M, SG, NOM),
    det("ime", POSS, F, SG, NOM),
    det("tim", POSS, M, SG, NON_NOM),
    det("time", POSS, F, SG, ACC),
    det("sime", POSS, F, SG, OBL),
    det("yt", POSS, M, SG, NOM),
    det("jot", POSS, F, SG, NOM),
    det("tënd", POSS, M, SG, NON_NOM),
    det("tënde", POSS, F, SG, ACC),
    det("sate", POSS, F, SG, OBL),
    det("ynë", POSS, M, SG, NOM),
    det("jonë", POSS, F, SG, NOM),
    det("sonë", POSS, F, SG, OBL),
    det("mi", POSS, M, PL, ANY),
    det("mia", POSS, F, PL, ANY),
    det("tu", POSS, M, PL, ANY),
    det("tua", POSS, F, PL, ANY),
    det("tanë", POSS, M, PL, ANY),
    det("tona", POSS, F, PL, ANY),
];

/// Numërorët themelorë mbi një: emri në shumës; «tre» me mashkullore, «tri» me femërore.
const NUMERALS: &[(&str, Option<Gender>)] = &[
    ("dy", None),
    ("tre", M),
    ("tri", F),
    ("katër", None),
    ("pesë", None),
    ("gjashtë", None),
    ("shtatë", None),
    ("tetë", None),
    ("nëntë", None),
    ("dhjetë", None),
    ("njëzet", None),
    ("tridhjetë", None),
    ("dyzet", None),
    ("pesëdhjetë", None),
    ("njëqind", None),
    ("qind", None),
    ("mijë", None),
];

fn det_of(t: &Token) -> Option<Det> {
    let f = low(&t.form);
    DETS.iter()
        .find(|d| d.form == f)
        .map(|d| det(d.form, d.kind, d.gender, d.number, d.cases))
        .or_else(|| NUMERALS.iter().find(|(n, _)| *n == f).map(|(n, g)| det(n, NUMR, *g, PL, ANY)))
}

fn determiner_noun(v: &View) -> Vec<Finding> {
    v.toks
        .iter()
        .filter(|d| matches!(d.deprel.as_str(), "det" | "det:poss" | "nummod") && !is_article(d))
        .filter_map(|d| {
            let h = v.get(d.head).filter(|h| is_noun(h))?;
            let spec = det_of(d)?;
            Some(judge_determiner(v, d, h, &spec))
        })
        .collect()
}

/// Përcaktori përputhet me emrin në numër, gjini e rasë; dëftori e sasiori duan emrin të
/// pashquar, pronori pas emrit të shquar («libri im»). Pronori para emrit shkon me emra
/// farefisnie të pashquar («im vëlla», «ime motër», «tim eti»); kur emri del i shquar nuk
/// vendoset. «ky njeriu» (dëftor + i shquar) është ligjërim bisedor; në rasat e zhdrejta
/// të shumësit shquarsia nuk duket (-ve).
fn judge_determiner(v: &View, d: &Token, h: &Token, spec: &Det) -> Finding {
    let ids = vec![d.id, h.id];
    let preposed = spec.kind == DetKind::Possessive && d.id < h.id;
    let want_def = match (spec.kind, preposed) {
        (DetKind::Demonstrative | DetKind::Quantifier, _) => Some(Definite::Ind),
        (DetKind::Possessive, false) => Some(Definite::Def),
        (DetKind::Possessive, true) => Some(Definite::Ind),
        (DetKind::Numeral, _) => None,
    };
    let bad_n = number(h).filter(|n| *n != spec.number);
    let bad_g = spec.gender.zip(gender(h)).filter(|(want, got)| want != got);
    let bad_c = case(h).filter(|c| !spec.cases.is_empty() && !spec.cases.contains(c));
    let bad_d = want_def.zip(definite(h)).filter(|(want, got)| want != got);
    let plural_oblique = number(h) == Some(Number::Plur) && case(h).map_or(false, oblique);
    let colloquial = spec.kind == DetKind::Demonstrative && definite(h) == Some(Definite::Def);
    decide(
        DETERMINER_NOUN,
        ids,
        vec![
            (number(h).is_none(), Verdict::Hold, "emri nuk e shfaq numrin".to_string()),
            (v.unreliable(h), Verdict::Hold, format!("shënimi i «{}» kundërshton mbaresën e vet", h.form)),
            (bad_n.is_some() && number_hidden(h), Verdict::Hold, format!("trajta «{}» nuk e dallon numrin", h.form)),
            (
                bad_n.is_some(),
                Verdict::Disagree,
                format!("«{}» kërkon {}, «{}» është {}", d.form, num_sq(spec.number), h.form, bad_n.map_or("", num_sq)),
            ),
            (
                bad_g.is_some(),
                Verdict::Disagree,
                bad_g.map(|(w, g)| format!("«{}» kërkon gjininë {}, «{}» është {}", d.form, gen_sq(w), h.form, gen_sq(g))).unwrap_or_default(),
            ),
            (bad_c.is_some() && governed_clash(v, h), Verdict::Hold, format!("rasa e shënuar e «{}» nuk përputhet me qeverisjen", h.form)),
            (bad_c.is_some(), Verdict::Disagree, format!("rasa e «{}» nuk përputhet me rasën e «{}»", d.form, h.form)),
            (colloquial, Verdict::Hold, format!("«{} {}»: dëftor me emër të shquar, ligjërim bisedor", d.form, h.form)),
            (bad_d.is_some() && plural_oblique, Verdict::Hold, "në rasat e zhdrejta të shumësit shquarsia nuk duket".to_string()),
            (bad_d.is_some() && preposed, Verdict::Hold, format!("pronori «{}» para emrit të shquar «{}»", d.form, h.form)),
            (
                bad_d.is_some(),
                Verdict::Disagree,
                match want_def {
                    Some(Definite::Def) => format!("me «{}» emri «{}» duhet të jetë i shquar", d.form, h.form),
                    _ => format!("pas «{}» emri «{}» duhet të jetë i pashquar", d.form, h.form),
                },
            ),
            (true, Verdict::Agree, format!("«{}» përputhet me «{}»", d.form, h.form)),
        ],
    )
}

// ---------------------------------------------------------------- trajtat e shkurtra

const CLITICS: [&str; 16] = ["më", "të", "e", "i", "na", "ju", "u", "ia", "ua", "ma", "ta", "jua", "ja", "iu", "m'", "t'"];

/// Trajtë e shkurtër përemërore: përemër i patheksuar si kundrinë ose dyfishim, jo pas
/// parafjale («për të» është trajtë e plotë).
fn is_clitic(t: &Token) -> bool {
    t.upos == "PRON"
        && CLITICS.contains(&low(&t.form).as_str())
        && case(t) != Some(Case::Nom)
        && (matches!(t.deprel.as_str(), "obj" | "iobj") || t.deprel.starts_with("expl"))
}

/// Trajta e shkurtër dhanore sipas vetës dhe numrit të kundrinës.
fn dative_clitic(p: Person, n: Number) -> &'static str {
    match (p, n) {
        (Person::P1, Number::Sing) => "më",
        (Person::P2, Number::Sing) => "të",
        (Person::P3, Number::Sing) => "i",
        (Person::P1, Number::Plur) => "na",
        (Person::P2, Number::Plur) => "ju",
        (Person::P3, Number::Plur) => "u",
    }
}

/// Trajta e shkurtër kallëzore sipas vetës dhe numrit të kundrinës.
fn accusative_clitic(p: Person, n: Number) -> &'static str {
    match (p, n) {
        (Person::P1, Number::Sing) => "më",
        (Person::P2, Number::Sing) => "të",
        (Person::P3, Number::Sing) => "e",
        (Person::P1, Number::Plur) => "na",
        (Person::P2, Number::Plur) => "ju",
        (Person::P3, Number::Plur) => "i",
    }
}

/// Pjesa dhanore e një trajte të shkurtër, edhe kur është e bashkuar («ia» → «i»).
fn dative_part(form: &str) -> &str {
    match form {
        "ia" | "ja" => "i",
        "ua" => "u",
        "ma" | "m'" => "më",
        "ta" | "t'" => "të",
        "jua" => "ju",
        f => f,
    }
}

/// Trajta dhanore e vetës së tretë para «u» joveprore bëhet «iu» si për njëjësin, ashtu
/// edhe për shumësin («Benit iu duk», «Atyre iu duk»); edhe e ndarë «i u».
fn dative_fits(v: &View, c: &Token, want: &str) -> bool {
    let f = low(&c.form);
    let before_u = f == "i" && c.id.checked_add(1).and_then(|i| v.get(i)).map_or(false, |t| low(&t.form) == "u");
    match f == "iu" || before_u {
        true => want == "i" || want == "u",
        false => dative_part(&f) == want,
    }
}

/// Emrat e kohës që në kallëzore janë rrethanorë («gjithë natën», «ditën e djeshme»)
/// dhe nuk dyfishohen. Tabelë paraprake: asnjë fjali e artë nuk e vë në provë.
pub const TIME_NOUNS: [&str; 14] = ["ditë", "natë", "javë", "muaj", "vit", "orë", "minutë", "mëngjes", "mbrëmje", "herë", "kohë", "verë", "dimër", "pranverë"];

const FULL_DATIVE: [&str; 7] = ["mua", "ty", "atij", "asaj", "neve", "juve", "atyre"];
const FULL_ACCUSATIVE: [&str; 6] = ["mua", "ty", "ne", "ju", "ata", "ato"];

/// Trajtat e shkurtra të foljes: ato që varen nga folja a ndihmësja e saj, dhe ato të
/// grupit të pandërprerë menjëherë para grupit foljor.
fn clitics_of<'a>(v: &View<'a>, verb: &Token) -> Vec<&'a Token> {
    let aux: Vec<usize> = v.kids(verb.id).filter(|k| is_verbal_group(k)).map(|k| k.id).collect();
    let start = aux.iter().copied().chain(std::iter::once(verb.id)).min().unwrap_or(verb.id);
    let mut cluster = Vec::new();
    let mut cur = v.get(start);
    while let Some(p) = cur.and_then(|c| v.prev(c)).filter(|p| clitic_bridge(p, &aux)) {
        cluster.push(p.id);
        cur = Some(p);
    }
    v.toks
        .iter()
        .filter(|t| {
            (is_clitic(t) && (t.head == verb.id || aux.contains(&t.head))) || (cluster.contains(&t.id) && clitic_like(t) && low(&t.form) != "të")
        })
        .collect()
}

/// Urdhërorja, sipas shënimit ose sipas trajtës së shumësit me trajtën e shkurtër të
/// ngjitur: lema + «e/i/u» + «ni» («mbajeni», «hapini»).
fn imperative(t: &Token) -> bool {
    let (f, l) = (low(&t.form), low(&t.lemma));
    let fused = l.chars().count() > 1 && ["eni", "ini", "uni"].iter().any(|e| f == format!("{l}{e}"));
    mood(t) == Some(Mood::Imp) || fused
}

/// Kundrina e zhdrejtë dyfishohet gjithmonë me trajtën e shkurtër dhanore;
/// kundrina e drejtë e shquar dyfishohet kur del para foljes; përemri vetor i plotë
/// dyfishohet gjithmonë. Urdhërorja e mban trajtën e shkurtër të ngjitur («jepja»).
fn clitic_doubling(v: &View) -> Vec<Finding> {
    v.toks
        .iter()
        .filter(|o| matches!(o.deprel.as_str(), "iobj" | "obj") && !is_clitic(o))
        .filter_map(|o| {
            let verb = v.get(o.head).filter(|h| matches!(h.upos.as_str(), "VERB" | "AUX"))?;
            match o.deprel.as_str() {
                "iobj" => indirect_object(v, o, verb),
                _ => direct_object(v, o, verb),
            }
        })
        .collect()
}

fn indirect_object(v: &View, o: &Token, verb: &Token) -> Option<Finding> {
    let full_pron = o.upos == "PRON" && FULL_DATIVE.contains(&low(&o.form).as_str());
    (is_noun(o) || full_pron).then_some(())?;
    let cl = clitics_of(v, verb);
    let p = person(o).unwrap_or(Person::P3);
    let wants: Vec<&str> = object_numbers(v, o).into_iter().map(|n| dative_clitic(p, n)).collect();
    let want = wants.join("/");
    let matching = cl.iter().copied().find(|c| wants.iter().any(|w| dative_fits(v, c, w)));
    let dative = matching.or(cl.iter().copied().find(|c| case(c) == Some(Case::Dat) || c.deprel == "iobj" || low(&c.form) != "e"));
    let governed = v.kids(o.id).any(|k| k.deprel == "case");
    let dative_case = !governed && (full_pron || case(o) == Some(Case::Dat));
    let ids = std::iter::once(o.id).chain(std::iter::once(verb.id)).chain(dative.map(|c| c.id)).collect();
    Some(decide(
        CLITIC_DOUBLING,
        ids,
        vec![
            (imperative(verb), Verdict::Hold, "urdhërorja e mban trajtën e shkurtër të ngjitur".to_string()),
            (!dative_case, Verdict::Hold, "kundrina e zhdrejtë nuk është në dhanore".to_string()),
            (v.unreliable(o), Verdict::Hold, format!("shënimi i «{}» kundërshton mbaresën e vet", o.form)),
            (matching.is_some(), Verdict::Agree, format!("trajta e shkurtër dyfishon «{}»", o.form)),
            (
                dative.is_none(),
                Verdict::Disagree,
                format!("kundrina e zhdrejtë «{}» kërkon trajtën e shkurtër{}", o.form, Some(&want).filter(|w| !w.is_empty()).map(|w| format!(" «{w}»")).unwrap_or_default()),
            ),
            (wants.is_empty(), Verdict::Hold, "kundrina nuk e shfaq numrin".to_string()),
            (
                true,
                Verdict::Disagree,
                format!("«{}» kërkon «{}», jo «{}»", o.form, want, dative.map_or("", |c| c.form.as_str())),
            ),
        ],
    ))
}

fn direct_object(v: &View, o: &Token, verb: &Token) -> Option<Finding> {
    let full_pron = o.upos == "PRON" && o.feat("PronType") == Some("Prs") && FULL_ACCUSATIVE.contains(&low(&o.form).as_str());
    let def_noun = is_noun(o) && definite(o) == Some(Definite::Def) && !v.time_nouns.contains(&low(&o.lemma).as_str());
    (full_pron || def_noun).then_some(())?;
    let questioned = v.kids(o.id).any(|k| matches!(k.feat("PronType"), Some("Int") | Some("Rel")));
    let cl = clitics_of(v, verb);
    let cluster = cl.iter().any(|c| case(c) == Some(Case::Dat) || c.deprel == "iobj");
    let p = person(o).unwrap_or(Person::P3);
    let wants: Vec<&str> = object_numbers(v, o).into_iter().map(|n| accusative_clitic(p, n)).collect();
    let want = Some(wants.join("/")).filter(|w| !w.is_empty());
    let matching = cl.iter().copied().find(|c| wants.contains(&low(&c.form).as_str()));
    let acc = matching
        .or(cl.iter().copied().find(|c| case(c) == Some(Case::Acc) && c.deprel != "iobj"))
        .or(cl.iter().copied().find(|c| low(&c.form) == "e"));
    let required = full_pron || (o.id < verb.id && !questioned);
    (acc.is_some() || required).then_some(())?;
    let ids = std::iter::once(o.id).chain(std::iter::once(verb.id)).chain(acc.map(|c| c.id)).collect();
    let why = match full_pron {
        true => "përemri vetor",
        false => "kundrina e shquar para foljes",
    };
    Some(decide(
        CLITIC_DOUBLING,
        ids,
        vec![
            (imperative(verb), Verdict::Hold, "urdhërorja e mban trajtën e shkurtër të ngjitur".to_string()),
            (v.unreliable(o), Verdict::Hold, format!("shënimi i «{}» kundërshton mbaresën e vet", o.form)),
            (matching.is_some(), Verdict::Agree, format!("trajta e shkurtër dyfishon «{}»", o.form)),
            (
                acc.is_none(),
                Verdict::Disagree,
                format!("{} «{}» kërkon trajtën e shkurtër{}", why, o.form, want.as_ref().map(|w| format!(" «{w}»")).unwrap_or_default()),
            ),
            (want.is_none(), Verdict::Hold, "kundrina nuk e shfaq numrin".to_string()),
            (cluster, Verdict::Hold, "grup trajtash të shkurtra: numri i kallëzores nuk dallohet".to_string()),
            (
                true,
                Verdict::Disagree,
                format!("«{}» kërkon «{}», jo «{}»", o.form, want.as_deref().unwrap_or(""), acc.map_or("", |c| c.form.as_str())),
            ),
        ],
    ))
}

// ---------------------------------------------------------------- mohimi

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NegContext {
    /// Dëftore, kushtore, habitore, e ardhmja me «do», foljet modale: «nuk/s'».
    Indicative,
    /// Lidhore, dëshirore, urdhërore, trajtat e pashtjelluara: «mos».
    NonIndicative,
}

fn is_negator(t: &Token) -> bool {
    matches!(t.upos.as_str(), "PART" | "ADV") && matches!(low(&t.form).as_str(), "nuk" | "s'" | "s" | "mos")
}

const MODALS: [&str; 12] = ["do", "mund", "mundet", "mundte", "mundem", "mundesh", "mundemi", "mundeni", "munden", "duhet", "duhej", "duhen"];

/// Fjalët që kapërcehen nga pjesëza mohuese deri te folja e mohuar.
fn negation_bridge(t: &Token) -> bool {
    clitic_like(t) || matches!(low(&t.form).as_str(), "të" | "do" | "u" | "s'" | "s" | "nuk" | "mos")
}

/// Konteksti i mohimit, sipas rendit të dëshmisë: «të» para pjesëzës → lidhore;
/// «duke/pa/për» para pjesëzës → trajtë e pashtjelluar; «do» ose modale pas pjesëzës →
/// dëftore; mënyra e foljes së parë pas pjesëzës (pjesëza qëndron menjëherë para grupit
/// foljor); mënyra e foljes kryesore.
fn negation_context(v: &View, n: &Token, h: &Token) -> Result<NegContext, &'static str> {
    let group: Vec<&Token> = std::iter::once(h).chain(v.kids(h.id).filter(|k| is_verbal_group(k) || k.deprel == "mark")).collect();
    let marker = |t: &Token| low(&t.form) == "të" && matches!(t.upos.as_str(), "PART" | "AUX" | "SCONJ");
    let te_before = group.iter().any(|t| marker(t) && t.id < n.id);
    let nonfinite = group.iter().any(|t| matches!(low(&t.form).as_str(), "duke" | "pa" | "për") && t.id < n.id);
    let next = v.toks.iter().filter(|t| t.id > n.id).find(|t| !negation_bridge(t));
    let crossed: Vec<&Token> = v.toks.iter().filter(|t| t.id > n.id && next.map_or(true, |x| t.id < x.id)).collect();
    let crossed_do = crossed.iter().any(|t| low(&t.form) == "do");
    let crossed_te = crossed.iter().any(|t| marker(t));
    let next_modal = next.map_or(false, |t| MODALS.contains(&low(&t.form).as_str()));
    let next_mood = next.filter(|t| matches!(t.upos.as_str(), "VERB" | "AUX")).and_then(mood);
    let m = next_mood.or_else(|| finite_of(v, h).and_then(mood)).or_else(|| mood(h));
    match (te_before, nonfinite, crossed_do || next_modal, crossed_te, m) {
        (true, _, _, _, _) => Ok(NegContext::NonIndicative),
        (false, true, _, _, _) => Ok(NegContext::NonIndicative),
        (false, false, true, _, _) => Ok(NegContext::Indicative),
        (false, false, false, true, _) => Err("«të» pas pjesëzës mohuese: rend i pazakontë"),
        (false, false, false, false, Some(Mood::Ind | Mood::Cnd | Mood::Adm)) => Ok(NegContext::Indicative),
        (false, false, false, false, Some(Mood::Sub | Mood::Opt | Mood::Imp)) => Ok(NegContext::NonIndicative),
        (false, false, false, false, None) => Err("mënyra e foljes nuk dallohet"),
    }
}

fn negation(v: &View) -> Vec<Finding> {
    v.toks.iter().filter(|t| is_negator(t)).map(|n| judge_negation(v, n)).collect()
}

/// Temat e foljeve dhe emrave të frikës e të dyshimit, pas të cilëve «mos» nuk mohon
/// («Druaj mos vonohet», «Dyshova mos ...», «Kam frikë mos vjen»).
const FEAR: [&str; 7] = ["druaj", "drua", "dysh", "frikë", "frikës", "tremb", "kujdes"];

/// Fjala e frikës sipas lemës ose trajtës (lema e shënuar shpesh është vetë trajta).
fn fear_word(t: &Token) -> bool {
    let (l, f) = (low(&t.lemma), low(&t.form));
    FEAR.iter().any(|s| l.starts_with(s) || f.starts_with(s))
}

/// Fjalia e mohuar varet nga një folje a emër i frikës, ose nga një folje me kundrinën «frikë».
fn fearful(v: &View, h: &Token) -> bool {
    v.get(h.head).map_or(false, |g| fear_word(g) || v.kids(g.id).any(fear_word))
}

/// «nuk/s'» mohon dëftoren, kushtoren, habitoren dhe të ardhmen; «mos» mohon lidhoren,
/// urdhëroren, dëshiroren dhe trajtat e pashtjelluara. «Mos» me dëftoren është e rregullt
/// në pyetje («Mos je i lodhur?»), pas «se/që» të frikës, pas foljeve të frikës e të
/// dyshimit, pas «po» kushtore, në «në mos» («Në mos gabohem») dhe në ndalimin me vetën e dytë.
fn judge_negation(v: &View, n: &Token) -> Finding {
    let is_mos = low(&n.form) == "mos";
    match v.get(n.head) {
        None => finding(NEGATION, vec![n.id], Verdict::Hold, "pjesëza mohuese pa folje"),
        Some(h) => {
            let ids = vec![n.id, h.id];
            let fin = finite_of(v, h);
            let lest = v.prev(n).map_or(false, |p| matches!(low(&p.form).as_str(), "se" | "që" | "sikur" | "po" | "në")) || fearful(v, h);
            let prohibitive = fin.map_or(false, |f| mood(f) == Some(Mood::Ind) && person(f) == Some(Person::P2) && f.feat("Tense") == Some("Pres"));
            match (negation_context(v, n, h), is_mos) {
                (Err(why), _) => finding(NEGATION, ids, Verdict::Hold, why),
                (Ok(NegContext::Indicative), false) => finding(NEGATION, ids, Verdict::Agree, format!("«{}» me dëftoren", n.form)),
                (Ok(NegContext::NonIndicative), true) => finding(NEGATION, ids, Verdict::Agree, "«mos» me lidhoren, urdhëroren ose të pashtjelluarën"),
                (Ok(NegContext::NonIndicative), false) => {
                    finding(NEGATION, ids, Verdict::Disagree, format!("me lidhoren dhe urdhëroren mohohet me «mos», jo «{}»", n.form))
                }
                (Ok(NegContext::Indicative), true) => decide(
                    NEGATION,
                    ids,
                    vec![
                        (v.question(), Verdict::Hold, "«mos» pyetëse me dëftoren".to_string()),
                        (lest, Verdict::Hold, "«mos» pas frikës, dyshimit ose kushtit".to_string()),
                        (prohibitive, Verdict::Agree, "ndalim me vetën e dytë: «mos» e rregullt".to_string()),
                        (true, Verdict::Disagree, "me dëftoren mohohet me «nuk» ose «s'», jo «mos»".to_string()),
                    ],
                ),
            }
        }
    }
}

// ---------------------------------------------------------------- vendi i trajtës së shkurtër

/// Fjalët që mund të qëndrojnë midis trajtës së shkurtër dhe foljes: trajta të tjera të
/// shkurtra, pjesëzat «të, do, u, nuk, s', mos» dhe ndihmëset e kohëve të përbëra.
fn clitic_bridge(t: &Token, aux: &[usize]) -> bool {
    clitic_like(t) || aux.contains(&t.id) || matches!(low(&t.form).as_str(), "të" | "do" | "u" | "nuk" | "s'" | "s" | "mos")
}

/// Trajtë e shkurtër edhe kur shënimi i pjesës së ligjëratës mungon a gabon, por jo
/// lidhëzë («e» = «dhe») a parafjalë.
fn clitic_like(t: &Token) -> bool {
    is_clitic(t) || (CLITICS.contains(&low(&t.form).as_str()) && !matches!(t.upos.as_str(), "CCONJ" | "SCONJ" | "ADP" | "PUNCT"))
}

/// Trajta e shkurtër qëndron para trajtës së shtjelluar (para ndihmëses në kohët e
/// përbëra) dhe para pjesores pas «duke/pa/për të»; pas urdhërores pohore mund të
/// ngjitet si enklitikë; me «mos» del para foljes.
fn clitic_placement(v: &View) -> Vec<Finding> {
    v.toks.iter().filter(|c| is_clitic(c)).map(|c| judge_placement(v, c)).collect()
}

fn judge_placement(v: &View, c: &Token) -> Finding {
    match v.get(c.head).filter(|h| matches!(h.upos.as_str(), "VERB" | "AUX")) {
        None => finding(CLITIC_PLACEMENT, vec![c.id], Verdict::Hold, "trajta e shkurtër nuk varet nga folja"),
        Some(h) => {
            let imperative = mood(h) == Some(Mood::Imp);
            let negated = v.kids(h.id).any(|k| low(&k.form) == "mos");
            let aux: Vec<usize> = v.kids(h.id).filter(|k| is_verbal_group(k)).map(|k| k.id).collect();
            let anchor = finite_of(v, h).filter(|_| !imperative).unwrap_or(h);
            let before = c.id < anchor.id;
            let bridged = v.between(c.id, anchor.id).all(|t| clitic_bridge(t, &aux));
            let next = v.toks.iter().filter(|t| t.id > c.id).find(|t| !clitic_bridge(t, &[]));
            let leans = next.map_or(false, |t| matches!(t.upos.as_str(), "VERB" | "AUX") && t.id != anchor.id && t.id != h.id);
            decide(
                CLITIC_PLACEMENT,
                vec![c.id, anchor.id],
                vec![
                    (imperative && !negated && !before, Verdict::Agree, "enklitikë pas urdhërores".to_string()),
                    (before && bridged, Verdict::Agree, format!("«{}» para «{}»", c.form, anchor.form)),
                    (leans, Verdict::Hold, "trajta e shkurtër qëndron para një foljeje tjetër".to_string()),
                    (imperative && negated, Verdict::Disagree, "me «mos» trajta e shkurtër del para foljes".to_string()),
                    (!before, Verdict::Disagree, format!("«{}» duhet të dalë para «{}»", c.form, anchor.form)),
                    (true, Verdict::Disagree, format!("midis «{}» dhe «{}» nuk futet fjalë tjetër", c.form, anchor.form)),
                ],
            )
        }
    }
}

// ---------------------------------------------------------------- kontrolli

/// Të gjitha gjetjet e fjalisë, sipas rendit të `RULES`, vetëm me dëshminë e shënimit.
pub fn check(sentence: &[Token]) -> Vec<Finding> {
    check_custom(sentence, None, &TIME_NOUNS)
}

/// Si `check`, me leksikun si dëshmitar të gjinisë së emrave dhe të mbiemrave të emërzuar.
pub fn check_with(sentence: &[Token], lexicon: &Lexicon) -> Vec<Finding> {
    check_custom(sentence, Some(lexicon), &TIME_NOUNS)
}

/// Kontrolli me leksik të zgjedhur dhe me tabelën e emrave të kohës që nuk dyfishohen
/// (`TIME_NOUNS` në `check`; bosh për ta matur rregullin pa tabelë).
pub fn check_custom(sentence: &[Token], lexicon: Option<&Lexicon>, time_nouns: &[&str]) -> Vec<Finding> {
    run(View { toks: sentence, lex: lexicon, time_nouns })
}

fn run(v: View) -> Vec<Finding> {
    let mut out = subject_verb(&v);
    out.extend(noun_adjective(&v));
    out.extend(adjective_article(&v));
    out.extend(genitive_article(&v));
    out.extend(determiner_noun(&v));
    out.extend(clitic_doubling(&v));
    out.extend(negation(&v));
    out.extend(clitic_placement(&v));
    out
}

// ---------------------------------------------------------------- montuesi

/// Grupi emëror që i jepet montuesit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Np {
    pub text: String,
    pub person: Person,
    pub number: Number,
    pub gender: Gender,
    pub definite: Definite,
    pub case: Case,
}

const THIRD_PERSONAL: [&str; 3] = ["atë", "ata", "ato"];

/// Trajta e shkurtër e kundrinës dhe rasa e saj.
#[derive(Debug, Clone, Copy)]
struct Clitic {
    form: &'static str,
    dative: bool,
}

/// Trajta e shkurtër që kërkon kundrina: dhanorja gjithmonë; kallëzorja kur është
/// përemër vetor ose kundrinë e shquar e nënkuptuar (tekst bosh).
fn object_clitic(o: &Np) -> Option<Clitic> {
    let pronoun = THIRD_PERSONAL.contains(&o.text.trim().to_lowercase().as_str());
    let dropped = o.text.trim().is_empty() && o.definite == Definite::Def;
    match (o.case, o.person, pronoun || dropped) {
        (Case::Dat, p, _) => Some(Clitic { form: dative_clitic(p, o.number), dative: true }),
        (Case::Acc, Person::P1 | Person::P2, _) => Some(Clitic { form: accusative_clitic(o.person, o.number), dative: false }),
        (Case::Acc, Person::P3, true) => Some(Clitic { form: accusative_clitic(Person::P3, o.number), dative: false }),
        _ => None,
    }
}

/// «të» + trajta e shkurtër: «ta», «t'i», «t'u»; të tjerat ndahen («të më», «të na»).
fn with_te(clitic: Option<Clitic>) -> String {
    match clitic.map(|c| c.form) {
        None => "të".to_string(),
        Some("e") => "ta".to_string(),
        Some(c) => match c.starts_with(['i', 'u']) {
            true => format!("t'{c}"),
            false => format!("të {c}"),
        },
    }
}

/// Trajta e shkurtër dhanore para «u» joveprore: «i/u + u» → «iu», «më + u» → «m'u»,
/// «të + u» → «t'u», «na u», «ju u». Kallëzorja nuk shkon me foljen joveprore.
fn before_nonactive(c: Clitic) -> Option<String> {
    match (c.dative, c.form) {
        (false, _) => None,
        (true, "i" | "u") => Some("iu".to_string()),
        (true, "më") => Some("m'u".to_string()),
        (true, "të") => Some("t'u".to_string()),
        (true, f) => Some(format!("{f} u")),
    }
}

/// Urdhërorja pohore e merr trajtën e shkurtër të vetës së tretë si enklitikë; pas zanores
/// futet «j» («lexoje», «jepi», «thuaju»); në shumës enklitika del para «-ni» («lexojeni»).
fn enclitic(verb: &str, clitic: &str, plural: bool) -> Option<String> {
    let (stem, tail) = match plural {
        true => (verb.strip_suffix("ni")?, "ni"),
        false => (verb, ""),
    };
    let glide = match stem.chars().last()? {
        'a' | 'e' | 'ë' | 'i' | 'o' | 'u' | 'y' => "j",
        _ => "",
    };
    Some(format!("{stem}{glide}{clitic}{tail}"))
}

/// Dëshirorja e foljeve me temë në zanore: «-fsha, -fsh, -ftë, -fshim, -fshi, -fshin» pas
/// zanores («lexoftë», «qofsh»).
fn optative_shape(word: &str) -> bool {
    const ENDINGS: [&str; 6] = ["fshin", "fshim", "fsha", "fshi", "fsh", "ftë"];
    let w = low(word);
    ENDINGS
        .iter()
        .filter_map(|e| w.strip_suffix(e))
        .any(|stem| stem.chars().last().map_or(false, |c| "aeëiouy".contains(c)))
}

/// Fjala e parë e grupit foljor dhe vendi që i jep mohimit e trajtës së shkurtër.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lead {
    /// Pa pjesëz: mënyra e jep thirrësi.
    Bare,
    /// «të» e lidhores: «të mos e lexojë», «ta lexojë».
    Subjunctive,
    /// «do të», «mund të», «duhet të»: «nuk» para, trajta e shkurtër pas «të».
    Modal,
    /// «le të», «për të»: «mos» pas «të».
    Particle,
    /// «po» e vazhdueshmes: «nuk po e lexon».
    Progressive,
    /// «duke»: «duke mos e lexuar».
    Gerund,
    /// «pa»: «pa e lexuar»; nuk mohohet më tej.
    Privative,
    /// «u» joveprore: «nuk iu kujtua».
    NonActive,
}

fn lead_of(words: &[&str]) -> (Lead, usize) {
    match words {
        ["le" | "për", "të", ..] => (Lead::Particle, 2),
        ["të", _, ..] => (Lead::Subjunctive, 1),
        [m, "të", ..] => match MODALS.contains(m) {
            true => (Lead::Modal, 2),
            false => (Lead::Bare, 0),
        },
        ["po", _, ..] => (Lead::Progressive, 1),
        ["duke", _, ..] => (Lead::Gerund, 1),
        ["pa", _, ..] => (Lead::Privative, 1),
        ["u", _, ..] => (Lead::NonActive, 1),
        _ => (Lead::Bare, 0),
    }
}

/// Grupi foljor pa pjesëz sipas mënyrës: dëftorja, kushtorja, habitorja me «nuk»;
/// dëshirorja dhe urdhërorja e mohuar me «mos»; urdhërorja pohore me enklitikë të vetës
/// së tretë dhe me proklitikë të vetës së parë e të dytë («Më thuaj»). Lidhorja pa «të»
/// nuk montohet.
fn bare_group(core: &str, mood: Mood, plural: bool, cl: Option<Clitic>, negative: bool) -> Option<Vec<Option<String>>> {
    let clw = cl.map(|c| c.form.to_string());
    let third = cl.map_or(false, |c| matches!(c.form, "e" | "i" | "u"));
    match (mood, negative) {
        (Mood::Ind | Mood::Cnd | Mood::Adm, _) => Some(vec![negative.then(|| "nuk".to_string()), clw, Some(core.to_string())]),
        (Mood::Opt, _) | (Mood::Imp, true) => Some(vec![negative.then(|| "mos".to_string()), clw, Some(core.to_string())]),
        (Mood::Sub, _) => None,
        (Mood::Imp, false) => match (third, core.contains(' ')) {
            (false, _) => Some(vec![clw, Some(core.to_string())]),
            (true, false) => enclitic(core, cl.map_or("", |c| c.form), plural).map(|w| vec![Some(w)]),
            (true, true) => None,
        },
    }
}

/// Grupi foljor me mohimin dhe trajtën e shkurtër në vendin e tyre; `None` kur trajta
/// foljore mban vetë mohim, kur trajta e shkurtër do të dyfishohej, ose kur kombinimi nuk
/// ka trajtë të rregullt.
fn verb_group(verb: &str, mood: Mood, plural: bool, cl: Option<Clitic>, negative: bool) -> Option<Vec<String>> {
    let words: Vec<&str> = verb.split_whitespace().collect();
    let lower: Vec<String> = words.iter().map(|w| low(w)).collect();
    let lw: Vec<&str> = lower.iter().map(String::as_str).collect();
    let negated = lw.iter().any(|w| matches!(*w, "nuk" | "s" | "mos") || w.starts_with("s'"));
    let (lead, k) = lead_of(&lw);
    let doubled = cl.is_some() && lw.get(k).map_or(false, |w| CLITICS.contains(w));
    (!negated && !doubled).then_some(())?;
    let core = words.get(k..).filter(|c| !c.is_empty())?.join(" ");
    let first = words.first().map(|w| w.to_string());
    let nuk = negative.then(|| "nuk".to_string());
    let clw = cl.map(|c| c.form.to_string());
    let group = match (lead, negative) {
        (Lead::Modal, _) => Some(vec![nuk, first, Some(with_te(cl)), Some(core)]),
        (Lead::Particle, true) => Some(vec![first, Some("të mos".to_string()), clw, Some(core)]),
        (Lead::Particle, false) => Some(vec![first, Some(with_te(cl)), Some(core)]),
        (Lead::Subjunctive, true) => Some(vec![Some("të mos".to_string()), clw, Some(core)]),
        (Lead::Subjunctive, false) => Some(vec![Some(with_te(cl)), Some(core)]),
        (Lead::Progressive, _) => Some(vec![nuk, first, clw, Some(core)]),
        (Lead::Gerund, _) => Some(vec![first, negative.then(|| "mos".to_string()), clw, Some(core)]),
        (Lead::Privative, true) => None,
        (Lead::Privative, false) => Some(vec![first, clw, Some(core)]),
        (Lead::NonActive, _) => cl.map_or(Some("u".to_string()), before_nonactive).map(|u| vec![nuk, Some(u), Some(core)]),
        (Lead::Bare, _) => bare_group(&core, mood, plural, cl, negative),
    };
    group.map(|g| g.into_iter().flatten().collect())
}

/// Montuesi i fjalisë së thjeshtë në mënyrën e dhënë. Rendi i fiksuar:
/// 1. kryefjala (kur ka tekst);
/// 2. grupi foljor: mohimi, trajta e shkurtër dhe folja sipas pjesëzës së parë:
///    - pa pjesëz: dëftore/kushtore/habitore «nuk e lexon»; dëshirore «mos e lexoftë»;
///      urdhërore «mos e lexo», pohore «lexoje», «më thuaj»;
///    - «të»: «ta lexojë», «të mos e lexojë»; «do/mund/duhet të»: «nuk do ta lexojë»,
///      «mund t'i shkruajë»; «le/për të»: «le ta lexojë», «për të mos e lexuar»;
///    - «po»: «nuk po e lexon»; «duke»: «duke mos e lexuar»; «pa»: «pa e lexuar»;
///    - «u» joveprore: «iu kujtua», «m'u duk»;
/// 3. kundrina pas foljes.
///
/// Dyfishimi: kundrina dhanore gjithmonë («i», «u», «më», «të», «na», «ju»); përemri
/// vetor kallëzor gjithmonë; emri i shquar kallëzor pas foljes jo (rendi asnjanës).
/// Mënyra vlen vetëm për trajtat pa pjesëz. `None` (asnjë hamendje) kur trajta foljore ka
/// vetë mohim ose trajtë të shkurtër që do të dyfishohej, lidhore pa «të», «pa» e mohuar,
/// kallëzore me «u» joveprore, urdhërore shumës pa «-ni».
pub fn clause_in_mood(subject: Np, verb_form: &str, mood: Mood, object: Option<Np>, negative: bool) -> Option<String> {
    let cl = object.as_ref().and_then(object_clitic);
    let group = verb_group(verb_form, mood, subject.number == Number::Plur, cl, negative)?;
    Some(
        std::iter::once(subject.text.trim().to_string())
            .chain(group)
            .chain(object.map(|o| o.text.trim().to_string()))
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Montuesi për fjalitë e Nurës: trajta pa pjesëz merret si dëftore (ose dëshirore kur
/// ka mbaresën «-fsh/-ftë» pas zanores); urdhërorja jepet me `clause_in_mood`. Rendi
/// dhe dyfishimi si te `clause_in_mood`; varg bosh kur nuk ka trajtë të rregullt.
pub fn clause(subject: Np, verb_form: &str, object: Option<Np>, negative: bool) -> String {
    let mood = match verb_form.split_whitespace().next().map_or(false, optative_shape) {
        true => Mood::Opt,
        false => Mood::Ind,
    };
    clause_in_mood(subject, verb_form, mood, object, negative).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: usize, form: &str, upos: &str, feats: &str, head: usize, deprel: &str) -> Token {
        Token::new(id, form, "_", upos, feats, head, deprel)
    }

    fn verdicts(s: &[Token], rule: &str) -> Vec<Verdict> {
        check(s).into_iter().filter(|f| f.rule == rule).map(|f| f.verdict).collect()
    }

    const NOM_DEF_M_PL: &str = "Case=Nom|Definite=Def|Gender=Masc|Number=Plur";
    const NOM_DEF_M_SG: &str = "Case=Nom|Definite=Def|Gender=Masc|Number=Sing";
    const NOM_DEF_F_SG: &str = "Case=Nom|Definite=Def|Gender=Fem|Number=Sing";
    const ACC_DEF_M_SG: &str = "Case=Acc|Definite=Def|Gender=Masc|Number=Sing";
    const ACC_IND_M_SG: &str = "Case=Acc|Definite=Ind|Gender=Masc|Number=Sing";
    const IND_3PL: &str = "Mood=Ind|Number=Plur|Person=3|Tense=Pres|VerbForm=Fin";
    const IND_3SG: &str = "Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin";
    const IND_1SG: &str = "Mood=Ind|Number=Sing|Person=1|Tense=Past|VerbForm=Fin";

    #[test]
    fn subject_and_verb_agree_in_person_and_number() {
        let good = vec![t(1, "Djemtë", "NOUN", NOM_DEF_M_PL, 2, "nsubj"), t(2, "vijnë", "VERB", IND_3PL, 0, "root")];
        assert_eq!(verdicts(&good, SUBJECT_VERB), vec![Verdict::Agree]);
        let bad = vec![t(1, "Djemtë", "NOUN", NOM_DEF_M_PL, 2, "nsubj"), t(2, "vjen", "VERB", IND_3SG, 0, "root")];
        assert_eq!(verdicts(&bad, SUBJECT_VERB), vec![Verdict::Disagree]);
        let person = vec![t(1, "Unë", "PRON", "", 2, "nsubj"), t(2, "shkon", "VERB", IND_3SG, 0, "root")];
        assert_eq!(verdicts(&person, SUBJECT_VERB), vec![Verdict::Disagree]);
    }

    #[test]
    fn coordinated_polite_and_existential_subjects() {
        let coord = vec![
            t(1, "Beni", "PROPN", NOM_DEF_M_SG, 4, "nsubj"),
            t(2, "dhe", "CCONJ", "", 3, "cc"),
            t(3, "Ana", "PROPN", NOM_DEF_F_SG, 1, "conj"),
            t(4, "shkuan", "VERB", "Mood=Ind|Number=Plur|Person=3|Tense=Past|VerbForm=Fin", 0, "root"),
        ];
        assert_eq!(verdicts(&coord, SUBJECT_VERB), vec![Verdict::Agree]);
        let polite = vec![
            t(1, "Ju", "PRON", "Case=Nom|Number=Sing|Person=2|PronType=Prs", 2, "nsubj"),
            t(2, "jeni", "AUX", "Mood=Ind|Number=Plur|Person=2|Tense=Pres|VerbForm=Fin", 0, "root"),
        ];
        assert_eq!(verdicts(&polite, SUBJECT_VERB), vec![Verdict::Agree]);
        let existential = vec![
            Token::new(1, "Ka", "kam", "VERB", IND_3SG, 0, "root"),
            t(2, "njerëz", "NOUN", "Case=Nom|Definite=Ind|Gender=Masc|Number=Plur", 1, "nsubj"),
        ];
        assert_eq!(verdicts(&existential, SUBJECT_VERB), vec![Verdict::Hold]);
    }

    #[test]
    fn adjective_agrees_with_its_noun() {
        let good = vec![
            t(1, "vajza", "NOUN", NOM_DEF_F_SG, 0, "root"),
            t(2, "e", "DET", "", 3, "det:adj"),
            t(3, "bukur", "ADJ", "Gender=Fem|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&good, NOUN_ADJECTIVE), vec![Verdict::Agree]);
        assert_eq!(verdicts(&good, ADJECTIVE_ARTICLE), vec![Verdict::Agree]);
        let bad = vec![
            t(1, "vajza", "NOUN", NOM_DEF_F_SG, 0, "root"),
            t(2, "i", "DET", "", 3, "det:adj"),
            t(3, "bukur", "ADJ", "Gender=Masc|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&bad, NOUN_ADJECTIVE), vec![Verdict::Disagree]);
        assert_eq!(verdicts(&bad, ADJECTIVE_ARTICLE), vec![Verdict::Disagree]);
    }

    #[test]
    fn article_follows_contact_with_the_definite_noun() {
        let contact = vec![
            t(1, "djalin", "NOUN", ACC_DEF_M_SG, 0, "root"),
            t(2, "e", "DET", "", 3, "det:adj"),
            t(3, "mirë", "ADJ", "Gender=Masc|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&contact, ADJECTIVE_ARTICLE), vec![Verdict::Agree]);
        let indefinite = vec![
            t(1, "një", "DET", "", 2, "det"),
            t(2, "djalë", "NOUN", ACC_IND_M_SG, 0, "root"),
            t(3, "të", "DET", "", 4, "det:adj"),
            t(4, "mirë", "ADJ", "Gender=Masc|Number=Sing", 2, "amod"),
        ];
        assert_eq!(verdicts(&indefinite, ADJECTIVE_ARTICLE), vec![Verdict::Agree]);
        let wrong = vec![
            t(1, "djalin", "NOUN", ACC_DEF_M_SG, 0, "root"),
            t(2, "të", "DET", "", 3, "det:adj"),
            t(3, "mirë", "ADJ", "Gender=Masc|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&wrong, ADJECTIVE_ARTICLE), vec![Verdict::Disagree]);
        let oblique = vec![
            t(1, "vajzës", "NOUN", "Case=Gen|Definite=Def|Gender=Fem|Number=Sing", 0, "root"),
            t(2, "së", "DET", "", 3, "det:adj"),
            t(3, "mirë", "ADJ", "Gender=Fem|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&oblique, ADJECTIVE_ARTICLE), vec![Verdict::Agree]);
    }

    #[test]
    fn predicative_article_follows_the_subject() {
        let fem = vec![
            t(1, "Vajza", "NOUN", NOM_DEF_F_SG, 4, "nsubj"),
            t(2, "është", "AUX", IND_3SG, 4, "cop"),
            t(3, "e", "DET", "", 4, "det:adj"),
            t(4, "mirë", "ADJ", "Gender=Fem|Number=Sing", 0, "root"),
        ];
        assert_eq!(verdicts(&fem, ADJECTIVE_ARTICLE), vec![Verdict::Agree]);
        assert_eq!(verdicts(&fem, SUBJECT_VERB), vec![Verdict::Agree]);
        let plural = vec![
            t(1, "Djemtë", "NOUN", NOM_DEF_M_PL, 4, "nsubj"),
            t(2, "janë", "AUX", IND_3PL, 4, "cop"),
            t(3, "i", "DET", "", 4, "det:adj"),
            t(4, "mirë", "ADJ", "Gender=Masc|Number=Plur", 0, "root"),
        ];
        assert_eq!(verdicts(&plural, ADJECTIVE_ARTICLE), vec![Verdict::Disagree]);
    }

    #[test]
    fn genitive_linker_agrees_with_the_possessed_noun() {
        let gen = |head: &str, feats: &str, art: &str| {
            vec![
                t(1, head, "NOUN", feats, 0, "root"),
                t(2, art, "DET", "", 3, "det"),
                t(3, "Benit", "PROPN", "Case=Gen|Definite=Def|Gender=Masc|Number=Sing", 1, "nmod:poss"),
            ]
        };
        assert_eq!(verdicts(&gen("libri", NOM_DEF_M_SG, "i"), GENITIVE_ARTICLE), vec![Verdict::Agree]);
        assert_eq!(verdicts(&gen("shtëpia", NOM_DEF_F_SG, "e"), GENITIVE_ARTICLE), vec![Verdict::Agree]);
        assert_eq!(verdicts(&gen("librin", ACC_DEF_M_SG, "e"), GENITIVE_ARTICLE), vec![Verdict::Agree]);
        assert_eq!(
            verdicts(&gen("shtëpisë", "Case=Dat|Definite=Def|Gender=Fem|Number=Sing", "së"), GENITIVE_ARTICLE),
            vec![Verdict::Agree]
        );
        assert_eq!(verdicts(&gen("libri", NOM_DEF_M_SG, "e"), GENITIVE_ARTICLE), vec![Verdict::Disagree]);
        assert_eq!(verdicts(&gen("libër", ACC_IND_M_SG, "të"), GENITIVE_ARTICLE), vec![Verdict::Agree]);
    }

    #[test]
    fn determiners_and_numerals_agree_with_the_noun() {
        let pair = |d: &str, upos: &str, rel: &str, feats: &str| vec![t(1, d, upos, "", 2, rel), t(2, "x", "NOUN", feats, 0, "root")];
        let nom_ind_m_sg = "Case=Nom|Definite=Ind|Gender=Masc|Number=Sing";
        let nom_ind_f_pl = "Case=Nom|Definite=Ind|Gender=Fem|Number=Plur";
        assert_eq!(verdicts(&pair("ky", "DET", "det", nom_ind_m_sg), DETERMINER_NOUN), vec![Verdict::Agree]);
        assert_eq!(verdicts(&pair("kjo", "DET", "det", nom_ind_m_sg), DETERMINER_NOUN), vec![Verdict::Disagree]);
        assert_eq!(verdicts(&pair("këta", "DET", "det", nom_ind_m_sg), DETERMINER_NOUN), vec![Verdict::Disagree]);
        assert_eq!(verdicts(&pair("tri", "NUM", "nummod", nom_ind_f_pl), DETERMINER_NOUN), vec![Verdict::Agree]);
        assert_eq!(verdicts(&pair("tre", "NUM", "nummod", nom_ind_f_pl), DETERMINER_NOUN), vec![Verdict::Disagree]);
        let one_def = vec![t(1, "një", "DET", "", 2, "det"), t(2, "libri", "NOUN", NOM_DEF_M_SG, 0, "root")];
        assert_eq!(verdicts(&one_def, DETERMINER_NOUN), vec![Verdict::Disagree]);
        let colloquial = vec![t(1, "ai", "DET", "", 2, "det"), t(2, "njeriu", "NOUN", NOM_DEF_M_SG, 0, "root")];
        assert_eq!(verdicts(&colloquial, DETERMINER_NOUN), vec![Verdict::Hold]);
    }

    #[test]
    fn indirect_and_fronted_objects_are_doubled() {
        let doubled = vec![
            t(1, "I", "PRON", "Case=Dat|Number=Sing|Person=3|PronType=Prs", 2, "iobj"),
            t(2, "dhashë", "VERB", IND_1SG, 0, "root"),
            t(3, "Benit", "PROPN", "Case=Dat|Definite=Def|Gender=Masc|Number=Sing", 2, "iobj"),
            t(4, "librin", "NOUN", ACC_DEF_M_SG, 2, "obj"),
        ];
        assert_eq!(verdicts(&doubled, CLITIC_DOUBLING), vec![Verdict::Agree]);
        let missing = vec![
            t(1, "Dhashë", "VERB", IND_1SG, 0, "root"),
            t(2, "Benit", "PROPN", "Case=Dat|Definite=Def|Gender=Masc|Number=Sing", 1, "iobj"),
            t(3, "librin", "NOUN", ACC_DEF_M_SG, 1, "obj"),
        ];
        assert_eq!(verdicts(&missing, CLITIC_DOUBLING), vec![Verdict::Disagree]);
        let fronted = |clitic: Option<&str>, feats: &str| {
            let mut s = vec![t(1, "Librat", "NOUN", feats, 3, "obj")];
            s.extend(clitic.map(|c| t(2, c, "PRON", "Case=Acc|Person=3|PronType=Prs", 3, "expl")));
            s.push(t(3, "lexova", "VERB", IND_1SG, 0, "root"));
            s
        };
        let pl = "Case=Acc|Definite=Def|Gender=Masc|Number=Plur";
        assert_eq!(verdicts(&fronted(Some("i"), pl), CLITIC_DOUBLING), vec![Verdict::Agree]);
        assert_eq!(verdicts(&fronted(Some("e"), pl), CLITIC_DOUBLING), vec![Verdict::Disagree]);
        assert_eq!(verdicts(&fronted(None, pl), CLITIC_DOUBLING), vec![Verdict::Disagree]);
    }

    #[test]
    fn negation_particle_follows_the_mood() {
        let neg = |words: &[(&str, &str, &str, &str)]| {
            words.iter().enumerate().map(|(i, (f, u, feats, rel))| t(i + 1, f, u, feats, words.len(), rel)).collect::<Vec<_>>()
        };
        let ind = neg(&[("Nuk", "PART", "", "advmod"), ("shkoj", "VERB", "Mood=Ind|Number=Sing|Person=1|Tense=Pres", "root")]);
        assert_eq!(verdicts(&ind, NEGATION), vec![Verdict::Agree]);
        let sub = "Mood=Sub|Number=Sing|Person=2|Tense=Pres";
        let mos_sub = neg(&[("Të", "PART", "", "mark"), ("mos", "PART", "", "advmod"), ("shkosh", "VERB", sub, "root")]);
        assert_eq!(verdicts(&mos_sub, NEGATION), vec![Verdict::Agree]);
        let nuk_sub = neg(&[("Të", "PART", "", "mark"), ("nuk", "PART", "", "advmod"), ("shkosh", "VERB", sub, "root")]);
        assert_eq!(verdicts(&nuk_sub, NEGATION), vec![Verdict::Disagree]);
        let imp = neg(&[("Mos", "PART", "", "advmod"), ("shko", "VERB", "Mood=Imp|Number=Sing|Person=2", "root")]);
        assert_eq!(verdicts(&imp, NEGATION), vec![Verdict::Agree]);
        let mos_ind = neg(&[("Mos", "PART", "", "advmod"), ("shkon", "VERB", IND_3SG, "root")]);
        assert_eq!(verdicts(&mos_ind, NEGATION), vec![Verdict::Disagree]);
        let future = |p: &str| {
            neg(&[(p, "PART", "", "advmod"), ("do", "PART", "", "mark"), ("të", "PART", "", "mark"), ("shkoj", "VERB", sub, "root")])
        };
        assert_eq!(verdicts(&future("Nuk"), NEGATION), vec![Verdict::Agree]);
        assert_eq!(verdicts(&future("Mos"), NEGATION), vec![Verdict::Disagree]);
        let question = vec![
            t(1, "Mos", "PART", "", 4, "advmod"),
            t(2, "je", "AUX", "Mood=Ind|Number=Sing|Person=2|Tense=Pres", 4, "cop"),
            t(3, "i", "DET", "", 4, "det:adj"),
            t(4, "lodhur", "ADJ", "Gender=Masc|Number=Sing", 0, "root"),
            t(5, "?", "PUNCT", "", 4, "punct"),
        ];
        assert_eq!(verdicts(&question, NEGATION), vec![Verdict::Hold]);
    }

    #[test]
    fn clitic_precedes_the_finite_verb() {
        let cl = "Case=Acc|Number=Sing|Person=3|PronType=Prs";
        let before = vec![t(1, "E", "PRON", cl, 2, "obj"), t(2, "lexova", "VERB", IND_1SG, 0, "root")];
        assert_eq!(verdicts(&before, CLITIC_PLACEMENT), vec![Verdict::Agree]);
        let after = vec![t(1, "Lexova", "VERB", IND_1SG, 0, "root"), t(2, "e", "PRON", cl, 1, "obj")];
        assert_eq!(verdicts(&after, CLITIC_PLACEMENT), vec![Verdict::Disagree]);
        let perfect = vec![
            t(1, "E", "PRON", cl, 3, "obj"),
            t(2, "kam", "AUX", "Mood=Ind|Number=Sing|Person=1|Tense=Pres|VerbForm=Fin", 3, "aux"),
            t(3, "lexuar", "VERB", "VerbForm=Part", 0, "root"),
        ];
        assert_eq!(verdicts(&perfect, CLITIC_PLACEMENT), vec![Verdict::Agree]);
        let split = vec![
            t(1, "Kam", "AUX", "Mood=Ind|Number=Sing|Person=1|Tense=Pres|VerbForm=Fin", 3, "aux"),
            t(2, "e", "PRON", cl, 3, "obj"),
            t(3, "lexuar", "VERB", "VerbForm=Part", 0, "root"),
        ];
        assert_eq!(verdicts(&split, CLITIC_PLACEMENT), vec![Verdict::Disagree]);
        let gerund = vec![t(1, "duke", "PART", "", 3, "mark"), t(2, "e", "PRON", cl, 3, "obj"), t(3, "mbajtur", "VERB", "VerbForm=Part", 0, "root")];
        assert_eq!(verdicts(&gerund, CLITIC_PLACEMENT), vec![Verdict::Agree]);
        let imp = "Mood=Imp|Number=Sing|Person=2";
        let prohibitive = vec![t(1, "Mos", "PART", "", 3, "advmod"), t(2, "e", "PRON", cl, 3, "obj"), t(3, "merr", "VERB", imp, 0, "root")];
        assert_eq!(verdicts(&prohibitive, CLITIC_PLACEMENT), vec![Verdict::Agree]);
        let late = vec![t(1, "Mos", "PART", "", 2, "advmod"), t(2, "merr", "VERB", imp, 0, "root"), t(3, "e", "PRON", cl, 2, "obj")];
        assert_eq!(verdicts(&late, CLITIC_PLACEMENT), vec![Verdict::Disagree]);
    }

    #[test]
    fn article_table_matches_the_paradigm() {
        assert_eq!(article(Gender::Masc, Number::Sing, Case::Nom, false), Some("i"));
        assert_eq!(article(Gender::Fem, Number::Sing, Case::Nom, true), Some("e"));
        assert_eq!(article(Gender::Masc, Number::Sing, Case::Acc, true), Some("e"));
        assert_eq!(article(Gender::Masc, Number::Sing, Case::Acc, false), Some("të"));
        assert_eq!(article(Gender::Fem, Number::Sing, Case::Gen, true), Some("së"));
        assert_eq!(article(Gender::Fem, Number::Sing, Case::Gen, false), Some("të"));
        assert_eq!(article(Gender::Masc, Number::Plur, Case::Nom, true), Some("e"));
        assert_eq!(article(Gender::Fem, Number::Plur, Case::Abl, true), Some("të"));
        assert_eq!(article(Gender::Neut, Number::Sing, Case::Nom, true), None);
    }

    #[test]
    fn conllu_reader_keeps_ids_and_skips_ranges() {
        let text = "# sent_id = a-1\n# text = Ma dha.\n1-2\tma\t_\t_\t_\t_\t_\t_\t_\t_\n1\tmë\tunë\tPRON\t_\tCase=Dat|Number=Sing|Person=1\t3\tiobj\t_\t_\n2\te\tai\tPRON\t_\tCase=Acc\t3\tobj\t_\t_\n3\tdha\tjap\tVERB\t_\tMood=Ind\t0\troot\t_\t_\n3.1\tx\tx\tX\t_\t_\t_\t_\t_\t_\n\n# sent_id = a-2\n1\tPo\tpo\tINTJ\tMood=Ind\t_\t0\troot\t_\t_\n";
        let s = parse_conllu_ids(text);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].0, "a-1");
        assert_eq!(s[0].1.iter().map(|t| t.form.as_str()).collect::<Vec<_>>(), vec!["më", "e", "dha"]);
        assert_eq!(s[1].1[0].feats, "Mood=Ind");
        assert_eq!(parse_conllu(text).len(), 2);
        assert!(check(&[]).is_empty());
        assert!(parse_conllu("\t\t\n#\n1\t").is_empty());
    }

    fn np(text: &str, person: Person, number: Number, gender: Gender, definite: Definite, case: Case) -> Np {
        Np { text: text.to_string(), person, number, gender, definite, case }
    }

    #[test]
    fn clause_orders_negation_clitic_verb_and_object() {
        let beni = || np("Beni", Person::P3, Number::Sing, Gender::Masc, Definite::Def, Case::Nom);
        let librin = || np("librin", Person::P3, Number::Sing, Gender::Masc, Definite::Def, Case::Acc);
        let nenes = || np("nënës", Person::P3, Number::Sing, Gender::Fem, Definite::Def, Case::Dat);
        let ate = || np("atë", Person::P3, Number::Sing, Gender::Masc, Definite::Def, Case::Acc);
        assert_eq!(clause(beni(), "lexon", Some(librin()), false), "Beni lexon librin");
        assert_eq!(clause(beni(), "lexon", Some(librin()), true), "Beni nuk lexon librin");
        assert_eq!(clause(beni(), "i shkruan", None, false), "Beni i shkruan");
        assert_eq!(clause(beni(), "shkruan", Some(nenes()), false), "Beni i shkruan nënës");
        assert_eq!(clause(beni(), "shkruan", Some(nenes()), true), "Beni nuk i shkruan nënës");
        assert_eq!(clause(beni(), "të shkruajë", Some(nenes()), false), "Beni t'i shkruajë nënës");
        assert_eq!(clause(beni(), "të lexojë", Some(ate()), false), "Beni ta lexojë atë");
        assert_eq!(clause(beni(), "të lexojë", Some(ate()), true), "Beni të mos e lexojë atë");
        assert_eq!(clause(beni(), "do të lexojë", Some(ate()), true), "Beni nuk do ta lexojë atë");
        let prinderve = np("prindërve", Person::P3, Number::Plur, Gender::Masc, Definite::Def, Case::Dat);
        assert_eq!(clause(beni(), "shkruan", Some(prinderve), false), "Beni u shkruan prindërve");
        let mua = np("mua", Person::P1, Number::Sing, Gender::Fem, Definite::Def, Case::Acc);
        let ana = np("Ana", Person::P3, Number::Sing, Gender::Fem, Definite::Def, Case::Nom);
        assert_eq!(clause(ana, "pa", Some(mua), false), "Ana më pa mua");
        let dropped = np("", Person::P1, Number::Sing, Gender::Masc, Definite::Def, Case::Nom);
        let it = np("", Person::P3, Number::Sing, Gender::Masc, Definite::Def, Case::Acc);
        assert_eq!(clause(dropped, "lexoj", Some(it), true), "nuk e lexoj");
    }

    #[test]
    fn clause_places_clitic_and_negation_by_the_verb_particle() {
        let beni = || np("Beni", Person::P3, Number::Sing, Gender::Masc, Definite::Def, Case::Nom);
        let ate = || np("atë", Person::P3, Number::Sing, Gender::Masc, Definite::Def, Case::Acc);
        let nenes = || np("nënës", Person::P3, Number::Sing, Gender::Fem, Definite::Def, Case::Dat);
        let atyre = || np("atyre", Person::P3, Number::Plur, Gender::Masc, Definite::Def, Case::Dat);
        assert_eq!(clause(beni(), "mund të lexojë", Some(ate()), false), "Beni mund ta lexojë atë");
        assert_eq!(clause(beni(), "mund të lexojë", Some(ate()), true), "Beni nuk mund ta lexojë atë");
        assert_eq!(clause(beni(), "duhet të shkruajë", Some(nenes()), false), "Beni duhet t'i shkruajë nënës");
        assert_eq!(clause(beni(), "le të lexojë", Some(ate()), false), "Beni le ta lexojë atë");
        assert_eq!(clause(beni(), "le të lexojë", Some(ate()), true), "Beni le të mos e lexojë atë");
        assert_eq!(clause(beni(), "për të lexuar", Some(ate()), true), "Beni për të mos e lexuar atë");
        assert_eq!(clause(beni(), "po lexon", Some(ate()), true), "Beni nuk po e lexon atë");
        assert_eq!(clause(beni(), "duke lexuar", Some(ate()), true), "Beni duke mos e lexuar atë");
        assert_eq!(clause(beni(), "pa lexuar", Some(ate()), false), "Beni pa e lexuar atë");
        assert_eq!(clause(beni(), "pa lexuar", Some(ate()), true), "");
        assert_eq!(clause(beni(), "lexoftë", Some(ate()), true), "Beni mos e lexoftë atë");
        assert_eq!(clause(beni(), "u kujtua", Some(nenes()), false), "Beni iu kujtua nënës");
        assert_eq!(clause(beni(), "u kujtua", Some(atyre()), true), "Beni nuk iu kujtua atyre");
        let mua = np("mua", Person::P1, Number::Sing, Gender::Fem, Definite::Def, Case::Dat);
        assert_eq!(clause(beni(), "u duk", Some(mua.clone()), false), "Beni m'u duk mua");
        assert_eq!(clause(beni(), "u kujtua", Some(ate()), false), "");
        assert_eq!(clause(beni(), "nuk lexon", Some(ate()), false), "");
        assert_eq!(clause(beni(), "e lexon", Some(ate()), false), "");
        let ti = || np("", Person::P2, Number::Sing, Gender::Masc, Definite::Def, Case::Nom);
        let ju = || np("", Person::P2, Number::Plur, Gender::Masc, Definite::Def, Case::Nom);
        assert_eq!(clause_in_mood(ti(), "lexo", Mood::Imp, Some(ate()), true), Some("mos e lexo atë".to_string()));
        assert_eq!(clause_in_mood(ti(), "lexo", Mood::Imp, Some(ate()), false), Some("lexoje atë".to_string()));
        assert_eq!(clause_in_mood(ti(), "jep", Mood::Imp, Some(nenes()), false), Some("jepi nënës".to_string()));
        assert_eq!(clause_in_mood(ju(), "lexoni", Mood::Imp, Some(ate()), false), Some("lexojeni atë".to_string()));
        assert_eq!(clause_in_mood(ti(), "thuaj", Mood::Imp, Some(mua), false), Some("më thuaj mua".to_string()));
        assert_eq!(clause_in_mood(ti(), "lexosh", Mood::Sub, Some(ate()), false), None);
    }

    #[test]
    fn plural_dative_is_doubled_by_iu_before_the_nonactive_u() {
        let fused = vec![
            t(1, "Studentëve", "NOUN", "Case=Dat|Definite=Def|Gender=Masc|Number=Plur", 3, "iobj"),
            t(2, "iu", "PRON", "Case=Dat|Number=Plur|Person=3|PronType=Prs", 3, "iobj"),
            t(3, "dha", "VERB", "Mood=Ind|Number=Sing|Person=3|Tense=Past|Voice=Mid", 0, "root"),
            t(4, "mundësia", "NOUN", NOM_DEF_F_SG, 3, "nsubj:pass"),
        ];
        assert_eq!(verdicts(&fused, CLITIC_DOUBLING), vec![Verdict::Agree]);
        let split = vec![
            t(1, "Atyre", "PRON", "Case=Dat|Number=Plur|Person=3|PronType=Prs", 4, "iobj"),
            t(2, "i", "PRON", "Case=Dat|Number=Sing|Person=3|PronType=Prs", 4, "iobj"),
            t(3, "u", "PART", "", 4, "expl:pass"),
            t(4, "duk", "VERB", "Mood=Ind|Number=Sing|Person=3|Tense=Past|Voice=Mid", 0, "root"),
        ];
        assert_eq!(verdicts(&split, CLITIC_DOUBLING), vec![Verdict::Agree]);
    }

    #[test]
    fn disjunct_objects_and_fused_imperatives() {
        let either = vec![
            t(1, "Benin", "PROPN", ACC_DEF_M_SG, 5, "obj"),
            t(2, "ose", "CCONJ", "", 3, "cc"),
            t(3, "Anën", "PROPN", "Case=Acc|Definite=Def|Gender=Fem|Number=Sing", 1, "conj"),
            t(4, "e", "PRON", "Case=Acc|Number=Sing|Person=3|PronType=Prs", 5, "obj"),
            t(5, "takova", "VERB", IND_1SG, 0, "root"),
        ];
        assert_eq!(verdicts(&either, CLITIC_DOUBLING), vec![Verdict::Agree]);
        let fused = vec![
            t(1, "komshiun", "NOUN", ACC_DEF_M_SG, 2, "obj"),
            Token::new(2, "mbajeni", "mbaj", "VERB", "Mood=Ind|Number=Sing|Person=1|Tense=Pres", 0, "root"),
        ];
        assert_eq!(verdicts(&fused, CLITIC_DOUBLING), vec![Verdict::Hold]);
    }

    #[test]
    fn preposed_possessive_takes_an_indefinite_kinship_noun() {
        let im = |noun: &str, feats: &str| vec![t(1, "Im", "DET", "", 2, "det:poss"), t(2, noun, "NOUN", feats, 0, "root")];
        assert_eq!(verdicts(&im("vëlla", "Case=Nom|Definite=Ind|Gender=Masc|Number=Sing"), DETERMINER_NOUN), vec![Verdict::Agree]);
        assert_eq!(verdicts(&im("vëllai", NOM_DEF_M_SG), DETERMINER_NOUN), vec![Verdict::Hold]);
        let after = vec![t(1, "libri", "NOUN", "Case=Nom|Definite=Ind|Gender=Masc|Number=Sing", 0, "root"), t(2, "im", "DET", "", 1, "det:poss")];
        assert_eq!(verdicts(&after, DETERMINER_NOUN), vec![Verdict::Disagree]);
    }

    #[test]
    fn expletive_mos_after_fear_and_in_ne_mos() {
        let fear = vec![
            Token::new(1, "Druaj", "druaj", "VERB", "Mood=Ind|Number=Sing|Person=1|Tense=Pres", 0, "root"),
            t(2, "mos", "PART", "", 3, "advmod"),
            t(3, "vonohet", "VERB", IND_3SG, 1, "ccomp"),
        ];
        assert_eq!(verdicts(&fear, NEGATION), vec![Verdict::Hold]);
        let condition = vec![
            t(1, "Në", "ADP", "", 3, "mark"),
            t(2, "mos", "PART", "", 3, "advmod"),
            t(3, "gabohem", "VERB", "Mood=Ind|Number=Sing|Person=1|Tense=Pres", 5, "advcl"),
            t(4, ",", "PUNCT", "", 3, "punct"),
            t(5, "erdhi", "VERB", "Mood=Ind|Number=Sing|Person=3|Tense=Past", 0, "root"),
        ];
        assert_eq!(verdicts(&condition, NEGATION), vec![Verdict::Hold]);
    }

    #[test]
    fn subject_cases_beyond_plain_agreement() {
        let adjectives = vec![
            t(1, "e", "DET", "", 2, "det:adj"),
            t(2, "mira", "ADJ", "Gender=Fem|Number=Sing", 7, "nsubj"),
            t(3, "dhe", "CCONJ", "", 5, "cc"),
            t(4, "e", "DET", "", 5, "det:adj"),
            t(5, "keqja", "ADJ", "Gender=Fem|Number=Sing", 2, "conj"),
            t(7, "marrin", "VERB", IND_3PL, 0, "root"),
        ];
        assert_eq!(verdicts(&adjectives, SUBJECT_VERB), vec![Verdict::Agree]);
        let ka = |subject: &str, feats: &str, first: bool| {
            let (s, k) = match first {
                true => (1, 2),
                false => (2, 1),
            };
            vec![t(s, subject, "NOUN", feats, k, "nsubj"), Token::new(k, "ka", "kam", "VERB", IND_3SG, 0, "root")]
        };
        assert_eq!(verdicts(&ka("Djemtë", NOM_DEF_M_PL, true), SUBJECT_VERB), vec![Verdict::Disagree]);
        let people = "Case=Nom|Definite=Ind|Gender=Masc|Number=Plur";
        assert_eq!(verdicts(&ka("njerëz", people, false), SUBJECT_VERB), vec![Verdict::Hold]);
        let seems = vec![
            t(1, "Duket", "VERB", IND_3SG, 0, "root"),
            t(2, "njerëzit", "NOUN", NOM_DEF_M_PL, 1, "nsubj"),
            t(3, "kishin", "VERB", "Mood=Ind|Number=Plur|Person=3|Tense=Past", 1, "ccomp"),
        ];
        assert_eq!(verdicts(&seems, SUBJECT_VERB), vec![Verdict::Hold]);
        let most = vec![
            t(1, "Shumica", "NOUN", NOM_DEF_F_SG, 6, "nsubj"),
            t(2, "e", "DET", "", 3, "det"),
            t(3, "studentëve", "NOUN", "Case=Gen|Definite=Def|Gender=Masc|Number=Plur", 1, "nmod:poss"),
            t(4, "janë", "AUX", IND_3PL, 6, "cop"),
            t(5, "të", "DET", "", 6, "det:adj"),
            t(6, "kënaqur", "ADJ", "Gender=Masc|Number=Plur", 0, "root"),
        ];
        assert_eq!(verdicts(&most, SUBJECT_VERB), vec![Verdict::Hold]);
        assert_eq!(verdicts(&most, ADJECTIVE_ARTICLE), vec![Verdict::Hold]);
        let fronted = vec![
            t(1, "gishtërinjtë", "NOUN", NOM_DEF_M_PL, 3, "nsubj"),
            t(2, "i", "PRON", "Case=Acc|Number=Plur|Person=3|PronType=Prs", 3, "iobj"),
            Token::new(3, "ka", "kam", "VERB", IND_3SG, 0, "root"),
            t(4, "nipi", "NOUN", NOM_DEF_M_SG, 3, "obj"),
        ];
        assert_eq!(verdicts(&fronted, SUBJECT_VERB), vec![Verdict::Hold]);
    }

    #[test]
    fn noun_form_overrides_a_contradicting_annotation() {
        let joshjen = vec![
            Token::new(1, "joshjen", "joshje", "NOUN", "Case=Acc|Definite=Ind|Gender=Fem|Number=Sing", 0, "root"),
            t(2, "të", "DET", "", 3, "det:adj"),
            t(3, "madhe", "ADJ", "Gender=Fem|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&joshjen, ADJECTIVE_ARTICLE), vec![Verdict::Hold]);
        let ide = vec![
            t(1, "ide", "NOUN", NOM_DEF_F_SG, 0, "root"),
            t(2, "e", "DET", "", 3, "det:adj"),
            t(3, "mirë", "ADJ", "Gender=Fem|Number=Sing", 1, "amod"),
        ];
        assert_eq!(verdicts(&ide, ADJECTIVE_ARTICLE), vec![Verdict::Hold]);
        let gender = vec![
            t(1, "vajza", "NOUN", NOM_DEF_F_SG, 0, "root"),
            t(2, "e", "DET", "", 3, "det:adj"),
            t(3, "mirë", "ADJ", "Gender=Masc|Number=Plur", 1, "amod"),
        ];
        assert_eq!(verdicts(&gender, NOUN_ADJECTIVE), vec![Verdict::Disagree]);
    }

    #[test]
    fn article_chain_before_a_genitive_noun() {
        let chain = |g_upos: &str| {
            vec![
                t(1, "njerëz", "NOUN", "Case=Nom|Definite=Ind|Gender=Masc|Number=Plur", 0, "root"),
                t(2, "e", "DET", "", 4, "det"),
                t(3, "të", "DET", "", 4, "det"),
                t(4, "vendit", g_upos, "Case=Gen|Definite=Def|Gender=Masc|Number=Sing", 1, "nmod:poss"),
            ]
        };
        assert_eq!(verdicts(&chain("NOUN"), GENITIVE_ARTICLE), vec![Verdict::Hold]);
        assert_eq!(verdicts(&chain("ADJ"), GENITIVE_ARTICLE), vec![Verdict::Disagree]);
    }
}
