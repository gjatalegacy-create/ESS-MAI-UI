//! Lakimi i emrit: rasat e njëjësit dhe të shumësit, në trajtën e pashquar dhe të shquar.
//!
//! Rendi i dëshmisë, i fiksuar:
//! 1. klasa e paradigmës dhe temat e leksemës në leksikun e gramatikës
//!    (uniparser-grammar-albanian 2.3.0, MIT): tema 0 është tema e shumësit, tema 1 trajta e
//!    pashquar e njëjësit, temat 2.. janë temat e ndërruara (motër → motr-, kalë → kal-,
//!    lule → lulj-). Çdo ndërrim fonetik (rënia e «ë»-së para mbaresës me zanore, shumësi me
//!    ndërrim zanoreje) vjen nga temat e leksemës, jo nga hamendja. Leksema lakohet vetëm kur
//!    është e qëndrueshme: emërorja e pashquar = lema, numri i temave sa kërkon klasa; klasa
//!    lakon vetëm kur i shkon tingullit të fundit të temës (`fits`) dhe qeliza jepet vetëm kur
//!    kufiri temë-mbaresë është i mundshëm në shqipe (`joinable`); dy homonime standarde që
//!    s'pajtohen japin `None`, edhe kur njëra ka vetëm shumës; shumësi jepet vetëm për grupet
//!    (klasë, formant) që kalojnë gjysmën Dev (`PLURAL_GATE`), njëjësi s'jepet për emrat
//!    femërorë që leksiku i ka me klasë mashkullore (`GENDER_HOLD`);
//! 2. për fjalët që leksiku nuk i njeh fare (në asnjë pjesë ligjërate) dhe që s'janë trajtë e
//!    lakuar e një emri të tij: mbaresa e lemës, vetëm ato që kalojnë portën e leksikut (Wilson
//!    95% ≥ 0.95) dhe të gjysmës Dev (≥ 0.95), të dyja në qelizat jo-identike (`SG_SHAPES`,
//!    `PL_SHAPES`);
//! 3. ndryshe `None`: emri nuk lakohet me hamendje.
//!
//! Klasa 3f e gramatikës (pa kallëzore të shquar standarde) mbetet jashtë.
//! Gjinorja = nyja e përparme + trajta e dhanores: për `Case::Gen` jepet trajta e dhanores,
//! nyja i takon modulit të nyjave. Rrjedhorja e pashquar e shumësit jepet me «-sh» (malesh),
//! me trajtën e dhanores («-ve») si variant të pranuar. Thirrorja nuk jepet.

use crate::gramatika::leksiku::{Lexeme, Lexicon};
use crate::gramatika::tipet::{Case, Definite, Form, NounCell, Number, Pos};
use Definite::{Def, Ind};
use Slot::{Abl, Acc, Dat, Nom};

/// Rasat me trajtë më vete; gjinorja merr trajtën e dhanores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Nom,
    Dat,
    Acc,
    Abl,
}

/// Mbaresa: numri i temës dhe prapashtesa.
type End = (usize, &'static str);

/// Rreshti i klasës: rasa, shquarsia, trajta standarde, pastaj variantet e pranuara.
type Row = (Slot, Definite, &'static [End]);

/// Çfarë kërkon klasa nga tema e saj.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    Any,
    /// Mbaresa «-i»: tema e shquar nuk mbaron me velare (mal → mali; mik → miku, jo *miki).
    NotVelar,
    /// Mbaresa «-u»: tema e shquar mbaron me velare ose zanore (zog → zogu, njeri → njeriu).
    VelarOrVowel,
    /// 3a: tema 2 është lema me «ë»-në e fundit të rënë, lema vetë ose lema pa «-a».
    Elided,
}

/// Klasa e paradigmës: emri në gramatikë, numri që lakon, rreshtat, kushti mbi temën dhe
/// rregulli që e dëshmon.
#[derive(Debug, Clone, Copy)]
struct Class {
    name: &'static str,
    number: Number,
    rows: &'static [Row],
    fit: Fit,
    rule: &'static str,
}

const S1A: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "i")]),
    (Nom, Def, &[(1, "i")]),
    (Dat, Def, &[(1, "it")]),
    (Acc, Def, &[(1, "in"), (1, "në")]),
    (Abl, Def, &[(1, "it")]),
];

const S1B: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "i")]),
    (Nom, Def, &[(2, "i")]),
    (Dat, Def, &[(2, "it")]),
    (Acc, Def, &[(2, "in"), (2, "inë")]),
    (Abl, Def, &[(2, "it")]),
];

const S1C: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "i")]),
    (Nom, Def, &[(1, "i")]),
    (Dat, Def, &[(1, "it")]),
    (Acc, Def, &[(1, "në"), (1, "in")]),
    (Abl, Def, &[(1, "it")]),
];

const S2A: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "u")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "u")]),
    (Nom, Def, &[(1, "u")]),
    (Dat, Def, &[(1, "ut")]),
    (Acc, Def, &[(1, "un"), (1, "në")]),
    (Abl, Def, &[(1, "ut")]),
];

const S2B: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "u")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "u")]),
    (Nom, Def, &[(2, "u")]),
    (Dat, Def, &[(2, "ut")]),
    (Acc, Def, &[(2, "un")]),
    (Abl, Def, &[(2, "ut")]),
];

const S3A: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "e")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "e")]),
    (Nom, Def, &[(2, "a")]),
    (Dat, Def, &[(2, "ës"), (2, "ësë")]),
    (Acc, Def, &[(2, "ën"), (2, "ënë")]),
    (Abl, Def, &[(2, "ës")]),
];

const S3B: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "e")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "e")]),
    (Nom, Def, &[(3, "a")]),
    (Dat, Def, &[(1, "s")]),
    (Acc, Def, &[(1, "n"), (1, "në")]),
    (Abl, Def, &[(1, "s")]),
];

const S3C: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "e")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "e")]),
    (Nom, Def, &[(2, "a")]),
    (Dat, Def, &[(1, "së")]),
    (Acc, Def, &[(1, "në")]),
    (Abl, Def, &[(1, "së")]),
];

const S3C1: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(4, "e")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(4, "e")]),
    (Nom, Def, &[(3, "a")]),
    (Dat, Def, &[(2, "së")]),
    (Acc, Def, &[(2, "në"), (2, "ë")]),
    (Abl, Def, &[(2, "së")]),
];

const S3D: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "e"), (1, "je")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "e"), (1, "je")]),
    (Nom, Def, &[(1, "a"), (1, "ja")]),
    (Dat, Def, &[(1, "së")]),
    (Acc, Def, &[(1, "në")]),
    (Abl, Def, &[(1, "së")]),
];

const S3E: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "e")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "e")]),
    (Nom, Def, &[(2, "a")]),
    (Dat, Def, &[(1, "s")]),
    (Acc, Def, &[(1, "n")]),
    (Abl, Def, &[(1, "s")]),
];

const S4A: &[Row] = &[(Nom, Def, &[(1, "t")]), (Acc, Def, &[(1, "t")])];

const S4A1: &[Row] = &[(Nom, Def, &[(1, "ët")]), (Acc, Def, &[(1, "ët")])];

const S4B: &[Row] = &[(Nom, Def, &[(1, "të")]), (Acc, Def, &[(1, "të")])];

const S4C: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "i")]),
    (Nom, Def, &[(1, "t")]),
    (Dat, Def, &[(2, "it")]),
    (Acc, Def, &[(1, "t")]),
    (Abl, Def, &[(2, "it")]),
];

const S4D: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "i")]),
    (Nom, Def, &[(1, "it")]),
    (Dat, Def, &[(1, "it")]),
    (Acc, Def, &[(1, "it")]),
    (Abl, Def, &[(1, "it")]),
];

