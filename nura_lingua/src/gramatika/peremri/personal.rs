//! Përemri vetor: trajtat e plota (theksore) dhe të shkurtrat (klitikët),
//! grupet e klitikëve dhe rregulli i dyfishimit të kundrinës me klitik.
//!
//! Tabelat: unë, mua, meje; ti, ty, teje; ai, atij, atë; ajo, asaj, atë;
//! ne, neve, nesh; ju, juve, jush; ata, atyre, ata; ato, atyre, ato.
//! Klitikët: më, të, i (dhanore), e (kallëzore), na, ju, u (dhanore), i (kallëzore).
//! Veta I dhe II nuk kanë gjinore: në vend të saj vjen përemri pronor.

use super::{over_gender, with};
use crate::gramatika::tipet::{Case, Form, Gender, Number, Person};

const FULL: &str = "P-V1 përemër vetor: tabela e mbyllur e rasave";
const SHORT: &str = "P-V2 përemër vetor i vetës III: trajta e plotë dhe ajo pas parafjalës (me të, prej tij)";
const AFTER_PREP: &str = "P-V3 përemër vetor i vetës III pas parafjalës: trajta e shkurtër (me të, me ta, prej tij)";
const CLITIC: &str = "P-S1 trajtë e shkurtër e përemrit vetor (dhanore, kallëzore)";
const CLUSTER: &str = "P-S2 grup klitikësh: dhanorja para kallëzores së vetës III, të shkrira (ma, ta, ia, jua, ua)";
const MIDDLE: &str = "P-S3 dhanore e shkurtër + pjesëza «u» e trajtës joveprore (m'u, t'u, iu)";

/// Trajta e plotë e përemrit vetor. Veta III kërkon gjininë; kur mungon,
/// përgjigjet vetëm aty ku mashkullorja dhe femërorja përkojnë.
pub fn personal(person: Person, number: Number, gender: Option<Gender>, case: Case) -> Option<Form> {
    match person {
        Person::P3 => over_gender(gender, |g| third(number, g, case)),
        Person::P1 | Person::P2 => speech(person, number, case),
    }
}

fn speech(person: Person, number: Number, case: Case) -> Option<Form> {
    let [nom, dat, acc, abl] = match (person, number) {
        (Person::P1, Number::Sing) => ["unë", "mua", "mua", "meje"],
        (Person::P2, Number::Sing) => ["ti", "ty", "ty", "teje"],
        (Person::P1, Number::Plur) => ["ne", "neve", "ne", "nesh"],
        (Person::P2, Number::Plur) => ["ju", "juve", "ju", "jush"],
        (Person::P3, _) => return None,
    };
    let text = match case {
        Case::Nom => Some(nom),
        Case::Dat => Some(dat),
        Case::Acc => Some(acc),
        Case::Abl => Some(abl),
        Case::Gen | Case::Voc => None,
    }?;
    Some(Form::one(text, FULL))
}

struct Third {
    nom: &'static str,
    obl: &'static str,
    short: &'static str,
    acc: &'static str,
    acc_short: &'static str,
    old_abl: &'static [&'static str],
}

/// Veta III: gjinorja dhe rrjedhorja kanë edhe trajtën e shkurtër (i tij, prej saj),
/// kallëzorja pas parafjalës ka «të, ta, to» (me të, me ta, me to).
fn third(number: Number, gender: Gender, case: Case) -> Option<Form> {
    let t = match (number, gender) {
        (Number::Sing, Gender::Masc) => {
            Third { nom: "ai", obl: "atij", short: "tij", acc: "atë", acc_short: "të", old_abl: &[] }
        }
        (Number::Sing, Gender::Fem) => {
            Third { nom: "ajo", obl: "asaj", short: "saj", acc: "atë", acc_short: "të", old_abl: &[] }
        }
        (Number::Plur, Gender::Masc) => {
            Third { nom: "ata", obl: "atyre", short: "tyre", acc: "ata", acc_short: "ta", old_abl: &["asish"] }
        }
        (Number::Plur, Gender::Fem) => {
            Third { nom: "ato", obl: "atyre", short: "tyre", acc: "ato", acc_short: "to", old_abl: &["asosh"] }
        }
        (_, Gender::Neut) => return None,
    };
    match case {
        Case::Nom => Some(Form::one(t.nom, FULL)),
        Case::Dat => Some(Form::one(t.obl, FULL)),
        Case::Gen => Some(with(t.obl, &[t.short], SHORT)),
        Case::Abl => {
            let mut f = with(t.obl, &[t.short], SHORT);
            f.variants.extend(t.old_abl.iter().map(|v| v.to_string()));
            Some(f)
        }
        Case::Acc => Some(with(t.acc, &[t.acc_short], SHORT)),
        Case::Voc => None,
    }
}

