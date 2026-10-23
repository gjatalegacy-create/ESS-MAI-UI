//! Përemri pronor dhe nyja e përparme.
//!
//! Pronori pas emrit përshtatet në gjini, numër dhe rasë me sendin e zotëruar:
//! «shoku im, shokun tim, shokut tim, shoqja ime, shoqes sime, shokët e mi, shokëve të mi».
//! Veta III është nyjë + tij/saj/tyre, ku nyja ndjek rregullin e nyjës së përparme:
//! «shoku i tij, shokun e tij, shoqes së tij, një shoku të tij».
//! Trajta e shquar qëndron pa emër: «imi, imja, të mitë, i tiji, e saja, të tyret».
//! Në shumës ajo merr gjithmonë nyjën «të», për çdo pronar: «të mitë, të tutë, të tanët,
//! të tonat, të tuajt, të tuajat, të tijtë, të tyret» («Si janë të tuajt?»).
//! Me tetë emrat e farefisnisë (atë, ëmë, vëlla, motër, bir, bijë, shoq, shoqe) pronori
//! i vetës I dhe II del para emrit: «im atë, ime motër, yt vëlla, jot motër, tët atë,
//! tyt eti, sat ëme»; në vetën III vjen nyja para emrit të shquar: «i ati, e ëma,
//! të atin, së ëmës».

use super::{pick, slot, with, Slot};
use crate::gramatika::tipet::{Case, Definite, Form, Gender, Number, Person};

const ARTICLE: &str = "P-N1 nyja e përparme: sipas rasës, numrit e gjinisë dhe sipas emrit të shquar ngjitur";
const FREE: &str = "P-N2 nyja e përparme e përemrit pa emër (i cili, i tiji, të tijtë, së cilës)";
const PREDICATIVE: &str = "P-N3 nyja e mbiemrit kallëzuesor: i/e/të te kryefjala, gjithmonë «të» te kundrina";
const ATTR: &str = "P-P1 përemër pronor i vetës I-II pas emrit të shquar";
const THIRD: &str = "P-P2 përemër pronor i vetës III: nyjë + tij/saj/tyre, edhe në f.sh. (tija/saja: nonst te uniparser)";
const SUBST: &str = "P-P3 përemër pronor në trajtë të shquar, pa emër";
const KIN: &str = "P-P4 përemër pronor para emrit të farefisnisë";
const KIN3: &str = "P-P5 veta III me emër farefisnie: nyjë + emër i shquar (i ati, e ëma)";
const OWN: &str = "P-P6 pronori vetvetor «i vet»: nyjë + vet/veta";

/// Nyja e përparme e mbiemrit si përcaktor (në grupin emëror), e gjinores dhe e
/// pronorit të vetës III. Emërore njëjës: i (m), e (f). Kallëzore dhe emërore shumës:
/// «e» pas emrit të shquar ngjitur, «të» në çdo rast tjetër («librin e ri», «librin tim
/// të ri», «një libër të ri»). Rasat e zhdrejta: «të», përveç femërores njëjës pas emrit
/// të shquar ngjitur: «së». Mbiemri kallëzuesor nuk është përcaktor, edhe kur del
/// menjëherë pas emrit: «e gjeta derën të hapur» (jo «derën e hapur»); për të vlen
/// `predicative_article`.
pub fn article(gender: Gender, number: Number, case: Case, head: Definite, adjacent: bool) -> Option<Form> {
    let close = head == Definite::Def && adjacent;
    let text = match (number, gender, slot(case)?, close) {
        (Number::Sing, Gender::Neut, _, _) => None,
        (Number::Sing, Gender::Masc, Slot::Nom, _) => Some("i"),
        (Number::Sing, Gender::Fem, Slot::Nom, _) => Some("e"),
        (_, _, Slot::Acc, true) | (Number::Plur, _, Slot::Nom, true) => Some("e"),
        (_, _, Slot::Acc, false) | (Number::Plur, _, Slot::Nom, false) => Some("të"),
        (Number::Sing, Gender::Fem, Slot::Obl, true) => Some("së"),
        (_, _, Slot::Obl, _) => Some("të"),
    }?;
    Some(Form::one(text, ARTICLE))
}

