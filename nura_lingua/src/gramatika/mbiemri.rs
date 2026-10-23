//! Mbiemri dhe nyja e përparme (nyja lidhore) e shqipes standarde.
//!
//! Klasat e mbiemrit, temat dhe tabela e nyjës vijnë nga uniparser-grammar-albanian 2.3.0, MIT
//! (Morozova, Rusakov, Arkhangelskiy): paradigmat «adj-1-1» … «adj-3-2», «zi», «ri», «keq»,
//! «madh», «vogël», «plak», «trim», «turk», «gege», «zoti», «inatçi», «budalla», «fukara»,
//! «unchangeable» dhe «article». Nga paradigma merren vetëm katër qelizat e pajtimit («m,sg»,
//! «f,sg», «m,pl», «f,pl»), jo qelizat me rasë të mbiemrit të emërzuar (te «adj-1-5» qeliza
//! «m,pl» është lema, fjalorë dygjuhësh; «m,pl,nom,indef» = «-a» është trajta e emërzuar).
//! Pjesorja e foljes kontrollohet me temën dhe mbaresën e klasës së foljes, nga i njëjti burim.
//!
//! Vetëm fjala që leksiku e njeh si mbiemër, ose që leksiku s'e ka fare, lakohet si mbiemër:
//! emri i leksikut (flamur, spital, nxënës) nuk merr trajta mbiemri me hamendje.
//!
//! Rendi i dëshmisë për trajtën e mbiemrit, i fiksuar:
//! 1. R-MB1, R-MB2, R-MB3: klasa e paradigmës së leksemës, qeliza (gjini, numër) = tema e n-të
//!    + mbaresa; tema e parë duhet të riprodhojë lemën, përndryshe qeliza është None dhe mbiemri
//!    i leksikut nuk kalon te rregullat e mbarimit;
//! 2. R-MB7: dëshmi kundërshtuese brenda leksemës: klasa «adj-2-1» nuk i ndan mbiemrat në «-më»
//!    që e bëjnë femëroren me «-me» (i rremë, e rreme) nga ata që e ruajnë «-më»-në (i murrmë,
//!    e murrmë), prandaj lexohen edhe sipas «adj-1-1»; edhe huazimi i ri i paprejardhur në klasë
//!    me nyjë (bezhë) lexohet edhe si i pandryshueshëm; qeliza jepet vetëm kur leximet pajtohen;
//! 3. R-MB6: kompozita që mungon në leksik merr lakimin e pjesës së dytë, kur pjesa e parë
//!    është emër i leksikut ose parashtesë (buzë|madh → buzëmadhe, buzëmëdhenj, buzëmëdha);
//! 4. R-MB4, R-MB5: mbarimi i lemës, vetëm për mbarime të qëndrueshme («-shëm», «-ës», «-ik»,
//!    «-or», «-ar», «-ët», «-ë» ...); pjesoret «-uar», «-yer», «-ur» vetëm kur folja është në leksik;
//! 5. kur asnjë nuk vlen, kthehet None: mbiemri nuk lakohet me hamendje.
//!
//! Rendi i dëshmisë për nyjën e mbiemrit:
//! 1. R-NY1: klasa e paradigmës (adj-2-*, adj-1-2-a, madh, zi ... me nyjë; adj-3-*, adj-1-5,
//!    trim, plak ... pa nyjë; klasat që nuk pajtohen japin None);
//! 2. R-NY2: ndërtimi i lemës, kur klasa merr nyjë: parashtesa dhe ndajfolja e përemri si pjesë
//!    e parë (i stërmadh, i pacaktuar, i patëmetë, i vetëquajtur, i bashkëprodhuar,
//!    i ashtuquajtur, i parëlindur), pjesorja e foljes (i anuluar) dhe baza + prapashtesë
//!    (i rrumbullakët, i drunjësuar) e ruajnë nyjën; kompozita emër + mbiemër i nyjshëm del pa
//!    nyjë (zemër|madh, flokë|verdhë, ndjell|a|keq, goj|ëmbël); None kur dy analiza
//!    kundërshtohen (dorë|shtrënguar dhe pjesore e «dorështrëngoj»; dorë|hekurt dhe dorëhekur +
//!    «-t»), kur pjesa e parë e panjohur ka të paktën katër shkronja, ose tre dhe është fjalë e
//!    leksikut, dhe kur lema është emër + emër (faqe|mollë), që klasa e leksikut e jep me nyjë;
//! 3. R-NY5: mbiemri i pandryshueshëm del pa nyjë (gri, blu), përveç atij me parashtesë para
//!    fjalës së leksikut («pa» + emër); parashtesa pa bazë në leksik jep None;
//! 4. R-NY4: kompozita jashtë leksikut ndjek pjesën e dytë;
//! 5. R-NY3: mbarimi i lemës; përndryshe None.

use crate::gramatika::leksiku::{Lexeme, Lexicon};
use crate::gramatika::tipet::{Case, Definite, Form, Gender, Number, Pos};

/// Pozicioni i nyjës ndaj fjalës që përcakton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Linking {
    /// Menjëherë pas emrit (librin e djalit, vajzës së mirë).
    Contact,
    /// E ndarë nga emri ose pas emrit të pashquar (librin tim të ri, djem të mirë).
    Distant,
    /// Para mbiemrit të paravendosur ose të emërzuar (të njëjtën, së bukurës).
    Prenominal,
}

/// Shtylla e tabelës: «ngjitur» vetëm pas emrit të shquar; «para» vetëm para fjalës së shquar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Contact,
    Distant,
    Prenominal,
}

fn column(head_definite: Definite, context: Linking) -> Column {
    match (head_definite, context) {
        (Definite::Def, Linking::Contact) => Column::Contact,
        (Definite::Def, Linking::Prenominal) => Column::Prenominal,
        _ => Column::Distant,
    }
}

/// Rasat që nyja i dallon: emërore (me thirroren), kallëzore, gjinore-dhanore dhe rrjedhore
/// (rrjedhoren e dallon vetëm asnjanësja).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtCase {
    Nom,
    Acc,
    Oblique,
    Abl,
}

fn art_case(case: Case) -> ArtCase {
    match case {
        Case::Nom | Case::Voc => ArtCase::Nom,
        Case::Acc => ArtCase::Acc,
        Case::Gen | Case::Dat => ArtCase::Oblique,
        Case::Abl => ArtCase::Abl,
    }
}

/// Nyja e përparme e mbiemrit të nyjshëm dhe e gjinores. Gjinia, numri dhe rasa janë të emrit
/// që përcaktohet (te gjinorja: të emrit zotërues, jo të gjinores).
///
/// Tabela standarde («ngjitur» = menjëherë pas emrit të shquar; «larg» = pas emrit të pashquar
/// ose e ndarë prej tij; «para» = para mbiemrit të shquar të paravendosur ose të emërzuar):
///
/// | numri, gjinia | rasa                          | ngjitur | larg | para |
/// |---------------|-------------------------------|---------|------|------|
/// | njëjës, m     | emërore, thirrore             | i       | i    | i    |
/// | njëjës, m     | kallëzore                     | e       | të   | të   |
/// | njëjës, m     | gjinore, dhanore, rrjedhore   | të      | të   | të   |
/// | njëjës, f     | emërore, thirrore             | e       | e    | e    |
/// | njëjës, f     | kallëzore                     | e       | të   | të   |
/// | njëjës, f     | gjinore, dhanore, rrjedhore   | së      | të   | së   |
/// | shumës        | emërore, kallëzore, thirrore  | e       | të   | të   |
/// | shumës        | gjinore, dhanore, rrjedhore   | të      | të   | të   |
/// | njëjës, a     | emërore, kallëzore            | e       | të   | të   |
/// | njëjës, a     | gjinore, dhanore              | të      | të   | të   |
/// | njëjës, a     | rrjedhore                     | —       | të   | të   |
///
/// Shembuj: djali i mirë, djalin e mirë, djalit të mirë; vajza e mirë, vajzës së mirë,
/// një vajze të mirë; djem të mirë, djemtë e mirë; libri i djalit, librin e djalit,
/// librit të djalit, shtëpisë së djalit, librat e djalit, libra të djalit.
/// Rrjedhorja asnjanëse ngjitur ka dy trajta në burim, prandaj kthehet None.
/// Shtylla «para» jep vetëm nyjën: trajta e shquar e mbiemrit të emërzuar (së mirës, të mirin)
/// nuk ndërtohet këtu.
pub fn article(gender: Gender, number: Number, case: Case, head_definite: Definite, context: Linking) -> Option<&'static str> {
    let col = column(head_definite, context);
    match (number, gender, art_case(case), col) {
        (Number::Plur, _, ArtCase::Nom | ArtCase::Acc, Column::Contact) => Some("e"),
        (Number::Plur, _, _, _) => Some("të"),
        (Number::Sing, Gender::Masc, ArtCase::Nom, _) => Some("i"),
        (Number::Sing, Gender::Fem, ArtCase::Nom, _) => Some("e"),
        (Number::Sing, Gender::Masc | Gender::Fem, ArtCase::Acc, Column::Contact) => Some("e"),
        (Number::Sing, Gender::Masc | Gender::Fem, ArtCase::Acc, _) => Some("të"),
        (Number::Sing, Gender::Masc, _, _) => Some("të"),
        (Number::Sing, Gender::Fem, _, Column::Distant) => Some("të"),
        (Number::Sing, Gender::Fem, _, _) => Some("së"),
        (Number::Sing, Gender::Neut, ArtCase::Nom | ArtCase::Acc, Column::Contact) => Some("e"),
        (Number::Sing, Gender::Neut, ArtCase::Abl, Column::Contact) => None,
        (Number::Sing, Gender::Neut, _, _) => Some("të"),
    }
}

/// A merr mbiemri nyjë të përparme, sipas cilës dëshmi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Articulation {
    pub takes: bool,
    pub rule: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takes {
    Yes,
    No,
    /// Klasa «adj-1-2»: «-m» merr nyjë (i afërm, i sipërm), «-s» jo (punonjës, mësues).
    ByEnding,
    /// Klasa e pandryshueshme: pa nyjë (gri, blu), përveç parashtesës para fjalës së leksikut
    /// («pa» + emër).
    Invariant,
    Unknown,
}