const S4E: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "i")]),
    (Nom, Def, &[(1, "të")]),
    (Dat, Def, &[(1, "it")]),
    (Acc, Def, &[(1, "të")]),
    (Abl, Def, &[(1, "it")]),
];

const S4F: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(2, "i")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(2, "i")]),
    (Nom, Def, &[(2, "itë")]),
    (Dat, Def, &[(2, "it")]),
    (Acc, Def, &[(2, "itë")]),
    (Abl, Def, &[(2, "it")]),
];

const SINEZOT: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(3, "")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(4, "")]),
    (Nom, Def, &[(1, "i")]),
    (Acc, Def, &[(2, "në")]),
];

const SSURNAME: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Acc, Ind, &[(1, "")]),
    (Nom, Def, &[(1, "")]),
    (Dat, Def, &[(1, "t")]),
    (Acc, Def, &[(1, "n")]),
    (Abl, Def, &[(1, "t")]),
];

const P5A: &[Row] = &[
    (Nom, Ind, &[(0, "")]),
    (Dat, Ind, &[(0, "ve")]),
    (Acc, Ind, &[(0, "")]),
    (Abl, Ind, &[(0, "sh"), (0, "ve")]),
    (Nom, Def, &[(0, "t"), (0, "të")]),
    (Dat, Def, &[(0, "ve"), (0, "vet")]),
    (Acc, Def, &[(0, "t"), (0, "të")]),
    (Abl, Def, &[(0, "ve"), (0, "vet"), (0, "shit")]),
];

const P5A1: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "ve")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "sh"), (1, "ve")]),
    (Nom, Def, &[(1, "t")]),
    (Dat, Def, &[(1, "ve"), (0, "vet")]),
    (Acc, Def, &[(1, "t")]),
    (Abl, Def, &[(1, "ve"), (0, "vet"), (1, "shit")]),
];

const P5B: &[Row] = &[
    (Nom, Ind, &[(0, "")]),
    (Dat, Ind, &[(0, "ve")]),
    (Acc, Ind, &[(0, "")]),
    (Abl, Ind, &[(0, "sh"), (0, "ve")]),
    (Nom, Def, &[(0, "të"), (0, "t")]),
    (Dat, Def, &[(0, "ve"), (0, "vet")]),
    (Acc, Def, &[(0, "të"), (0, "t")]),
    (Abl, Def, &[(0, "ve"), (0, "vet"), (0, "shit")]),
];

const P5C: &[Row] = &[
    (Nom, Ind, &[(0, "")]),
    (Dat, Ind, &[(0, "ve"), (0, "ëve")]),
    (Acc, Ind, &[(0, "")]),
    (Abl, Ind, &[(0, "ish"), (0, "ve"), (0, "ëve")]),
    (Nom, Def, &[(0, "it"), (0, "itë")]),
    (Dat, Def, &[(0, "ve"), (0, "vet"), (0, "ëve"), (0, "ëvet")]),
    (Acc, Def, &[(0, "it"), (0, "itë")]),
    (Abl, Def, &[(0, "ve"), (0, "vet"), (0, "ëve"), (0, "ëvet"), (0, "ishit"), (0, "ëshit")]),
];

const P5C1: &[Row] = &[
    (Nom, Ind, &[(0, "")]),
    (Dat, Ind, &[(0, "ve")]),
    (Acc, Ind, &[(0, "")]),
    (Abl, Ind, &[(2, "ish"), (0, "ve")]),
    (Nom, Def, &[(2, "it"), (2, "itë")]),
    (Dat, Def, &[(0, "ve"), (0, "vet")]),
    (Acc, Def, &[(2, "it"), (2, "itë")]),
    (Abl, Def, &[(0, "ve"), (0, "vet"), (2, "ishit"), (2, "ëshit")]),
];

const P5D: &[Row] = &[
    (Nom, Ind, &[(0, "")]),
    (Dat, Ind, &[(0, "ve")]),
    (Acc, Ind, &[(0, "")]),
    (Abl, Ind, &[(0, "ësh"), (0, "ve")]),
    (Nom, Def, &[(0, "ët")]),
    (Dat, Def, &[(0, "ve"), (0, "vet")]),
    (Acc, Def, &[(0, "ët")]),
    (Abl, Def, &[(0, "ve"), (0, "vet"), (0, "ëshit")]),
];

const P5E: &[Row] = &[
    (Nom, Ind, &[(0, "")]),
    (Dat, Ind, &[(0, "ëve")]),
    (Acc, Ind, &[(0, "")]),
    (Abl, Ind, &[(0, "ësh"), (0, "ëve")]),
    (Nom, Def, &[(0, "ët")]),
    (Dat, Def, &[(0, "ëve"), (0, "ëvet")]),
    (Acc, Def, &[(0, "ët")]),
    (Abl, Def, &[(0, "ëve"), (0, "ëvet"), (0, "ëshit")]),
];

const PVJEHERR: &[Row] = &[
    (Nom, Ind, &[(1, "")]),
    (Dat, Ind, &[(1, "ve"), (0, "ve")]),
    (Acc, Ind, &[(1, "")]),
    (Abl, Ind, &[(1, "sh"), (0, "sh"), (1, "ve"), (0, "ve")]),
    (Nom, Def, &[(1, "t")]),
    (Dat, Def, &[(1, "ve"), (0, "ve"), (1, "vet"), (0, "vet")]),
    (Acc, Def, &[(1, "t")]),
    (Abl, Def, &[(1, "ve"), (0, "ve"), (1, "vet"), (0, "vet"), (1, "shit"), (0, "shit")]),
];