/// Nyja e mbiemrit kallëzuesor. I kryefjalës (emërore): i (m.nj.), e (f.nj.), të (shumës):
/// «djali është i mirë», «vajzat janë të mira». I kundrinës (kallëzore): gjithmonë «të»,
/// sido që të jetë emri: «e gjeta derën të hapur», «i bëri djemtë të lumtur».
/// Kallëzuesori nuk del në rasat e zhdrejta: None.
pub fn predicative_article(gender: Gender, number: Number, case: Case) -> Option<Form> {
    let text = match (number, gender, slot(case)?) {
        (_, _, Slot::Obl) | (Number::Sing, Gender::Neut, _) => None,
        (Number::Sing, Gender::Masc, Slot::Nom) => Some("i"),
        (Number::Sing, Gender::Fem, Slot::Nom) => Some("e"),
        (Number::Plur, _, Slot::Nom) | (_, _, Slot::Acc) => Some("të"),
    }?;
    Some(Form::one(text, PREDICATIVE))
}

/// Nyja e përemrit që qëndron pa emër: emërore i/e/të, kallëzore të,
/// rasat e zhdrejta të, femërorja njëjës së.
pub fn free_article(gender: Gender, number: Number, case: Case) -> Option<Form> {
    let text = match (number, gender, slot(case)?) {
        (Number::Sing, Gender::Neut, _) => None,
        (Number::Sing, Gender::Masc, Slot::Nom) => Some("i"),
        (Number::Sing, Gender::Fem, Slot::Nom) => Some("e"),
        (Number::Sing, Gender::Fem, Slot::Obl) => Some("së"),
        (Number::Plur, _, Slot::Nom) | (_, _, Slot::Acc) | (_, _, Slot::Obl) => Some("të"),
    }?;
    Some(Form::one(text, FREE))
}

fn cell(g: Gender, n: Number) -> Option<usize> {
    match (g, n) {
        (Gender::Masc, Number::Sing) => Some(0),
        (Gender::Fem, Number::Sing) => Some(1),
        (Gender::Masc, Number::Plur) => Some(2),
        (Gender::Fem, Number::Plur) => Some(3),
        (Gender::Neut, _) => None,
    }
}

