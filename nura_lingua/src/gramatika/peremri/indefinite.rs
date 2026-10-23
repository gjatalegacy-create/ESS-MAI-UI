//! Përemrat e pacaktuar dhe të prejardhurit e pyetësve.
//!
//! Tabelë e mbyllur: «disa» (shumës: disa, disave; si përcaktor para emrit nuk lakohet:
//! «u tregova disa shokëve») dhe «të gjithë» (të gjithë, të gjithëve; të gjitha, të gjithave;
//! si përcaktor para emrit nuk lakohet: «të gjithë njerëzve»).
//!
//! Të tjerët lakohen sipas klasës së paradigmës që u jep leksiku i gramatikës
//! (uniparser-grammar-albanian 2.3.0, MIT), me temat e lemës:
//! - interrog-pro: dikush, dikujt, dikë; askush, askujt, askënd; kushdo, kujtdo, këdo
//!   (kallëzorja vetëm me temat e leksikut; vetëm njëjës); «sa»: sa, save (shumës; si
//!   përcaktor para emrit nuk lakohet: «sa njerëzve»);
//! - dem-pro-cili: secili, secilit, secilin; secila, secilës; secilët; secilat;
//! - cilido: pjesëza «-do» pas trajtës së lakuar: cilitdo, cilëndo, cilëtdo;
//! - recp-indef-pro: njëri, njërit, njërin; njëra, njërës, njërën;
//! - indef-pro-tjetër: tjetër, tjetri, tjetrin, tjetrës; të tjerë, tjerët, tjerëve; tjera, tjerat;
//! - dem-pro-tillë: tillë, tilli, tillin; tilla, tillës; tillët, tillëve; tillat;
//! - det-pro: asgjë, asgjëje; diçka, diçkaje (vetëm njëjës).
//!
//! Lema e pandryshueshme e leksikut (asnjë, çdo, ndonjë, kaq) jepet vetëm në emërore
//! e kallëzore: si përemër në rasat e zhdrejta ajo zëvendësohet (asnjërit, çdonjërit).
//! Kur e njëjta leksemë ka edhe klasë lakimi edhe «unchangeable» (tërë), përgjigjja jepet
//! vetëm aty ku të dyja japin lemën: «tërë ditën», «gjatë tërë ditës»; ndryshe None.

use super::interrogative::cili_suffix;
use super::{over_definite, over_gender, slot, with, Query, Slot};
use crate::gramatika::leksiku::{Lexeme, Lexicon};
use crate::gramatika::tipet::{Case, Definite, Form, Gender, Number};

const DISA: &str = "P-T1 disa: shumës, rasat e zhdrejta «disave», si përcaktor i palakuar";
const ALL: &str = "P-T2 të gjithë: si përemër lakohet, para emrit mbetet i palakuar";
const INTERROG: &str = "P-L1 klasa interrog-pro: temat e emërores, e rasave të zhdrejta dhe e kallëzores";
const HOW_MANY: &str = "P-L9 «sa»: rasat e zhdrejta «save» në shumës, si përcaktor i palakuar";
const CILI: &str = "P-L2 klasa dem-pro-cili: mbaresat e trajtës së shquar";
const DO: &str = "P-L3 klasa cilido: pjesëza «-do» pas trajtës së lakuar";
const RECP: &str = "P-L4 klasa recp-indef-pro: mbaresat e emrit të shquar njëjës";
const OTHER: &str = "P-L5 klasa indef-pro-tjetër";
const SUCH: &str = "P-L6 klasa dem-pro-tillë";
const NOTHING: &str = "P-L7 klasa det-pro: vetëm njëjës; emërore = kallëzore, rasat e zhdrejta «-je»";
const QOFTE: &str = "P-L8 pjesëza «qoftë» pas trajtës me «-do»";
const INVARIANT: &str = "P-I1 përemër i pandryshueshëm sipas leksikut";

const CLASSES: &[&str] = &[
    "interrog-pro",
    "dem-pro-cili",
    "cilido",
    "recp-indef-pro-m-1",
    "recp-indef-pro-f-1",
    "recp-indef-pro-m-2",
    "recp-indef-pro-f-2",
    "indef-pro-tjetër",
    "dem-pro-tillë",
    "det-pro",
];

/// «disa»: emërore e kallëzore «disa», rasat e zhdrejta «disave» si përemër; si përcaktor
/// para emrit mbetet «disa» («disa shokëve»), që jepet si variant.
pub fn disa(case: Case) -> Option<Form> {
    match slot(case)? {
        Slot::Nom | Slot::Acc => Some(Form::one("disa", DISA)),
        Slot::Obl => Some(with("disave", &["disa"], DISA)),
    }
}

