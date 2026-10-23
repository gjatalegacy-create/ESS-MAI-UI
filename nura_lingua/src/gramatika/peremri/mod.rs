//! Përemri i shqipes standarde: vetor, vetvetor, dëftor, pronor, pyetës,
//! lidhor, i pacaktuar dhe nyja e përparme. Klasë e mbyllur: tabelat e plota
//! janë vetë shkenca e saj.
//!
//! Burimet: Gramatika e gjuhës shqipe I (ASHSH 2002), kreu i përemrit; klasat e
//! paradigmës dhe mbaresat nga uniparser-grammar-albanian 2.3.0, MIT.
//!
//! Rendi i dëshmisë, i fiksuar:
//! 1. tabela e mbyllur e lemës (vetorët, dëftorët, pronorët, kush, cili, i cili,
//!    vetja, disa, të gjithë, çfarë, që);
//! 2. klasa e paradigmës së lemës në leksikun e gramatikës (interrog-pro,
//!    dem-pro-cili, cilido, recp-indef-pro, indef-pro-tjetër, dem-pro-tillë, det-pro);
//! 3. lema e pandryshueshme sipas leksikut, vetëm në emërore dhe kallëzore.
//!
//! Kur mungon një tipar, përgjigjja jepet vetëm kur të gjitha vlerat e tij japin
//! të njëjtën trajtë; përndryshe kthehet None.

pub mod demonstrative;
pub mod indefinite;
pub mod interrogative;
pub mod personal;
pub mod possessive;
pub mod reflexive;

pub use demonstrative::{demonstrative, Deixis};
pub use indefinite::{all_of, by_class, disa, invariant};
pub use interrogative::{kush, relative, which};
pub use personal::{
    clitic, clitic_cluster, clitic_doubling, clitic_with_u, cluster, doubling_clitic, doubling_for, personal,
    prepositional, Doubling, Object, Role, Verdict,
};
pub use possessive::{
    article, free_article, kin_noun, own, own_substantive, possessive, possessive_kinship, possessive_substantive,
    predicative_article,
};
pub use reflexive::reflexive;

use crate::gramatika::leksiku::Lexicon;
use crate::gramatika::tipet::{Case, Definite, Form, Gender, Number, Person};

/// Tiparet e kërkuara për një trajtë. Ato që mungojnë merren nga lema vetëm kur lema
/// është vetë ajo qelizë (ai, kjo, këta, cila); përndryshe mbeten të hapura.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Query {
    pub gender: Option<Gender>,
    pub number: Option<Number>,
    pub case: Option<Case>,
    /// Te pronorët e gjymtyrës: shquarja e emrit prijës; te të tjerët: shquarja e vetë përemrit.
    pub definite: Option<Definite>,
}

/// Vendi i rasës në tabelat e përemrit: gjinorja, dhanorja dhe rrjedhorja ndajnë një trajtë.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Nom,
    Acc,
    Obl,
}

pub(crate) fn slot(case: Case) -> Option<Slot> {
    match case {
        Case::Nom => Some(Slot::Nom),
        Case::Acc => Some(Slot::Acc),
        Case::Gen | Case::Dat | Case::Abl => Some(Slot::Obl),
        Case::Voc => None,
    }
}