/// Klasa e paradigmës sipas emrit të saj në gramatikë; klasat pa trajtë standarde mbeten jashtë.
fn class_of(par: &str) -> Option<Class> {
    use Fit::{Any, Elided, NotVelar, VelarOrVowel};
    let sg = |name, rows, fit, rule| Some(Class { name, number: Number::Sing, rows, fit, rule });
    let pl = |name, rows, rule| Some(Class { name, number: Number::Plur, rows, fit: Any, rule });
    match par {
        "1a" => sg("1a", S1A, NotVelar, "R-E1a: tema + -i/-it/-in (mal → mali, malit, malin)"),
        "1b" => sg("1b", S1B, NotVelar, "R-E1b: tema e ndërruar + -i/-it/-in (kalë → kali, kalit, kalin)"),
        "1c" => sg("1c", S1C, Any, "R-E1c: tema + -i/-it/-në (budalla → budallai, budallanë)"),
        "2a" => sg("2a", S2A, VelarOrVowel, "R-E2a: tema + -u/-ut/-un (zog → zogu, zogut, zogun)"),
        "2b" => sg("2b", S2B, VelarOrVowel, "R-E2b: tema e ndërruar + -u/-ut/-un (krye → kreu, kreut, kreun)"),
        "3a" => sg("3a", S3A, Elided, "R-E3a: tema e ndërruar + -a/-ës/-ën (motër → motra, motrës, motrën)"),
        "3b" => sg("3b", S3B, Any, "R-E3b: -ja/-s/-n (lule → lulja, lules, lulen; lagje → lagjja)"),
        "3c" => sg("3c", S3C, Any, "R-E3c: -ja/-së/-në (gjë → gjëja, gjësë, gjënë)"),
        "3c-1" => sg("3c-1", S3C1, Any, "R-E3c-1: temat e ndërruara + -a/-së/-në"),
        "3d" => sg("3d", S3D, Any, "R-E3d: zanore e theksuar + -a/-së/-në (shtëpi → shtëpia, shtëpisë, shtëpinë)"),
        "3e" => sg("3e", S3E, Any, "R-E3e: -ja/-s/-n (nano → nanoja, nanos, nanon)"),
        "4a" => sg("4a", S4A, Any, "R-E4a: asnjanës, e shquar -t"),
        "4a-1" => sg("4a-1", S4A1, Any, "R-E4a-1: asnjanës, e shquar -ët"),
        "4b" => sg("4b", S4B, Any, "R-E4b: asnjanës, e shquar -të"),
        "4c" => sg("4c", S4C, Any, "R-E4c: asnjanës -t/-it"),
        "4d" => sg("4d", S4D, Any, "R-E4d: asnjanës -it"),
        "4e" => sg("4e", S4E, Any, "R-E4e: asnjanës -të/-it"),
        "4f" => sg("4f", S4F, Any, "R-E4f: asnjanës -itë/-it"),
        "inezot" => sg("inezot", SINEZOT, Any, "R-E inëzot: temat e veçanta të leksemës"),
        "surname" => sg("surname", SSURNAME, Any, "R-E mbiemër: -t/-n"),
        "5a" => pl("5a", P5A, "R-E5a: shumës + -t/-ve/-sh (male → malet, maleve, malesh)"),
        "5a-1" => pl("5a-1", P5A1, "R-E5a-1: tema e shumësit 1 + -t/-ve/-sh"),
        "5b" => pl("5b", P5B, "R-E5b: shumës + -të/-ve/-sh (zogj → zogjtë, zogjve, zogjsh)"),
        "5c" => pl("5c", P5C, "R-E5c: shumës + -it/-ve/-ish (njerëz → njerëzit, njerëzve, njerëzish)"),
        "5c-1" => pl("5c-1", P5C1, "R-E5c-1: shumës + -it/-ve/-ish me temë të ndërruar"),
        "5d" => pl("5d", P5D, "R-E5d: shumës + -ët/-ve/-ësh"),
        "5e" => pl("5e", P5E, "R-E5e: shumës + -ët/-ëve/-ësh (net → netët, netëve, netësh)"),
        "vjeherr" => pl("vjeherr", PVJEHERR, "R-E vjehërr: dy tema shumësi"),
        _ => None,
    }
}

fn slot_of(case: Case) -> Option<Slot> {
    match case {
        Case::Nom => Some(Nom),
        Case::Gen | Case::Dat => Some(Dat),
        Case::Acc => Some(Acc),
        Case::Abl => Some(Abl),
        Case::Voc => None,
    }
}

const VOWEL: [char; 7] = ['a', 'e', 'ë', 'i', 'o', 'u', 'y'];

/// Shkronjat e alfabetit të shqipes (dyshkronjëshat shkruhen me to).
const ALPHABET: &str = "abcçdeëfghijklmnopqrstuvxyz";

fn albanian(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| ALPHABET.contains(c))
}

/// Kufiri temë-mbaresë i mundshëm në shqipen standarde; nuk lindin kurrë:
/// «ë» + zanore dhe zanore + «ë» (motër → motra; *heshtaës); e njëjta shkronjë dy herë
/// (*krosistt, *kalaa); a/o/u + «e» (*heshtae); «-të» pas «ë»-së (shokët, jo *shokëtë);
/// «-sh» pas s/sh/z/zh (*punonjëssh); «-së»/«-në» pas bashkëtingëllores (*afrimnë);
/// «-s», «-n», «-t» pas bashkëtingëllores, veç zanore + j (kuajt, akujt) dhe ua/ye + r (duart,
/// dyert); në shumës «-t» pas «-i» (xhamitë, jo *xhamit) dhe «-it»/«-ish» pas zanores (*gjyshait).
fn joinable(stem: &str, suffix: &str, number: Number) -> bool {
    match (stem.chars().last(), suffix.chars().next()) {
        (Some(a), Some(b)) => !blocked(stem, a, suffix, b, number == Number::Plur),
        _ => true,
    }
}

fn blocked(stem: &str, a: char, suffix: &str, b: char, plural: bool) -> bool {
    let vowel = |c: char| VOWEL.contains(&c);
    let glide = a == 'j' && stem.chars().rev().nth(1).is_some_and(vowel);
    let open = vowel(a) || glide;
    let sibilant = ["s", "sh", "z", "zh"].iter().any(|s| stem.ends_with(s));
    let diphthong_r = stem.ends_with("uar") || stem.ends_with("yer");
    [
        a == 'ë' && vowel(b),
        vowel(a) && b == 'ë',
        a == b,
        "aou".contains(a) && b == 'e',
        suffix == "të" && a == 'ë',
        suffix.starts_with("sh") && sibilant,
        (suffix == "së" || suffix == "në") && !vowel(a),
        (suffix == "s" || suffix == "n") && !open,
        suffix == "t" && !(open || diphthong_r),
        plural && suffix == "t" && a == 'i',
        plural && b == 'i' && vowel(a),
    ]
    .contains(&true)
}

/// Velare fundore: k, g, h (jo h-ja e dyshkronjëshave sh, th, dh, zh, xh).
fn velar(stem: &str) -> bool {
    let digraph = ["sh", "th", "dh", "zh", "xh"].iter().any(|d| stem.ends_with(d));
    stem.ends_with(['k', 'g', 'h']) && !digraph
}

/// Tema 2 e klasës 3a: lema me «ë»-në e fundit të rënë (motër → motr-, vajzë → vajz-),
/// lema vetë (arrëz → arrëz-) ose lema pa «-a» (Berisha → Berish-).
fn elided(lemma: &str, stem: &str) -> bool {
    let dropped = lemma.rfind('ë').map(|i| {
        let (head, tail) = lemma.split_at(i);
        format!("{head}{}", tail.strip_prefix('ë').unwrap_or(tail))
    });
    stem == lemma || dropped.as_deref() == Some(stem) || lemma.strip_suffix('a') == Some(stem)
}

/// Klasa i shkon temës sipas kushtit të saj (`Fit`); ndryshe qelizat e saj mbeten `None`.
fn fits(lemma: &str, stems: &[Vec<String>], class: &Class) -> bool {
    let first = |i: usize| stems.get(i).and_then(|v| v.first()).map(String::as_str);
    let def = class.rows.iter().find(|r| r.0 == Nom && r.1 == Def).and_then(|r| r.2.first()).and_then(|e| first(e.0));
    match class.fit {
        Fit::Any => true,
        Fit::NotVelar => def.is_none_or(|s| !velar(s)),
        Fit::VelarOrVowel => def.is_none_or(|s| velar(s) || s.ends_with(VOWEL)),
        Fit::Elided => first(2).is_none_or(|s| elided(lemma, s)),
    }
}

/// Trajtat e një qelize të klasës: e para është standardja; `None` kur tema e saj mungon
/// ose kur kufiri temë-mbaresë s'është i mundshëm.
fn realize(stems: &[Vec<String>], class: &Class, slot: Slot, def: Definite) -> Option<Vec<String>> {
    let ends = class.rows.iter().find(|r| r.0 == slot && r.1 == def).map(|r| r.2)?;
    let (head, _) = ends.split_first()?;
    stems.get(head.0).and_then(|v| v.first()).filter(|s| joinable(s, head.1, class.number))?;
    let mut out: Vec<String> = Vec::new();
    for (i, suffix) in ends {
        for s in stems.get(*i).into_iter().flatten().filter(|s| joinable(s, suffix, class.number)) {
            let f = format!("{s}{suffix}");
            match out.contains(&f) {
                true => {}
                false => out.push(f),
            }
        }
    }
    Some(out)
}

