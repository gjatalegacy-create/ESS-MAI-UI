//! Përemri vetvetor: vetja, vetes, veten (dhe vetvetja, vetvetes, vetveten).
//! Trajta e pashquar «vete» («për vete», «në vetvete») del vetëm në emërore e kallëzore;
//! rasat e zhdrejta të pashquara nuk janë të ngulitura në gjuhën standarde: None.

use super::{slot, Slot};
use crate::gramatika::tipet::{Case, Definite, Form};

const DEF: &str = "P-R1 përemër vetvetor i shquar: -ja, -en, -es";
const IND: &str = "P-R2 përemër vetvetor i pashquar pas parafjalës: vete, vetvete";

/// `compound` jep trajtën e përforcuar «vetvetja».
pub fn reflexive(case: Case, definite: Definite, compound: bool) -> Option<Form> {
    let stem = match compound {
        true => "vetvet",
        false => "vet",
    };
    match (definite, slot(case)?) {
        (Definite::Def, Slot::Nom) => Some(Form::one(format!("{stem}ja"), DEF)),
        (Definite::Def, Slot::Acc) => Some(Form::one(format!("{stem}en"), DEF)),
        (Definite::Def, Slot::Obl) => Some(Form::one(format!("{stem}es"), DEF)),
        (Definite::Ind, Slot::Nom | Slot::Acc) => Some(Form::one(format!("{stem}e"), IND)),
        (Definite::Ind, Slot::Obl) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vetja() {
        let r = |c, d, k| reflexive(c, d, k).map(|f| f.text);
        assert_eq!(r(Case::Nom, Definite::Def, false).as_deref(), Some("vetja"));
        assert_eq!(r(Case::Acc, Definite::Def, false).as_deref(), Some("veten"));
        assert_eq!(r(Case::Dat, Definite::Def, false).as_deref(), Some("vetes"));
        assert_eq!(r(Case::Abl, Definite::Def, true).as_deref(), Some("vetvetes"));
        assert_eq!(r(Case::Acc, Definite::Ind, true).as_deref(), Some("vetvete"));
        assert_eq!(r(Case::Dat, Definite::Ind, false), None);
    }
}
