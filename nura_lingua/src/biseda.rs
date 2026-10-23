//! Biseda: Nura lidh fjalët sipas kuptimit dhe përgjigjet me fjali të lidhura, jo si fjalor.
//!
//! Rendi i punës, i njëjtë për çdo hyrje:
//! 1. Synimi nga fjalët-shenjë: «pse» → shkaku, «ku» → vendi, «për çfarë» → përdorimi,
//!    «çfarë bën» → veprimi, «lidhje» ose dy fjalë → lidhja, një fjalë → portreti.
//! 2. Fjalët e përmbajtjes → lema e grafit (trajtat e lakuara shkojnë te lema).
//! 3. Faktet: vetëm lidhjet e dijes së përbashkët që i dëshmojnë dy dëshmitarë të pavarur,
//!    dhe kategoria që e pranon planifikuesi i fjalës.
//! 4. Rëndësia: çdo fakt merr një shkallë (`Fact::salience`) sipas rregullave të deklaruara te
//!    `weigh` — fakti për një nënlloj ose për një kuptim tjetër nuk thuhet për fjalën e zhveshur;
//!    fakti i vërtetë por rastësor («ari gjendet në dhëmb») nuk bëhet fakt kryesor.
//! 5. Plani: fjalia e parë është lidhja që kërkohet; pastaj një fakt që e shpjegon; në fund
//!    një pyetje që e çon bisedën më tej.
//! 6. Realizimi: çdo trajtë vjen nga gramatika; fjalia me një trajtë të pasigurt nuk thuhet.
//! 7. Oratoria (`crate::oratoria`): në një renditje, i pari është fakti më i rëndësishëm; të tjerët
//!    vihen nga më i shkurtri te më i gjati (ligji i Behaghel-it); kur dy fakte mund të thuhen në dy renditje, merret ajo me vlerësimin
//!    retorik më të lartë (ritmi, kadenca, baraspesha, rritja, fryma — formula të deklaruara).

use crate::discourse;
use crate::gramatika::tipet::{Case, Definite, Gender, Number};
use crate::graph::{NodeKind, Pos, Rel, Source};
use crate::morph::capitalize;
use crate::query::{tokenize, CandStatus, Engine, Focus, SenseCand};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Portrait,
    Relation,
    Cause,
    Purpose,
    Place,
    Ability,
}

impl Intent {
    pub fn code(self) -> &'static str {
        match self {
            Intent::Portrait => "PORTRAIT",
            Intent::Relation => "RELATION",
            Intent::Cause => "CAUSE",
            Intent::Purpose => "PURPOSE",
            Intent::Place => "PLACE",
            Intent::Ability => "ABILITY",
        }
    }
}

/// Llojet e lidhjeve të dijes së përbashkët që biseda di t'i thotë.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    IsA,
    CapableOf,
    UsedFor,
    AtLocation,
    HasProperty,
    HasA,
    PartOf,
    MadeOf,
    Causes,
    Desires,
    HasPrerequisite,
    MotivatedBy,
    CausesDesire,
}

impl Kind {
    pub fn from_rel(name: &str) -> Option<Kind> {
        match name {
            "CS_IS_A" => Some(Kind::IsA),
            "CS_CAPABLE_OF" => Some(Kind::CapableOf),
            "CS_USED_FOR" => Some(Kind::UsedFor),
            "CS_AT_LOCATION" | "CS_LOCATED_NEAR" => Some(Kind::AtLocation),
            "CS_HAS_PROPERTY" => Some(Kind::HasProperty),
            "CS_HAS_A" => Some(Kind::HasA),
            "CS_PART_OF" => Some(Kind::PartOf),
            "CS_MADE_OF" => Some(Kind::MadeOf),
            "CS_CAUSES" => Some(Kind::Causes),
            "CS_DESIRES" => Some(Kind::Desires),
            "CS_HAS_PREREQUISITE" => Some(Kind::HasPrerequisite),
            "CS_MOTIVATED_BY" | "CS_MOTIVATED_BY_GOAL" => Some(Kind::MotivatedBy),
            "CS_CAUSES_DESIRE" => Some(Kind::CausesDesire),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Kind::IsA => "CS_IS_A",
            Kind::CapableOf => "CS_CAPABLE_OF",
            Kind::UsedFor => "CS_USED_FOR",
            Kind::AtLocation => "CS_AT_LOCATION",
            Kind::HasProperty => "CS_HAS_PROPERTY",
            Kind::HasA => "CS_HAS_A",
            Kind::PartOf => "CS_PART_OF",
            Kind::MadeOf => "CS_MADE_OF",
            Kind::Causes => "CS_CAUSES",
            Kind::Desires => "CS_DESIRES",
            Kind::HasPrerequisite => "CS_HAS_PREREQUISITE",
            Kind::MotivatedBy => "CS_MOTIVATED_BY",
            Kind::CausesDesire => "CS_CAUSES_DESIRE",
        }
    }
}

/// Bishti i strukturuar i një lidhjeje, siç e shkruan importuesi: «v=ruaj;o=shtëpi»,
/// «v=luaj;p=me;o=lodër», «n=kafshë;a=vogël», «a=kripur», «v=vrapoj;r=shpejt». Kopula dhe
/// ndajfolja e vetme nuk thuhen.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tail {
    Noun { noun: String, adj: Option<String> },
    Verb { verb: String, prep: Option<String>, object: Option<String>, adj: Option<String>, adv: Option<String> },
    Adj(String),
}

impl Tail {
    pub fn parse(s: &str) -> Option<Tail> {
        let mut parts: BTreeMap<&str, &str> = BTreeMap::new();
        for p in s.split(';') {
            match p.split_once('=') {
                Some((k, v)) => {
                    parts.insert(k.trim(), v.trim());
                }
                None => {}
            }
        }
        let v = |k: &str| parts.get(k).copied().filter(|x| !x.is_empty()).map(str::to_string);
        match (v("v"), v("n"), v("a"), v("cop").or_else(|| v("copn"))) {
            (_, _, _, Some(_)) => None,
            (Some(verb), _, _, None) => Some(Tail::Verb { verb, prep: v("p"), object: v("o"), adj: v("a"), adv: v("r") }),
            (None, Some(noun), _, None) => Some(Tail::Noun { noun, adj: v("a") }),
            (None, None, Some(a), None) => Some(Tail::Adj(a)),
            _ => None,
        }
    }

    pub fn noun(n: &str) -> Tail {
        Tail::Noun { noun: n.to_string(), adj: None }
    }

    /// Fjala kryesore e bishtit (për krahasime dhe për pyetjen e radhës).
    pub fn key(&self) -> &str {
        match self {
            Tail::Noun { noun, .. } => noun,
            Tail::Verb { verb, .. } => verb,
            Tail::Adj(a) => a,
        }
    }

    pub fn object(&self) -> Option<&str> {
        match self {
            Tail::Verb { object, .. } => object.as_deref(),
            Tail::Noun { noun, .. } => Some(noun),
            Tail::Adj(_) => None,
        }
    }
}

/// Si thuhet kundrinori: i shquar njëjës («ruan shtëpinë»), i shquar shumës («gjuan minjtë»)
/// ose i pashquar për lëndët e pandashme («pi ujë»). Vendoset nga fjala angleze e burimit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectForm {
    DefSing,
    DefPlur,
    Mass,
}

/// Emrat anglisht të pandashëm (lëndë, ushqime, dukuri) që shqip thuhen të pashquar si kundrinor.
const MASS: &[&str] = &[
    "water", "milk", "food", "wine", "beer", "meat", "grass", "money", "coffee", "tea", "blood", "air", "music",
    "information", "salt", "sugar", "bread", "rice", "oil", "gas", "fuel", "energy", "light", "heat", "paper", "wood",
    "sand", "snow", "rain", "ice", "soup", "juice", "honey", "cheese", "butter", "flour", "medicine", "knowledge",
    "love", "time", "fun", "work", "help", "news", "advice", "electricity", "smoke", "dust", "mud", "gold", "silver",
    "iron", "cotton", "wool", "leather", "soap", "fruit", "hay", "seed", "corn", "wheat", "grain", "fish", "cream",
    "chocolate", "candy", "dirt", "garbage", "trash", "power", "fire", "noise", "sleep", "exercise", "rest",
];

const PLURAL_IRREGULAR: &[&str] = &["mice", "people", "children", "men", "women", "feet", "teeth", "geese", "oxen", "lice"];

/// Trajta e kundrinorit sipas fjalës së fundit të bishtit anglisht (`en` = «koka|bishti»).
pub fn object_form(en: &str) -> ObjectForm {
    let last = en.split('|').nth(1).unwrap_or("").rsplit('_').next().unwrap_or("").to_lowercase();
    let plural = PLURAL_IRREGULAR.contains(&last.as_str())
        || (last.ends_with('s') && !last.ends_with("ss") && !last.ends_with("us") && !last.ends_with("is") && last.chars().count() > 3);
    match (MASS.contains(&last.as_str()), plural) {
        (true, _) => ObjectForm::Mass,
        (false, true) => ObjectForm::DefPlur,
        (false, false) => ObjectForm::DefSing,
    }
}

/// Një fakt i folshëm: lidhja, dy anët e saj dhe dëshmitarët.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub kind: Kind,
    pub head: String,
    pub tail: Tail,
    pub object: ObjectForm,
    pub sources: BTreeSet<Source>,
    pub datasets: BTreeSet<String>,
    /// Përkthimin e dëshmojnë të dyja urat (Wiktionary + WordNet).
    pub strong: bool,
    /// Koka e lidhjes është emër i thjeshtë («n=…»), jo veprim a cilësi.
    pub head_noun: bool,
    /// Forca e dëshmisë së dytë: 3 = raundi i parë (kontrolli më i rreptë), 2 = e pranuar dy herë
    /// ose nga dy grupe të dhënash, 1 = rregulli 2 nga 3 i raundit të dytë, 0 = pa dëshmi të dytë.
    pub witness_rank: u8,
    /// Pesha e ConceptNet × 1000 (renditja brenda të njëjtit lloj).
    pub weight: u32,
    /// Numri i rreshtave të ConceptNet-it që e pohojnë (më shumë se një pohues = më i zakonshëm).
    pub rows: u32,
    /// Mbiemri i kokës kur fakti flet për një nënlloj («libër i shenjtë», «hekur i nxehtë»).
    pub head_adj: Option<String>,
    /// Fjala angleze e kokës në rreshtin e burimit («machine», «cars»).
    pub en_head: String,
    /// Bishti anglisht në rreshtin e burimit («close», «dissolve in water», «sitting»).
    pub en_tail: String,
    /// Shkalla e rëndësisë për fjalën për të cilin flitet (shih `weigh`): 0 = s'flet për këtë
    /// fjalë/kuptim, 1 = e vërtetë por rastësore, 2 = e zakonshme, 3 = përkufizuese.
    pub salience: u8,
}

/// Shkallët e rëndësisë.
pub const FOREIGN: u8 = 0;
pub const INCIDENTAL: u8 = 1;
pub const TYPICAL: u8 = 2;
pub const DEFINITIONAL: u8 = 3;

/// Format që i duhen bisedës. I jep gramatika; `None` do të thotë «nuk e di me siguri».
pub trait Forms {
    /// Pjesa e ligjëratës sipas leksikut gramatikor.
    fn is_noun(&self, lemma: &str) -> bool;
    fn is_verb(&self, lemma: &str) -> bool;
    fn noun(&self, lemma: &str, case: Case, definite: Definite, number: Number) -> Option<String>;
    fn gender(&self, noun: &str) -> Option<Gender>;
    fn animate(&self, noun: &str) -> Option<bool>;
    fn human(&self, noun: &str) -> bool;
    /// Njeri që ushtron një punë a profesion (mjek, mësues, punëtor).
    fn worker(&self, noun: &str) -> bool;
    /// E tashmja dëftore, veta e tretë.
    fn verb(&self, lemma: &str, number: Number) -> Option<String>;
    /// E tashmja lidhore, veta e tretë, pa «të».
    fn subjunctive(&self, lemma: &str, number: Number) -> Option<String>;
    fn participle(&self, lemma: &str) -> Option<String>;
    /// Mbiemri kallëzuesor që përshtatet me emrin (me nyjë kur e ka).
    fn adjective(&self, lemma: &str, gender: Gender, number: Number) -> Option<String>;
    /// Mbiemri pas emrit, i përshtatur në gjini, numër, rasë dhe trajtë («kafshë e vogël»,
    /// «në shtëpi të madhe»).
    fn adjective_np(&self, lemma: &str, gender: Gender, number: Number, case: Case, definite: Definite) -> Option<String>;
    /// Klasa leksikografike e WordNet-it (`klasa::*`) e kuptimit për të cilin flitet, kur dihet.
    fn class(&self, _lemma: &str) -> Option<u8> {
        None
    }
    /// Si e lexon WordNet-i fjalën angleze të burimit (koka e rreshtit të ConceptNet-it): në
    /// kuptimin e fjalës shqipe për të cilin flitet, apo në një kuptim tjetër të saj.
    fn reading(&self, _lemma: &str, _en: &str) -> Reading {
        Reading::Unknown
    }
    /// Folja është vetëm kalimtare sipas leksikut («mbyll», «ul»): pa kundrinor të drejtë kërkon
    /// trajtën joveprore («dera mbyllet», «për t'u ulur»).
    fn transitive_only(&self, _verb: &str) -> bool {
        false
    }
    /// E tashmja dëftore joveprore, veta e tretë («mbyllet», «tretet»).
    fn verb_mid(&self, _lemma: &str, _number: Number) -> Option<String> {
        None
    }
    /// Lema është vetë joveprore («ulem», «ngjitem»).
    fn mid_lemma(&self, _verb: &str) -> bool {
        false
    }
    /// Kuptimet e hapura të fjalës (synset-et), i pari ai për të cilin flitet.
    fn senses(&self, _lemma: &str) -> Vec<String> {
        Vec::new()
    }
    /// Klasa leksikografike e një kuptimi.
    fn sense_class(&self, _synset: &str) -> Option<u8> {
        None
    }
    /// Kuptimi i fjalës shqipe ku lexohet fjala angleze e burimit.
    fn read_as(&self, _lemma: &str, _en: &str) -> Option<String> {
        None
    }
    /// Dy kuptime janë i njëjti ose njëri është hiperonim i tjetrit deri në dy hapa.
    fn near(&self, a: &str, b: &str) -> bool {
        a == b
    }
    /// Emri i shkurtër shqip i një kuptimi («pije alkoolike», «stinë»).
    fn sense_label(&self, _lemma: &str, _synset: &str) -> Option<String> {
        None
    }
}