/// «të gjithë / të gjitha» (vetëm shumës). Në rasat e zhdrejta trajta e përemrit
/// (të gjithëve) ka si variant trajtën e përcaktorit para emrit (të gjithë njerëzve).
pub fn all_of(gender: Option<Gender>, number: Number, case: Case) -> Option<Form> {
    over_gender(gender, |g| {
        let (base, obl) = match (number, g) {
            (Number::Sing, _) | (_, Gender::Neut) => return None,
            (Number::Plur, Gender::Masc) => ("të gjithë", "të gjithëve"),
            (Number::Plur, Gender::Fem) => ("të gjitha", "të gjithave"),
        };
        match slot(case)? {
            Slot::Nom | Slot::Acc => Some(Form::one(base, ALL)),
            Slot::Obl => Some(with(obl, &[base], ALL)),
        }
    })
}

fn pronouns<'a>(lex: &'a Lexicon, lemma: &str) -> impl Iterator<Item = &'a Lexeme> {
    lex.entries(lemma).iter().filter(|l| l.standard && l.flags.iter().any(|f| f == "PRO"))
}

/// Trajta sipas klasës së paradigmës së lemës në leksik; lema me «-doqoftë» që mungon
/// në leksik lakohet si lema me «-do» dhe merr «qoftë» në fund. Leksema që ka edhe
/// «unchangeable» përgjigjet vetëm kur klasa e lakimit jep vetë lemën.
pub fn by_class(lex: &Lexicon, lemma: &str, q: &Query) -> Option<Form> {
    pronouns(lex, lemma)
        .find_map(|lx| {
            let fixed = lx.paradigms.iter().any(|p| p == "unchangeable");
            lx.paradigms.iter().find_map(|p| class_form(lx, p, q)).filter(|f| !fixed || f.text == lemma)
        })
        .or_else(|| {
            let base = lemma.strip_suffix("qoftë").filter(|b| b.ends_with("do"))?;
            pronouns(lex, lemma).next().is_none().then_some(())?;
            let f = by_class(lex, base, q)?;
            Some(Form {
                text: format!("{}qoftë", f.text),
                variants: f.variants.iter().map(|v| format!("{v}qoftë")).collect(),
                rule: QOFTE,
            })
        })
}

/// Lema e pandryshueshme: vetëm kur asnjë leksemë standarde e saj nuk ka klasë lakimi.
pub fn invariant(lex: &Lexicon, lemma: &str, case: Option<Case>) -> Option<Form> {
    let all: Vec<&Lexeme> = pronouns(lex, lemma).collect();
    let declinable = all.iter().any(|l| l.paradigms.iter().any(|p| CLASSES.contains(&p.as_str())));
    let fixed = all.iter().any(|l| !l.paradigms.is_empty() && l.paradigms.iter().all(|p| p == "unchangeable"));
    let direct = matches!(case, None | Some(Case::Nom) | Some(Case::Acc));
    (fixed && !declinable && direct).then(|| Form::one(lemma, INVARIANT))
}

fn class_form(lx: &Lexeme, par: &str, q: &Query) -> Option<Form> {
    let st = lx.clean_stems();
    match par {
        "interrog-pro" => interrog(&st, q),
        "dem-pro-cili" => cili_class(&st, q),
        "cilido" => cilido(&st, q),
        "recp-indef-pro-m-1" => recp(&st, q, Gender::Masc, ["u", "un", "ut"]),
        "recp-indef-pro-f-1" => recp(&st, q, Gender::Fem, ["ja", "en", "es"]),
        "recp-indef-pro-m-2" => recp(&st, q, Gender::Masc, ["i", "in", "it"]),
        "recp-indef-pro-f-2" => recp(&st, q, Gender::Fem, ["a", "ën", "ës"]),
        "indef-pro-tjetër" => adjectival(&st, q, Shape::Other),
        "dem-pro-tillë" => adjectival(&st, q, Shape::Such),
        "det-pro" => det_pro(&st, q),
        _ => None,
    }
}

/// Tema e i-të me variantet e saj, jo bosh.
fn stem(st: &[Vec<String>], i: usize) -> Option<&[String]> {
    st.get(i).map(Vec::as_slice).filter(|v| !v.is_empty())
}

