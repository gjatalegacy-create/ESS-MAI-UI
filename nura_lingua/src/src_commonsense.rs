//! Shtresa e dijes së përbashkët: ConceptNet 5.7 (pjesa anglisht) e kthyer në lidhje ndërmjet
//! fjalëve shqipe.
//!
//! Rregullat e deklaruara:
//! - Mbahen rreshtat e OMCS-së, Verbosity-t dhe Wiktionary-t. WordNet-i hidhet (është në graf si
//!   burim më vete), po ashtu OpenCyc dhe DBpedia (ontologji dhe emra të përveçëm).
//! - Fjala angleze kalon në shqip vetëm kur pajtohen dy ura të pavarura:
//!   (A) Wiktionary: fjala shqipe e ka fjalën angleze në glosën e parë leksikore të hyrjes;
//!   (B) WordNet: fjala shqipe është etiketë AlbaNet/BabelNet e një sinseti të fjalës angleze me
//!   të njëjtën pjesë ligjërate, me kuptimet sipas shpeshtësisë në korpus.
//!   Pajtimi jep `AB`; pajtimi te kuptimi më i shpeshtë i WordNet-it me një kuptim tjetër të
//!   Wiktionary-t, ose me një sinonim të dëshmuar nga një fjalor, jep `AB2`. Një kandidat i vetëm
//!   në njërën urë, kur tjetra s'ka asgjë, jep `A1`/`B1` (i dobët, `weak=1`). Paqartësia ose
//!   kundërthënia → HOLD: rreshti nuk merret (HOLD nuk është mohim).
//! - Pajtimi te një kuptim i dytësor i WordNet-it s'mjafton kur Wiktionary e jep fjalën angleze me
//!   një fjalë tjetër («letter»: «letër», jo «shkronjë»); dy fjalë të barabarta në çdo dëshmi s'zgjidhen
//!   sipas alfabetit.
//! - Leksiku i modelit është një dëshmitar i përkthimit, jo autoritet: kur pajtohet me urën e vetme
//!   të një skaji të dobët, skaji e shënon (`witness=model_claude`); kur s'pajtohet, s'e heq. Kur ura
//!   e vetme kundërshtohet (një fjalë tjetër në Wiktionary, ose fjalë të WordNet-it vetëm në kuptime
//!   të tjera), Wiktionary vetëm s'mjafton: pa dëshmitarin e dytë → HOLD.
//! - Fraza: një fjalë; folje + kundrinor; folje + parafjalë + emër; folje + mbiemër + emër;
//!   folje + ndajfolje; mbiemër + emër; «be X» (kopulë). Fraza që WordNet-i e ka si lemë (p.sh.
//!   «ice cream», «cellular phone») përkthehet si njësi dhe nuk zbërthehet. Një fjalë e pahartuar →
//!   HOLD. Kalket mbeten jashtë: folja e lehtë me emër abstrakt («have fun», «pay attention»), folja
//!   frazore («take in film»), folja me gerund («enjoy doing»); pjesorja s'bëhet folje veprore
//!   («punished»), por mbiemri i WordNet-it jep kopulën («tired» → «është i lodhur»).
//! - Skaji: `Word(sq, koka) → Word(sq, fjala kryesore e bishtit)`, `Rel::Sem("CS_…")`, burimi
//!   ConceptNet 5.7, atributet `en`, `w`, `ds`, `rows`, `head`, `tail`, `map`, `cid`. E njëjta
//!   treshe shqipe nga disa rreshta → një skaj, me peshën më të madhe, listën e dataset-eve dhe
//!   numrin e rreshtave.
//! - Dëshmia e modelit për treshen (`model/sq_model_deshmi_cs.tsv`, vetëm rreshtat `CONFIRM`) është
//!   dëshmitari i dytë i lidhjes: treshja me të njëjtin `cid` merr pranë skajit të ConceptNet-it një
//!   skaj të dyfishtë me burim `model_claude` (`witness=cs`). Treshja thuhet vetëm me dy dëshmitarë
//!   (`spoken`); dëshmia e modelit vetëm s'mjafton, dhe treshja pa dëshmi mbetet vetëm me ConceptNet-in.
//!
//! Stil: pa if/else — vetëm match.

use crate::graph::*;
use crate::import::ImportReport;
use crate::sha256::sha256_hex;
use crate::src_kaikki_en::{destress, gloss_form_of};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub const TSV: &str = "raw/conceptnet57/conceptnet-5.7.0.en-commonsense.tsv";

/// Indekset e WordNet 3.0: renditja e kuptimeve sipas shpeshtësisë.
pub const PWN_INDEX: [(&str, Pos); 4] = [
    ("raw/pwn30/index.noun", Pos::Noun),
    ("raw/pwn30/index.verb", Pos::Verb),
    ("raw/pwn30/index.adj", Pos::Adj),
    ("raw/pwn30/index.adv", Pos::Adv),
];

/// Indeksi i kuptimeve të WordNet 3.0: sa herë është shënuar çdo kuptim në korpus.
pub const PWN_SENSES: &str = "raw/pwn30/index.sense";

/// Parashtesa e marrëdhënieve të kësaj shtrese.
pub const PREFIX: &str = "CS_";

/// Dëshmia e modelit për treshet, nga dosja `ui_ess_mai`.
pub const WITNESS: &str = "model/sq_model_deshmi_cs.tsv";

/// Vlera e atributit `witness` te skaji i dyfishtë i dëshmisë së modelit.
pub const WITNESS_CS: &str = "cs";

/// Forma e pritur e një ane të marrëdhënies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Emër, ose emër + mbiemër.
    Thing,
    /// Veprim ose emër; fjala e vetme merr pjesën e ligjëratës më të shpeshtë në WordNet.
    Event,
    /// Veprim ose emër; fjala që është folje lexohet si folje.
    Act,
    /// Veprimtari (koka e nënngjarjeve, kushteve dhe qëllimeve, dhe vetë nënngjarjet): fjala që
    /// WordNet-i e njeh si folje lexohet vetëm si folje; kur folja mbetet HOLD, mbetet edhe rreshti.
    Activity,
    /// Mbiemër.
    Quality,
    /// Folje (edhe pjesore: «eaten» → «eat»).
    Action,
    /// E njëjta pjesë ligjërate në të dy anët (e kundërta).
    Same,
}

const RELATIONS: &[(&str, &str, Slot, Slot)] = &[
    ("/r/IsA", "CS_IS_A", Slot::Thing, Slot::Thing),
    ("/r/PartOf", "CS_PART_OF", Slot::Thing, Slot::Thing),
    ("/r/HasA", "CS_HAS_A", Slot::Thing, Slot::Thing),
    ("/r/MadeOf", "CS_MADE_OF", Slot::Thing, Slot::Thing),
    ("/r/UsedFor", "CS_USED_FOR", Slot::Thing, Slot::Act),
    ("/r/CapableOf", "CS_CAPABLE_OF", Slot::Thing, Slot::Act),
    ("/r/AtLocation", "CS_AT_LOCATION", Slot::Thing, Slot::Thing),
    ("/r/LocatedNear", "CS_LOCATED_NEAR", Slot::Thing, Slot::Thing),
    ("/r/HasProperty", "CS_HAS_PROPERTY", Slot::Thing, Slot::Quality),
    ("/r/Causes", "CS_CAUSES", Slot::Event, Slot::Event),
    ("/r/CausesDesire", "CS_CAUSES_DESIRE", Slot::Event, Slot::Act),
    ("/r/HasPrerequisite", "CS_HAS_PREREQUISITE", Slot::Activity, Slot::Event),
    ("/r/HasSubevent", "CS_HAS_SUBEVENT", Slot::Activity, Slot::Activity),
    ("/r/HasFirstSubevent", "CS_HAS_FIRST_SUBEVENT", Slot::Activity, Slot::Activity),
    ("/r/HasLastSubevent", "CS_HAS_LAST_SUBEVENT", Slot::Activity, Slot::Activity),
    ("/r/MotivatedByGoal", "CS_MOTIVATED_BY", Slot::Activity, Slot::Event),
    ("/r/Desires", "CS_DESIRES", Slot::Thing, Slot::Event),
    ("/r/NotDesires", "CS_NOT_DESIRES", Slot::Thing, Slot::Event),
    ("/r/ReceivesAction", "CS_RECEIVES_ACTION", Slot::Thing, Slot::Action),
    ("/r/CreatedBy", "CS_CREATED_BY", Slot::Thing, Slot::Thing),
    ("/r/Antonym", "CS_ANTONYM", Slot::Same, Slot::Same),
    ("/r/MannerOf", "CS_MANNER_OF", Slot::Action, Slot::Action),
    ("/r/ObstructedBy", "CS_OBSTRUCTED_BY", Slot::Event, Slot::Event),
    ("/r/Entails", "CS_ENTAILS", Slot::Action, Slot::Action),
    ("/r/SymbolOf", "CS_SYMBOL_OF", Slot::Thing, Slot::Thing),
    ("/r/NotCapableOf", "CS_NOT_CAPABLE_OF", Slot::Thing, Slot::Act),
    ("/r/NotHasProperty", "CS_NOT_HAS_PROPERTY", Slot::Thing, Slot::Quality),
    ("/r/NotUsedFor", "CS_NOT_USED_FOR", Slot::Thing, Slot::Act),
];

/// Emri shqip i marrëdhënies ConceptNet («/r/UsedFor» → «CS_USED_FOR»).
pub fn rel_name(cn: &str) -> Option<&'static str> {
    RELATIONS.iter().find(|r| r.0 == cn).map(|r| r.1)
}

/// OMCS, Verbosity dhe Wiktionary mbahen; WordNet, OpenCyc dhe DBpedia jo.
pub fn kept_dataset(ds: &str) -> bool {
    ds == "/d/conceptnet/4/en" || ds == "/d/verbosity" || ds.starts_with("/d/wiktionary/")
}

/// Cilësia e hartimit anglisht → shqip (më e mira e para).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapCode {
    /// Të dyja urat pajtohen te kuptimi kryesor i Wiktionary-t.
    AB,
    /// Të dyja urat pajtohen: një kuptim tjetër i Wiktionary-t dhe kuptimi më i shpeshtë i WordNet-it.
    AB2,
    /// Vetëm Wiktionary, një kandidat i vetëm (i dobët).
    A1,
    /// Vetëm WordNet, një kandidat i vetëm në kuptimin më të shpeshtë (i dobët).
    B1,
}

impl MapCode {
    pub fn code(self) -> &'static str {
        match self {
            MapCode::AB => "AB",
            MapCode::AB2 => "AB2",
            MapCode::A1 => "A1",
            MapCode::B1 => "B1",
        }
    }
    /// I dobët: vetëm një urë e dëshmon.
    pub fn weak(self) -> bool {
        matches!(self, MapCode::A1 | MapCode::B1)
    }
    pub fn parse(s: &str) -> Option<MapCode> {
        match s {
            "AB" => Some(MapCode::AB),
            "AB2" => Some(MapCode::AB2),
            "A1" => Some(MapCode::A1),
            "B1" => Some(MapCode::B1),
            _ => None,
        }
    }
}

/// Arsyeja pse një rresht mbetet jashtë (HOLD nuk është mohim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Hold {
    Dataset,
    Relation,
    Shape,
    Unknown,
    Ambiguous,
    Conflict,
    SelfLoop,
}

impl Hold {
    fn label(self) -> &'static str {
        match self {
            Hold::Dataset => "dataset i përjashtuar",
            Hold::Relation => "marrëdhënie jashtë listës",
            Hold::Shape => "frazë jashtë formave të lejuara",
            Hold::Unknown => "fjalë pa përkthim",
            Hold::Ambiguous => "përkthim i paqartë",
            Hold::Conflict => "urat kundërshtohen",
            Hold::SelfLoop => "koka dhe bishti e njëjta fjalë",
        }
    }
}

/// Fjalë shqipe e hartuar (ose fjalë funksionale e fiksuar, pa nyje dhe pa kod).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub key: &'static str,
    pub lemma: String,
    pub word: Option<u32>,
    pub code: Option<MapCode>,
    pub en: String,
}

/// Frazë shqipe e strukturuar; `main` është fjala ku lidhet skaji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase {
    pub parts: Vec<Part>,
    pub main: usize,
}

impl Phrase {
    /// «v=ruaj;o=shtëpi», «n=ujë;a=kripur», «cop=lumtur».
    pub fn structured(&self) -> String {
        self.parts.iter().map(|p| format!("{}={}", p.key, p.lemma)).collect::<Vec<_>>().join(";")
    }
    pub fn main_word(&self) -> Option<u32> {
        self.parts.get(self.main).and_then(|p| p.word)
    }
    /// Kodi më i dobët i fjalëve të hartuara.
    pub fn code(&self) -> MapCode {
        self.parts.iter().filter_map(|p| p.code).max().unwrap_or(MapCode::AB)
    }
}

const DETERMINERS: &[&str] = &["a", "an", "the", "your", "my", "his", "her", "their", "its", "it's", "one's", "some", "any", "our"];

/// Fjalë që e bëjnë frazën të paqartë ose jo-leksikore: rreshti mbetet jashtë.
const BLOCKERS: &[&str] = &[
    "not", "no", "never", "and", "or", "but", "nor", "it", "them", "him", "me", "you", "us", "i", "he", "she", "we", "they",
    "yourself", "himself", "herself", "itself", "themselves", "oneself", "myself", "ourselves", "something", "someone",
    "somebody", "anything", "anyone", "anybody", "everything", "everyone", "nothing", "nobody", "this", "that", "these",
    "those", "there", "here", "if", "when", "very", "too", "so", "can", "will", "would", "should", "could", "self",
];

/// Foljet e lehta: me emër abstrakt kuptimin e mban emri dhe shqipja zgjedh foljen e vet («have fun»,
/// «make decision», «pay attention»), prandaj s'përkthehen fjalë për fjalë.
const LIGHT_VERBS: &[&str] = &["have", "take", "make", "give", "get", "do", "pay", "throw", "catch", "start"];

/// Skedarët leksikografikë abstraktë të emrave në WordNet: veprim, cilësi, njohje, komunikim, ngjarje,
/// ndjenjë, motiv, dukuri, proces, gjendje, kohë.
const ABSTRACT_LEX: &[u8] = &[4, 7, 9, 10, 11, 12, 16, 19, 22, 26, 28];

/// Ushqimi pas «have»/«take» është ngrënie («have lunch» = «ha drekë»).
const FOOD_LEX: u8 = 13;

/// Parafjalët me përkthim të sigurt. «to» merret vetëm pas foljeve të lëvizjes.
const PREPOSITIONS: &[(&str, &str)] = &[
    ("in", "në"),
    ("into", "në"),
    ("inside", "brenda"),
    ("with", "me"),
    ("without", "pa"),
    ("from", "nga"),
    ("under", "nën"),
    ("through", "nëpër"),
    ("to", "në"),
];

const MOTION: &[&str] = &["go", "come", "walk", "run", "drive", "travel", "fly", "return", "move", "ride", "swim", "sail", "hurry", "rush"];

/// Pjesëzat e foljeve frazore dhe parafjalët: kuptimi i tyre varet nga folja («give up», «go out»),
/// prandaj nuk përkthehen veçmas si ndajfolje, emër apo mbiemër.
const PARTICLES: &[&str] = &[
    "up", "down", "out", "off", "on", "in", "over", "away", "back", "around", "about", "along", "through", "by",
    "from", "to", "at", "for", "of", "into", "onto", "with", "without", "after", "before", "together", "apart",
    "aside", "forward", "under", "upon", "across", "behind", "inside", "outside",
];

/// Trajtat e foljes «be»: vetëm në krye të frazës «be X» (kopula).
const COPULA: &[&str] = &["be", "being", "been", "is", "are", "was", "were", "am"];

/// Emra që s'kthehen në njëjës: njëjësi ka kuptim tjetër ose s'ekziston («glasses», «goods»).
const PLURALIA: &[&str] = &[
    "glasses", "scissors", "pants", "trousers", "jeans", "shorts", "clothes", "pajamas", "pyjamas", "goods", "news",
    "means", "species", "series", "savings", "earnings", "belongings", "surroundings", "outskirts", "headquarters",
    "premises", "groceries", "customs", "manners", "binoculars", "tongs", "pliers", "tweezers", "shears",
];

/// Shumësat e parregullt të emrave anglisht → njëjësi.
const IRREGULAR_PLURAL: &[(&str, &str)] = &[
    ("men", "man"), ("women", "woman"), ("children", "child"), ("people", "person"), ("feet", "foot"),
    ("teeth", "tooth"), ("mice", "mouse"), ("geese", "goose"), ("things", "thing"),
];

/// Format e parregullta të foljeve angleze → lema.
const IRREGULAR_EN: &[(&str, &str)] = &[
    ("ate", "eat"), ("eaten", "eat"), ("drank", "drink"), ("drunk", "drink"), ("went", "go"), ("gone", "go"),
    ("seen", "see"),
    ("made", "make"), ("built", "build"), ("bought", "buy"), ("brought", "bring"), ("caught", "catch"),
    ("taught", "teach"), ("thought", "think"), ("fought", "fight"), ("found", "find"), ("held", "hold"),
    ("kept", "keep"), ("left", "leave"), ("lost", "lose"), ("paid", "pay"), ("said", "say"),
    ("sold", "sell"), ("told", "tell"), ("sent", "send"), ("spent", "spend"), ("felt", "feel"),
    ("slept", "sleep"), ("swept", "sweep"), ("fed", "feed"), ("led", "lead"), ("met", "meet"),
    ("written", "write"), ("wrote", "write"), ("driven", "drive"), ("drove", "drive"), ("ridden", "ride"),
    ("rode", "ride"), ("given", "give"), ("gave", "give"), ("taken", "take"), ("took", "take"),
    ("broken", "break"), ("broke", "break"), ("stolen", "steal"), ("stole", "steal"), ("worn", "wear"),
    ("wore", "wear"), ("torn", "tear"), ("thrown", "throw"), ("threw", "throw"), ("grown", "grow"),
    ("grew", "grow"), ("known", "know"), ("knew", "know"), ("done", "do"), ("did", "do"),
    ("sung", "sing"), ("sang", "sing"), ("hung", "hang"), ("dug", "dig"), ("won", "win"),
    ("bitten", "bite"), ("hidden", "hide"), ("frozen", "freeze"), ("chosen", "choose"), ("spoken", "speak"),
    ("forgotten", "forget"), ("beaten", "beat"), ("shaken", "shake"), ("fallen", "fall"), ("begun", "begin"),
    ("began", "begin"), ("became", "become"), ("came", "come"), ("ran", "run"), ("swam", "swim"),
    ("swum", "swim"), ("flew", "fly"), ("flown", "fly"), ("blown", "blow"), ("drawn", "draw"),
    ("sat", "sit"), ("stood", "stand"), ("understood", "understand"), ("woke", "wake"), ("woken", "wake"),
    ("burnt", "burn"), ("learnt", "learn"), ("spat", "spit"),
];