/// Klasat leksikografike të WordNet 3.0 (numri i skedarit në `lexnames`) që përdorin rregullat e
/// rëndësisë.
pub mod klasa {
    pub const TOPS: u8 = 3;
    pub const ACT: u8 = 4;
    pub const ANIMAL: u8 = 5;
    pub const ARTIFACT: u8 = 6;
    pub const ATTRIBUTE: u8 = 7;
    pub const BODY: u8 = 8;
    pub const COGNITION: u8 = 9;
    pub const COMMUNICATION: u8 = 10;
    pub const EVENT: u8 = 11;
    pub const FEELING: u8 = 12;
    pub const FOOD: u8 = 13;
    pub const GROUP: u8 = 14;
    pub const LOCATION: u8 = 15;
    pub const MOTIVE: u8 = 16;
    pub const OBJECT: u8 = 17;
    pub const PERSON: u8 = 18;
    pub const PHENOMENON: u8 = 19;
    pub const PLANT: u8 = 20;
    pub const POSSESSION: u8 = 21;
    pub const PROCESS: u8 = 22;
    pub const QUANTITY: u8 = 23;
    pub const RELATION: u8 = 24;
    pub const SHAPE: u8 = 25;
    pub const STATE: u8 = 26;
    pub const SUBSTANCE: u8 = 27;
    pub const TIME: u8 = 28;

    /// Klasat që s'kanë një «vend të zakonshëm» si fakt kryesor: abstraksionet, ngjarjet, vetë
    /// vendet dhe trupat natyrorë (deti s'«gjendet në plazh»), lëndët (ari s'«gjendet në dhëmb»).
    pub const PLACELESS: [u8; 19] = [
        TOPS, ACT, ATTRIBUTE, COGNITION, COMMUNICATION, EVENT, FEELING, GROUP, LOCATION, MOTIVE, OBJECT, PHENOMENON,
        PROCESS, QUANTITY, RELATION, SHAPE, STATE, SUBSTANCE, TIME,
    ];

    /// Klasat që paraqiten para së gjithash me qëllimin e tyre («për çfarë përdoret»), jo me atë
    /// që «bëjnë»: sendet e bëra, ushqimi, lëndët, pasuria. (Pjesa e trupit paraqitet me atë që
    /// bën: «syri sheh».)
    pub const PURPOSE_FIRST: [u8; 4] = [ARTIFACT, FOOD, SUBSTANCE, POSSESSION];

    /// Vendi i një pjese të trupit është pjesë tjetër e trupit ose qenia që e ka.
    pub const BODY_PLACES: [u8; 3] = [BODY, ANIMAL, PERSON];
}

/// Leximi i fjalës angleze të burimit sipas renditjes së kuptimeve në WordNet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// Kuptimi i parë i fjalës angleze, ndër ato që ka fjala shqipe, është ai për të cilin flitet.
    Same,
    /// Kuptimi i parë është një kuptim tjetër i fjalës shqipe («machine» → «makinë» si mekanizëm,
    /// jo si veturë).
    Other,
    /// WordNet-i s'e njeh lidhjen.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub intent: Intent,
    pub words: Vec<String>,
    pub sentences: Vec<String>,
    pub question: Option<String>,
    pub facts: Vec<Fact>,
}

impl Reply {
    pub fn text(&self) -> String {
        self.sentences.join(" ")
    }
}

// ───────────────────────────── leximi i hyrjes ─────────────────────────────

const FILLER: &[&str] = &[
    "ç'është", "ç'janë", "çfarë", "ç'", "çka", "ka", "kanë", "me", "dhe", "e", "i", "të", "së", "si", "se", "pse", "përse", "ku", "kur",
    "është", "janë", "a", "në", "për", "nga", "mes", "midis", "ndërmjet", "lidhje", "lidhja", "lidhjen", "lidhet",
    "lidhen", "ç'lidhje", "shërben", "shërbejnë", "përdoret", "përdoren", "përdorim", "përdorimi", "ç'përdorim", "bën",
    "bëjnë", "ç'bën", "ç'bëjnë", "di", "cili", "cila", "kush", "një", "më", "fol", "folë", "tregomë", "tregom",
    "thuaj", "ky", "kjo", "ai", "ajo", "apo", "ose", "edhe", "pra", "tani", "mua", "unë", "ti", "ne", "ju", "mund",
    "duhet", "shumë", "pak", "gjë", "diçka", "rreth", "lidhur", "ndodhet", "dini", "njeh", "njihni",
    "ndodhen", "zakonisht", "ndodh", ",",
];

/// Foljet që në një pyetje me «ku» vetëm e mbushin pyetjen («ku jeton ariu», «ku punon mjeku»).
const PLACE_FILLER: &[&str] = &[
    "jeton", "jetojnë", "gjendet", "gjenden", "rri", "rrinë", "qëndron", "punon", "punojnë", "banon", "banojnë",
    "rritet", "rriten", "fle", "flenë", "shkon", "shkojnë", "gjen",
];

/// Synimi dhe fjalët e përmbajtjes së hyrjes.
pub fn read(input: &str) -> (Intent, Vec<String>) {
    let (intent, words, _) = read_cued(input);
    (intent, words)
}

/// Si `read`, por tregon edhe nëse lidhja u kërkua shprehimisht («dhe», «me», «lidhje»). Pa
/// shenjë, fjala e dytë është kontekst i së parës («zjarr armë») derisa një lidhje e dëshmuar
/// të tregojë të kundërtën.
pub fn read_cued(input: &str) -> (Intent, Vec<String>, bool) {
    let t = tokenize(input);
    let v: Vec<&str> = t.iter().map(String::as_str).collect();
    let has = |xs: &[&str]| v.iter().any(|w| xs.contains(w));
    let cue = match v.as_slice() {
        ["pse" | "përse", ..] => Some(Intent::Cause),
        ["ku", ..] => Some(Intent::Place),
        ["për", "çfarë" | "ç'" | "çka", ..] | ["ç'përdorim", ..] => Some(Intent::Purpose),
        // «çfarë di për X» = «më fol për X» (portreti), jo «çfarë di të bëjë X».
        ["çfarë" | "çka" | "ç'", "di" | "dini" | "njeh" | "njihni", "për", ..] => Some(Intent::Portrait),
        ["çfarë" | "çka", "bën" | "bëjnë" | "di", ..] | ["ç'bën" | "ç'bëjnë", ..] => Some(Intent::Ability),
        _ => None,
    };
    let cue = cue
        .or_else(|| has(&["shërben", "shërbejnë", "përdoret", "përdoren", "përdorim", "përdorimi"]).then_some(Intent::Purpose))
        .or_else(|| has(&["lidhje", "lidhja", "lidhjen", "lidhet", "lidhen", "ç'lidhje"]).then_some(Intent::Relation));
    let place = cue == Some(Intent::Place);
    let words: Vec<String> = v
        .iter()
        .filter(|w| !FILLER.contains(w) && !(place && PLACE_FILLER.contains(w)))
        .filter(|w| w.chars().count() > 1 && w.chars().all(|c| c.is_alphabetic() || c == '\''))
        .map(|w| w.to_string())
        .collect();
    let intent = match (cue, words.len() >= 2) {
        (Some(i), _) => i,
        (None, true) => Intent::Relation,
        (None, false) => Intent::Portrait,
    };
    let asked = cue.is_some() || has(&["dhe", "me", "edhe"]);
    (intent, words, asked)
}

// ───────────────────────────── faktet ─────────────────────────────

/// Burimet leksikore që dëshmojnë një lidhje të dijes së përbashkët pa qenë pjesë e ConceptNet-it:
/// përkufizimet dhe marrëdhëniet e WordNet-it, glosat shqipe të AlbaNet-it dhe të Gazullit,
/// marrëdhëniet e Wiktionary-t dhe të LEXIN-it.
pub fn lexical_witness(s: Source) -> bool {
    matches!(s, Source::Pwn30 | Source::OmwAls | Source::KaikkiEn | Source::KaikkiDe | Source::Gazulli1941 | Source::Lexin)
}

/// Dy dëshmitarë të pavarur: ConceptNet + dëshmia e veçantë për treshen shqipe, ose ConceptNet + një
/// burim leksikor i pavarur, ose ConceptNet nga dy grupe të dhënash kur edhe përkthimin e dëshmojnë
/// të dyja urat.
pub fn spoken(sources: &BTreeSet<Source>, datasets: &BTreeSet<String>, strong: bool) -> bool {
    let cn = sources.contains(&Source::ConceptNet57);
    let lex = sources.iter().any(|s| lexical_witness(*s));
    match (cn, sources.contains(&Source::ModelClaude) || lex, datasets.len() >= 2 && strong) {
        (true, true, _) => true,
        (true, false, true) => true,
        _ => false,
    }
}

fn lemma_of(eng: &Engine, id: u32) -> Option<String> {
    match &eng.g.node(id).kind {
        NodeKind::Word { lemma, .. } => Some(lemma.clone()),
        _ => None,
    }
}

/// Faktet e folshme të një fjale: `outgoing` = fjala është koka; ndryshe fjala është bishti.
pub fn facts(eng: &Engine, word: u32, outgoing: bool) -> Vec<Fact> {
    facts_where(eng, word, outgoing, spoken)
}

/// Faktet sipas një rregulli dëshmie të dhënë (për matje: p.sh. edhe me një dëshmitar).
pub fn facts_where(eng: &Engine, word: u32, outgoing: bool, rule: fn(&BTreeSet<Source>, &BTreeSet<String>, bool) -> bool) -> Vec<Fact> {
    let edges: Vec<(u32, &crate::graph::Edge)> = match outgoing {
        true => eng.g.out_edges(word).collect(),
        false => eng.g.in_edges(word).collect(),
    };
    let mut groups: BTreeMap<(Kind, u32, String, String), Fact> = BTreeMap::new();
    for (_, e) in edges {
        let kind = match &e.rel {
            Rel::Sem(name) => Kind::from_rel(name),
            _ => None,
        };
        let (kind, tail) = match (kind, e.attr("tail").and_then(Tail::parse)) {
            (Some(k), Some(t)) => (k, t),
            _ => continue,
        };
        let head = match lemma_of(eng, e.from) {
            Some(h) => h,
            None => continue,
        };
        let other = match outgoing {
            true => e.to,
            false => e.from,
        };
        let raw = e.attr("tail").unwrap_or("").to_string();
        let w = e.attr("w").and_then(|w| w.parse::<f32>().ok()).map(|w| (w * 1000.0) as u32).unwrap_or(1000);
        let object = object_form(e.attr("en").unwrap_or(""));
        let head_raw = e.attr("head").unwrap_or("").to_string();
        let head_adj = head_raw.split(';').find_map(|p| p.strip_prefix("a=")).map(str::to_string);
        let rows = e.attr("rows").and_then(|r| r.parse::<u32>().ok()).unwrap_or(0);
        let en_head = e.attr("en").and_then(|x| x.split('|').next()).unwrap_or("").replace('_', " ").to_lowercase();
        let en_tail = e.attr("en").and_then(|x| x.split('|').nth(1)).unwrap_or("").replace('_', " ").to_lowercase();
        let entry = groups.entry((kind, other, raw, head_raw)).or_insert_with(|| Fact {
            kind,
            head,
            tail,
            object,
            sources: BTreeSet::new(),
            datasets: BTreeSet::new(),
            strong: false,
            head_noun: e.attr("head").map(|h| h.starts_with("n=") && !h.contains(';')).unwrap_or(true),
            witness_rank: 0,
            weight: 0,
            rows: 0,
            head_adj,
            en_head: String::new(),
            en_tail: String::new(),
            salience: TYPICAL,
        });
        let rank = match (e.source, e.attr("reason").unwrap_or("").trim_end()) {
            (Source::ModelClaude, r) => match (r.ends_with("(r3)"), r.ends_with("(r2)")) {
                (true, _) => 1,
                (false, true) => 2,
                (false, false) => 3,
            },
            _ => 0,
        };
        entry.witness_rank = entry.witness_rank.max(rank);
        entry.sources.insert(e.source);
        entry.strong = entry.strong || matches!(e.attr("map"), Some("AB" | "AB2"));
        for d in e.attr("ds").unwrap_or("").split([',', '|', ';']).map(str::trim).filter(|d| !d.is_empty()) {
            entry.datasets.insert(d.to_string());
        }
        entry.weight = entry.weight.max(w);
        entry.rows = entry.rows.max(rows);
        match (entry.en_head.is_empty(), en_head.is_empty()) {
            (true, false) => {
                entry.en_head = en_head;
                entry.en_tail = en_tail;
            }
            _ => {}
        }
    }
    let mut out: Vec<Fact> = groups
        .into_values()
        .filter(|f| rule(&f.sources, &f.datasets, f.strong))
        .map(|mut f| {
            let lex = f.sources.iter().any(|s| lexical_witness(*s));
            f.witness_rank = match (f.witness_rank, f.datasets.len() >= 2 && f.strong, lex) {
                (0, true, _) | (0, _, true) => 2,
                (r, _, _) => r,
            };
            f
        })
        .collect();
    // Të parat thuhen faktet me dëshminë më të fortë, pastaj ato me përkthim nga dy urat, pastaj
    // sipas peshës së ConceptNet-it.
    out.sort_by(|a, b| {
        b.witness_rank
            .cmp(&a.witness_rank)
            .then(b.strong.cmp(&a.strong))
            .then(b.weight.cmp(&a.weight))
            .then(a.kind.cmp(&b.kind))
            .then(a.tail.cmp(&b.tail))
    });
    out
}

/// Etiketat shqipe të paraardhësve të kuptimit kryesor në WordNet, me largësinë (1 = prindi).
/// Kuptimet e hapura të një fjale nga të cilat zgjidhet ai për të cilin flitet: pas eliminimit
/// pa kontekst, pa kuptimet që vijnë vetëm nga Wiktionary ose nga modeli; kur ka kuptime emri,
/// vetëm ato (biseda flet për sende, qenie dhe dukuri).
pub fn open_pool(f: &Focus) -> Vec<&SenseCand> {
    let open: Vec<&SenseCand> = f
        .senses
        .iter()
        .filter(|s| s.status == CandStatus::Open && !matches!(s.via, Source::KaikkiEn | Source::ModelClaude))
        .collect();
    let nouns: Vec<&SenseCand> = open.iter().copied().filter(|s| s.pos == Pos::Noun).collect();
    match nouns.is_empty() {
        true => open,
        false => nouns,
    }
}

/// Kuptimi për të cilin flitet, me të njëjtin rregull si te kuptimi i fjalës: ai që ka përkthimin
/// kryesor të Wiktionary-t; pa të, ai që korpusi i WordNet-it e njeh qartë si më të shpeshtin
/// (≥ 3 etiketime dhe ≥ 1,5 herë më shumë se i dyti); pa asnjërën, i pari i mbetur.
pub fn portrait_sense<'f>(eng: &Engine, f: &'f Focus) -> Option<&'f SenseCand> {
    let pool = open_pool(f);
    let en_main: Option<String> = discourse::bridge_translation(eng, f.resolved.word).into_iter().next().map(|e| e.to_lowercase());
    let by_bridge = en_main.and_then(|e| pool.iter().copied().find(|x| x.labels_en.iter().any(|l| l.replace('_', " ").to_lowercase() == e)));
    let mut freq: Vec<(u32, &SenseCand)> = pool.iter().map(|s| (eng.sense_frequency(s.sense), *s)).collect();
    freq.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.sense.cmp(&b.1.sense)));
    let by_corpus = match freq.as_slice() {
        [(t1, s1), (t2, _), ..] => (*t1 >= 3 && *t1 * 2 >= *t2 * 3).then_some(*s1),
        _ => None,
    };
    by_bridge.or(by_corpus).or_else(|| pool.first().copied())
}