/// Klasa e mbiemrit: qelizat e pajtimit (m. nj., f. nj., m. sh., f. sh.) si (tema, mbaresa).
struct Class {
    name: &'static str,
    cells: [(usize, &'static str); 4],
    takes: Takes,
    rule: &'static str,
}

const CLASSES: &[Class] = &[
    Class { name: "adj-1-1", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::Yes, rule: "R-MB1 adj-1-1: tema e dytë + «e» në femërore" },
    Class { name: "adj-1-1-a", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::Unknown, rule: "R-MB1 adj-1-1-a: tema e dytë + «e» në femërore" },
    Class { name: "adj-1-1-b", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::Unknown, rule: "R-MB1 adj-1-1-b: tema e dytë + «e» në femërore" },
    Class { name: "adj-1-2", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::ByEnding, rule: "R-MB1 adj-1-2: tema e dytë + «e» në femërore" },
    Class { name: "adj-1-2-a", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::Yes, rule: "R-MB1 adj-1-2-a: «-shëm» → «-shme» në femërore" },
    Class { name: "adj-1-3", cells: [(0, ""), (0, "e"), (0, ""), (0, "e")], takes: Takes::Yes, rule: "R-MB1 adj-1-3: tema + «e» në femërore" },
    Class { name: "adj-1-4", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::Yes, rule: "R-MB1 adj-1-4: «-ëm» → «-me» në femërore" },
    Class { name: "adj-1-4-a", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::Yes, rule: "R-MB1 adj-1-4-a: tema e dytë + «e» në femërore" },
    Class { name: "adj-1-5", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::No, rule: "R-MB1 adj-1-5: «-ësh» → «-ëshe» në femërore, mashkullorja shumës = lema (qeliza «m,pl»)" },
    Class { name: "adj-2-1", cells: [(0, ""), (0, ""), (0, ""), (1, "a")], takes: Takes::Yes, rule: "R-MB1 adj-2-1: «-ë» → «-a» në femërore shumës" },
    Class { name: "adj-2-1-a", cells: [(0, ""), (0, ""), (0, ""), (1, "a")], takes: Takes::Yes, rule: "R-MB1 adj-2-1-a: «-ë» → «-a» në femërore shumës" },
    Class { name: "adj-2-2", cells: [(0, ""), (0, ""), (0, ""), (1, "a")], takes: Takes::Yes, rule: "R-MB1 adj-2-2: tema e dytë + «a» në femërore shumës" },
    Class { name: "adj-2-2-a", cells: [(0, ""), (0, ""), (0, ""), (0, "a")], takes: Takes::Yes, rule: "R-MB1 adj-2-2-a: tema + «a» në femërore shumës" },
    Class { name: "adj-2-3", cells: [(0, ""), (0, ""), (0, ""), (0, "a")], takes: Takes::Yes, rule: "R-MB1 adj-2-3: tema + «a» në femërore shumës" },
    Class { name: "adj-3-1", cells: [(0, ""), (0, "e"), (0, "ë"), (0, "e")], takes: Takes::No, rule: "R-MB1 adj-3-1: «-e» femërore, «-ë» mashkullore shumës" },
    Class { name: "adj-3-2", cells: [(0, ""), (0, "e"), (0, "ë"), (0, "e")], takes: Takes::No, rule: "R-MB1 adj-3-2: «-e» femërore, «-ë» mashkullore shumës" },
    Class { name: "zi", cells: [(0, ""), (1, "ë"), (2, ""), (3, "")], takes: Takes::Yes, rule: "R-MB2 zi: katër tema (zi, zezë, zinj, zeza)" },
    Class { name: "ri", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::Yes, rule: "R-MB2 ri: katër tema (ri, re, rinj, reja)" },
    Class { name: "keq", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::Yes, rule: "R-MB2 keq: katër tema (keq, keqe, këqij, këqija)" },
    Class { name: "madh", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::Yes, rule: "R-MB2 madh: katër tema (madh, madhe, mëdhenj, mëdha)" },
    Class { name: "vogël", cells: [(0, ""), (0, ""), (2, ""), (4, "")], takes: Takes::Yes, rule: "R-MB2 vogël: tema e tretë dhe e pesta në shumës (vegjël, vogla)" },
    Class { name: "zoti", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::Yes, rule: "R-MB2 zoti: katër tema (zoti, zonja, zotë, zonja)" },
    Class { name: "plak", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::No, rule: "R-MB2 plak: katër tema (plak, plakë, pleq, plaka)" },
    Class { name: "trim", cells: [(0, ""), (0, "e"), (0, "a"), (0, "e")], takes: Takes::No, rule: "R-MB2 trim: «-e» femërore, «-a» mashkullore shumës" },
    Class { name: "turk", cells: [(0, ""), (0, "e"), (1, ""), (0, "e")], takes: Takes::No, rule: "R-MB2 turk: tema e dytë në mashkullore shumës (turq)" },
    Class { name: "gege", cells: [(0, ""), (1, "e"), (0, ""), (1, "e")], takes: Takes::No, rule: "R-MB2 gege: «-ë» → «-e» në femërore" },
    Class { name: "inatçi", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::No, rule: "R-MB2 inatçi: katër tema" },
    Class { name: "budalla", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::No, rule: "R-MB2 budalla: katër tema" },
    Class { name: "fukara", cells: [(0, ""), (1, ""), (2, ""), (3, "")], takes: Takes::No, rule: "R-MB2 fukara: katër tema" },
    Class { name: "unchangeable", cells: [(0, ""), (0, ""), (0, ""), (0, "")], takes: Takes::Invariant, rule: "R-MB3 e pandryshueshme" },
];

fn class_of(name: &str) -> Option<&'static Class> {
    CLASSES.iter().find(|c| c.name == name.trim())
}

fn cell_index(gender: Gender, number: Number) -> Option<usize> {
    match (gender, number) {
        (Gender::Masc, Number::Sing) => Some(0),
        (Gender::Fem, Number::Sing) => Some(1),
        (Gender::Masc, Number::Plur) => Some(2),
        (Gender::Fem, Number::Plur) => Some(3),
        (Gender::Neut, _) => None,
    }
}

/// Qeliza nga temat: tema e parë zëvendësohet me lemën, që duhet të jetë një nga variantet e saj.
fn from_stems(lemma: &str, stems: &[Vec<String>], class: &Class, cell: usize) -> Option<Form> {
    let first = stems.first()?;
    first.iter().any(|v| v == lemma).then_some(())?;
    let (n, ending) = class.cells.get(cell).copied()?;
    let variants = stems.get(n)?;
    let head = match n {
        0 => lemma.to_string(),
        _ => variants.first()?.clone(),
    };
    let text = format!("{head}{ending}");
    let rest: Vec<String> = variants.iter().map(|v| format!("{v}{ending}")).filter(|v| *v != text).collect();
    Some(Form { text, variants: rest, rule: class.rule })
}

/// Klasat e mbiemrit të leksemës; leksema që nuk është mbiemër nuk ka klasë mbiemri.
fn known_classes(lx: &Lexeme) -> Vec<&'static Class> {
    match lx.pos {
        Pos::Adj => lx.paradigms.iter().filter_map(|p| class_of(p)).collect(),
        _ => Vec::new(),
    }
}

fn has_flag(lx: &Lexeme, flag: &str) -> bool {
    lx.flags.iter().any(|f| f.trim() == flag)
}

/// Huazim i ri i paprejardhur (shenja «new» pa «deriv» në leksik): mbiemrat e tillë janë të
/// panyjshëm dhe të pandryshueshëm ose të klasës «adj-3».
fn new_loan(lx: &Lexeme) -> bool {
    has_flag(lx, "new") && !has_flag(lx, "deriv")
}