type Table = [[&'static str; 3]; 4];

/// Pas emrit të shquar: [emërore, kallëzore, e zhdrejtë] për m.nj., f.nj., m.sh., f.sh.
static ATTR_1SG: Table = [["im", "tim", "tim"], ["ime", "time", "sime"], ["e mi", "e mi", "të mi"], ["e mia", "e mia", "të mia"]];
static ATTR_2SG: Table =
    [["yt", "tënd", "tënd"], ["jote", "tënde", "sate"], ["e tu", "e tu", "të tu"], ["e tua", "e tua", "të tua"]];
static ATTR_1PL: Table = [["ynë", "tonë", "tonë"], ["jonë", "tonë", "sonë"], ["tanë", "tanë", "tanë"], ["tona", "tona", "tona"]];
static ATTR_2PL: Table =
    [["juaj", "tuaj", "tuaj"], ["juaj", "tuaj", "suaj"], ["tuaj", "tuaj", "tuaj"], ["tuaja", "tuaja", "tuaja"]];

/// Trajta e shquar, pa emër.
static SUBST_1SG: Table = [
    ["imi", "timin", "timit"],
    ["imja", "timen", "simes"],
    ["të mitë", "të mitë", "të mive"],
    ["të miat", "të miat", "të miave"],
];
static SUBST_2SG: Table = [
    ["yti", "tëndin", "tëndit"],
    ["jotja", "tënden", "sates"],
    ["të tutë", "të tutë", "të tuve"],
    ["të tuat", "të tuat", "të tuave"],
];
static SUBST_1PL: Table = [
    ["yni", "tonin", "tonit"],
    ["jona", "tonën", "sonës"],
    ["të tanët", "të tanët", "të tanëve"],
    ["të tonat", "të tonat", "të tonave"],
];
static SUBST_2PL: Table = [
    ["juaji", "tuajin", "tuajit"],
    ["juaja", "tuajën", "suajës"],
    ["të tuajt", "të tuajt", "të tuajve"],
    ["të tuajat", "të tuajat", "të tuajave"],
];
static SUBST_TIJ: Table =
    [["tiji", "tijin", "tijit"], ["tija", "tijën", "tijës"], ["tijtë", "tijtë", "tijve"], ["tijat", "tijat", "tijave"]];
static SUBST_SAJ: Table =
    [["saji", "sajin", "sajit"], ["saja", "sajën", "sajës"], ["sajtë", "sajtë", "sajve"], ["sajat", "sajat", "sajave"]];
static SUBST_TYRE: Table =
    [["tyri", "tyrin", "tyrit"], ["tyrja", "tyren", "tyres"], ["tyret", "tyret", "tyreve"], ["tyret", "tyret", "tyreve"]];
static SUBST_VET: Table =
    [["veti", "vetin", "vetit"], ["veta", "vetën", "vetës"], ["vetët", "vetët", "vetëve"], ["vetat", "vetat", "vetave"]];

fn lookup(t: &Table, g: Gender, n: Number, case: Case) -> Option<&'static str> {
    let row = t.get(cell(g, n)?)?;
    Some(pick(*row, slot(case)?))
}

/// Pronari i vetës III: tij (m), saj (f), tyre (shumës).
fn third_stem(owner_number: Number, owner_gender: Option<Gender>) -> Option<&'static str> {
    match (owner_number, owner_gender) {
        (Number::Plur, _) => Some("tyre"),
        (Number::Sing, Some(Gender::Masc)) => Some("tij"),
        (Number::Sing, Some(Gender::Fem)) => Some("saj"),
        (Number::Sing, Some(Gender::Neut) | None) => None,
    }
}

/// Përemri pronor pas emrit. `definite` është shquarja e emrit prijës.
/// Pas emrit të pashquar, veta I dhe II ka dy ndërtime («një shok imi», «një shok i imi»)
/// dhe rregulli nuk zgjedh: None.
/// Veta III nuk ndryshon temën: «motrat e tij, e saj» (uniparser poss-pro-3-sg-m/f e shënon
/// «tija, saja» për f.sh. si josstandarde).
pub fn possessive(
    owner_person: Person,
    owner_number: Number,
    owner_gender: Option<Gender>,
    possessed_gender: Gender,
    possessed_number: Number,
    case: Case,
    definite: Definite,
) -> Option<Form> {
    match owner_person {
        Person::P3 => {
            let stem = third_stem(owner_number, owner_gender)?;
            let a = article(possessed_gender, possessed_number, case, definite, true)?;
            Some(Form::one(format!("{} {stem}", a.text), THIRD))
        }
        Person::P1 | Person::P2 => {
            // Varianti i femërores njëjës në rasat e zhdrejta sipas gramatikës: tënde, tuaj.
            let (t, alt): (&Table, &'static [&'static str]) = match (owner_person, owner_number, definite) {
                (_, _, Definite::Ind) => None,
                (Person::P1, Number::Sing, Definite::Def) => Some((&ATTR_1SG, &[][..])),
                (Person::P2, Number::Sing, Definite::Def) => Some((&ATTR_2SG, &["tënde"][..])),
                (Person::P1, Number::Plur, Definite::Def) => Some((&ATTR_1PL, &[][..])),
                (_, Number::Plur, Definite::Def) => Some((&ATTR_2PL, &["tuaj"][..])),
                (_, Number::Sing, Definite::Def) => None,
            }?;
            let text = lookup(t, possessed_gender, possessed_number, case)?;
            let variants = match (possessed_gender, possessed_number, slot(case)) {
                (Gender::Fem, Number::Sing, Some(Slot::Obl)) => alt,
                _ => &[],
            };
            Some(with(text, variants, ATTR))
        }
    }
}