pub fn taxonomy(eng: &Engine, lemma: &str) -> Vec<(String, usize)> {
    let mut f = eng.focus(lemma);
    eng.eliminate(&mut f, None, &[]);
    taxonomy_from(eng, portrait_sense(eng, &f).map(|s| s.sense))
}

/// Paraardhësit e një kuptimi në WordNet (deri në tetë hapa), me emrat shqip dhe largësinë.
pub fn taxonomy_from(eng: &Engine, start: Option<u32>) -> Vec<(String, usize)> {
    let start = match start {
        Some(s) => s,
        None => return Vec::new(),
    };
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut seen: BTreeSet<u32> = BTreeSet::from([start]);
    let mut frontier = vec![start];
    for depth in 1..=8 {
        let mut next = Vec::new();
        for n in frontier {
            for (_, e) in eng.g.out_edges(n) {
                let up = match &e.rel {
                    Rel::Wn(s) => s == "@" || s == "@i",
                    Rel::Bn(r) => r == "HYPERNYM",
                    _ => false,
                };
                match (up, seen.insert(e.to)) {
                    (true, true) => {
                        for w in eng.g.words_of_sense(e.to, crate::graph::Lang::Sq) {
                            match lemma_of(eng, w) {
                                Some(l) => out.push((discourse::clean_label(&l), depth)),
                                None => {}
                            }
                        }
                        next.push(e.to);
                    }
                    _ => {}
                }
            }
        }
        frontier = next;
    }
    out
}

/// Kategoria që planifikuesi i fjalës e pranon (koherente me përkufizimin, e dëshmuar).
pub fn category(eng: &Engine, lemma: &str) -> Option<String> {
    let mut f = eng.focus(lemma);
    eng.eliminate(&mut f, None, &[]);
    let sense = portrait_sense(eng, &f)?;
    let facts = discourse::gather(eng, &f, sense.sense, sense.gloss_sq.as_deref());
    facts.category.into_iter().find(|c| !c.contains(' ') || c.split_whitespace().count() <= 2)
}

// ───────────────────────────── realizimi ─────────────────────────────

/// Subjekti i fjalisë: emri në trajtën e shquar emërore.
struct Subject {
    lemma: String,
    text: String,
    gender: Gender,
    animate: bool,
    human: bool,
    worker: bool,
}

fn subject(forms: &dyn Forms, lemma: &str) -> Option<Subject> {
    let text = forms.noun(lemma, Case::Nom, Definite::Def, Number::Sing)?;
    Some(Subject {
        lemma: lemma.to_string(),
        text,
        gender: forms.gender(lemma).unwrap_or(Gender::Masc),
        animate: forms.animate(lemma).unwrap_or(false),
        human: forms.human(lemma),
        worker: forms.worker(lemma),
    })
}

/// Emri me mbiemrin e tij në një rasë dhe trajtë të dhënë.
fn noun_phrase(forms: &dyn Forms, noun: &str, adj: Option<&str>, case: Case, definite: Definite, number: Number) -> Option<String> {
    let n = forms.noun(noun, case, definite, number)?;
    match adj {
        Some(a) => forms.adjective_np(a, forms.gender(noun)?, number, case, definite).map(|a| format!("{n} {a}")),
        None => Some(n),
    }
}

/// Kundrinori i foljes: pa parafjalë sipas trajtës së burimit; me parafjalë sipas rasës që
/// kërkon parafjala («me», «në», «për», «mbi», «nën», «pa» → kallëzore e pashquar; «nga», «te»
/// → emërore e shquar). Parafjalët e tjera nuk thuhen.
fn object_phrase(forms: &dyn Forms, prep: Option<&str>, noun: &str, adj: Option<&str>, how: ObjectForm) -> Option<String> {
    let number = match how {
        ObjectForm::DefPlur => Number::Plur,
        _ => Number::Sing,
    };
    let (case, definite) = match (prep, how) {
        (None, ObjectForm::Mass) => (Case::Acc, Definite::Ind),
        (None, _) => (Case::Acc, Definite::Def),
        (Some("nga" | "te" | "tek"), _) => (Case::Nom, Definite::Def),
        (Some("me" | "në" | "mbi" | "nën" | "për" | "pa" | "drejt" | "ndaj" | "kundër" | "nëpër" | "përmes"), _) => (Case::Acc, Definite::Ind),
        (Some(_), _) => return None,
    };
    let np = noun_phrase(forms, noun, adj, case, definite, number)?;
    Some(match prep {
        Some(p) => format!("{p} {np}"),
        None => np,
    })
}

/// Folja me plotësat e saj: kundrinor (me parafjalë a mbiemër) dhe ndajfolje.
fn verb_phrase(forms: &dyn Forms, head: String, tail: &Tail, how: ObjectForm) -> Option<String> {
    match tail {
        Tail::Verb { prep, object, adj, adv, .. } => {
            let obj = match object {
                Some(o) => Some(object_phrase(forms, prep.as_deref(), o, adj.as_deref(), how)?),
                None => None,
            };
            let parts: Vec<String> = [Some(head), obj, adv.clone()].into_iter().flatten().collect();
            Some(parts.join(" "))
        }
        _ => None,
    }
}

/// Folja e vendit sipas subjektit: kush ushtron një punë punon, njeriu tjetër është, gjallesa
/// jeton, sendi gjendet.
fn place_verb(s: &Subject, number: Number) -> &'static str {
    match (s.worker, s.human, s.animate, number) {
        (true, _, _, Number::Sing) => "punon",
        (true, _, _, Number::Plur) => "punojnë",
        (false, true, _, Number::Sing) => "është",
        (false, true, _, Number::Plur) => "janë",
        (false, false, true, Number::Sing) => "jeton",
        (false, false, true, Number::Plur) => "jetojnë",
        (false, false, false, Number::Sing) => "gjendet",
        (false, false, false, Number::Plur) => "gjenden",
    }
}

/// Folja e vendit kur vendi është i mundshëm, jo i zakonshëm: «mund të gjendet», «mund të jetojë».
fn maybe_place_verb(s: &Subject, number: Number) -> &'static str {
    match (s.worker, s.human, s.animate, number) {
        (true, _, _, Number::Sing) => "mund të punojë",
        (true, _, _, Number::Plur) => "mund të punojnë",
        (false, true, _, Number::Sing) => "mund të jetë",
        (false, true, _, Number::Plur) => "mund të jenë",
        (false, false, true, Number::Sing) => "mund të jetojë",
        (false, false, true, Number::Plur) => "mund të jetojnë",
        (false, false, false, Number::Sing) => "mund të gjendet",
        (false, false, false, Number::Plur) => "mund të gjenden",
    }
}

/// Pjesa e përbashkët e disa kallëzuesve thuhet një herë: «jeton zakonisht në ujë, në oqean dhe
/// në akuarium». Parafjala mbetet me secilin plotës.
fn factor(preds: &[String]) -> Vec<String> {
    // Kallëzuesi që është fillim i një tjetri («për të pushuar» / «për të pushuar këmbët») thuhet
    // vetëm një herë, në trajtën e plotë.
    let kept: Vec<String> = preds.iter().filter(|p| !preds.iter().any(|q| q.starts_with(&format!("{p} ")))).cloned().collect();
    let preds = kept.as_slice();
    let toks: Vec<Vec<&str>> = preds.iter().map(|p| p.split(' ').collect()).collect();
    let shortest = toks.iter().map(Vec::len).min().unwrap_or(0);
    let mut n = (0..shortest).take_while(|&i| toks.iter().all(|t| t[i] == toks[0][i])).count();
    while n > 0 && matches!(toks[0][n - 1], "në" | "me" | "për" | "të" | "nga" | "te" | "mbi" | "nën") {
        n -= 1;
    }
    // Kallëzuesi që është vetëm pjesa e përbashkët («për të ngrënë») bashkohet me ata që e
    // plotësojnë («për të ngrënë lëngjet dhe kosin»).
    let full: Vec<&Vec<&str>> = match toks.iter().any(|t| t.len() > n) {
        true => toks.iter().filter(|t| t.len() > n || n == 0).collect(),
        false => toks.iter().collect(),
    };
    match (n, full.len() >= 2) {
        (0, _) | (_, false) => full.iter().map(|t| t.join(" ")).collect(),
        (n, true) => full
            .iter()
            .enumerate()
            .map(|(i, t)| match i {
                0 => t.join(" "),
                _ => t[n..].join(" "),
            })
            .collect(),
    }
}

/// Lista natyrale: «a», «a dhe b», «a, b dhe c».
fn join_and(items: &[String]) -> Option<String> {
    match items {
        [] => None,
        [a] => Some(a.clone()),
        [init @ .., last] => Some(format!("{} dhe {last}", init.join(", "))),
    }
}

/// Përemri «nga ana e tij/saj» sipas gjinisë.
fn on_its_side(g: Gender) -> &'static str {
    match g {
        Gender::Fem => "nga ana e saj",
        _ => "nga ana e tij",
    }
}

/// Kallëzuesi i një fakti për një subjekt të dhënë (pa subjektin).
fn predicate(forms: &dyn Forms, s: &Subject, f: &Fact, number: Number) -> Option<String> {
    let gender = s.gender;
    let plural = |sing: &str, plur: &str| -> String {
        match number {
            Number::Sing => sing.to_string(),
            Number::Plur => plur.to_string(),
        }
    };
    match (f.kind, &f.tail) {
        (Kind::IsA, Tail::Noun { noun, adj }) => {
            let np = noun_phrase(forms, noun, adj.as_deref(), Case::Nom, Definite::Ind, number)?;
            Some(format!("{} {np}", plural("është", "janë")))
        }
        (Kind::CapableOf, Tail::Verb { verb, .. }) => {
            let head = match middle(forms, s, f) {
                true => forms.verb_mid(verb, number)?,
                false => forms.verb(verb, number)?,
            };
            verb_phrase(forms, head, &f.tail, f.object)
        }
        (Kind::UsedFor, Tail::Verb { verb, .. }) => {
            let p = forms.participle(verb)?;
            let to = match middle(forms, s, f) {
                true => "t'u",
                false => "të",
            };
            verb_phrase(forms, format!("{} për {to} {p}", plural("përdoret", "përdoren")), &f.tail, f.object)
        }
        (Kind::UsedFor, Tail::Noun { noun, adj }) => {
            let np = noun_phrase(forms, noun, adj.as_deref(), Case::Acc, Definite::Ind, Number::Sing)?;
            Some(format!("{} për {np}", plural("përdoret", "përdoren")))
        }
        (Kind::AtLocation, Tail::Noun { noun, adj }) => {
            let np = noun_phrase(forms, noun, adj.as_deref(), Case::Acc, Definite::Ind, Number::Sing)?;
            match f.salience >= TYPICAL {
                true => Some(format!("{} zakonisht në {np}", place_verb(s, number))),
                false => Some(format!("{} në {np}", maybe_place_verb(s, number))),
            }
        }
        (Kind::HasProperty, Tail::Adj(a)) => {
            let adj = forms.adjective(a, gender, number)?;
            Some(format!("{} {adj}", plural("është", "janë")))
        }
        (Kind::HasA, Tail::Noun { noun, adj }) => {
            let np = noun_phrase(forms, noun, adj.as_deref(), Case::Acc, Definite::Ind, Number::Sing)?;
            Some(format!("{} {np}", plural("ka", "kanë")))
        }
        (Kind::PartOf, Tail::Noun { noun, adj: None }) => {
            let whole = forms.noun(noun, Case::Dat, Definite::Def, Number::Sing)?;
            Some(format!("{} pjesë e {whole}", plural("është", "janë")))
        }
        (Kind::MadeOf, Tail::Noun { noun, adj: None }) => {
            let m = forms.noun(noun, Case::Nom, Definite::Def, Number::Sing)?;
            Some(format!("{} nga {m}", plural("bëhet", "bëhen")))
        }
        (Kind::Causes, Tail::Noun { noun, adj }) => {
            let np = noun_phrase(forms, noun, adj.as_deref(), Case::Acc, Definite::Ind, Number::Sing)?;
            Some(format!("{} {np}", plural("shkakton", "shkaktojnë")))
        }
        (Kind::Desires, Tail::Verb { verb, .. }) => {
            let v = forms.subjunctive(verb, number)?;
            verb_phrase(forms, format!("{} të {v}", plural("dëshiron", "dëshirojnë")), &f.tail, f.object)
        }
        _ => None,
    }
}

/// Foljet angleze me ndërrim shkakor («the door closes» / «she closes the door»): Levin (1993),
/// klasat 45.1 (break), 45.2 (bend), 45.3 (cook), 45.4 (ndryshim gjendjeje), 51.3.1 (roll). Me
/// subjekt jo të gjallë dhe pa kundrinor, shqipja i thotë në trajtën joveprore («dera mbyllet»,
/// «kripa tretet në ujë», «akulli shkrihet»).
const ERGATIVE: &[&str] = &[
    "break", "chip", "crack", "crash", "crush", "fracture", "rip", "shatter", "smash", "snap", "splinter", "split", "tear",
    "bend", "crease", "crinkle", "crumple", "fold", "wrinkle", "bake", "boil", "cook", "fry", "roast", "simmer", "steam",
    "open", "close", "shut", "melt", "dissolve", "freeze", "thaw", "dry", "cool", "warm", "heat", "burn", "grow", "increase",
    "decrease", "improve", "darken", "brighten", "empty", "fill", "flatten", "harden", "soften", "shrink", "sink", "stretch",
    "widen", "narrow", "start", "stop", "begin", "end", "finish", "move", "turn", "roll", "spin", "rotate", "bounce",
    "slide", "swing", "shake", "drop", "explode", "collapse", "evaporate", "rot", "decay", "rust", "light", "extinguish",
    "inflate", "deflate", "sharpen", "loosen", "tighten", "unfold", "wake",
];

/// Foljet angleze të qëndrimit të trupit dhe të kujdesit për veten (Levin 1993, klasat 50 dhe
/// 41.1.1), si edhe «climb»: shqipja i thotë me folje vetvetore («ulem», «shtrihem», «ngjitem»,
/// «lahem», «vishem»), pra pa kundrinor në trajtën joveprore («për t'u ulur»).
const REFLEXIVE: &[&str] = &[
    "sit", "lie", "kneel", "squat", "crouch", "climb", "bathe", "dress", "undress", "shave", "hide", "wake",
];