/// R-MB7: klasat rivale kur leksema mban dëshmi kundërshtuese.
/// «-më» në «adj-2-1»: një pjesë e mbiemrave në «-më» e bën femëroren me «-me» (i rremë, e rreme;
/// i pasmë, e pasme), një pjesë e ruan «-më»-në (i murrmë, e murrmë); klasa nuk i ndan, prandaj
/// shtohet leximi «adj-1-1». Huazimi i ri i paprejardhur në klasë me nyjë: lexohet edhe si i pandryshueshëm.
fn rivals(lx: &Lexeme, classes: &[&'static Class]) -> Vec<&'static Class> {
    let soft = lx.lemma.ends_with("më") && classes.iter().any(|c| c.name.starts_with("adj-2-1"));
    let loan = new_loan(lx) && classes.iter().any(|c| c.takes == Takes::Yes);
    [(soft, "adj-1-1"), (loan, "unchangeable")].iter().filter(|(on, _)| *on).filter_map(|(_, name)| class_of(name)).collect()
}

/// Klasat e leksemës bashkë me klasat rivale.
fn classes_of(lx: &Lexeme) -> Vec<&'static Class> {
    let known = known_classes(lx);
    let rival = rivals(lx, &known);
    known.into_iter().chain(rival).collect()
}

/// Trajta sipas klasës së paradigmës; çdo klasë (edhe rivalja) duhet të japë të njëjtën trajtë,
/// përndryshe None.
fn by_paradigm(lx: &Lexeme, cell: usize) -> Option<Form> {
    let stems = lx.clean_stems();
    let forms: Option<Vec<Form>> = classes_of(lx).into_iter().map(|c| from_stems(&lx.lemma, &stems, c, cell)).collect();
    let forms = forms?;
    let first = forms.first()?;
    forms.iter().all(|f| f.text == first.text).then(|| first.clone())
}

type Cell = Option<(usize, &'static str)>;

/// Rregull sipas mbarimit të lemës: mbarimi, tema e dytë (heqje, shtesë), qelizat si te klasat
/// dhe a është pjesore (që kërkon foljen në leksik).
struct Shape {
    suffix: &'static str,
    stem1: (&'static str, &'static str),
    cells: [Cell; 4],
    takes: Takes,
    verbal: bool,
    rule: &'static str,
}

const S0: Cell = Some((0, ""));
const S0E: Cell = Some((0, "e"));
const S0A: Cell = Some((0, "a"));
const S0EE: Cell = Some((0, "ë"));
const S1E: Cell = Some((1, "e"));
const S1A: Cell = Some((1, "a"));

const SHAPES: &[Shape] = &[
    Shape { suffix: "shëm", stem1: ("shëm", "shm"), cells: [S0, S1E, S0, S1E], takes: Takes::Yes, verbal: false, rule: "R-MB4: «-shëm» → «-shme» (sipas lemës)" },
    Shape { suffix: "ues", stem1: ("", ""), cells: [S0, S0E, S0, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-ues» → «-uese» (sipas lemës)" },
    Shape { suffix: "ës", stem1: ("", ""), cells: [S0, S0E, S0, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-ës» → «-ëse» (sipas lemës)" },
    Shape { suffix: "ik", stem1: ("", ""), cells: [S0, S0E, S0EE, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-ik» → «-ike», «-ikë» (sipas lemës)" },
    Shape { suffix: "or", stem1: ("", ""), cells: [S0, S0E, S0EE, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-or» → «-ore», «-orë» (sipas lemës)" },
    Shape { suffix: "ar", stem1: ("", ""), cells: [S0, S0E, S0EE, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-ar» → «-are», «-arë» (sipas lemës)" },
    Shape { suffix: "al", stem1: ("", ""), cells: [S0, S0E, S0EE, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-al» → «-ale», «-alë» (sipas lemës)" },
    Shape { suffix: "iv", stem1: ("", ""), cells: [S0, S0E, S0EE, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-iv» → «-ive», «-ivë» (sipas lemës)" },
    Shape { suffix: "ist", stem1: ("", ""), cells: [S0, S0E, S0EE, S0E], takes: Takes::No, verbal: false, rule: "R-MB4: «-ist» → «-iste», «-istë» (sipas lemës)" },
    Shape { suffix: "uar", stem1: ("", ""), cells: [S0, S0, S0, S0A], takes: Takes::Yes, verbal: true, rule: "R-MB4: pjesore «-uar» e foljes së leksikut, «-a» vetëm në femërore shumës" },
    Shape { suffix: "yer", stem1: ("", ""), cells: [S0, S0, S0, S0A], takes: Takes::Yes, verbal: true, rule: "R-MB4: pjesore «-yer» e foljes së leksikut, «-a» vetëm në femërore shumës" },
    Shape { suffix: "ur", stem1: ("", ""), cells: [S0, S0, S0, S0A], takes: Takes::Yes, verbal: true, rule: "R-MB4: pjesore «-ur» e foljes së leksikut, «-a» vetëm në femërore shumës" },
    Shape { suffix: "ët", stem1: ("", ""), cells: [S0, S0, S0, S0A], takes: Takes::Yes, verbal: false, rule: "R-MB4: «-ët» i pandryshuar, «-a» në femërore shumës (sipas lemës)" },
    Shape { suffix: "më", stem1: ("", ""), cells: [None, None, None, None], takes: Takes::Unknown, verbal: false, rule: "R-MB4: «-më» pa rregull të sigurt" },
    Shape { suffix: "ë", stem1: ("ë", ""), cells: [S0, S0, S0, S1A], takes: Takes::Unknown, verbal: false, rule: "R-MB5: «-ë» i pandryshuar, «-ë» → «-a» në femërore shumës (sipas lemës)" },
];

/// Mbarimi më i gjatë që i përshtatet lemës vendos («-uar» para «-ar», «-më» para «-ë»).
fn shape_of(lemma: &str) -> Option<&'static Shape> {
    SHAPES
        .iter()
        .filter(|s| lemma.ends_with(s.suffix) && lemma.chars().count() > s.suffix.chars().count() + 1)
        .max_by_key(|s| s.suffix.chars().count())
}

/// Mbarimi i lemës; mbarimi i pjesores pranohet vetëm me dëshminë e foljes (`participle`).
fn shape_at(lemma: &str, participle: bool) -> Option<&'static Shape> {
    shape_of(lemma).filter(|s| participle || !s.verbal)
}

fn shape_form(lemma: &str, gender: Gender, number: Number, participle: bool) -> Option<Form> {
    let cell = cell_index(gender, number)?;
    let shape = shape_at(lemma, participle)?;
    let (cut, add) = shape.stem1;
    let stem1 = lemma.strip_suffix(cut).map(|b| format!("{b}{add}"))?;
    let stems = [lemma.to_string(), stem1];
    let (n, ending) = shape.cells.get(cell).copied().flatten()?;
    let head = stems.get(n)?;
    Some(Form::one(format!("{head}{ending}"), shape.rule))
}

/// Trajta e mbiemrit vetëm sipas mbarimit të lemës. Mbarimet e pjesores («-uar», «-yer», «-ur»)
/// kërkojnë foljen në leksik, prandaj këtu japin None.
pub fn adjective_by_shape(lemma: &str, gender: Gender, number: Number) -> Option<Form> {
    shape_form(lemma, gender, number, false)
}

/// Trajta e mbiemrit (pa nyjë) për gjininë dhe numrin e emrit: klasa e paradigmës së leksemës;
/// trajta e lemës vetëm kur leksema është mbiemër pa klasë; None për leksemë që s'është mbiemër.
pub fn adjective(lx: &Lexeme, gender: Gender, number: Number) -> Option<Form> {
    let cell = cell_index(gender, number)?;
    match (lx.pos == Pos::Adj, known_classes(lx).is_empty()) {
        (true, false) => by_paradigm(lx, cell),
        (true, true) => adjective_by_shape(&lx.lemma, gender, number),
        (false, _) => None,
    }
}

/// Parashtesat dhe pjesët e para ndajfoljore, përemërore e rendore që e ruajnë nyjën e mbiemrit
/// (i papunuar, i stërmadh, i paramenduar, i mosbesueshëm, i porsalindur, i vetëpunësuar,
/// i bashkëprodhuar, i ashtuquajtur, i parëlindur, i prapambetur, i lartpërmendur,
/// i poshtëshënuar, i sipërthënë); asnjëra nuk është pjesë e parë emërore e kompozitës.
const PREFIXES: &[&str] = &[
    "pa", "mos", "jo", "stër", "para", "mbi", "nën", "ndër", "kundër", "tej", "ri", "sapo", "porsa", "posa", "gjysmë", "mirë", "keq", "shumë",
    "gjithë", "pas", "për", "jashtë", "vetë", "bashkë", "ashtu", "parë", "prapa", "lart", "poshtë", "sipër",
];

/// Klasat që hyjnë si pjesë e dytë në kompozitat emër + mbiemër (zemërmadh, flokëzi, gojëhapur).
const COMPOUND_HEADS: &[&str] = &["adj-2-1", "adj-2-1-a", "adj-2-2", "adj-2-2-a", "adj-1-3", "madh", "zi", "keq", "vogël", "ri"];

/// Prapashtesat mbiemërore që, pas një baze të leksikut, e bëjnë lemën prejardhje dhe jo kompozitë
/// (rrumbullak|ët, drunj|të, hekur|t, drunj|ësuar).
const DERIVATIONAL: &[&str] = &["t", "ët", "të", "shëm", "ësuar", "ëzuar"];

/// Prapashtesat foljore e «-shëm»: nuk formojnë mbiemër nga mbiemër kompozit, prandaj baza e tyre
/// nuk krijon dyshim (kripë|suar ← krip|ësuar).
const VERBAL_SUFFIXES: &[&str] = &["shëm", "ësuar", "ëzuar"];

/// Pjesorja standarde e klasave të foljes, nga uniparser: (klasa, tema, mbaresa). Klasa me «-1»
/// në fund ka pjesoren e klasës pa të (lexoj-1 si lexoj).
const PARTICIPLES: &[(&str, usize, &str)] = &[
    ("arrij", 1, "ur"), ("beh", 0, "ur"), ("bie", 2, "rë"), ("blej", 0, "rë"), ("brej", 1, "ur"), ("bëj", 0, "rë"), ("dal", 0, "ë"),
    ("dhjes", 2, "rë"), ("dhjes", 4, "r"), ("di", 2, "ur"), ("di", 3, "ë"), ("dua", 3, "ur"), ("dua", 5, "ur"), ("dërsij", 1, "ur"),
    ("eci", 0, "ur"), ("flas", 4, "ur"), ("flas", 5, "ur"), ("fsheh", 0, "ur"), ("gjej", 1, "ur"), ("gjej", 2, "ur"), ("ha", 1, "ë"),
    ("hap", 0, "ur"), ("heq", 0, "ur"), ("humbas", 3, "ur"), ("hyj", 0, "rë"), ("jam", 3, "ë"), ("jap", 3, "ë"), ("kam", 2, "ur"),
    ("kthej", 1, "r"), ("kthej", 1, "rë"), ("laj", 0, "rë"), ("lexoj", 1, "r"), ("lexoj", 2, "ur"), ("lë", 2, "ë"), ("lë", 3, "rë"),
    ("marr", 0, "ë"), ("mat", 1, "ur"), ("mej", 1, "ur"), ("ndiej", 0, "r"), ("ndiej", 0, "rë"), ("njoh", 0, "ur"), ("pi", 0, "rë"),
    ("pres-wait", 3, "ur"), ("prier", 1, "ë"), ("rroj", 1, "ur"), ("ruaj", 2, "ur"), ("ruej", 2, "ë"), ("shkruaj", 0, "r"),
    ("shkruaj", 0, "rë"), ("shoh", 3, "rë"), ("shtie", 1, "rë"), ("shtie", 2, "ë"), ("shtie", 3, "ë"), ("shtie", 4, "rë"),
    ("shtie", 5, "ur"), ("them", 3, "ë"), ("them", 5, "ë"), ("them", 8, "ë"), ("vdes", 2, "ur"), ("vete", 2, "ur"), ("vete", 4, "ur"),
    ("vij", 2, "ur"), ("vjel", 0, "ë"), ("vras", 3, "rë"), ("zhvesh", 0, "ur"), ("zë", 2, "ë"),
];

/// Pjesoret standarde të foljes: tema e klasës + mbaresa e klasës (anulua|r, hap|ur, bë|rë, gjet|ur).
fn participles(verb: &Lexeme) -> Vec<String> {
    let stems = verb.clean_stems();
    let mut out = Vec::new();
    for p in &verb.paradigms {
        let class = p.trim().strip_suffix("-1").unwrap_or(p.trim());
        for (_, n, ending) in PARTICIPLES.iter().filter(|(c, _, _)| *c == class) {
            out.extend(stems.get(*n).into_iter().flatten().map(|s| format!("{s}{ending}")));
        }
    }
    out
}

fn class_takes(lx: &Lexeme) -> Option<Takes> {
    let all: Vec<Takes> = classes_of(lx).into_iter().map(|c| c.takes).collect();
    let first = all.first().copied()?;
    all.iter().all(|t| *t == first).then_some(first)
}

fn standard_entry(lexicon: &Lexicon, word: &str, pos: Pos) -> bool {
    lexicon.entries(word).iter().any(|l| l.pos == pos && l.standard)
}

/// Emër i plotë i leksikut: standard, jo shkurtim, jo njëkohësisht mbiemër a numëror, jo parashtesë.
fn compound_noun(lexicon: &Lexicon, x: &str) -> bool {
    let noun = lexicon
        .entries(x)
        .iter()
        .any(|l| l.pos == Pos::Noun && l.standard && !l.paradigms.iter().any(|p| p.starts_with("abbreviation")));
    let other = lexicon.standard(x, Pos::Adj).is_some() || lexicon.standard(x, Pos::Num).is_some();
    noun && !other && !PREFIXES.contains(&x)
}

/// Lema pa `k` shkronjat e fundit, me shtesë.
fn trimmed(x: &str, k: usize, add: &str) -> Option<String> {
    let (i, _) = x.char_indices().rev().nth(k.checked_sub(1)?)?;
    x.get(..i).filter(|b| b.chars().count() >= 2).map(|b| format!("{b}{add}"))
}

/// Tema e shumësit e një emri të leksikut (flokë ← flok, leshra ← lesh, drunj ← dru, halle ← hall):
/// emri kërkohet ndër lemat që dalin nga tema me heqjen e një a dy shkronjave të fundit, dhe tema
/// duhet të jetë tema e parë (e shumësit) e paradigmës «5» të tij.
fn plural_stem(lexicon: &Lexicon, x: &str) -> bool {
    let plural = |l: &Lexeme| {
        l.pos == Pos::Noun
            && l.standard
            && l.paradigms.iter().any(|p| p.starts_with('5'))
            && l.clean_stems().first().map(|s| s.iter().any(|v| v == x)).unwrap_or(false)
    };
    [1usize, 2]
        .iter()
        .flat_map(|k| ["", "ë", "e"].iter().filter_map(move |add| trimmed(x, *k, add)))
        .filter(|n| compound_noun(lexicon, n))
        .any(|n| lexicon.entries(&n).iter().any(plural))
}

/// Emër a folje e leksikut + zanorja lidhëse «a» (ndjell|a|keq, ndjell|a|zi).
fn linked(lexicon: &Lexicon, x: &str) -> bool {
    x.strip_suffix('a')
        .filter(|b| b.chars().count() >= 2)
        .map(|b| compound_noun(lexicon, b) || standard_entry(lexicon, b, Pos::Verb))
        .unwrap_or(false)
}

/// Pjesa e parë e kompozitës: emër i leksikut, tema e shumësit të tij, ose emër a folje + «a»;
/// kurrë parashtesë a pjesë e parë ndajfoljore (vetë, bashkë, parë).
fn noun_member(lexicon: &Lexicon, x: &str) -> bool {
    !PREFIXES.contains(&x) && (compound_noun(lexicon, x) || plural_stem(lexicon, x) || linked(lexicon, x))
}

/// Pjesa e parë pranohet edhe me rënien e «ë»-së para «ë»-së nistore të mbiemrit (goj|ëmbël ← gojë + ëmbël).
fn member_split(lexicon: &Lexicon, x: &str, y: &str) -> bool {
    noun_member(lexicon, x) || (y.starts_with('ë') && noun_member(lexicon, &format!("{x}ë")))
}

/// Mbiemër i leksikut me klasë që hyn si pjesë e dytë e kompozitës.
fn compound_head(lexicon: &Lexicon, y: &str) -> bool {
    let head = lexicon.standard(y, Pos::Adj).map(|lx| lx.paradigms.iter().any(|p| COMPOUND_HEADS.contains(&p.trim()))).unwrap_or(false);
    head && y.chars().count() >= 2
}

fn classed_adjective<'a>(lexicon: &'a Lexicon, y: &str) -> Option<&'a Lexeme> {
    lexicon.standard(y, Pos::Adj).filter(|lx| !known_classes(lx).is_empty() && y.chars().count() >= 2)
}

/// Baza pas parashtesës: mbiemër a emër standard i leksikut ose pjesore e foljes së leksikut, me
/// të paktën tre shkronja (pa|fund, para|menduar); jo pjesëz, pasthirrmë a lidhëz (nën|të, pas|hëm).
fn prefix_base(lexicon: &Lexicon, rest: &str) -> bool {
    let word = lexicon.entries(rest).iter().any(|l| l.standard && matches!(l.pos, Pos::Adj | Pos::Noun));
    rest.chars().count() >= 3 && (word || verbal_participle(lexicon, rest))
}

/// Parashtesë + bazë (pa|caktuar, pa|mëshirë, para|menduar, porsa|punuar, vetë|quajtur), ose
/// «pa të» + mbiemër (pa|të|metë): mbiemri ruan nyjën.
fn prefixed(lexicon: &Lexicon, lemma: &str) -> bool {
    let plain = PREFIXES.iter().filter_map(|p| lemma.strip_prefix(p)).any(|rest| prefix_base(lexicon, rest));
    let privative = lemma.strip_prefix("patë").map(|rest| standard_entry(lexicon, rest, Pos::Adj)).unwrap_or(false);
    plain || privative
}

/// Lema nis me parashtesë dhe ka të paktën dy shkronja pas saj.
fn prefix_start(lemma: &str) -> bool {
    PREFIXES.iter().filter_map(|p| lemma.strip_prefix(p)).any(|rest| rest.chars().count() >= 2)
}

/// Pjesore e një foljeje standarde të leksikut: folja gjendet nga mbarimi (anul|uar ← anuloj,
/// kth|yer ← kthej, bë|rë ← bëj, hap|ur ← hap, mbaj|tur ← mbaj) ose është folja që i jep emrin
/// klasës (pas|ur ← kam, thën|ë ← them, ardh|ur ← vij), dhe lema duhet të jetë pjesorja që jep
/// klasa e saj (gjerë nuk është pjesore e «gjej», që jep gjetur; mbarë jo e «mbaj»).
fn verbal_participle(lexicon: &Lexicon, lemma: &str) -> bool {
    const PAIRS: &[(&str, &str)] = &[
        ("uar", "oj"), ("uar", "uaj"), ("yer", "ej"), ("ur", ""), ("tur", "j"), ("tur", ""), ("rë", "j"), ("në", "j"), ("në", ""), ("ë", ""),
    ];
    let mut candidates: Vec<String> = PAIRS.iter().filter_map(|(suffix, add)| lemma.strip_suffix(suffix).map(|b| format!("{b}{add}"))).collect();
    let mut classes: Vec<&str> = PARTICIPLES.iter().map(|(c, _, _)| *c).collect();
    classes.dedup();
    candidates.extend(classes.iter().map(|c| c.to_string()));
    candidates
        .iter()
        .flat_map(|v| lexicon.entries(v).iter().filter(|l| l.pos == Pos::Verb && l.standard))
        .any(|verb| participles(verb).iter().any(|p| p == lemma))
}

/// Baza e prejardhjes që është vetë mbiemër kompozit me të njëjtën pjesë të parë
/// (dorëhekur = dorë|hekur): «-t» pas saj s'dallohet nga kompozita dorë|hekurt.
fn compound_base(lexicon: &Lexicon, base: &str, x: &str) -> bool {
    let adjective = lexicon.entries(base).iter().any(|l| l.standard && l.pos == Pos::Adj);
    let rest = base.strip_prefix(x).filter(|r| r.chars().count() >= 2);
    adjective && rest.map(|r| lexicon.entries(r).iter().any(|l| l.standard)).unwrap_or(false)
}

/// Leximi si prejardhje: bazë e leksikut më e gjatë se pjesa e parë `x` + prapashtesë mbiemërore
/// (rrumbullak|ët, drunj|ësuar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Derivation {
    /// Baza nuk është kompozitë e pjesës së parë.
    Clear,
    /// Baza është mbiemër kompozit me të njëjtën pjesë të parë (dorëhekur|t, syçelë|t).
    Doubtful,
    Absent,
}

fn derivation(lexicon: &Lexicon, lemma: &str, x: &str) -> Derivation {
    let bases: Vec<(&str, &str)> = DERIVATIONAL
        .iter()
        .filter_map(|s| lemma.strip_suffix(s).map(|b| (*s, b)))
        .filter(|(_, b)| b.chars().count() > x.chars().count())
        .filter(|(_, b)| lexicon.entries(b).iter().any(|l| l.standard) || plural_stem(lexicon, b))
        .collect();
    let clear = bases.iter().any(|(s, b)| VERBAL_SUFFIXES.contains(s) || !compound_base(lexicon, b, x));
    match (clear, bases.is_empty()) {
        (true, _) => Derivation::Clear,
        (false, false) => Derivation::Doubtful,
        (false, true) => Derivation::Absent,
    }
}

/// Emër i thjeshtë si pjesë e dytë: emër standard i leksikut me të paktën katër shkronja, pa lexim
/// mbiemri, foljeje, numërori a përemri (mollë, shqiponjë, hekur, mjaltë). Copat e shkurtra që
/// janë mbaresë pjesoreje (uri|tur, mi|tur) nuk lexohen si emër.
fn plain_noun(lexicon: &Lexicon, y: &str) -> bool {
    let entries = lexicon.entries(y);
    let noun = entries.iter().any(|l| l.pos == Pos::Noun && l.standard);
    let other = entries.iter().any(|l| matches!(l.pos, Pos::Adj | Pos::Verb | Pos::Num | Pos::Pron));
    noun && !other && y.chars().count() >= 4
}

/// Kompozitë emër + emër (faqe|mollë, sy|shqiponjë, zemër|hekur): mbiemër pa nyjë në normë, ndërsa
/// klasa e leksikut e jep me nyjë. Prejardhja nga një bazë më e gjatë (fildish|të, jo fil|dishtë)
/// nuk lexohet kështu.
fn noun_noun(lexicon: &Lexicon, lemma: &str) -> bool {
    let split = lemma
        .char_indices()
        .skip(2)
        .map(|(i, _)| lemma.split_at(i))
        .filter(|(x, y)| plain_noun(lexicon, y) && member_split(lexicon, x, y))
        .max_by_key(|(x, _)| x.chars().count());
    split.map(|(x, _)| derivation(lexicon, lemma, x) == Derivation::Absent).unwrap_or(false)
}

/// Pjesë e parë e panjohur që mund të jetë pjesë kompozite: të paktën katër shkronja, ose tre kur
/// është fjalë e leksikut. Copat më të shkurtra (vë|shtirë, th|artë, dë|lirë) janë rastësi.
fn doubtful_member(lexicon: &Lexicon, x: &str) -> bool {
    let n = x.chars().count();
    n >= 4 || (n == 3 && !lexicon.entries(x).is_empty())
}

/// Analiza e ndërtimit të lemës, për nyjën.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Analysis {
    /// Parashtesë + fjalë e leksikut.
    Prefixed,
    /// Pjesore e foljes së leksikut.
    Participle,
    /// Bazë e leksikut + prapashtesë mbiemërore, më e gjatë se pjesa e parë e kompozitës.
    Derived,
    /// Emër + mbiemër: (pjesa para mbiemrit, mbiemri).
    Compound(String, String),
    /// Emër + emër (faqe|mollë): norma e do pa nyjë, klasa e leksikut me nyjë.
    NounNoun,
    /// Dy analiza që nuk pajtohen, ose mbiemër në fund me pjesë të parë të panjohur.
    Conflict,
    Simple,
}

/// Ndarja emër + mbiemër me pjesën e parë më të gjatë (`head` zgjedh pjesët e dyta të pranueshme);
/// `loan`: leksema është huazim i ri, që nuk formon kompozita vendi. Pjesorja e foljes vjen para
/// parashtesës (poshtëruar ← poshtëroj, jo poshtë|ruar); pjesa e parë dyshkronjëshe (pe|shuar)
/// nuk qëndron përballë pjesores së foljes («peshoj»).
fn analyse(lexicon: &Lexicon, lemma: &str, head: &dyn Fn(&str) -> bool, loan: bool) -> Analysis {
    let tails: Vec<(&str, &str)> = lemma.char_indices().skip(2).map(|(i, _)| lemma.split_at(i)).filter(|(_, y)| head(y)).collect();
    let valid = tails.iter().filter(|(x, y)| member_split(lexicon, x, y)).max_by_key(|(x, _)| x.chars().count()).copied();
    let participle = verbal_participle(lexicon, lemma);
    let derived = valid.map(|(x, _)| derivation(lexicon, lemma, x)).unwrap_or(Derivation::Absent);
    let short = valid.map(|(x, _)| x.chars().count() <= 2).unwrap_or(false);
    let unexplained = || tails.iter().any(|(x, y)| !member_split(lexicon, x, y) && doubtful_member(lexicon, x));
    match (prefixed(lexicon, lemma), valid, participle, derived) {
        (_, None, true, _) => Analysis::Participle,
        (true, ..) => Analysis::Prefixed,
        (false, Some(_), _, Derivation::Clear) => Analysis::Derived,
        (false, Some(_), _, Derivation::Doubtful) => Analysis::Conflict,
        (false, Some((x, y)), false, Derivation::Absent) => Analysis::Compound(x.to_string(), y.to_string()),
        (false, Some(_), true, Derivation::Absent) => match loan || short {
            true => Analysis::Participle,
            false => Analysis::Conflict,
        },
        (false, None, false, _) => match (unexplained(), noun_noun(lexicon, lemma)) {
            (true, _) => Analysis::Conflict,
            (false, true) => Analysis::NounNoun,
            (false, false) => Analysis::Simple,
        },
    }
}

/// Kompozita emër + mbiemër i nyjshëm (zemër|madh, flokë|zi, ndjell|a|keq, goj|ëmbël), e cila del pa nyjë:
/// (pjesa para mbiemrit, mbiemri). None për lemën e parashtesuar, për pjesoren e foljes, për
/// prejardhjen nga një bazë më e gjatë dhe kur analizat kundërshtohen.
pub fn noun_adjective_compound(lexicon: &Lexicon, lemma: &str) -> Option<(String, String)> {
    let loan = lexicon.standard(lemma, Pos::Adj).map(|lx| has_flag(lx, "new")).unwrap_or(false);
    match analyse(lexicon, lemma, &|y| compound_head(lexicon, y), loan) {
        Analysis::Compound(x, y) => Some((x, y)),
        _ => None,
    }
}

const ART_CLASS: &str = "R-NY1: klasa e paradigmës merr nyjë";
const ART_NONE: &str = "R-NY1: klasa e paradigmës nuk merr nyjë";
const ART_ENDING_M: &str = "R-NY1: klasa adj-1-2 me «-m» merr nyjë";
const ART_ENDING_S: &str = "R-NY1: klasa adj-1-2 me «-s» nuk merr nyjë";
const ART_COMPOUND: &str = "R-NY2: kompozitë emër + mbiemër i nyjshëm, pa nyjë";
const ART_PREFIX: &str = "R-NY2: parashtesë a pjesë e parë ndajfoljore + mbiemër, emër a pjesore e leksikut, nyja ruhet";
const ART_PARTICIPLE: &str = "R-NY2: pjesore e foljes së leksikut, nyja ruhet";
const ART_DERIVED: &str = "R-NY2: bazë e leksikut + prapashtesë mbiemërore, nyja ruhet";
const ART_INVARIANT: &str = "R-NY5: mbiemër i pandryshueshëm, pa nyjë";
const ART_INVARIANT_PREFIX: &str = "R-NY5: mbiemër i pandryshueshëm me parashtesë, merr nyjë";
const ART_SHAPE_YES: &str = "R-NY3: mbarimi i lemës merr nyjë";
const ART_SHAPE_NO: &str = "R-NY3: mbarimi i lemës nuk merr nyjë";

/// A merr mbiemri i leksikut nyjë: klasa, pastaj ndërtimi i lemës. None për leksemë që s'është mbiemër.
pub fn articulation(lexicon: &Lexicon, lx: &Lexeme) -> Option<Articulation> {
    let yes = |rule| Some(Articulation { takes: true, rule });
    let no = |rule| Some(Articulation { takes: false, rule });
    match class_takes(lx)? {
        Takes::Yes => match analyse(lexicon, &lx.lemma, &|y| compound_head(lexicon, y), has_flag(lx, "new")) {
            Analysis::Prefixed => yes(ART_PREFIX),
            Analysis::Participle => yes(ART_PARTICIPLE),
            Analysis::Derived => yes(ART_DERIVED),
            Analysis::Compound(..) => no(ART_COMPOUND),
            Analysis::Simple => yes(ART_CLASS),
            Analysis::NounNoun | Analysis::Conflict => None,
        },
        Takes::No => no(ART_NONE),
        Takes::Invariant => match (prefixed(lexicon, &lx.lemma), prefix_start(&lx.lemma)) {
            (true, _) => yes(ART_INVARIANT_PREFIX),
            (false, true) => None,
            (false, false) => no(ART_INVARIANT),
        },
        Takes::ByEnding => match (lx.lemma.ends_with('m'), lx.lemma.ends_with('s')) {
            (true, _) => yes(ART_ENDING_M),
            (_, true) => no(ART_ENDING_S),
            _ => None,
        },
        Takes::Unknown => None,
    }
}

fn shape_articulation(lemma: &str, participle: bool) -> Option<Articulation> {
    match shape_at(lemma, participle)?.takes {
        Takes::Yes => Some(Articulation { takes: true, rule: ART_SHAPE_YES }),
        Takes::No => Some(Articulation { takes: false, rule: ART_SHAPE_NO }),
        Takes::ByEnding | Takes::Invariant | Takes::Unknown => None,
    }
}

/// A merr nyjë mbiemri jashtë leksikut, sipas mbarimit të lemës; mbarimet e pjesores kërkojnë
/// foljen në leksik, prandaj këtu japin None.
pub fn articulation_by_shape(lemma: &str) -> Option<Articulation> {
    shape_articulation(lemma, false)
}

const RULE_HEAD: &str = "R-MB6: kompozita merr lakimin e pjesës së dytë (buzë|madh → buzëmëdha)";
const ART_HEAD_PREFIX: &str = "R-NY4: parashtesë + mbiemër, nyja e pjesës së dytë";
const ART_HEAD_NONE: &str = "R-NY4: pjesa e dytë e panyjshme, pa nyjë";

/// Pjesa e dytë e lemës që është mbiemër i leksikut me klasë, kur pjesa e parë është parashtesë
/// (stër|madh) ose pjesë e parë kompozite e njohur (buzë|madh, flokë|zi, goj|ëmbël); përndryshe asgjë.
/// Pjesa e parë kthehet ashtu si shkruhet para mbiemrit, që lema = pjesa e parë + mbiemri.
pub fn compound_tail<'a>(lexicon: &'a Lexicon, lemma: &str) -> Option<(String, &'a Lexeme)> {
    let prefix = PREFIXES
        .iter()
        .filter_map(|p| lemma.strip_prefix(p).map(|y| (p.to_string(), y)))
        .find_map(|(p, y)| classed_adjective(lexicon, y).map(|lx| (p, lx)));
    let noun = || match analyse(lexicon, lemma, &|y| classed_adjective(lexicon, y).is_some(), false) {
        Analysis::Compound(x, y) => classed_adjective(lexicon, &y).map(|lx| (x, lx)),
        _ => None,
    };
    prefix.or_else(noun)
}

fn inherit(prefix: &str, head: &Lexeme, gender: Gender, number: Number) -> Option<Form> {
    let f = by_paradigm(head, cell_index(gender, number)?)?;
    let text = format!("{prefix}{}", f.text);
    let variants = f.variants.iter().map(|v| format!("{prefix}{v}")).collect();
    Some(Form { text, variants, rule: RULE_HEAD })
}

/// Nyja e kompozitës jashtë leksikut: emër + mbiemër i nyjshëm del pa nyjë; parashtesa ruan
/// nyjën e pjesës së dytë; pjesa e dytë e panyjshme mbetet e panyjshme.
fn head_articulation(prefix: &str, head: &Lexeme) -> Option<Articulation> {
    let takes = class_takes(head)?;
    match (takes, PREFIXES.contains(&prefix)) {
        (Takes::Yes, true) => Some(Articulation { takes: true, rule: ART_HEAD_PREFIX }),
        (Takes::Yes, false) => Some(Articulation { takes: false, rule: ART_COMPOUND }),
        (Takes::No, _) => Some(Articulation { takes: false, rule: ART_HEAD_NONE }),
        _ => None,
    }
}

/// Burimi i trajtave të mbiemrit të zgjidhur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Klasa e paradigmës së leksemës.
    Paradigm,
    /// Lakimi i pjesës së dytë të kompozitës.
    Head,
    /// Mbarimi i lemës; `participle`: folja e pjesores është në leksik.
    Shape { participle: bool },
    /// Leksiku e njeh fjalën, por jo si mbiemër: asnjë trajtë.
    Hold,
}

/// Mbiemri i zgjidhur nga leksiku (ose nga pjesa e dytë, ose vetëm nga lema, kur leksiku s'e ka).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjLex {
    pub lemma: String,
    pub lexeme: Option<Lexeme>,
    /// Pjesa e parë dhe leksema e pjesës së dytë, kur lema s'ka klasë mbiemri në leksik.
    pub head: Option<(String, Lexeme)>,
    pub source: Source,
    pub articulation: Option<Articulation>,
}

impl AdjLex {
    /// Rendi i dëshmisë: leksema-mbiemër me klasë (kur temat s'japin lemën, qeliza është None dhe
    /// nuk kalohet te kompozita a te mbarimi); fjalë e leksikut që s'është mbiemër → asgjë; pjesa e
    /// dytë e kompozitës; mbarimi i lemës (pjesorja vetëm me foljen në leksik). Lema kërkohet pa
    /// hapësira anash dhe me shkronja të vogla.
    pub fn resolve(lexicon: &Lexicon, lemma: &str) -> AdjLex {
        let key = lemma.trim().to_lowercase();
        let lemma = key.as_str();
        let lexeme = lexicon.standard(lemma, Pos::Adj).cloned();
        let classed = lexeme.as_ref().map(|lx| !known_classes(lx).is_empty()).unwrap_or(false);
        let foreign = lexeme.is_none() && !lexicon.entries(lemma).is_empty();
        let head = match (classed, foreign) {
            (false, false) => compound_tail(lexicon, lemma).map(|(x, lx)| (x, lx.clone())),
            _ => None,
        };
        let participle = verbal_participle(lexicon, lemma);
        let source = match (classed, foreign, head.is_some()) {
            (true, _, _) => Source::Paradigm,
            (false, true, _) => Source::Hold,
            (false, false, true) => Source::Head,
            (false, false, false) => Source::Shape { participle },
        };
        let articulation = match (&lexeme, &head, source) {
            (Some(lx), _, Source::Paradigm) => articulation(lexicon, lx),
            (_, Some((x, lx)), Source::Head) => head_articulation(x, lx),
            (_, _, Source::Shape { .. }) => shape_articulation(lemma, participle),
            _ => None,
        };
        AdjLex { lemma: key, lexeme, head, source, articulation }
    }

    pub fn form(&self, gender: Gender, number: Number) -> Option<Form> {
        match self.source {
            Source::Paradigm => self.lexeme.as_ref().and_then(|lx| by_paradigm(lx, cell_index(gender, number)?)),
            Source::Head => self.head.as_ref().and_then(|(x, lx)| inherit(x, lx, gender, number)),
            Source::Shape { participle } => shape_form(&self.lemma, gender, number, participle),
            Source::Hold => None,
        }
    }
}

/// Togfjalëshi i mbiemrit që pajtohet me emrin, me nyjën ngjitur pas emrit kur emri është i shquar:
/// «i mirë», «e mirë», «të mirë», «së mirë»; pa nyjë për mbiemrat e panyjshëm («shqiptare»).
/// `noun_gender` është gjinia e emrit në numrin e dhënë (`Lexeme::gender(number)`): emri «mf»
/// është mashkullor në njëjës dhe femëror në shumës (mendim i mirë, mendime të mira).
pub fn np_agree(noun_gender: Gender, number: Number, case: Case, noun_definite: Definite, adj: &AdjLex) -> Option<String> {
    np_agree_at(noun_gender, number, case, noun_definite, Linking::Contact, adj)
}

/// Si `np_agree`, me gjininë e marrë nga leksema e emrit për numrin e dhënë.
pub fn np_agree_lexeme(noun: &Lexeme, number: Number, case: Case, noun_definite: Definite, adj: &AdjLex) -> Option<String> {
    np_agree(noun.gender(number)?, number, case, noun_definite, adj)
}

/// Si `np_agree`, me pozicionin e nyjës të dhënë. Për `Linking::Prenominal` kthehet None: mbiemri i
/// paravendosur a i emërzuar merr trajtën e shquar (së mirës, të mirin), që ky modul nuk e ndërton.
pub fn np_agree_at(noun_gender: Gender, number: Number, case: Case, noun_definite: Definite, context: Linking, adj: &AdjLex) -> Option<String> {
    let form = adj.form(noun_gender, number)?;
    let takes = adj.articulation?.takes;
    match (context, takes) {
        (Linking::Prenominal, _) => None,
        (_, true) => article(noun_gender, number, case, noun_definite, context).map(|a| format!("{a} {}", form.text)),
        (_, false) => Some(form.text),
    }
}

/// Trajta e fjalorit: emërore e pashquar, si në kallëzuesin emëror («e madhe», «të mëdhenj»).
pub fn citation(adj: &AdjLex, gender: Gender, number: Number) -> Option<String> {
    np_agree_at(gender, number, Case::Nom, Definite::Ind, Linking::Distant, adj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gramatika::leksiku::parse_line;

    fn adj_line(lemma: &str, par: &str, stems: &str, flags: &str) -> String {
        format!("{lemma}\t{lemma}\tA\ta\t-\t-\t-\t-\t-\t{flags}\tstd\t{par}\t{stems}\t-\t-\t-\t-\t-")
    }

    fn lexeme(lemma: &str, par: &str, stems: &str) -> Lexeme {
        parse_line(&adj_line(lemma, par, stems, "alb")).unwrap()
    }

    fn noun(lemma: &str) -> String {
        format!("{lemma}\t{lemma}\tNOUN\tn\tm\t-\t-\t-\t-\talb\tstd\t1a\t{lemma}.\t-\t-\t-\t-\t-")
    }

    fn noun_pl(lemma: &str, plural: &str) -> String {
        format!("{lemma}\t{lemma}\tNOUN\tn\tm\t-\t-\t-\t-\talb\tstd\t1a,5a\t{plural}.|{lemma}.\t-\t-\t-\t-\t-")
    }

    /// Folje e klasës «lexoj» me katër temat e burimit (kërko, kërkua, kërku, kërk).
    fn verb(lemma: &str) -> String {
        let stem = lemma.strip_suffix("oj").unwrap_or(lemma);
        format!("{lemma}\t{lemma}\tV\tv\t-\t-\t-\t-\t-\talb\tstd\tlexoj\t{stem}o.|{stem}ua.|{stem}u.|{stem}.\t-\t-\t-\t-\t-")
    }

    fn verb_of(lemma: &str, class: &str, stems: &str) -> String {
        format!("{lemma}\t{lemma}\tV\tv\t-\t-\t-\t-\t-\talb\tstd\t{class}\t{stems}\t-\t-\t-\t-\t-")
    }

    fn text(lx: &Lexeme, g: Gender, n: Number) -> String {
        adjective(lx, g, n).unwrap().text
    }

    fn takes(lexicon: &Lexicon, lemma: &str) -> Option<bool> {
        AdjLex::resolve(lexicon, lemma).articulation.map(|a| a.takes)
    }

    #[test]
    fn article_table_for_adjectives() {
        use Case::*;
        use Definite::*;
        use Gender::*;
        use Linking::*;
        use Number::*;
        assert_eq!(article(Masc, Sing, Nom, Def, Contact), Some("i"));
        assert_eq!(article(Masc, Sing, Nom, Ind, Contact), Some("i"));
        assert_eq!(article(Fem, Sing, Nom, Ind, Contact), Some("e"));
        assert_eq!(article(Masc, Plur, Nom, Ind, Contact), Some("të"));
        assert_eq!(article(Fem, Plur, Nom, Def, Contact), Some("e"));
        assert_eq!(article(Masc, Sing, Dat, Def, Contact), Some("të"));
        assert_eq!(article(Fem, Sing, Dat, Def, Contact), Some("së"));
        assert_eq!(article(Fem, Sing, Dat, Ind, Contact), Some("të"));
        assert_eq!(article(Masc, Sing, Acc, Def, Contact), Some("e"));
        assert_eq!(article(Masc, Sing, Acc, Ind, Contact), Some("të"));
        assert_eq!(article(Masc, Sing, Acc, Def, Distant), Some("të"));
        assert_eq!(article(Fem, Sing, Abl, Def, Contact), Some("së"));
        assert_eq!(article(Fem, Sing, Voc, Ind, Contact), Some("e"));
    }

    #[test]
    fn article_table_for_genitives() {
        use Case::*;
        use Definite::*;
        use Gender::*;
        use Linking::*;
        use Number::*;
        // libri i djalit, librin e djalit, librit të djalit
        assert_eq!(article(Masc, Sing, Nom, Def, Contact), Some("i"));
        assert_eq!(article(Masc, Sing, Acc, Def, Contact), Some("e"));
        assert_eq!(article(Masc, Sing, Gen, Def, Contact), Some("të"));
        // shtëpia e djalit, shtëpisë së djalit
        assert_eq!(article(Fem, Sing, Nom, Def, Contact), Some("e"));
        assert_eq!(article(Fem, Sing, Gen, Def, Contact), Some("së"));
        // librat e djalit, libra të djalit, librave të djalit
        assert_eq!(article(Masc, Plur, Nom, Def, Contact), Some("e"));
        assert_eq!(article(Masc, Plur, Nom, Ind, Contact), Some("të"));
        assert_eq!(article(Masc, Plur, Dat, Def, Contact), Some("të"));
        // shtëpia e madhe e fshatit, shtëpinë e madhe të fshatit
        assert_eq!(article(Fem, Sing, Nom, Def, Distant), Some("e"));
        assert_eq!(article(Fem, Sing, Acc, Def, Distant), Some("të"));
    }

    #[test]
    fn article_before_a_definite_adjective() {
        use Case::*;
        use Definite::*;
        use Gender::*;
        use Linking::*;
        use Number::*;
        // e njëjta, të njëjtën, së njëjtës, të njëjtat
        assert_eq!(article(Fem, Sing, Nom, Def, Prenominal), Some("e"));
        assert_eq!(article(Fem, Sing, Acc, Def, Prenominal), Some("të"));
        assert_eq!(article(Fem, Sing, Dat, Def, Prenominal), Some("së"));
        assert_eq!(article(Fem, Plur, Nom, Def, Prenominal), Some("të"));
        assert_eq!(article(Neut, Sing, Abl, Def, Contact), None);
    }

    #[test]
    fn adjective_classes_from_stems() {
        use Gender::*;
        use Number::*;
        let mire = lexeme("mirë", "adj-2-1", "mirë.|mir.");
        assert_eq!(text(&mire, Fem, Sing), "mirë");
        assert_eq!(text(&mire, Masc, Plur), "mirë");
        assert_eq!(text(&mire, Fem, Plur), "mira");
        let madh = lexeme("madh", "madh", "madh.|madhe.|mëdhenj.|mëdha.");
        assert_eq!(text(&madh, Fem, Sing), "madhe");
        assert_eq!(text(&madh, Masc, Plur), "mëdhenj");
        assert_eq!(text(&madh, Fem, Plur), "mëdha");
        let zi = lexeme("zi", "zi", "zi.|zez.|zinj.|zeza.");
        assert_eq!(text(&zi, Fem, Sing), "zezë");
        assert_eq!(text(&zi, Masc, Plur), "zinj");
        let vogel = lexeme("vogël", "vogël", "vogël.|vogl.|vegjël.|vegjl.|vogla.");
        assert_eq!(text(&vogel, Fem, Sing), "vogël");
        assert_eq!(text(&vogel, Masc, Plur), "vegjël");
        assert_eq!(text(&vogel, Fem, Plur), "vogla");
        let shqiptar = lexeme("shqiptar", "adj-3-1", "shqiptar.");
        assert_eq!(text(&shqiptar, Fem, Sing), "shqiptare");
        assert_eq!(text(&shqiptar, Masc, Plur), "shqiptarë");
        let shem = lexeme("rrezikshëm", "adj-1-2-a", "rrezikshëm.|rrezikshm.");
        assert_eq!(text(&shem, Fem, Plur), "rrezikshme");
        let plak = lexeme("plak", "plak", "plak.|plakë.|pleq.|plaka.|plak.");
        assert_eq!(text(&plak, Masc, Plur), "pleq");
        let trim = lexeme("trim", "trim", "trim.|trimëresh.");
        assert_eq!(text(&trim, Masc, Plur), "trima");
        // fjalorë dygjuhësh, fjalë dygjuhëshe: qeliza «m,pl» e klasës është lema
        let dygjuhesh = lexeme("dygjuhësh", "adj-1-5", "dygjuhësh.|dygjuhësh.");
        assert_eq!(text(&dygjuhesh, Masc, Plur), "dygjuhësh");
        assert_eq!(text(&dygjuhesh, Fem, Sing), "dygjuhëshe");
        assert!(adjective(&mire, Neut, Sing).is_none());
    }

    #[test]
    fn untrusted_stems_hold_without_falling_back_to_the_shape() {
        use Gender::*;
        use Number::*;
        let lexicon = Lexicon::parse(&adj_line("amët", "adj-2-2-a", "amt.|amt.", "alb"));
        let adj = AdjLex::resolve(&lexicon, "amët");
        assert_eq!(adj.source, Source::Paradigm);
        assert!(adj.form(Fem, Plur).is_none());
        assert!(citation(&adj, Fem, Plur).is_none());
        assert!(adjective_by_shape("amët", Fem, Plur).is_some(), "vetëm mbarimi do të jepte trajtë");
    }

    #[test]
    fn lemma_is_trimmed_and_lowercased() {
        let lexicon = Lexicon::parse(&[adj_line("mirë", "adj-2-1", "mirë.|mir.", "alb"), noun_pl("motor", "motorë")].join("\n"));
        let mire = AdjLex::resolve(&lexicon, "  Mirë ");
        assert_eq!(mire.lemma, "mirë");
        assert_eq!(citation(&mire, Gender::Fem, Number::Plur).unwrap(), "të mira");
        let motor = AdjLex::resolve(&lexicon, "Motor");
        assert_eq!(motor.source, Source::Hold);
        assert!(motor.form(Gender::Fem, Number::Sing).is_none());
    }

    #[test]
    fn stems_that_do_not_give_the_lemma_are_not_trusted() {
        let bad = lexeme("amët", "adj-2-2-a", "amt.|amt.");
        assert!(by_paradigm(&bad, 3).is_none());
    }

    #[test]
    fn a_noun_of_the_lexicon_is_not_inflected_as_an_adjective() {
        use Gender::*;
        use Number::*;
        let flamur = parse_line(&noun_pl("flamur", "flamuj")).unwrap();
        assert!(adjective(&flamur, Fem, Sing).is_none());
        let lexicon = Lexicon::parse(&[noun_pl("flamur", "flamuj"), noun_pl("motor", "motorë")].join("\n"));
        for w in ["flamur", "motor"] {
            let adj = AdjLex::resolve(&lexicon, w);
            assert_eq!(adj.source, Source::Hold);
            assert!(adj.articulation.is_none());
            assert!(citation(&adj, Fem, Plur).is_none());
        }
    }

    #[test]
    fn conflicting_evidence_inside_the_lexeme_holds() {
        use Gender::*;
        use Number::*;
        // i rremë, e rreme, të rreme: «-më» e bën femëroren me «-me», jo si klasa «adj-2-1»
        let rreme = lexeme("rremë", "adj-2-1", "rremë.|rrem.");
        assert!(adjective(&rreme, Fem, Sing).is_none());
        assert!(adjective(&rreme, Fem, Plur).is_none());
        assert_eq!(text(&rreme, Masc, Plur), "rremë");
        let majme = lexeme("majmë", "adj-1-1", "majmë.|majm.");
        assert_eq!(text(&majme, Fem, Sing), "majme");
        // bezhë: huazim i ri, i pandryshueshëm e pa nyjë në normë (këpucë bezhë)
        let bezhe = adj_line("bezhë", "adj-2-1", "bezhë.|bezh.", "new");
        let lexicon = Lexicon::parse(&bezhe);
        let adj = AdjLex::resolve(&lexicon, "bezhë");
        assert!(adj.articulation.is_none());
        assert!(adj.form(Fem, Plur).is_none());
        assert_eq!(adj.form(Fem, Sing).unwrap().text, "bezhë");
    }

    #[test]
    fn invariant_adjectives() {
        let lexicon = Lexicon::parse(
            &[
                adj_line("gri", "unchangeable", "gri.", "-"),
                adj_line("pafund", "unchangeable", "pafund.", "alb"),
                adj_line("parafundit", "unchangeable", "parafundit.", "alb"),
                noun_pl("fund", "funde"),
            ]
            .join("\n"),
        );
        assert_eq!(takes(&lexicon, "gri"), Some(false));
        assert_eq!(takes(&lexicon, "pafund"), Some(true));
        assert_eq!(citation(&AdjLex::resolve(&lexicon, "pafund"), Gender::Fem, Number::Sing).unwrap(), "e pafund");
        assert_eq!(takes(&lexicon, "parafundit"), None, "parashtesë pa bazë në leksik: asnjë hamendje");
    }

    #[test]
    fn shape_rules_are_narrow() {
        use Gender::*;
        use Number::*;
        assert_eq!(adjective_by_shape("ngjitshëm", Fem, Sing).unwrap().text, "ngjitshme");
        assert_eq!(adjective_by_shape("habitor", Masc, Plur).unwrap().text, "habitorë");
        assert_eq!(adjective_by_shape("mësues", Fem, Sing).unwrap().text, "mësuese");
        assert_eq!(adjective_by_shape("bardhë", Fem, Plur).unwrap().text, "bardha");
        assert!(adjective_by_shape("kërkuar", Fem, Plur).is_none(), "pjesorja kërkon foljen në leksik");
        assert!(adjective_by_shape("jetim", Fem, Sing).is_none(), "bashkëtingëllore pa prapashtesë të njohur: asgjë me hamendje");
        assert!(adjective_by_shape("majmë", Fem, Sing).is_none());
        assert!(adjective_by_shape("vjetër", Fem, Sing).is_none());
        assert!(adjective_by_shape("huaj", Fem, Plur).is_none());
        assert!(adjective_by_shape("komun", Fem, Sing).is_none());
        assert!(articulation_by_shape("bardhë").is_none());
        assert!(articulation_by_shape("obskur").is_none());
        assert_eq!(articulation_by_shape("rrezikshëm").map(|a| a.takes), Some(true));
        assert_eq!(articulation_by_shape("mësues").map(|a| a.takes), Some(false));
    }

    #[test]
    fn participle_shape_needs_the_verb() {
        use Gender::*;
        use Number::*;
        let lexicon = Lexicon::parse(&verb("kërkoj"));
        let kerkuar = AdjLex::resolve(&lexicon, "kërkuar");
        assert_eq!(kerkuar.source, Source::Shape { participle: true });
        assert_eq!(citation(&kerkuar, Fem, Plur).unwrap(), "të kërkuara");
        let obskur = AdjLex::resolve(&lexicon, "obskur");
        assert!(obskur.form(Fem, Sing).is_none());
        assert!(obskur.articulation.is_none());
    }

    #[test]
    fn compounds_prefixes_and_participles() {
        let text = [
            noun("zemër"),
            noun("pac"),
            adj_line("madh", "madh", "madh.|madhe.|mëdhenj.|mëdha.", "alb"),
            adj_line("caktuar", "adj-2-2", "caktuar.|caktuar.", "alb"),
            adj_line("aktuar", "adj-2-2", "aktuar.|aktuar.", "alb"),
            adj_line("uluar", "adj-2-2", "uluar.|uluar.", "alb"),
            noun("an"),
            "anuloj\tanuloj\tV\tv\t-\t-\t-\t-\t-\tnew\tstd\tlexoj\tanulo.|anulua.|anulu.|anul.\t-\t-\t-\t-\t-".to_string(),
            "anuluar\tanuluar\tA\ta\t-\t-\t-\t-\t-\tnew,deriv\tstd\tadj-2-2\tanuluar.|anuluar.\t-\t-\t-\t-\t-".to_string(),
            noun("dorë"),
            verb("dorështrëngoj"),
            adj_line("shtrënguar", "adj-2-2", "shtrënguar.|shtrënguar.", "deriv"),
            adj_line("dorështrënguar", "adj-2-2", "dorështrënguar.|dorështrënguar.", "deriv"),
            noun("pun"),
            adj_line("sapunuar", "adj-2-2", "sapunuar.|sapunuar.", "deriv"),
            adj_line("punuar", "adj-2-2", "punuar.|punuar.", "deriv"),
            adj_line("porsapunuar", "adj-2-2", "porsapunuar.|porsapunuar.", "deriv"),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        assert_eq!(noun_adjective_compound(&lexicon, "zemërmadh"), Some(("zemër".to_string(), "madh".to_string())));
        assert!(noun_adjective_compound(&lexicon, "pacaktuar").is_none(), "pa + caktuar: parashtesë, jo pac + aktuar");
        assert!(noun_adjective_compound(&lexicon, "anuluar").is_none(), "pjesore e foljes anuloj, jo an + uluar");
        assert_eq!(takes(&lexicon, "anuluar"), Some(true));
        assert!(noun_adjective_compound(&lexicon, "dorështrënguar").is_none());
        assert_eq!(takes(&lexicon, "dorështrënguar"), None, "dorë|shtrënguar dhe pjesore e dorështrëngoj: dëshmi kundërshtuese");
        assert_eq!(takes(&lexicon, "porsapunuar"), Some(true), "porsa + punuar: parashtesë");
    }

    #[test]
    fn hand_checked_compounds() {
        let text = [
            noun_pl("flok", "flokë"),
            verb_of("ndjell", "vjel", "ndjell.|ndill.|ndoll.|nduall."),
            "gojë\tgojë\tNOUN\tn\tf\t-\t-\t-\t-\talb\tstd\t3a,5a\tgojë.|gojë.|goj.\t-\t-\t-\t-\t-".to_string(),
            noun_pl("rrumbull", "rrumbuj"),
            "dru\tdru\tNOUN\tn\tf\t-\t-\t-\t-\talb\tstd\t3c,5b\tdru.//drunj.|dru.|druj.\t-\t-\t-\t-\t-".to_string(),
            noun("patë"),
            adj_line("verdhë", "adj-2-1", "verdhë.|verdh.", "alb"),
            adj_line("flokëverdhë", "adj-2-1", "flokëverdhë.|flokëverdh.", "alb,deriv"),
            adj_line("keq", "keq", "keq.|keqe.|këqij.|këqija.", "alb"),
            adj_line("ndjellakeq", "keq", "ndjellakeq.|ndjellakeqe.|ndjellakëqij.|ndjellakëqija.", "alb,deriv"),
            adj_line("ëmbël", "adj-2-2", "ëmbël.|ëmbl.", "alb"),
            adj_line("gojëmbël", "adj-2-2", "gojëmbël.|gojëmbl.", "alb,deriv"),
            adj_line("rrumbullak", "adj-3-2", "rrumbullak.", "alb,deriv"),
            adj_line("akët", "adj-2-2-a", "akët.|akt.", "alb,deriv"),
            adj_line("rrumbullakët", "adj-2-2-a", "rrumbullakët.|rrumbullakt.", "alb,deriv"),
            adj_line("njësuar", "adj-2-2", "njësuar.|njësuar.", "alb,deriv"),
            adj_line("drunjësuar", "adj-2-2", "drunjësuar.|drunjësuar.", "alb,deriv"),
            adj_line("metë", "adj-2-1", "metë.|met.", "alb,deriv"),
            adj_line("patëmetë", "adj-2-1", "patëmetë.|patëmet.", "alb,deriv"),
            adj_line("madh", "madh", "madh.|madhe.|mëdhenj.|mëdha.", "alb"),
            adj_line("grekomadh", "madh", "grekomadh.|grekomadhe.|grekomëdhenj.|grekomëdha.", "alb,deriv"),
            adj_line("pasmë", "adj-2-1", "pasmë.|pasm.", "alb"),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        assert_eq!(takes(&lexicon, "flokëverdhë"), Some(false), "flokë|verdhë: tema e shumësit + mbiemër");
        assert_eq!(citation(&AdjLex::resolve(&lexicon, "flokëverdhë"), Gender::Fem, Number::Sing).unwrap(), "flokëverdhë");
        assert_eq!(takes(&lexicon, "ndjellakeq"), Some(false), "ndjell|a|keq: zanore lidhëse");
        assert_eq!(takes(&lexicon, "gojëmbël"), Some(false), "goj|ëmbël: rënia e «ë»-së");
        assert_eq!(takes(&lexicon, "rrumbullakët"), Some(true), "rrumbullak|ët: prejardhje, jo rrumbull + akët");
        assert_eq!(takes(&lexicon, "drunjësuar"), Some(true), "drunj|ësuar: prejardhje, jo dru + njësuar");
        assert_eq!(takes(&lexicon, "patëmetë"), Some(true), "pa të metë: parashtesë");
        assert_eq!(takes(&lexicon, "grekomadh"), None, "pjesa e parë e panjohur: asnjë hamendje");
        assert_eq!(takes(&lexicon, "madh"), Some(true));
        let pasme = AdjLex::resolve(&lexicon, "pasmë");
        assert!(pasme.form(Gender::Fem, Number::Sing).is_none());
        assert_eq!(citation(&pasme, Gender::Masc, Number::Sing).unwrap(), "i pasmë");
    }

    #[test]
    fn adverbial_first_parts_keep_the_article() {
        let text = [
            noun_pl("vetë", "veta"),
            noun_pl("bashkë", "bashka"),
            noun_pl("parë", "parë"),
            adj_line("quajtur", "adj-2-2", "quajtur.|quajtur.", "alb"),
            adj_line("vetëquajtur", "adj-2-2", "vetëquajtur.|vetëquajtur.", "alb,deriv"),
            adj_line("prodhuar", "adj-2-2", "prodhuar.|prodhuar.", "alb"),
            adj_line("bashkëprodhuar", "adj-2-2", "bashkëprodhuar.|bashkëprodhuar.", "alb,deriv"),
            adj_line("lindur", "adj-2-2", "lindur.|lindur.", "alb"),
            adj_line("parëlindur", "adj-2-2", "parëlindur.|parëlindur.", "alb,deriv"),
            adj_line("punësuar", "adj-2-2", "punësuar.|punësuar.", "alb"),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        // i vetëquajtur, i bashkëprodhuar, djali i parëlindur: jo kompozita emër + mbiemër
        assert_eq!(takes(&lexicon, "vetëquajtur"), Some(true));
        assert_eq!(takes(&lexicon, "bashkëprodhuar"), Some(true));
        assert_eq!(takes(&lexicon, "parëlindur"), Some(true));
        assert!(noun_adjective_compound(&lexicon, "vetëquajtur").is_none());
        let outside = AdjLex::resolve(&lexicon, "vetëpunësuar");
        assert_eq!(citation(&outside, Gender::Masc, Number::Sing).unwrap(), "i vetëpunësuar");
    }

    #[test]
    fn noun_noun_compounds_and_short_fragments() {
        let text = [
            "faqe\tfaqe\tNOUN\tn\tf\t-\t-\t-\t-\talb\tstd\t3a,5a\tfaqe.|faqe.|faq.\t-\t-\t-\t-\t-".to_string(),
            "mollë\tmollë\tNOUN\tn\tf\t-\t-\t-\t-\talb\tstd\t3a,5a\tmollë.|mollë.|moll.\t-\t-\t-\t-\t-".to_string(),
            adj_line("faqemollë", "adj-2-1", "faqemollë.|faqemoll.", "deriv"),
            noun("zemër"),
            noun_pl("hekur", "hekura"),
            adj_line("zemërhekur", "adj-2-2", "zemërhekur.|zemërhekur.", "alb,deriv"),
            noun("uri"),
            noun("tur"),
            adj_line("uritur", "adj-2-2", "uritur.|uritur.", "alb"),
            noun_pl("fildish", "fildishë"),
            noun("fil"),
            noun("dishtë"),
            adj_line("fildishtë", "adj-2-1", "fildishtë.|fildisht.", "alb,deriv"),
            verb_of("vë", "lë", "vë.|vu.|vën.|vën."),
            adj_line("shtirë", "adj-2-1", "shtirë.|shtir.", "alb"),
            adj_line("vështirë", "adj-2-1", "vështirë.|vështir.", "alb,deriv"),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        // vajzë faqemollë: emër + emër, klasa e leksikut thotë me nyjë, prandaj asnjë përgjigje
        assert_eq!(takes(&lexicon, "faqemollë"), None);
        assert_eq!(takes(&lexicon, "zemërhekur"), None);
        // i uritur: «tur» është copë e mbaresës së pjesores, jo emër i dytë
        assert_eq!(takes(&lexicon, "uritur"), Some(true));
        // i fildishtë: fildish + «-të», jo fil + dishtë
        assert_eq!(takes(&lexicon, "fildishtë"), Some(true));
        // i vështirë: «vë» nuk është pjesë e parë kompozite
        assert_eq!(takes(&lexicon, "vështirë"), Some(true));
    }

    #[test]
    fn participles_follow_the_verb_class() {
        let text = [
            verb_of("gjej", "gjej", "gje.|gjend.//gjind.//gjënd.|gjet."),
            verb_of("mbaj", "brej", "mba.|mbajt."),
            verb_of("bëj", "bëj", "bë.|bër."),
            verb_of("hap", "hap", "hap."),
            verb_of("kam", "kam", "ka.|ke.|pas.|kish.|pat.|ki.|pa."),
            noun("sur"),
            adj_line("pasur", "adj-2-2", "pasur.|pasur.", "alb"),
            "të\ttë\tPART\tpart\t-\t-\t-\t-\t-\talb\tstd\tunchangeable\ttë.\t-\t-\t-\t-\t-".to_string(),
            "hëm\thëm\tINTJ\tintj\t-\t-\t-\t-\t-\talb\tstd\tunchangeable\thëm.\t-\t-\t-\t-\t-".to_string(),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        assert!(verbal_participle(&lexicon, "gjetur"));
        assert!(!verbal_participle(&lexicon, "gjerë"), "gjej jep gjetur, jo gjerë");
        assert!(verbal_participle(&lexicon, "mbajtur"));
        assert!(!verbal_participle(&lexicon, "mbarë"), "mbaj jep mbajtur, jo mbarë");
        assert!(verbal_participle(&lexicon, "bërë"));
        assert!(verbal_participle(&lexicon, "hapur"));
        // i pasur: pjesore e «kam» (tema «pas» + «-ur»), jo pa + sur
        assert!(verbal_participle(&lexicon, "pasur"));
        let pasur = AdjLex::resolve(&lexicon, "pasur").articulation.unwrap();
        assert_eq!(pasur, Articulation { takes: true, rule: ART_PARTICIPLE });
        assert!(!prefixed(&lexicon, "nëntë"), "nën + pjesëz nuk është parashtesë + bazë");
        assert!(!prefixed(&lexicon, "pashëm"), "pas + pasthirrmë nuk është parashtesë + bazë");
    }

    #[test]
    fn derivation_from_a_compound_adjective_holds() {
        let text = [
            "dorë\tdorë\tNOUN\tn\tf\t-\t-\t-\t-\talb\tstd\t3a,5a\tduar.|dorë.|dor.\t-\t-\t-\t-\t-".to_string(),
            noun_pl("hekur", "hekura"),
            adj_line("hekurt", "adj-2-2", "hekurt.|hekurt.", "alb,deriv"),
            adj_line("dorëhekur", "adj-2-2", "dorëhekur.|dorëhekur.", "alb,deriv"),
            adj_line("dorëhekurt", "adj-2-2", "dorëhekurt.|dorëhekurt.", "alb,deriv"),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        // dorë|hekurt (pa nyjë) apo dorëhekur + «-t» (me nyjë): dy analiza, asnjë përgjigje
        assert_eq!(takes(&lexicon, "dorëhekurt"), None);
    }

    #[test]
    fn compound_outside_the_lexicon_inherits_the_head() {
        let text = [noun("buzë"), adj_line("madh", "madh", "madh.|madhe.|mëdhenj.|mëdha.", "alb")].join("\n");
        let lexicon = Lexicon::parse(&text);
        let adj = AdjLex::resolve(&lexicon, "buzëmadh");
        assert_eq!(adj.form(Gender::Fem, Number::Plur).unwrap().text, "buzëmëdha");
        assert_eq!(adj.form(Gender::Masc, Number::Plur).unwrap().rule, RULE_HEAD);
        assert_eq!(adj.articulation.map(|a| a.takes), Some(false));
        assert_eq!(citation(&adj, Gender::Masc, Number::Plur).unwrap(), "buzëmëdhenj");
        let prefixed = AdjLex::resolve(&lexicon, "stërmadh");
        assert_eq!(citation(&prefixed, Gender::Fem, Number::Sing).unwrap(), "e stërmadhe");
    }

    #[test]
    fn agreement_phrases() {
        let text = [
            noun("zemër"),
            adj_line("madh", "madh", "madh.|madhe.|mëdhenj.|mëdha.", "alb"),
            adj_line("zemërmadh", "madh", "zemërmadh.|zemërmadhe.|zemërmëdhenj.|zemërmëdha.", "alb,deriv"),
            adj_line("stërmadh", "madh", "stërmadh.|stërmadhe.|stërmëdhenj.|stërmëdha.", "alb,deriv"),
            adj_line("mirë", "adj-2-1", "mirë.|mir.", "alb"),
            adj_line("shqiptar", "adj-3-1", "shqiptar.", "alb,deriv"),
            "mendim\tmendim\tNOUN\tn\tmf\t-\t-\t-\t-\talb\tstd\t1a,5a\tmendime.|mendim.\t-\t-\t-\t-\t-".to_string(),
        ]
        .join("\n");
        let lexicon = Lexicon::parse(&text);
        let mire = AdjLex::resolve(&lexicon, "mirë");
        assert_eq!(np_agree(Gender::Masc, Number::Sing, Case::Nom, Definite::Def, &mire).unwrap(), "i mirë");
        assert_eq!(np_agree(Gender::Fem, Number::Sing, Case::Nom, Definite::Ind, &mire).unwrap(), "e mirë");
        assert_eq!(np_agree(Gender::Masc, Number::Plur, Case::Nom, Definite::Ind, &mire).unwrap(), "të mirë");
        assert_eq!(np_agree(Gender::Fem, Number::Sing, Case::Dat, Definite::Def, &mire).unwrap(), "së mirë");
        assert_eq!(np_agree(Gender::Fem, Number::Plur, Case::Acc, Definite::Def, &mire).unwrap(), "e mira");
        assert!(np_agree_at(Gender::Fem, Number::Sing, Case::Dat, Definite::Def, Linking::Prenominal, &mire).is_none());
        let mendim = lexicon.standard("mendim", Pos::Noun).unwrap();
        assert_eq!(np_agree_lexeme(mendim, Number::Plur, Case::Nom, Definite::Ind, &mire).unwrap(), "të mira");
        assert_eq!(np_agree_lexeme(mendim, Number::Sing, Case::Nom, Definite::Def, &mire).unwrap(), "i mirë");
        let madh = AdjLex::resolve(&lexicon, "madh");
        assert_eq!(citation(&madh, Gender::Masc, Number::Plur).unwrap(), "të mëdhenj");
        assert_eq!(citation(&madh, Gender::Fem, Number::Sing).unwrap(), "e madhe");
        let zemermadh = AdjLex::resolve(&lexicon, "zemërmadh");
        assert_eq!(zemermadh.articulation.unwrap().rule, ART_COMPOUND);
        assert_eq!(citation(&zemermadh, Gender::Fem, Number::Sing).unwrap(), "zemërmadhe");
        let stermadh = AdjLex::resolve(&lexicon, "stërmadh");
        assert_eq!(citation(&stermadh, Gender::Fem, Number::Plur).unwrap(), "të stërmëdha");
        let shqiptar = AdjLex::resolve(&lexicon, "shqiptar");
        assert_eq!(np_agree(Gender::Fem, Number::Sing, Case::Gen, Definite::Def, &shqiptar).unwrap(), "shqiptare");
        let unknown = AdjLex::resolve(&lexicon, "bardhë");
        assert!(np_agree(Gender::Fem, Number::Sing, Case::Nom, Definite::Def, &unknown).is_none());
    }
}