/// Përemri pronor në trajtë të shquar, pa emër: «imi, timin, simes, të mitë, i tiji, së sajës».
pub fn possessive_substantive(
    owner_person: Person,
    owner_number: Number,
    owner_gender: Option<Gender>,
    possessed_gender: Gender,
    possessed_number: Number,
    case: Case,
) -> Option<Form> {
    let (t, third) = match (owner_person, owner_number) {
        (Person::P1, Number::Sing) => (&SUBST_1SG, false),
        (Person::P2, Number::Sing) => (&SUBST_2SG, false),
        (Person::P1, Number::Plur) => (&SUBST_1PL, false),
        (Person::P2, Number::Plur) => (&SUBST_2PL, false),
        (Person::P3, _) => match third_stem(owner_number, owner_gender)? {
            "tij" => (&SUBST_TIJ, true),
            "saj" => (&SUBST_SAJ, true),
            _ => (&SUBST_TYRE, true),
        },
    };
    let text = lookup(t, possessed_gender, possessed_number, case)?;
    match third {
        true => {
            let a = free_article(possessed_gender, possessed_number, case)?;
            Some(Form::one(format!("{} {text}", a.text), SUBST))
        }
        false => Some(Form::one(text, SUBST)),
    }
}

/// Gjinia e emrit të farefisnisë që e merr pronorin përpara. Ndërtimi është i ngulitur
/// vetëm për këta emra: atë (at), vëlla, bir, shoq; ëmë, motër, bijë, shoqe.
/// Çdo emër tjetër (dajë, xhaxha, nënë, gjysh) e merr pronorin pas: «daja im»; None.
pub fn kin_noun(kin: &str) -> Option<Gender> {
    match kin {
        "atë" | "at" | "vëlla" | "bir" | "shoq" => Some(Gender::Masc),
        "ëmë" | "motër" | "bijë" | "shoqe" => Some(Gender::Fem),
        _ => None,
    }
}

/// Pronori para emrit të farefisnisë (njëjës), vetëm për emrat e `kin_noun`.
/// Veta I-II: «im atë, tim atë, ime motër, sime motre, yt vëlla, tët vëlla, tyt eti,
/// jot motër, sat ëme». Veta III njëjës: kthehet nyja që i prin emrit të shquar
/// («i ati, të atin, e ëma, së ëmës»). Pronarët në shumës nuk e kanë këtë ndërtim: None.
pub fn possessive_kinship(owner_person: Person, owner_number: Number, kin: &str, case: Case) -> Option<Form> {
    let row = match (owner_person, owner_number, kin_noun(kin)?) {
        (_, Number::Plur, _) | (_, _, Gender::Neut) => None,
        (Person::P1, Number::Sing, Gender::Masc) => Some((["im", "tim", "tim"], KIN)),
        (Person::P1, Number::Sing, Gender::Fem) => Some((["ime", "time", "sime"], KIN)),
        (Person::P2, Number::Sing, Gender::Masc) => Some((["yt", "tët", "tyt"], KIN)),
        (Person::P2, Number::Sing, Gender::Fem) => Some((["jot", "tët", "sat"], KIN)),
        (Person::P3, Number::Sing, Gender::Masc) => Some((["i", "të", "të"], KIN3)),
        (Person::P3, Number::Sing, Gender::Fem) => Some((["e", "të", "së"], KIN3)),
    };
    let (r, rule) = row?;
    Some(Form::one(pick(r, slot(case)?), rule))
}

/// Pronori vetvetor pas emrit: «librin e vet», «shoqes së vet», «motrat e veta».
/// Ndryshe nga tij/saj, f.sh. «veta» është standarde (uniparser poss-pro-vet, f,pl).
pub fn own(possessed_gender: Gender, possessed_number: Number, case: Case, definite: Definite) -> Option<Form> {
    let a = article(possessed_gender, possessed_number, case, definite, true)?;
    let stem = match (possessed_gender, possessed_number) {
        (Gender::Fem, Number::Plur) => "veta",
        _ => "vet",
    };
    Some(Form::one(format!("{} {stem}", a.text), OWN))
}