/// Trajtat bazë të mundshme të foljes së parë angleze të bishtit («sitting» → «sit», «closes» → «close»).
fn en_verb_bases(en_tail: &str) -> Vec<String> {
    let w = en_tail.split_whitespace().next().unwrap_or("");
    let mut out = vec![w.to_string()];
    for suffix in ["ing", "ed", "es", "s"] {
        match w.strip_suffix(suffix) {
            Some(b) => {
                out.push(b.to_string());
                out.push(format!("{b}e"));
                let mut c = b.chars();
                let last = c.next_back();
                match (last, c.next_back()) {
                    (Some(x), Some(y)) => match x == y {
                        true => out.push(c.as_str().to_string() + &y.to_string()),
                        false => {}
                    },
                    _ => {}
                }
            }
            None => {}
        }
    }
    out
}

/// Trajta joveprore thuhet kur lema e bishtit është vetë joveprore («ulem»), ose kur folja nuk ka kundrinor të drejtë (vetëm me parafjalë, ose pa
/// plotës), folja shqipe është vetëm kalimtare sipas leksikut, dhe folja angleze e burimit është
/// (a) vetvetore në shqip, ose (b) me ndërrim shkakor dhe subjekti nuk është qenie e gjallë.
fn middle(forms: &dyn Forms, s: &Subject, f: &Fact) -> bool {
    let bases = en_verb_bases(&f.en_tail);
    let reflexive = bases.iter().any(|b| REFLEXIVE.contains(&b.as_str()));
    let ergative = !s.animate && bases.iter().any(|b| ERGATIVE.contains(&b.as_str()));
    match &f.tail {
        Tail::Verb { verb, prep, object, .. } => {
            forms.mid_lemma(verb) || (reflexive || ergative) && (object.is_none() || prep.is_some()) && forms.transitive_only(verb)
        }
        _ => false,
    }
}

fn sentence(s: &str) -> String {
    let t = s.trim().trim_end_matches(['.', ',']);
    format!("{}.", capitalize(t))
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Fakti më i mirë nga llojet e dhëna, ndër ata me rëndësi të paktën `min`: së pari rëndësia,
/// pastaj forca e dëshmisë, pastaj radha e llojeve, pastaj renditja e fakteve (përkthimi nga dy
/// urat, pesha).
fn best_from<'a>(facts: &'a [Fact], kinds: &[Kind], used: &BTreeSet<String>, min: u8) -> Option<&'a Fact> {
    facts
        .iter()
        .enumerate()
        .filter(|(_, f)| !used.contains(f.tail.key()) && f.salience >= min)
        .filter_map(|(i, f)| {
            kinds
                .iter()
                .position(|k| *k == f.kind)
                .map(|k| (std::cmp::Reverse(f.salience), std::cmp::Reverse(f.witness_rank), k, i, f))
        })
        .min_by_key(|(s, r, k, i, _)| (*s, *r, *k, *i))
        .map(|(_, _, _, _, f)| f)
}

/// Fakti më i mirë që mund të jetë fakt kryesor (i zakonshëm ose përkufizues).
fn best<'a>(facts: &'a [Fact], kinds: &[Kind], used: &BTreeSet<String>) -> Option<&'a Fact> {
    best_from(facts, kinds, used, TYPICAL)
}

/// Pyetja që e çon bisedën më tej: lidhja me një fjalë që nuk u përmend ende.
fn follow_up(forms: &dyn Forms, s: &Subject, facts: &[Fact], used: &BTreeSet<String>) -> Option<String> {
    let next = facts.iter().filter_map(|f| f.tail.object()).find(|o| !used.contains(*o) && *o != s.lemma)?;
    let other = forms.noun(next, Case::Acc, Definite::Def, Number::Sing)?;
    Some(format!("Dëshiron të shohim si lidhet {} me {other}?", lower_first(&s.text)))
}

/// Portreti i një fjale: çfarë është, çfarë bën, ku e gjen — të lidhura, jo si zë fjalori.
pub fn portrait(forms: &dyn Forms, lemma: &str, all: &[Fact], cat: Option<String>, tax: &[(String, usize)]) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let s = subject(forms, lemma)?;
    // Portreti thotë vetëm faktet që mund të jenë kryesore për këtë fjalë në këtë kuptim.
    let kept: Vec<Fact> = all.iter().filter(|f| f.salience >= TYPICAL).cloned().collect();
    let facts = kept.as_slice();
    let mut used: BTreeSet<String> = BTreeSet::from([lemma.to_string()]);
    let mut said: Vec<Fact> = Vec::new();
    // Kategoria: lidhja «lloj i» që e dëshmon edhe taksonomia e WordNet-it (më e afërta), pastaj
    // kategoria e planifikuesit të fjalës, pastaj lidhja më e fortë «lloj i».
    let near = |t: &str| tax.iter().filter(|(l, _)| l == t).map(|(_, d)| *d).min();
    let confirmed = facts
        .iter()
        .filter(|f| f.kind == Kind::IsA)
        .filter_map(|f| near(f.tail.key()).map(|d| (d, f)))
        .min_by_key(|(d, _)| *d)
        .map(|(_, f)| f.clone());
    let isa = confirmed.clone().or_else(|| match &cat {
        Some(_) => None,
        None => facts.iter().find(|f| f.kind == Kind::IsA).cloned(),
    });
    let category = confirmed.as_ref().map(|f| f.tail.key().to_string()).or(cat).or_else(|| isa.as_ref().map(|f| f.tail.key().to_string()));
    // Sendi i bërë, ushqimi, lënda dhe pjesa e trupit paraqiten me qëllimin e tyre; gjallesat dhe
    // dukuritë me atë që bëjnë. Lloji tjetër merret vetëm kur është më i rëndësishëm.
    let purpose_first = forms.class(lemma).map(|c| klasa::PURPOSE_FIRST.contains(&c)).unwrap_or(false);
    let function = match purpose_first {
        true => {
            let purpose = best(facts, &[Kind::UsedFor], &used);
            let ability = best(facts, &[Kind::CapableOf], &used);
            match (purpose, ability) {
                (Some(p), Some(a)) => match a.salience > p.salience {
                    true => Some(a.clone()),
                    false => Some(p.clone()),
                },
                (p, a) => p.or(a).cloned(),
            }
        }
        false => best(facts, &[Kind::CapableOf, Kind::UsedFor], &used).cloned(),
    };
    let lead = match (&category, &function) {
        (Some(c), Some(f)) => predicate(forms, &s, f, Number::Sing).map(|p| format!("{} është {c} që {p}", s.text)),
        (Some(c), None) => Some(format!("{} është {c}", s.text)),
        (None, Some(f)) => predicate(forms, &s, f, Number::Sing).map(|p| format!("{} {p}", s.text)),
        (None, None) => None,
    };
    let mut out: Vec<String> = Vec::new();
    match lead {
        Some(l) => {
            out.push(sentence(&l));
            category.iter().for_each(|c| {
                used.insert(c.clone());
            });
            isa.iter().for_each(|f| said.push(f.clone()));
            function.iter().for_each(|f| {
                used.insert(f.tail.key().to_string());
                f.tail.object().iter().for_each(|o| {
                    used.insert(o.to_string());
                });
                said.push(f.clone());
            });
        }
        None => {}
    }
    let mut more: Vec<(String, Fact)> = Vec::new();
    let lead_kind = function.as_ref().map(|f| f.kind);
    for kinds in [
        &[Kind::AtLocation][..],
        &[Kind::HasProperty][..],
        &[Kind::CapableOf, Kind::UsedFor][..],
        &[Kind::HasA, Kind::MadeOf, Kind::PartOf][..],
        &[Kind::Desires][..],
    ] {
        // «Përdoret …» nuk përsëritet në fjalinë e dytë; as i njëjti lloj fakti dy herë në të.
        let fresh: Vec<Kind> = kinds
            .iter()
            .copied()
            .filter(|k| !(*k == Kind::UsedFor && lead_kind == Some(Kind::UsedFor)) && !more.iter().any(|(_, m)| m.kind == *k))
            .collect();
        match (more.len() < 2, best(facts, &fresh, &used)) {
            (true, Some(f)) => match predicate(forms, &s, f, Number::Sing) {
                Some(p) => {
                    used.insert(f.tail.key().to_string());
                    f.tail.object().iter().for_each(|o| {
                        used.insert(o.to_string());
                    });
                    more.push((p, f.clone()));
                }
                None => {}
            },
            _ => {}
        }
    }
    let subject_first = match out.is_empty() {
        true => format!("{} ", s.text),
        false => String::new(),
    };
    match more.as_slice() {
        [] => {}
        [(a, fa)] => {
            out.push(sentence(&format!("{subject_first}{a}")));
            said.push(fa.clone());
        }
        [(a, fa), (b, fb), ..] => {
            let both = [sentence(&format!("{subject_first}{a} dhe {b}")), sentence(&format!("{subject_first}{b} dhe {a}"))];
            out.push(crate::oratoria::zgjidh(&both).cloned().unwrap_or_else(|| both[0].clone()));
            said.push(fa.clone());
            said.push(fb.clone());
        }
    }
    match out.is_empty() {
        true => None,
        false => Some((out, follow_up(forms, &s, facts, &used), said)),
    }
}

/// Rrënja e emrit për rregullin e përsëritjes: emri pa zanoren fundore («hekur», «peshk»).
fn stem(lemma: &str) -> String {
    let t = lemma.trim_end_matches(['ë', 'e', 'a', 'i', 'o', 'u', 'y']);
    t.to_string()
}

/// Fakti që thotë fjalën me vetveten («hekuri përdoret për të hekurosur», «peshku … për të
/// peshkuar»): folja e bishtit është bërë nga vetë emri i kokës (rrënja ≥ 4 shkronja).
fn restates(lemma: &str, f: &Fact) -> bool {
    let root = stem(lemma);
    match (&f.tail, root.chars().count() >= 4) {
        (Tail::Verb { verb, .. }, true) => verb.starts_with(&root) && verb != lemma,
        _ => false,
    }
}

/// Faktet që i dëshmon një burim leksikor (përkufizimi a lidhja e një fjalori) janë përkufizuese.
fn definitional(f: &Fact) -> bool {
    f.sources.iter().any(|s| lexical_witness(*s))
}

/// Mjaft pohues: pesha e ConceptNet-it ≥ 2, ose ≥ 2 rreshta, ose ≥ 2 grupe të dhënash.
fn well_attested(f: &Fact) -> bool {
    f.weight >= 2000 || f.rows >= 2 || f.datasets.len() >= 2
}

/// Shkalla e rëndësisë së një fakti për fjalën `lemma` (koka), me rregulla të deklaruara, në këtë radhë:
/// 1. Fakti për një nënlloj («libër i shenjtë» → kishë) ose për një kuptim tjetër të fjalës
///    (fjala angleze e burimit lexohet nga WordNet-i në një kuptim tjetër, të një klase tjetër)
///    → 0: nuk thuhet për fjalën e zhveshur.
/// 2. Fakti që e përsërit fjalën me vetveten (folja e bërë nga vetë emri) → 1.
/// 3. Fakti me dëshmi përkufizuese (burim leksikor) → 3.
/// 4. Vendi: për klasat pa «vend të zakonshëm» (`klasa::PLACELESS`), për pjesën e trupit jashtë
///    trupit ose qenies, ose me më pak se dy pohues → 1 (thuhet vetëm si «mund të gjendet»).
/// 5. Cilësia me një pohues të vetëm → 1.
/// 6. Të tjerat → 2.
pub fn salience(forms: &dyn Forms, lemma: &str, class: Option<u8>, f: &Fact) -> u8 {
    salience_in(forms, lemma, class, forms.reading(lemma, &f.en_head) == Reading::Other, f)
}

/// Rëndësia kur dihet nëse fakti lexohet në një kuptim tjetër nga ai për të cilin flitet.
fn salience_in(forms: &dyn Forms, lemma: &str, class: Option<u8>, other_sense: bool, f: &Fact) -> u8 {
    let foreign = f.head_adj.is_some() || other_sense;
    let place_odd = match (f.kind, &f.tail) {
        (Kind::AtLocation, Tail::Noun { noun, .. }) => {
            let placeless = class.map(|c| klasa::PLACELESS.contains(&c)).unwrap_or(false);
            let body_out = match (class, forms.class(noun)) {
                (Some(klasa::BODY), Some(t)) => !klasa::BODY_PLACES.contains(&t),
                _ => false,
            };
            placeless || body_out || !well_attested(f)
        }
        _ => false,
    };
    let property_odd = f.kind == Kind::HasProperty && !well_attested(f);
    match (foreign, restates(lemma, f), definitional(f), place_odd || property_odd) {
        (true, _, _, _) => FOREIGN,
        (false, true, _, _) => INCIDENTAL,
        (false, false, true, _) => DEFINITIONAL,
        (false, false, false, true) => INCIDENTAL,
        (false, false, false, false) => TYPICAL,
    }
}

/// Vendos rëndësinë e çdo fakti të fjalës `lemma` (koka e fakteve).
pub fn weigh(forms: &dyn Forms, lemma: &str, facts: &mut [Fact]) {
    let class = forms.class(lemma);
    for f in facts.iter_mut() {
        f.salience = salience(forms, lemma, class, f);
    }
}

/// Si `weigh`, por për një kuptim të caktuar të fjalës (fjala me disa kuptime).
pub fn weigh_as(forms: &dyn Forms, lemma: &str, sense: &str, facts: &mut [Fact]) {
    let class = forms.sense_class(sense);
    for f in facts.iter_mut() {
        let other = forms.read_as(lemma, &f.en_head).map(|x| !forms.near(&x, sense)).unwrap_or(false);
        f.salience = salience_in(forms, lemma, class, other, f);
    }
}

// ───────────────────── fjalët me disa kuptime ─────────────────────

/// Fjala e lidhjes është shumë e përgjithshme kur e përmendin më shumë se kaq shpjegime
/// («njeri», «pjesë», «gjë»): nuk përdoret si urë mes dy kuptimeve.
const HUB: usize = 100;

/// Fjala është zë fjalori: ka gramatikë, kuptim ose shpjegim krahinor.
fn headword(eng: &Engine, id: u32) -> bool {
    eng.g.out_edges(id).any(|(_, e)| e.rel == Rel::Lex || e.rel == Rel::Sem("REGIONAL_GLOSS".into())) || !eng.g.senses_of(id).is_empty()
}