/// Etiketat e Wiktionary-t që e bëjnë glosën e parë jo-standarde: ajo s'vlen si urë.
const NONSTANDARD_TAGS: &[&str] = &[
    "form-of", "alt-of", "alternative", "morpheme", "letter", "abbreviation", "initialism", "regional", "dialectal",
    "archaic", "dated", "obsolete", "Arbëresh", "Arvanitika", "rare", "nonstandard", "derogatory", "vulgar", "slang",
    "poetic", "offensive", "pejorative",
];

/// Fjalët e para që e kthejnë një pjesë glose në përshkrim, jo në përkthim.
const GLOSS_FUNCTION: &[&str] = &[
    "of", "or", "relating", "pertaining", "any", "one", "someone", "something", "used", "having", "in", "with", "that",
    "which", "alternative", "obsolete",
];

/// Dëshmitarët shqip të sinseteve të WordNet-it (jo modeli, jo Wiktionary).
fn wordnet_witness(s: Source) -> bool {
    matches!(s, Source::OmwAls | Source::BabelNet53)
}

/// Fjala angleze e termit ConceptNet: shkronja të vogla, pa nyje e pronorë. `None` kur fraza ka
/// përemër, mohim, lidhëz ose shenjë jo-fjalë.
pub fn en_tokens(term: &str) -> Option<Vec<String>> {
    let toks: Vec<String> = term
        .split('_')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && !DETERMINERS.contains(&t.as_str()))
        .collect();
    let bad = toks.iter().any(|t| {
        BLOCKERS.contains(&t.as_str())
            || !t.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            || t.starts_with('-')
            || t.ends_with('-')
    });
    let toks: Vec<String> = match (toks.len() > 1, toks.first().map(String::as_str)) {
        (true, Some("to")) => toks[1..].to_vec(),
        _ => toks,
    };
    match (bad, toks.is_empty(), toks.len() <= 4) {
        (false, false, true) => Some(toks),
        _ => None,
    }
}

/// Pjesët e glosës: «to watch over, to observe, to guard» → [watch over, observe, guard].
/// Hiqen kllapat, «to» e foljes dhe nyja; pjesët me më shumë se 3 fjalë ose që nisin me fjalë
/// funksionale nuk janë përkthime. Pjesa me kllapa («lead (metal)», «Russian (language)») shënohet
/// e kufizuar (`true`): vlen vetëm për një kuptim të fjalës angleze. Kllapa në fund të glosës e
/// kufizon gjithë glosën («light, bright (colour)»).
pub fn gloss_items(gloss: &str) -> Vec<(String, bool)> {
    gloss_split(gloss).0.into_iter().map(|i| (i.text, i.qualifier.is_some())).collect()
}

/// Një pjesë e glosës: përkthimi dhe kufizimi i tij në kllapa, kur ka.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GlossItem {
    text: String,
    qualifier: Option<String>,
}

/// Pjesët e glosës (`gloss_items`) dhe numri i të gjitha pjesëve të saj, edhe atyre që s'janë
/// përkthime. Emri shkencor në kllapa («grape (Vitis vinifera)») sqaron, s'kufizon.
fn gloss_split(gloss: &str) -> (Vec<GlossItem>, usize) {
    const MARK: char = '\u{1}';
    let mut depth = 0usize;
    let mut flat = String::new();
    let mut inner = String::new();
    let mut quals: Vec<String> = Vec::new();
    for ch in gloss.chars() {
        match (ch, depth) {
            ('(', 0) => {
                depth = 1;
                inner.clear();
            }
            ('(', _) => {
                depth += 1;
                inner.push(ch);
            }
            (')', 0) => {}
            (')', 1) => {
                depth = 0;
                match inner.trim().chars().next().map(char::is_uppercase) {
                    Some(false) => {
                        quals.push(inner.trim().to_lowercase());
                        flat.push(MARK);
                    }
                    _ => flat.push(' '),
                }
            }
            (')', _) => {
                depth -= 1;
                inner.push(ch);
            }
            (c, 0) => flat.push(c),
            (c, _) => inner.push(c),
        }
    }
    let segments = flat.split([',', ';', ':']).filter(|p| !p.replace(MARK, " ").trim().is_empty()).count();
    let trailing = match flat.trim_end().trim_end_matches('.').trim_end().ends_with(MARK) {
        true => quals.last().cloned(),
        false => None,
    };
    let mut seen = 0usize;
    let items = flat
        .split([',', ';', ':'])
        .map(|p| {
            let marks = p.matches(MARK).count();
            let own: Vec<String> = quals.iter().skip(seen).take(marks).cloned().collect();
            seen += marks;
            let qualifier = match own.is_empty() {
                true => trailing.clone(),
                false => Some(own.join(" ")),
            };
            let t = p.replace(MARK, " ");
            let t = t.trim().trim_end_matches('.').trim().to_lowercase();
            let t = ["to ", "a ", "an ", "the "].iter().find_map(|a| t.strip_prefix(a)).map(|r| r.trim().to_string()).unwrap_or(t);
            GlossItem { text: t.split_whitespace().collect::<Vec<_>>().join(" "), qualifier }
        })
        .filter(|i| {
            let words: Vec<&str> = i.text.split(' ').collect();
            !i.text.is_empty()
                && words.len() <= 3
                && !GLOSS_FUNCTION.contains(&words[0])
                && i.text.chars().all(|c| c.is_ascii_lowercase() || c == ' ' || c == '-' || c == '\'')
        })
        .collect();
    (items, segments)
}

/// Pjesa e glosës vlen për pjesën e ligjëratës së fjalës shqipe, përveç kur WordNet-i e njeh fjalën
/// angleze vetëm me pjesë të tjera («beautiful» s'është emër: «lojë» nuk bëhet «beautiful»).
fn pos_fits(wn: &HashMap<(String, Pos), Wn>, item: &str, p: Pos) -> bool {
    let known = |q: Pos| wn.contains_key(&(item.to_string(), q));
    known(p) || ![Pos::Noun, Pos::Verb, Pos::Adj, Pos::Adv].into_iter().any(known)
}

fn kaikki_pos(p: &str) -> Option<Pos> {
    match p {
        "noun" => Some(Pos::Noun),
        "verb" => Some(Pos::Verb),
        "adj" => Some(Pos::Adj),
        "adv" => Some(Pos::Adv),
        _ => None,
    }
}

fn strip_article(s: &str) -> &str {
    ["i ", "e ", "të ", "së "].iter().find_map(|a| s.strip_prefix(a)).unwrap_or(s)
}

/// Fjala shqipe në graf për një etiketë: shkronja të vogla, pa «një», mbiemri pa nyjë kur
/// fjala pa nyjë ekziston.
fn resolve(g: &Graph, raw: &str, p: Pos) -> Option<(String, u32)> {
    let clean = raw.replace('_', " ").to_lowercase();
    let clean = clean.trim();
    let clean = clean.strip_prefix("një ").unwrap_or(clean);
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let bare = match p {
        Pos::Adj => strip_article(&clean).to_string(),
        _ => clean.clone(),
    };
    match (g.get(&word_key(Lang::Sq, &bare)), g.get(&word_key(Lang::Sq, &clean))) {
        (Some(w), _) => Some((bare, w)),
        (None, Some(w)) => Some((clean, w)),
        (None, None) => None,
    }
}

/// Krahasim pa shenjat diakritike: AlbaNet ka edhe trajta pa «ë»/«ç» («shkolle» = «shkollë»).
fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ë' => 'e',
            'ç' => 'c',
            o => o,
        })
        .collect()
}

/// Një përkthim anglisht i një fjale shqipe sipas Wiktionary-t.
#[derive(Debug, Clone)]
struct WiktHit {
    lemma: String,
    word: u32,
    /// 0 = glosa e parë leksikore e hyrjes (kuptimi kryesor); 1, 2, … = kuptimet e tjera.
    sense: usize,
    /// Vendi i fjalës angleze brenda glosës.
    item: usize,
    /// Pjesa e glosës ka kufizim në kllapa: kuptim i ngushtë i fjalës angleze.
    qualified: bool,
    /// Kuptim i figurshëm ose bisedor i fjalës shqipe.
    marked: bool,
    /// Glosa ka vetëm këtë pjesë: përkthim i drejtpërdrejtë («letër»: «letter»).
    alone: bool,
}

/// Ura (A) e plotë.
#[derive(Debug, Default)]
struct Wiktionary {
    /// (fjalë angleze, pjesë ligjërate) → fjalët shqipe.
    hits: HashMap<(String, Pos), Vec<WiktHit>>,
    /// (fjalë shqipe, pjesë ligjërate) → pjesët e glosave të saj.
    items: HashMap<(u32, Pos), Vec<String>>,
    /// Fjalë shqipe → numri i kuptimeve leksikore në Wiktionary.
    senses: HashMap<u32, usize>,
}

/// Ura (A): (fjalë angleze, pjesë ligjërate) → fjalët shqipe që e kanë në një glosë leksikore
/// standarde të hyrjes. Glosa e parë leksikore është kuptimi kryesor; kur ajo është jo-standarde
/// (dialektore, e vjetruar, e rrallë …), hyrja s'ka kuptim kryesor të vlefshëm si urë. Pjesa që
/// WordNet-i e njeh vetëm me pjesë tjetër ligjërate s'vlen (`pos_fits`).
fn wiktionary_bridge(g: &Graph, kaikki: &str, wn: &HashMap<(String, Pos), Wn>) -> Wiktionary {
    let mut out = Wiktionary::default();
    let mut cur: Option<(String, Pos, u32)> = None;
    let mut sense = 0usize;
    for line in kaikki.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match (c.first().copied(), &cur) {
            (Some("E"), _) => {
                let pos = c.get(2).and_then(|p| kaikki_pos(p));
                let word = destress(c.get(1).copied().unwrap_or("").trim()).to_lowercase();
                let lemma = match pos {
                    Some(Pos::Adj) => strip_article(&word).to_string(),
                    _ => word,
                };
                cur = pos.and_then(|p| g.get(&word_key(Lang::Sq, &lemma)).map(|w| (lemma, p, w)));
                sense = 0;
            }
            (Some("G"), Some((lemma, pos, w))) => {
                let tags: Vec<&str> = c.get(2).copied().unwrap_or("").split(',').map(str::trim).collect();
                let gloss = c.get(3).copied().unwrap_or("");
                let lexical = !tags.contains(&"form-of") && !tags.contains(&"alt-of") && gloss_form_of(gloss).is_none();
                let gheg_only = tags.contains(&"Gheg") && !tags.iter().any(|t| matches!(*t, "Tosk" | "standard" | "Standard"));
                let standard = !tags.iter().any(|t| NONSTANDARD_TAGS.contains(t)) && !gheg_only;
                let marked = tags.iter().any(|t| matches!(*t, "figuratively" | "colloquial"));
                match (lexical, standard) {
                    (true, true) => {
                        let (items, segments) = gloss_split(gloss);
                        let alone = segments == 1;
                        for (i, GlossItem { text: item, qualifier }) in items.into_iter().enumerate() {
                            let qualified = qualifier.is_some();
                            match pos_fits(wn, &item, *pos) {
                                true => {}
                                false => continue,
                            }
                            out.items.entry((*w, *pos)).or_default().push(item.clone());
                            let v = out.hits.entry((item, *pos)).or_default();
                            match v.iter().any(|h| h.lemma == *lemma) {
                                true => {}
                                false => {
                                    v.push(WiktHit {
                                        lemma: lemma.clone(),
                                        word: *w,
                                        sense,
                                        item: i,
                                        qualified,
                                        marked,
                                        alone,
                                    })
                                }
                            }
                        }
                        sense += 1;
                        *out.senses.entry(*w).or_default() += 1;
                    }
                    (true, false) => {
                        sense += 1;
                        *out.senses.entry(*w).or_default() += 1;
                    }
                    (false, _) => {}
                }
            }
            _ => {}
        }
    }
    out
}

/// Dëshmitë shqipe të pavarura për një fjalë.
#[derive(Debug, Clone, Copy, Default)]
struct Info {
    /// E dëshmuar në një burim shqip të pavarur (gramatikë, UD, Wiktionary, LEXIN, Wikidata …).
    attested: bool,
    /// E dëshmuar edhe jashtë Wiktionary-t anglisht dhe Lexemave të Wikidata-s (që i marrin
    /// shpesh nga Wiktionary), që ura A të mos jetë dëshmitari i vetëm.
    beyond: bool,
    /// Në leksikun gramatikor (uniparser): shenjë e trajtës standarde.
    standard: bool,
    /// Pjesët e ligjëratës sipas leksikut gramatikor vetëm.
    std_pos: u8,
    /// Pjesët e ligjëratës sipas gramatikës, UD-së dhe Wiktionary-t (bit për çdo pjesë).
    pos: u8,
    /// Sa fjalorë dhe korpuse të pavarura e dëshmojnë fjalën (shenjë e përdorimit të zakonshëm).
    breadth: u8,
}

fn pos_bit(p: Pos) -> u8 {
    match p {
        Pos::Noun => 1,
        Pos::Verb => 2,
        Pos::Adj => 4,
        Pos::Adv => 8,
    }
}

fn independent_sq(s: Source) -> bool {
    matches!(
        s,
        Source::UdSqTsa
            | Source::UdSqStaf
            | Source::Uniparser
            | Source::KaikkiEn
            | Source::KaikkiDe
            | Source::Lexin
            | Source::WdLexemes
            | Source::Loanwords
            | Source::Gazulli1941
    )
}

/// Burimet që numërohen për përhapjen e një fjale: fjalorët dhe korpuset (jo urat e WordNet-it, që
/// do ta favorizonin fjalën e urës B, jo modeli, jo rregullat e nxjerra, jo ConceptNet-i).
fn breadth_bit(s: Source) -> u16 {
    match s {
        Source::UdSqTsa | Source::UdSqStaf => 1,
        Source::Uniparser => 2,
        Source::KaikkiEn => 4,
        Source::KaikkiDe => 8,
        Source::Lexin => 16,
        Source::WdLexemes => 32,
        Source::Loanwords => 64,
        Source::Gazulli1941 => 128,
        Source::EuroVoc => 256,
        _ => 0,
    }
}

fn word_info(g: &Graph) -> Vec<Info> {
    let mut v = vec![Info::default(); g.nodes.len()];
    let mut seen = vec![0u16; g.nodes.len()];
    for e in &g.edges {
        let bit = breadth_bit(e.source);
        seen[e.from as usize] |= bit;
        seen[e.to as usize] |= bit;
        match (independent_sq(e.source), matches!(e.source, Source::KaikkiEn | Source::WdLexemes)) {
            (true, beyond) => {
                for n in [e.from as usize, e.to as usize] {
                    v[n].attested = true;
                    v[n].beyond |= !beyond;
                }
            }
            (false, _) => {}
        }
        let pos = match (&e.rel, e.source) {
            (Rel::Lex, Source::ModelClaude) => None,
            (Rel::Lex, s) => {
                let p = e.attr("pos").and_then(Pos::from_wn);
                match (s == Source::Uniparser, p) {
                    (true, Some(p)) => {
                        v[e.from as usize].standard = true;
                        v[e.from as usize].std_pos |= pos_bit(p);
                    }
                    (true, None) => v[e.from as usize].standard = true,
                    (false, _) => {}
                }
                p
            }
            (Rel::HasSense, Source::KaikkiEn) => match &g.node(e.to).kind {
                NodeKind::Sense { pos, .. } => Some(*pos),
                _ => None,
            },
            (Rel::OccursIn, _) => e.attr("upos").and_then(Pos::from_upos),
            _ => None,
        };
        match pos {
            Some(p) => v[e.from as usize].pos |= pos_bit(p),
            None => {}
        }
    }
    for (i, s) in seen.into_iter().enumerate() {
        v[i].breadth = s.count_ones() as u8;
    }
    v
}

/// Sinsetet e WordNet-it të një leme angleze (sipas shpeshtësisë) dhe shënimet e tyre në korpus.
#[derive(Debug, Clone, Default)]
struct Wn {
    synsets: Vec<String>,
    /// Sa herë është shënuar çdo kuptim në korpus (`index.sense`), sipas renditjes.
    counts: Vec<u32>,
    /// Skedari leksikografik i çdo kuptimi (`noun.act` = 4, `noun.food` = 13 …); 0 kur s'dihet.
    lex: Vec<u8>,
}

/// `index.sense`: (lema, pjesa e ligjëratës, offset) → (numri i shënimeve në korpus, skedari
/// leksikografik).
fn parse_sense_counts(text: &str) -> HashMap<(String, Pos, String), (u32, u8)> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            let (lemma, rest) = f.first()?.split_once('%')?;
            let pos = match rest.chars().next()? {
                '1' => Pos::Noun,
                '2' => Pos::Verb,
                '3' | '5' => Pos::Adj,
                '4' => Pos::Adv,
                _ => return None,
            };
            let count: u32 = f.get(3)?.parse().ok()?;
            let lex: u8 = rest.split(':').nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
            Some(((lemma.replace('_', " "), pos, f.get(1)?.to_string()), (count, lex)))
        })
        .collect()
}