/// Trajta e qelizës nga temat dhe klasat e dhëna: klasa e parë që e ka qelizën jep tekstin,
/// të tjerat japin variante.
fn decline_with(stems: &[Vec<String>], classes: &[Class], cell: NounCell) -> Option<Form> {
    let slot = slot_of(cell.case)?;
    let mut found = classes
        .iter()
        .filter(|c| c.number == cell.number)
        .filter_map(|c| realize(stems, c, slot, cell.definite).map(|f| (c.rule, f)));
    let (rule, first) = found.next()?;
    let mut forms = first;
    for (_, more) in found {
        for f in more {
            match forms.contains(&f) {
                true => {}
                false => forms.push(f),
            }
        }
    }
    let mut it = forms.into_iter();
    let text = it.next()?;
    Some(Form { text, variants: it.collect(), rule })
}

fn classes_of(lex: &Lexeme) -> Vec<Class> {
    lex.paradigms.iter().filter_map(|p| class_of(p)).collect()
}

/// Leksema është e qëndrueshme kur (1) trajta e saj e emërores së pashquar është vetë lema
/// (njëjësi kur ka klasë njëjësi, ndryshe tema e njëjësit, ndryshe shumësi) dhe (2) numri i
/// temave nuk e kalon numrin që kërkojnë klasat e saj: tema të tepërta tregojnë klasë tjetër.
/// Ndryshe temat s'i besohen.
fn consistent(lemma: &str, stems: &[Vec<String>], classes: &[Class]) -> bool {
    let nom = |number| decline_with(stems, classes, NounCell { case: Case::Nom, number, definite: Ind });
    let has_sg = classes.iter().any(|c| c.number == Number::Sing);
    let stem1 = stems.get(1).and_then(|v| v.first()).is_some_and(|s| s == lemma);
    let anchored = match (nom(Number::Sing), has_sg) {
        (Some(f), _) => f.text == lemma,
        (None, true) => stem1,
        (None, false) => nom(Number::Plur).is_some_and(|f| f.text == lemma),
    };
    anchored && stems.len() <= needed(classes)
}

/// Sa tema kërkojnë klasat: numri më i madh i temës në mbaresat e tyre + 1.
fn needed(classes: &[Class]) -> usize {
    classes.iter().flat_map(|c| c.rows.iter()).flat_map(|r| r.2.iter()).map(|e| e.0 + 1).max().unwrap_or(0)
}

/// Tema 3 e klasës 3b (e shquara pa «-a») sipas drejtshkrimit, për lemat me «-e» pas
/// bashkëtingëllores: lema pa «-e» + «j» (lule → lulj-, lagje → lagjj-, përgjigje → përgjigjj-);
/// lema pa «-e» kur ajo mbaron me «j» a «nj» (lëvizje → lëvizj-). Pas «n» dhe «g» vendos
/// leksiku (gjermane → gjermanja, kolege → kolegia): `None`, pa kontroll.
fn palatal_stem(lemma: &str) -> Option<String> {
    let base = lemma.strip_suffix('e').filter(|b| b.chars().last().is_some_and(|c| !VOWEL.contains(&c)))?;
    let free = base.ends_with('n') || base.ends_with('g');
    match (base.ends_with("gj"), base.ends_with('j'), free) {
        (true, _, _) => Some(format!("{base}j")),
        (false, true, _) => Some(base.to_string()),
        (false, false, true) => None,
        (false, false, false) => Some(format!("{base}j")),
    }
}

/// Temat e leksemës: vetëm variantet me shkronja të alfabetit të shqipes (jo «NONE», jo
/// shkurtesa me vizë). Te 3b tema 3 pranohet vetëm kur ka variantin e drejtshkrimit
/// (`palatal_stem`), i cili del i pari; ndryshe tema 3 mbetet bosh dhe e shquara e njëjësit
/// `None` (rumanishte: jo *rumanishteja; gjegje: jo *gjegja).
fn stems_of(lex: &Lexeme) -> Vec<Vec<String>> {
    let palatal = lex.paradigms.iter().any(|p| p == "3b");
    let want = palatal_stem(&lex.lemma);
    lex.clean_stems()
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let mut v: Vec<String> = v.into_iter().filter(|s| albanian(s)).collect();
            let check = want.as_ref().filter(|_| palatal && i == 3);
            let _ = check.map(|w| match v.contains(w) {
                true => v.sort_by_key(|s| s != w),
                false => v.clear(),
            });
            v
        })
        .collect()
}

/// Lakimi nga temat dhe klasat e leksemës, pa portat e besimit (`decline`).
fn decline_raw(lex: &Lexeme, cell: NounCell) -> Option<Form> {
    let classes = classes_of(lex);
    let stems = stems_of(lex);
    let fitted: Vec<Class> = classes.iter().copied().filter(|c| fits(&lex.lemma, &stems, c)).collect();
    consistent(&lex.lemma, &stems, &classes).then(|| decline_with(&stems, &fitted, cell)).flatten()
}

/// Emra femërorë që leksiku i jep me klasë mashkullore 1a/1b (vetull → *vetulli, bishë →
/// *bishi; badër, cumër sipas gjysmës Dev): gjinia e leksikut s'u besohet, njëjësi mbetet
/// `None`.
pub const GENDER_HOLD: [&str; 7] = ["badër", "bishë", "cumër", "kryevepër", "nofull", "vetull", "vorbull"];

/// Formanti i shumësit: si del emërorja e pashquar e shumësit nga lema: «=» e njëjtë
/// (lule → lule), «+e» (mal → male), «+ë» (gur → gurë), «+a» (lis → lisa), «ë>a»
/// (vajzë → vajza), «*» çdo ndërrim tjetër (motër → motra, natë → net, zog → zogj).
fn formant(lemma: &str, plural: &str) -> &'static str {
    let schwa_a = lemma.strip_suffix('ë').and_then(|b| plural.strip_prefix(b)) == Some("a");
    match (plural.strip_prefix(lemma), schwa_a) {
        (Some(""), _) => "=",
        (Some("e"), _) => "+e",
        (Some("ë"), _) => "+ë",
        (Some("a"), _) => "+a",
        (_, true) => "ë>a",
        _ => "*",
    }
}

/// Grupet (klasa e njëjësit, formanti) ku shumësi i leksikut pajtohet me arin në gjysmën Dev:
/// të paktën 20 lema dhe saktësia në qelizat e shumësit ≥ 0.95 (1a +e 0.970, 3a ë>a 0.951,
/// 3a * 0.983, 3b = 1.000, 3d = 0.987). Jashtë mbeten 1a +ë 0.858, 1a +a 0.914, 1a = 0.701,
/// 1a * 0.866, 1b * 0.894, 2a +ë 0.930, 2a * 0.925, 3a = 0.919, 3c = 0.813 dhe grupet me
/// më pak se 20 lema: shumësi i tyre mbetet `None`.
const PLURAL_GATE: [(&str, &str); 5] = [("1a", "+e"), ("3a", "ë>a"), ("3a", "*"), ("3b", "="), ("3d", "=")];

/// Shumësi i leksemës i besohet kur grupi i saj (klasa e parë e njëjësit, formanti) është
/// në `PLURAL_GATE`.
fn plural_trusted(lex: &Lexeme) -> bool {
    let sg = classes_of(lex).into_iter().find(|c| c.number == Number::Sing).map(|c| c.name);
    let nom = decline_raw(lex, NounCell { case: Case::Nom, number: Number::Plur, definite: Ind });
    sg.zip(nom).is_some_and(|(s, f)| PLURAL_GATE.contains(&(s, formant(&lex.lemma, &f.text))))
}