/// Leximet me dëshmi të një fjale të shtypur: vetë forma si zë fjalori, trajta e atestuar
/// (tabelat e Wiktionary-t, UD) e një leme, edhe kur fjala është shkruar pa «ë»/«ç» («vere» →
/// «verë», «dere» → «derë»). Rregullat e heqjes së mbaresave nuk janë dëshmi.
pub fn readings(eng: &Engine, word: &str) -> Vec<(String, u32)> {
    let low = word.trim().to_lowercase();
    let mut forms: Vec<String> = vec![low.clone()];
    crate::gramatika::fonologjia::diakritikat::candidates(&low)
        .unwrap_or_default()
        .into_iter()
        .filter(|v| *v != low)
        .for_each(|v| forms.push(v));
    let mut out: Vec<(String, u32)> = Vec::new();
    let mut push = |l: String, id: u32| match out.iter().any(|(x, _)| *x == l) {
        true => {}
        false => out.push((l, id)),
    };
    for f in &forms {
        let id = eng.lex.get(f).copied();
        id.filter(|&i| headword(eng, i)).iter().for_each(|&i| push(f.clone(), i));
        let inflected: Vec<(String, u32)> = id
            .map(|i| {
                eng.g
                    .out_edges(i)
                    .filter(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()) && e.source != Source::ModelClaude && e.to != i)
                    .filter(|(_, e)| !e.attr("tags").unwrap_or("").contains("participle"))
                    // Trajta alternative (drejtshkrim a dialekt tjetër) nuk është lexim i lakuar.
                    .filter(|(_, e)| e.attr("kind") != Some("alt_of"))
                    .filter_map(|(_, e)| lemma_of(eng, e.to).map(|l| (l, e.to)))
                    .collect()
            })
            .unwrap_or_default();
        inflected.into_iter().filter(|(_, i)| headword(eng, *i)).for_each(|(l, i)| push(l, i));
        eng.morph
            .attested_lemmas(f)
            .into_iter()
            .filter_map(|l| eng.lex.get(&l).map(|&i| (l, i)))
            .filter(|(_, i)| headword(eng, *i))
            .for_each(|(l, i)| push(l, i));
    }
    out
}

/// Përkufizimi shqip i një kuptimi (ose i fjalës), kur fillon me emrin e llojit («njeri që …»,
/// «tokë me bar …»): vetëm atëherë mund të thuhet «X është …».
fn np_gloss(eng: &Engine, word: u32, sense: Option<&str>) -> Option<(String, String)> {
    let g = eng.g;
    g.out_edges(word)
        .filter(|(_, e)| e.rel == Rel::Sem(crate::src_perkufizime::REL.into()) && e.attr("genus") == Some("1") && e.attr("i") == Some("0"))
        .filter(|(_, e)| sense.map(|s| e.attr("sense").map(|k| k.ends_with(&format!(":{s}"))).unwrap_or(false)).unwrap_or(true))
        .filter_map(|(_, e)| e.attr("sense").map(|k| (k, e.to, e.source)))
        .filter_map(|(k, genus, from)| {
            let (code, synset) = k.split_once(':')?;
            let src = Source::parse(code)?;
            let node = g.get(&crate::graph::sense_key(src, synset))?;
            let text = g
                .node(node)
                .glosses
                .iter()
                .find(|t| t.lang == crate::graph::Lang::Sq && t.source == from)
                .map(|t| lower_first(t.text.trim().trim_start_matches(['.', ' ', ';']).trim_end_matches('.')))?;
            // Përkufizimi në formë fjalie («X është një …») nuk thuhet pas «… është».
            let sentence_like = text.split_whitespace().any(|w| matches!(w, "është" | "janë" | "ishte"));
            match sentence_like {
                true => None,
                false => Some((text, lemma_of(eng, genus)?)),
            }
        })
        .next()
}

/// Fjala është emër sipas leksikut gramatikor.
fn noun_node(eng: &Engine, n: u32) -> bool {
    eng.g.out_edges(n).any(|(_, e)| e.rel == Rel::Lex && e.attr("pos") == Some("n"))
}

/// Lidhja e dy fjalëve sipas shpjegimeve. Merret vetëm kur njëri përkufizim e përmend
/// drejtpërdrejt anën tjetër ose një fjalë të përkufizimit të saj; format e lejuara, në këtë radhë:
/// 1. përkufizimi i njërës e përmend tjetrën;
/// 2. një emër X e përmend në përkufizimin e vet secilën prej tyre;
/// 3. një emër X e përmend njërën dhe një emër Y që e përmend përkufizimi i tjetrës
///    («kullotë: tokë me bar ku ushqehen bagëtitë» — «bari: njeri që ruan … bagëti»).
/// Urat janë emra jo shumë të përgjithshëm (`HUB`); ndër urat e mundshme merret ajo me shumën më
/// të vogël të `df` (më e veçanta), në barazim nyja më e vogël. Kthen urën X, kur ka.
fn definition_bridge(eng: &Engine, a: u32, b: u32) -> Option<(Option<u32>, u32)> {
    let g = eng.g;
    let rel = Rel::Sem(crate::src_perkufizime::REL.into());
    let df = |n: u32| g.in_edges(n).filter(|(_, e)| e.rel == rel).filter_map(|(_, e)| e.attr("df").and_then(|d| d.parse::<usize>().ok())).max().unwrap_or(0);
    let bridge_ok = |n: u32| df(n) <= HUB && noun_node(eng, n);
    let out = |n: u32| -> BTreeSet<u32> { g.out_edges(n).filter(|(_, e)| e.rel == rel).map(|(_, e)| e.to).collect() };
    let inn = |n: u32| -> BTreeSet<u32> { g.in_edges(n).filter(|(_, e)| e.rel == rel).map(|(_, e)| e.from).collect() };
    // 1. drejtpërdrejt: kthen përkufizimin që e përmend tjetrën
    match (out(a).contains(&b), out(b).contains(&a)) {
        (true, _) => return Some((None, a)),
        (false, true) => return Some((None, b)),
        _ => {}
    }
    // 2. X i përmend të dyja
    let both: Option<u32> = inn(a).intersection(&inn(b)).copied().filter(|&x| bridge_ok(x)).min_by_key(|&x| (df(x), x));
    match both {
        Some(x) => return Some((Some(x), x)),
        None => {}
    }
    // 3. X përmend njërën dhe Y-në që e përmend tjetra
    let third = |from: u32, to: u32| -> Option<(usize, u32)> {
        let ys: BTreeSet<u32> = out(to).into_iter().filter(|&y| bridge_ok(y)).collect();
        inn(from)
            .into_iter()
            .filter(|&x| bridge_ok(x))
            .filter_map(|x| out(x).intersection(&ys).map(|&y| df(x) + df(y)).min().map(|c| (c, x)))
            .min()
    };
    [third(a, b), third(b, a)].into_iter().flatten().min().map(|(_, x)| (Some(x), x))
}

/// Fjalia që i lidh dy leximet sipas shpjegimeve: thuhet përkufizimi i urës (ose i fjalës që e
/// përmend tjetrën drejtpërdrejt).
fn link_sentence(eng: &Engine, forms: &dyn Forms, a: u32, b: u32) -> Option<String> {
    let (bridge, quoted) = definition_bridge(eng, a, b)?;
    let name = |n: u32| lemma_of(eng, n).and_then(|l| forms.noun(&l, Case::Nom, Definite::Def, Number::Sing));
    let (first, last) = (name(a)?, name(b)?);
    let lemma = lemma_of(eng, quoted)?;
    let (gloss, _) = np_gloss(eng, quoted, None)?;
    let subject = forms.noun(&lemma, Case::Nom, Definite::Def, Number::Sing)?;
    match bridge {
        Some(_) => {
            let through = forms.noun(&lemma, Case::Abl, Definite::Def, Number::Sing)?;
            Some(format!("{} dhe {} lidhen përmes {through}: {} është {gloss}.", capitalize(&first), lower_first(&last), lower_first(&subject)))
        }
        None => Some(format!("{} dhe {} lidhen drejtpërdrejt: {} është {gloss}.", capitalize(&first), lower_first(&last), lower_first(&subject))),
    }
}

/// Një lexim i fjalës: lema, kuptimi (kur ka disa), faktet e peshuara për të.
struct View {
    lemma: String,
    id: u32,
    sense: Option<String>,
    facts: Vec<Fact>,
    /// Përkufizimi që mund të thuhet si «X është …» dhe lema e llojit të tij.
    gloss: Option<(String, String)>,
}

impl View {
    fn knows(&self) -> bool {
        self.facts.iter().any(|f| f.kind != Kind::IsA && f.salience >= TYPICAL)
    }
}

/// Leximet e fjalës që Nura mund t'i thotë: për çdo lemë me dëshmi, kuptimet e saj të largëta
/// (jo hiperonim a hiponim i njëri-tjetrit deri në dy hapa). Leximi i parë është ai i zgjedhur
/// për bisedën; një lexim tjetër merret vetëm kur Nura di diçka për të (një fakt i zakonshëm),
/// ose, kur është lemë tjetër, kur ka një përkufizim që mund të thuhet.
fn views(eng: &Engine, forms: &dyn Forms, chosen: &(String, u32), word: &str, rule: Witness) -> Vec<View> {
    let mut lemmas: Vec<(String, u32)> = vec![chosen.clone()];
    readings(eng, word).into_iter().filter(|(l, _)| *l != chosen.0).for_each(|x| lemmas.push(x));
    let mut out: Vec<View> = Vec::new();
    for (n, (lemma, id)) in lemmas.into_iter().take(3).enumerate() {
        let all = facts_where(eng, id, true, rule);
        let senses = forms.senses(&lemma);
        let mut reps: Vec<String> = Vec::new();
        senses.iter().for_each(|s| match reps.iter().any(|r| forms.near(r, s)) {
            true => {}
            false => reps.push(s.clone()),
        });
        let views: Vec<View> = match reps.is_empty() {
            true => {
                let mut facts = all.clone();
                weigh(forms, &lemma, &mut facts);
                vec![View { lemma: lemma.clone(), id, sense: None, facts, gloss: np_gloss(eng, id, None) }]
            }
            false => reps
                .iter()
                .take(3)
                .map(|r| {
                    let mut facts = all.clone();
                    weigh_as(forms, &lemma, r, &mut facts);
                    View { lemma: lemma.clone(), id, sense: Some(r.clone()), facts, gloss: np_gloss(eng, id, Some(r)) }
                })
                .collect(),
        };
        for (k, v) in views.into_iter().enumerate() {
            let keep = match (n, k) {
                (0, 0) => true,
                (0, _) => v.knows(),
                _ => v.knows() || v.gloss.is_some(),
            };
            match keep {
                true => out.push(v),
                false => {}
            }
        }
    }
    out
}

/// Fjalia e një leximi dhe emri që e dallon: fjalia e parë e portretit (emri = kategoria e saj),
/// ose përkufizimi kur s'ka fakte; pa kategori, emri i kuptimit («pije alkoolike», «organ»).
fn view_sentence(eng: &Engine, forms: &dyn Forms, v: &View) -> Option<(String, Option<String>)> {
    let tax = match &v.sense {
        Some(s) => taxonomy_from(eng, eng.g.get(&crate::graph::sense_key(Source::Pwn30, s))),
        None => Vec::new(),
    };
    let from_facts = || -> Option<(String, Option<String>)> {
        match v.knows() {
            true => portrait(forms, &v.lemma, &v.facts, None, &tax).and_then(|(s, _, said)| {
                let cat = said.iter().find(|f| f.kind == Kind::IsA).map(|f| f.tail.key().to_string());
                s.into_iter().next().map(|t| (t, cat))
            }),
            false => None,
        }
    };
    let from_gloss = || -> Option<(String, Option<String>)> {
        let subject = forms.noun(&v.lemma, Case::Nom, Definite::Def, Number::Sing)?;
        let (gloss, genus) = v.gloss.clone()?;
        Some((format!("{} është {gloss}.", capitalize(&subject)), Some(genus)))
    };
    let (text, cat) = from_facts().or_else(from_gloss)?;
    let name = cat.clone().or_else(|| v.sense.as_ref().and_then(|s| forms.sense_label(&v.lemma, s)));
    // Pa kategori në fjali, emri i kuptimit i dallon leximet e së njëjtës fjalë («si organ, …»).
    let text = match (&cat, &name, &v.sense) {
        (None, Some(l), Some(_)) => format!("Si {l}, {}", lower_first(&text)),
        _ => text,
    };
    Some((text, name))
}

/// «barin si bimë», «verën si pije alkoolike»: fjala dhe emri i leximit të kërkuar.
fn sense_hint(input: &str) -> Option<(String, String)> {
    let low = input.trim().trim_end_matches(['?', '.', '!']).to_lowercase();
    let (left, right) = low.split_once(" si ")?;
    match (left.split_whitespace().count(), left.starts_with("si ") || left.is_empty(), right.trim().is_empty()) {
        (1, false, false) => Some((left.trim().to_string(), right.trim().to_string())),
        _ => None,
    }
}

/// Portreti i plotë i një leximi: faktet e tij, ose përkufizimi kur s'ka fakte; pyetja çon te
/// fjala e parë që përmend përkufizimi.
fn view_portrait(eng: &Engine, forms: &dyn Forms, v: &View) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let tax = match &v.sense {
        Some(s) => taxonomy_from(eng, eng.g.get(&crate::graph::sense_key(Source::Pwn30, s))),
        None => Vec::new(),
    };
    let with_facts = match v.knows() {
        true => portrait(forms, &v.lemma, &v.facts, None, &tax),
        false => None,
    };
    let by_gloss = || {
        let (gloss, _) = v.gloss.clone()?;
        let subject = forms.noun(&v.lemma, Case::Nom, Definite::Def, Number::Sing)?;
        let rel = Rel::Sem(crate::src_perkufizime::REL.into());
        let next = eng
            .g
            .out_edges(v.id)
            .filter(|(_, e)| e.rel == rel && e.attr("genus") != Some("1"))
            .filter(|(_, e)| e.attr("df").and_then(|d| d.parse::<usize>().ok()).map(|d| d <= HUB).unwrap_or(false))
            .filter(|(_, e)| noun_node(eng, e.to))
            .filter_map(|(_, e)| lemma_of(eng, e.to))
            .next();
        let question = next
            .and_then(|n| forms.noun(&n, Case::Acc, Definite::Def, Number::Sing))
            .map(|n| format!("Dëshiron të shohim si lidhet {} me {n}?", lower_first(&subject)));
        Some((vec![format!("{} është {gloss}.", capitalize(&subject))], question, Vec::new()))
    };
    with_facts.or_else(by_gloss)
}

/// Leximi që përputhet me emrin e kërkuar («bimë», «pije alkoolike»): emri i leximit është i
/// njëjtë, ose njëri fillon me tjetrin.
fn hinted_view(eng: &Engine, forms: &dyn Forms, word: &str, label: &str, rule: Witness) -> Option<(String, (Vec<String>, Option<String>, Vec<Fact>))> {
    let chosen = readings(eng, word).into_iter().next()?;
    let vs = views(eng, forms, &chosen, word, rule);
    let first = label.split_whitespace().next().unwrap_or(label);
    let fits = |n: &str| n == label || label.starts_with(n) || n.starts_with(label) || n == first;
    let v = vs.iter().find(|v| view_sentence(eng, forms, v).and_then(|(_, n)| n).map(|n| fits(&n)).unwrap_or(false))?;
    view_portrait(eng, forms, v).map(|r| (v.lemma.clone(), r))
}