/// Pronori vetvetor pa emër: «i veti, të vetin, së vetës, të vetët, të vetat».
pub fn own_substantive(possessed_gender: Gender, possessed_number: Number, case: Case) -> Option<Form> {
    let text = lookup(&SUBST_VET, possessed_gender, possessed_number, case)?;
    let a = free_article(possessed_gender, possessed_number, case)?;
    Some(Form::one(format!("{} {text}", a.text), OWN))
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Definite = Definite::Def;
    const I: Definite = Definite::Ind;
    const M: Gender = Gender::Masc;
    const F: Gender = Gender::Fem;
    const S: Number = Number::Sing;
    const P: Number = Number::Plur;

    fn art(g: Gender, n: Number, c: Case, d: Definite, adj: bool) -> String {
        article(g, n, c, d, adj).map(|f| f.text).unwrap_or_default()
    }

    #[test]
    fn article_agreement() {
        assert_eq!(art(M, S, Case::Nom, D, true), "i");
        assert_eq!(art(F, S, Case::Nom, I, true), "e");
        assert_eq!(art(M, S, Case::Acc, D, true), "e");
        assert_eq!(art(M, S, Case::Acc, D, false), "të");
        assert_eq!(art(F, S, Case::Acc, I, true), "të");
        assert_eq!(art(F, S, Case::Dat, D, true), "së");
        assert_eq!(art(F, S, Case::Dat, I, true), "të");
        assert_eq!(art(M, S, Case::Gen, D, true), "të");
        assert_eq!(art(M, P, Case::Nom, D, true), "e");
        assert_eq!(art(F, P, Case::Nom, I, true), "të");
        assert_eq!(art(F, P, Case::Abl, D, true), "të");
        assert_eq!(article(Gender::Neut, S, Case::Nom, D, true), None);
    }

    fn poss(p: Person, n: Number, og: Option<Gender>, g: Gender, pn: Number, c: Case, d: Definite) -> Option<String> {
        possessive(p, n, og, g, pn, c, d).map(|f| f.text)
    }

    #[test]
    fn attributive_first_second() {
        assert_eq!(poss(Person::P1, S, None, M, S, Case::Nom, D).as_deref(), Some("im"));
        assert_eq!(poss(Person::P1, S, None, F, S, Case::Dat, D).as_deref(), Some("sime"));
        assert_eq!(poss(Person::P1, S, None, M, P, Case::Nom, D).as_deref(), Some("e mi"));
        assert_eq!(poss(Person::P1, S, None, F, P, Case::Abl, D).as_deref(), Some("të mia"));
        assert_eq!(poss(Person::P2, S, None, M, S, Case::Acc, D).as_deref(), Some("tënd"));
        assert_eq!(poss(Person::P2, S, None, F, S, Case::Gen, D).as_deref(), Some("sate"));
        assert_eq!(poss(Person::P1, P, None, F, S, Case::Dat, D).as_deref(), Some("sonë"));
        assert_eq!(poss(Person::P1, P, None, M, P, Case::Acc, D).as_deref(), Some("tanë"));
        assert_eq!(poss(Person::P2, P, None, F, P, Case::Nom, D).as_deref(), Some("tuaja"));
        assert_eq!(poss(Person::P1, S, None, M, S, Case::Nom, I), None);
    }

    #[test]
    fn attributive_third() {
        let m = Some(M);
        assert_eq!(poss(Person::P3, S, m, M, S, Case::Nom, D).as_deref(), Some("i tij"));
        assert_eq!(poss(Person::P3, S, m, M, S, Case::Acc, D).as_deref(), Some("e tij"));
        assert_eq!(poss(Person::P3, S, Some(F), F, S, Case::Dat, D).as_deref(), Some("së saj"));
        assert_eq!(poss(Person::P3, S, Some(F), F, S, Case::Dat, I).as_deref(), Some("të saj"));
        assert_eq!(poss(Person::P3, P, None, M, P, Case::Nom, D).as_deref(), Some("e tyre"));
        assert_eq!(poss(Person::P3, P, None, M, P, Case::Nom, I).as_deref(), Some("të tyre"));
        assert_eq!(poss(Person::P3, S, None, M, S, Case::Nom, D), None);
    }

    #[test]
    fn substantive_and_kinship() {
        let s = |p, n, og, g, pn, c| possessive_substantive(p, n, og, g, pn, c).map(|f| f.text);
        assert_eq!(s(Person::P1, S, None, M, S, Case::Acc).as_deref(), Some("timin"));
        assert_eq!(s(Person::P1, S, None, M, P, Case::Nom).as_deref(), Some("të mitë"));
        assert_eq!(s(Person::P2, S, None, F, S, Case::Nom).as_deref(), Some("jotja"));
        assert_eq!(s(Person::P3, S, Some(M), M, S, Case::Nom).as_deref(), Some("i tiji"));
        assert_eq!(s(Person::P3, S, Some(F), F, S, Case::Dat).as_deref(), Some("së sajës"));
        assert_eq!(s(Person::P3, P, None, F, P, Case::Acc).as_deref(), Some("të tyret"));
        assert_eq!(s(Person::P1, P, None, M, P, Case::Nom).as_deref(), Some("të tanët"));
        assert_eq!(s(Person::P1, P, None, F, P, Case::Acc).as_deref(), Some("të tonat"));
        assert_eq!(s(Person::P2, P, None, M, P, Case::Dat).as_deref(), Some("të tuajve"));
        assert_eq!(s(Person::P2, P, None, F, P, Case::Nom).as_deref(), Some("të tuajat"));
        assert_eq!(s(Person::P1, P, None, M, S, Case::Nom).as_deref(), Some("yni"));
        let k = |p, kin, c| possessive_kinship(p, S, kin, c).map(|f| f.text);
        assert_eq!(k(Person::P1, "atë", Case::Nom).as_deref(), Some("im"));
        assert_eq!(k(Person::P1, "motër", Case::Dat).as_deref(), Some("sime"));
        assert_eq!(k(Person::P2, "motër", Case::Nom).as_deref(), Some("jot"));
        assert_eq!(k(Person::P2, "vëlla", Case::Acc).as_deref(), Some("tët"));
        assert_eq!(k(Person::P2, "atë", Case::Dat).as_deref(), Some("tyt"));
        assert_eq!(k(Person::P2, "ëmë", Case::Dat).as_deref(), Some("sat"));
        assert_eq!(k(Person::P3, "ëmë", Case::Dat).as_deref(), Some("së"));
        assert_eq!(k(Person::P3, "bir", Case::Nom).as_deref(), Some("i"));
        assert_eq!(possessive_kinship(Person::P1, P, "atë", Case::Nom), None);
        assert_eq!(k(Person::P1, "dajë", Case::Nom), None);
        assert_eq!(k(Person::P1, "xhaxha", Case::Nom), None);
        assert_eq!(kin_noun("shoqe"), Some(F));
    }

    #[test]
    fn predicative_is_not_attributive() {
        let p = |g, n, c| predicative_article(g, n, c).map(|f| f.text);
        assert_eq!(art(F, S, Case::Acc, D, true), "e");
        assert_eq!(p(F, S, Case::Acc).as_deref(), Some("të"));
        assert_eq!(p(M, P, Case::Acc).as_deref(), Some("të"));
        assert_eq!(p(M, S, Case::Nom).as_deref(), Some("i"));
        assert_eq!(p(F, S, Case::Nom).as_deref(), Some("e"));
        assert_eq!(p(F, P, Case::Nom).as_deref(), Some("të"));
        assert_eq!(p(F, S, Case::Dat), None);
    }

    #[test]
    fn reflexive_possessive() {
        assert_eq!(own(M, S, Case::Acc, D).map(|f| f.text).as_deref(), Some("e vet"));
        assert_eq!(own(F, S, Case::Dat, D).map(|f| f.text).as_deref(), Some("së vet"));
        assert_eq!(own(F, P, Case::Nom, D).map(|f| f.text).as_deref(), Some("e veta"));
        assert_eq!(own_substantive(M, P, Case::Nom).map(|f| f.text).as_deref(), Some("të vetët"));
        assert_eq!(own_substantive(F, S, Case::Acc).map(|f| f.text).as_deref(), Some("të vetën"));
    }
}