/// Trajta e emrit në një qelizë, sipas klasës dhe temave të leksemës. Qëndrueshmëria matet me
/// të gjitha klasat; qelizat jepen vetëm nga klasat që i shkojnë temës. Njëjësi mbetet `None`
/// për emrat e `GENDER_HOLD`, shumësi për grupet jashtë `PLURAL_GATE`.
pub fn decline(lex: &Lexeme, cell: NounCell) -> Option<Form> {
    let open = match cell.number {
        Number::Sing => !GENDER_HOLD.contains(&lex.lemma.as_str()),
        Number::Plur => plural_trusted(lex),
    };
    open.then(|| decline_raw(lex, cell)).flatten()
}

/// Leksema standarde vetëm me shumës për një lemë që s'është vetë shumës (çorap: çorape krahas
/// çorapë): dëshmi e dytë e leksikut për shumësin e lemës.
fn plural_twin(l: &Lexeme, cell: NounCell) -> Option<Form> {
    let classes = classes_of(l);
    let stems = stems_of(l);
    let only_plural = !classes.is_empty() && classes.iter().all(|c| c.number == Number::Plur);
    let usable = l.standard && l.pos == Pos::Noun && only_plural && cell.number == Number::Plur && stems.len() <= needed(&classes);
    usable.then(|| decline_with(&stems, &classes, cell)).flatten().filter(|f| f.text != l.lemma)
}

/// Renditja e leksemave të një leme: standardja, me klasë emri, me njëjës të plotë
/// (me emërore të pashquar), me shumës.
fn rank(l: &Lexeme) -> (bool, bool, bool, bool) {
    let cs = classes_of(l);
    let full = cs.iter().any(|c| c.number == Number::Sing && c.rows.iter().any(|r| r.0 == Nom && r.1 == Ind));
    let pl = cs.iter().any(|c| c.number == Number::Plur);
    (l.standard, !cs.is_empty(), full, pl)
}

/// Leksema e emrit për një lemë: më e larta në renditje; ndër të barabartat, e para e leksikut.
pub fn noun_lexeme<'a>(lx: &'a Lexicon, lemma: &str) -> Option<&'a Lexeme> {
    lx.entries(lemma).iter().filter(|l| l.pos == Pos::Noun).min_by_key(|l| std::cmp::Reverse(rank(l)))
}

/// Qelizat e lakimit, në rendin e tabelës shkollore.
pub const CELLS: [NounCell; 20] = {
    const fn c(case: Case, number: Number, definite: Definite) -> NounCell {
        NounCell { case, number, definite }
    }
    [
        c(Case::Nom, Number::Sing, Ind),
        c(Case::Gen, Number::Sing, Ind),
        c(Case::Dat, Number::Sing, Ind),
        c(Case::Acc, Number::Sing, Ind),
        c(Case::Abl, Number::Sing, Ind),
        c(Case::Nom, Number::Sing, Def),
        c(Case::Gen, Number::Sing, Def),
        c(Case::Dat, Number::Sing, Def),
        c(Case::Acc, Number::Sing, Def),
        c(Case::Abl, Number::Sing, Def),
        c(Case::Nom, Number::Plur, Ind),
        c(Case::Gen, Number::Plur, Ind),
        c(Case::Dat, Number::Plur, Ind),
        c(Case::Acc, Number::Plur, Ind),
        c(Case::Abl, Number::Plur, Ind),
        c(Case::Nom, Number::Plur, Def),
        c(Case::Gen, Number::Plur, Def),
        c(Case::Dat, Number::Plur, Def),
        c(Case::Acc, Number::Plur, Def),
        c(Case::Abl, Number::Plur, Def),
    ]
};

/// Tabela e plotë e lakimit të leksemës: vetëm qelizat që gramatika i dëshmon.
pub fn paradigm(lex: &Lexeme) -> Vec<(NounCell, Form)> {
    CELLS.iter().filter_map(|c| decline(lex, *c).map(|f| (*c, f))).collect()
}

/// Tabela e plotë e lakimit sipas lemës, me të njëjtin rend dëshmie si `decline_lemma`.
pub fn paradigm_lemma(lx: &Lexicon, lemma: &str) -> Vec<(NounCell, Form)> {
    CELLS.iter().filter_map(|c| decline_lemma(lx, lemma, *c).map(|f| (*c, f))).collect()
}

/// Lakimi sipas lemës: leksema më e plotë e emrit; kur dy leksema të barabarta (homonime)
/// japin trajta të ndryshme për qelizën, `None`. Për shumësin, edhe leksema standarde vetëm me
/// shumës e së njëjtës lemë (`plural_twin`) është homonime: kur s'pajtohet, `None`
/// (xham: xhame / xhama). Leksema e paqëndrueshme mbetet `None`, jo hamendje. Mbaresa e lemës
/// vendos vetëm kur leksiku nuk e njeh fjalën në asnjë pjesë ligjërate dhe fjala s'është trajtë
/// e lakuar e një emri të leksikut; fjala që leksiku e njeh vetëm si folje, ndajfolje, lidhëz
/// etj. nuk lakohet si emër.
pub fn decline_lemma(lx: &Lexicon, lemma: &str, cell: NounCell) -> Option<Form> {
    match (noun_lexeme(lx, lemma), lx.entries(lemma).is_empty()) {
        (Some(best), _) => {
            let form = decline(best, cell)?;
            let top = rank(best);
            let others = || lx.entries(lemma).iter().filter(|l| l.pos == Pos::Noun && !std::ptr::eq(*l, best));
            let peers = others().filter(|l| rank(l) == top).all(|l| decline(l, cell).is_none_or(|f| f.text == form.text));
            let twins = others().filter_map(|l| plural_twin(l, cell)).all(|f| f.text == form.text);
            (peers && twins).then_some(form)
        }
        (None, true) => (!known_form(lx, lemma)).then(|| decline_shape(lemma, cell)).flatten(),
        (None, false) => None,
    }
}

/// Fjala është trajtë e lakuar e një emri të leksikut (malit, vajzës, librat), edhe kur porta e
/// besimit e mban atë qelizë: lema e tij gjendet duke hequr 1–4 shkronja nga fundi dhe duke
/// rikthyer «-ë», «-e» ose «ë»-në e rënë para sonantes së fundit (libr- → libër).
fn known_form(lx: &Lexicon, word: &str) -> bool {
    (1..=4)
        .filter_map(|n| cut(word, n))
        .filter(|base| !base.is_empty())
        .flat_map(lemma_guesses)
        .any(|l| {
            let forms = |x: &Lexeme| CELLS.iter().filter_map(|c| decline_raw(x, *c).or_else(|| plural_twin(x, *c))).any(|f| f.accepts(word));
            lx.entries(&l).iter().filter(|x| x.pos == Pos::Noun).any(forms)
        })
}

fn lemma_guesses(base: &str) -> Vec<String> {
    let inner = base
        .chars()
        .last()
        .filter(|c| "rlmn".contains(*c))
        .and_then(|c| base.strip_suffix(c).filter(|h| h.chars().last().is_some_and(|p| !VOWEL.contains(&p))).map(|h| format!("{h}ë{c}")));
    [Some(base.to_string()), Some(format!("{base}ë")), Some(format!("{base}e")), inner].into_iter().flatten().collect()
}

/// Mbaresë e lemës me klasë të qëndrueshme: temat ndërtohen nga lema (sa shkronja hiqen
/// nga fundi, çfarë shtohet), duke filluar nga tema `from`, pastaj lakohen si klasa e gramatikës.
struct Shape {
    suffix: &'static str,
    class: &'static str,
    from: usize,
    stems: &'static [(usize, &'static str)],
    rule: &'static str,
}

const fn sg(suffix: &'static str, class: &'static str, stems: &'static [(usize, &'static str)], rule: &'static str) -> Shape {
    Shape { suffix, class, from: 1, stems, rule }
}