/// Përgjigjja për një fjalë me disa lexime: secili lexim me një fjali, pastaj lidhja mes tyre
/// sipas shpjegimeve (kur janë fjalë të ndryshme), dhe pyetja se për cilin e ka fjalën.
fn homograph(eng: &Engine, forms: &dyn Forms, word: &str, vs: &[View]) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let said: Vec<(String, Option<String>, &View)> =
        vs.iter().filter_map(|v| view_sentence(eng, forms, v).map(|(s, n)| (s, n, v))).take(3).collect();
    match said.len() >= 2 {
        true => {}
        false => return None,
    }
    let count = match said.len() {
        2 => "dy",
        _ => "tri",
    };
    // Fjala thuhet siç u shkrua kur është e dëshmuar ashtu; ndryshe me shkronjat e rikthyera.
    let typed = word.trim().to_lowercase();
    let shown = match eng.lex.get(&typed).map(|&id| headword(eng, id) || eng.g.out_edges(id).any(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()))) {
        Some(true) => typed,
        _ => said[0].2.lemma.clone(),
    };
    let mut out = vec![format!("Me «{shown}» mund të kesh parasysh {count} gjëra.")];
    said.iter().for_each(|(s, _, _)| out.push(s.clone()));
    // Dy fjalë të ndryshme lidhen sipas shpjegimeve të tyre.
    let pair = said.iter().enumerate().find_map(|(i, (_, _, a))| said.iter().skip(i + 1).find(|(_, _, b)| b.id != a.id).map(|(_, _, b)| (*a, *b)));
    pair.and_then(|(a, b)| link_sentence(eng, forms, a.id, b.id)).iter().for_each(|s| out.push(s.clone()));
    // Pyetja: «për barin si vend, si bimë apo për bariun?»
    let repeated = |v: &View| said.iter().filter(|(_, _, x)| x.id == v.id).count() > 1;
    let mut options: Vec<String> = Vec::new();
    let mut last_acc: Option<String> = None;
    for (_, name, v) in &said {
        let acc = forms.noun(&v.lemma, Case::Acc, Definite::Def, Number::Sing)?;
        let tail = match (repeated(v), name) {
            (true, Some(n)) => format!(" si {n}"),
            _ => String::new(),
        };
        let head = match last_acc.as_deref() == Some(acc.as_str()) && !tail.is_empty() {
            true => String::new(),
            false => format!("për {acc}"),
        };
        options.push(format!("{head}{tail}").trim().to_string());
        last_acc = Some(acc);
    }
    let question = match options.as_slice() {
        [init @ .., last] => Some(format!("E ke fjalën {} apo {last}?", init.join(", "))),
        [] => None,
    };
    let facts: Vec<Fact> = said.iter().flat_map(|(_, _, v)| v.facts.iter().filter(|f| f.salience >= TYPICAL).take(1).cloned()).collect();
    Some((out, question, facts))
}

/// Lidhja ndërmjet dy fjalëve: drejtpërdrejt, përmes një kategorie të përbashkët ose të një fjale të tretë.
pub fn relation(
    forms: &dyn Forms,
    a: &str,
    b: &str,
    fa: &[Fact],
    fb: &[Fact],
    cat_a: Option<String>,
    cat_b: Option<String>,
) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let sa = subject(forms, a)?;
    let sb = subject(forms, b)?;
    // Lidhja e drejtpërdrejtë: së pari ajo për vetë fjalën, pastaj ajo për një nënlloj të saj.
    let direct = |facts: &'_ [Fact], other: &str| -> Option<Fact> {
        let mut found: Vec<&Fact> = facts.iter().filter(|f| f.tail.key() == other || f.tail.object() == Some(other)).collect();
        found.sort_by_key(|f| (f.head_adj.is_some(), std::cmp::Reverse(f.salience)));
        found.first().map(|f| (*f).clone())
    };
    let direct_ab = direct(fa, b).map(|f| (&sa, f));
    let direct_ba = direct(fb, a).map(|f| (&sb, f));
    match direct_ab.or(direct_ba) {
        Some((s, f)) => {
            let p = predicate(forms, s, &f, Number::Sing)?;
            let head_text = match &f.head_adj {
                Some(adj) => noun_phrase(forms, &s.lemma, Some(adj), Case::Nom, Definite::Def, Number::Sing)?,
                None => s.text.clone(),
            };
            let mut out = vec![sentence(&format!("{head_text} {p}"))];
            let mut said = vec![f.clone()];
            let own = match s.lemma == a {
                true => fa,
                false => fb,
            };
            let used: BTreeSet<String> = BTreeSet::from([a.to_string(), b.to_string(), f.tail.key().to_string()]);
            let f = &f;
            let (other, other_facts) = match s.lemma == a {
                true => (&sb, fb),
                false => (&sa, fa),
            };
            let here = best(own, &[Kind::CapableOf, Kind::UsedFor], &used).and_then(|g| predicate(forms, s, g, Number::Sing).map(|p| (p, g.clone())));
            let there = best(other_facts, &[Kind::CapableOf, Kind::UsedFor, Kind::IsA], &used)
                .and_then(|g| predicate(forms, other, g, Number::Sing).map(|p| (p, g.clone())));
            let (here, there) = match f.head_adj.is_some() {
                true => (None, None),
                false => (here, there),
            };
            match (f.kind, here, there) {
                (Kind::AtLocation, Some((p, g)), _) => {
                    out.push(sentence(&format!("Aty {p}")));
                    said.push(g);
                }
                (_, _, Some((p, g))) => {
                    let first = out.pop().unwrap_or_default();
                    out.push(sentence(&format!(
                        "{}; {}, {}, {p}",
                        first.trim_end_matches('.'),
                        lower_first(&other.text),
                        on_its_side(other.gender)
                    )));
                    said.push(g);
                }
                _ => {}
            }
            let q = follow_up(forms, s, own, &used);
            return Some((out, q, said));
        }
        None => {}
    }
    let cats_a: BTreeSet<String> = fa.iter().filter(|f| f.kind == Kind::IsA).map(|f| f.tail.key().to_string()).chain(cat_a).collect();
    let cats_b: BTreeSet<String> = fb.iter().filter(|f| f.kind == Kind::IsA).map(|f| f.tail.key().to_string()).chain(cat_b).collect();
    let common = cats_a.intersection(&cats_b).next().cloned();
    let shared_place = fa
        .iter()
        .filter(|f| f.kind == Kind::AtLocation)
        .find_map(|f| fb.iter().find(|g| g.kind == Kind::AtLocation && g.tail == f.tail).map(|g| (f.clone(), g.clone())));
    let both = match (sa.gender, sb.gender) {
        (Gender::Fem, Gender::Fem) => "të dyja",
        _ => "të dy",
    };
    let mut out: Vec<String> = Vec::new();
    let mut said: Vec<Fact> = Vec::new();
    match (&common, &shared_place) {
        (Some(c), _) => {
            let plural = forms.noun(c, Case::Nom, Definite::Ind, Number::Plur)?;
            out.push(sentence(&format!("{} dhe {} janë {both} {plural}", sa.text, lower_first(&sb.text))));
        }
        (None, Some((f, g))) => {
            let place = f.tail.key();
            out.push(sentence(&format!(
                "{} dhe {} {} {both} në {place}",
                sa.text,
                lower_first(&sb.text),
                place_verb(&sa, Number::Plur)
            )));
            said.push(f.clone());
            said.push(g.clone());
        }
        (None, None) => {}
    }
    let used: BTreeSet<String> = BTreeSet::from([a.to_string(), b.to_string()]).into_iter().chain(common.clone()).collect();
    let xa = best(fa, &[Kind::CapableOf, Kind::UsedFor, Kind::HasProperty], &used);
    let xb = xa.and_then(|x| fb.iter().find(|g| g.kind == x.kind && g.tail.key() != x.tail.key() && !used.contains(g.tail.key())));
    match (xa, xb) {
        (Some(x), Some(y)) => match (predicate(forms, &sa, x, Number::Sing), predicate(forms, &sb, y, Number::Sing)) {
            (Some(px), Some(py)) => {
                out.push(sentence(&format!("{} {px}, ndërsa {} {py}", sa.text, lower_first(&sb.text))));
                said.push(x.clone());
                said.push(y.clone());
            }
            _ => {}
        },
        _ => {}
    }
    match out.is_empty() {
        true => None,
        false => {
            let q = Some("Të intereson më shumë çfarë i afron apo çfarë i dallon?".to_string());
            Some((out, q, said))
        }
    }
}

/// Shkaku: «pse qan fëmija» → çfarë e shkakton veprimin, sipas lidhjeve të dëshmuara: X shkakton
/// veprimin, X ngjall dëshirën për të, veprimi bëhet «nga X», ose veprimi ka si qëllim a kusht X.
/// Pa asnjë shkak të dëshmuar, Nura e thotë hapur (pezullim, jo mohim).
pub fn cause(forms: &dyn Forms, event: &str, subject_noun: Option<&str>, into_event: &[Fact], from_event: &[Fact]) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let reason = |f: &Fact| -> Option<String> {
        let noun = match (f.kind, &f.tail) {
            (Kind::Causes | Kind::CausesDesire, Tail::Verb { verb, .. }) => match (verb == event, f.head_noun) {
                (true, true) => Some(f.head.clone()),
                _ => None,
            },
            (Kind::CapableOf, Tail::Verb { verb, prep: Some(p), object: Some(o), .. }) => match (verb == event, p.as_str()) {
                (true, "nga" | "prej") => Some(o.clone()),
                _ => None,
            },
            (Kind::MotivatedBy | Kind::HasPrerequisite, Tail::Noun { noun, adj: None }) => Some(noun.clone()),
            _ => None,
        }?;
        match forms.is_noun(&noun) {
            true => forms.noun(&noun, Case::Nom, Definite::Def, Number::Sing),
            false => None,
        }
    };
    let mut reasons: Vec<String> = Vec::new();
    let mut said: Vec<Fact> = Vec::new();
    for f in into_event.iter().chain(from_event.iter()) {
        match (reason(f), reasons.len() < 2) {
            (Some(r), true) => match reasons.contains(&r) {
                true => {}
                false => {
                    reasons.push(r);
                    said.push(f.clone());
                }
            },
            _ => {}
        }
    }
    let verb = forms.verb(event, Number::Sing)?;
    let subject = subject_noun.and_then(|n| forms.noun(n, Case::Nom, Definite::Def, Number::Sing));
    let reasons = crate::oratoria::rendit(&reasons);
    let because = match reasons.as_slice() {
        [] => None,
        [one] => Some(format!("nga {one}")),
        [one, two, ..] => Some(format!("nga {one} ose nga {two}")),
    };
    let lead = match (&subject, &because) {
        (Some(s), Some(b)) => format!("{s} {verb} zakonisht {b}"),
        (None, Some(b)) => format!("Zakonisht {verb} {b}"),
        (Some(s), None) => format!("Ende nuk kam një shkak të dëshmuar pse {} {verb}", lower_first(s)),
        (None, None) => return None,
    };
    let question = match because {
        Some(_) => "Dëshiron të shohim më nga afër njërën nga këto arsye?".to_string(),
        None => "Dëshiron të të tregoj çfarë tjetër di për këtë?".to_string(),
    };
    Some((vec![sentence(&lead)], Some(question), said))
}

/// Rregulli i dëshmisë që vendos cilat lidhje thuhen.
pub type Witness = fn(&BTreeSet<Source>, &BTreeSet<String>, bool) -> bool;

/// Përgjigjja e plotë e bisedës për një hyrje, ose `None` kur grafi nuk ka lidhje të folshme.
pub fn converse(eng: &Engine, forms: &dyn Forms, input: &str) -> Option<Reply> {
    converse_with(eng, forms, input, spoken)
}

