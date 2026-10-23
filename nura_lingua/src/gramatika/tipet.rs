//! Kategoritë gramatikore të shqipes standarde, të përbashkëta për të gjitha
//! modulet e gramatikës.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gender {
    Masc,
    Fem,
    Neut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Number {
    Sing,
    Plur,
}

/// Rasat. Gjinorja = nyjë e përparme + trajta e dhanores; rrjedhorja e
/// pashquar shumës ka edhe trajtën e vjetër me «-sh».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Case {
    Nom,
    Gen,
    Dat,
    Acc,
    Abl,
    Voc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Definite {
    Ind,
    Def,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Person {
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mood {
    Ind,
    Sub,
    Cnd,
    Adm,
    Opt,
    Imp,
}

/// Kohët: të thjeshta (Pres, Imperf, Aor) dhe të përbëra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tense {
    Pres,
    Imperf,
    Aor,
    Perf,
    Pluperf,
    PastAnterior,
    Fut,
    FutPerf,
    FutPast,
    FutPastPerf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Voice {
    Act,
    Mid,
}

/// Trajtat e pashtjelluara të foljes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NonFinite {
    Participle,
    Infinitive,
    Gerund,
    Privative,
    Absolutive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pos {
    Noun,
    Propn,
    Adj,
    Verb,
    Adv,
    Pron,
    Det,
    Num,
    Adp,
    Conj,
    Part,
    Intj,
    Other,
}

/// Një qelizë e lakimit të emrit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NounCell {
    pub case: Case,
    pub number: Number,
    pub definite: Definite,
}

/// Një qelizë e zgjedhimit të foljes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VerbCell {
    pub mood: Mood,
    pub tense: Tense,
    pub person: Person,
    pub number: Number,
    pub voice: Voice,
}

/// Përgjigjja e një rregulli: trajta(t) dhe dëshmia që i jep.
/// `None` në vend të `Form` do të thotë «nuk e di me siguri»: asnjë hamendje.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub text: String,
    /// Variante të pranuara të të njëjtës qelizë (p.sh. «-ve»/«-ave»).
    pub variants: Vec<String>,
    pub rule: &'static str,
}

impl Form {
    pub fn one(text: impl Into<String>, rule: &'static str) -> Form {
        Form { text: text.into(), variants: Vec::new(), rule }
    }

    pub fn accepts(&self, s: &str) -> bool {
        self.text == s || self.variants.iter().any(|v| v == s)
    }
}

pub fn ud_case(s: &str) -> Option<Case> {
    match s {
        "Nom" => Some(Case::Nom),
        "Gen" => Some(Case::Gen),
        "Dat" => Some(Case::Dat),
        "Acc" => Some(Case::Acc),
        "Abl" => Some(Case::Abl),
        "Voc" => Some(Case::Voc),
        _ => None,
    }
}

pub fn ud_gender(s: &str) -> Option<Gender> {
    match s {
        "Masc" => Some(Gender::Masc),
        "Fem" => Some(Gender::Fem),
        "Neut" => Some(Gender::Neut),
        _ => None,
    }
}

pub fn ud_number(s: &str) -> Option<Number> {
    match s {
        "Sing" => Some(Number::Sing),
        "Plur" => Some(Number::Plur),
        _ => None,
    }
}

/// Vlera e një tipari në vargun UD «A=b|C=d».
pub fn ud_feat<'a>(feats: &'a str, key: &str) -> Option<&'a str> {
    feats.split('|').find_map(|kv| kv.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
}

/// Ndarja e qëndrueshme zhvillim/provë sipas lemës (FNV-1a, 32 bit):
/// rregullat ndërtohen duke parë vetëm gjysmën `Dev`; saktësia raportohet në `Test`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Dev,
    Test,
}

pub fn split_of(lemma: &str) -> Split {
    let h = lemma.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
    match h & 1 {
        0 => Split::Dev,
        _ => Split::Test,
    }
}