/// Tema + mbaresë, me variantet e temës si variante të trajtës.
fn suffixed(v: &[String], suffix: &str, rule: &'static str) -> Option<Form> {
    let (first, rest) = v.split_first()?;
    Some(Form {
        text: format!("{first}{suffix}"),
        variants: rest.iter().map(|r| format!("{r}{suffix}")).collect(),
        rule,
    })
}

/// Kallëzorja merr vetëm temat që jep leksiku (dikë; çdokë, çdokënd): asnjë «-kënd»
/// e shtuar me rregull, sepse trajta si «askë» ose «tjetërkë» nuk janë standarde.
/// «kush» dhe të prejardhurit e tij janë vetëm njëjës. «sa» në rasat e zhdrejta është
/// shumës («save»); me emër njëjës (sasi: «sa bukë») është përcaktor i palakuar: None.
fn interrog(st: &[Vec<String>], q: &Query) -> Option<Form> {
    let s = slot(q.case?)?;
    let i = match s {
        Slot::Nom => 0,
        Slot::Obl => 1,
        Slot::Acc => 2,
    };
    let v = stem(st, i)?;
    let kush = stem(st, 0)?.iter().any(|x| x.contains("kush"));
    match (kush, q.number, s) {
        (true, Some(Number::Plur), _) => None,
        (true, _, _) | (false, _, Slot::Nom | Slot::Acc) => suffixed(v, "", INTERROG),
        (false, Some(Number::Plur), Slot::Obl) => {
            let mut f = suffixed(v, "", HOW_MANY)?;
            f.variants.extend(stem(st, 0)?.first().cloned());
            Some(f)
        }
        (false, _, Slot::Obl) => None,
    }
}

fn inherent_definite(q: &Query) -> Option<()> {
    (q.definite != Some(Definite::Ind)).then_some(())
}

fn cili_class(st: &[Vec<String>], q: &Query) -> Option<Form> {
    inherent_definite(q)?;
    let number = q.number?;
    let s = slot(q.case?)?;
    over_gender(q.gender, |g| {
        let (i, suffix) = cili_suffix(g, number, s)?;
        let v = stem(st, i).or_else(|| stem(st, 0))?;
        suffixed(v, suffix, CILI)
    })
}

fn cilido(st: &[Vec<String>], q: &Query) -> Option<Form> {
    inherent_definite(q)?;
    let number = q.number?;
    let s = slot(q.case?)?;
    let (pre, post) = stem(st, 0)?.first()?.split_once('.')?;
    over_gender(q.gender, |g| {
        let (_, suffix) = cili_suffix(g, number, s)?;
        Some(Form::one(format!("{pre}{suffix}{post}"), DO))
    })
}