/// Përemri vetor pas parafjalës. Veta III merr trajtën e shkurtër: kallëzore «me të,
/// me ta, me to», rrjedhore e gjinore «prej tij, prej saj, prej tyre»; emërorja mbetet
/// e plotë («nga ai»). Veta I dhe II ka trajtat e plota («me mua», «prej meje», «për ty»).
/// Asnjë parafjalë nuk merr dhanore: None.
pub fn prepositional(person: Person, number: Number, gender: Option<Gender>, case: Case) -> Option<Form> {
    match (person, case) {
        (_, Case::Dat | Case::Voc) => None,
        (Person::P1 | Person::P2, _) => speech(person, number, case),
        (Person::P3, Case::Nom) => personal(person, number, gender, case),
        (Person::P3, Case::Acc | Case::Gen | Case::Abl) => {
            let full = personal(person, number, gender, case)?;
            let short = full.variants.first()?.clone();
            let mut variants = vec![full.text];
            variants.extend(full.variants.into_iter().skip(1));
            Some(Form { text: short, variants, rule: AFTER_PREP })
        }
    }
}

/// Trajta e shkurtër (klitiku) në dhanore ose kallëzore.
pub fn clitic(person: Person, number: Number, case: Case) -> Option<Form> {
    let text = match (person, number, case) {
        (_, _, Case::Nom | Case::Gen | Case::Abl | Case::Voc) => None,
        (Person::P1, Number::Sing, _) => Some("më"),
        (Person::P2, Number::Sing, _) => Some("të"),
        (Person::P3, Number::Sing, Case::Dat) => Some("i"),
        (Person::P3, Number::Sing, _) => Some("e"),
        (Person::P1, Number::Plur, _) => Some("na"),
        (Person::P2, Number::Plur, _) => Some("ju"),
        (Person::P3, Number::Plur, Case::Dat) => Some("u"),
        (Person::P3, Number::Plur, _) => Some("i"),
    }?;
    Some(Form::one(text, CLITIC))
}

/// Shkrirja e dhanores me kallëzoren e vetës III: më+e → ma, më+i → m'i,
/// të+e → ta, të+i → t'i, i+e/i → ia, na+e → na e, ju+e/i → jua, u+e/i → ua.
fn fused(dat: (Person, Number), acc: Number) -> &'static str {
    match (dat, acc) {
        ((Person::P1, Number::Sing), Number::Sing) => "ma",
        ((Person::P1, Number::Sing), Number::Plur) => "m'i",
        ((Person::P2, Number::Sing), Number::Sing) => "ta",
        ((Person::P2, Number::Sing), Number::Plur) => "t'i",
        ((Person::P3, Number::Sing), _) => "ia",
        ((Person::P1, Number::Plur), Number::Sing) => "na e",
        ((Person::P1, Number::Plur), Number::Plur) => "na i",
        ((Person::P2, Number::Plur), _) => "jua",
        ((Person::P3, Number::Plur), _) => "ua",
    }
}

/// Grupi i klitikëve para foljes. Në grup kallëzorja është gjithmonë e vetës III;
/// çdo kombinim tjetër nuk ekziston dhe jep None.
pub fn cluster(dat: Option<(Person, Number)>, acc: Option<(Person, Number)>) -> Option<Form> {
    match (dat, acc) {
        (None, None) => None,
        (Some((p, n)), None) => clitic(p, n, Case::Dat),
        (None, Some((p, n))) => clitic(p, n, Case::Acc),
        (Some(d), Some((Person::P3, an))) => Some(Form::one(fused(d, an), CLUSTER)),
        (Some(_), Some(_)) => None,
    }
}

/// Teksti i grupit të klitikëve (ma, ta, ia, na e, jua, m'i, t'i, ua ...).
pub fn clitic_cluster(dat: Option<(Person, Number)>, acc: Option<(Person, Number)>) -> Option<String> {
    cluster(dat, acc).map(|f| f.text)
}