fn parse_pwn_index(text: &str, pos: Pos, counts: &HashMap<(String, Pos, String), (u32, u8)>, out: &mut HashMap<(String, Pos), Wn>) {
    for l in text.lines().filter(|l| !l.starts_with(' ') && !l.trim().is_empty()) {
        let f: Vec<&str> = l.split_whitespace().collect();
        let n: usize = f.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let p_cnt: usize = f.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
        match (f.len() >= 6 + p_cnt + n, n > 0) {
            (true, true) => {
                let lemma = f[0].replace('_', " ");
                let offsets = &f[f.len() - n..];
                let synsets = offsets.iter().map(|o| format!("{o}-{}", pos.code())).collect();
                let found: Vec<(u32, u8)> =
                    offsets.iter().map(|o| counts.get(&(lemma.clone(), pos, o.to_string())).copied().unwrap_or((0, 0))).collect();
                let c = found.iter().map(|x| x.0).collect();
                let lex = found.iter().map(|x| x.1).collect();
                out.insert((lemma, pos), Wn { synsets, counts: c, lex });
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
struct Tr {
    lemma: String,
    word: u32,
    code: MapCode,
}

#[derive(Debug, Clone)]
struct BCand {
    lemma: String,
    word: u32,
    ranks: BTreeSet<usize>,
}

/// Fjalë shqipe që pajtohet në të dyja urat.
#[derive(Debug, Clone)]
struct Agree {
    /// 0 = kuptimi kryesor në Wiktionary, 1 = kuptim tjetër.
    class: usize,
    item: usize,
    first: usize,
    lemma: String,
    word: u32,
    ranks: BTreeSet<usize>,
    /// Pjesa e glosës së Wiktionary-t ka kufizim në kllapa.
    qualified: bool,
}

/// Urat e përkthimit dhe hartuesi i frazave.
pub struct Mapper<'g> {
    g: &'g Graph,
    wn: HashMap<(String, Pos), Wn>,
    wikt: HashMap<(String, Pos), Vec<WiktHit>>,
    /// Pjesët e glosave të Wiktionary-t për çdo fjalë shqipe, sipas pjesës së ligjëratës.
    items: HashMap<(u32, Pos), Vec<String>>,
    /// Numri i kuptimeve leksikore të çdo fjale shqipe në Wiktionary.
    wsenses: HashMap<u32, usize>,
    /// Fjalët shqipe që kanë të paktën një përkthim anglisht në Wiktionary, sipas pjesës së ligjëratës.
    glossed: HashSet<(u32, Pos)>,
    info: Vec<Info>,
    /// Çiftet e fjalëve shqipe të lidhura si sinonime, variante ose trajta nga një fjalor.
    syn: HashSet<(u32, u32)>,
    /// Fjalët e leksikut gramatikor sipas trajtës pa diakritikë.
    std_fold: HashMap<String, Vec<u32>>,
    /// Përkthimet e leksikut të modelit (`model_witness`): dëshmitar i dytë i një përkthimi të vetëm.
    witness: HashSet<(String, Pos, String)>,
    cache: HashMap<(String, Pos), Result<Tr, Hold>>,
}

impl<'g> Mapper<'g> {
    /// `pwn_index` = indekset e WordNet-it sipas pjesës së ligjëratës; `sense_index` = `index.sense`.
    pub fn new(g: &'g Graph, kaikki: &str, pwn_index: &[(Pos, &str)], sense_index: &str) -> Self {
        let counts = parse_sense_counts(sense_index);
        let mut wn = HashMap::new();
        for (p, t) in pwn_index {
            parse_pwn_index(t, *p, &counts, &mut wn);
        }
        let mut syn: HashSet<(u32, u32)> = HashSet::new();
        for e in g.edges.iter().filter(|e| e.source != Source::ModelClaude) {
            let close = match &e.rel {
                Rel::Sem(r) => matches!(r.as_str(), "SYNONYM" | "DIALECT_VARIANT_OF" | "NATIVE_EQUIVALENT" | "FORM_OF"),
                _ => false,
            };
            match close && e.from != e.to {
                true => {
                    syn.insert((e.from, e.to));
                    syn.insert((e.to, e.from));
                }
                false => {}
            }
        }
        let info = word_info(g);
        let mut std_fold: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, n) in g.nodes.iter().enumerate() {
            match (&n.kind, info[i].standard) {
                (NodeKind::Word { lang: Lang::Sq, lemma }, true) => std_fold.entry(fold(lemma)).or_default().push(i as u32),
                _ => {}
            }
        }
        let w = wiktionary_bridge(g, kaikki, &wn);
        let glossed = w.hits.iter().flat_map(|((_, p), v)| v.iter().map(move |h| (h.word, *p))).collect();
        Mapper {
            g,
            wn,
            wikt: w.hits,
            items: w.items,
            wsenses: w.senses,
            glossed,
            info,
            syn,
            std_fold,
            witness: HashSet::new(),
            cache: HashMap::new(),
        }
    }

    /// Leksiku i modelit si dëshmitar i dytë i përkthimit (`model_witness`).
    pub fn with_witness(mut self, witness: HashSet<(String, Pos, String)>) -> Self {
        self.witness = witness;
        self.cache.clear();
        self
    }

    /// Leksiku i modelit e jep fjalën shqipe për fjalën angleze me këtë pjesë ligjërate.
    fn witnessed(&self, lemma: &str, p: Pos, e: &str) -> bool {
        self.witness.contains(&(lemma.to_string(), p, e.to_string()))
    }

    /// Sa herë është shënuar në korpus kuptimi më i shpeshtë ndër renditjet e dhëna.
    fn freq(&self, e: &str, p: Pos, ranks: &BTreeSet<usize>) -> u32 {
        self.wn
            .get(&(e.to_string(), p))
            .map(|w| ranks.iter().filter_map(|r| w.counts.get(*r).copied()).max().unwrap_or(0))
            .unwrap_or(0)
    }

    /// Një nga kuptimet është shënuar në korpus, ose fjala s'ka asnjë shënim (s'dihet).
    fn counted(&self, e: &str, p: Pos, ranks: &BTreeSet<usize>) -> bool {
        self.freq(e, p, ranks) > 0 || self.usage(e, p) == 0
    }

    fn known(&self, e: &str, p: Pos) -> bool {
        self.wn.contains_key(&(e.to_string(), p)) || self.wikt.contains_key(&(e.to_string(), p))
    }

    /// Sa herë është shënuar fjala angleze në korpus me këtë pjesë ligjërate (të gjitha kuptimet).
    fn usage(&self, e: &str, p: Pos) -> u32 {
        self.wn.get(&(e.to_string(), p)).map(|w| w.counts.iter().sum()).unwrap_or(0)
    }

    /// Lemat e mundshme angleze të një fjale: vetë fjala kur njihet; ndryshe format pa mbaresë
    /// (vetëm kur `base_only` s'kërkon formën bazë), të verifikuara te WordNet/Wiktionary. Emri në
    /// shumës merr njëjësin e parë që njihet («shoes» → «shoe», jo kuptimi i veçantë i shumësit;
    /// «fires» → «fire», jo «fir»), përveç emrave pa njëjës të njëjtë («glasses», «-ics»).
    fn forms(&self, tok: &str, p: Pos, base_only: bool) -> Vec<String> {
        let exact = self.known(tok, p);
        let mut c: Vec<String> = Vec::new();
        let lookup = |table: &[(&str, &str)]| table.iter().find(|(f, _)| *f == tok).map(|(_, l)| l.to_string());
        let irregular = match p {
            Pos::Noun => lookup(IRREGULAR_PLURAL),
            _ => lookup(IRREGULAR_EN),
        };
        let undouble = |s: &str| -> Option<String> {
            let b = s.as_bytes();
            match b.len() >= 3 && b[b.len() - 1] == b[b.len() - 2] && !b"aeiou".contains(&b[b.len() - 1]) {
                true => Some(s[..s.len() - 1].to_string()),
                false => None,
            }
        };
        match p {
            Pos::Noun => {
                let sibilant = |s: &&str| ["s", "x", "z", "ch", "sh", "o"].iter().any(|e| s.ends_with(e));
                c.extend(irregular);
                c.extend(tok.strip_suffix('s').filter(|s| !s.ends_with('s')).map(str::to_string));
                c.extend(tok.strip_suffix("ies").map(|s| format!("{s}y")));
                c.extend(tok.strip_suffix("ves").map(|s| format!("{s}f")));
                c.extend(tok.strip_suffix("ves").map(|s| format!("{s}fe")));
                c.extend(tok.strip_suffix("es").filter(sibilant).map(str::to_string));
            }
            Pos::Verb => {
                c.extend(irregular);
                c.extend(tok.strip_suffix("ying").map(|s| format!("{s}ie")));
                c.extend(tok.strip_suffix("ing").map(str::to_string));
                c.extend(tok.strip_suffix("ing").map(|s| format!("{s}e")));
                c.extend(tok.strip_suffix("ing").and_then(undouble));
                c.extend(tok.strip_suffix("ied").map(|s| format!("{s}y")));
                c.extend(tok.strip_suffix('d').filter(|s| s.ends_with('e')).map(str::to_string));
                c.extend(tok.strip_suffix("ed").map(str::to_string));
                c.extend(tok.strip_suffix("ed").and_then(undouble));
                c.extend(tok.strip_suffix("ies").map(|s| format!("{s}y")));
                c.extend(tok.strip_suffix("es").map(str::to_string));
                c.extend(tok.strip_suffix('s').filter(|s| !s.ends_with('s')).map(str::to_string));
            }
            Pos::Adj | Pos::Adv => {}
        }
        let plural = match p {
            Pos::Noun => lookup(IRREGULAR_PLURAL).filter(|l| self.known(l, p)),
            _ => None,
        };
        let mut stems: Vec<String> = Vec::new();
        for f in c.into_iter().filter(|f| f.len() >= 2 && f != tok && self.known(f, p)) {
            match stems.contains(&f) {
                true => {}
                false => stems.push(f),
            }
        }
        let plural_only = PLURALIA.contains(&tok) || tok.ends_with("ics");
        let singular = p == Pos::Noun && !base_only && !stems.is_empty() && !plural_only;
        match (plural, singular, exact, base_only) {
            (Some(l), _, _, _) => vec![l],
            (None, true, _, _) => stems.into_iter().take(1).collect(),
            (None, false, true, _) => vec![tok.to_string()],
            (None, false, false, true) => vec![],
            (None, false, false, false) => stems,
        }
    }

    /// Kandidati i WordNet-it vlen kur fjala dëshmohet në një burim shqip të pavarur dhe pjesa e
    /// ligjëratës (kur dihet) përputhet; folja dhe emri me nyjë në krye duhet ta kenë të dëshmuar.
    fn fits(&self, lemma: &str, w: u32, p: Pos) -> bool {
        let i = self.info[w as usize];
        let article = matches!(lemma.split(' ').next(), Some("i" | "e" | "të" | "së"));
        let pos_ok = i.pos == 0 || i.pos & pos_bit(p) != 0;
        let article_ok = !article || p == Pos::Adj || i.pos & pos_bit(p) != 0;
        let head_verb = lemma
            .split(' ')
            .next()
            .and_then(|h| self.g.get(&word_key(Lang::Sq, h)))
            .map(|h| self.info[h as usize].pos & pos_bit(Pos::Verb) != 0)
            .unwrap_or(false);
        let phrase_ok = !(p == Pos::Verb && lemma.contains(' ')) || i.pos & pos_bit(p) != 0 || head_verb;
        i.attested && pos_ok && article_ok && phrase_ok
    }

    /// Ura (B): fjalët shqipe të sinseteve të fjalës angleze, me renditjen e kuptimeve. Trajtat
    /// pa diakritikë bashkohen me fjalën e shkruar drejt; mbetet vetëm fjala që e kalon `fits`.
    fn wordnet(&self, e: &str, p: Pos) -> Vec<BCand> {
        let mut groups: Vec<(String, Vec<(String, u32)>, BTreeSet<usize>)> = Vec::new();
        let syns = self.wn.get(&(e.to_string(), p)).map(|w| w.synsets.clone()).unwrap_or_default();
        for (rank, syn) in syns.iter().enumerate() {
            let sid = match self.g.get(&sense_key(Source::Pwn30, syn)) {
                Some(s) => s,
                None => continue,
            };
            let mut names: Vec<String> = self
                .g
                .in_edges(sid)
                .filter(|(_, x)| x.rel == Rel::HasSense && wordnet_witness(x.source))
                .filter_map(|(_, x)| match &self.g.node(x.from).kind {
                    NodeKind::Word { lang: Lang::Sq, lemma } => Some(lemma.clone()),
                    _ => None,
                })
                .collect();
            names.extend(
                self.g.node(sid).labels.iter().filter(|t| t.lang == Lang::Sq && wordnet_witness(t.source)).map(|t| t.text.clone()),
            );
            for n in names {
                match resolve(self.g, &n, p).map(|(l, w)| self.base_form(l, w)) {
                    Some((lemma, word)) => {
                        let f = fold(&lemma);
                        match groups.iter_mut().find(|x| x.0 == f) {
                            Some(x) => {
                                x.2.insert(rank);
                                match x.1.iter().any(|(l, _)| *l == lemma) {
                                    true => {}
                                    false => x.1.push((lemma, word)),
                                }
                            }
                            None => groups.push((f, vec![(lemma, word)], [rank].into_iter().collect())),
                        }
                    }
                    None => {}
                }
            }
        }
        groups
            .into_iter()
            .filter_map(|(_, members, ranks)| {
                let mut ok: Vec<(bool, String, u32)> = members
                    .into_iter()
                    .filter(|(l, w)| self.fits(l, *w, p))
                    .map(|(l, w)| (!self.info[w as usize].standard, l, w))
                    .collect();
                ok.sort();
                ok.into_iter().next().map(|(_, lemma, word)| BCand { lemma, word, ranks })
            })
            .collect()
    }

    /// Etiketa që është trajtë e lakuar e një fjale tjetër («drita» → «dritë», «shkronje» →
    /// «shkronjë») kthehet në lemën e saj, kur s'është vetë në leksikun gramatikor dhe të gjitha
    /// lidhjet FORM_OF të fjalorit çojnë te e njëjta lemë.
    fn base_form(&self, lemma: String, w: u32) -> (String, u32) {
        let targets: BTreeSet<u32> = self
            .g
            .out_edges(w)
            .filter(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()) && e.source != Source::ModelClaude && e.to != w)
            .map(|(_, e)| e.to)
            .collect();
        let lemma_of = |t: u32| match &self.g.node(t).kind {
            NodeKind::Word { lemma, .. } => Some(lemma.clone()),
            _ => None,
        };
        match (self.info[w as usize].standard, targets.len(), targets.iter().next().and_then(|t| lemma_of(*t))) {
            (false, 1, Some(l)) => (l, *targets.iter().next().unwrap_or(&w)),
            _ => (lemma, w),
        }
    }

    /// Dy fjalë shqipe janë sinonime të dëshmuara nga një fjalor (jo nga modeli).
    fn synonyms(&self, a: u32, b: u32) -> bool {
        a == b || self.syn.contains(&(a, b))
    }

    /// Fjala e pajtuar përballë kandidatëve të tjerë. Wiktionary është vendimtar kur fjala angleze
    /// është në krye të glosës kryesore të fjalës dhe asnjë fjalë tjetër shqipe (jo sinonim) s'e ka
    /// në glosën e saj kryesore. HOLD kur:
    /// - një tjetër i pajtuar s'ndan asnjë kuptim me të dhe kuptimi i tij është i shpeshtë (të paktën
    ///   një e treta e shënimeve të kuptimit të më të mirit; një e pesta kur Wiktionary s'është
    ///   vendimtar): dy kuptime të fjalës angleze («party»: «parti» dhe «festë»);
    /// - një kuptim më i shpeshtë i fjalës angleze ka një fjalë tjetër shqipe që e njeh edhe
    ///   Wiktionary-t, ose çfarëdo fjale tjetër kur vetë fjala angleze s'është në krye të glosës;
    /// - pajtimi s'është te kuptimi më i shpeshtë i WordNet-it, fjala angleze s'është në krye të
    ///   glosës së saj, dhe Wiktionary-t ka një fjalë tjetër që e ka atë në krye;
    /// - Wiktionary s'është vendimtar dhe: kuptimi më i shpeshtë i WordNet-it ka një fjalë tjetër
    ///   shqipe, ndërsa fjala angleze s'është në krye të glosës kryesore ose ai kuptim është më shumë
    ///   se dyfish më i shpeshtë se i pajtuari; ose kuptimi i pajtuar s'është shënuar asnjëherë në
    ///   korpus ndërsa kuptimi kryesor po;
    /// - edhe kur Wiktionary është vendimtar: pajtimi s'është te kuptimi i parë i WordNet-it, ai
    ///   kuptim është shënuar më shumë se dyfish, dhe Wiktionary ka një fjalë tjetër të leksikut
    ///   gramatikor që e ka fjalën angleze si glosë të vetme të kuptimit të dytë ose të tretë, jo të
    ///   figurshëm a bisedor, jo të kufizuar, dhe që s'është në të njëjtin sinset me të pajtuarën
    ///   («letter»: «shkronjë» pajtohet te shkronja e alfabetit, por «letër» është letra që shkruhet);
    ///   ose kur një fjalë tjetër (jo
    ///   sinonim) pajtohet po ashtu me glosën kryesore («cut»: «shkurtoj» dhe «pres» te kuptimi i
    ///   pakësimit).
    /// Pjesa e glosës me kufizim në kllapa s'është vendimtare. Sinonimet e dëshmuara nuk e bëjnë të
    /// paqartë.
    fn judge(&self, e: &str, p: Pos, best: &Agree, peers: &[Agree], b: &[BCand], a: &[WiktHit]) -> Result<Tr, Hold> {
        let apart = |ranks: &BTreeSet<usize>, w: u32| ranks.is_disjoint(&best.ranks) && !self.synonyms(w, best.word);
        let typed = |w: u32| self.info[w as usize].std_pos & pos_bit(p) != 0;
        let decisive = best.class == 0
            && best.item == 0
            && !best.qualified
            && !a.iter().any(|h| h.sense == 0 && fold(&h.lemma) != fold(&best.lemma) && !self.synonyms(h.word, best.word));
        let seen = self.freq(e, p, &best.ranks);
        let top = seen + 1;
        let most = self.wn.get(&(e.to_string(), p)).and_then(|w| w.counts.iter().copied().max()).unwrap_or(0);
        let rare = seen == 0 && most >= 3;
        let first = self.wn.get(&(e.to_string(), p)).and_then(|w| w.counts.first().copied()).unwrap_or(0);
        let displaced = !best.ranks.contains(&0)
            && (best.class > 0 || best.item > 0 || first + 1 > 2 * top)
            && b.iter().any(|c| c.ranks.contains(&0) && c.word != best.word && !self.synonyms(c.word, best.word));
        let share = match decisive {
            true => 3,
            false => 5,
        };
        let split = peers
            .iter()
            .any(|c| c.word != best.word && apart(&c.ranks, c.word) && (self.freq(e, p, &c.ranks) + 1) * share >= top);
        let weak_a = best.class > 0 || best.item > 0;
        let earlier = b.iter().any(|c| {
            let first = c.ranks.iter().next().copied().unwrap_or(usize::MAX);
            let witnessed = a.iter().any(|h| fold(&h.lemma) == fold(&c.lemma));
            c.word != best.word && first < best.first && (witnessed || weak_a) && apart(&c.ranks, c.word)
        });
        let rival = best.first > 0
            && best.item > 0
            && a.iter().any(|h| h.sense == 0 && h.item == 0 && h.word != best.word && !self.synonyms(h.word, best.word));
        let minor = !best.ranks.contains(&0) && first > 2 * seen;
        let corank = |h: &WiktHit| b.iter().any(|c| fold(&c.lemma) == fold(&h.lemma) && !c.ranks.is_disjoint(&best.ranks));
        let named = a.iter().any(|h| {
            (1..=2).contains(&h.sense)
                && h.alone
                && !h.qualified
                && !h.marked
                && typed(h.word)
                && fold(&h.lemma) != fold(&best.lemma)
                && !self.synonyms(h.word, best.word)
                && !corank(h)
        });
        let twin = peers
            .iter()
            .any(|c| c.word != best.word && c.class == 0 && c.item == 0 && !self.synonyms(c.word, best.word));
        let code = match best.class {
            0 => MapCode::AB,
            _ => MapCode::AB2,
        };
        match (split, earlier, rival, !decisive && (rare || displaced), minor && (named || twin)) {
            (false, false, false, false, false) => Ok(Tr { lemma: best.lemma.clone(), word: best.word, code }),
            _ => Err(Hold::Ambiguous),
        }
    }

    /// Rregulli i pajtimit të urave për një lemë angleze me pjesë ligjërate të dhënë:
    /// 1. kuptimi kryesor i Wiktionary-t ∩ WordNet → `AB`;
    /// 2. një kuptim tjetër i Wiktionary-t ∩ kuptimi më i shpeshtë i WordNet-it → `AB2`
    ///    (pajtim ka edhe kur fjala e kuptimit kryesor të Wiktionary-t dhe ajo e WordNet-it janë
    ///    sinonime të dëshmuara nga një fjalor; atëherë merret fjala e WordNet-it);
    /// 3. pa pajtim: vetëm Wiktionary (një fjalë, ose e vetmja standarde, me fjalën angleze në krye
    ///    të glosës), kur kuptimi më i shpeshtë i WordNet-it s'ka fjalë shqipe → `A1`; vetëm WordNet
    ///    (një fjalë në kuptimin më të shpeshtë), kur Wiktionary s'ka asgjë → `B1`;
    /// 4. ndryshe HOLD: urat kundërshtohen, ose ka më shumë se një përkthim.
    fn decide(&self, e: &str, p: Pos) -> Result<Tr, Hold> {
        let a: Vec<WiktHit> = self.wikt.get(&(e.to_string(), p)).cloned().unwrap_or_default();
        let b = self.wordnet(e, p);
        // Pajtimi merr drejtshkrimin e Wiktionary-t («shkronjë», jo «shkronje» e AlbaNet-it).
        let exact = self.agreements(e, p, &a, &b, false);
        let mut agree = match exact.is_empty() {
            true => self.agreements(e, p, &a, &b, true),
            false => exact,
        };
        // Ndër të barabartët: më shumë kuptime të shënuara në korpus («jetoj» para «banoj» për «live»),
        // pastaj fjala e leksikut gramatikor, ajo që e dëshmojnë më shumë fjalorë e korpuse («vdekje»
        // para «mort», «nënë» para «ëmë») dhe ajo me më pak kuptime të tjera në Wiktionary.
        let support = |x: &Agree| {
            let w = self.wn.get(&(e.to_string(), p));
            let sum: u32 = x.ranks.iter().filter_map(|r| w.and_then(|w| w.counts.get(*r))).sum();
            std::cmp::Reverse((sum, x.ranks.len()))
        };
        let rank = |x: &Agree| {
            let i = self.info[x.word as usize];
            (
                x.first,
                x.class,
                x.item,
                support(x),
                !i.standard,
                std::cmp::Reverse(i.breadth),
                self.wsenses.get(&x.word).copied().unwrap_or(0),
            )
        };
        agree.sort_by(|x, y| rank(x).cmp(&rank(y)).then_with(|| x.lemma.cmp(&y.lemma)));
        // Dy fjalë të barabarta në çdo dëshmi: rendi alfabetik s'vendos. Dy fjalë jo-sinonime të
        // pajtuara vetëm nëpërmjet sinonimeve, në të njëjtin kuptim: shqipja e ndan konceptin
        // («uncle»: «xhaxha» dhe «dajë»).
        let tie = match (agree.first(), agree.get(1)) {
            (Some(x), Some(y)) => {
                rank(x) == rank(y)
                    || (x.class == 1
                        && (x.first, x.class, x.item) == (y.first, y.class, y.item)
                        && !x.ranks.is_disjoint(&y.ranks)
                        && !self.synonyms(x.word, y.word))
            }
            _ => false,
        };
        match (agree.first(), tie) {
            (Some(_), true) => Err(Hold::Ambiguous),
            (Some(best), false) => self.judge(e, p, best, &agree, &b, &a),
            (None, _) => self.single(e, p, &a, &b),
        }
    }

    /// Fjalët e pajtuara: e njëjta fjalë në të dyja urat (pa diakritikë), ose, kur `bridged`, një
    /// fjalë e kuptimit kryesor të Wiktionary-t që është sinonim i dëshmuar i një fjale të kuptimit
    /// më të shpeshtë të WordNet-it, me kodin `AB2`; glosa kryesore s'ka kufizim në kllapa. Nga dy
    /// sinonimet merret:
    /// - fjala e WordNet-it kur Wiktionary e përkthen me një glosë që e ka fjalën angleze («pemë»:
    ///   «fruit tree»);
    /// - kur Wiktionary s'e përkthen fjalën e WordNet-it, ajo që e dëshmojnë më shumë burime («ide»
    ///   para «shllim»), me fjalën e Wiktionary-t kur janë baras dhe është në leksikun gramatikor;
    /// - kur Wiktionary e përkthen fjalën e WordNet-it me tjetër kuptim («detyroj»: «compel»), fjala
    ///   e Wiktionary-t, vetëm kur është në leksikun gramatikor dhe e dëshmuar së paku sa tjetra
    ///   («shtyj» për «push»); ndryshe s'ka pajtim.
    fn agreements(&self, e: &str, p: Pos, a: &[WiktHit], b: &[BCand], bridged: bool) -> Vec<Agree> {
        let mut agree: Vec<Agree> = Vec::new();
        let typed = |w: u32| self.info[w as usize].std_pos & pos_bit(p) != 0;
        for c in b {
            let first = c.ranks.iter().next().copied().unwrap_or(usize::MAX);
            let hit = a
                .iter()
                .filter(|h| match bridged {
                    false => fold(&h.lemma) == fold(&c.lemma),
                    true => h.sense == 0 && !h.qualified && self.synonyms(h.word, c.word),
                })
                .min_by_key(|h| (h.sense, h.item));
            let class = |h: &WiktHit| match bridged {
                false => h.sense.min(1),
                true => 1,
            };
            let chosen = |h: &WiktHit| {
                let common = self.info[h.word as usize].breadth >= self.info[c.word as usize].breadth;
                let glossed = self.glossed.contains(&(c.word, p));
                match (bridged, self.carries(c.word, p, e), glossed, typed(h.word) && common) {
                    (false, ..) => Some((h.lemma.clone(), h.word)),
                    (true, true, _, _) | (true, false, false, false) => Some(self.spelled(&c.lemma, c.word, p)),
                    (true, false, _, true) => Some(self.spelled(&h.lemma, h.word, p)),
                    (true, false, true, false) => None,
                }
            };
            match hit.and_then(|h| chosen(h).map(|x| (h, class(h), x))) {
                Some((h, class, (lemma, word))) => {
                    match (class == 0 || first == 0, agree.iter_mut().find(|x| x.word == word)) {
                        (false, _) => {}
                        (true, Some(x)) => {
                            x.ranks.extend(c.ranks.iter().copied());
                            x.first = x.first.min(first);
                        }
                        (true, None) => agree.push(Agree {
                            class,
                            item: h.item,
                            first,
                            lemma,
                            word,
                            ranks: c.ranks.clone(),
                            qualified: h.qualified,
                        }),
                    }
                }
                None => {}
            }
        }
        agree
    }

    /// Një nga glosat e Wiktionary-t të fjalës shqipe e ka fjalën angleze si fjalë më vete.
    fn carries(&self, w: u32, p: Pos, e: &str) -> bool {
        let british = [("yze", "yse"), ("ize", "ise"), ("or", "our")]
            .iter()
            .find_map(|(us, uk)| e.strip_suffix(us).map(|stem| format!("{stem}{uk}")));
        let needles: Vec<String> = [Some(e.to_string()), british].into_iter().flatten().map(|x| format!(" {x} ")).collect();
        self.items
            .get(&(w, p))
            .map(|v| v.iter().any(|it| needles.iter().any(|n| format!(" {it} ").contains(n.as_str()))))
            .unwrap_or(false)
    }

    /// Trajta standarde e fjalës: «rruge» → «rrugë» kur vetëm njëra fjalë e leksikut gramatikor ka
    /// të njëjtën trajtë pa diakritikë, me të njëjtën pjesë ligjërate («enjt» s'bëhet folja «ënjt»).
    fn spelled(&self, lemma: &str, w: u32, p: Pos) -> (String, u32) {
        let alt = self
            .std_fold
            .get(&fold(lemma))
            .filter(|v| v.len() == 1 && self.info[v[0] as usize].std_pos & pos_bit(p) != 0)
            .map(|v| v[0]);
        match (self.info[w as usize].standard, alt) {
            (false, Some(s)) => match &self.g.node(s).kind {
                NodeKind::Word { lemma, .. } => (lemma.clone(), s),
                _ => (lemma.to_string(), w),
            },
            _ => (lemma.to_string(), w),
        }
    }

    /// Pa pajtim: një urë e vetme, vetëm kur s'ka asnjë kundërshti.
    /// - `A1`: kuptimi më i shpeshtë i WordNet-it s'ka fjalë shqipe; Wiktionary ka një fjalë të
    ///   vetme (trajtat pa diakritikë janë e njëjta fjalë) me fjalën angleze në glosën kryesore, në
    ///   krye të saj; kur leksiku gramatikor e njeh fjalën, e njeh me këtë pjesë ligjërate; dhe kur
    ///   ajo s'është në leksikun gramatikor, asnjë fjalë e leksikut s'e ka fjalën angleze në një
    ///   nga tri kuptimet e para.
    ///   Kur glosa kryesore e disa fjalëve e ka fjalën angleze, vlen vetëm ajo e vetmja e leksikut
    ///   gramatikor me këtë pjesë ligjërate që e ka në krye, dhe vetëm kur asnjë nga të tjerat s'është
    ///   shprehje («marr frymë» për «breathe»). Fjala dëshmohet edhe jashtë Wiktionary-t. Folja me
    ///   kufizim në kllapa në glosë («to ride (horses)») s'vlen për fjalën angleze në përgjithësi
    ///   («ride bicycle»): HOLD.
    /// - Wiktionary vetëm s'mjafton, por duhet edhe dëshmitari i dytë i përkthimit (leksiku i
    ///   modelit), kur:
    ///   - një fjalë tjetër e leksikut gramatikor (jo sinonim i dëshmuar) e ka fjalën angleze në krye
    ///     të kuptimit të dytë ose të tretë, jo të figurshëm a bisedor, jo të kufizuar («win»: «fitoj»
    ///     përballë «shperr»);
    ///   - WordNet-i ka fjalë shqipe vetëm në kuptime të tjera të shënuara në korpus dhe fjala e
    ///     Wiktionary-t s'është njëra prej tyre as sinonim i tyre («pen»: «kotec» përballë «stallë»);
    ///     pa dëshmitarin e dytë urat kundërshtohen. Kuptimet që s'janë shënuar asnjëherë në korpus
    ///     s'kundërshtojnë («yard»: «vathë» te kuptimi i nëntë).
    /// - `B1`: Wiktionary s'ka asgjë, kuptimi më i shpeshtë i WordNet-it ka një fjalë të vetme dhe
    ///   Wiktionary, kur e njeh atë fjalë me këtë pjesë ligjërate, e përkthen me një fjalë angleze
    ///   të po atij sinseti («telefon» = «telephone» për «phone»).
    fn single(&self, e: &str, p: Pos, a: &[WiktHit], b: &[BCand]) -> Result<Tr, Hold> {
        let b0: Vec<&BCand> = b.iter().filter(|c| c.ranks.contains(&0)).collect();
        let typed = |w: u32| self.info[w as usize].std_pos & pos_bit(p) != 0;
        let primary: BTreeSet<String> = a.iter().filter(|h| h.sense == 0).map(|h| fold(&h.lemma)).collect();
        let lead: BTreeSet<String> =
            a.iter().filter(|h| h.sense == 0 && h.item == 0 && typed(h.word)).map(|h| fold(&h.lemma)).collect();
        let group = |f: &str| a.iter().filter(|h| fold(&h.lemma) == f).collect::<Vec<&WiktHit>>();
        // Folja me kufizim në kllapa («to ride (horses)») vlen vetëm për atë kundrinor.
        let plain = |h: &WiktHit| !h.qualified || p != Pos::Verb;
        let rival = |f: &str, hits: &[&WiktHit]| {
            a.iter().any(|h| {
                (1..=2).contains(&h.sense)
                    && h.item == 0
                    && !h.qualified
                    && !h.marked
                    && typed(h.word)
                    && fold(&h.lemma) != f
                    && !hits.iter().any(|x| self.synonyms(x.word, h.word))
            })
        };
        let r = match (a.is_empty(), b0.is_empty(), primary.len(), lead.len()) {
            (true, true, _, _) => match b.is_empty() {
                true => Err(Hold::Unknown),
                false => Err(Hold::Ambiguous),
            },
            (true, false, _, _) => {
                let other = self.glossed.contains(&(b0[0].word, p)) && !self.cosense(e, p, b0[0].word);
                match (b0.len(), other) {
                    (1, false) => {
                        let (lemma, word) = self.spelled(&b0[0].lemma, b0[0].word, p);
                        Ok((Tr { lemma, word, code: MapCode::B1 }, false, false))
                    }
                    (1, true) => Err(Hold::Conflict),
                    _ => Err(Hold::Ambiguous),
                }
            }
            (false, false, _, _) => Err(Hold::Conflict),
            (false, true, 1, _) => {
                let f = primary.iter().next().cloned().unwrap_or_default();
                let hits = group(&f);
                let head = hits.iter().any(|h| h.sense == 0 && h.item == 0);
                let narrow = !hits.iter().any(|h| h.sense == 0 && h.item == 0 && plain(h));
                let pos_ok = hits.iter().any(|h| !self.info[h.word as usize].standard || typed(h.word));
                let known = hits.iter().any(|h| typed(h.word));
                let other = a.iter().any(|h| h.sense <= 2 && fold(&h.lemma) != f && typed(h.word));
                match (head, pos_ok, known || !other) {
                    (true, true, true) => self.only_wiktionary(&hits, p).map(|t| (t, rival(&f, &hits), narrow)),
                    _ => Err(Hold::Ambiguous),
                }
            }
            (false, true, _, 1) => {
                let f = lead.iter().next().cloned().unwrap_or_default();
                let hits = group(&f);
                let rivals = primary.iter().filter(|x| **x != f).any(|x| group(x).iter().any(|h| h.lemma.contains(' ')));
                let narrow = !hits.iter().any(|h| h.sense == 0 && h.item == 0 && plain(h));
                match rivals {
                    false => self.only_wiktionary(&hits, p).map(|t| (t, rival(&f, &hits), narrow)),
                    true => Err(Hold::Ambiguous),
                }
            }
            (false, true, _, _) => Err(Hold::Ambiguous),
        };
        // Dëshmitari i dytë: leksiku i modelit e jep të njëjtën fjalë për fjalën angleze.
        let second = |t: &Tr| self.witnessed(&t.lemma, p, e);
        let counted: Vec<&BCand> = b.iter().filter(|c| self.counted(e, p, &c.ranks)).collect();
        let apart = |t: &Tr| {
            !counted.is_empty() && !b.iter().any(|c| fold(&c.lemma) == fold(&t.lemma) || self.synonyms(c.word, t.word))
        };
        match r {
            Ok((t, contested, narrow)) => match (narrow, contested || apart(&t), second(&t), apart(&t)) {
                (true, ..) => Err(Hold::Ambiguous),
                (false, false, _, _) | (false, true, true, _) => Ok(t),
                (false, true, false, true) => Err(Hold::Conflict),
                (false, true, false, false) => Err(Hold::Ambiguous),
            },
            Err(h) => Err(h),
        }
    }

    /// Wiktionary e përkthen fjalën shqipe me një fjalë angleze të sinsetit kryesor të `e`.
    fn cosense(&self, e: &str, p: Pos, w: u32) -> bool {
        self.wn
            .get(&(e.to_string(), p))
            .and_then(|x| x.synsets.first())
            .and_then(|syn| self.g.get(&sense_key(Source::Pwn30, syn)))
            .map(|sid| {
                self.g.node(sid).labels.iter().filter(|t| t.lang == Lang::En).any(|t| {
                    self.wikt.get(&(t.text.to_lowercase(), p)).map(|v| v.iter().any(|h| h.word == w)).unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    /// Fjala e vetme e Wiktionary-t, në drejtshkrimin standard: trajta e leksikut gramatikor, pastaj
    /// ajo me «ë»/«ç».
    fn only_wiktionary(&self, hits: &[&WiktHit], p: Pos) -> Result<Tr, Hold> {
        let pick = hits.iter().min_by_key(|h| {
            (!self.info[h.word as usize].standard, h.lemma == fold(&h.lemma), h.sense, h.item, h.lemma.clone())
        });
        match pick.map(|h| self.spelled(&h.lemma, h.word, p)) {
            Some((lemma, word)) => match self.info[word as usize].beyond {
                true => Ok(Tr { lemma, word, code: MapCode::A1 }),
                false => Err(Hold::Unknown),
            },
            None => Err(Hold::Unknown),
        }
    }

    /// Dëshmia e vendimit për një fjalë angleze, për auditim: kandidatët e të dy urave dhe kodi.
    pub fn explain(&mut self, e: &str, p: Pos) -> String {
        let a: Vec<String> = self
            .wikt
            .get(&(e.to_string(), p))
            .map(|v| {
                v.iter()
                    .map(|h| {
                        let q = match (h.qualified, h.marked) {
                            (true, true) => "qm",
                            (true, false) => "q",
                            (false, true) => "m",
                            (false, false) => "",
                        };
                        format!("{}#{}.{}{q}", h.lemma, h.sense, h.item)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let hits: Vec<WiktHit> = self.wikt.get(&(e.to_string(), p)).cloned().unwrap_or_default();
        let cands = self.wordnet(e, p);
        let b: Vec<String> = cands.iter().map(|c| format!("{}@{:?}", c.lemma, c.ranks)).collect();
        let exact = self.agreements(e, p, &hits, &cands, false);
        let agree = match exact.is_empty() {
            true => self.agreements(e, p, &hits, &cands, true),
            false => exact,
        };
        let ag: Vec<String> = agree.iter().map(|x| format!("{}:{}.{}@{:?}", x.lemma, x.class, x.item, x.ranks)).collect();
        let r = match self.translate(e, p) {
            Ok(t) => format!("{} {}", t.code.code(), t.lemma),
            Err(h) => format!("HOLD {}", h.label()),
        };
        format!("{e}/{} A={a:?} B={b:?} AB={ag:?} → {r}", p.code())
    }

    fn translate(&mut self, e: &str, p: Pos) -> Result<Tr, Hold> {
        let key = (e.to_string(), p);
        match self.cache.get(&key) {
            Some(r) => r.clone(),
            None => {
                let r = self.decide(e, p);
                self.cache.insert(key, r.clone());
                r
            }
        }
    }

    fn part(&mut self, key: &'static str, tok: &str, p: Pos, base_only: bool) -> Result<Part, Hold> {
        let mut err = Hold::Unknown;
        for f in self.forms(tok, p, base_only) {
            match self.translate(&f, p) {
                Ok(t) => return Ok(Part { key, lemma: t.lemma, word: Some(t.word), code: Some(t.code), en: f }),
                Err(h) => err = err.max(h),
            }
        }
        Err(err)
    }

    /// Pjesa e ligjëratës së një fjale të vetme në vend veprimi: kur fjala është edhe folje edhe
    /// emër, emri kur kuptimet e tij janë shënuar në korpus më shumë se dyfish (katërfish kur
    /// `verb_first`) se ato të foljes («mother» si emër, «sleep» si folje).
    fn event_pos(&self, tok: &str, verb_first: bool) -> Option<Pos> {
        let v = self.forms(tok, Pos::Verb, false).first().map(|f| self.usage(f, Pos::Verb));
        let n = self.forms(tok, Pos::Noun, false).first().map(|f| self.usage(f, Pos::Noun));
        let factor = match verb_first {
            true => 4,
            false => 2,
        };
        match (v, n) {
            (None, None) => None,
            (Some(_), None) => Some(Pos::Verb),
            (None, Some(_)) => Some(Pos::Noun),
            (Some(v), Some(n)) => match n > factor * v + factor {
                true => Some(Pos::Noun),
                false => Some(Pos::Verb),
            },
        }
    }

    /// Fraza shumëfjalëshe është njësi vetëm kur WordNet-i e ka si lemë («ice cream»): pjesët e
    /// glosave të Wiktionary-t janë shpesh copa shpjegimi («go to work»), jo njësi.
    fn unit_pos(&self, joined: &str, slot: Slot) -> Option<Pos> {
        let order: &[Pos] = match slot {
            Slot::Thing => &[Pos::Noun],
            Slot::Quality => &[Pos::Adj],
            Slot::Action => &[Pos::Verb],
            Slot::Event | Slot::Act | Slot::Activity => &[Pos::Verb, Pos::Noun],
            Slot::Same => &[Pos::Adj, Pos::Verb, Pos::Noun, Pos::Adv],
        };
        order.iter().copied().find(|p| self.wn.contains_key(&(joined.to_string(), *p)))
    }

    fn one(part: Result<Part, Hold>) -> Result<Phrase, Hold> {
        part.map(|p| Phrase { parts: vec![p], main: 0 })
    }

    fn many(parts: Vec<Result<Part, Hold>>) -> Result<Phrase, Hold> {
        parts.into_iter().collect::<Result<Vec<Part>, Hold>>().map(|parts| Phrase { parts, main: 0 })
    }

    fn fixed(key: &'static str, sq: &str, en: &str) -> Result<Part, Hold> {
        Ok(Part { key, lemma: sq.to_string(), word: None, code: None, en: en.to_string() })
    }

    /// Folja në krye të frazës: trajta bazë, ose gerundi («playing games») kur s'është mbiemër. Fjala
    /// që është edhe emër vlen si folje vetëm kur emri s'është shënuar në korpus më shumë se katërfish
    /// se folja («mother ducklings» jo, «fly kite» po).
    fn lead_verb(&self, t: &str) -> bool {
        let forms = self.forms(t, Pos::Verb, false);
        let shape = self.known(t, Pos::Verb) || (t.ends_with("ing") && !self.known(t, Pos::Adj) && !forms.is_empty());
        let verb = forms.first().map(|f| self.usage(f, Pos::Verb)).unwrap_or(0);
        let noun = self.forms(t, Pos::Noun, false).first().map(|f| self.usage(f, Pos::Noun)).unwrap_or(0);
        let verbal = !self.known(t, Pos::Noun) || 4 * verb + 4 > noun;
        shape && verbal
    }

    /// Fraza shqipe për një anë të marrëdhënies. Trajta e «be» jashtë kopulës, pjesëza në fund të
    /// frazës dhe pjesëza si mbiemër i emrit mbeten jashtë, përveç kur e gjithë fraza është njësi
    /// e njohur («give up»); po ashtu pjesorja ose gerundi si mbiemër i emrit («giving rose»).
    pub fn phrase(&mut self, toks: &[String], slot: Slot) -> Result<Phrase, Hold> {
        let verbal = matches!(slot, Slot::Event | Slot::Act | Slot::Activity) && toks.len() == 2;
        let particle = |i: usize| toks.get(i).map(|t| PARTICLES.contains(&t.as_str())).unwrap_or(false);
        let stray = toks.iter().enumerate().any(|(i, t)| COPULA.contains(&t.as_str()) && !(i == 0 && verbal))
            || (toks.len() > 1 && particle(toks.len() - 1))
            || (slot == Slot::Thing && toks.len() > 1 && particle(0));
        let joined = toks.join(" ");
        let unit = match toks.len() > 1 {
            true => self.unit_pos(&joined, slot),
            false => None,
        };
        match (stray, unit) {
            (true, None) => return Err(Hold::Shape),
            _ => {}
        }
        let key = |p: Pos| match p {
            Pos::Noun => "n",
            Pos::Verb => "v",
            Pos::Adj => "a",
            Pos::Adv => "r",
        };
        match (unit, slot, toks) {
            (Some(p), _, _) => Self::one(self.part(key(p), &joined, p, false)),
            (None, Slot::Thing, [n]) => Self::one(self.part("n", n, Pos::Noun, false)),
            (None, Slot::Thing, [a, n]) => {
                let participle = (a.ends_with("ing") || a.ends_with("ed")) && !self.forms(a, Pos::Verb, false).is_empty();
                match (self.compound(a, n), participle) {
                    (Some(c), _) => Self::one(self.part("n", &c, Pos::Noun, false)),
                    (None, true) => Err(Hold::Shape),
                    (None, false) => Self::many(vec![self.part("n", n, Pos::Noun, false), self.part("a", a, Pos::Adj, true)]),
                }
            }
            (None, Slot::Quality, [a]) => Self::one(self.part("a", a, Pos::Adj, false)),
            (None, Slot::Action, [v]) => Self::one(self.part("v", v, Pos::Verb, false)),
            (None, Slot::Event | Slot::Act | Slot::Activity, [t]) => self.single_event(t, slot),
            (None, Slot::Event | Slot::Act | Slot::Activity, [first, x]) => match first.as_str() {
                "be" | "being" => self.copula(x),
                _ => self.verb_two(first, x),
            },
            (None, Slot::Event | Slot::Act | Slot::Activity, [v, m, n]) => self.verb_three(v, m, n),
            _ => Err(Hold::Shape),
        }
    }

    /// Fjala e vetme në vend veprimi:
    /// - pjesorja («punished», «tired») s'kthehet në folje veprore, se do të ndërronte zgjedhjen dhe
    ///   rolin e kryefjalës: kur WordNet-i e njeh si mbiemër jep kopulën («është i lodhur»), ndryshe HOLD;
    /// - mbiemri që s'është as folje as emër jep kopulën («be happy»);
    /// - te `Activity` fjala që është folje lexohet vetëm si folje («smoke» → «pi duhan», jo «tym»);
    /// - ndryshe pjesa e ligjëratës sipas shpeshtësisë (`event_pos`).
    fn single_event(&mut self, t: &str, slot: Slot) -> Result<Phrase, Hold> {
        let past = t.ends_with("ed") || IRREGULAR_EN.iter().any(|(f, _)| *f == t);
        let verb = !self.forms(t, Pos::Verb, false).is_empty();
        let participle = past && verb && !self.wn.contains_key(&(t.to_string(), Pos::Verb));
        let adj = self.wn.contains_key(&(t.to_string(), Pos::Adj));
        let noun = !self.forms(t, Pos::Noun, false).is_empty();
        match (participle, adj, verb || noun, slot == Slot::Activity && verb) {
            (true, true, _, _) | (false, true, false, _) => Self::one(self.part("cop", t, Pos::Adj, true)),
            (true, false, _, _) => Err(Hold::Shape),
            (false, _, _, true) => Self::one(self.part("v", t, Pos::Verb, false)),
            (false, _, _, false) => match self.event_pos(t, slot != Slot::Event) {
                Some(Pos::Noun) => Self::one(self.part("n", t, Pos::Noun, false)),
                Some(p) => Self::one(self.part("v", t, p, false)),
                None => Err(Hold::Unknown),
            },
        }
    }

    /// Folja e lehtë («have», «make», «take» …) në trajtë çfarëdo.
    fn light(&self, v: &str) -> bool {
        LIGHT_VERBS.contains(&v) || self.forms(v, Pos::Verb, false).iter().any(|f| LIGHT_VERBS.contains(&f.as_str()))
    }

    /// Emri pas foljes së lehtë është abstrakt kur kuptimi i parë, ose një kuptim i shënuar në korpus,
    /// është veprim, ngjarje, ndjenjë, gjendje, kohë … («fun», «decision», «attention»); pas «have» dhe
    /// «take» edhe ushqimi («lunch»). Emri që WordNet-i s'e njeh vlen si abstrakt.
    fn abstract_object(&self, v: &str, n: &str) -> bool {
        let eat = ["have", "take"].iter().any(|l| *l == v || self.forms(v, Pos::Verb, false).iter().any(|f| f == l));
        let form = self.forms(n, Pos::Noun, false).into_iter().next().unwrap_or_else(|| n.to_string());
        match self.wn.get(&(form, Pos::Noun)) {
            Some(w) => w.lex.iter().zip(w.counts.iter()).enumerate().any(|(r, (lex, count))| {
                (r == 0 || *count > 0) && (ABSTRACT_LEX.contains(lex) || (eat && *lex == FOOD_LEX))
            }),
            None => true,
        }
    }

    /// Emri i përbërë që WordNet-i e ka si lemë («cellular phone»): përkthehet si njësi, jo mbiemër +
    /// emër («telefon qelizor»).
    fn compound(&self, m: &str, n: &str) -> Option<String> {
        let mut forms = self.forms(n, Pos::Noun, false);
        forms.push(n.to_string());
        forms.into_iter().map(|f| format!("{m} {f}")).find(|c| self.wn.contains_key(&(c.clone(), Pos::Noun)))
    }

    /// Folja frazore: WordNet-i e njeh foljen bashkë me pjesëzën si lemë («take in», «fill in»), prandaj
    /// pjesëza s'është parafjalë e emrit që vjen pas.
    fn phrasal(&self, v: &str, m: &str) -> bool {
        let mut forms = self.forms(v, Pos::Verb, false);
        forms.push(v.to_string());
        forms.iter().any(|f| self.wn.contains_key(&(format!("{f} {m}"), Pos::Verb)))
    }

    /// «be X»: mbiemër → `cop`, emër → `copn`.
    fn copula(&mut self, x: &str) -> Result<Phrase, Hold> {
        match (self.known(x, Pos::Adj), self.forms(x, Pos::Noun, false).is_empty()) {
            (true, _) => Self::one(self.part("cop", x, Pos::Adj, true)),
            (false, false) => Self::one(self.part("copn", x, Pos::Noun, false)),
            (false, true) => Err(Hold::Unknown),
        }
    }

    /// Folje + kundrinor, folje + ndajfolje, ose mbiemër + emër. Fjala e parë që është edhe
    /// folje edhe mbiemër («clean water», «open door»), ose gerund që është edhe mbiemër
    /// («entertaining people»), mbetet e paqartë; folja frazore («give up») mbetet jashtë. Mbiemër +
    /// emër vlen vetëm kur fjala e parë s'është folje («light fire» s'është «zjarr i çelur»); folja e
    /// lehtë me emër abstrakt mbetet e paqartë («have fun», «pay attention»); folja me gerund
    /// («enjoy doing») mbetet jashtë.
    fn verb_two(&mut self, first: &str, x: &str) -> Result<Phrase, Hold> {
        let noun = !self.forms(x, Pos::Noun, false).is_empty();
        let adv = self.known(x, Pos::Adv);
        let adj = self.known(first, Pos::Adj);
        let verbal = !self.forms(first, Pos::Verb, false).is_empty() || self.known(first, Pos::Verb);
        let gerund = adj && first.ends_with("ing") && !self.forms(first, Pos::Verb, false).is_empty();
        let idiom = self.light(first) && self.abstract_object(first, x);
        // Folje + gerund («enjoy doing», «go fishing»): shqipja do lidhore, jo kundrinor.
        let catenative = x.ends_with("ing") && !self.forms(x, Pos::Verb, false).is_empty();
        match (PARTICLES.contains(&x) || gerund || catenative, self.lead_verb(first), adj, noun, adv) {
            (true, ..) => Err(Hold::Shape),
            (false, true, true, true, _) => Err(Hold::Ambiguous),
            (false, true, _, true, _) => match idiom {
                true => Err(Hold::Ambiguous),
                false => {
                    let parts = vec![self.part("v", first, Pos::Verb, false), self.part("o", x, Pos::Noun, false)];
                    Self::transitive(Self::many(parts))
                }
            },
            (false, true, _, false, true) => {
                Self::many(vec![self.part("v", first, Pos::Verb, false), self.part("r", x, Pos::Adv, true)])
            }
            (false, false, true, true, _) => match verbal {
                true => Err(Hold::Ambiguous),
                false => Self::many(vec![self.part("n", x, Pos::Noun, false), self.part("a", first, Pos::Adj, true)]),
            },
            _ => Err(Hold::Shape),
        }
    }

    /// Folja shqipe me kundrinor të drejtë duhet të jetë veprore: trajta vetvetore/pësore në «-em»
    /// («rritem», «përgjigjem») s'merr kundrinor («grow flowers» ≠ «rritem lule»).
    fn transitive(ph: Result<Phrase, Hold>) -> Result<Phrase, Hold> {
        ph.and_then(|ph| {
            let reflexive = ph.parts.first().map(|v| v.key == "v" && v.lemma.ends_with("em")).unwrap_or(false);
            let object = ph.parts.get(1).map(|o| o.key == "o").unwrap_or(false);
            match reflexive && object {
                true => Err(Hold::Shape),
                false => Ok(ph),
            }
        })
    }

    /// Folje + parafjalë + emër, ose folje + mbiemër + emër. Folja frazore me pjesëzën në mes
    /// («take in film») dhe folja e lehtë me emër abstrakt («have good time») mbeten të paqarta.
    fn verb_three(&mut self, v: &str, m: &str, n: &str) -> Result<Phrase, Hold> {
        let prep = PREPOSITIONS
            .iter()
            .find(|(en, _)| *en == m)
            .filter(|(en, _)| *en != "to" || MOTION.contains(&v))
            .map(|(en, sq)| (*en, *sq));
        let adj = self.known(m, Pos::Adj) && !self.known(m, Pos::Verb) && !PARTICLES.contains(&m);
        let idiom = (prep.is_some() && self.phrasal(v, m)) || (prep.is_none() && self.light(v) && self.abstract_object(v, n));
        match (self.lead_verb(v) && !PARTICLES.contains(&n), idiom, prep, adj) {
            (true, true, ..) => Err(Hold::Ambiguous),
            (true, false, Some((en, sq)), _) => Self::many(vec![
                self.part("v", v, Pos::Verb, false),
                Self::fixed("p", sq, en),
                self.part("o", n, Pos::Noun, false),
            ]),
            (true, false, None, true) => match self.compound(m, n) {
                Some(c) => Self::transitive(Self::many(vec![self.part("v", v, Pos::Verb, false), self.part("o", &c, Pos::Noun, false)])),
                None => Self::transitive(Self::many(vec![
                    self.part("v", v, Pos::Verb, false),
                    self.part("o", n, Pos::Noun, false),
                    self.part("a", m, Pos::Adj, true),
                ])),
            },
            _ => Err(Hold::Shape),
        }
    }

    /// E kundërta: të dyja anët me të njëjtën pjesë ligjërate; kur vlejnë disa, ajo me më shumë
    /// kuptime të shënuara në korpus.
    fn antonym(&mut self, h: &[String], t: &[String]) -> Result<(Phrase, Phrase), Hold> {
        match h.iter().chain(t.iter()).any(|x| COPULA.contains(&x.as_str())) {
            true => return Err(Hold::Shape),
            false => {}
        }
        let (hj, tj) = (h.join(" "), t.join(" "));
        let mut err = Hold::Unknown;
        let mut best: Option<(u32, Phrase, Phrase)> = None;
        for p in [Pos::Adj, Pos::Verb, Pos::Noun, Pos::Adv] {
            let key = match p {
                Pos::Noun => "n",
                Pos::Verb => "v",
                Pos::Adj => "a",
                Pos::Adv => "r",
            };
            match (self.known(&hj, p), self.known(&tj, p)) {
                (true, true) => {}
                _ => continue,
            }
            match (Self::one(self.part(key, &hj, p, true)), Self::one(self.part(key, &tj, p, true))) {
                (Ok(a), Ok(b)) => {
                    let score = self.usage(&a.parts[0].en, p) + self.usage(&b.parts[0].en, p);
                    match best.as_ref().map(|x| score > x.0).unwrap_or(true) {
                        true => best = Some((score, a, b)),
                        false => {}
                    }
                }
                (Err(x), _) | (_, Err(x)) => err = err.max(x),
            }
        }
        best.map(|(_, a, b)| (a, b)).ok_or(err)
    }

    /// Hartimi i një rreshti: (emri i marrëdhënies, koka, bishti).
    pub fn map_row(&mut self, rel: &str, head: &str, tail: &str) -> Result<(&'static str, Phrase, Phrase), Hold> {
        let spec = RELATIONS.iter().find(|r| r.0 == rel).ok_or(Hold::Relation)?;
        let h = en_tokens(head).ok_or(Hold::Shape)?;
        let t = en_tokens(tail).ok_or(Hold::Shape)?;
        let (hp, tp) = match (spec.2, spec.3) {
            (Slot::Same, _) | (_, Slot::Same) => self.antonym(&h, &t)?,
            (hs, ts) => {
                let hp = self.phrase(&h, hs)?;
                let tp = self.phrase(&t, ts)?;
                (hp, tp)
            }
        };
        match (hp.main_word(), tp.main_word()) {
            (Some(a), Some(b)) => match a == b {
                true => Err(Hold::SelfLoop),
                false => Ok((spec.1, hp, tp)),
            },
            _ => Err(Hold::Shape),
        }
    }
}

/// Identifikuesi i qëndrueshëm i një treshe shqipe: 12 shifrat e para heks të SHA-256.
pub fn cid(rel: &str, head: &str, tail: &str) -> String {
    sha256_hex(format!("{rel}|{head}|{tail}").as_bytes()).chars().take(12).collect()
}

struct Agg {
    from: u32,
    to: u32,
    weight: f64,
    en: String,
    ds: BTreeSet<String>,
    rows: usize,
    code: MapCode,
    model: bool,
}

/// Pjesa e ligjëratës së një pjese të frazës sipas çelësit të saj.
fn part_pos(key: &str) -> Option<Pos> {
    match key {
        "n" | "o" | "copn" => Some(Pos::Noun),
        "v" => Some(Pos::Verb),
        "a" | "cop" => Some(Pos::Adj),
        "r" => Some(Pos::Adv),
        _ => None,
    }
}

/// Përkthimet anglisht të leksikut të modelit: (lemë shqipe, pjesë ligjërate, fjalë angleze).
/// Modeli është një dëshmitar i përkthimit, jo autoritet: kur pajtohet me urën e vetme, skaji i dobët
/// e shënon (`witness=model_claude`); kur s'pajtohet ose hesht, skaji mbetet ashtu siç është.
pub fn model_witness(text: &str) -> HashSet<(String, Pos, String)> {
    let mut out = HashSet::new();
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.starts_with("lemma\t") && !l.trim().is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        let lemma = c.first().map(|x| x.trim().to_lowercase()).unwrap_or_default();
        let lemma = strip_article(&lemma).to_string();
        match c.get(1).and_then(|x| Pos::from_wn(x.trim())) {
            Some(p) => {
                for en in c.get(4).copied().unwrap_or("").split([',', ';']) {
                    let en = en.trim().to_lowercase();
                    let en = en.strip_prefix("to ").map(str::to_string).unwrap_or(en);
                    match en.is_empty() {
                        true => {}
                        false => {
                            out.insert((lemma.clone(), p, en));
                        }
                    }
                }
            }
            None => {}
        }
    }
    out
}

/// Dëshmia e modelit për treshet: `cid ⇥ gjykimi ⇥ arsyeja` → (cid, arsyeja). Merren vetëm rreshtat
/// `CONFIRM`; çdo gjykim tjetër s'është dëshmi dhe s'është veto.
pub fn witness_rows(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("cid\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').map(str::trim).collect();
            match c.as_slice() {
                [id, "CONFIRM", rest @ ..] => Some((id.to_string(), rest.first().copied().unwrap_or("").to_string())),
                _ => None,
            }
        })
        .collect()
}

/// Grupet e ndryshme të të dhënave të ConceptNet-it që e japin skajin (atributi `ds`).
fn datasets(e: &Edge) -> usize {
    e.attr("ds").unwrap_or("").split(',').map(str::trim).filter(|d| !d.is_empty()).collect::<BTreeSet<_>>().len()
}

/// Treshja `CS_…` thuhet vetëm me dy dëshmitarë të pavarur: skaji i ConceptNet-it bashkë me dëshminë
/// e modelit për të njëjtën treshe (`witness=cs`, i njëjti `cid`), ose ConceptNet-i nga dy grupe të
/// dhënash kur përkthimin e dëshmojnë të dyja urat (`AB`, `AB2`), si te `biseda::spoken`. `edge`
/// është cilido skaj i treshes; dëshmia e modelit vetëm s'mjafton.
pub fn spoken(g: &Graph, from: u32, edge: &Edge) -> bool {
    let id = match &edge.rel {
        Rel::Sem(r) => r.starts_with(PREFIX),
        _ => false,
    }
    .then(|| edge.attr("cid"))
    .flatten();
    match id {
        None => false,
        Some(id) => {
            let same: Vec<&Edge> = g
                .out_edges(from)
                .map(|(_, e)| e)
                .filter(|e| e.to == edge.to && e.rel == edge.rel && e.attr("cid") == Some(id))
                .collect();
            let cn: Vec<&Edge> = same.iter().copied().filter(|e| e.source == Source::ConceptNet57).collect();
            let model = same.iter().any(|e| e.source == Source::ModelClaude && e.attr("witness") == Some(WITNESS_CS));
            let two = cn.iter().any(|e| datasets(e) >= 2 && matches!(e.attr("map"), Some("AB" | "AB2")));
            match cn.is_empty() {
                true => false,
                false => model || two,
            }
        }
    }
}

/// Importon shtresën: `text` = TSV-ja e ConceptNet-it anglisht, `kaikki` = TSV-ja e Wiktionary-t
/// (ura A), `pwn_index` + `sense_index` = indekset e WordNet-it (renditja dhe shpeshtësia e
/// kuptimeve për urën B), `model` = leksiku i modelit (dëshmitar i përkthimit të skajeve të dobëta;
/// bosh kur mungon), `deshmi` = dëshmia e modelit për treshet (dëshmitari i dytë i lidhjes; bosh
/// kur mungon).
#[allow(clippy::too_many_arguments)]
pub fn import(
    g: &mut Graph,
    text: &str,
    kaikki: &str,
    pwn_index: &[(Pos, &str)],
    sense_index: &str,
    model: &str,
    deshmi: &str,
    rep: &mut ImportReport,
) -> usize {
    let mut holds: BTreeMap<Hold, usize> = BTreeMap::new();
    let (mut rows, mut kept) = (0usize, 0usize);
    let mut agg: BTreeMap<(&'static str, String, String), Agg> = BTreeMap::new();
    let witness = model_witness(model);
    {
        let mut m = Mapper::new(g, kaikki, pwn_index, sense_index).with_witness(witness.clone());
        for l in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
            let c: Vec<&str> = l.split('\t').collect();
            let (rel, head, tail, w, ds) = match c.as_slice() {
                [r, h, t, w, d, ..] => (*r, *h, *t, w.parse::<f64>().unwrap_or(1.0), *d),
                _ => continue,
            };
            rows += 1;
            let mapped = match kept_dataset(ds) {
                true => {
                    kept += 1;
                    m.map_row(rel, head, tail)
                }
                false => Err(Hold::Dataset),
            };
            match mapped {
                Ok((name, hp, tp)) => {
                    let code = hp.code().max(tp.code());
                    let weak: Vec<&Part> =
                        hp.parts.iter().chain(tp.parts.iter()).filter(|p| p.code.map(MapCode::weak).unwrap_or(false)).collect();
                    let seen = |p: &&Part| {
                        part_pos(p.key).map(|q| witness.contains(&(p.lemma.clone(), q, p.en.clone()))).unwrap_or(false)
                    };
                    let confirmed = !weak.is_empty() && weak.iter().all(seen);
                    let key = (name, hp.structured(), tp.structured());
                    let (from, to) = (hp.main_word().unwrap_or(0), tp.main_word().unwrap_or(0));
                    let a = agg.entry(key).or_insert_with(|| Agg {
                        from,
                        to,
                        weight: f64::MIN,
                        en: String::new(),
                        ds: BTreeSet::new(),
                        rows: 0,
                        code,
                        model: false,
                    });
                    let en = format!("{head}|{tail}");
                    match (w > a.weight, w == a.weight && en < a.en) {
                        (true, _) | (_, true) => {
                            a.weight = w;
                            a.en = en;
                        }
                        _ => {}
                    }
                    a.ds.insert(ds.to_string());
                    a.rows += 1;
                    a.code = a.code.min(code);
                    a.model |= confirmed;
                }
                Err(h) => *holds.entry(h).or_default() += 1,
            }
        }
    }
    let confirmed = witness_rows(deshmi);
    let mut matched: BTreeSet<String> = BTreeSet::new();
    let mut cs: Vec<(&'static str, u32, u32)> = Vec::with_capacity(agg.len());
    let mut by_rel: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_code: BTreeMap<&'static str, usize> = BTreeMap::new();
    let (mut merged, mut witnessed) = (0usize, 0usize);
    for ((name, head, tail), a) in &agg {
        let id = cid(name, head, tail);
        let mut attrs: Vec<(String, String)> = vec![
            ("en".into(), a.en.clone()),
            ("w".into(), format!("{:.3}", a.weight)),
            ("ds".into(), a.ds.iter().cloned().collect::<Vec<_>>().join(",")),
            ("rows".into(), a.rows.to_string()),
            ("head".into(), head.clone()),
            ("tail".into(), tail.clone()),
            ("map".into(), a.code.code().into()),
            ("cid".into(), id.clone()),
        ];
        match (a.code.weak(), a.model) {
            (true, true) => {
                attrs.push(("weak".into(), "1".into()));
                attrs.push(("witness".into(), Source::ModelClaude.code().into()));
                witnessed += 1;
            }
            (true, false) => attrs.push(("weak".into(), "1".into())),
            (false, _) => {}
        }
        let eid = g.add_edge(Edge {
            from: a.from,
            to: a.to,
            rel: Rel::Sem(name.to_string()),
            source: Source::ConceptNet57,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs,
        });
        match confirmed.get(&id) {
            Some(reason) => {
                g.add_edge(Edge {
                    from: a.from,
                    to: a.to,
                    rel: Rel::Sem(name.to_string()),
                    source: Source::ModelClaude,
                    lang: Some(Lang::Sq),
                    method: Method::ModelAuthored,
                    attrs: vec![
                        ("witness".into(), WITNESS_CS.into()),
                        ("cid".into(), id.clone()),
                        ("head".into(), head.clone()),
                        ("tail".into(), tail.clone()),
                        ("reason".into(), reason.clone()),
                    ],
                });
                matched.insert(id);
            }
            None => {}
        }
        cs.push((name, a.from, eid));
        *by_rel.entry(name).or_default() += 1;
        *by_code.entry(a.code.code()).or_default() += 1;
        merged += a.rows;
    }
    let mut said: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (name, from, eid) in &cs {
        match spoken(g, *from, &g.edges[*eid as usize]) {
            true => *said.entry(name).or_default() += 1,
            false => {}
        }
    }
    let holds: BTreeMap<&str, usize> = holds.into_iter().map(|(h, n)| (h.label(), n)).collect();
    rep.note(format!(
        "conceptnet_en: {rows} rreshta, {kept} nga OMCS/Verbosity/Wiktionary (WordNet, OpenCyc, DBpedia jashtë); {} skaje shqipe CS_ nga {merged} rreshta, sipas hartimit {by_code:?} (AB = dy ura të pajtuara, A1/B1 = një urë, i dobët; {witnessed} të dobëta me përkthimin e dëshmuar edhe nga leksiku i modelit); {} me dëshminë e modelit për treshen ({WITNESS}: {} CONFIRM, {} pa skaj përkatës; skaj i dyfishtë model_claude, witness=cs); HOLD {holds:?}; sipas marrëdhënies {by_rel:?}",
        agg.len(),
        matched.len(),
        confirmed.len(),
        confirmed.len() - matched.len()
    ));
    rep.note(format!(
        "conceptnet_en: {} treshe të folshme (dy dëshmitarë: ConceptNet + dëshmia e modelit, ose ConceptNet nga dy grupe të dhënash me përkthim AB/AB2); sipas marrëdhënies {said:?}",
        said.values().sum::<usize>()
    ));
    agg.len()
}

/// Nyja e citimit të mbiemrit sipas gramatikës («i»/«e»); «i» kur s'dihet.
fn article(g: &Graph, adj: &str) -> String {
    g.get(&word_key(Lang::Sq, adj))
        .and_then(|w| g.out_edges(w).find_map(|(_, e)| match e.rel == Rel::Lex {
            true => e.attr("article").map(str::to_string),
            false => None,
        }))
        .unwrap_or_else(|| "i".to_string())
}

/// Përshkrim i thjeshtë shqip i një fraze të strukturuar, me lema: «v=ruaj;o=shtëpi» → «ruaj
/// shtëpi», «n=ujë;a=kripur» → «ujë i kripur», «cop=lumtur» → «është i lumtur».
pub fn render(g: &Graph, structured: &str) -> String {
    let parts: Vec<(&str, &str)> = structured.split(';').filter_map(|p| p.split_once('=')).collect();
    let verbal = parts.iter().any(|(k, _)| *k == "v");
    parts
        .iter()
        .map(|(k, v)| match (*k, verbal) {
            ("a", true) => format!("të {v}"),
            ("a", false) => format!("{} {v}", article(g, v)),
            ("cop", _) => format!("është {} {v}", article(g, v)),
            ("copn", _) => format!("është {v}"),
            _ => v.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(g: &mut Graph, w: &str, pos: Pos) -> u32 {
        let wid = g.ensure_word(Lang::Sq, w);
        g.add_edge(Edge {
            from: wid,
            to: wid,
            rel: Rel::Lex,
            source: Source::Uniparser,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![("pos".into(), pos.code().into())],
        });
        wid
    }

    /// Fjalë jashtë leksikut gramatikor, e dëshmuar vetëm nga LEXIN-i.
    fn rare(g: &mut Graph, w: &str, pos: Pos) -> u32 {
        let wid = g.ensure_word(Lang::Sq, w);
        g.add_edge(Edge {
            from: wid,
            to: wid,
            rel: Rel::Lex,
            source: Source::Lexin,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![("pos".into(), pos.code().into())],
        });
        wid
    }

    fn sense(g: &mut Graph, syn: &str, pos: Pos, sq: &[&str]) {
        let sid = g.ensure(sense_key(Source::Pwn30, syn), NodeKind::Sense { source: Source::Pwn30, synset: syn.into(), pos });
        for w in sq {
            let wid = lex(g, strip_article(w), pos);
            let wid = match strip_article(w) == *w {
                true => wid,
                false => g.ensure_word(Lang::Sq, w),
            };
            g.add_edge(Edge {
                from: wid,
                to: sid,
                rel: Rel::HasSense,
                source: Source::OmwAls,
                lang: Some(Lang::Sq),
                method: Method::OmwOffsetMap,
                attrs: vec![],
            });
        }
    }

    /// Graf i vogël: qen/shtëpi/ruaj/ujë/kripur me dy urat; leh vetëm në Wiktionary; breg/bankë
    /// në dy kuptime të ndryshme; mace/macë sinonime në të njëjtin kuptim.
    fn fixture() -> (Graph, String, Vec<(Pos, String)>) {
        let mut g = Graph::new();
        sense(&mut g, "00000001-n", Pos::Noun, &["qen"]);
        sense(&mut g, "00000002-n", Pos::Noun, &["shtëpi"]);
        sense(&mut g, "00000003-v", Pos::Verb, &["ruaj", "mbroj"]);
        sense(&mut g, "00000004-n", Pos::Noun, &["ujë"]);
        sense(&mut g, "00000005-a", Pos::Adj, &["i kripur"]);
        sense(&mut g, "00000006-n", Pos::Noun, &["breg"]);
        sense(&mut g, "00000007-n", Pos::Noun, &["bankë"]);
        sense(&mut g, "00000008-n", Pos::Noun, &["mace", "macë"]);
        sense(&mut g, "00000009-n", Pos::Noun, &["shkollë"]);
        sense(&mut g, "00000010-v", Pos::Verb, &["shkoj"]);
        sense(&mut g, "00000011-n", Pos::Noun, &["muzikë"]);
        sense(&mut g, "00000012-v", Pos::Verb, &["dëgjoj"]);
        sense(&mut g, "00000013-a", Pos::Adj, &["i lumtur"]);
        sense(&mut g, "00000014-n", Pos::Noun, &["dru"]);
        sense(&mut g, "00000016-n", Pos::Noun, &["njeri"]);
        lex(&mut g, "leh", Pos::Verb);
        lex(&mut g, "pemë", Pos::Noun);
        lex(&mut g, "insan", Pos::Noun);
        let kaikki = [
            "E\tqen\tnoun\tm\t\t",
            "G\t1\tmasculine\tdog",
            "G\t2\tfiguratively\tunfaithful person",
            "E\tshtëpi\tnoun\tf\t\t",
            "G\t1\tfeminine\thouse",
            "E\truaj\tverb\t\t\t",
            "G\t1\t\tto watch over, to observe, to guard",
            "E\tleh\tverb\t\t\t",
            "G\t1\tintransitive\tto bark, yap",
            "E\tujë\tnoun\tm\t\t",
            "G\t1\t\twater",
            "E\ti kripur\tadj\t\t\t",
            "G\t1\t\tsalt, salty",
            "E\tbankë\tnoun\tf\t\t",
            "G\t1\t\tbank",
            "E\tbreg\tnoun\tm\t\t",
            "G\t1\t\tshore, bank",
            "E\tmace\tnoun\tf\t\t",
            "G\t1\t\tcat",
            "E\tmacë\tnoun\tf\t\t",
            "G\t1\t\tcat",
            "E\tshkollë\tnoun\tf\t\t",
            "G\t1\t\tschool",
            "E\tshkoj\tverb\t\t\t",
            "G\t1\t\tto go",
            "E\tlumtur\tadj\t\t\t",
            "G\t1\t\thappy",
            "E\tpemë\tnoun\tf\t\t",
            "G\t1\t\ttree",
            "E\tnjeri\tnoun\tm\t\t",
            "G\t1\t\tman",
            "G\t2\t\thuman, person, individual",
            "E\tinsan\tnoun\tm\t\t",
            "G\t1\tmasculine\tperson, human",
            "E\tdru\tnoun\tm\t\t",
            "G\t1\t\twood, timber",
        ]
        .join("\n");
        let idx = vec![
            (Pos::Noun, [
                "dog n 1 0 1 1 00000001",
                "house n 1 0 1 1 00000002",
                "water n 1 0 1 1 00000004",
                "bank n 2 0 2 2 00000006 00000007",
                "cat n 1 0 1 1 00000008",
                "school n 1 0 1 1 00000009",
                "music n 1 0 1 1 00000011",
                "wood n 1 0 1 1 00000014",
                "person n 1 0 1 1 00000016",
            ].join("\n")),
            (Pos::Verb, ["guard v 1 0 1 1 00000003", "go v 1 0 1 1 00000010", "listen v 1 0 1 1 00000012"].join("\n")),
            (Pos::Adj, ["salt a 1 0 1 1 00000005", "happy a 1 0 1 1 00000013"].join("\n")),
        ];
        (g, kaikki, idx)
    }

    fn mapper<'a>(g: &'a Graph, kaikki: &str, idx: &[(Pos, String)]) -> Mapper<'a> {
        let refs: Vec<(Pos, &str)> = idx.iter().map(|(p, t)| (*p, t.as_str())).collect();
        Mapper::new(g, kaikki, &refs, "")
    }

    fn toks(s: &str) -> Vec<String> {
        en_tokens(s).unwrap()
    }

    #[test]
    fn english_terms_lose_determiners_and_block_pronouns() {
        assert_eq!(toks("guard_the_house"), vec!["guard", "house"]);
        assert_eq!(toks("eat_your_food"), vec!["eat", "food"]);
        assert_eq!(toks("to_eat_food"), vec!["eat", "food"]);
        assert_eq!(en_tokens("eat_it"), None);
        assert_eq!(en_tokens("not_sleep"), None);
        assert_eq!(en_tokens("cats_and_dogs"), None);
        assert_eq!(en_tokens("a.m"), None);
    }

    #[test]
    fn gloss_items_split_the_first_gloss_into_translations() {
        let plain = |g: &str| gloss_items(g).into_iter().map(|(t, _)| t).collect::<Vec<_>>();
        assert_eq!(plain("to watch over, to observe, to guard"), vec!["watch over", "observe", "guard"]);
        assert_eq!(plain("to bark, yap"), vec!["bark", "yap"]);
        assert!(plain("one who works in a mine").is_empty());
        // Kufizimi në kllapa e shënon pjesën si kuptim të ngushtë.
        assert_eq!(gloss_items("a (male) dog; hound"), vec![("dog".to_string(), true), ("hound".to_string(), false)]);
        assert_eq!(gloss_items("Russian (language)"), vec![("russian".to_string(), true)]);
    }

    #[test]
    fn two_agreeing_bridges_give_ab_and_one_bridge_is_weak() {
        let (g, k, idx) = fixture();
        let mut m = mapper(&g, &k, &idx);
        let t = m.translate("dog", Pos::Noun).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("qen", MapCode::AB));
        // Folja: «guard» është në glosën e parë të «ruaj» dhe «ruaj» është në sinsetin e «guard».
        let t = m.translate("guard", Pos::Verb).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("ruaj", MapCode::AB));
        // Vetëm Wiktionary, një kandidat: i dobët.
        let t = m.translate("bark", Pos::Verb).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("leh", MapCode::A1));
        // Mbiemri pa nyjë: «i kripur» (AlbaNet) = «kripur» (Wiktionary).
        let t = m.translate("salt", Pos::Adj).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("kripur", MapCode::AB));
        // Sinonime në të njëjtin kuptim: zgjidhet njëri, pa paqartësi.
        assert_eq!(m.translate("cat", Pos::Noun).unwrap().code, MapCode::AB);
        // Kuptimi kryesor i «insan» është «person», por WordNet-i thotë «njeri»; kuptimi i dytë i
        // «njeri» në Wiktionary e dëshmon: dy ura pajtohen jashtë kuptimit kryesor.
        let t = m.translate("person", Pos::Noun).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("njeri", MapCode::AB2));
        assert!(m.explain("person", Pos::Noun).contains("njeri#1.1"));
    }

    #[test]
    fn ambiguity_and_conflict_hold_instead_of_guessing() {
        let (mut g, k, mut idx) = fixture();
        let mut m = mapper(&g, &k, &idx);
        // «bank»: bankë dhe breg pajtohen të dy, por në kuptime të ndryshme.
        assert_eq!(m.translate("bank", Pos::Noun).unwrap_err(), Hold::Ambiguous);
        assert_eq!(m.translate("unicorn", Pos::Noun).unwrap_err(), Hold::Unknown);
        drop(m);
        // Wiktionary thotë «pemë» për «tree», WordNet-i «dru»: urat kundërshtohen.
        sense(&mut g, "00000015-n", Pos::Noun, &["dru"]);
        idx[0].1.push_str("\ntree n 1 0 1 1 00000015");
        let mut m = mapper(&g, &k, &idx);
        assert_eq!(m.translate("tree", Pos::Noun).unwrap_err(), Hold::Conflict);
    }

    #[test]
    fn phrases_become_structured_albanian_tails() {
        let (g, k, idx) = fixture();
        let mut m = mapper(&g, &k, &idx);
        assert_eq!(m.phrase(&toks("guard_house"), Slot::Act).unwrap().structured(), "v=ruaj;o=shtëpi");
        assert_eq!(m.phrase(&toks("salt_water"), Slot::Thing).unwrap().structured(), "n=ujë;a=kripur");
        assert_eq!(m.phrase(&toks("be_happy"), Slot::Act).unwrap().structured(), "cop=lumtur");
        assert_eq!(m.phrase(&toks("go_to_school"), Slot::Act).unwrap().structured(), "v=shkoj;p=në;o=shkollë");
        assert_eq!(m.phrase(&toks("barking"), Slot::Act).unwrap().structured(), "v=leh");
        assert_eq!(m.phrase(&toks("dogs"), Slot::Thing).unwrap().structured(), "n=qen");
        // «to» pas një foljeje që s'është lëvizje nuk përkthehet: HOLD.
        assert_eq!(m.phrase(&toks("listen_to_music"), Slot::Act).unwrap_err(), Hold::Shape);
        // Fjalë e pahartuar brenda frazës: HOLD për gjithë frazën.
        assert!(m.phrase(&toks("guard_unicorn"), Slot::Act).is_err());
        let p = m.phrase(&toks("guard_house"), Slot::Act).unwrap();
        assert_eq!(p.main_word(), g.get(&word_key(Lang::Sq, "ruaj")));
    }

    #[test]
    fn duplicate_albanian_triples_merge_into_one_edge() {
        let (mut g, k, idx) = fixture();
        let refs: Vec<(Pos, &str)> = idx.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let text = [
            "# koka",
            "/r/CapableOf\tdog\tguard_house\t2.000\t/d/conceptnet/4/en",
            "/r/CapableOf\tdogs\tguard_the_house\t1.000\t/d/verbosity",
            "/r/CapableOf\tdog\tbark\t1.000\t/d/conceptnet/4/en",
            "/r/IsA\tdog\thouse\t1.000\t/d/wordnet/3.1",
            "/r/AtLocation\tdog\tbank\t1.000\t/d/conceptnet/4/en",
        ]
        .join("\n");
        let model = "lemma\tpos\tgender\tdefinition\tenglish\tcategory\tsynonyms\tantonyms\tderived_from\n\
                     leh\tv\t\tlëshon zërin e qenit\tbark\t\t\t\t\n\
                     ruaj\tv\t\tmbaj larg rrezikut\tkeep\t\t\t\t\n";
        let mut rep = ImportReport::default();
        let n = import(&mut g, &text, &k, &refs, "", model, "", &mut rep);
        assert_eq!(n, 2);
        let qen = g.get(&word_key(Lang::Sq, "qen")).unwrap();
        let cs: Vec<&Edge> = g.out_edges(qen).map(|(_, e)| e).filter(|e| e.rel == Rel::Sem("CS_CAPABLE_OF".into())).collect();
        assert_eq!(cs.len(), 2);
        let guard = cs.iter().find(|e| e.attr("tail") == Some("v=ruaj;o=shtëpi")).unwrap();
        assert_eq!(guard.attr("rows"), Some("2"));
        assert_eq!(guard.attr("w"), Some("2.000"));
        assert_eq!(guard.attr("ds"), Some("/d/conceptnet/4/en,/d/verbosity"));
        assert_eq!(guard.attr("en"), Some("dog|guard_house"));
        assert_eq!(guard.attr("map"), Some("AB"));
        assert_eq!(guard.attr("cid").map(str::len), Some(12));
        assert_eq!(guard.source, Source::ConceptNet57);
        let bark = cs.iter().find(|e| e.attr("tail") == Some("v=leh")).unwrap();
        assert_eq!((bark.attr("map"), bark.attr("weak")), (Some("A1"), Some("1")));
        // Leksiku i modelit e dëshmon përkthimin e dobët «bark» → «leh»; skaji mbetet i dobët dhe
        // nga ConceptNet-i, me dëshmitarin të shënuar. Skaji AB nuk merr shënim.
        assert_eq!(bark.attr("witness"), Some("model_claude"));
        assert_eq!(bark.source, Source::ConceptNet57);
        assert_eq!(guard.attr("witness"), None);
        assert!(rep.lines[0].contains("dataset i përjashtuar"), "{}", rep.lines[0]);
        assert_eq!(render(&g, "n=ujë;a=kripur"), "ujë i kripur");
        assert_eq!(render(&g, "v=ruaj;o=shtëpi"), "ruaj shtëpi");
        assert_eq!(render(&g, "cop=lumtur"), "është i lumtur");
    }

    #[test]
    fn phrasal_verbs_stray_copula_and_reflexive_objects_hold() {
        let (mut g, mut k, mut idx) = fixture();
        sense(&mut g, "00000017-v", Pos::Verb, &["rritem"]);
        sense(&mut g, "00000018-n", Pos::Noun, &["lule"]);
        k.push_str("\nE\trritem\tverb\t\t\t\nG\t1\t\tto grow\nE\tlule\tnoun\tf\t\t\nG\t1\t\tflower");
        idx[0].1.push_str("\nflower n 1 0 1 1 00000018");
        idx[1].1.push_str("\ngrow v 1 0 1 1 00000017");
        let mut m = mapper(&g, &k, &idx);
        // «rritem» është vetvetore: «grow flowers» s'bëhet «rritem lule».
        assert_eq!(m.phrase(&toks("grow_flowers"), Slot::Act).unwrap_err(), Hold::Shape);
        assert_eq!(m.phrase(&toks("grow"), Slot::Act).unwrap().structured(), "v=rritem");
        // Pjesëza në fund: folje frazore, jo folje + ndajfolje ose emër.
        assert_eq!(m.phrase(&toks("go_out"), Slot::Act).unwrap_err(), Hold::Shape);
        // Trajta e «be» vetëm në krye të kopulës.
        assert_eq!(m.phrase(&toks("dog_is_happy"), Slot::Act).unwrap_err(), Hold::Shape);
        assert_eq!(m.phrase(&toks("being_happy"), Slot::Act).unwrap().structured(), "cop=lumtur");
        // Shumësi merr njëjësin.
        assert_eq!(m.phrase(&toks("flowers"), Slot::Thing).unwrap().structured(), "n=lule");
    }

    #[test]
    fn single_bridge_needs_a_clean_witness() {
        let (mut g, mut k, mut idx) = fixture();
        // «smart»: glosa kryesore e një fjale jashtë leksikut gramatikor, ndërsa një fjalë e leksikut
        // e ka në një kuptim tjetër: HOLD.
        rare(&mut g, "veshtë", Pos::Adj);
        lex(&mut g, "zgjuar", Pos::Adj);
        // «phone»: vetëm WordNet; Wiktionary e përkthen «telefon» me «telephone», të po atij sinseti.
        sense(&mut g, "00000019-n", Pos::Noun, &["telefon"]);
        let sid = g.get(&sense_key(Source::Pwn30, "00000019-n")).unwrap();
        for l in ["telephone", "phone"] {
            g.add_label(sid, Text { lang: Lang::En, text: l.into(), source: Source::Pwn30 });
        }
        // «lettuce»: vetëm WordNet, por Wiktionary e njeh «ryshfet» si «bribe»: kundërshti.
        sense(&mut g, "00000020-n", Pos::Noun, &["ryshfet"]);
        k.push_str(
            "\nE\tveshtë\tadj\t\t\t\nG\t1\t\tsmart\nE\tzgjuar\tadj\t\t\t\nG\t1\t\tawake\nG\t2\t\tclever, smart\
             \nE\ttelefon\tnoun\tm\t\t\nG\t1\t\ttelephone\nE\tryshfet\tnoun\tm\t\t\nG\t1\t\tbribe",
        );
        idx[0].1.push_str("\nphone n 1 0 1 1 00000019\ntelephone n 1 0 1 1 00000019\nlettuce n 1 0 1 1 00000020");
        let mut m = mapper(&g, &k, &idx);
        assert_eq!(m.translate("smart", Pos::Adj).unwrap_err(), Hold::Ambiguous);
        let t = m.translate("phone", Pos::Noun).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("telefon", MapCode::B1));
        assert_eq!(m.translate("lettuce", Pos::Noun).unwrap_err(), Hold::Conflict);
    }

    #[test]
    fn minor_sense_agreement_holds_unless_wiktionary_is_decisive() {
        let (mut g, mut k, mut idx) = fixture();
        // «floor»: «kat» pajtohet te kuptimi i dytë; kuptimi kryesor (dyshemeja) ka fjalë tjetër dhe
        // është më shumë se dyfish më i shpeshtë; Wiktionary njeh edhe «lëmë»: HOLD.
        sense(&mut g, "00000021-n", Pos::Noun, &["dysheme"]);
        sense(&mut g, "00000022-n", Pos::Noun, &["kat"]);
        lex(&mut g, "lëmë", Pos::Noun);
        // «find»: «gjej» është në krye të glosës së vetme që e ka «find»: vendimtar, edhe pse AlbaNet-i
        // e vë një fjalë tjetër te kuptimi i parë.
        sense(&mut g, "00000023-v", Pos::Verb, &["ndesh"]);
        sense(&mut g, "00000025-v", Pos::Verb, &["gjej"]);
        lex(&mut g, "pikas", Pos::Verb);
        k.push_str(
            "\nE\tkat\tnoun\tm\t\t\nG\t1\t\tfloor, storey\nE\tlëmë\tnoun\tf\t\t\nG\t1\t\tfloor, threshing floor\
             \nE\tgjej\tverb\t\t\t\nG\t1\t\tto find\nE\tpikas\tverb\t\t\t\nG\t1\t\tto notice\nG\t2\t\tto find",
        );
        idx[0].1.push_str("\nfloor n 2 0 2 2 00000021 00000022");
        idx[1].1.push_str("\nfind v 3 0 3 3 00000023 00000024 00000025");
        let senses = [
            "floor%1:06:00:: 00000021 1 38",
            "floor%1:06:01:: 00000022 2 16",
            "find%2:40:02:: 00000023 1 159",
            "find%2:39:02:: 00000024 2 141",
            "find%2:40:00:: 00000025 3 86",
        ]
        .join("\n");
        let refs: Vec<(Pos, &str)> = idx.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let mut m = Mapper::new(&g, &k, &refs, &senses);
        assert_eq!(m.translate("floor", Pos::Noun).unwrap_err(), Hold::Ambiguous);
        let t = m.translate("find", Pos::Verb).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("gjej", MapCode::AB));
    }

    #[test]
    fn model_lexicon_witnesses_but_never_vetoes() {
        let model = "lemma\tpos\tgender\tdefinition\tenglish\tcategory\tsynonyms\tantonyms\tderived_from\n\
                     i kripur\ta\t\tqë ka kripë\tsalty, salt\t\t\t\t\n\
                     leh\tv\t\tlëshon zë\tgrowl\t\t\t\t\n";
        let w = model_witness(model);
        assert!(w.contains(&("kripur".to_string(), Pos::Adj, "salt".to_string())));
        let (mut g, k, idx) = fixture();
        let refs: Vec<(Pos, &str)> = idx.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let mut rep = ImportReport::default();
        import(&mut g, "/r/CapableOf\tdog\tbark\t1.000\t/d/conceptnet/4/en", &k, &refs, "", model, "", &mut rep);
        let qen = g.get(&word_key(Lang::Sq, "qen")).unwrap();
        let bark = g.out_edges(qen).map(|(_, e)| e).find(|e| e.attr("tail") == Some("v=leh")).unwrap();
        // Modeli thotë tjetër («growl»): skaji i dobët mbetet, pa shënimin e dëshmitarit.
        assert_eq!((bark.attr("map"), bark.attr("weak"), bark.attr("witness")), (Some("A1"), Some("1"), None));
    }

    #[test]
    fn model_witness_doubles_only_confirmed_triples_and_never_speaks_alone() {
        let (mut g, k, idx) = fixture();
        let refs: Vec<(Pos, &str)> = idx.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let text = [
            "/r/CapableOf\tdog\tguard_house\t2.000\t/d/conceptnet/4/en",
            "/r/CapableOf\tdog\tbark\t1.000\t/d/conceptnet/4/en",
            "/r/CapableOf\tdogs\tbark\t1.000\t/d/verbosity",
            "/r/AtLocation\tdog\thouse\t1.000\t/d/conceptnet/4/en",
            "/r/AtLocation\tdogs\thouse\t1.000\t/d/verbosity",
        ]
        .join("\n");
        let guard = cid("CS_CAPABLE_OF", "n=qen", "v=ruaj;o=shtëpi");
        let bark = cid("CS_CAPABLE_OF", "n=qen", "v=leh");
        let house = cid("CS_AT_LOCATION", "n=qen", "n=shtëpi");
        let deshmi = format!(
            "# dëshmi\ncid\tverdict\treason\n{guard}\tCONFIRM\tqeni ruan shtëpinë\n{bark}\tREJECT\tjo\nffffffffffff\tCONFIRM\tpa skaj\n"
        );
        assert_eq!(witness_rows(&deshmi).len(), 2);
        let mut rep = ImportReport::default();
        import(&mut g, &text, &k, &refs, "", "", &deshmi, &mut rep);
        let qen = g.get(&word_key(Lang::Sq, "qen")).unwrap();
        let edges: Vec<&Edge> = g.out_edges(qen).map(|(_, e)| e).collect();
        let count = |c: &str, s: Source| edges.iter().filter(|e| e.attr("cid") == Some(c) && e.source == s).count();
        let cn = |c: &str| *edges.iter().find(|e| e.attr("cid") == Some(c) && e.source == Source::ConceptNet57).unwrap();
        // CONFIRM: skaj i dyfishtë pranë skajit të ConceptNet-it; REJECT dhe heshtja: vetëm ConceptNet-i.
        assert_eq!((count(&guard, Source::ConceptNet57), count(&guard, Source::ModelClaude)), (1, 1));
        assert_eq!((count(&bark, Source::ConceptNet57), count(&bark, Source::ModelClaude)), (1, 0));
        assert_eq!((count(&house, Source::ConceptNet57), count(&house, Source::ModelClaude)), (1, 0));
        assert_eq!(g.edges.iter().filter(|e| e.source == Source::ModelClaude).count(), 1);
        let witness = *edges.iter().find(|e| e.source == Source::ModelClaude).unwrap();
        assert_eq!(witness.rel, Rel::Sem("CS_CAPABLE_OF".into()));
        assert_eq!((witness.attr("witness"), witness.attr("tail"), witness.method), (Some("cs"), Some("v=ruaj;o=shtëpi"), Method::ModelAuthored));
        // Dy dëshmitarë: ConceptNet + modeli; ose dy grupe të dhënash me përkthim AB.
        assert!(spoken(&g, qen, cn(&guard)));
        assert!(spoken(&g, qen, witness));
        assert!(spoken(&g, qen, cn(&house)));
        // Dy grupe të dhënash, por përkthim i dobët (A1): një dëshmitar.
        assert_eq!(cn(&bark).attr("map"), Some("A1"));
        assert!(!spoken(&g, qen, cn(&bark)));
        assert!(rep.lines[0].contains(WITNESS) && rep.lines[0].contains("1 pa skaj përkatës"), "{}", rep.lines[0]);
        // Dëshmia e modelit pa skajin e ConceptNet-it nuk thuhet.
        let mut solo = Graph::new();
        let (a, b) = (solo.ensure_word(Lang::Sq, "qen"), solo.ensure_word(Lang::Sq, "leh"));
        let e = Edge {
            from: a,
            to: b,
            rel: Rel::Sem("CS_CAPABLE_OF".into()),
            source: Source::ModelClaude,
            lang: Some(Lang::Sq),
            method: Method::ModelAuthored,
            attrs: vec![("witness".into(), "cs".into()), ("cid".into(), bark.clone())],
        };
        solo.add_edge(e.clone());
        assert!(!spoken(&solo, a, &e));
    }

    /// Lidhje sinonimie nga një fjalor (jo nga modeli).
    fn synonym(g: &mut Graph, a: &str, b: &str) {
        let (x, y) = (g.ensure_word(Lang::Sq, a), g.ensure_word(Lang::Sq, b));
        g.add_edge(Edge {
            from: x,
            to: y,
            rel: Rel::Sem("SYNONYM".into()),
            source: Source::Lexin,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![],
        });
    }

    /// Hartuesi mbi një graf të vogël: hyrjet e Wiktionary-t, indeksi i WordNet-it dhe shënimet e
    /// kuptimeve në korpus.
    fn small<'a>(g: &'a Graph, kaikki: &[&str], idx: &[(Pos, &str)], senses: &[&str]) -> Mapper<'a> {
        Mapper::new(g, &kaikki.join("\n"), idx, &senses.join("\n"))
    }

    fn ph(m: &mut Mapper, s: &str, slot: Slot) -> Result<String, Hold> {
        m.phrase(&toks(s), slot).map(|p| p.structured())
    }

    #[test]
    fn one_bridge_against_counted_wordnet_senses_needs_a_second_witness() {
        let mut g = Graph::new();
        lex(&mut g, "kotec", Pos::Noun);
        sense(&mut g, "00000051-n", Pos::Noun, &["stallë"]);
        lex(&mut g, "oborr", Pos::Noun);
        sense(&mut g, "00000053-n", Pos::Noun, &["vathë"]);
        lex(&mut g, "shperr", Pos::Verb);
        lex(&mut g, "fitoj", Pos::Verb);
        let kaikki = [
            "E\tkotec\tnoun\tm\t\t",
            "G\t1\t\tpen",
            "E\toborr\tnoun\tf\t\t",
            "G\t1\t\tyard, courtyard",
            "E\tshperr\tverb\t\t\t",
            "G\t1\t\tto win",
            "E\tfitoj\tverb\t\t\t",
            "G\t1\t\tto profit, earn",
            "G\t2\t\tto win",
        ];
        let idx = [(Pos::Noun, "pen n 2 0 2 2 00000050 00000051\nyard n 2 0 2 2 00000052 00000053")];
        let senses =
            ["pen%1:06:00:: 00000050 1 8", "pen%1:06:01:: 00000051 2 1", "yard%1:23:00:: 00000052 1 34", "yard%1:06:00:: 00000053 2 0"];
        let mut m = small(&g, &kaikki, &idx, &senses);
        // «pen»: WordNet-i ka vetëm «stallë», në një kuptim tjetër të shënuar në korpus: urat kundërshtohen.
        assert_eq!(m.translate("pen", Pos::Noun).unwrap_err(), Hold::Conflict);
        // «yard»: «vathë» është te një kuptim që s'është shënuar kurrë: s'kundërshton.
        assert_eq!(m.translate("yard", Pos::Noun).unwrap().code, MapCode::A1);
        // «win»: «fitoj» e ka «win» në krye të kuptimit të dytë: «shperr» vetëm s'mjafton.
        assert_eq!(m.translate("win", Pos::Verb).unwrap_err(), Hold::Ambiguous);
        // Me dëshmitarin e dytë të përkthimit (leksiku i modelit) përkthimi i vetëm mbahet, i dobët.
        let w = model_witness("lemma\tpos\tgender\tdefinition\tenglish\nkotec\tn\t\t\tpen\nshperr\tv\t\t\twin\n");
        let mut m = small(&g, &kaikki, &idx, &senses).with_witness(w);
        let t = m.translate("pen", Pos::Noun).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("kotec", MapCode::A1));
        assert_eq!(m.translate("win", Pos::Verb).unwrap().code, MapCode::A1);
    }

    #[test]
    fn agreement_on_a_minor_sense_holds_when_wiktionary_names_another_word() {
        let mut g = Graph::new();
        sense(&mut g, "00000061-n", Pos::Noun, &["shkronjë"]);
        lex(&mut g, "letër", Pos::Noun);
        let kaikki = [
            "E\tshkronjë\tnoun\tf\t\t",
            "G\t1\tfeminine\tletter",
            "E\tletër\tnoun\tf\t\t",
            "G\t1\tfeminine\tpaper",
            "G\t2\tfeminine\tfoil",
            "G\t3\tfeminine\tletter",
        ];
        let idx = [(Pos::Noun, "letter n 2 0 2 2 00000060 00000061")];
        let senses = ["letter%1:10:00:: 00000060 1 65", "letter%1:10:01:: 00000061 2 7"];
        // «shkronjë» pajtohet te shkronja e alfabetit; letra që shkruhet («letër») është kuptimi kryesor.
        assert_eq!(small(&g, &kaikki, &idx, &senses).translate("letter", Pos::Noun).unwrap_err(), Hold::Ambiguous);
    }

    #[test]
    fn synonym_bridge_keeps_the_direct_translation_when_the_wordnet_word_means_else() {
        let mut g = Graph::new();
        lex(&mut g, "shtyj", Pos::Verb);
        sense(&mut g, "00000070-v", Pos::Verb, &["detyroj"]);
        synonym(&mut g, "shtyj", "detyroj");
        lex(&mut g, "dru", Pos::Noun);
        sense(&mut g, "00000071-n", Pos::Noun, &["pemë"]);
        synonym(&mut g, "dru", "pemë");
        let kaikki = [
            "E\tshtyj\tverb\t\t\t",
            "G\t1\t\tto push",
            "E\tdetyroj\tverb\t\t\t",
            "G\t1\t\tto compel, to force",
            "E\tdru\tnoun\tm\t\t",
            "G\t1\t\ttree",
            "E\tpemë\tnoun\tf\t\t",
            "G\t1\t\tfruit tree",
        ];
        let idx = [(Pos::Verb, "push v 1 0 1 1 00000070"), (Pos::Noun, "tree n 1 0 1 1 00000071")];
        let mut m = small(&g, &kaikki, &idx, &[]);
        // «detyroj» është «compel»: mbetet përkthimi i drejtpërdrejtë «shtyj».
        let t = m.translate("push", Pos::Verb).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("shtyj", MapCode::AB2));
        // «pemë» e ka «tree» në glosë («fruit tree»): merret fjala e WordNet-it.
        let t = m.translate("tree", Pos::Noun).unwrap();
        assert_eq!((t.lemma.as_str(), t.code), ("pemë", MapCode::AB2));
    }

    #[test]
    fn gloss_parts_follow_wordnet_part_of_speech_and_spelling_keeps_the_part_of_speech() {
        let mut g = Graph::new();
        lex(&mut g, "lojë", Pos::Noun);
        rare(&mut g, "enjt", Pos::Noun);
        lex(&mut g, "ënjt", Pos::Verb);
        let kaikki = [
            "E\tlojë\tnoun\tf\t\t",
            "G\t1\tfeminine\tbeautiful, fair-skinned, plump woman",
            "E\tEnjt\tnoun\tm\t\t",
            "G\t1\tmasculine\tJupiter",
        ];
        let idx = [(Pos::Adj, "beautiful a 1 0 1 1 00000096")];
        let mut m = small(&g, &kaikki, &idx, &[]);
        // «beautiful» s'është emër në WordNet: glosa përshkruese e «lojë» s'e bën përkthim.
        assert_eq!(m.translate("beautiful", Pos::Noun).unwrap_err(), Hold::Unknown);
        // «enjt» (Jupiteri) s'shkruhet si folja «ënjt».
        assert_eq!(m.translate("jupiter", Pos::Noun).unwrap().lemma, "enjt");
    }

    #[test]
    fn light_verbs_phrasal_verbs_participles_and_activities_hold_instead_of_calques() {
        let mut g = Graph::new();
        sense(&mut g, "00000080-v", Pos::Verb, &["kam"]);
        sense(&mut g, "00000081-n", Pos::Noun, &["argëtim"]);
        sense(&mut g, "00000082-v", Pos::Verb, &["marr"]);
        sense(&mut g, "00000084-n", Pos::Noun, &["film"]);
        sense(&mut g, "00000085-v", Pos::Verb, &["ndëshkoj"]);
        sense(&mut g, "00000086-a", Pos::Adj, &["i lodhur"]);
        sense(&mut g, "00000087-n", Pos::Noun, &["qen"]);
        sense(&mut g, "00000090-n", Pos::Noun, &["tym"]);
        sense(&mut g, "00000092-n", Pos::Noun, &["zjarr"]);
        let kaikki = [
            "E\tkam\tverb\t\t\t",
            "G\t1\t\tto have",
            "E\targëtim\tnoun\tm\t\t",
            "G\t1\t\tfun",
            "E\tmarr\tverb\t\t\t",
            "G\t1\t\tto take",
            "E\tfilm\tnoun\tm\t\t",
            "G\t1\t\tfilm",
            "E\tndëshkoj\tverb\t\t\t",
            "G\t1\t\tto punish",
            "E\tlodhur\tadj\t\ti lodhur\t",
            "G\t1\t\ttired",
            "E\tqen\tnoun\tm\t\t",
            "G\t1\t\tdog",
            "E\ttym\tnoun\tm\t\t",
            "G\t1\t\tsmoke",
            "E\tzjarr\tnoun\tm\t\t",
            "G\t1\t\tfire",
        ];
        let nouns = [
            "fun n 1 0 1 1 00000081",
            "film n 1 0 1 1 00000084",
            "dog n 1 0 1 1 00000087",
            "smoke n 1 0 1 1 00000090",
            "fire n 1 0 1 1 00000092",
            "light n 1 0 1 1 00000093",
        ]
        .join("\n");
        let verbs = [
            "have v 1 0 1 1 00000080",
            "take v 1 0 1 1 00000082",
            "take_in v 1 0 1 1 00000083",
            "punish v 1 0 1 1 00000085",
            "tire v 1 0 1 1 00000088",
            "smoke v 1 0 1 1 00000091",
            "light v 1 0 1 1 00000094",
        ]
        .join("\n");
        let idx = [(Pos::Noun, nouns.as_str()), (Pos::Verb, verbs.as_str()), (Pos::Adj, "tired a 1 0 1 1 00000086\nlight a 1 0 1 1 00000095")];
        let senses = ["fun%1:04:00:: 00000081 1 9", "light%1:19:00:: 00000093 1 200", "light%2:30:00:: 00000094 1 10"];
        let mut m = small(&g, &kaikki, &idx, &senses);
        // Folja e lehtë me emër abstrakt: «have fun» s'është «kam argëtim»; me send konkret po.
        assert_eq!(ph(&mut m, "have_fun", Slot::Act).unwrap_err(), Hold::Ambiguous);
        assert_eq!(ph(&mut m, "have_dog", Slot::Act).unwrap(), "v=kam;o=qen");
        // Folja frazore me pjesëzën në mes: «take in film» s'është «marr në film».
        assert_eq!(ph(&mut m, "take_in_film", Slot::Act).unwrap_err(), Hold::Ambiguous);
        // Pjesorja s'bëhet folje veprore; mbiemri i WordNet-it jep kopulën.
        assert_eq!(ph(&mut m, "punished", Slot::Event).unwrap_err(), Hold::Shape);
        assert_eq!(ph(&mut m, "tired", Slot::Event).unwrap(), "cop=lodhur");
        // «light fire»: fjala e parë është folje, s'lexohet si mbiemër i emrit.
        assert_eq!(ph(&mut m, "light_fire", Slot::Act).unwrap_err(), Hold::Ambiguous);
        // Veprimtaria lexohet vetëm si folje: «smoke» pa folje shqipe mbetet jashtë, s'bëhet «tym».
        assert!(ph(&mut m, "smoke", Slot::Activity).is_err());
        assert_eq!(ph(&mut m, "smoke", Slot::Thing).unwrap(), "n=tym");
    }

    #[test]
    fn equal_witnesses_hold_instead_of_an_alphabetical_choice() {
        let mut g = Graph::new();
        sense(&mut g, "00000097-n", Pos::Noun, &["zvarranik", "rrëshqanor"]);
        let kaikki = ["E\tzvarranik\tnoun\tm\t\t", "G\t1\t\treptile", "E\trrëshqanor\tnoun\tm\t\t", "G\t1\t\treptile"];
        let idx = [(Pos::Noun, "reptile n 1 0 1 1 00000097")];
        assert_eq!(small(&g, &kaikki, &idx, &[]).translate("reptile", Pos::Noun).unwrap_err(), Hold::Ambiguous);
        // Fjala që e dëshmon edhe një fjalor tjetër zgjidhet.
        rare(&mut g, "zvarranik", Pos::Noun);
        assert_eq!(small(&g, &kaikki, &idx, &[]).translate("reptile", Pos::Noun).unwrap().lemma, "zvarranik");
    }
}