/// Rreshti [emërore, kallëzore, e zhdrejtë] i një tabele.
pub(crate) fn pick(row: [&'static str; 3], s: Slot) -> &'static str {
    let [nom, acc, obl] = row;
    match s {
        Slot::Nom => nom,
        Slot::Acc => acc,
        Slot::Obl => obl,
    }
}

pub(crate) fn with(text: impl Into<String>, variants: &[&str], rule: &'static str) -> Form {
    Form { text: text.into(), variants: variants.iter().map(|v| v.to_string()).collect(), rule }
}

/// Dy rrugë që duhet të japin të njëjtën trajtë; mbeten vetëm variantet e përbashkëta.
pub(crate) fn agree(a: Option<Form>, b: Option<Form>) -> Option<Form> {
    let (a, b) = (a?, b?);
    (a.text == b.text).then(|| Form {
        variants: a.variants.iter().filter(|v| b.variants.contains(v)).cloned().collect(),
        text: a.text,
        rule: a.rule,
    })
}

pub(crate) fn over_gender(g: Option<Gender>, f: impl Fn(Gender) -> Option<Form>) -> Option<Form> {
    match g {
        Some(g) => f(g),
        None => agree(f(Gender::Masc), f(Gender::Fem)),
    }
}

pub(crate) fn over_number(n: Option<Number>, f: impl Fn(Number) -> Option<Form>) -> Option<Form> {
    match n {
        Some(n) => f(n),
        None => agree(f(Number::Sing), f(Number::Plur)),
    }
}

pub(crate) fn over_definite(d: Option<Definite>, f: impl Fn(Definite) -> Option<Form>) -> Option<Form> {
    match d {
        Some(d) => f(d),
        None => agree(f(Definite::Def), f(Definite::Ind)),
    }
}

/// Pronari sipas lemës së përemrit pronor: veta, numri dhe, në vetën e tretë njëjës, gjinia.
pub fn owner_of(lemma: &str) -> Option<(Person, Number, Option<Gender>)> {
    match lemma {
        "im" | "ime" | "imi" | "imja" => Some((Person::P1, Number::Sing, None)),
        "yt" | "jote" | "yti" | "jotja" => Some((Person::P2, Number::Sing, None)),
        "ynë" | "jonë" | "yni" | "jona" => Some((Person::P1, Number::Plur, None)),
        "juaj" | "juaji" | "juaja" => Some((Person::P2, Number::Plur, None)),
        "tij" | "i tij" | "tiji" | "i tiji" => Some((Person::P3, Number::Sing, Some(Gender::Masc))),
        "saj" | "i saj" | "saji" | "i saji" => Some((Person::P3, Number::Sing, Some(Gender::Fem))),
        "tyre" | "i tyre" | "tyri" | "i tyri" => Some((Person::P3, Number::Plur, None)),
        _ => None,
    }
}

/// Lemat e trajtës së shquar (të pavarur) të pronorit: imi, yti, i tiji ...
fn substantival(lemma: &str) -> bool {
    matches!(
        lemma,
        "imi" | "imja" | "yti" | "jotja" | "yni" | "jona" | "juaji" | "juaja" | "tiji" | "i tiji" | "saji" | "i saji"
            | "tyri" | "i tyri"
    )
}

/// Gjinia dhe numri i qelizës që është vetë lema (ai: m.nj., kjo: f.nj., këta: m.sh.).
/// Pronorët dhe «vet» nuk kanë qelizë të tillë: gjinia dhe numri i tyre janë
/// përshtatje me sendin e zotëruar dhe vijnë vetëm nga tiparet.
fn lemma_cell(lemma: &str) -> Option<(Option<Gender>, Number)> {
    match lemma {
        "ai" | "ky" | "cili" | "i cili" => Some((Some(Gender::Masc), Number::Sing)),
        "ajo" | "kjo" | "cila" | "e cila" => Some((Some(Gender::Fem), Number::Sing)),
        "ata" | "këta" => Some((Some(Gender::Masc), Number::Plur)),
        "ato" | "këto" => Some((Some(Gender::Fem), Number::Plur)),
        "disa" | "gjithë" | "të gjithë" => Some((None, Number::Plur)),
        "të gjitha" => Some((Some(Gender::Fem), Number::Plur)),
        _ => None,
    }
}

const CLOSED: &[&str] = &[
    "unë", "ti", "ne", "ju", "ai", "ajo", "ata", "ato", "ky", "kjo", "këta", "këto", "kush", "cili", "cila", "i cili",
    "e cila", "vete", "vetja", "vetvete", "vetvetja", "disa", "gjithë", "të gjithë", "të gjitha", "çfarë", "ç'", "që",
    "vet", "i vet", "veti", "i veti",
];

/// Lema me tabelë të mbyllur në këtë modul (jo nga klasa e leksikut).
pub fn is_closed(lemma: &str) -> bool {
    CLOSED.contains(&lemma) || owner_of(lemma).is_some()
}

/// Trajta e një lemë përemërore sipas tabelës së mbyllur.
pub fn closed(lemma: &str, q: &Query) -> Option<Form> {
    match lemma {
        "çfarë" => interrogative::what(q.case),
        "ç'" => interrogative::what(q.case).map(|f| with("ç'", &["çfarë"], f.rule)),
        "që" => interrogative::that(q.case),
        _ => closed_cased(lemma, q, q.case?),
    }
}

fn closed_cased(lemma: &str, q: &Query, case: Case) -> Option<Form> {
    let cell = lemma_cell(lemma);
    let number = q.number.or(cell.map(|(_, n)| n));
    // Gjinia e lemës vlen vetëm për numrin e vetë lemës: «ai» nuk thotë gjë për gjininë e shumësit.
    let gender = q.gender.or(cell.filter(|(_, n)| Some(*n) == number).and_then(|(g, _)| g));
    match lemma {
        "unë" => speech(Person::P1, Number::Sing, q, case),
        "ti" => speech(Person::P2, Number::Sing, q, case),
        "ne" => speech(Person::P1, Number::Plur, q, case),
        "ju" => speech(Person::P2, Number::Plur, q, case),
        "ai" | "ajo" | "ata" | "ato" => personal(Person::P3, number?, gender, case),
        "ky" | "kjo" | "këta" | "këto" => demonstrative(Deixis::Near, gender, number?, case),
        "kush" => (q.number != Some(Number::Plur)).then(|| kush(case)).flatten(),
        "cili" | "cila" => which(gender, number?, case),
        "i cili" | "e cila" => relative(gender, number?, case),
        "vetja" => reflexive(case, Definite::Def, false),
        "vetvetja" => reflexive(case, Definite::Def, true),
        "vete" => over_definite(q.definite, |d| reflexive(case, d, false)),
        "vetvete" => over_definite(q.definite, |d| reflexive(case, d, true)),
        "disa" => (number == Some(Number::Plur)).then(|| disa(case)).flatten(),
        "gjithë" | "të gjithë" | "të gjitha" => all_of(gender, number?, case),
        "vet" | "i vet" => over_number(q.number, |n| {
            over_gender(q.gender, |g| over_definite(q.definite, |d| own(g, n, case, d)))
        }),
        "veti" | "i veti" => over_number(q.number, |n| over_gender(q.gender, |g| own_substantive(g, n, case))),
        _ => owner_of(lemma).and_then(|(op, on, og)| match substantival(lemma) {
            true => over_number(q.number, |n| {
                over_gender(q.gender, |g| possessive_substantive(op, on, og, g, n, case))
            }),
            false => over_number(q.number, |n| {
                over_gender(q.gender, |g| over_definite(q.definite, |d| possessive(op, on, og, g, n, case, d)))
            }),
        }),
    }
}

/// Veta I dhe II: numri është pjesë e lemës; tipari kundërshtues jep None.
fn speech(person: Person, number: Number, q: &Query, case: Case) -> Option<Form> {
    match q.number.map_or(true, |n| n == number) {
        true => personal(person, number, None, case),
        false => None,
    }
}

/// Trajta e një lemë përemërore sipas rendit të dëshmisë: tabela e mbyllur,
/// klasa e leksikut, lema e pandryshueshme.
pub fn inflect(lex: &Lexicon, lemma: &str, q: &Query) -> Option<Form> {
    match is_closed(lemma) {
        true => closed(lemma, q),
        false => by_class(lex, lemma, q).or_else(|| invariant(lex, lemma, q.case)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(gender: Option<Gender>, number: Option<Number>, case: Case) -> Query {
        Query { gender, number, case: Some(case), definite: None }
    }

    #[test]
    fn closed_tables_answer_by_lemma() {
        let t = |l: &str, qq: Query| closed(l, &qq).map(|f| f.text);
        assert_eq!(t("unë", q(None, None, Case::Abl)).as_deref(), Some("meje"));
        assert_eq!(t("ai", q(Some(Gender::Fem), Some(Number::Plur), Case::Dat)).as_deref(), Some("atyre"));
        assert_eq!(t("kjo", q(None, None, Case::Dat)).as_deref(), Some("kësaj"));
        assert_eq!(t("cili", q(Some(Gender::Fem), Some(Number::Plur), Case::Dat)).as_deref(), Some("cilave"));
        assert_eq!(t("e cila", q(None, None, Case::Gen)).as_deref(), Some("së cilës"));
        assert_eq!(t("kush", q(None, None, Case::Acc)).as_deref(), Some("kë"));
        assert_eq!(t("disa", q(None, None, Case::Dat)).as_deref(), Some("disave"));
        assert_eq!(t("çfarë", Query::default()).as_deref(), Some("çfarë"));
    }

    #[test]
    fn missing_feature_answers_only_when_forms_coincide() {
        assert_eq!(closed("ai", &q(None, None, Case::Acc)).map(|f| f.text).as_deref(), Some("atë"));
        assert_eq!(over_gender(None, |g| demonstrative(Deixis::Near, Some(g), Number::Sing, Case::Dat)), None);
        assert_eq!(closed("im", &q(Some(Gender::Masc), None, Case::Nom)), None);
    }

    #[test]
    fn contradicting_number_is_held() {
        assert_eq!(closed("unë", &q(None, Some(Number::Plur), Case::Nom)), None);
        assert_eq!(closed("kush", &q(None, Some(Number::Plur), Case::Nom)), None);
    }

    fn qd(gender: Option<Gender>, number: Option<Number>, case: Case) -> Query {
        Query { gender, number, case: Some(case), definite: Some(Definite::Def) }
    }

    #[test]
    fn possessed_number_comes_only_from_the_query() {
        let t = |l: &str, qq: Query| closed(l, &qq).map(|f| f.text);
        let f = Some(Gender::Fem);
        let m = Some(Gender::Masc);
        assert_eq!(t("saj", qd(f, None, Case::Dat)), None);
        assert_eq!(t("im", qd(f, None, Case::Dat)), None);
        assert_eq!(t("tyre", qd(m, None, Case::Nom)), None);
        assert_eq!(t("vet", qd(m, None, Case::Nom)), None);
        assert_eq!(t("saj", qd(f, None, Case::Acc)).as_deref(), Some("e saj"));
        assert_eq!(t("im", qd(f, Some(Number::Plur), Case::Dat)).as_deref(), Some("të mia"));
        assert_eq!(t("imi", qd(None, Some(Number::Sing), Case::Nom)), None);
        assert_eq!(t("imi", qd(m, Some(Number::Sing), Case::Acc)).as_deref(), Some("timin"));
    }

    #[test]
    fn lemma_gender_holds_only_for_the_lemma_number() {
        let t = |l: &str, qq: Query| closed(l, &qq).map(|f| f.text);
        let p = Some(Number::Plur);
        assert_eq!(t("ai", q(None, p, Case::Nom)), None);
        assert_eq!(t("ky", q(None, p, Case::Nom)), None);
        assert_eq!(t("cili", q(None, p, Case::Nom)), None);
        assert_eq!(t("i cili", q(None, p, Case::Nom)), None);
        assert_eq!(t("ai", q(None, p, Case::Dat)).as_deref(), Some("atyre"));
        assert_eq!(t("ata", q(None, None, Case::Nom)).as_deref(), Some("ata"));
        assert_eq!(t("ajo", q(None, Some(Number::Sing), Case::Nom)).as_deref(), Some("ajo"));
    }
}