/// Dhanorja e shkurtër para pjesëzës «u» të trajtës joveprore: m'u duk, t'u duk,
/// iu duk, na u duk, ju u duk, iu duk (atyre).
pub fn clitic_with_u(person: Person, number: Number) -> Form {
    let text = match (person, number) {
        (Person::P1, Number::Sing) => "m'u",
        (Person::P2, Number::Sing) => "t'u",
        (Person::P3, _) => "iu",
        (Person::P1, Number::Plur) => "na u",
        (Person::P2, Number::Plur) => "ju u",
    };
    Form::one(text, MIDDLE)
}

/// Funksioni sintaksor i kundrinës.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Kundrinë e zhdrejtë (në dhanore).
    Indirect,
    /// Kundrinë e drejtë (në kallëzore).
    Direct,
}

/// Kundrina, siç e sheh rregulli i dyfishimit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Object {
    pub role: Role,
    pub definite: bool,
    /// Përemër vetor në trajtë të plotë (mua, ty, atë, ne ...).
    pub stressed_pronoun: bool,
    /// E nxjerrë përpara foljes si temë (Librin e lexova).
    pub fronted: bool,
    /// Në fokus kontrastiv ose si informacion i ri (LIBRIN lexova, jo gazetën).
    pub focus: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doubling {
    Required,
    Optional,
    Excluded,
}

/// Vendimi i dyfishimit dhe rregulli që e jep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    pub doubling: Doubling,
    pub rule: &'static str,
}

/// Dyfishimi i kundrinës me klitik, sipas rendit:
/// 1. kundrina e zhdrejtë dyfishohet gjithmonë: «I dhashë Agimit librin»;
/// 2. përemri vetor i plotë si kundrinë dyfishohet gjithmonë: «Më pa mua»;
/// 3. kundrina e drejtë në fokus nuk dyfishohet: «LIBRIN lexova»;
/// 4. kundrina e drejtë e shquar e nxjerrë përpara si temë dyfishohet: «Librin e lexova»;
/// 5. kundrina e drejtë e shquar në vendin e saj mund të dyfishohet, kur është e njohur: «E lexova librin»;
/// 6. kundrina e pashquar e nxjerrë përpara mund të dyfishohet, kur është specifike;
/// 7. kundrina e pashquar në vendin e saj nuk dyfishohet: «Lexova një libër».
pub fn clitic_doubling(o: &Object) -> Verdict {
    let v = |doubling, rule| Verdict { doubling, rule };
    match (o.role, o.stressed_pronoun, o.focus, o.definite, o.fronted) {
        (Role::Indirect, ..) => v(Doubling::Required, "P-Y1 kundrina e zhdrejtë dyfishohet me klitik"),
        (Role::Direct, true, ..) => v(Doubling::Required, "P-Y2 përemri vetor i plotë si kundrinë dyfishohet"),
        (Role::Direct, false, true, ..) => v(Doubling::Excluded, "P-Y3 kundrina në fokus nuk dyfishohet"),
        (Role::Direct, false, false, true, true) => {
            v(Doubling::Required, "P-Y4 kundrina e shquar e nxjerrë përpara si temë dyfishohet")
        }
        (Role::Direct, false, false, true, false) => {
            v(Doubling::Optional, "P-Y5 kundrina e shquar në vend: klitiku e shënon si të njohur")
        }
        (Role::Direct, false, false, false, true) => {
            v(Doubling::Optional, "P-Y6 kundrina e pashquar e nxjerrë përpara: klitiku e shënon si specifike")
        }
        (Role::Direct, false, false, false, false) => {
            v(Doubling::Excluded, "P-Y7 kundrina e pashquar në vend nuk dyfishohet")
        }
    }
}

/// Vendimi sipas vetës së kundrinës. Kundrina e vetës I dhe II shprehet gjithmonë
/// me klitik, edhe në fokus: «më pa», «të kërkoj», «na thirri», «MUA më pa, jo ty».
/// Veta III ndjek `clitic_doubling`. Kur kundrina është vetëm klitiku (pa emër e pa
/// trajtë të plotë: «e pashë»), klitiku është vetë kundrina dhe merret me `clitic`.
pub fn doubling_for(o: &Object, person: Person) -> Verdict {
    match person {
        Person::P1 | Person::P2 => {
            Verdict { doubling: Doubling::Required, rule: "P-Y0 kundrina e vetës I-II shprehet gjithmonë me klitik" }
        }
        Person::P3 => clitic_doubling(o),
    }
}