/// Si `converse`, me një rregull dëshmie të dhënë (për matje).
pub fn converse_with(eng: &Engine, forms: &dyn Forms, input: &str, rule: Witness) -> Option<Reply> {
    let (intent, words, asked) = read_cued(input);
    // Kur një formë e shkruar ka disa lexime («mësuesi»: trajta e shquar e «mësues», ose emri
    // femëror «mësuesi»), merret leximi me më shumë lidhje të dëshmuara; në barazim, ai me më
    // shumë lidhje në graf (fjala më e zakonshme).
    let evidence = |id: u32| {
        (
            facts_where(eng, id, true, rule).len() + facts_where(eng, id, false, rule).len(),
            eng.g.out_edges(id).count() + eng.g.in_edges(id).count(),
        )
    };
    // Leximet me dëshmi (vetë forma, trajtë e atestuar, edhe pa «ë»/«ç») kanë përparësi; vetëm
    // kur s'ka asnjë të tillë provohen rregullat e heqjes së mbaresave.
    let resolved: Vec<(String, u32)> = words
        .iter()
        .filter_map(|w| {
            let r = eng.resolve(w);
            let first = r.word.map(|id| (r.lemma.clone(), id));
            let guessed = || -> Vec<(String, u32)> {
                first
                    .clone()
                    .into_iter()
                    .chain(
                        eng.morph
                            .lemma_candidates(w)
                            .into_iter()
                            .filter(|(l, _)| l != &r.lemma && l != w)
                            .filter_map(|(l, _)| eng.lex.get(&l).map(|&id| (l, id))),
                    )
                    .collect()
            };
            let attested = readings(eng, w);
            let pool: Vec<(String, u32)> = match attested.is_empty() {
                true => guessed(),
                false => attested,
            };
            pool.into_iter().fold(None, |best: Option<((String, u32), (usize, usize))>, cand| {
                let score = evidence(cand.1);
                match best {
                    Some((b, bs)) => match score > bs {
                        true => Some((cand, score)),
                        false => Some((b, bs)),
                    },
                    None => Some((cand, score)),
                }
            })
            .map(|(c, _)| c)
        })
        .collect();
    let lemmas: Vec<String> = resolved.iter().map(|(l, _)| l.clone()).collect();
    // «verën si pije alkoolike»: leximi i kërkuar me emër, kur fjala ka atë lexim.
    let hinted = sense_hint(input).and_then(|(w, label)| hinted_view(eng, forms, &w, &label, rule));
    match hinted {
        Some((lemma, (sentences, question, facts))) => {
            return Some(Reply { intent: Intent::Portrait, words: vec![lemma], sentences, question, facts });
        }
        None => {}
    }
    let made = match (intent, resolved.as_slice()) {
        (Intent::Relation, [(a, wa), (b, wb), ..]) => {
            let mut fa = facts_where(eng, *wa, true, rule);
            let mut fb = facts_where(eng, *wb, true, rule);
            weigh(forms, a, &mut fa);
            weigh(forms, b, &mut fb);
            // Pa shenjë lidhjeje, fjala e dytë është kontekst; vetëm një lidhje e drejtpërdrejtë e
            // dëshmuar tregon se pyetet për lidhjen.
            let direct = fa.iter().any(|f| f.tail.key() == b.as_str() || f.tail.object() == Some(b.as_str()))
                || fb.iter().any(|f| f.tail.key() == a.as_str() || f.tail.object() == Some(a.as_str()));
            let found = match (asked, direct) {
                (false, false) => None,
                _ => relation(forms, a, b, &fa, &fb, category(eng, a), category(eng, b)),
            };
            // Pa lidhje të dijes së përbashkët, lidhja sipas përkufizimeve; pa asnjërën, pezullim.
            let by_definition = || {
                link_sentence(eng, forms, *wa, *wb).map(|s| (vec![s], Some("Dëshiron të të tregoj çfarë di për secilën veç e veç?".to_string()), Vec::new()))
            };
            match (found, asked) {
                (Some(r), _) => Some(r),
                (None, true) => by_definition().or_else(|| unlinked(a, b)),
                (None, false) => None,
            }
        }
        (Intent::Cause, [_, ..]) => {
            let event = resolved
                .iter()
                .find(|(l, _)| forms.is_verb(l) && !forms.is_noun(l))
                .or_else(|| resolved.iter().find(|(l, _)| forms.is_verb(l)));
            let noun = resolved.iter().find(|(l, _)| Some(l) != event.map(|(e, _)| e)).map(|(l, _)| l.as_str());
            match event {
                Some((e, we)) => cause(forms, e, noun, &facts_where(eng, *we, false, rule), &facts_where(eng, *we, true, rule)),
                None => Some((
                    vec!["Ende nuk kam një shkak të dëshmuar për këtë.".to_string()],
                    Some("Dëshiron të ma thuash me fjalë të tjera?".to_string()),
                    Vec::new(),
                )),
            }
        }
        (Intent::Purpose | Intent::Place | Intent::Ability | Intent::Portrait | Intent::Relation, [(a, wa), ..]) => {
            let mut fa = facts_where(eng, *wa, true, rule);
            weigh(forms, a, &mut fa);
            let focus: Vec<Fact> = match intent {
                Intent::Purpose => fa.iter().filter(|f| f.kind == Kind::UsedFor).cloned().collect(),
                Intent::Place => fa.iter().filter(|f| f.kind == Kind::AtLocation).cloned().collect(),
                Intent::Ability => fa.iter().filter(|f| f.kind == Kind::CapableOf).cloned().collect(),
                _ => fa.clone(),
            };
            // Portreti kërkon të paktën një lidhje të rëndësishme përtej kategorisë; ndryshe flet
            // rruga e fjalës.
            let linked = fa.iter().any(|f| f.kind != Kind::IsA && f.salience >= TYPICAL);
            // Fjala e vetme me disa lexime që Nura i njeh: i thotë të gjitha dhe i lidh.
            let several = match (intent, words.len()) {
                (Intent::Portrait, 1) => homograph(eng, forms, &words[0], &views(eng, forms, &(a.clone(), *wa), &words[0], rule)),
                _ => None,
            };
            match (intent, linked, several) {
                (_, _, Some(h)) => Some(h),
                (Intent::Purpose | Intent::Place | Intent::Ability, _, None) => list(forms, a, &focus, &fa).or_else(|| unknown_aspect(forms, a, intent)),
                (_, true, None) => portrait(forms, a, &fa, category(eng, a), &taxonomy(eng, a)),
                // Pa fakte të folshme: përkufizimi i emrit që s'ka kuptim në AlbaNet/WordNet (rruga e
                // fjalës e njeh dobët), kur mund të thuhet si «X është …».
                (Intent::Portrait, false, None) => match forms.is_noun(a) && forms.senses(a).is_empty() {
                    false => None,
                    true => {
                        let mut facts = fa.clone();
                        weigh(forms, a, &mut facts);
                        let v = View { lemma: a.clone(), id: *wa, sense: None, facts, gloss: np_gloss(eng, *wa, None) };
                        view_portrait(eng, forms, &v)
                    }
                },
                (_, false, None) => None,
            }
        }
        _ => None,
    };
    made.map(|(sentences, question, facts)| Reply { intent, words: lemmas, sentences, question, facts })
}

/// Kur dy fjalët nuk lidhen me asnjë lidhje të dëshmuar, Nura e thotë hapur (pezullim, jo mohim).
fn unlinked(a: &str, b: &str) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    Some((
        vec![format!("Ende nuk kam një lidhje të dëshmuar mes «{a}» dhe «{b}».")],
        Some("Dëshiron të të tregoj çfarë di për secilën veç e veç?".to_string()),
        Vec::new(),
    ))
}

/// Pyetja për vendin, përdorimin ose veprimin e një fjale pa lidhje të dëshmuar: Nura e thotë
/// hapur për vetë fjalën e pyetur (jo për fjalën-shenjë) dhe ofron atë që di.
fn unknown_aspect(forms: &dyn Forms, lemma: &str, intent: Intent) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let acc = forms.noun(lemma, Case::Acc, Definite::Def, Number::Sing)?;
    let what = match intent {
        Intent::Place => "një vend",
        Intent::Purpose => "një përdorim",
        _ => "një veprim",
    };
    Some((
        vec![format!("Ende nuk kam {what} të dëshmuar për {acc}.")],
        Some(format!("Dëshiron të të tregoj çfarë di për {acc}?")),
        Vec::new(),
    ))
}