const fn pl(suffix: &'static str, class: &'static str, stems: &'static [(usize, &'static str)], rule: &'static str) -> Shape {
    Shape { suffix, class, from: 0, stems, rule }
}

/// Mbaresë pa klasë: fiton mbi mbaresën më të shkurtër dhe jep `None`.
const fn hold(suffix: &'static str, rule: &'static str) -> Shape {
    Shape { suffix, class: "", from: 1, stems: &[], rule }
}

const SAME: &[(usize, &str)] = &[(0, "")];
const E_J: &[(usize, &str)] = &[(0, ""), (0, "j"), (1, "j")];

/// Njëjësi sipas mbaresës së lemës.
const SG_SHAPES: &[Shape] = &[
    sg("ë", "3a", &[(0, ""), (1, "")], "R-E9 njëjës: -ë → 3a (lemë pa «ë» + -a/-ës/-ën)"),
    sg("je", "3b", &[(0, ""), (0, "j"), (1, "")], "R-E9 njëjës: -je → 3b (lëvizje → lëvizja, lëvizjeje)"),
    sg("ore", "3b", E_J, "R-E9 njëjës: -ore → 3b (punëtore → punëtorja)"),
    sg("are", "3b", E_J, "R-E9 njëjës: -are → 3b (fshatare → fshatarja)"),
    sg("iste", "3b", E_J, "R-E9 njëjës: -iste → 3b (artiste → artistja)"),
    sg("si", "3d", SAME, "R-E9 njëjës: -si → 3d (-a/-së/-në)"),
    sg("ji", "3d", SAME, "R-E9 njëjës: -ji → 3d (-a/-së/-në)"),
    sg("tari", "3d", SAME, "R-E9 njëjës: -tari → 3d (-a/-së/-në)"),
    sg("im", "1a", SAME, "R-E9 njëjës: -im → 1a (-i/-it/-in)"),
    sg("s", "1a", SAME, "R-E9 njëjës: -s → 1a (-i/-it/-in)"),
    sg("t", "1a", SAME, "R-E9 njëjës: -t → 1a (-i/-it/-in)"),
    sg("n", "1a", SAME, "R-E9 njëjës: -n → 1a (-i/-it/-in)"),
    hold("ën", "R-E9 njëjës: -ën → pa trajtë («ë» para sonantes fundore bie: libër → libri, gjarpër → gjarpri)"),
    sg("ar", "1a", SAME, "R-E9 njëjës: -ar → 1a (-i/-it/-in)"),
    sg("or", "1a", SAME, "R-E9 njëjës: -or → 1a (-i/-it/-in)"),
    sg("er", "1a", SAME, "R-E9 njëjës: -er → 1a (-i/-it/-in)"),
    sg("sh", "1a", SAME, "R-E9 njëjës: -sh → 1a (-i/-it/-in)"),
    sg("c", "1a", SAME, "R-E9 njëjës: -c → 1a (-i/-it/-in)"),
    sg("p", "1a", SAME, "R-E9 njëjës: -p → 1a (-i/-it/-in)"),
    sg("ç", "1a", SAME, "R-E9 njëjës: -ç → 1a (-i/-it/-in)"),
    sg("th", "1a", SAME, "R-E9 njëjës: -th → 1a (-i/-it/-in)"),
    sg("q", "1a", SAME, "R-E9 njëjës: -q → 1a (-i/-it/-in)"),
    sg("d", "1a", SAME, "R-E9 njëjës: -d → 1a (-i/-it/-in)"),
    sg("f", "1a", SAME, "R-E9 njëjës: -f → 1a (-i/-it/-in)"),
    sg("al", "1a", SAME, "R-E9 njëjës: -al → 1a (-i/-it/-in)"),
    sg("il", "1a", SAME, "R-E9 njëjës: -il → 1a (-i/-it/-in)"),
];

/// Shumësi sipas prapashtesës së lemës.
const PL_SHAPES: &[Shape] = &[
    pl("së", "5a", &[(1, "a")], "R-E9 shumës: -së → -sa, 5a (kërkesë → kërkesa, kërkesat)"),
    pl("rinë", "5a", &[(1, "a")], "R-E9 shumës: -rinë → -rina, 5a"),
    pl("e", "5a", SAME, "R-E9 shumës: -e → i njëjtë, 5a (lule → lule, lulet)"),
    pl("im", "5a", &[(0, "e")], "R-E9 shumës: -im → -ime, 5a (mendim → mendime)"),
    pl("si", "5b", SAME, "R-E9 shumës: -si → i njëjtë, 5b (pasuri → pasuritë)"),
    pl("ëri", "5b", SAME, "R-E9 shumës: -ëri → i njëjtë, 5b"),
    pl("ji", "5b", SAME, "R-E9 shumës: -ji → i njëjtë, 5b"),
    pl("tari", "5b", SAME, "R-E9 shumës: -tari → i njëjtë, 5b"),
    pl("tar", "5a", &[(0, "ë")], "R-E9 shumës: -tar → -tarë, 5a (luftëtar → luftëtarë)"),
    pl("ist", "5a", &[(0, "ë")], "R-E9 shumës: -ist → -istë, 5a (artist → artistë)"),
    pl("zëm", "5a", &[(2, "ma")], "R-E9 shumës: -zëm → -zma, 5a (realizëm → realizma)"),
    pl("itet", "5a", &[(0, "e")], "R-E9 shumës: -itet → -itete, 5a"),
    pl("cion", "5a", &[(0, "e")], "R-E9 shumës: -cion → -cione, 5a"),
];

/// Lema pa `n` shkronjat e fundit.
fn cut(s: &str, n: usize) -> Option<&str> {
    match n {
        0 => Some(s),
        _ => s.char_indices().rev().nth(n - 1).map(|(i, _)| &s[..i]),
    }
}