/// Klitiku që dyfishon kundrinën, kur dyfishimi nuk përjashtohet (sipas `doubling_for`).
pub fn doubling_clitic(o: &Object, person: Person, number: Number) -> Option<Form> {
    let case = match o.role {
        Role::Indirect => Case::Dat,
        Role::Direct => Case::Acc,
    };
    match doubling_for(o, person).doubling {
        Doubling::Excluded => None,
        Doubling::Required | Doubling::Optional => clitic(person, number, case),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(f: Option<Form>) -> Option<String> {
        f.map(|f| f.text)
    }

    #[test]
    fn full_forms() {
        assert_eq!(text(personal(Person::P1, Number::Sing, None, Case::Acc)).as_deref(), Some("mua"));
        assert_eq!(text(personal(Person::P1, Number::Sing, None, Case::Abl)).as_deref(), Some("meje"));
        assert_eq!(text(personal(Person::P2, Number::Sing, None, Case::Dat)).as_deref(), Some("ty"));
        assert_eq!(text(personal(Person::P1, Number::Plur, None, Case::Abl)).as_deref(), Some("nesh"));
        assert_eq!(text(personal(Person::P2, Number::Plur, None, Case::Dat)).as_deref(), Some("juve"));
        assert_eq!(text(personal(Person::P3, Number::Sing, Some(Gender::Fem), Case::Dat)).as_deref(), Some("asaj"));
        assert_eq!(text(personal(Person::P3, Number::Plur, Some(Gender::Fem), Case::Acc)).as_deref(), Some("ato"));
        assert_eq!(personal(Person::P1, Number::Sing, None, Case::Gen), None);
        assert_eq!(personal(Person::P3, Number::Sing, Some(Gender::Neut), Case::Nom), None);
    }

    #[test]
    fn after_preposition_forms_are_variants() {
        let f = personal(Person::P3, Number::Sing, Some(Gender::Masc), Case::Acc).unwrap();
        assert!(f.accepts("atë") && f.accepts("të"));
        let f = personal(Person::P3, Number::Plur, Some(Gender::Masc), Case::Abl).unwrap();
        assert!(f.accepts("atyre") && f.accepts("tyre"));
        let f = personal(Person::P3, Number::Sing, None, Case::Acc).unwrap();
        assert_eq!(f.text, "atë");
    }

    #[test]
    fn clitics() {
        assert_eq!(text(clitic(Person::P3, Number::Sing, Case::Dat)).as_deref(), Some("i"));
        assert_eq!(text(clitic(Person::P3, Number::Sing, Case::Acc)).as_deref(), Some("e"));
        assert_eq!(text(clitic(Person::P3, Number::Plur, Case::Dat)).as_deref(), Some("u"));
        assert_eq!(text(clitic(Person::P3, Number::Plur, Case::Acc)).as_deref(), Some("i"));
        assert_eq!(text(clitic(Person::P1, Number::Plur, Case::Acc)).as_deref(), Some("na"));
        assert_eq!(clitic(Person::P1, Number::Sing, Case::Nom), None);
    }

    #[test]
    fn clusters() {
        let s = (Person::P3, Number::Sing);
        let p = (Person::P3, Number::Plur);
        assert_eq!(clitic_cluster(Some((Person::P1, Number::Sing)), Some(s)).as_deref(), Some("ma"));
        assert_eq!(clitic_cluster(Some((Person::P1, Number::Sing)), Some(p)).as_deref(), Some("m'i"));
        assert_eq!(clitic_cluster(Some((Person::P2, Number::Sing)), Some(s)).as_deref(), Some("ta"));
        assert_eq!(clitic_cluster(Some((Person::P2, Number::Sing)), Some(p)).as_deref(), Some("t'i"));
        assert_eq!(clitic_cluster(Some(s), Some(s)).as_deref(), Some("ia"));
        assert_eq!(clitic_cluster(Some(s), Some(p)).as_deref(), Some("ia"));
        assert_eq!(clitic_cluster(Some((Person::P1, Number::Plur)), Some(s)).as_deref(), Some("na e"));
        assert_eq!(clitic_cluster(Some((Person::P2, Number::Plur)), Some(p)).as_deref(), Some("jua"));
        assert_eq!(clitic_cluster(Some(p), Some(s)).as_deref(), Some("ua"));
        assert_eq!(clitic_cluster(Some(s), None).as_deref(), Some("i"));
        assert_eq!(clitic_cluster(None, Some(p)).as_deref(), Some("i"));
        assert_eq!(clitic_cluster(Some(s), Some((Person::P1, Number::Sing))), None);
        assert_eq!(clitic_cluster(None, None), None);
        assert_eq!(clitic_with_u(Person::P1, Number::Sing).text, "m'u");
        assert_eq!(clitic_with_u(Person::P3, Number::Plur).text, "iu");
    }

    #[test]
    fn doubling() {
        let base = Object { role: Role::Direct, definite: true, stressed_pronoun: false, fronted: false, focus: false };
        let io = Object { role: Role::Indirect, definite: false, ..base };
        assert_eq!(clitic_doubling(&io).doubling, Doubling::Required);
        assert_eq!(clitic_doubling(&Object { fronted: true, ..base }).doubling, Doubling::Required);
        assert_eq!(clitic_doubling(&base).doubling, Doubling::Optional);
        assert_eq!(clitic_doubling(&Object { focus: true, fronted: true, ..base }).doubling, Doubling::Excluded);
        assert_eq!(clitic_doubling(&Object { definite: false, ..base }).doubling, Doubling::Excluded);
        let me = Object { stressed_pronoun: true, focus: true, ..base };
        assert_eq!(clitic_doubling(&me).doubling, Doubling::Required);
        assert_eq!(text(doubling_clitic(&io, Person::P3, Number::Sing)).as_deref(), Some("i"));
        assert_eq!(text(doubling_clitic(&me, Person::P1, Number::Sing)).as_deref(), Some("më"));
        assert_eq!(doubling_clitic(&Object { definite: false, ..base }, Person::P3, Number::Sing), None);
    }

    #[test]
    fn first_and_second_person_objects_always_take_the_clitic() {
        let bare = Object { role: Role::Direct, definite: false, stressed_pronoun: false, fronted: false, focus: false };
        let focus = Object { definite: true, focus: true, ..bare };
        assert_eq!(doubling_for(&bare, Person::P1).doubling, Doubling::Required);
        assert!(doubling_for(&focus, Person::P2).rule.starts_with("P-Y0"));
        assert_eq!(text(doubling_clitic(&bare, Person::P1, Number::Sing)).as_deref(), Some("më"));
        assert_eq!(text(doubling_clitic(&focus, Person::P2, Number::Sing)).as_deref(), Some("të"));
        assert_eq!(text(doubling_clitic(&bare, Person::P1, Number::Plur)).as_deref(), Some("na"));
        assert_eq!(doubling_for(&focus, Person::P3).doubling, Doubling::Excluded);
    }

    #[test]
    fn after_preposition() {
        let m = Some(Gender::Masc);
        let f = Some(Gender::Fem);
        assert_eq!(text(prepositional(Person::P3, Number::Sing, m, Case::Acc)).as_deref(), Some("të"));
        assert_eq!(text(prepositional(Person::P3, Number::Plur, m, Case::Acc)).as_deref(), Some("ta"));
        assert_eq!(text(prepositional(Person::P3, Number::Plur, f, Case::Acc)).as_deref(), Some("to"));
        assert_eq!(text(prepositional(Person::P3, Number::Sing, m, Case::Abl)).as_deref(), Some("tij"));
        assert_eq!(text(prepositional(Person::P3, Number::Sing, f, Case::Abl)).as_deref(), Some("saj"));
        assert_eq!(text(prepositional(Person::P3, Number::Plur, None, Case::Abl)).as_deref(), Some("tyre"));
        assert_eq!(text(prepositional(Person::P3, Number::Sing, f, Case::Nom)).as_deref(), Some("ajo"));
        assert_eq!(text(prepositional(Person::P1, Number::Sing, None, Case::Acc)).as_deref(), Some("mua"));
        assert_eq!(text(prepositional(Person::P2, Number::Sing, None, Case::Abl)).as_deref(), Some("teje"));
        assert!(prepositional(Person::P3, Number::Sing, m, Case::Acc).unwrap().accepts("atë"));
        assert_eq!(prepositional(Person::P3, Number::Sing, m, Case::Dat), None);
        assert_eq!(prepositional(Person::P3, Number::Plur, None, Case::Acc), None);
    }
}