/// Përgjigjja e drejtpërdrejtë për «ku», «për çfarë», «çfarë bën»: deri në tri fakte të të njëjtit lloj.
fn list(forms: &dyn Forms, lemma: &str, focus: &[Fact], all: &[Fact]) -> Option<(Vec<String>, Option<String>, Vec<Fact>)> {
    let s = subject(forms, lemma)?;
    // Faktet e zakonshme para atyre rastësore; ato për një nënlloj a kuptim tjetër nuk thuhen.
    // Faktet rastësore thuhen vetëm kur s'ka asnjë të zakonshëm.
    let mut ordered: Vec<&Fact> = focus.iter().filter(|f| f.salience > FOREIGN).collect();
    ordered.sort_by_key(|f| std::cmp::Reverse(f.salience));
    let top = ordered.first().map(|f| f.salience >= TYPICAL).unwrap_or(false);
    let picked: Vec<(String, &Fact)> = ordered
        .into_iter()
        .filter(|f| f.salience >= TYPICAL || !top)
        .filter_map(|f| predicate(forms, &s, f, Number::Sing).map(|p| (p, f)))
        .take(3)
        .collect();
    // Rendi: i pari është fakti më i rëndësishëm (siç i renditin rëndësia, dëshmia dhe pesha); të
    // tjerët vijnë sipas ligjit të Behaghel-it (pjesa më e gjatë në fund).
    let said_preds: Vec<String> = picked.iter().map(|(p, _)| p.clone()).collect();
    let preds: Vec<String> = match said_preds.split_first() {
        Some((first, rest)) => std::iter::once(first.clone()).chain(crate::oratoria::rendit(rest)).collect(),
        None => Vec::new(),
    };
    let text = format!("{} {}", s.text, join_and(&factor(&preds))?);
    let used: BTreeSet<String> = picked.iter().map(|(_, f)| f.tail.key().to_string()).chain([lemma.to_string()]).collect();
    let said: Vec<Fact> = picked.iter().map(|(_, f)| (*f).clone()).collect();
    Some((vec![sentence(&text)], follow_up(forms, &s, all, &used), said))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Toy;

    impl Forms for Toy {
        fn is_noun(&self, lemma: &str) -> bool {
            self.noun(lemma, Case::Nom, Definite::Def, Number::Sing).is_some()
        }
        fn is_verb(&self, lemma: &str) -> bool {
            self.verb(lemma, Number::Sing).is_some()
        }
        fn noun(&self, lemma: &str, case: Case, definite: Definite, number: Number) -> Option<String> {
            let t: &[(&str, &str, &str, &str, &str)] = &[
                ("qen", "qeni", "qenin", "qenit", "qenë"),
                ("mace", "macja", "macen", "maces", "mace"),
                ("shtëpi", "shtëpia", "shtëpinë", "shtëpisë", "shtëpi"),
                ("mjek", "mjeku", "mjekun", "mjekut", "mjekë"),
                ("spital", "spitali", "spitalin", "spitalit", "spitale"),
                ("kafshë", "kafsha", "kafshën", "kafshës", "kafshë"),
                ("mi", "miu", "miun", "miut", "minj"),
                ("i sëmurë", "i sëmuri", "të sëmurin", "të sëmurit", "të sëmurë"),
                ("dhimbje", "dhimbja", "dhimbjen", "dhimbjes", "dhimbje"),
                ("uri", "uria", "urinë", "urisë", "uri"),
                ("fëmijë", "fëmija", "fëmijën", "fëmijës", "fëmijë"),
                ("lodër", "lodra", "lodrën", "lodrës", "lodra"),
                ("shkollë", "shkolla", "shkollën", "shkollës", "shkolla"),
                ("ar", "ari", "arin", "arit", "arë"),
                ("metal", "metali", "metalin", "metalit", "metale"),
                ("dhëmb", "dhëmbi", "dhëmbin", "dhëmbit", "dhëmbë"),
                ("vlerë", "vlera", "vlerën", "vlerës", "vlera"),
                ("libër", "libri", "librin", "librit", "libra"),
                ("kishë", "kisha", "kishën", "kishës", "kisha"),
                ("makinë", "makina", "makinën", "makinës", "makina"),
                ("argëtim", "argëtimi", "argëtimin", "argëtimit", "argëtime"),
                ("anije", "anija", "anijen", "anijes", "anije"),
                ("hekur", "hekuri", "hekurin", "hekurit", "hekura"),
                ("derë", "dera", "derën", "derës", "dyer"),
                ("dorë", "dora", "dorën", "dorës", "duar"),
                ("xhep", "xhepi", "xhepin", "xhepit", "xhepa"),
                ("sy", "syri", "syrin", "syrit", "sy"),
                ("fytyrë", "fytyra", "fytyrën", "fytyrës", "fytyra"),
            ];
            let row = t.iter().find(|r| r.0 == lemma)?;
            match (case, definite, number) {
                (Case::Nom, Definite::Def, Number::Sing) => Some(row.1.to_string()),
                (Case::Acc, Definite::Def, Number::Sing) => Some(row.2.to_string()),
                (Case::Dat, Definite::Def, Number::Sing) => Some(row.3.to_string()),
                (Case::Nom, Definite::Ind, Number::Plur) => Some(row.4.to_string()),
                (Case::Acc, Definite::Def, Number::Plur) => match lemma {
                    "mi" => Some("minjtë".to_string()),
                    _ => None,
                },
                (Case::Acc, Definite::Ind, Number::Plur) => Some(row.4.to_string()),
                (Case::Nom | Case::Acc, Definite::Ind, Number::Sing) => Some(row.0.to_string()),
                _ => None,
            }
        }
        fn gender(&self, noun: &str) -> Option<Gender> {
            match noun {
                "mace" | "shtëpi" | "kafshë" | "dhimbje" | "uri" | "vlerë" | "kishë" | "makinë" | "anije" | "derë" | "dorë" | "fytyrë" => {
                    Some(Gender::Fem)
                }
                _ => Some(Gender::Masc),
            }
        }
        fn animate(&self, noun: &str) -> Option<bool> {
            Some(matches!(noun, "qen" | "mace" | "mjek" | "fëmijë" | "mi"))
        }
        fn human(&self, noun: &str) -> bool {
            matches!(noun, "mjek" | "fëmijë")
        }
        fn worker(&self, noun: &str) -> bool {
            noun == "mjek"
        }
        fn verb(&self, lemma: &str, number: Number) -> Option<String> {
            let t: &[(&str, &str, &str)] = &[("ruaj", "ruan", "ruajnë"), ("gjuaj", "gjuan", "gjuajnë"), ("kuroj", "kuron", "kurojnë"), ("qaj", "qan", "qajnë"), ("leh", "leh", "lehin"), ("mbyll", "mbyll", "mbyllin")];
            let row = t.iter().find(|r| r.0 == lemma)?;
            match number {
                Number::Sing => Some(row.1.to_string()),
                Number::Plur => Some(row.2.to_string()),
            }
        }
        fn subjunctive(&self, lemma: &str, _n: Number) -> Option<String> {
            match lemma {
                "luaj" => Some("luajë".into()),
                _ => None,
            }
        }
        fn participle(&self, lemma: &str) -> Option<String> {
            match lemma {
                "ruaj" => Some("ruajtur".into()),
                "ngarkoj" => Some("ngarkuar".into()),
                "hekuros" => Some("hekurosur".into()),
                _ => None,
            }
        }
        fn adjective_np(&self, lemma: &str, gender: Gender, _n: Number, case: Case, definite: Definite) -> Option<String> {
            match (lemma, gender, case, definite) {
                ("vogël", Gender::Fem, Case::Nom, Definite::Ind) => Some("e vogël".into()),
                ("shenjtë", Gender::Masc, Case::Nom, Definite::Def) => Some("i shenjtë".into()),
                _ => None,
            }
        }
        fn adjective(&self, lemma: &str, gender: Gender, _n: Number) -> Option<String> {
            match (lemma, gender) {
                ("besnik", Gender::Masc) => Some("besnik".into()),
                ("besnik", _) => Some("besnike".into()),
                _ => None,
            }
        }
        fn class(&self, lemma: &str) -> Option<u8> {
            match lemma {
                "ar" | "hekur" => Some(klasa::SUBSTANCE),
                "dhëmb" | "dorë" | "sy" | "fytyrë" => Some(klasa::BODY),
                "xhep" | "makinë" | "derë" | "libër" | "kishë" => Some(klasa::ARTIFACT),
                _ => None,
            }
        }
        fn reading(&self, lemma: &str, en: &str) -> Reading {
            match (lemma, en) {
                ("makinë", "machine") => Reading::Other,
                ("makinë", "car") => Reading::Same,
                _ => Reading::Unknown,
            }
        }
        fn transitive_only(&self, verb: &str) -> bool {
            verb == "mbyll"
        }
        fn verb_mid(&self, lemma: &str, _n: Number) -> Option<String> {
            match lemma {
                "mbyll" => Some("mbyllet".into()),
                _ => None,
            }
        }
    }

    fn fact(kind: Kind, head: &str, tail: Tail) -> Fact {
        Fact {
            kind,
            head: head.into(),
            object: ObjectForm::DefSing,
            tail,
            sources: BTreeSet::from([Source::ConceptNet57, Source::ModelClaude]),
            datasets: BTreeSet::from(["/d/conceptnet/4/en".to_string()]),
            strong: true,
            head_noun: true,
            witness_rank: 3,
            weight: 2000,
            rows: 1,
            head_adj: None,
            en_head: String::new(),
            en_tail: String::new(),
            salience: TYPICAL,
        }
    }

    fn verb(v: &str, o: Option<&str>) -> Tail {
        Tail::Verb { verb: v.into(), prep: None, object: o.map(str::to_string), adj: None, adv: None }
    }

    #[test]
    fn reading_finds_intent_and_content_words() {
        assert_eq!(read("çfarë lidhje ka deti me kripën").0, Intent::Relation);
        assert_eq!(read("çfarë lidhje ka deti me kripën").1, vec!["deti".to_string(), "kripën".to_string()]);
        assert_eq!(read("qeni dhe macja").0, Intent::Relation);
        assert_eq!(read("pse qan fëmija").0, Intent::Cause);
        assert_eq!(read("ku jeton ariu").0, Intent::Place);
        assert_eq!(read("ku jeton ariu").1, vec!["ariu".to_string()]);
        assert_eq!(read("ku punon mjeku").1, vec!["mjeku".to_string()]);
        assert_eq!(read("pse flenë njerëzit").1, vec!["flenë".to_string(), "njerëzit".to_string()]);
        assert_eq!(read("ç'është zemra").1, vec!["zemra".to_string()]);
        assert_eq!(read("për çfarë shërben thika").0, Intent::Purpose);
        assert_eq!(read("çfarë bën mjeku").0, Intent::Ability);
        assert_eq!(read("qen").0, Intent::Portrait);
    }

    #[test]
    fn object_form_follows_the_english_source() {
        assert_eq!(object_form("cat|hunt_mice"), ObjectForm::DefPlur);
        assert_eq!(object_form("dog|guard_house"), ObjectForm::DefSing);
        assert_eq!(object_form("cow|give_milk"), ObjectForm::Mass);
        assert_eq!(object_form("dog|chase_cats"), ObjectForm::DefPlur);
        assert_eq!(object_form("fish|swim"), ObjectForm::DefSing);
    }

    #[test]
    fn tails_parse() {
        assert_eq!(Tail::parse("v=ruaj;o=shtëpi"), Some(verb("ruaj", Some("shtëpi"))));
        assert_eq!(Tail::parse("n=spital"), Some(Tail::noun("spital")));
        assert_eq!(
            Tail::parse("v=luaj;p=me;o=lodër"),
            Some(Tail::Verb { verb: "luaj".into(), prep: Some("me".into()), object: Some("lodër".into()), adj: None, adv: None })
        );
        assert_eq!(Tail::parse("n=kafshë;a=vogël"), Some(Tail::Noun { noun: "kafshë".into(), adj: Some("vogël".into()) }));
        assert_eq!(Tail::parse("cop=i lumtur"), None);
        assert_eq!(Tail::parse("a=kripur"), Some(Tail::Adj("kripur".into())));
        assert_eq!(Tail::parse("cop=n:kafshë"), None);
    }

    #[test]
    fn witnesses_decide_what_is_spoken() {
        let one = BTreeSet::from([Source::ConceptNet57]);
        let two = BTreeSet::from([Source::ConceptNet57, Source::ModelClaude]);
        let model = BTreeSet::from([Source::ModelClaude]);
        let ds1 = BTreeSet::from(["/d/conceptnet/4/en".to_string()]);
        let ds2 = BTreeSet::from(["/d/conceptnet/4/en".to_string(), "/d/verbosity".to_string()]);
        assert!(!spoken(&one, &ds1, true));
        assert!(spoken(&two, &ds1, false));
        assert!(spoken(&one, &ds2, true));
        assert!(!spoken(&one, &ds2, false), "dy grupe të dhënash, por përkthim i dobët");
        assert!(!spoken(&model, &ds2, true));
        let lex = BTreeSet::from([Source::ConceptNet57, Source::Pwn30]);
        assert!(spoken(&lex, &ds1, false), "ConceptNet + përkufizimi i WordNet-it");
        assert!(!spoken(&BTreeSet::from([Source::Pwn30]), &ds1, true), "burimi leksikor vetëm nuk mjafton");
    }

    #[test]
    fn portrait_links_category_function_and_place() {
        let f = vec![
            fact(Kind::IsA, "qen", Tail::noun("kafshë")),
            fact(Kind::CapableOf, "qen", verb("ruaj", Some("shtëpi"))),
            fact(Kind::AtLocation, "qen", Tail::noun("shtëpi")),
            fact(Kind::HasProperty, "qen", Tail::Adj("besnik".into())),
            fact(Kind::CapableOf, "qen", verb("leh", None)),
        ];
        let (s, q, said) = portrait(&Toy, "qen", &f, None, &[]).unwrap();
        assert_eq!(s[0], "Qeni është kafshë që ruan shtëpinë.");
        // Renditja e dy fakteve e zgjedh oratoria (formula të deklaruara), jo rendi i shkrimit.
        let both = ["Është besnik dhe leh.".to_string(), "Leh dhe është besnik.".to_string()];
        assert_eq!(Some(&s[1]), crate::oratoria::zgjidh(&both));
        assert!(said.len() >= 3);
        assert!(q.is_none() || q.unwrap().ends_with('?'));
    }

    #[test]
    fn relation_prefers_a_direct_link_then_explains_it() {
        let fa = vec![
            fact(Kind::AtLocation, "mjek", Tail::noun("spital")),
            fact(Kind::CapableOf, "mjek", verb("kuroj", Some("i sëmurë"))),
        ];
        let (s, _, _) = relation(&Toy, "mjek", "spital", &fa, &[], None, None).unwrap();
        assert_eq!(s, vec!["Mjeku punon zakonisht në spital.".to_string(), "Aty kuron të sëmurin.".to_string()]);
    }

    #[test]
    fn relation_through_a_shared_category_with_contrast() {
        let fa = vec![fact(Kind::IsA, "qen", Tail::noun("kafshë")), fact(Kind::CapableOf, "qen", verb("ruaj", Some("shtëpi")))];
        let mut hunt = fact(Kind::CapableOf, "mace", verb("gjuaj", Some("mi")));
        hunt.object = object_form("cat|hunt_mice");
        let fb = vec![fact(Kind::IsA, "mace", Tail::noun("kafshë")), hunt];
        let (s, q, _) = relation(&Toy, "qen", "mace", &fa, &fb, None, None).unwrap();
        assert_eq!(s[0], "Qeni dhe macja janë të dy kafshë.");
        assert_eq!(s[1], "Qeni ruan shtëpinë, ndërsa macja gjuan minjtë.");
        assert!(q.is_some());
    }

    #[test]
    fn prepositions_and_adjectives_take_their_case() {
        let small = fact(Kind::IsA, "fëmijë", Tail::parse("n=kafshë;a=vogël").unwrap());
        let s = subject(&Toy, "fëmijë").unwrap();
        assert_eq!(predicate(&Toy, &s, &small, Number::Sing).as_deref(), Some("është kafshë e vogël"));
        let mut toys = fact(Kind::CapableOf, "fëmijë", Tail::parse("v=ruaj;p=me;o=lodër").unwrap());
        toys.object = object_form("child|keep_with_toys");
        assert_eq!(predicate(&Toy, &s, &toys, Number::Sing).as_deref(), Some("ruan me lodra"));
        let odd = fact(Kind::CapableOf, "fëmijë", Tail::parse("v=ruaj;p=pranë;o=lodër").unwrap());
        assert_eq!(predicate(&Toy, &s, &odd, Number::Sing), None, "parafjala pa rregull rase nuk thuhet");
    }

    #[test]
    fn the_category_confirmed_by_the_taxonomy_comes_first() {
        let f = vec![
            fact(Kind::IsA, "qen", Tail::noun("shok")),
            fact(Kind::IsA, "qen", Tail::noun("kafshë")),
            fact(Kind::CapableOf, "qen", verb("leh", None)),
        ];
        let tax = vec![("mishngrënës".to_string(), 2), ("kafshë".to_string(), 5)];
        let (s, _, _) = portrait(&Toy, "qen", &f, None, &tax).unwrap();
        assert!(s[0].starts_with("Qeni është kafshë"), "{}", s[0]);
    }

    #[test]
    fn shared_verbs_are_said_once() {
        let p = vec!["jeton zakonisht në ujë".to_string(), "jeton zakonisht në oqean".to_string(), "jeton zakonisht në akuarium".to_string()];
        assert_eq!(join_and(&factor(&p)).unwrap(), "jeton zakonisht në ujë, në oqean dhe në akuarium");
        let u = vec!["përdoret për të prerë".to_string(), "përdoret për qëllim".to_string()];
        assert_eq!(join_and(&factor(&u)).unwrap(), "përdoret për të prerë dhe për qëllim");
        let e = vec!["përdoret për të ngrënë".to_string(), "përdoret për të ngrënë lëngjet".to_string(), "përdoret për të ngrënë kosin".to_string()];
        assert_eq!(join_and(&factor(&e)).unwrap(), "përdoret për të ngrënë lëngjet dhe kosin");
        let d = vec!["ruan shtëpinë".to_string(), "leh".to_string()];
        assert_eq!(join_and(&factor(&d)).unwrap(), "ruan shtëpinë dhe leh");
    }

    #[test]
    fn a_person_is_somewhere_a_worker_works_there() {
        let at_school = fact(Kind::AtLocation, "fëmijë", Tail::noun("shkollë"));
        let child = subject(&Toy, "fëmijë").unwrap();
        assert_eq!(predicate(&Toy, &child, &at_school, Number::Sing).as_deref(), Some("është zakonisht në shkollë"));
        let at_hospital = fact(Kind::AtLocation, "mjek", Tail::noun("spital"));
        let doctor = subject(&Toy, "mjek").unwrap();
        assert_eq!(predicate(&Toy, &doctor, &at_hospital, Number::Sing).as_deref(), Some("punon zakonisht në spital"));
    }

    #[test]
    fn a_question_without_witnessed_facts_is_answered_about_the_word_asked() {
        let (s, q, said) = unknown_aspect(&Toy, "qen", Intent::Place).unwrap();
        assert_eq!(s[0], "Ende nuk kam një vend të dëshmuar për qenin.");
        assert_eq!(q.as_deref(), Some("Dëshiron të të tregoj çfarë di për qenin?"));
        assert!(said.is_empty());
    }

    #[test]
    fn cause_names_the_reasons() {
        let into = vec![
            fact(Kind::Causes, "dhimbje", verb("qaj", None)),
            fact(Kind::Causes, "uri", verb("qaj", None)),
        ];
        let (s, _, _) = cause(&Toy, "qaj", Some("fëmijë"), &into, &[]).unwrap();
        assert_eq!(s[0], "Fëmija qan zakonisht nga dhimbja ose nga uria.");
        let pain = vec![fact(Kind::CapableOf, "fëmijë", Tail::parse("v=qaj;p=nga;o=dhimbje").unwrap())];
        let (s, _, _) = cause(&Toy, "qaj", Some("fëmijë"), &pain, &[]).unwrap();
        assert_eq!(s[0], "Fëmija qan zakonisht nga dhimbja.");
        let (s, q, said) = cause(&Toy, "leh", Some("qen"), &[], &[]).unwrap();
        assert_eq!(s[0], "Ende nuk kam një shkak të dëshmuar pse qeni leh.");
        assert!(q.is_some() && said.is_empty());
    }

    #[test]
    fn unsure_forms_are_not_spoken() {
        let f = vec![fact(Kind::UsedFor, "qen", verb("shkruaj", None))];
        assert!(portrait(&Toy, "qen", &f, None, &[]).is_none());
    }

    fn weak(mut f: Fact) -> Fact {
        f.weight = 1000;
        f.rows = 1;
        f
    }

    #[test]
    fn a_true_but_incidental_fact_is_not_the_main_fact() {
        let mut f = vec![
            fact(Kind::IsA, "ar", Tail::noun("metal")),
            weak(fact(Kind::AtLocation, "ar", Tail::noun("dhëmb"))),
            fact(Kind::HasA, "ar", Tail::noun("vlerë")),
        ];
        weigh(&Toy, "ar", &mut f);
        assert_eq!(f[1].salience, INCIDENTAL, "lënda s'ka «vend të zakonshëm», dhe ka vetëm një pohues");
        let (s, _, said) = portrait(&Toy, "ar", &f, None, &[]).unwrap();
        assert_eq!(s, vec!["Ari është metal.".to_string(), "Ka vlerë.".to_string()]);
        assert!(said.iter().all(|x| x.tail.key() != "dhëmb"));
        // Kur pyetet drejtpërdrejt «ku», thuhet si e mundshme, jo si e zakonshme.
        let places: Vec<Fact> = f.iter().filter(|x| x.kind == Kind::AtLocation).cloned().collect();
        let (s, _, _) = list(&Toy, "ar", &places, &f).unwrap();
        assert_eq!(s[0], "Ari mund të gjendet në dhëmb.");
    }

    #[test]
    fn a_fact_about_a_subkind_is_said_of_the_subkind_only() {
        let mut holy = fact(Kind::AtLocation, "libër", Tail::noun("kishë"));
        holy.head_adj = Some("shenjtë".into());
        let mut f = vec![holy, fact(Kind::IsA, "libër", Tail::noun("metal"))];
        weigh(&Toy, "libër", &mut f);
        assert_eq!(f[0].salience, FOREIGN);
        let (s, _, _) = relation(&Toy, "libër", "kishë", &f, &[], None, None).unwrap();
        assert_eq!(s[0], "Libri i shenjtë mund të gjendet në kishë.");
        let said = portrait(&Toy, "libër", &f, None, &[]).map(|(s, _, _)| s.concat()).unwrap_or_default();
        assert!(!said.contains("kishë"));
    }

    #[test]
    fn a_fact_of_another_sense_is_left_out() {
        let mut machine = fact(Kind::UsedFor, "makinë", verb("ngarkoj", Some("anije")));
        machine.en_head = "machine".into();
        let mut car = fact(Kind::UsedFor, "makinë", Tail::noun("argëtim"));
        car.en_head = "car".into();
        let mut f = vec![machine, car];
        weigh(&Toy, "makinë", &mut f);
        assert_eq!((f[0].salience, f[1].salience), (FOREIGN, TYPICAL));
        let (s, _, _) = portrait(&Toy, "makinë", &f, None, &[]).unwrap();
        assert_eq!(s[0], "Makina përdoret për argëtim.");
    }

    #[test]
    fn a_fact_that_restates_the_word_is_not_the_main_fact() {
        let mut f = vec![fact(Kind::UsedFor, "hekur", verb("hekuros", None)), fact(Kind::IsA, "hekur", Tail::noun("metal"))];
        weigh(&Toy, "hekur", &mut f);
        assert_eq!(f[0].salience, INCIDENTAL);
    }

    #[test]
    fn a_change_of_state_without_object_takes_the_middle_voice() {
        let mut close = fact(Kind::CapableOf, "derë", verb("mbyll", None));
        close.en_tail = "close".into();
        let s = subject(&Toy, "derë").unwrap();
        assert_eq!(predicate(&Toy, &s, &close, Number::Sing).as_deref(), Some("mbyllet"));
        // Pa folje angleze me ndërrim shkakor, folja mbetet veprore.
        close.en_tail = "lock".into();
        assert_eq!(predicate(&Toy, &s, &close, Number::Sing).as_deref(), Some("mbyll"));
    }

    #[test]
    fn a_body_part_is_placed_in_the_body() {
        let mut f = vec![fact(Kind::AtLocation, "dorë", Tail::noun("xhep")), fact(Kind::AtLocation, "sy", Tail::noun("fytyrë"))];
        weigh(&Toy, "dorë", &mut f[..1]);
        weigh(&Toy, "sy", &mut f[1..]);
        assert_eq!((f[0].salience, f[1].salience), (INCIDENTAL, TYPICAL));
    }
}