/// Lakimi sipas mbaresës, për emrat që mungojnë në leksik: mbaresa më e gjatë që përputhet.
/// Rrënja para mbaresës duhet të ketë të paktën dy shkronja dhe një zanore; lema duhet të jetë
/// fjalë e vetme me shkronja të vogla të alfabetit të shqipes. Mbaresa hyn kur kalon dy porta,
/// të dyja në qelizat jo-identike (pa emëroren e kallëzoren e pashquar të njëjësit):
/// (1) leksemat standarde të leksikut që mbarojnë njësoj japin të njëjtat trajta me kufi të
/// poshtëm Wilson 95% ≥ 0.95, (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n), z = 1.96;
/// (2) saktësia në gjysmën Dev është ≥ 0.95. Mbeten jashtë njëjësi i «-k» (0.946), «-se»
/// (0.944), «-ëri» (0.943) dhe shumësi i «-ës», «-ues», «-tor».
pub fn decline_shape(lemma: &str, cell: NounCell) -> Option<Form> {
    let word = albanian(lemma);
    let table = match cell.number {
        Number::Sing => SG_SHAPES,
        Number::Plur => PL_SHAPES,
    };
    let shape = table
        .iter()
        .filter(|s| word && lemma.strip_suffix(s.suffix).is_some_and(|root| root.chars().count() >= 2 && root.contains(VOWEL)))
        .max_by_key(|s| s.suffix.chars().count())?;
    let stems: Vec<Vec<String>> = (0..shape.from)
        .map(|_| Vec::new())
        .chain(shape.stems.iter().map(|(n, add)| cut(lemma, *n).map(|b| vec![format!("{b}{add}")]).unwrap_or_default()))
        .collect();
    let class = class_of(shape.class)?;
    decline_with(&stems, &[class], cell).map(|f| Form { rule: shape.rule, ..f })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gramatika::leksiku::parse_line;

    fn row(lemma: &str, gender: &str, status: &str, pars: &str, stems: &str) -> String {
        format!("{lemma}\t{lemma}\tNOUN\tn\t{gender}\tinanim\t-\t-\t-\t-\t{status}\t{pars}\t{stems}\t-\t-\t-\t-\t-")
    }

    fn lexeme(lemma: &str, gender: &str, pars: &str, stems: &str) -> Lexeme {
        parse_line(&row(lemma, gender, "std", pars, stems)).unwrap()
    }

    fn cell(case: Case, number: Number, definite: Definite) -> NounCell {
        NounCell { case, number, definite }
    }

    fn text(lex: &Lexeme, case: Case, number: Number, definite: Definite) -> String {
        decline(lex, cell(case, number, definite)).unwrap().text
    }

    #[test]
    fn masculine_classes_with_i_and_u() {
        let mal = lexeme("mal", "mf", "1a,5a", "male.|mal.");
        assert_eq!(text(&mal, Case::Nom, Number::Sing, Def), "mali");
        assert_eq!(text(&mal, Case::Dat, Number::Sing, Ind), "mali");
        assert_eq!(text(&mal, Case::Gen, Number::Sing, Def), "malit");
        assert_eq!(text(&mal, Case::Acc, Number::Sing, Def), "malin");
        assert_eq!(text(&mal, Case::Nom, Number::Plur, Ind), "male");
        assert_eq!(text(&mal, Case::Nom, Number::Plur, Def), "malet");
        assert_eq!(text(&mal, Case::Dat, Number::Plur, Def), "maleve");
        let kale = lexeme("kalë", "m", "1b,5a", "kuaj.|kalë.|kal.");
        assert_eq!(text(&kale, Case::Nom, Number::Sing, Def), "kali");
        assert_eq!(text(&kale, Case::Acc, Number::Sing, Ind), "kalë");
        assert_eq!(decline_raw(&kale, cell(Case::Nom, Number::Plur, Def)).unwrap().text, "kuajt");
        let zog = lexeme("zog", "m", "2a,5b", "zogj.|zog.");
        assert_eq!(text(&zog, Case::Nom, Number::Sing, Def), "zogu");
        assert_eq!(text(&zog, Case::Abl, Number::Sing, Def), "zogut");
        assert_eq!(decline_raw(&zog, cell(Case::Acc, Number::Plur, Def)).unwrap().text, "zogjtë");
        let njeri = lexeme("njeri", "m", "2a,5c", "njerëz.|njeri.");
        assert_eq!(decline_raw(&njeri, cell(Case::Nom, Number::Plur, Def)).unwrap().text, "njerëzit");
        assert_eq!(decline_raw(&njeri, cell(Case::Abl, Number::Plur, Ind)).unwrap().text, "njerëzish");
    }

    #[test]
    fn plural_only_for_groups_that_pass_dev() {
        assert_eq!(formant("mal", "male"), "+e");
        assert_eq!(formant("gur", "gurë"), "+ë");
        assert_eq!(formant("lis", "lisa"), "+a");
        assert_eq!(formant("vajzë", "vajza"), "ë>a");
        assert_eq!(formant("lule", "lule"), "=");
        assert_eq!(formant("motër", "motra"), "*");
        assert_eq!(formant("zog", "zogj"), "*");
        let gur = lexeme("gur", "m", "1a,5a", "gurë.|gur.");
        assert_eq!(text(&gur, Case::Nom, Number::Sing, Def), "guri");
        assert!(decline(&gur, cell(Case::Nom, Number::Plur, Ind)).is_none());
        let zog = lexeme("zog", "m", "2a,5b", "zogj.|zog.");
        assert!(decline(&zog, cell(Case::Nom, Number::Plur, Def)).is_none());
        let vajze = lexeme("vajzë", "f", "3a,5a", "vajza.|vajzë.|vajz.");
        assert_eq!(text(&vajze, Case::Nom, Number::Plur, Def), "vajzat");
        assert_eq!(text(&vajze, Case::Dat, Number::Plur, Ind), "vajzave");
    }

    #[test]
    fn witness_gender_errors_hold_the_singular() {
        let vetull = lexeme("vetull", "m", "1a,5a", "vetulla.|vetull.");
        assert_eq!(decline_raw(&vetull, cell(Case::Nom, Number::Sing, Def)).unwrap().text, "vetulli");
        assert!(decline(&vetull, cell(Case::Nom, Number::Sing, Def)).is_none());
        assert!(paradigm(&vetull).is_empty());
    }

    #[test]
    fn third_stem_of_3b_follows_the_orthography() {
        assert_eq!(palatal_stem("lule").as_deref(), Some("lulj"));
        assert_eq!(palatal_stem("lagje").as_deref(), Some("lagjj"));
        assert_eq!(palatal_stem("lëvizje").as_deref(), Some("lëvizj"));
        assert_eq!(palatal_stem("gjermane"), None);
        assert_eq!(palatal_stem("qye"), None);
        let lagje = lexeme("lagje", "f", "3b,5a", "lagje.|lagje.|lagjej.|lagj.//lagjj.");
        let f = decline(&lagje, cell(Case::Nom, Number::Sing, Def)).unwrap();
        assert_eq!(f.text, "lagjja");
        assert!(f.accepts("lagja"));
        let gjegje = lexeme("gjegje", "f", "3b,5a", "gjegje.|gjegje.|gjegjej.|gjegj.");
        assert!(decline(&gjegje, cell(Case::Nom, Number::Sing, Def)).is_none());
        assert_eq!(text(&gjegje, Case::Dat, Number::Sing, Def), "gjegjes");
        let rus = lexeme("rusishte", "f", "3b,5a", "rusishte.|rusishte.|rusishtej.|rusishtej.");
        assert!(decline(&rus, cell(Case::Nom, Number::Sing, Def)).is_none());
        assert_eq!(text(&rus, Case::Dat, Number::Sing, Ind), "rusishteje");
    }

    #[test]
    fn feminine_classes_take_alternations_from_stems() {
        let moter = lexeme("motër", "f", "3a,5a", "motra.|motër.|motr.");
        assert_eq!(text(&moter, Case::Nom, Number::Sing, Def), "motra");
        assert_eq!(text(&moter, Case::Dat, Number::Sing, Ind), "motre");
        assert_eq!(text(&moter, Case::Dat, Number::Sing, Def), "motrës");
        assert_eq!(text(&moter, Case::Acc, Number::Sing, Def), "motrën");
        assert_eq!(text(&moter, Case::Dat, Number::Plur, Ind), "motrave");
        let lule = lexeme("lule", "f", "3b,5a", "lule.|lule.|lulej.|lulj.");
        assert_eq!(text(&lule, Case::Nom, Number::Sing, Def), "lulja");
        assert_eq!(text(&lule, Case::Dat, Number::Sing, Ind), "luleje");
        assert_eq!(text(&lule, Case::Acc, Number::Sing, Def), "lulen");
        let shtepi = lexeme("shtëpi", "f", "3d,5b", "shtëpi.|shtëpi.");
        assert_eq!(text(&shtepi, Case::Dat, Number::Sing, Def), "shtëpisë");
        assert_eq!(text(&shtepi, Case::Acc, Number::Sing, Def), "shtëpinë");
        assert_eq!(text(&shtepi, Case::Nom, Number::Plur, Def), "shtëpitë");
        let grua = lexeme("grua", "f", "3b,5b", "gra.|grua.|gruaj.|gruaj.");
        assert_eq!(text(&grua, Case::Nom, Number::Sing, Def), "gruaja");
        assert_eq!(text(&grua, Case::Dat, Number::Sing, Def), "gruas");
        assert_eq!(decline_raw(&grua, cell(Case::Nom, Number::Plur, Def)).unwrap().text, "gratë");
        let nate = lexeme("natë", "f", "3a,5e", "net.|natë.|nat.");
        assert_eq!(text(&nate, Case::Nom, Number::Plur, Ind), "net");
        assert_eq!(text(&nate, Case::Dat, Number::Plur, Ind), "netëve");
        assert_eq!(text(&nate, Case::Nom, Number::Plur, Def), "netët");
    }

    #[test]
    fn ablative_plural_and_variants() {
        let mal = lexeme("mal", "mf", "1a,5a", "male.|mal.");
        let abl = decline(&mal, cell(Case::Abl, Number::Plur, Ind)).unwrap();
        assert_eq!(abl.text, "malesh");
        assert!(abl.accepts("maleve"));
        let def = decline(&mal, cell(Case::Nom, Number::Plur, Def)).unwrap();
        assert!(def.accepts("malet") && def.accepts("maletë"));
        assert!(decline(&mal, cell(Case::Voc, Number::Sing, Ind)).is_none());
        let sy = lexeme("sy", "m", "1b,5b", "sy.|sy.|syr.//syn.");
        let f = decline(&sy, cell(Case::Nom, Number::Sing, Def)).unwrap();
        assert_eq!(f.text, "syri");
        assert!(f.accepts("syni"));
        assert_eq!(paradigm(&mal).len(), CELLS.len());
    }

    #[test]
    fn hold_when_the_lexeme_is_not_consistent() {
        let mass = lexeme("ujë", "n", "4a", "ujëra.|ujë.");
        assert!(decline(&mass, cell(Case::Nom, Number::Plur, Ind)).is_none());
        assert!(decline(&mass, cell(Case::Dat, Number::Sing, Ind)).is_none());
        assert_eq!(text(&mass, Case::Nom, Number::Sing, Def), "ujët");
        let missing_stem = lexeme("x", "m", "1b,5a", "xa.|x.");
        assert!(decline(&missing_stem, cell(Case::Nom, Number::Sing, Def)).is_none());
        let not_lemma = lexeme("dhallë", "m", "1a", "dhallë.|dhall.");
        assert!(paradigm(&not_lemma).is_empty());
        let extra_stems = lexeme("ame", "f", "3a,5a", "ame.|ame.|amej.|amj.");
        assert!(paradigm(&extra_stems).is_empty());
        let hiatus = lexeme("thopërç", "m", "1a,5c", "thopërçë.|thopërç.");
        assert!(decline(&hiatus, cell(Case::Nom, Number::Plur, Def)).is_none());
        assert_eq!(text(&hiatus, Case::Nom, Number::Sing, Def), "thopërçi");
        let adj = lexeme("mirë", "m", "adj-1-1", "mirë.");
        assert!(paradigm(&adj).is_empty());
    }

    #[test]
    fn lexeme_choice_and_homonyms() {
        let text = [
            row("ujë", "n", "std", "4a", "ujëra.|ujë."),
            row("ujë", "mf", "std", "1b,5a", "ujëra.|ujë.|uj."),
            row("bar", "mf", "std", "1a,5a", "barëra.|bar."),
            row("bar", "mf", "std", "1a,5a", "bare.|bar."),
        ]
        .join("\n");
        let lx = Lexicon::parse(&text);
        let at = |l: &str, c| decline_lemma(&lx, l, c).map(|f| f.text);
        assert_eq!(at("ujë", cell(Case::Nom, Number::Sing, Def)).as_deref(), Some("uji"));
        assert_eq!(at("ujë", cell(Case::Nom, Number::Plur, Def)), None);
        assert_eq!(at("bar", cell(Case::Nom, Number::Sing, Def)).as_deref(), Some("bari"));
        assert_eq!(at("bar", cell(Case::Nom, Number::Plur, Ind)), None);
        assert_eq!(at("mendim", cell(Case::Nom, Number::Plur, Def)).as_deref(), Some("mendimet"));
        assert_eq!(paradigm_lemma(&lx, "ujë").len(), 10);
    }

    #[test]
    fn plural_only_homonym_is_a_second_witness() {
        let one = row("xham", "m", "std", "1a,5a", "xhame.|xham.");
        let twin = row("xham", "mf", "std", "5a", "xhama.");
        let alone = Lexicon::parse(&one);
        let both = Lexicon::parse(&[one.clone(), twin].join("\n"));
        let nom_pl = cell(Case::Nom, Number::Plur, Ind);
        assert_eq!(decline_lemma(&alone, "xham", nom_pl).map(|f| f.text).as_deref(), Some("xhame"));
        assert_eq!(decline_lemma(&both, "xham", nom_pl), None);
        assert_eq!(decline_lemma(&both, "xham", cell(Case::Nom, Number::Sing, Def)).map(|f| f.text).as_deref(), Some("xhami"));
        assert_eq!(decline_lemma(&both, "xhama", nom_pl), None);
    }

    #[test]
    fn shape_fallback_for_reliable_endings() {
        let t = |l: &str, case, number, definite| decline_shape(l, cell(case, number, definite)).map(|f| f.text);
        assert_eq!(t("mendim", Case::Nom, Number::Sing, Def).as_deref(), Some("mendimi"));
        assert_eq!(t("mendim", Case::Nom, Number::Plur, Def).as_deref(), Some("mendimet"));
        assert_eq!(t("lëvizje", Case::Nom, Number::Sing, Def).as_deref(), Some("lëvizja"));
        assert_eq!(t("lëvizje", Case::Dat, Number::Sing, Ind).as_deref(), Some("lëvizjeje"));
        assert_eq!(t("lëvizje", Case::Nom, Number::Plur, Def).as_deref(), Some("lëvizjet"));
        assert_eq!(t("mësues", Case::Nom, Number::Plur, Def), None);
        assert_eq!(t("mësues", Case::Nom, Number::Sing, Def).as_deref(), Some("mësuesi"));
        assert_eq!(t("mbretëreshë", Case::Nom, Number::Sing, Def).as_deref(), Some("mbretëresha"));
        assert_eq!(t("kërkesë", Case::Dat, Number::Sing, Def).as_deref(), Some("kërkesës"));
        assert_eq!(t("kërkesë", Case::Nom, Number::Plur, Def).as_deref(), Some("kërkesat"));
        assert_eq!(t("realizëm", Case::Nom, Number::Plur, Ind).as_deref(), Some("realizma"));
        assert_eq!(t("mirësi", Case::Acc, Number::Sing, Def).as_deref(), Some("mirësinë"));
        assert_eq!(t("luftëtar", Case::Nom, Number::Plur, Ind).as_deref(), Some("luftëtarë"));
        assert_eq!(t("fshatar", Case::Nom, Number::Sing, Def).as_deref(), Some("fshatari"));
        assert_eq!(t("armik", Case::Nom, Number::Sing, Def), None);
        assert_eq!(t("nxënëse", Case::Nom, Number::Sing, Def), None);
        assert_eq!(t("vajzë", Case::Nom, Number::Plur, Ind), None);
        assert_eq!(t("realizëm", Case::Nom, Number::Sing, Def), None);
        assert_eq!(t("dhe", Case::Nom, Number::Plur, Ind), None);
        assert_eq!(t("im", Case::Nom, Number::Sing, Def), None);
        assert_eq!(t("Mendim", Case::Nom, Number::Sing, Def), None);
        assert_eq!(t("mal", Case::Nom, Number::Sing, Def), None);
        assert_eq!(t("", Case::Nom, Number::Sing, Def), None);
        assert!(decline_shape("mendim", cell(Case::Nom, Number::Sing, Def)).unwrap().rule.contains("-im"));
    }
}