fn recp(st: &[Vec<String>], q: &Query, gender: Gender, endings: [&'static str; 3]) -> Option<Form> {
    inherent_definite(q)?;
    (q.number != Some(Number::Plur) && q.gender == Some(gender)).then_some(())?;
    let v = stem(st, 0)?;
    let ends_in_vowel = v.first()?.chars().last().map_or(true, |c| "aeiouyë".contains(c));
    (!ends_in_vowel).then_some(())?;
    suffixed(v, super::pick(endings, slot(q.case?)?), RECP)
}

#[derive(Clone, Copy)]
enum Shape {
    Other,
    Such,
}

/// Klasat mbiemërore «tjetër» dhe «tillë»: (indeksi i temës, mbaresa) për
/// [emërore, kallëzore, e zhdrejtë], sipas gjinisë, numrit dhe shquarjes.
fn adjectival_cells(shape: Shape, g: Gender, n: Number, d: Definite) -> Option<[(usize, &'static str); 3]> {
    let b = match shape {
        Shape::Other => 2,
        Shape::Such => 1,
    };
    match (g, n, d, shape) {
        (Gender::Neut, ..) => None,
        (Gender::Masc, Number::Sing, Definite::Ind, _) => Some([(0, ""), (0, ""), (b, "i")]),
        (Gender::Masc, Number::Sing, Definite::Def, _) => Some([(b, "i"), (b, "in"), (b, "it")]),
        (Gender::Fem, Number::Sing, Definite::Ind, _) => Some([(0, ""), (0, ""), (b, "e")]),
        (Gender::Fem, Number::Sing, Definite::Def, _) => Some([(b, "a"), (b, "ën"), (b, "ës")]),
        (Gender::Masc, Number::Plur, Definite::Ind, Shape::Other) => Some([(1, "ë"), (1, "ë"), (1, "ëve")]),
        (Gender::Masc, Number::Plur, Definite::Def, Shape::Other) => Some([(1, "ët"), (1, "ët"), (1, "ëve")]),
        (Gender::Masc, Number::Plur, Definite::Ind, Shape::Such) => Some([(0, ""), (0, ""), (0, "ve")]),
        (Gender::Masc, Number::Plur, Definite::Def, Shape::Such) => Some([(0, "t"), (0, "t"), (0, "ve")]),
        (Gender::Fem, Number::Plur, Definite::Ind, _) => Some([(1, "a"), (1, "a"), (1, "ave")]),
        (Gender::Fem, Number::Plur, Definite::Def, _) => Some([(1, "at"), (1, "at"), (1, "ave")]),
    }
}

/// Rrjedhorja e pashquar e shumësit ka edhe trajtën e vjetër me «-sh» (tjerësh, tillash).
fn old_ablative(shape: Shape, g: Gender) -> Option<(usize, &'static str)> {
    match (shape, g) {
        (Shape::Other, Gender::Masc) => Some((1, "ësh")),
        (Shape::Such, Gender::Masc) => Some((0, "sh")),
        (_, Gender::Fem) => Some((1, "ash")),
        (_, Gender::Neut) => None,
    }
}

fn adjectival(st: &[Vec<String>], q: &Query, shape: Shape) -> Option<Form> {
    let number = q.number?;
    let case = q.case?;
    let s = slot(case)?;
    let rule = match shape {
        Shape::Other => OTHER,
        Shape::Such => SUCH,
    };
    over_gender(q.gender, |g| {
        over_definite(q.definite, |d| {
            let [nom, acc, obl] = adjectival_cells(shape, g, number, d)?;
            let (i, suffix) = match s {
                Slot::Nom => nom,
                Slot::Acc => acc,
                Slot::Obl => obl,
            };
            let mut f = suffixed(stem(st, i)?, suffix, rule)?;
            let old = match (case, number, d) {
                (Case::Abl, Number::Plur, Definite::Ind) => old_ablative(shape, g),
                _ => None,
            };
            f.variants.extend(old.and_then(|(j, sx)| stem(st, j)?.first().map(|t| format!("{t}{sx}"))));
            // Pas emrit të pashquar njëjës mbetet i palakuar: «një vendi tjetër», «një njeriu të tillë».
            let attributive = match (number, d, s) {
                (Number::Sing, Definite::Ind, Slot::Obl) => stem(st, 0).and_then(|v| v.first().cloned()),
                _ => None,
            };
            f.variants.extend(attributive);
            Some(f)
        })
    })
}

/// «asgjë, diçka, gjithçka, kurrgjë» janë vetëm njëjës: shumësi nuk ekziston dhe jep None
/// («disa», i vetmi me shumës, është te tabela e mbyllur).
fn det_pro(st: &[Vec<String>], q: &Query) -> Option<Form> {
    (q.definite != Some(Definite::Def) && q.number != Some(Number::Plur)).then_some(())?;
    let v = stem(st, 0)?;
    match slot(q.case?)? {
        Slot::Nom | Slot::Acc => suffixed(v, "", NOTHING),
        Slot::Obl => (q.number? == Number::Sing).then(|| suffixed(v, "je", NOTHING)).flatten(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gramatika::leksiku::Lexicon;

    const TSV: &str = "lex\tlemma\tupos\tpos\tgender\tanim\ttrans\tmed\tborr\tflags\tstatus\tparadigms\tstems\tlexref\tderiv\tdef_nom_sg\tdef_check\ttrans_en_bridge
dikush\tdikush\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tinterrog-pro\tdikush.|dikujt.|dikë.\t-\t-\t-\t-\tsomebody
askush\taskush\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tinterrog-pro\taskush.|askujt.|askënd.\t-\t-\t-\t-\tnobody
secili\tsecili\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tdem-pro-cili\tsecil.|secil.\t-\t-\t-\t-\teach
cilido\tcilido\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tcilido\tcil.do\t-\t-\t-\t-\tevery
kushdo\tkushdo\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tinterrog-pro\tkushdo.|kujtdo.|këdo.\t-\t-\t-\t-\twhoever
çdokush\tçdokush\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tinterrog-pro\tçdokush.|çdokujt.|çdokë.//çdokënd.\t-\t-\t-\t-\teveryone
diçka\tdiçka\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tdet-pro\tdiçka.\t-\t-\t-\t-\tsomething
njëri\tnjëri\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\trecp-indef-pro-m-2,recp-indef-pro-f-2\tnjër.\t-\t-\t-\t-\tone
mosnjëri\tmosnjëri\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\trecp-indef-pro-m-2,recp-indef-pro-f-2\tmosnjëri.\t-\t-\t-\t-\tnobody
tjetër\ttjetër\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb\tstd\tindef-pro-tjetër\ttjetër.|tjer.|tjetr.\t-\t-\t-\t-\tother
tillë\ttillë\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb\tstd\tdem-pro-tillë\ttillë.|till.\t-\t-\t-\t-\tsuch
asgjë\tasgjë\tPRO\t-\t-\t-\t-\t-\t-\tPRO,indef,alb,deriv\tstd\tdet-pro\tasgjë.\t-\t-\t-\t-\tnothing
asnjë\tasnjë\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb,deriv\tstd\tunchangeable\tasnjë.\t-\t-\t-\t-\tno
sa\tsa\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb\tstd\tinterrog-pro\tsa.|save.|sa.\t-\t-\t-\t-\thow many
tërë\ttërë\tPRO\t-\t-\t-\t-\t-\t-\tPRO,alb\tstd\tunchangeable,dem-pro-tillë\ttërë.|tër.\t-\t-\t-\t-\twhole";

    fn lex() -> Lexicon {
        Lexicon::parse(TSV)
    }

    fn q(g: Option<Gender>, n: Option<Number>, c: Case, d: Option<Definite>) -> Query {
        Query { gender: g, number: n, case: Some(c), definite: d }
    }

    fn t(l: &str, qq: Query) -> Option<String> {
        by_class(&lex(), l, &qq).map(|f| f.text)
    }

    #[test]
    fn kush_class() {
        assert_eq!(t("dikush", q(None, None, Case::Dat, None)).as_deref(), Some("dikujt"));
        assert_eq!(t("dikush", q(None, None, Case::Acc, None)).as_deref(), Some("dikë"));
        let f = by_class(&lex(), "askush", &q(None, None, Case::Acc, None)).unwrap();
        assert!(f.accepts("askënd") && !f.accepts("askë"));
        let f = by_class(&lex(), "çdokush", &q(None, None, Case::Acc, None)).unwrap();
        assert!(f.accepts("çdokë") && f.accepts("çdokënd"));
    }

    #[test]
    fn sa_is_plural_in_oblique_cases() {
        let p = Some(Number::Plur);
        let f = by_class(&lex(), "sa", &q(None, p, Case::Dat, None)).unwrap();
        assert_eq!((f.text.as_str(), f.rule), ("save", HOW_MANY));
        assert!(f.accepts("sa"));
        assert_eq!(t("sa", q(None, Some(Number::Sing), Case::Dat, None)), None);
        assert_eq!(t("sa", q(None, None, Case::Dat, None)), None);
        assert_eq!(t("sa", q(None, p, Case::Nom, None)).as_deref(), Some("sa"));
        assert_eq!(t("dikush", q(None, p, Case::Nom, None)), None);
    }

    #[test]
    fn unchangeable_and_declining_must_agree() {
        let m = Some(Gender::Masc);
        let s = Some(Number::Sing);
        assert_eq!(t("tërë", q(m, s, Case::Acc, Some(Definite::Ind))).as_deref(), Some("tërë"));
        assert_eq!(t("tërë", q(m, s, Case::Acc, Some(Definite::Def))), None);
        assert_eq!(t("tërë", q(m, s, Case::Dat, Some(Definite::Ind))), None);
        assert_eq!(t("tillë", q(m, s, Case::Acc, Some(Definite::Def))).as_deref(), Some("tillin"));
    }

    #[test]
    fn qofte_after_do() {
        let msg = q(Some(Gender::Masc), Some(Number::Sing), Case::Acc, None);
        let f = by_class(&lex(), "cilidoqoftë", &msg).unwrap();
        assert_eq!((f.text.as_str(), f.rule), ("cilindoqoftë", QOFTE));
        let fpl = q(Some(Gender::Fem), Some(Number::Plur), Case::Dat, None);
        assert_eq!(t("cilidoqoftë", fpl).as_deref(), Some("cilavedoqoftë"));
        assert_eq!(t("kushdoqoftë", q(None, None, Case::Dat, None)).as_deref(), Some("kujtdoqoftë"));
        assert_eq!(t("dikushqoftë", q(None, None, Case::Dat, None)), None);
    }

    #[test]
    fn cili_classes() {
        let fsg = q(Some(Gender::Fem), Some(Number::Sing), Case::Dat, None);
        assert_eq!(t("secili", fsg).as_deref(), Some("secilës"));
        let mpl = q(Some(Gender::Masc), Some(Number::Plur), Case::Nom, None);
        assert_eq!(t("cilido", mpl).as_deref(), Some("cilëtdo"));
        let msg = q(Some(Gender::Masc), Some(Number::Sing), Case::Acc, None);
        assert_eq!(t("cilido", msg).as_deref(), Some("cilindo"));
        assert_eq!(t("njëri", q(Some(Gender::Fem), None, Case::Acc, None)).as_deref(), Some("njërën"));
        assert_eq!(t("njëri", q(Some(Gender::Masc), None, Case::Dat, None)).as_deref(), Some("njërit"));
        assert_eq!(t("njëri", q(None, None, Case::Dat, None)), None);
        assert_eq!(t("mosnjëri", q(Some(Gender::Masc), None, Case::Dat, None)), None);
    }

    #[test]
    fn adjectival_classes() {
        let m = Some(Gender::Masc);
        let f = Some(Gender::Fem);
        let s = Some(Number::Sing);
        let p = Some(Number::Plur);
        let d = Some(Definite::Def);
        let i = Some(Definite::Ind);
        assert_eq!(t("tjetër", q(m, s, Case::Nom, i)).as_deref(), Some("tjetër"));
        assert_eq!(t("tjetër", q(m, s, Case::Dat, i)).as_deref(), Some("tjetri"));
        assert_eq!(t("tjetër", q(m, s, Case::Acc, d)).as_deref(), Some("tjetrin"));
        assert_eq!(t("tjetër", q(f, s, Case::Abl, d)).as_deref(), Some("tjetrës"));
        assert_eq!(t("tjetër", q(m, p, Case::Nom, i)).as_deref(), Some("tjerë"));
        assert_eq!(t("tjetër", q(m, p, Case::Nom, d)).as_deref(), Some("tjerët"));
        assert_eq!(t("tjetër", q(f, p, Case::Dat, d)).as_deref(), Some("tjerave"));
        assert_eq!(t("tjetër", q(m, p, Case::Dat, None)).as_deref(), Some("tjerëve"));
        assert_eq!(t("tjetër", q(m, s, Case::Nom, None)), None);
        assert_eq!(t("tillë", q(f, s, Case::Nom, d)).as_deref(), Some("tilla"));
        assert_eq!(t("tillë", q(m, p, Case::Dat, i)).as_deref(), Some("tillëve"));
        assert_eq!(t("tillë", q(f, p, Case::Nom, i)).as_deref(), Some("tilla"));
        let old = by_class(&lex(), "tjetër", &q(f, p, Case::Abl, i)).unwrap();
        assert!(old.accepts("tjerash"));
    }

    #[test]
    fn det_pro_and_invariant() {
        assert_eq!(t("asgjë", q(None, None, Case::Acc, None)).as_deref(), Some("asgjë"));
        assert_eq!(t("asgjë", q(None, Some(Number::Sing), Case::Dat, None)).as_deref(), Some("asgjëje"));
        assert_eq!(t("asgjë", q(None, None, Case::Dat, None)), None);
        assert_eq!(t("asgjë", q(None, Some(Number::Plur), Case::Dat, None)), None);
        assert_eq!(t("diçka", q(None, Some(Number::Plur), Case::Abl, None)), None);
        assert_eq!(t("asgjë", q(None, Some(Number::Plur), Case::Nom, None)), None);
        assert_eq!(invariant(&lex(), "asnjë", Some(Case::Nom)).map(|f| f.text).as_deref(), Some("asnjë"));
        assert_eq!(invariant(&lex(), "asnjë", Some(Case::Dat)), None);
        assert_eq!(invariant(&lex(), "tjetër", None), None);
    }

    #[test]
    fn closed_indefinites() {
        assert_eq!(disa(Case::Abl).map(|f| f.text).as_deref(), Some("disave"));
        assert!(disa(Case::Dat).unwrap().accepts("disa"));
        assert_eq!(disa(Case::Acc).unwrap().variants.len(), 0);
        let all = all_of(Some(Gender::Fem), Number::Plur, Case::Dat).unwrap();
        assert_eq!(all.text, "të gjithave");
        assert!(all.accepts("të gjitha"));
        assert_eq!(all_of(None, Number::Plur, Case::Nom), None);
    }
}
