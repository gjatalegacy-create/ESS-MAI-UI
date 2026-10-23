//! Përemri pyetës dhe lidhor.
//!
//! «kush» nuk ka gjini as numër: kush, kujt, kë (kënd). «cili» lakohet si emër i shquar:
//! cili, cilit, cilin; cila, cilës, cilën; cilët, cilëve; cilat, cilave. Lidhori «i cili»
//! merr nyjën e përemrit pa emër: i cili, të cilin, të cilit, e cila, së cilës, të cilët.
//! «çfarë» (ç') dhe «që» nuk lakohen; «që» qëndron vetëm në emërore dhe kallëzore.

use super::{free_article, over_gender, slot, with, Slot};
use crate::gramatika::tipet::{Case, Form, Gender, Number};

const KUSH: &str = "P-K1 kush: tabela e mbyllur (kush, kujt, kë)";
const CILI: &str = "P-K2 cili: mbaresat e trajtës së shquar";
const I_CILI: &str = "P-K3 i cili: nyjë e përemrit pa emër + cili";
const WHAT: &str = "P-K4 çfarë: i pandryshueshëm, me trajtën e shkurtuar ç'";
const THAT: &str = "P-K5 që: lidhor i pandryshueshëm, vetëm emërore e kallëzore";

/// Pyetësi «kush»: emërore kush, kallëzore kë (kënd), rasat e zhdrejta kujt.
pub fn kush(case: Case) -> Option<Form> {
    match slot(case)? {
        Slot::Nom => Some(Form::one("kush", KUSH)),
        Slot::Acc => Some(with("kë", &["kënd"], KUSH)),
        Slot::Obl => Some(Form::one("kujt", KUSH)),
    }
}

/// Mbaresa e klasës «cili» dhe tema që e merr (0: e njëjësit, 1: e femërores shumës).
pub(crate) fn cili_suffix(gender: Gender, number: Number, s: Slot) -> Option<(usize, &'static str)> {
    let [nom, acc, obl] = match (gender, number) {
        (Gender::Masc, Number::Sing) => ["i", "in", "it"],
        (Gender::Fem, Number::Sing) => ["a", "ën", "ës"],
        (Gender::Masc, Number::Plur) => ["ët", "ët", "ëve"],
        (Gender::Fem, Number::Plur) => ["at", "at", "ave"],
        (Gender::Neut, _) => return None,
    };
    let stem = match (gender, number) {
        (Gender::Fem, Number::Plur) => 1,
        _ => 0,
    };
    Some((
        stem,
        match s {
            Slot::Nom => nom,
            Slot::Acc => acc,
            Slot::Obl => obl,
        },
    ))
}

/// Pyetësi «cili».
pub fn which(gender: Option<Gender>, number: Number, case: Case) -> Option<Form> {
    over_gender(gender, |g| {
        let (_, suffix) = cili_suffix(g, number, slot(case)?)?;
        Some(Form::one(format!("cil{suffix}"), CILI))
    })
}

/// Lidhori «i cili».
pub fn relative(gender: Option<Gender>, number: Number, case: Case) -> Option<Form> {
    over_gender(gender, |g| {
        let (_, suffix) = cili_suffix(g, number, slot(case)?)?;
        let a = free_article(g, number, case)?;
        Some(Form::one(format!("{} cil{suffix}", a.text), I_CILI))
    })
}

/// «çfarë»: e njëjta trajtë në çdo rasë.
pub(crate) fn what(case: Option<Case>) -> Option<Form> {
    match case {
        Some(Case::Voc) => None,
        _ => Some(with("çfarë", &["ç'"], WHAT)),
    }
}

/// «që»: lidhor i pandryshueshëm; në rasat e zhdrejta vjen «i cili».
pub(crate) fn that(case: Option<Case>) -> Option<Form> {
    match case {
        None | Some(Case::Nom) | Some(Case::Acc) => Some(Form::one("që", THAT)),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kush_table() {
        assert_eq!(kush(Case::Nom).map(|f| f.text).as_deref(), Some("kush"));
        assert_eq!(kush(Case::Dat).map(|f| f.text).as_deref(), Some("kujt"));
        let acc = kush(Case::Acc).unwrap();
        assert!(acc.accepts("kë") && acc.accepts("kënd"));
    }

    #[test]
    fn cili_and_i_cili() {
        let w = |g, n, c| which(Some(g), n, c).map(|f| f.text);
        assert_eq!(w(Gender::Masc, Number::Sing, Case::Acc).as_deref(), Some("cilin"));
        assert_eq!(w(Gender::Fem, Number::Sing, Case::Dat).as_deref(), Some("cilës"));
        assert_eq!(w(Gender::Masc, Number::Plur, Case::Abl).as_deref(), Some("cilëve"));
        assert_eq!(w(Gender::Fem, Number::Plur, Case::Nom).as_deref(), Some("cilat"));
        let r = |g, n, c| relative(Some(g), n, c).map(|f| f.text);
        assert_eq!(r(Gender::Masc, Number::Sing, Case::Nom).as_deref(), Some("i cili"));
        assert_eq!(r(Gender::Masc, Number::Sing, Case::Acc).as_deref(), Some("të cilin"));
        assert_eq!(r(Gender::Fem, Number::Sing, Case::Gen).as_deref(), Some("së cilës"));
        assert_eq!(r(Gender::Masc, Number::Plur, Case::Dat).as_deref(), Some("të cilëve"));
        assert_eq!(r(Gender::Fem, Number::Plur, Case::Nom).as_deref(), Some("të cilat"));
        assert_eq!(which(None, Number::Sing, Case::Nom), None);
    }

    #[test]
    fn invariants() {
        assert!(what(Some(Case::Dat)).unwrap().accepts("ç'"));
        assert_eq!(that(Some(Case::Dat)), None);
        assert_eq!(that(None).map(|f| f.text).as_deref(), Some("që"));
    }
}
