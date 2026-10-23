//! Përemri dëftor i afërsisë (ky, kjo, këta, këto) dhe i largësisë (ai, ajo, ata, ato).
//! Si përemër dhe si përcaktor para emrit ka të njëjtat trajta: «këtij djali», «kësaj vajze».
//! Gjinorja, dhanorja dhe rrjedhorja përkojnë; rrjedhorja e shumësit ka edhe trajtën
//! e vjetër me «-sh» (kësish, kësosh, asish, asosh).

use super::{over_gender, pick, slot, with};
use crate::gramatika::tipet::{Case, Form, Gender, Number};

const RULE: &str = "P-D1 përemër dëftor: tabela e mbyllur (ky/ai)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deixis {
    /// ky, kjo, këta, këto
    Near,
    /// ai, ajo, ata, ato
    Far,
}

type Row = ([&'static str; 3], &'static [&'static str]);

fn row(deixis: Deixis, gender: Gender, number: Number) -> Option<Row> {
    match (deixis, number, gender) {
        (_, _, Gender::Neut) => None,
        (Deixis::Near, Number::Sing, Gender::Masc) => Some((["ky", "këtë", "këtij"], &[])),
        (Deixis::Near, Number::Sing, Gender::Fem) => Some((["kjo", "këtë", "kësaj"], &[])),
        (Deixis::Near, Number::Plur, Gender::Masc) => Some((["këta", "këta", "këtyre"], &["kësish"])),
        (Deixis::Near, Number::Plur, Gender::Fem) => Some((["këto", "këto", "këtyre"], &["kësosh"])),
        (Deixis::Far, Number::Sing, Gender::Masc) => Some((["ai", "atë", "atij"], &[])),
        (Deixis::Far, Number::Sing, Gender::Fem) => Some((["ajo", "atë", "asaj"], &[])),
        (Deixis::Far, Number::Plur, Gender::Masc) => Some((["ata", "ata", "atyre"], &["asish"])),
        (Deixis::Far, Number::Plur, Gender::Fem) => Some((["ato", "ato", "atyre"], &["asosh"])),
    }
}

/// Trajta e dëftorit; pa gjini përgjigjet vetëm aty ku gjinitë përkojnë (këtë, këtyre).
pub fn demonstrative(deixis: Deixis, gender: Option<Gender>, number: Number, case: Case) -> Option<Form> {
    over_gender(gender, |g| {
        let (r, old) = row(deixis, g, number)?;
        let text = pick(r, slot(case)?);
        let olds = match case {
            Case::Abl => old,
            Case::Nom | Case::Acc | Case::Gen | Case::Dat | Case::Voc => &[],
        };
        Some(with(text, olds, RULE))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(d: Deixis, g: Option<Gender>, n: Number, c: Case) -> Option<String> {
        demonstrative(d, g, n, c).map(|f| f.text)
    }

    #[test]
    fn near_and_far() {
        assert_eq!(t(Deixis::Near, Some(Gender::Masc), Number::Sing, Case::Dat).as_deref(), Some("këtij"));
        assert_eq!(t(Deixis::Near, Some(Gender::Fem), Number::Sing, Case::Gen).as_deref(), Some("kësaj"));
        assert_eq!(t(Deixis::Near, None, Number::Sing, Case::Acc).as_deref(), Some("këtë"));
        assert_eq!(t(Deixis::Near, None, Number::Plur, Case::Dat).as_deref(), Some("këtyre"));
        assert_eq!(t(Deixis::Far, Some(Gender::Fem), Number::Sing, Case::Abl).as_deref(), Some("asaj"));
        assert_eq!(t(Deixis::Far, Some(Gender::Masc), Number::Plur, Case::Acc).as_deref(), Some("ata"));
        assert_eq!(t(Deixis::Near, None, Number::Plur, Case::Nom), None);
        assert_eq!(t(Deixis::Near, Some(Gender::Masc), Number::Sing, Case::Voc), None);
        let f = demonstrative(Deixis::Near, Some(Gender::Fem), Number::Plur, Case::Abl).unwrap();
        assert!(f.accepts("këtyre") && f.accepts("kësosh"));
    }
}
