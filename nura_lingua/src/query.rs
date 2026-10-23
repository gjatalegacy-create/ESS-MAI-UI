//! Kolapsi i fjalës: hyrja → i₀ → Xi (forma/përdorimi) + Yi (kuptimet) →
//! eliminim brenda secilës hapësirë → verifikim kundrejt burimit → plan shprehjeje.
//!
//! Lidhja me formulën e ESS-MAI (GCL, `GCL_NURALOGIC_KANUNI.md`) është
//! PROJEKTIM I RI i frymëzuar nga ajo, jo kod i trashëguar:
//! - i₀ = forma që shkroi përdoruesi; ende nuk dihet cili kuptim është;
//! - Xi = dëshmitë e formës (përdorime UD, marrëdhënie forme në ConceptNet);
//! - Yi = kuptimet kandidate (Albanet → WordNet);
//! - −Yi = eliminimi i kuptimeve brenda hapësirës së tyre me rregulla R-E*;
//! - Y verifikohet i pari (kuptimi/marrëdhënia në burim), X trashëgon besimin.
//! Mungesa në burim NUK është mohim; paqartësia jep HOLD + pyetje (iZ).
//!
//! Gramatika e mbështetur (fushë e kufizuar e shqipes):
//!   <fjalë> [fjalë konteksti…]            eksplorim
//!   ç'është <fjalë> | çfarë është <fjalë>  kuptimi
//!   a është <X> <Y>?                        pyetje "lloj i"
//!   <X> është <Y> | <X> nuk është <Y>      pohim (me polaritet)
//!   nëse <X> është <Y>, (atëherë) <X'> është <Y'>   kusht
//!   e kundërta e <fjalë> | <fjalë> e kundërt  antonimi

use crate::emotion::Event;
use crate::discourse;
use crate::expression::*;
use crate::graph::*;
use crate::morph::Morph;
use crate::MASS_SCALE;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

/// Fjalë funksionale shqipe — lista vjen nga `light/src/lingua.rs` i ESS-MAI
/// (Stopwords FULL, SQ), e kopjuar pa ndryshim; plus pjesëzat e gramatikës sonë.
pub const SQ_STOPWORDS: &[&str] = &[
    "dhe", "ose", "për", "nga", "një", "por", "është", "janë", "jam", "jemi", "jeni", "kjo", "ky",
    "kët", "ajo", "ata", "ato", "atë", "këto", "këta", "duke", "pas", "para", "mbi", "nën", "tek",
    "deri", "që", "kur", "pse", "çfarë", "cila", "cili", "cilët", "nuk", "mos", "edhe", "apo",
    "ndaj", "pra", "vetëm", "shumë", "pak", "gjithë", "disa", "tij", "saj", "tyre", "ynë", "jonë",
    "ime", "tim", "tënd", "tuaj", "kanë", "kishte", "ishte", "qenë", "bëhet", "bëri", "gjatë",
    "sipas", "brenda", "jashtë", "midis", "ndër", "tjetër", "tjera", "gjithashtu", "kështu",
    "ashtu", "aty", "këtu",
    // pjesëza të gramatikës së kufizuar (të shtuara këtu)
    "a", "në", "e", "i", "të", "me", "si", "nëse", "atëherë", "ç'është", "çka",
    // fjalë bisede që nuk janë objekt i pyetjes ("më fol për…", "çfarë do të thotë…")
    "fol", "trego", "thuaj", "shpjego", "lutem", "dua", "di", "kuptimi", "kuptim", "thotë", "do",
    "mund", "më", "diçka", "rreth", "ka", "quhet", "thuhet", "përkthehet", "mes", "çdo", "pak",
];

/// Fjalët-shenjë të pyetjeve me faset dhe krahasim (rregulla të deklaruara).
fn cue_form(v: &[&str]) -> Option<Form> {
    let has = |cues: &[&str]| v.iter().any(|w| cues.contains(w));
    let cues_all: &[&str] = &[
        "dallim", "dallimi", "ç'dallim", "ndryshim", "ndryshimi", "krahaso", "krahasim", "dallon", "ndryshon",
        "pjesë", "pjesët", "pjesësh", "përbëhet", "përbërës", "lloje", "llojet", "llojesh", "sinonim",
        "sinonimi", "sinonime", "sinonimet", "anglisht", "anglishte", "anglishten", "kategori", "kategoria",
        "prejardhja", "prejardhje", "prejardhjen", "rrjedh", "formuar", "shqip", "shqipe", "shqipja",
    ];
    let mut content: Vec<String> = v
        .iter()
        .filter(|w| !is_stop(w) && !cues_all.contains(w) && **w != ",")
        .map(|w| w.to_string())
        .collect();
    // Mbiemër pas nyjës, edhe kur fjala vetë është në listën e fjalëve funksionale
    // («sinonime të gjatë»: «gjatë» si mbiemër, jo si parafjalë).
    match (content.is_empty(), after_article(v)) {
        (true, Some(x)) => content.push(x),
        _ => {}
    }
    let compare = has(&["dallim", "dallimi", "ç'dallim", "ndryshim", "ndryshimi", "krahaso", "krahasim", "dallon", "ndryshon"]);
    let facet = match (
        has(&["prejardhja", "prejardhje", "prejardhjen", "rrjedh", "formuar"]),
        has(&["shqip", "shqipe", "shqipja"]),
        has(&["pjesë", "pjesët", "pjesësh", "përbëhet", "përbërës"]),
        has(&["lloje", "llojet", "llojesh"]),
        has(&["sinonim", "sinonimi", "sinonime", "sinonimet"]),
        has(&["anglisht", "anglishte", "anglishten"]),
        has(&["kategori", "kategoria"]),
    ) {
        (true, _, _, _, _, _, _) => Some(Facet::Origin),
        (_, true, _, _, _, _, _) => Some(Facet::Native),
        (_, _, true, _, _, _, _) => Some(Facet::Parts),
        (_, _, _, true, _, _, _) => Some(Facet::Kinds),
        (_, _, _, _, true, _, _) => Some(Facet::Synonyms),
        (_, _, _, _, _, true, _) => Some(Facet::TranslationEn),
        (_, _, _, _, _, _, true) => Some(Facet::Category),
        _ => None,
    };
    match (compare, facet, content.as_slice()) {
        (true, _, [a, b, ..]) => Some(Form::Compare { a: a.clone(), b: b.clone() }),
        (false, Some(f), [.., x]) => Some(Form::FacetQ { focus: x.clone(), facet: f }),
        _ => None,
    }
}

/// Teksti i një opsioni për pyetjen «cilin kuptim?» (butonat e ndërfaqes): përkufizimi shqip i
/// pastruar (kuptimi i parë, pa kryefjalën e përsëritur); ndryshe një etiketë tjetër shqipe;
/// ndryshe etiketa angleze, e shënuar si e tillë.
pub fn option_label(s: &SenseCand, lemma: &str) -> String {
    let head = s.labels_sq.first().cloned().unwrap_or_else(|| lemma.to_string());
    let gloss = s
        .gloss_sq
        .as_ref()
        .map(|g| crate::expression::strip_subject(&head, g.split(';').next().unwrap_or("").trim()));
    match (gloss, Engine::alt_label(s, lemma)) {
        (Some(g), _) => g.trim().trim_end_matches('.').to_string(),
        (None, Some(l)) => format!("si «{l}»"),
        (None, None) => format!("{} (në anglisht)", s.labels_en.first().cloned().unwrap_or_default().replace('_', " ")),
    }
}

fn is_stop(w: &str) -> bool {
    SQ_STOPWORDS.contains(&w)
}

/// Fjala pas nyjës së fundit të mbiemrit («i», «e», «të»), bashkë me nyjën: «i gjatë».
/// Nyja ruhet sepse tregon mbiemër («i zi» = ngjyrë, jo «zi» = zi i dhimbjes).
fn after_article(v: &[&str]) -> Option<String> {
    v.windows(2)
        .filter(|w| match (w[0], w[1]) {
            ("i" | "e" | "të", "fjalës" | "fjala" | "një" | "kundërta" | "kundërtën" | ",") => false,
            ("i" | "e" | "të", _) => true,
            _ => false,
        })
        .last()
        .map(|w| format!("{} {}", w[0], w[1]))
}

/// Hyrja nis me nyjën e mbiemrit («i bardhë», «e bukur»).
pub fn typed_article(input: &str) -> bool {
    let low = input.trim().to_lowercase();
    let toks: Vec<&str> = low.split_whitespace().collect();
    matches!(toks.as_slice(), ["i" | "e" | "të", _, ..])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Simple {
    pub subj: String,
    pub obj: String,
    pub polarity: Polarity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form {
    Explore { focus: String, context: Vec<String> },
    Meaning { focus: String, context: Vec<String> },
    IsA { claim: Simple, asked: bool },
    Opposite { focus: String },
    Conditional { cond: Simple, cons: Simple },
    /// "pse X është Y?" — shpjegim me zinxhir kategorish
    Why { claim: Simple },
    /// "ç'dallim ka X nga Y?" / "krahaso X me Y"
    Compare { a: String, b: String },
    /// "çfarë pjesësh ka X?", "llojet e X", "sinonim i X", "si quhet X në anglisht?"
    FacetQ { focus: String, facet: Facet },
    /// "fjali qen kafshë": Nura ndërton vetë fjalinë nga fjalët, sipas lidhjeve dhe përputhjeve.
    Compose { words: Vec<String> },
    /// Biseda: fjalët lidhen sipas kuptimit (portret, lidhje, shkak, vend, përdorim, veprim).
    Converse { intent: crate::biseda::Intent, words: Vec<String> },
}

impl Form {
    pub fn code(&self) -> &'static str {
        match self {
            Form::Explore { .. } => "EXPLORE",
            Form::Meaning { .. } => "MEANING",
            Form::IsA { asked: true, .. } => "ISA_QUESTION",
            Form::IsA { asked: false, .. } => "ISA_CLAIM",
            Form::Opposite { .. } => "OPPOSITE",
            Form::Conditional { .. } => "CONDITIONAL",
            Form::Why { .. } => "WHY",
            Form::Compare { .. } => "COMPARE",
            Form::FacetQ { .. } => "FACET",
            Form::Compose { .. } => "COMPOSE",
            Form::Converse { .. } => "CONVERSE",
        }
    }
}

pub fn tokenize(input: &str) -> Vec<String> {
    let lower = input.to_lowercase().replace('’', "'").replace(',', " , ");
    lower
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| matches!(c, '?' | '.' | '!' | ':' | ';' | '"' | '«' | '»' | '(' | ')'))
                .to_string()
        })
        .filter(|t| !t.is_empty())
        .collect()
}

fn join_obj(rest: &[String]) -> Option<String> {
    let words: Vec<&str> = rest
        .iter()
        .map(|s| s.as_str())
        .filter(|w| !matches!(*w, "një" | "lloj" | ","))
        .collect();
    match words.is_empty() {
        true => None,
        false => Some(words.join(" ")),
    }
}

fn parse_simple(t: &[String]) -> Option<Simple> {
    let v: Vec<&str> = t.iter().map(|s| s.as_str()).collect();
    match v.as_slice() {
        ["a", "nuk", "është", x, rest @ ..] | [x, "nuk", "është", rest @ ..] => Some(Simple {
            subj: x.to_string(),
            obj: join_obj(&rest.iter().map(|s| s.to_string()).collect::<Vec<_>>())?,
            polarity: Polarity::Neg,
        }),
        ["a", "është", x, rest @ ..] | [x, "është", rest @ ..] => Some(Simple {
            subj: x.to_string(),
            obj: join_obj(&rest.iter().map(|s| s.to_string()).collect::<Vec<_>>())?,
            polarity: Polarity::Pos,
        }),
        _ => None,
    }
}

pub fn parse(input: &str) -> Result<Form, String> {
    let t = tokenize(input);
    let v: Vec<&str> = t.iter().map(|s| s.as_str()).collect();
    let content = |xs: &[&str]| -> Vec<String> {
        xs.iter().filter(|w| !is_stop(w) && **w != ",").map(|w| w.to_string()).collect()
    };
    // "pse X është Y?" dhe pyetjet me fjalë-shenjë kontrollohen para ndërtimeve të tjera.
    match v.as_slice() {
        ["pse", ..] => {
            return parse_simple(&t[1..])
                .map(|claim| Form::Why { claim })
                .ok_or_else(|| "pyetja «pse» duhet të jetë «pse X është Y?»".to_string())
        }
        ["fjali" | "fjalia" | "fjali:" | "ndërto" | "krijo", rest @ ..] => {
            // Nyja e mbiemrit mbetet me fjalën e vet: «i madh» është një fjalë.
            let mut words: Vec<String> = Vec::new();
            let mut pending: Option<&str> = None;
            for w in rest.iter().filter(|w| !matches!(**w, "," | "fjali" | "fjalinë" | "me" | "nga")) {
                match (pending.take(), matches!(*w, "i" | "e" | "të")) {
                    (None, true) => pending = Some(w),
                    (None, false) => words.push(w.to_string()),
                    (Some(a), _) => words.push(format!("{a} {w}")),
                }
            }
            return match words.len() {
                2 | 3 => Ok(Form::Compose { words }),
                _ => Err("fjalia ndërtohet nga dy ose tri fjalë: «fjali qen kafshë»".into()),
            };
        }
        _ => {}
    }
    match v.as_slice() {
        [.., "nga", "vjen", "fjala", a @ ("i" | "e" | "të"), x] | [.., "si", "është", "formuar", "fjala", a @ ("i" | "e" | "të"), x] => {
            return Ok(Form::FacetQ { focus: format!("{a} {x}"), facet: Facet::Origin })
        }
        [.., "nga", "vjen", "fjala", x] | [.., "si", "është", "formuar", "fjala", x] => {
            return Ok(Form::FacetQ { focus: x.to_string(), facet: Facet::Origin })
        }
        _ => {}
    }
    match cue_form(&v) {
        Some(f) => return Ok(f),
        None => {}
    }
    match v.as_slice() {
        [] => Err("hyrja është bosh".into()),
        ["nëse", ..] => {
            let rest = &t[1..];
            let split = rest
                .iter()
                .position(|w| w == "atëherë")
                .or_else(|| rest.iter().position(|w| w == ","))
                .ok_or("kushti kërkon «atëherë» ose presje")?;
            let cond_t: Vec<String> = rest[..split].iter().filter(|w| *w != ",").cloned().collect();
            let cons_t: Vec<String> =
                rest[split + 1..].iter().filter(|w| *w != "," && *w != "atëherë").cloned().collect();
            match (parse_simple(&cond_t), parse_simple(&cons_t)) {
                (Some(cond), Some(cons)) => Ok(Form::Conditional { cond, cons }),
                _ => Err("kushti duhet të jetë «nëse X është Y, atëherë Z është W»".into()),
            }
        }
        // Me nyjë: «ç'është i bardhë?» — nyja tregon mbiemrin, nuk është vetë fjala.
        ["ç'është", a @ ("i" | "e" | "të"), x, rest @ ..]
        | ["çfarë" | "çka", "është", a @ ("i" | "e" | "të"), x, rest @ ..]
        | ["ç'kuptim", "ka", a @ ("i" | "e" | "të"), x, rest @ ..] => {
            Ok(Form::Meaning { focus: format!("{a} {x}"), context: content(rest) })
        }
        ["ç'është", x, rest @ ..] | ["çfarë", "është", x, rest @ ..] | ["çka", "është", x, rest @ ..]
        | ["ç'kuptim", "ka", x, rest @ ..] => Ok(Form::Meaning { focus: x.to_string(), context: content(rest) }),
        [.., "e", "kundërta", "e", x] | [.., "të", "kundërtën", "e", x] => Ok(Form::Opposite { focus: x.to_string() }),
        [.., "e", "kundërta", "e", "fjalës", a @ ("i" | "e" | "të"), x]
        | [.., "të", "kundërtën", "e", "fjalës", a @ ("i" | "e" | "të"), x]
        | [.., "e", "kundërta", "e", a @ ("i" | "e" | "të"), x]
        | [.., "të", "kundërtën", "e", a @ ("i" | "e" | "të"), x] => Ok(Form::Opposite { focus: format!("{a} {x}") }),
        [.., "e", "kundërta", "e", "fjalës", x] | [.., "të", "kundërtën", "e", "fjalës", x] => {
            Ok(Form::Opposite { focus: x.to_string() })
        }
        [x, "e", "kundërt", ..] | ["kundër", x, ..] => Ok(Form::Opposite { focus: x.to_string() }),
        ["a", ..] => parse_simple(&t)
            .map(|claim| Form::IsA { claim, asked: true })
            .ok_or_else(|| "pyetja duhet të jetë «a është X Y?»".to_string()),
        _ => match parse_simple(&t) {
            Some(claim) => Ok(Form::IsA { claim, asked: false }),
            None => {
                let words = content(&v);
                // «i bukur», «e kuqe …»: nyja dhe fjala bashkë janë fokusi.
                match (v.as_slice(), words.split_first(), after_article(&v)) {
                    ([a @ ("i" | "e" | "të"), x, rest @ ..], _, _) => {
                        Ok(Form::Explore { focus: format!("{a} {x}"), context: content(rest) })
                    }
                    (_, Some((f, ctx)), _) => Ok(Form::Explore { focus: f.clone(), context: ctx.to_vec() }),
                    (_, None, Some(x)) => Ok(Form::Explore { focus: x, context: vec![] }),
                    // Një fjalë e vetme pyetet si fjalë, edhe kur është në listën e fjalëve
                    // funksionale («para» = të holla, «mes», «ka» = kau).
                    ([x], None, None) => Ok(Form::Explore { focus: x.to_string(), context: vec![] }),
                    (_, None, None) => Err("nuk ka fjalë përmbajtjeje në hyrje".into()),
                }
            }
        },
    }
}

// ───────────────────────────── analiza ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandStatus {
    Open,
    Eliminated(String),
    Selected(String),
}

#[derive(Debug, Clone)]
pub struct SenseCand {
    pub sense: u32,
    pub synset: String,
    pub pos: Pos,
    pub via: Source,
    pub labels_sq: Vec<String>,
    pub labels_en: Vec<String>,
    pub gloss_sq: Option<String>,
    pub gloss_en: Option<String>,
    pub score: u32,
    pub status: CandStatus,
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub sentence: u32,
    pub edge: u32,
    pub sent_id: String,
    pub source: Source,
    pub form: String,
    pub feats: String,
    pub deprel: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct FormRel {
    pub concept: u32,
    pub edge: u32,
    pub rel: String,
    /// true: koncepti i fjalës është fillimi i skajit (fjala → tjetri).
    pub outgoing: bool,
    pub other_concept: u32,
    pub other_term: String,
    pub other_lang: Option<Lang>,
    /// Kodi i gjuhës nga URI-ja e ConceptNet (p.sh. "sq", "en", "de").
    pub other_lang_code: String,
    pub dataset: String,
    pub license: String,
}

/// Rendi i marrëdhënieve ConceptNet për paraqitje (më informueset para).
pub fn cn_rank(rel: &str) -> u8 {
    match rel {
        "/r/Antonym" => 0,
        "/r/IsA" => 1,
        "/r/Synonym" => 2,
        "/r/FormOf" => 3,
        "/r/PartOf" | "/r/SimilarTo" | "/r/DistinctFrom" => 4,
        "/r/DerivedFrom" => 5,
        "/r/RelatedTo" => 6,
        "/r/EtymologicallyDerivedFrom" | "/r/EtymologicallyRelatedTo" => 7,
        _ => 8,
    }
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub input: String,
    pub lemma: String,
    pub word: Option<u32>,
    pub how: String,
}

#[derive(Debug, Clone)]
pub struct Focus {
    pub resolved: Resolved,
    pub senses: Vec<SenseCand>,
    pub usages: Vec<Usage>,
    pub form_rels: Vec<FormRel>,
    pub sources_hit: Vec<Source>,
}

#[derive(Debug, Clone)]
pub struct PathStep {
    pub node: u32,
    pub edge: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ProcessStep {
    pub code: &'static str,
    pub label: String,
    pub nodes: Vec<u32>,
    pub edges: Vec<u32>,
    pub event: Option<Event>,
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub input: String,
    pub form: Form,
    pub focus: Option<Focus>,
    pub other: Option<Focus>,
    pub path: Vec<PathStep>,
    pub cond_path: Vec<PathStep>,
    pub bridge_path: Vec<PathStep>,
    pub plan: ExprPlan,
    pub steps: Vec<ProcessStep>,
}

pub struct Engine<'a> {
    pub g: &'a Graph,
    pub morph: &'a Morph,
    pub lex: &'a HashMap<String, u32>,
    /// A është ConceptNet importuar vetëm pjesërisht (sipas SOURCES.lock)?
    /// Parazgjedhja konservative është `true`.
    pub conceptnet_partial: bool,
    /// Leksiku gramatikor, kur është ngarkuar: trajtat që nuk janë të atestuara i jep gramatika.
    pub gram: Option<&'a crate::gramatika::leksiku::Lexicon>,
}

const HYPER: [&str; 2] = ["@", "@i"];

/// Skaj "lloj i": hiperonim WordNet (@, @i) ose BabelNet (HYPERNYM).
fn is_hyper(rel: &Rel) -> bool {
    match rel {
        Rel::Wn(s) => HYPER.contains(&s.as_str()),
        Rel::Bn(r) => r == "HYPERNYM",
        _ => false,
    }
}

/// Burimet që dëshmojnë një kuptim: AlbaNet kalon përmes WordNet; BabelNet është vetvetiu.
fn sense_sources(via: Source) -> Vec<Source> {
    match via {
        Source::BabelNet53 => vec![Source::BabelNet53],
        Source::ModelClaude => vec![Source::ModelClaude],
        other => vec![other, Source::Pwn30],
    }
}
/// Burimet që përmbajnë fjalë shqipe (WordNet 3.0 është vetëm anglisht).
pub const SQ_SOURCES: [Source; 5] = [Source::OmwAls, Source::BabelNet53, Source::ConceptNet57, Source::UdSqTsa, Source::UdSqStaf];
const MAX_SENSES: usize = 16;

impl<'a> Engine<'a> {
    pub fn new(g: &'a Graph, morph: &'a Morph, lex: &'a HashMap<String, u32>) -> Self {
        Self { g, morph, lex, conceptnet_partial: true, gram: None }
    }

    pub fn with_grammar(mut self, lx: &'a crate::gramatika::leksiku::Lexicon) -> Self {
        self.gram = Some(lx);
        self
    }

    pub fn with_conceptnet_partial(mut self, partial: bool) -> Self {
        self.conceptnet_partial = partial;
        self
    }

    /// Forma e shtypur → lemë që ekziston në graf (pa shpikur ekzistencë). Rendi:
    /// (1) forma e shtypur ose ajo me nyjë («pasur» → «i pasur»), kur ka kuptim shqip;
    /// (2) forma e shtypur si fjalë fjalori (me gramatikë ose kuptim);
    /// (3) lema e trajtës së lakuar (UD, Wiktionary, rregullat) me kuptim shqip;
    /// (4) çdo lemë me kuptim; (5) çdo lemë në leksikon.
    /// Kur shtypet nyja («i zi»), pranohen vetëm fjalët me nyjë ose mbiemrat.
    pub fn resolve(&self, input: &str) -> Resolved {
        // Theksi i fjalorëve («ullí») nuk është pjesë e drejtshkrimit.
        let low: String = input
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| *c != '\u{301}')
            .map(|c| match c {
                'á' => 'a',
                'é' => 'e',
                'í' => 'i',
                'ó' => 'o',
                'ú' => 'u',
                'ý' => 'y',
                other => other,
            })
            .collect();
        let (article, bare) = match low.split_once(' ') {
            Some((a @ ("i" | "e" | "të"), rest)) => (Some(a.to_string()), rest.trim().to_string()),
            _ => (None, low.clone()),
        };
        let mut cands = self.morph.lemma_candidates(&bare);
        // Trajtat e lakuara sipas Wiktionary-t («kompjuterit» → «kompjuter»).
        let form_of: Vec<(String, &'static str)> = self
            .lex
            .get(&bare)
            .map(|&w| {
                self.g
                    .out_edges(w)
                    .filter(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()))
                    .filter_map(|(_, e)| match &self.g.node(e.to).kind {
                        NodeKind::Word { lemma, .. } => Some((lemma.clone(), "trajtë e lakuar sipas Wiktionary")),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        cands.extend(form_of);
        let with_article: Vec<(String, &'static str)> = cands
            .iter()
            .flat_map(|(l, _)| [(format!("i {l}"), "mbiemër me nyje"), (format!("e {l}"), "mbiemër me nyje")])
            .collect();
        cands.extend(with_article);
        // Pa nyjë të shtypur, «e X» është zakonisht fjalë tjetër (emri «e mira» ≠ «mirë»):
        // provohen vetëm forma e shtypur dhe ajo me «i»; «e X» mbetet te kandidatët e fundit.
        // Me nyjë të shtypur provohen vetëm forma e shtypur dhe fjala pa nyjë («i mirë» ≠ emri
        // «e mira»).
        let exact: Vec<String> = match &article {
            Some(a) => vec![format!("{a} {bare}"), bare.clone()],
            None => vec![bare.clone(), format!("i {bare}")],
        };
        // Trajta e lakuar sipas Wiktionary-t (shumës, e shquar …, jo pjesore) shkon te lema e saj
        // kur ajo ka kuptim ose gramatikë: «miq» → «mik», «pyje» → «pyll», «kuaj» → «kalë».
        let inflected_of: Option<String> = self.lex.get(&bare).and_then(|&w| {
            self.g
                .out_edges(w)
                .filter(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()) && !e.attr("tags").unwrap_or("").contains("participle"))
                // Shumësi shkon te lema, përveç kur forma është vetë fjalë e leksikut gramatikor
                // («ligj» = ligj, jo shumësi «të ligj» i «i lig»); trajtat e tjera vetëm kur forma vetë
                // nuk është fjalë fjalori («ka» = kau, jo vetëm «ai ka»).
                .filter(|(_, e)| match e.attr("tags").unwrap_or("").contains("plural") {
                    true => !self.g.out_edges(w).any(|(_, x)| x.rel == Rel::Lex && x.source == Source::Uniparser),
                    false => !self.g.out_edges(w).any(|(_, x)| x.rel == Rel::Lex),
                })
                .filter(|(_, e)| !self.g.senses_of(e.to).is_empty() || self.g.out_edges(e.to).any(|(_, x)| x.rel == Rel::Lex))
                .find_map(|(_, e)| match &self.g.node(e.to).kind {
                    NodeKind::Word { lemma, .. } => Some(lemma.clone()),
                    _ => None,
                })
        });
        let sq_sense = |l: &str| {
            self.lex.get(l).map(|&w| self.g.senses_of(w).iter().any(|(_, e)| e.source != Source::KaikkiEn)).unwrap_or(false)
        };
        let any_sense = |l: &str| self.lex.get(l).map(|&w| !self.g.senses_of(w).is_empty()).unwrap_or(false);
        let regional = Rel::Sem("REGIONAL_GLOSS".into());
        let headword = |l: &str| {
            self.lex
                .get(l)
                .map(|&w| {
                    self.g.out_edges(w).any(|(_, e)| e.rel == Rel::Lex || e.rel == regional) || !self.g.senses_of(w).is_empty()
                })
                .unwrap_or(false)
        };
        let adj_ok = |l: &str| match &article {
            None => true,
            Some(_) => {
                matches!(l.split_whitespace().next(), Some("i" | "e" | "të"))
                    || self
                        .lex
                        .get(l)
                        .map(|&w| discourse::independent_pos(self, w).contains(&Pos::Adj))
                        .unwrap_or(false)
            }
        };
        let pick: Option<(String, String)> = inflected_of
            .map(|l| (l, "trajtë e lakuar sipas Wiktionary".to_string()))
            .filter(|_| article.is_none())
            .or_else(|| exact.iter().find(|l| sq_sense(l) && adj_ok(l)).map(|l| (l.clone(), "forma e shtypur".to_string())))
            .map(|x| x);
        let pick: Option<(String, String)> = pick
            .or_else(|| exact.iter().find(|l| headword(l) && adj_ok(l)).map(|l| (l.clone(), "forma e shtypur".to_string())))
            .or_else(|| cands.iter().find(|(l, _)| sq_sense(l) && adj_ok(l)).map(|(l, h)| (l.clone(), h.to_string())))
            .or_else(|| cands.iter().find(|(l, _)| any_sense(l)).map(|(l, h)| (l.clone(), h.to_string())))
            .or_else(|| cands.iter().find(|(l, _)| self.lex.contains_key(l)).map(|(l, h)| (l.clone(), h.to_string())));
        match pick {
            Some((l, how)) => Resolved { input: input.to_string(), word: self.lex.get(&l).copied(), lemma: l, how },
            None => Resolved { input: input.to_string(), lemma: low, word: None, how: "nuk u gjet në leksikon".into() },
        }
    }

    pub fn focus(&self, input: &str) -> Focus {
        let r = self.resolve(input);
        let mut senses = Vec::new();
        let mut usages = Vec::new();
        let mut form_rels = Vec::new();
        match r.word {
            Some(w) => {
                for (sid, e) in self.g.senses_of(w) {
                    let n = self.g.node(sid);
                    let (synset, pos) = match &n.kind {
                        NodeKind::Sense { synset, pos, .. } => (synset.clone(), *pos),
                        _ => continue,
                    };
                    senses.push(SenseCand {
                        sense: sid,
                        synset,
                        pos,
                        via: e.source,
                        labels_sq: n.labels.iter().filter(|t| t.lang == Lang::Sq).map(|t| t.text.clone()).collect(),
                        labels_en: n.labels.iter().filter(|t| t.lang == Lang::En).map(|t| t.text.clone()).collect(),
                        // Një «përkufizim» vetëm me fjalë funksionale (p.sh. «një») nuk është përkufizim.
                        gloss_sq: self
                            .g
                            .gloss_in(sid, Lang::Sq)
                            .map(|t| t.text.clone())
                            .filter(|t| tokenize(t).iter().any(|w| !is_stop(w) && w.chars().count() >= 3)),
                        gloss_en: self.g.gloss_in(sid, Lang::En).map(|t| t.text.clone()),
                        score: 0,
                        status: CandStatus::Open,
                    });
                }
                senses.sort_by(|a, b| a.synset.cmp(&b.synset));
                senses.truncate(MAX_SENSES);
                for (eid, e) in self.g.out_edges(w) {
                    match e.rel {
                        Rel::OccursIn => {
                            let s = e.to;
                            let (sent_id, source) = match &self.g.node(s).kind {
                                NodeKind::Sentence { sent_id, source } => (sent_id.clone(), *source),
                                _ => continue,
                            };
                            usages.push(Usage {
                                sentence: s,
                                edge: eid,
                                sent_id,
                                source,
                                form: e.attr("form").unwrap_or("").to_string(),
                                feats: e.attr("feats").unwrap_or("").to_string(),
                                deprel: e.attr("deprel").unwrap_or("").to_string(),
                                text: self.g.gloss_in(s, Lang::Sq).map(|t| t.text.clone()).unwrap_or_default(),
                            });
                        }
                        _ => {}
                    }
                }
                for (_, fe) in self.g.in_edges(w).filter(|(_, e)| e.rel == Rel::FormOf) {
                    let c = fe.from;
                    let both = self
                        .g
                        .out_edges(c)
                        .map(|(id, e)| (id, e, e.to, true))
                        .chain(self.g.in_edges(c).map(|(id, e)| (id, e, e.from, false)));
                    for (eid, e, other, outgoing) in both {
                        match &e.rel {
                            Rel::Cn(rel) => {
                                let (term, lang, code) = match &self.g.node(other).kind {
                                    NodeKind::Concept { uri } => (
                                        concept_term(uri),
                                        uri.split('/').nth(2).and_then(Lang::parse),
                                        uri.split('/').nth(2).unwrap_or("und").to_string(),
                                    ),
                                    _ => continue,
                                };
                                form_rels.push(FormRel {
                                    concept: c,
                                    edge: eid,
                                    rel: rel.clone(),
                                    outgoing,
                                    other_concept: other,
                                    other_term: term,
                                    other_lang: lang,
                                    other_lang_code: code,
                                    dataset: e.attr("dataset").unwrap_or("").to_string(),
                                    license: e.attr("license").unwrap_or("").to_string(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                // Renditja për ekran: marrëdhëniet më informuese dhe ato shqip para.
                form_rels.sort_by(|a, b| {
                    (cn_rank(&a.rel), a.other_lang != Some(Lang::Sq), &a.other_term)
                        .cmp(&(cn_rank(&b.rel), b.other_lang != Some(Lang::Sq), &b.other_term))
                });
            }
            None => {}
        }
        let mut sources_hit: Vec<Source> = senses.iter().map(|s| s.via).collect();
        match senses.is_empty() {
            true => {}
            false => sources_hit.push(Source::Pwn30),
        }
        sources_hit.extend(usages.iter().map(|u| u.source));
        match form_rels.is_empty() {
            true => {}
            false => sources_hit.push(Source::ConceptNet57),
        }
        sources_hit.sort();
        sources_hit.dedup();
        Focus { resolved: r, senses, usages, form_rels, sources_hit }
    }

    /// Nënshkrimi shqip i një kuptimi: përkufizimi, shembujt, lemat shqipe të
    /// kuptimit dhe të fqinjëve të drejtpërdrejtë semantikë.
    fn sense_signature(&self, sid: u32) -> HashSet<String> {
        let mut words = HashSet::new();
        let push_text = |t: &str, set: &mut HashSet<String>| {
            for w in tokenize(t) {
                match is_stop(&w) || w.chars().count() < 3 {
                    true => {}
                    false => {
                        set.insert(w);
                    }
                }
            }
        };
        let n = self.g.node(sid);
        for t in n.glosses.iter().chain(n.examples.iter()).chain(n.labels.iter()).filter(|t| t.lang == Lang::Sq) {
            push_text(&t.text, &mut words);
        }
        let neigh: Vec<u32> = self
            .g
            .out_edges(sid)
            .filter(|(_, e)| matches!(e.rel, Rel::Wn(_) | Rel::Bn(_)))
            .map(|(_, e)| e.to)
            .chain(self.g.in_edges(sid).filter(|(_, e)| matches!(e.rel, Rel::Wn(_) | Rel::Bn(_))).map(|(_, e)| e.from))
            .collect();
        for m in neigh {
            for t in self.g.node(m).labels.iter().filter(|t| t.lang == Lang::Sq) {
                push_text(&t.text, &mut words);
            }
        }
        words
    }

    /// Rregullat e eliminimit −Yi. Kthen numrin e eliminuar.
    pub(crate) fn eliminate(&self, f: &mut Focus, need_pos: Option<Pos>, context: &[String]) -> u32 {
        let mut removed = 0;
        // R-E1 — roli sintaksor kërkon një pjesë ligjërate.
        match need_pos {
            Some(p) => {
                let any_match = f.senses.iter().any(|s| s.pos == p);
                for s in f.senses.iter_mut() {
                    match (any_match, s.pos == p, &s.status) {
                        (true, false, CandStatus::Open) => {
                            s.status = CandStatus::Eliminated(format!(
                                "R-E1: roli në fjali kërkon {}, ky kuptim është {}",
                                p.label_sq(),
                                s.pos.label_sq()
                            ));
                            removed += 1;
                        }
                        _ => {}
                    }
                }
            }
            None => {}
        }
        // R-E2 — mbështetja e kontekstit në përkufizimin/fqinjët shqip.
        let ctx_lemmas: Vec<String> = context
            .iter()
            .flat_map(|c| self.morph.lemma_candidates(c).into_iter().map(|(l, _)| l))
            .collect();
        match ctx_lemmas.is_empty() {
            true => {}
            false => {
                for s in f.senses.iter_mut() {
                    let sig = self.sense_signature(s.sense);
                    let hits = ctx_lemmas.iter().filter(|l| sig.contains(*l)).count() as u32;
                    s.score = hits * 1_000;
                }
                let best = f.senses.iter().filter(|s| s.status == CandStatus::Open).map(|s| s.score).max().unwrap_or(0);
                match best > 0 {
                    true => {
                        for s in f.senses.iter_mut() {
                            match (&s.status, s.score < best) {
                                (CandStatus::Open, true) => {
                                    s.status = CandStatus::Eliminated(format!(
                                        "R-E2: konteksti ({}) mbështet më pak këtë kuptim ({} kundrejt {})",
                                        context.join(", "),
                                        s.score / 1_000,
                                        best / 1_000
                                    ));
                                    removed += 1;
                                }
                                _ => {}
                            }
                        }
                    }
                    false => {}
                }
            }
        }
        removed
    }

    /// Rregulla për të folur me njeriun (jo për të vendosur kuptimin):
    /// R-E5 — kuptimet me përkufizim shqip të njëjtë (ose njëri fillimi i tjetrit) paraqiten si një;
    /// R-E6 — kur një kuptim i së njëjtës pjesë ligjërate ka përkufizim shqip, ai pa përkufizim
    /// nuk pyetet (një mbiemër pa përkufizim nuk humbet për shkak të një emri me përkufizim).
    fn dedupe_for_speech(f: &mut Focus) -> u32 {
        let norm = |g: &str| g.trim().trim_end_matches('.').to_lowercase();
        let sq_pos: Vec<Pos> =
            f.senses.iter().filter(|s| s.status == CandStatus::Open && s.gloss_sq.is_some()).map(|s| s.pos).collect();
        let mut seen: Vec<String> = Vec::new();
        let mut removed = 0;
        for s in f.senses.iter_mut() {
            let same_pos_sq = sq_pos.contains(&s.pos);
            match (&s.status, &s.gloss_sq, same_pos_sq) {
                (CandStatus::Open, Some(g), _) => {
                    let k = norm(g);
                    match seen.iter().any(|x| k.starts_with(x.as_str()) || x.starts_with(k.as_str())) {
                        true => {
                            s.status = CandStatus::Eliminated("R-E5: përkufizim i njëjtë me një kuptim tjetër".into());
                            removed += 1;
                        }
                        false => seen.push(k),
                    }
                }
                (CandStatus::Open, None, true) => {
                    s.status = CandStatus::Eliminated(
                        "R-E6: pa përkufizim shqip, ndërsa një kuptim tjetër i së njëjtës pjesë ligjërate ka".into(),
                    );
                    removed += 1;
                }
                _ => {}
            }
        }
        removed
    }

    /// R-E0 — kuptimi duhet të ketë pjesën e ligjëratës që i japin fjalës gramatika, UD dhe
    /// Wiktionary (ose që kërkon nyja e shtypur: «i bardhë» → mbiemër). Kuptimet e tjera
    /// eliminohen; kur s'mbetet asnjë, fjala përgjigjet nga faktet e saj.
    fn by_pos(f: &mut Focus, want: &BTreeSet<Pos>) -> u32 {
        let mut removed = 0;
        let wanted = want.iter().map(|p| p.label_sq()).collect::<Vec<_>>().join("/");
        for s in f.senses.iter_mut() {
            match (want.is_empty(), want.contains(&s.pos), &s.status) {
                (false, false, CandStatus::Open) => {
                    s.status = CandStatus::Eliminated(format!(
                        "R-E0: kuptimi është {}, fjala sipas fjalorëve është {wanted}",
                        s.pos.label_sq()
                    ));
                    removed += 1;
                }
                _ => {}
            }
        }
        removed
    }

    /// Pjesa e ligjëratës që pritet: nyja e shtypur kërkon mbiemër; ndryshe ajo që i japin
    /// fjalës (dhe trajtave të saj me nyjë) gramatika, UD dhe Wiktionary.
    fn wanted_pos(&self, typed: &str, f: &Focus) -> BTreeSet<Pos> {
        match typed_article(typed) {
            true => BTreeSet::from([Pos::Adj]),
            false => discourse::aliases(self, f).into_iter().flat_map(|w| discourse::independent_pos(self, w)).collect(),
        }
    }

    /// Termi i fjalës: kur përdoruesi e shtypi me nyjë («i ri»), citohet ashtu, si mbiemër;
    /// ndryshe sipas pjesës kryesore të ligjëratës.
    fn typed_term(&self, typed: &str, f: &Focus) -> TermRef {
        let low = typed.trim().to_lowercase();
        let toks: Vec<&str> = low.split_whitespace().collect();
        match toks.as_slice() {
            ["të", x, ..] => TermRef::cited(&format!("i {x}")),
            [a @ ("i" | "e"), x, ..] => TermRef::cited(&format!("{a} {x}")),
            _ => discourse::term_of(self, f.resolved.word, &f.resolved.lemma),
        }
    }

    /// Termi i fjalës për një kuptim të zgjedhur: kuptimi emëror i një fjale që zakonisht
    /// citohet (p.sh. «fortë» = forcë) citohet pa nyjë; folja, mbiemri, ndajfolja citohen.
    fn sense_term(word: &TermRef, pos: Pos) -> TermRef {
        match (pos, word.cite) {
            (Pos::Noun, true) => TermRef::cited(crate::derive::strip_article(&word.lemma)),
            (Pos::Noun, false) => word.clone(),
            (_, true) => word.clone(),
            (_, false) => TermRef::cited(&word.lemma),
        }
    }

    /// Një etiketë shqipe e kuptimit, tjetër nga vetë fjala (për opsionet e pyetjes).
    pub fn alt_label(s: &SenseCand, lemma: &str) -> Option<String> {
        let bare = crate::derive::strip_article(lemma).to_string();
        s.labels_sq
            .iter()
            .map(|l| discourse::clean_label(l))
            .find(|l| !l.is_empty() && crate::derive::strip_article(l) != bare)
    }

    /// R-E7 — kur një kuptim emëror i fjalës ka si të vetmin hiperonim një kuptim tjetër të
    /// hapur të së njëjtës fjalë, njëri prej tyre është i tepërt. Cili mbetet e vendos përkthimi
    /// kryesor i fjalës (fjalori i Wiktionary-t): mbahet kuptimi që e ka atë përkthim («kalë» →
    /// horse, jo equine; «emër» → name, jo first name). Pa këtë dëshmi nuk eliminohet asgjë.
    fn subsume(&self, f: &mut Focus) -> u32 {
        let open: Vec<u32> = Self::open_senses(f);
        let main_en: HashSet<String> = discourse::dict_en(self, f)
            .into_iter()
            .chain(discourse::bridge_translation(self, f.resolved.word))
            .map(|x| x.to_lowercase())
            .collect();
        let only_parent = |child: u32, parent: u32| {
            let hs: Vec<u32> = self
                .g
                .out_edges(child)
                .filter(|(_, e)| match &e.rel {
                    Rel::Wn(s) => s == "@" || s == "@i",
                    Rel::Bn(r) => r == "HYPERNYM",
                    _ => false,
                })
                .map(|(_, e)| e.to)
                .collect();
            hs.as_slice() == [parent]
        };
        let has_main = |sid: u32| {
            self.g
                .node(sid)
                .labels
                .iter()
                .any(|t| t.lang == Lang::En && main_en.contains(&t.text.replace('_', " ").to_lowercase()))
        };
        let pairs: Vec<(u32, u32)> = open
            .iter()
            .flat_map(|&p| open.iter().filter(move |&&c| c != p).map(move |&c| (p, c)))
            .filter(|&(p, c)| only_parent(c, p))
            .collect();
        let mut drop: Vec<u32> = Vec::new();
        for (p, c) in pairs {
            match (has_main(p), has_main(c)) {
                (false, true) => drop.push(p),
                (true, false) => drop.push(c),
                _ => {}
            }
        }
        let mut removed = 0;
        for s in f.senses.iter_mut() {
            match (&s.status, drop.contains(&s.sense), s.pos == Pos::Noun) {
                (CandStatus::Open, true, true) => {
                    s.status = CandStatus::Eliminated(
                        "R-E7: kuptim i së njëjtës fjalë në raport lloj–kategori; mbetet ai me përkthimin kryesor".into(),
                    );
                    removed += 1;
                }
                _ => {}
            }
        }
        removed + self.near_duplicates(f)
    }

    /// R-E5b — dy kuptime të së njëjtës pjesë ligjërate që ndajnë një etiketë angleze dhe janë
    /// fqinjë në hierarki (motra nën të njëjtin hiperonim, ose prind–fëmijë) janë i njëjti kuptim
    /// për njeriun (p.sh. dy kuptimet e «javë»): mbetet ai me përkthimin kryesor, ndryshe i pari.
    fn near_duplicates(&self, f: &mut Focus) -> u32 {
        let open: Vec<u32> = Self::open_senses(f);
        let main_en: HashSet<String> = discourse::dict_en(self, f).into_iter().map(|x| x.to_lowercase()).collect();
        let en = |sid: u32| -> HashSet<String> {
            self.g
                .node(sid)
                .labels
                .iter()
                .filter(|t| t.lang == Lang::En)
                .map(|t| t.text.replace('_', " ").to_lowercase())
                .collect()
        };
        let hyp = |sid: u32| -> HashSet<u32> {
            self.g
                .out_edges(sid)
                .filter(|(_, e)| match &e.rel {
                    Rel::Wn(s) => s == "@" || s == "@i",
                    _ => false,
                })
                .map(|(_, e)| e.to)
                .collect()
        };
        let pos_of = |sid: u32| f.senses.iter().find(|s| s.sense == sid).map(|s| s.pos);
        let mut drop: Vec<u32> = Vec::new();
        for (i, &a) in open.iter().enumerate() {
            for &b in open.iter().skip(i + 1) {
                let share = en(a).intersection(&en(b)).next().is_some();
                let near = hyp(a).intersection(&hyp(b)).next().is_some() || hyp(a).contains(&b) || hyp(b).contains(&a);
                let main_b = en(b).iter().any(|x| main_en.contains(x)) && !en(a).iter().any(|x| main_en.contains(x));
                match (share && near && pos_of(a) == pos_of(b), drop.contains(&a) || drop.contains(&b), main_b) {
                    (true, false, true) => drop.push(a),
                    (true, false, false) => drop.push(b),
                    _ => {}
                }
            }
        }
        let mut removed = 0;
        for s in f.senses.iter_mut() {
            match (&s.status, drop.contains(&s.sense)) {
                (CandStatus::Open, true) => {
                    s.status = CandStatus::Eliminated("R-E5b: i njëjti kuptim me një kuptim fqinj (etiketë angleze e përbashkët)".into());
                    removed += 1;
                }
                _ => {}
            }
        }
        removed
    }

    pub(crate) fn open_senses(f: &Focus) -> Vec<u32> {
        f.senses.iter().filter(|s| s.status == CandStatus::Open).map(|s| s.sense).collect()
    }

    /// Kërkim në gjerësi mbi hiperonimet (@, @i): nga `start` te cilido nga `targets`.
    pub fn hyper_path(&self, start: u32, targets: &HashSet<u32>) -> Option<Vec<PathStep>> {
        let mut prev: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut q = VecDeque::from([start]);
        let mut seen = HashSet::from([start]);
        while let Some(n) = q.pop_front() {
            match targets.contains(&n) {
                true => {
                    let mut nodes = vec![n];
                    let mut edges = vec![];
                    let mut cur = n;
                    while let Some(&(p, e)) = prev.get(&cur) {
                        nodes.push(p);
                        edges.push(e);
                        cur = p;
                    }
                    nodes.reverse();
                    edges.reverse();
                    // edges[i] = skaji nga nodes[i] te nodes[i+1]
                    return Some(
                        nodes
                            .iter()
                            .enumerate()
                            .map(|(i, &nd)| PathStep { node: nd, edge: edges.get(i).copied() })
                            .collect(),
                    );
                }
                false => {}
            }
            for (eid, e) in self.g.out_edges(n) {
                match (&e.rel, seen.contains(&e.to)) {
                    (rel, false) => match is_hyper(rel) {
                        true => {
                            seen.insert(e.to);
                            prev.insert(e.to, (n, eid));
                            q.push_back(e.to);
                        }
                        false => {}
                    },
                    _ => {}
                }
            }
        }
        None
    }

    /// Konteksti i kuptimit për fjalinë shqipe: vetëm përkufizim shqip; kur
    /// mungon, identifikuesi i synset-it — kurrë tekst anglisht si shqip.
    fn context_label(&self, s: &SenseCand) -> String {
        self.short_gloss(s).unwrap_or_else(|| format!("synset {} të WordNet", s.synset))
    }

    fn short_gloss(&self, s: &SenseCand) -> Option<String> {
        let lemma = s.labels_sq.first().cloned().unwrap_or_default();
        s.gloss_sq
            .clone()
            .map(|g| g.split(';').next().unwrap_or("").to_string())
            .map(|g| crate::expression::strip_subject(&lemma, &g))
            .map(|g| {
                let words: Vec<&str> = g.split_whitespace().collect();
                match words.len() > 9 {
                    true => format!("{} …", words[..9].join(" ")),
                    false => g,
                }
            })
    }

    /// Analiza e plotë e një hyrjeje.
    pub fn analyze(&self, input: &str) -> Result<Analysis, String> {
        let parsed = parse(input);
        // Hyrjet e lira (një fjalë, dy fjalë, «pse/ku/për çfarë …») shkojnë së pari te biseda;
        // kur biseda s'ka lidhje të dëshmuara, vazhdon rruga e fjalës.
        // Pyetja e shprehur për kuptimin («ç'kuptim ka X») merr kuptimin, si te një njeri.
        let meaning_asked = tokenize(input).iter().any(|t| t.starts_with("kuptim") || t == "ç'kuptim");
        let free = match (&parsed, meaning_asked) {
            (_, true) => false,
            (Err(_) | Ok(Form::Explore { .. }) | Ok(Form::Meaning { .. }), false) => true,
            (Ok(_), false) => false,
        };
        match free.then(|| self.analyze_converse(input)).flatten() {
            Some(a) => return Ok(a),
            None => {}
        }
        let form = parsed?;
        let mut steps: Vec<ProcessStep> = Vec::new();
        let terms = tokenize(input).len() as u32;
        let a = match &form {
            Form::Explore { focus, context } | Form::Meaning { focus, context } => {
                self.analyze_word(input, form.clone(), focus, context, None, terms, &mut steps)
            }
            Form::IsA { claim, asked } => self.analyze_isa(input, form.clone(), claim, *asked, None, terms, &mut steps),
            Form::Conditional { cond, cons } => {
                self.analyze_isa(input, form.clone(), cons, false, Some(cond.clone()), terms, &mut steps)
            }
            Form::Opposite { focus } => self.analyze_opposite(input, form.clone(), focus, terms, &mut steps),
            Form::Why { claim } => self.analyze_why(input, form.clone(), claim, terms, &mut steps),
            Form::Compare { a, b } => self.analyze_compare(input, form.clone(), a, b, terms, &mut steps),
            Form::FacetQ { focus, facet } => self.analyze_facet(input, form.clone(), focus, *facet, terms, &mut steps),
            Form::Compose { words } => self.analyze_compose(input, form.clone(), words, terms, &mut steps),
            Form::Converse { .. } => return self.analyze_converse(input).ok_or_else(|| "asnjë lidhje e dëshmuar".to_string()),
        };
        Ok(a)
    }

    /// Fjalia e ndërtuar nga Nura: lidhjet (nga grafi) dhe përputhjet (nga gramatika) bëhen
    /// një pohim i vetëm; kur fjalët nuk lidhen, Nura e thotë.
    fn analyze_compose(&self, input: &str, form: Form, words: &[String], terms: u32, steps: &mut Vec<ProcessStep>) -> Analysis {
        let f = self.focus(&words[0]);
        self.step_i0(&f, terms, steps);
        self.step_split(&f, steps);
        let mut plan = ExprPlan { user_claim: None, user_condition: None, verdict: Verdict::Stated, condition: None, core: vec![], question: None };
        match crate::sentence::compose(self, words) {
            Ok(c) => {
                let sources: Vec<Source> = c.links.iter().flat_map(|l| l.sources.iter().copied()).collect();
                plan.core.push(Clause {
                    claim: Claim::Sentence {
                        text: c.text.clone(),
                        frame: c.frame,
                        support: c.links.iter().map(|l| l.label.clone()).collect(),
                        counter: c.counter.clone(),
                    },
                    polarity: c.polarity,
                    attribution: Some(Attribution { sources, note: format!("kornizë {}", c.frame) }),
                });
                steps.push(ProcessStep {
                    code: "VERIFY",
                    label: format!("Y = 1: fjalia «{}» mbështetet nga {} lidhje", c.text, c.links.len()),
                    nodes: vec![],
                    edges: vec![],
                    event: Some(Event::Verified { evidence_mass: Self::evidence_mass(c.links.len().max(1), 1) }),
                });
            }
            Err(reason) => {
                plan.verdict = Verdict::Unresolved;
                plan.core.push(Clause {
                    claim: Claim::NoLink { subj: TermRef::word(&words[0]), obj: TermRef::word(words.last().map(String::as_str).unwrap_or("")), relation: "fjali" },
                    polarity: Polarity::Neg,
                    attribution: Some(Attribution { sources: vec![], note: reason }),
                });
                steps.push(ProcessStep { code: "ABSENT", label: "asnjë lidhje që të mbajë një fjali".into(), nodes: vec![], edges: vec![], event: Some(Event::Absent { sources_checked: 0 }) });
            }
        }
        steps.push(ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) });
        Analysis { input: input.to_string(), form, focus: Some(f), other: None, path: vec![], cond_path: vec![], bridge_path: vec![], plan, steps: std::mem::take(steps) }
    }

    /// Etiketa e shkurtër e një kuptimi për butonat e zgjedhjes: sinonimi ose kategoria me pak
    /// fjalë («si «banesë»»); glosa vetëm kur s'ka asgjë më të shkurtër.
    pub fn short_label(&self, s: &SenseCand, lemma: &str) -> String {
        Self::alt_label(s, lemma)
            .or_else(|| self.first_sq_hypernym(s.sense).map(|(l, _)| l))
            .filter(|l| l.split_whitespace().count() <= 3)
            .map(|l| format!("si «{l}»"))
            .unwrap_or_else(|| option_label(s, lemma))
    }

    /// Shpeshtësia e një kuptimi në korpusin e WordNet-it (0 kur s'dihet).
    pub fn sense_frequency(&self, sense: u32) -> u32 {
        self.g
            .out_edges(sense)
            .find(|(_, e)| e.rel == Rel::Sem("SENSE_FREQ".into()))
            .and_then(|(_, e)| e.attr("tag").and_then(|t| t.parse().ok()))
            .unwrap_or(0)
    }

    /// Biseda: lidhjet e dëshmuara bëhen fjali të lidhura; pa lidhje, `None`.
    pub fn analyze_converse(&self, input: &str) -> Option<Analysis> {
        let forms = crate::fjalet::Fjalet::new(self);
        let reply = crate::biseda::converse(self, &forms, input)?;
        let terms = tokenize(input).len() as u32;
        let sources: BTreeSet<Source> = reply.facts.iter().flat_map(|f| f.sources.iter().copied()).collect();
        let links: Vec<String> = reply
            .facts
            .iter()
            .map(|f| format!("{} {} {}", f.head, f.kind.code(), f.tail.key()))
            .collect();
        let steps = vec![
            ProcessStep {
                code: "I0",
                label: format!("hyrja: {} fjalë; synimi {}", terms, reply.intent.code()),
                nodes: vec![],
                edges: vec![],
                event: Some(Event::Submitted { terms }),
            },
            ProcessStep {
                code: "LINKS",
                label: format!("{} lidhje të dëshmuara për {}", reply.facts.len(), reply.words.join(", ")),
                nodes: vec![],
                edges: vec![],
                event: Some(Event::CandidatesFound { senses: reply.facts.len() as u32, sources_hit: sources.len() as u32 }),
            },
            ProcessStep {
                code: "VERIFY",
                label: format!("çdo lidhje ka dy dëshmitarë: {}", links.join("; ")),
                nodes: vec![],
                edges: vec![],
                event: Some(Event::Verified { evidence_mass: Self::evidence_mass(sources.len().max(1), 1) }),
            },
            ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) },
        ];
        let plan = ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: None,
            core: vec![Clause {
                claim: Claim::Discourse { text: reply.text(), intent: reply.intent.code(), links: links.clone() },
                polarity: Polarity::Pos,
                attribution: Some(Attribution { sources: sources.into_iter().collect(), note: format!("biseda {}", reply.intent.code()) }),
            }],
            question: reply.question.clone(),
        };
        Some(Analysis {
            input: input.to_string(),
            form: Form::Converse { intent: reply.intent, words: reply.words.clone() },
            focus: None,
            other: None,
            path: vec![],
            cond_path: vec![],
            bridge_path: vec![],
            plan,
            steps,
        })
    }

    fn step_i0(&self, f: &Focus, terms: u32, steps: &mut Vec<ProcessStep>) {
        steps.push(ProcessStep {
            code: "I0",
            label: format!(
                "i₀ = «{}» → lemë «{}» ({})",
                f.resolved.input, f.resolved.lemma, f.resolved.how
            ),
            nodes: f.resolved.word.into_iter().collect(),
            edges: vec![],
            event: Some(Event::Submitted { terms }),
        });
    }

    fn step_split(&self, f: &Focus, steps: &mut Vec<ProcessStep>) {
        let mut nodes: Vec<u32> = f.senses.iter().map(|s| s.sense).collect();
        nodes.extend(f.usages.iter().take(6).map(|u| u.sentence));
        nodes.extend(f.form_rels.iter().map(|r| r.concept));
        steps.push(ProcessStep {
            code: "SPLIT",
            label: format!(
                "ndarja: Yi = {} kuptime (Albanet→WordNet) · Xi = {} përdorime UD + {} marrëdhënie forme (ConceptNet)",
                f.senses.len(),
                f.usages.len(),
                f.form_rels.len()
            ),
            nodes,
            edges: vec![],
            event: Some(Event::CandidatesFound { senses: f.senses.len() as u32, sources_hit: f.sources_hit.len() as u32 }),
        });
    }

    fn step_elim(&self, f: &Focus, removed: u32, steps: &mut Vec<ProcessStep>) {
        match removed > 0 {
            true => {
                let el: Vec<u32> = f
                    .senses
                    .iter()
                    .filter(|s| matches!(s.status, CandStatus::Eliminated(_)))
                    .map(|s| s.sense)
                    .collect();
                steps.push(ProcessStep {
                    code: "ELIM",
                    label: format!("−Yi: {removed} kuptime u eliminuan brenda hapësirës së tyre"),
                    nodes: el,
                    edges: vec![],
                    event: Some(Event::Eliminated { removed, remaining: Self::open_senses(f).len() as u32 }),
                });
            }
            false => {}
        }
    }



    fn evidence_mass(sources: usize, path_len: usize) -> u32 {
        let base = 5_000 + (sources as u32).min(4) * 1_000;
        base.saturating_sub((path_len as u32).saturating_sub(1) * 150).clamp(3_000, MASS_SCALE)
    }

    #[allow(clippy::too_many_arguments)]
    fn analyze_word(
        &self,
        input: &str,
        form: Form,
        focus: &str,
        context: &[String],
        _unused: Option<()>,
        terms: u32,
        steps: &mut Vec<ProcessStep>,
    ) -> Analysis {
        let mut f = self.focus(focus);
        self.step_i0(&f, terms, steps);
        let mut plan = ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: None,
            core: vec![],
            question: None,
        };
        let word = self.typed_term(focus, &f);
        // R-E0: vetëm kuptimet me pjesën e ligjëratës që i japin fjalës fjalorët.
        let want = self.wanted_pos(focus, &f);
        let r0 = Self::by_pos(&mut f, &want);
        // Kuptimet vetëm me glosë angleze (Wiktionary anglisht) janë urë përkthimi, jo
        // kuptime shqipe: fjala përshkruhet nga faktet shqipe dhe anglishtja thuhet si përkthim.
        // Kuptimi i leksikut të modelit nuk hap rrugën e kuptimeve (pa pyetje për kuptime, pa humbur
        // faktet e fjalës): përkufizimi i tij thuhet i pari te rruga e fjalës.
        let sq_open = f
            .senses
            .iter()
            .any(|s| !matches!(s.via, Source::KaikkiEn | Source::ModelClaude) && s.status == CandStatus::Open);
        let model_def: Option<(TermRef, String)> = f
            .senses
            .iter()
            .filter(|s| s.via == Source::ModelClaude && s.status == CandStatus::Open)
            .find_map(|s| s.gloss_sq.clone().map(|g| (Self::sense_term(&word, s.pos), g)));
        // Përkthimi i kuptimit të modelit: thuhet për të njëjtin kuptim që shpjegon përkufizimi
        // («ankth» = anxiety, jo «nightmare»; «kordhë» = sabre, jo «gut string» i homografit). Kur e
        // dëshmon edhe Wiktionary-t, merret ai; ndryshe përkthimi i vetë leksikut.
        let model_items: Vec<String> = f
            .senses
            .iter()
            .filter(|s| s.via == Source::ModelClaude && s.status == CandStatus::Open)
            .flat_map(|s| s.labels_en.iter().flat_map(|l| l.split([',', ';']).map(|x| x.trim().to_lowercase()).collect::<Vec<_>>()))
            .filter(|m| !m.is_empty())
            .collect();
        let confirmed_en = model_items.iter().find(|m| {
            f.senses.iter().filter(|s| s.via == Source::KaikkiEn).any(|s| {
                s.labels_en
                    .iter()
                    .chain(s.gloss_en.iter())
                    .any(|g| g.to_lowercase().split([',', ';', '(']).any(|x| x.trim() == m.as_str()))
            })
        });
        let model_en: Option<String> = confirmed_en.or(model_items.first()).cloned();
        let en_bridge = discourse::bridge_translation(self, f.resolved.word);
        // Faktet në nivel fjale (LEXIN, Wiktionary, rregullat e fjalëformimit, …).
        let word_facts = discourse::gather_word(self, &f);
        // Përkufizimi i modelit e thotë vetë llojin e sendit («yll që i jep Tokës dritë»): kategoria e
        // veçantë nuk thuhet, që të mos përsëritet as të flasë për një kuptim tjetër («arrë»: kokërr ≠ pemë).
        let word_facts = match &model_def {
            Some(_) => discourse::Facts { category: vec![], ..word_facts },
            None => word_facts,
        };
        let has_word_facts = [
            &word_facts.regional,
            &word_facts.synonyms,
            &word_facts.opposites,
            &word_facts.category,
            &word_facts.origin,
            &word_facts.native,
            &word_facts.dialect,
        ]
        .iter()
        .any(|v| !v.is_empty());
        // Fjala që e njeh gramatika, por për të cilën s'ka asnjë fakt tjetër: thuhet pjesa e
        // ligjëratës (dhe gjinia), jo «nuk e njoh».
        let grammar = f.resolved.word.map(|w| discourse::grammar_fact(self, w)).unwrap_or_default();
        let nothing = f.usages.is_empty()
            && f.form_rels.is_empty()
            && !has_word_facts
            && en_bridge.is_empty()
            && grammar.is_empty()
            && model_def.is_none();
        match (!sq_open, nothing) {
            (true, true) => {
                plan.verdict = Verdict::Unresolved;
                // WordNet 3.0 nuk ka lema shqipe, prandaj nuk numërohet si burim i kontrolluar.
                let checked = SQ_SOURCES.to_vec();
                plan.core.push(Clause {
                    claim: Claim::NotInSource { word, sources: checked.clone(), partial: self.conceptnet_partial },
                    polarity: Polarity::Neg,
                    attribution: None,
                });
                steps.push(ProcessStep {
                    code: "ABSENT",
                    label: format!("«{}» mungon në {} burimet shqipe të importuara", f.resolved.lemma, checked.len()),
                    nodes: vec![],
                    edges: vec![],
                    event: Some(Event::Absent { sources_checked: checked.len() as u32 }),
                });
            }
            (true, false) => {
                self.step_split(&f, steps);
                self.step_elim(&f, r0, steps);
                match f.usages.first() {
                    Some(u) => plan.core.push(Clause {
                        claim: Claim::Usage {
                            word: word.clone(),
                            form: u.form.clone(),
                            sentence: u.text.clone(),
                            sent_id: u.sent_id.clone(),
                            source: u.source,
                        },
                        polarity: Polarity::Pos,
                        attribution: None,
                    }),
                    None => {}
                }
                // Përkufizimi nga leksiku i modelit (fjala s'ka kuptim tjetër shqip).
                match &model_def {
                    Some((sword, gloss)) => plan.core.push(Clause {
                        claim: Claim::HasMeaning { word: sword.clone(), gloss: gloss.clone(), gloss_lang: Lang::Sq, index: 1, of: 1 },
                        polarity: Polarity::Pos,
                        attribution: Some(Attribution { sources: vec![Source::ModelClaude], note: "përkufizim i leksikut të modelit".into() }),
                    }),
                    None => {}
                }
                // Plani i fjalës: prejardhja, sinonimet, të kundërtat, fjala shqipe, dialekti.
                let has_category = !word_facts.category.is_empty();
                // Përkthimi anglisht: ura e Wiktionary-t; kur mungon, fjalori (Wiktionary në ConceptNet).
                let dict = discourse::dict_en(self, &f);
                // Kuptimi krahinor i rrallë është përkufizimi vetëm kur fjala s'ka kuptim të zakonshëm;
                // ndryshe thuhet pas përkthimit, si shtesë («thuhej edhe për»).
                let common = !en_bridge.is_empty() || !dict.is_empty();
                let (word_facts, regional_after) = match common {
                    false => (word_facts, vec![]),
                    true => {
                        let r = word_facts.regional.clone();
                        (discourse::Facts { regional: vec![], ..word_facts }, r)
                    }
                };
                plan.core.extend(discourse::describe(&word, word_facts, &[Source::SqRules], 3));
                let (en, en_src) = match (&model_en, en_bridge.is_empty()) {
                    (Some(m), _) => (vec![m.clone()], Source::KaikkiEn),
                    (None, false) => (en_bridge.clone(), Source::KaikkiEn),
                    (None, true) => (dict.into_iter().take(1).collect::<Vec<_>>(), Source::ConceptNet57),
                };
                match en.is_empty() {
                    false => plan.core.push(Clause {
                        claim: Claim::Facet { word: word.clone(), facet: Facet::TranslationEn, items: en },
                        polarity: Polarity::Pos,
                        attribution: Some(Attribution { sources: vec![en_src], note: "përkthim i fjalës".into() }),
                    }),
                    true => {}
                }
                match regional_after.is_empty() {
                    false => plan.core.push(Clause {
                        claim: Claim::Facet { word: word.clone(), facet: Facet::Regional, items: regional_after },
                        polarity: Polarity::Pos,
                        attribution: Some(Attribution {
                            sources: vec![Source::Gazulli1941],
                            note: "kuptim krahinor i rrallë, pas kuptimit të zakonshëm".into(),
                        }),
                    }),
                    true => {}
                }
                // Barazvlerësit në gjuhë të tjera me emër shqip (jo anglishtja, që u tha), vetëm kur
                // fjala nuk është shumëkuptimëshe: ndryshe do të përziheshin përkthime të kuptimeve
                // të ndryshme («mot» = mot/vit). FormOf vetëm në drejtimin e vet dhe jo për trajtat
                // e vetë fjalës; IsA vetëm kur fjalorët s'dhanë kategori.
                let poly = discourse::bridge_polysemous(self, f.resolved.word);
                let lemma = f.resolved.lemma.clone();
                let same = |t: &str| {
                    let t = t.to_lowercase();
                    t == lemma || crate::morph::strip_candidates(&t).iter().any(|c| *c == lemma)
                };
                let mut seen: Vec<String> = Vec::new();
                for r in f.form_rels.iter().filter(|r| {
                    let sq = r.other_lang == Some(Lang::Sq);
                    let _ = has_category;
                    let rel_ok = matches!((r.rel.as_str(), sq), ("/r/Synonym", false));
                    let lang_ok = match sq {
                        true => true,
                        false => crate::expression::known_lang(&r.other_lang_code) && r.other_lang_code != "en" && !poly,
                    };
                    rel_ok && lang_ok && !same(&r.other_term) && !r.other_term.ends_with(" translations")
                }) {
                    // Emrat gjermanë shkruhen me shkronjë të madhe.
                    let de_noun = match &self.g.node(r.other_concept).kind {
                        NodeKind::Concept { uri } => uri.split('/').nth(4) == Some("n"),
                        _ => false,
                    };
                    let term = match (r.other_lang_code.as_str(), de_noun) {
                        ("de", true) => crate::morph::capitalize(&r.other_term),
                        _ => r.other_term.clone(),
                    };
                    let key = format!("{}:{}", r.other_lang_code, term.to_lowercase());
                    match (seen.contains(&key), seen.len() < 3) {
                        (false, true) => {
                            seen.push(key);
                            let relation = match r.rel.as_str() {
                                "/r/IsA" => "lloj i (IsA)",
                                _ => "sinonim / barazvlerës (Synonym)",
                            };
                            plan.core.push(Clause {
                                claim: Claim::Related {
                                    a: word.clone(),
                                    b: TermRef::word(&term),
                                    b_lang: r.other_lang_code.clone(),
                                    relation: relation.to_string(),
                                },
                                polarity: Polarity::Pos,
                                attribution: Some(Attribution {
                                    sources: vec![Source::ConceptNet57],
                                    note: format!("{} · {}", r.dataset, r.license),
                                }),
                            });
                        }
                        _ => {}
                    }
                }
                // Kur s'ka asnjë fakt tjetër, fakti gramatikor.
                let said_something = plan.core.iter().any(|c| !matches!(c.claim, Claim::Usage { .. }));
                match (said_something, grammar.is_empty()) {
                    (false, false) => plan.core.push(Clause {
                        claim: Claim::Facet { word: word.clone(), facet: Facet::Grammar, items: grammar.clone() },
                        polarity: Polarity::Pos,
                        attribution: Some(Attribution { sources: vec![Source::Uniparser, Source::KaikkiEn, Source::WdLexemes], note: String::new() }),
                    }),
                    _ => {}
                }
                // Kur ka çfarë të thuhet, Nura flet; mungesa thuhet vetëm kur s'ka asgjë tjetër.
                let content = plan.core.iter().any(|c| !matches!(c.claim, Claim::Usage { .. }));
                match content {
                    true => steps.push(ProcessStep {
                        code: "VERIFY",
                        label: "Yi pa kuptim shqip; përgjigje nga faktet e fjalës (Xi)".into(),
                        nodes: vec![],
                        edges: vec![],
                        event: Some(Event::Verified { evidence_mass: Self::evidence_mass(f.sources_hit.len(), 1) }),
                    }),
                    false => {
                        plan.verdict = Verdict::Unresolved;
                        plan.core.insert(
                            0,
                            Clause {
                                claim: Claim::NotInSource { word: word.clone(), sources: vec![Source::OmwAls], partial: false },
                                polarity: Polarity::Neg,
                                attribution: None,
                            },
                        );
                        steps.push(ProcessStep {
                            code: "ABSENT",
                            label: "Yi bosh: asnjë kuptim në Albanet; vetëm dëshmi forme (Xi)".into(),
                            nodes: vec![],
                            edges: vec![],
                            event: Some(Event::Absent { sources_checked: 1 }),
                        });
                    }
                }
            }
            (false, _) => {
                self.step_split(&f, steps);
                let removed = r0 + self.eliminate(&mut f, None, context) + Self::dedupe_for_speech(&mut f);
                let removed = removed + self.subsume(&mut f);
                self.step_elim(&f, removed, steps);
                let open = Self::open_senses(&f);
                let total = f.senses.len();
                // Kuptimi kryesor: ai që ka përkthimin kryesor të fjalës sipas Wiktionary-t, ose ai që
                // korpusi i WordNet-it e njeh qartë si më të shpeshtin (≥ 3 herë dhe ≥ 1,5 herë më shumë
                // se i dyti). Pa njërën nga këto dëshmi, Nura nuk zgjedh vetë: pyet.
                let en_main: Option<String> = discourse::bridge_translation(self, f.resolved.word).into_iter().next().map(|e| e.to_lowercase());
                let by_bridge = en_main.and_then(|e| {
                    f.senses
                        .iter()
                        .filter(|x| x.status == CandStatus::Open)
                        .find(|x| x.labels_en.iter().any(|l| l.replace('_', " ").to_lowercase() == e))
                        .map(|x| x.sense)
                });
                let mut freq: Vec<(u32, u32)> = open.iter().map(|&s| (self.sense_frequency(s), s)).collect();
                freq.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                let by_corpus = match freq.as_slice() {
                    [(t1, s1), (t2, _), ..] => (*t1 >= 3 && *t1 * 2 >= *t2 * 3).then_some(*s1),
                    _ => None,
                };
                let dominant: Option<u32> = match open.len() {
                    1 => open.first().copied(),
                    _ => by_bridge.or(by_corpus),
                };
                match dominant {
                    None => {
                        let n = open.len();
                        plan.verdict = Verdict::Unresolved;
                        // Opsionet numërohen 1..n, në rendin e kuptimeve të mbetura.
                        let options: Vec<String> = f
                            .senses
                            .iter()
                            .filter(|s| s.status == CandStatus::Open)
                            .enumerate()
                            .map(|(i, s)| match (self.short_gloss(s), Self::alt_label(s, &f.resolved.lemma)) {
                                (Some(g), _) => format!("({}) «{}»", i + 1, g),
                                (None, Some(l)) => format!("({}) si «{}»", i + 1, l),
                                (None, None) => format!(
                                    "({}) «{}» (në anglisht)",
                                    i + 1,
                                    s.labels_en.first().cloned().unwrap_or_default().replace('_', " ")
                                ),
                            })
                            .collect();
                        plan.core.push(Clause {
                            claim: Claim::SenseCount { word: word.clone(), open: n, total },
                            polarity: Polarity::Pos,
                            attribution: Some(Attribution { sources: vec![Source::OmwAls, Source::Pwn30], note: String::new() }),
                        });
                        let short: Vec<String> = f
                            .senses
                            .iter()
                            .filter(|x| x.status == CandStatus::Open)
                            .filter_map(|x| {
                                Self::alt_label(x, &f.resolved.lemma)
                                    .or_else(|| self.first_sq_hypernym(x.sense).map(|(l, _)| l))
                                    .filter(|l| l.split_whitespace().count() <= 3)
                            })
                            .fold(Vec::new(), |mut acc: Vec<String>, l| {
                                match acc.contains(&l) {
                                    true => {}
                                    false => acc.push(l),
                                }
                                acc
                            });
                        let _ = &options;
                        plan.question = Some(match short.as_slice() {
                            [] => format!("Për cilin kuptim të «{}» e ke fjalën?", word.lemma),
                            [a] => format!("E ke fjalën për «{}» si «{a}», apo për diçka tjetër?", word.lemma),
                            [a, b] => format!("E ke fjalën për «{}» si «{a}» apo si «{b}»?", word.lemma),
                            [a, b, c, ..] => format!("E ke fjalën për «{}» si «{a}», si «{b}» apo si «{c}»?", word.lemma),
                        });
                        steps.push(ProcessStep {
                            code: "HOLD",
                            label: format!("{n} kuptime mbeten të barabarta → HOLD + pyetje iZ"),
                            nodes: open,
                            edges: vec![],
                            event: Some(Event::Ambiguous { remaining: n as u32 }),
                        });
                    }
                    Some(d) => {
                        let idx = f.senses.iter().position(|s| s.sense == d).unwrap_or(0);
                        f.senses[idx].status = CandStatus::Selected(match (open.len(), removed > 0) {
                            (1, true) => "i vetmi i mbetur pas eliminimit".into(),
                            (1, false) => "kuptimi i vetëm në burim".into(),
                            _ => "kuptimi më i zakonshëm; të tjerët mbeten pyetje".into(),
                        });
                        let s = &f.senses[idx];
                        let sword = Self::sense_term(&word, s.pos);
                        let (gloss, lang) = match (&s.gloss_sq, &s.gloss_en) {
                            (Some(g), _) => (g.clone(), Lang::Sq),
                            (None, Some(g)) => (g.clone(), Lang::En),
                            (None, None) => (s.labels_en.join(", "), Lang::En),
                        };
                        // Plani i tekstit: pas përkufizimit, deri në 2 fasete të dobishme
                        // (kategoria koherente me përkufizimin, sinonimet, e kundërta, …).
                        let def_sq = match lang {
                            Lang::Sq => Some(s.gloss_sq.clone().unwrap_or_default()),
                            Lang::En => None,
                        };
                        let facts = discourse::gather(self, &f, s.sense, def_sq.as_deref());
                        let budget = match lang {
                            Lang::Sq => 2,
                            Lang::En => 3,
                        };
                        let described = discourse::describe(&sword, facts, &sense_sources(s.via), budget);
                        // Përkufizimi anglisht thuhet vetëm kur s'ka asnjë fakt shqip për fjalën.
                        match (lang, described.is_empty()) {
                            (Lang::En, false) => {}
                            _ => plan.core.push(Clause {
                                claim: Claim::HasMeaning { word: sword.clone(), gloss, gloss_lang: lang, index: idx + 1, of: total },
                                polarity: Polarity::Pos,
                                attribution: Some(Attribution {
                                    sources: sense_sources(s.via),
                                    note: format!("synset {}", s.synset),
                                }),
                            }),
                        }
                        plan.core.extend(described);
                        let mass = Self::evidence_mass(f.sources_hit.len(), 1);
                        steps.push(ProcessStep {
                            code: "VERIFY",
                            label: format!("Y = 1: kuptimi {} mbështetet nga burimi", s.synset),
                            nodes: vec![s.sense],
                            edges: vec![],
                            event: Some(Event::Verified { evidence_mass: mass }),
                        });
                        // Me disa kuptime, Nura flet për më të zakonshmin dhe pyet për të tjerët me fjalë
                        // të shkurtra, jo me zëra fjalori.
                        let others: Vec<String> = f
                            .senses
                            .iter()
                            .filter(|x| x.status == CandStatus::Open)
                            .filter_map(|x| {
                                Self::alt_label(x, &f.resolved.lemma)
                                    .or_else(|| self.first_sq_hypernym(x.sense).map(|(l, _)| l))
                                    .filter(|l| l.split_whitespace().count() <= 3)
                            })
                            .fold(Vec::new(), |mut acc: Vec<String>, l| {
                                match acc.contains(&l) {
                                    true => {}
                                    false => acc.push(l),
                                }
                                acc
                            });
                        match others.as_slice() {
                            [] => {}
                            [a] => plan.question = Some(format!("Apo e ke fjalën për «{}» si «{a}»?", word.lemma)),
                            [a, b, ..] => plan.question = Some(format!("Apo e ke fjalën për «{}» si «{a}» ose si «{b}»?", word.lemma)),
                        }
                        match open.len() > 1 {
                            true => steps.push(ProcessStep {
                                code: "HOLD",
                                label: format!("{} kuptime: flitet për më të zakonshmin, të tjerët mbeten pyetje iZ", open.len()),
                                nodes: open.clone(),
                                edges: vec![],
                                event: Some(Event::Ambiguous { remaining: open.len() as u32 - 1 }),
                            }),
                            false => {}
                        }
                    }
                }
            }
        }
        // Biseda, jo fjalor: përkthimi anglisht dhe barazvlerësit në gjuhë të tjera thuhen vetëm
        // kur pyeten, ose kur janë e vetmja gjë që dihet për fjalën.
        let foreign = |c: &Clause| match &c.claim {
            Claim::Facet { facet: Facet::TranslationEn, .. } => true,
            Claim::Related { b_lang, .. } => b_lang != "sq",
            _ => false,
        };
        let native = plan.core.iter().any(|c| !foreign(c) && !matches!(c.claim, Claim::Usage { .. } | Claim::NotInSource { .. }));
        match (matches!(form, Form::Explore { .. } | Form::Meaning { .. }), native) {
            (true, true) => plan.core.retain(|c| !foreign(c)),
            _ => {}
        }
        steps.push(ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) });
        Analysis {
            input: input.into(),
            form,
            focus: Some(f),
            other: None,
            path: vec![],
            cond_path: vec![],
            bridge_path: vec![],
            plan,
            steps: std::mem::take(steps),
        }
    }

    pub fn first_sq_hypernym(&self, sense: u32) -> Option<(String, Vec<PathStep>)> {
        // BFS lart deri te kuptimi i parë që ka lemë shqipe.
        let mut q = VecDeque::from([(sense, vec![PathStep { node: sense, edge: None }])]);
        let mut seen = HashSet::from([sense]);
        while let Some((n, path)) = q.pop_front() {
            match (n != sense, self.g.label_in(n, Lang::Sq)) {
                (true, Some(l)) => return Some((l.to_lowercase(), path)),
                _ => {}
            }
            for (eid, e) in self.g.out_edges(n) {
                match (&e.rel, seen.contains(&e.to)) {
                    (rel, false) => match is_hyper(rel) {
                        true => {
                            seen.insert(e.to);
                            let mut p = path.clone();
                            p.last_mut().map(|x| x.edge = Some(eid));
                            p.push(PathStep { node: e.to, edge: None });
                            q.push_back((e.to, p));
                        }
                        false => {}
                    },
                    _ => {}
                }
            }
        }
        None
    }

    /// Skaji ConceptNet `/r/IsA` nga një koncept i formës `f` te një koncept i formës `o`
    /// (vetëm drejtimi f → o; koncepte shqipe; lidhja me fjalën sipas përputhjes së lemës).
    fn cn_isa<'f>(&self, f: &'f Focus, o: &Focus) -> Option<&'f FormRel> {
        let o_concepts: HashSet<u32> = match o.resolved.word {
            Some(w) => self.g.in_edges(w).filter(|(_, e)| e.rel == Rel::FormOf).map(|(_, e)| e.from).collect(),
            None => HashSet::new(),
        };
        f.form_rels
            .iter()
            .find(|r| r.rel == "/r/IsA" && r.outgoing && o_concepts.contains(&r.other_concept))
    }

    /// Rruga hiperonimike nga kuptimet e hapura të `f` te cilido kuptim i `o`.
    pub(crate) fn isa_path(&self, f: &Focus, o: &Focus) -> Option<(u32, Vec<PathStep>)> {
        let targets: HashSet<u32> = o.senses.iter().map(|s| s.sense).collect();
        Self::open_senses(f)
            .into_iter()
            .filter_map(|s| self.hyper_path(s, &targets).map(|p| (s, p)))
            .min_by_key(|(_, p)| p.len())
    }

    #[allow(clippy::too_many_arguments)]
    fn analyze_isa(
        &self,
        input: &str,
        form: Form,
        claim: &Simple,
        asked: bool,
        cond: Option<Simple>,
        terms: u32,
        steps: &mut Vec<ProcessStep>,
    ) -> Analysis {
        let mut f = self.focus(&claim.subj);
        let o = self.focus(&claim.obj);
        self.step_i0(&f, terms, steps);
        self.step_split(&f, steps);
        let removed = self.eliminate(&mut f, Some(Pos::Noun), &[]);
        self.step_elim(&f, removed, steps);
        let subj_t = TermRef::word(&f.resolved.lemma);
        let obj_t = TermRef::word(&o.resolved.lemma);
        let user_clause = Clause {
            claim: Claim::IsA { subj: subj_t.clone(), obj: obj_t.clone() },
            polarity: claim.polarity,
            attribution: None,
        };
        let mut plan = ExprPlan {
            user_claim: match asked {
                true => None,
                false => Some(user_clause.clone()),
            },
            user_condition: None,
            verdict: Verdict::Unresolved,
            condition: None,
            core: vec![],
            question: None,
        };
        let mut path_out = vec![];
        let mut cond_out = vec![];
        let mut bridge_out = vec![];
        // Kushti: vlerësohet veçmas dhe ruhet si kusht.
        let cond_clause = cond.as_ref().map(|c| {
            let cf = self.focus(&c.subj);
            let co = self.focus(&c.obj);
            Clause {
                claim: Claim::IsA { subj: TermRef::word(&cf.resolved.lemma), obj: TermRef::word(&co.resolved.lemma) },
                polarity: c.polarity,
                attribution: None,
            }
        });
        // Dëshmia kryesore: rruga e kuptimeve (WordNet). Dëshmia dytësore: skaji
        // /r/IsA i ConceptNet ndërmjet FORMAVE shqipe (pa kuptim të caktuar).
        let wn_path = match (f.senses.is_empty(), o.senses.is_empty()) {
            (false, false) => self.isa_path(&f, &o),
            _ => None,
        };
        let cn_isa = self.cn_isa(&f, &o);
        let any_missing = f.senses.is_empty() || o.senses.is_empty();
        match (wn_path, cn_isa, any_missing) {
            (None, Some(r), _) => {
                plan.verdict = match (asked, claim.polarity) {
                    (true, _) => Verdict::Answered,
                    (false, Polarity::Pos) => Verdict::UserClaimSupported,
                    (false, Polarity::Neg) => Verdict::UserClaimContradicted,
                };
                plan.core.push(Clause {
                    claim: Claim::IsA { subj: subj_t.clone(), obj: obj_t.clone() },
                    polarity: Polarity::Pos,
                    attribution: Some(Attribution {
                        sources: vec![Source::ConceptNet57],
                        note: format!(
                            "skaj /r/IsA ndërmjet formave shqipe, pa kuptim të caktuar; {} · {}",
                            r.dataset, r.license
                        ),
                    }),
                });
                let mass = Self::evidence_mass(1, 2);
                steps.push(ProcessStep {
                    code: "VERIFY",
                    label: "Y = 1 në nivel forme: /r/IsA në ConceptNet (WordNet pa rrugë)".into(),
                    nodes: vec![r.concept, r.other_concept],
                    edges: vec![r.edge],
                    event: Some(match (asked, claim.polarity) {
                        (false, Polarity::Neg) => Event::Contradiction { evidence_mass: mass },
                        _ => Event::Verified { evidence_mass: mass },
                    }),
                });
            }
            (None, None, true) => {
                let missing: Vec<&Focus> = [&f, &o].into_iter().filter(|x| x.senses.is_empty()).collect();
                // Fjala që grafi e njeh (pa kuptim në fjalor): thuhet «nuk e di nëse», jo «nuk e njoh».
                let all_known = missing.iter().all(|m| m.resolved.word.is_some());
                match all_known {
                    true => plan.core.push(Clause {
                        claim: Claim::NoLink { subj: subj_t.clone(), obj: obj_t.clone(), relation: "lloj" },
                        polarity: Polarity::Neg,
                        attribution: None,
                    }),
                    false => {
                        for m in missing.iter().filter(|m| m.resolved.word.is_none()) {
                            plan.core.push(Clause {
                                claim: Claim::NotInSource {
                                    word: TermRef::word(&m.resolved.lemma),
                                    sources: vec![Source::OmwAls],
                                    partial: false,
                                },
                                polarity: Polarity::Neg,
                                attribution: None,
                            });
                        }
                    }
                }
                steps.push(ProcessStep {
                    code: "ABSENT",
                    label: format!(
                        "mungon kuptimi në Albanet për: {}",
                        missing.iter().map(|m| m.resolved.lemma.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                    nodes: vec![],
                    edges: vec![],
                    event: Some(Event::Absent { sources_checked: 1 }),
                });
            }
            (Some((start, path)), _, _) => {
                    let en_only = path
                        .iter()
                        .filter(|p| self.g.label_in(p.node, Lang::Sq).is_none())
                        .count();
                    for s in f.senses.iter_mut() {
                        match (s.sense == start, &s.status) {
                            (true, _) => s.status = CandStatus::Selected("ka rrugë hiperonimie drejt objektit".into()),
                            (false, CandStatus::Open) => {
                                s.status = CandStatus::Eliminated("R-E3: pa rrugë hiperonimie drejt objektit".into())
                            }
                            _ => {}
                        }
                    }
                    // Konteksti i kuptimit thuhet vetëm kur një kuptim tjetër i fjalës nuk e ka këtë
                    // rrugë: atëherë pohimi vlen vetëm për këtë kuptim.
                    let targets: HashSet<u32> = o.senses.iter().map(|s| s.sense).collect();
                    let some_fail = f
                        .senses
                        .iter()
                        .filter(|s| s.sense != start && matches!(s.status, CandStatus::Open | CandStatus::Selected(_)))
                        .any(|s| self.hyper_path(s.sense, &targets).is_none());
                    // Kualifikuesi: një etiketë shqipe e kuptimit (jo një përkufizim i prerë).
                    let gloss = match (f.senses.len() > 1, some_fail) {
                        (true, true) => f
                            .senses
                            .iter()
                            .find(|s| s.sense == start)
                            .map(|s| Self::alt_label(s, &f.resolved.lemma).unwrap_or_else(|| self.context_label(s))),
                        _ => None,
                    };
                    let attribution = Attribution {
                        sources: vec![Source::OmwAls, Source::Pwn30],
                        note: format!(
                            "zinxhir hiperonimesh {} hapa; {} nyje pa lemë shqipe",
                            path.len().saturating_sub(1),
                            en_only
                        ),
                    };
                    let positive = Clause {
                        claim: Claim::IsA {
                            subj: TermRef { lemma: f.resolved.lemma.clone(), sense_gloss: gloss, cite: false },
                            obj: obj_t.clone(),
                        },
                        polarity: Polarity::Pos,
                        attribution: Some(attribution),
                    };
                    plan.verdict = match (asked, claim.polarity) {
                        (true, _) => Verdict::Answered,
                        (false, Polarity::Pos) => Verdict::UserClaimSupported,
                        (false, Polarity::Neg) => Verdict::UserClaimContradicted,
                    };
                    plan.core.push(positive);
                    let mass = Self::evidence_mass(2, path.len());
                    steps.push(ProcessStep {
                        code: "VERIFY",
                        label: format!("Y = 1: rrugë hiperonimie me {} hapa", path.len().saturating_sub(1)),
                        nodes: path.iter().map(|p| p.node).collect(),
                        edges: path.iter().filter_map(|p| p.edge).collect(),
                        event: Some(match (asked, claim.polarity) {
                            (false, Polarity::Neg) => Event::Contradiction { evidence_mass: mass },
                            _ => Event::Verified { evidence_mass: mass },
                        }),
                    });
                    path_out = path;
            }
            (None, None, false) => {
                    plan.core.push(Clause {
                        claim: Claim::NoLink { subj: subj_t.clone(), obj: obj_t.clone(), relation: "lloj" },
                        polarity: Polarity::Neg,
                        attribution: Some(Attribution {
                            sources: vec![Source::OmwAls, Source::Pwn30, Source::ConceptNet57],
                            note: String::new(),
                        }),
                    });
                    steps.push(ProcessStep {
                        code: "ABSENT",
                        label: "asnjë rrugë hiperonimie dhe asnjë /r/IsA — mungesë lidhjeje, jo mohim".into(),
                        nodes: vec![],
                        edges: vec![],
                        event: Some(Event::Absent { sources_checked: 3 }),
                    });
            }
        }
        // Kushti: plani i sistemit e ruan si kusht; nëse objekti i kushtit
        // arrin objektin e përfundimit, kjo shënohet si urë (rrjedhje).
        match (&cond, cond_clause) {
            (Some(c), Some(cc)) => {
                let cf = self.focus(&c.subj);
                let co = self.focus(&c.obj);
                let cond_path = self.isa_path(&cf, &co);
                let co_targets: HashSet<u32> = o.senses.iter().map(|s| s.sense).collect();
                let bridge = co
                    .senses
                    .iter()
                    .filter_map(|s| self.hyper_path(s.sense, &co_targets))
                    .min_by_key(|p| p.len());
                let note = match (&cond_path, &bridge) {
                    (Some(_), Some(b)) => format!(
                        "kushti ka mbështetje në burim; «{}» → «{}» me {} hapa, prandaj përfundimi rrjedh nga kushti",
                        co.resolved.lemma,
                        o.resolved.lemma,
                        b.len().saturating_sub(1)
                    ),
                    (None, Some(_)) => "kushti nuk ka mbështetje në burim; ura kusht→përfundim ekziston".into(),
                    (_, None) => "nuk u gjet urë nga objekti i kushtit te objekti i përfundimit".into(),
                };
                plan.user_condition = match plan.user_claim.is_some() {
                    true => Some(cc.clone()),
                    false => None,
                };
                // Pohimi i sistemit ruan formën kushtore.
                plan.condition = Some(Clause { attribution: None, ..cc });
                match plan.core.first_mut() {
                    Some(first) => {
                        first.attribution = Some(Attribution {
                            sources: vec![Source::OmwAls, Source::Pwn30],
                            note,
                        })
                    }
                    None => {}
                }
                cond_out = cond_path.map(|(_, p)| p).unwrap_or_default();
                bridge_out = bridge.unwrap_or_default();
                steps.push(ProcessStep {
                    code: "CONDITION",
                    label: "kushti u vlerësua veçmas dhe u ruajt si kusht («nëse … atëherë …»)".into(),
                    nodes: cond_out.iter().chain(bridge_out.iter()).map(|p| p.node).collect(),
                    edges: cond_out.iter().chain(bridge_out.iter()).filter_map(|p| p.edge).collect(),
                    event: None,
                });
            }
            _ => {}
        }
        steps.push(ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) });
        Analysis {
            input: input.into(),
            form,
            focus: Some(f),
            other: Some(o),
            path: path_out,
            cond_path: cond_out,
            bridge_path: bridge_out,
            plan,
            steps: std::mem::take(steps),
        }
    }

    fn analyze_opposite(&self, input: &str, form: Form, focus: &str, terms: u32, steps: &mut Vec<ProcessStep>) -> Analysis {
        let f = self.focus(focus);
        self.step_i0(&f, terms, steps);
        self.step_split(&f, steps);
        let word = discourse::term_of(self, f.resolved.word, &f.resolved.lemma);
        let mut plan = ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: None,
            core: vec![],
            question: None,
        };
        // E njëjta zgjedhje si te përshkrimi i fjalës: antonimet e burimeve (në të dy drejtimet),
        // mohimet sipas rregullave dhe ConceptNet, me rregullin e dëshmive.
        let (ants, srcs) = discourse::word_opposites(self, &f);
        match ants.is_empty() {
            true => {
                plan.verdict = Verdict::Unresolved;
                plan.core.push(Clause {
                    claim: Claim::NotInSource { word: word.clone(), sources: vec![Source::ConceptNet57], partial: self.conceptnet_partial },
                    polarity: Polarity::Neg,
                    attribution: Some(Attribution { sources: vec![Source::ConceptNet57], note: "kërkuar: e kundërta në shqip".into() }),
                });
                steps.push(ProcessStep {
                    code: "ABSENT",
                    label: "asnjë e kundërt shqip e dëshmuar".into(),
                    nodes: vec![],
                    edges: vec![],
                    event: Some(Event::Absent { sources_checked: 1 }),
                });
            }
            false => {
                for a in &ants {
                    plan.core.push(Clause {
                        claim: Claim::Opposite { a: word.clone(), b: TermRef::word(a) },
                        polarity: Polarity::Pos,
                        attribution: Some(Attribution { sources: srcs.clone(), note: String::new() }),
                    });
                }
                steps.push(ProcessStep {
                    code: "VERIFY",
                    label: format!("Y = 1: {} e kundërt(a) shqip", ants.len()),
                    nodes: vec![],
                    edges: vec![],
                    event: Some(Event::Verified { evidence_mass: Self::evidence_mass(srcs.len().max(1), 1) }),
                });
            }
        }
        steps.push(ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) });
        Analysis {
            input: input.into(),
            form,
            focus: Some(f),
            other: None,
            path: vec![],
            cond_path: vec![],
            bridge_path: vec![],
            plan,
            steps: std::mem::take(steps),
        }
    }

    /// Etiketat shqipe të nyjeve të ndërmjetme të një rruge (pa kryefjalën dhe objektin), të
    /// pastruara («një thundrak» → «thundrak») dhe pa përsëritje.
    fn chain_labels(&self, path: &[PathStep]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let n = path.len();
        for st in path.iter().skip(1).take(n.saturating_sub(2)) {
            match discourse::sq_label(self, st.node) {
                Some(l) => match out.contains(&l) {
                    true => {}
                    false => out.push(l),
                },
                None => {}
            }
        }
        out
    }

    /// "pse X është Y?" — përgjigjja është vetë arsyeja: zinxhiri i kategorive.
    fn analyze_why(&self, input: &str, form: Form, claim: &Simple, terms: u32, steps: &mut Vec<ProcessStep>) -> Analysis {
        let asked = claim.polarity == Polarity::Pos;
        let mut a = self.analyze_isa(input, form, claim, asked, None, terms, steps);
        let chain = self.chain_labels(&a.path);
        let subj = a.focus.as_ref().map(|f| f.resolved.lemma.clone());
        let obj = a.other.as_ref().map(|o| o.resolved.lemma.clone());
        // «Pse qeni nuk është kafshë?» — pohimi i mohuar nuk shpjegohet me «sepse»: përgjigjja
        // e pyetjes «a është» e korrigjon («Jo, qeni është kafshë»).
        let negated = claim.polarity == Polarity::Neg;
        match (a.path.is_empty() || negated, subj, obj) {
            (false, Some(sj), Some(ob)) => {
                let attribution = a.plan.core.first().and_then(|c| c.attribution.clone());
                a.plan.verdict = Verdict::Stated;
                a.plan.core = vec![Clause {
                    claim: Claim::Chain { subj: TermRef::word(&sj), chain, obj: TermRef::word(&ob), conclude: true },
                    polarity: Polarity::Pos,
                    attribution,
                }];
            }
            _ => {}
        }
        a
    }

    /// "ç'dallim ka X nga Y?" — kategoria e përbashkët dhe dallimi.
    fn analyze_compare(&self, input: &str, form: Form, a: &str, b: &str, terms: u32, steps: &mut Vec<ProcessStep>) -> Analysis {
        let fa = self.focus(a);
        let fb = self.focus(b);
        self.step_i0(&fa, terms, steps);
        self.step_split(&fa, steps);
        let clause = discourse::compare(self, &fa, &fb);
        let ok = clause.polarity == Polarity::Pos;
        steps.push(ProcessStep {
            code: match ok {
                true => "VERIFY",
                false => "ABSENT",
            },
            label: "krahasimi: paraardhësi i përbashkët në hierarki".into(),
            nodes: vec![],
            edges: vec![],
            event: Some(match ok {
                true => Event::Verified { evidence_mass: Self::evidence_mass(2, 2) },
                false => Event::Absent { sources_checked: 2 },
            }),
        });
        steps.push(ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) });
        Analysis {
            input: input.into(),
            form,
            focus: Some(fa),
            other: Some(fb),
            path: vec![],
            cond_path: vec![],
            bridge_path: vec![],
            plan: ExprPlan {
                user_claim: None,
                user_condition: None,
                verdict: Verdict::Stated,
                condition: None,
                core: vec![clause],
                question: None,
            },
            steps: std::mem::take(steps),
        }
    }

    /// Pyetje për një faset ("çfarë pjesësh ka X?"): zgjidhet kuptimi i parë që ka përgjigje.
    fn analyze_facet(&self, input: &str, form: Form, focus: &str, facet: Facet, terms: u32, steps: &mut Vec<ProcessStep>) -> Analysis {
        let mut f = self.focus(focus);
        self.step_i0(&f, terms, steps);
        self.step_split(&f, steps);
        let word = self.typed_term(focus, &f);
        // Të njëjtat rregulla si te kuptimi: pjesa e ligjëratës, përsëritjet, lloj–kategoria.
        let want = self.wanted_pos(focus, &f);
        let removed = Self::by_pos(&mut f, &want) + Self::dedupe_for_speech(&mut f) + self.subsume(&mut f);
        self.step_elim(&f, removed, steps);
        // Kuptimet vetëm-anglisht (urë) nuk zgjidhen si kuptim: fjala përgjigjet nga faktet e saj.
        let sq_senses = f.senses.iter().any(|s| s.via != Source::KaikkiEn);
        let chosen = match (facet, sq_senses) {
            (Facet::TranslationEn, _) => None,
            (_, true) => discourse::sense_with_facet(self, &f, facet),
            (_, false) => None,
        };
        let clause = match (facet, chosen) {
            (Facet::TranslationEn, _) => self.translation_clause(&f, &word),
            (_, Some(sid)) => {
                let via = f.senses.iter().find(|s| s.sense == sid).map(|s| s.via).unwrap_or(Source::OmwAls);
                let pos = f.senses.iter().find(|s| s.sense == sid).map(|s| s.pos).unwrap_or(Pos::Noun);
                discourse::single(&Self::sense_term(&word, pos), discourse::gather(self, &f, sid, None), facet, &sense_sources(via))
            }
            // Fjalët pa kuptim shqip: faktet në nivel fjale (rregulla, LEXIN, Wiktionary, ConceptNet).
            (_, None) => discourse::single(&word, discourse::gather_word(self, &f), facet, &[Source::SqRules]),
        };
        let ok = clause.polarity == Polarity::Pos;
        steps.push(ProcessStep {
            code: match ok {
                true => "VERIFY",
                false => "ABSENT",
            },
            label: format!("faseti «{}»", facet.code()),
            nodes: chosen.into_iter().collect(),
            edges: vec![],
            event: Some(match ok {
                true => Event::Verified { evidence_mass: Self::evidence_mass(1, 1) },
                false => Event::Absent { sources_checked: 2 },
            }),
        });
        steps.push(ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) });
        Analysis {
            input: input.into(),
            form,
            focus: Some(f),
            other: None,
            path: vec![],
            cond_path: vec![],
            bridge_path: vec![],
            plan: ExprPlan {
                user_claim: None,
                user_condition: None,
                verdict: Verdict::Stated,
                condition: None,
                core: vec![clause],
                question: None,
            },
            steps: std::mem::take(steps),
        }
    }

    /// «Si quhet X në anglisht?»: etiketat e një kuptimi shqip që i dëshmon edhe fjalori
    /// (Wiktionary); pastaj ura e Wiktionary-t; pastaj fjalori vetë; pastaj etiketat e kuptimit
    /// të parë shqip.
    fn translation_clause(&self, f: &Focus, word: &TermRef) -> Clause {
        let dict = discourse::dict_en(self, f);
        let mut hit: Vec<String> = Vec::new();
        for l in f
            .senses
            .iter()
            .filter(|s| s.via != Source::KaikkiEn)
            .flat_map(|s| s.labels_en.iter().map(|l| l.replace('_', " ")))
            .filter(|l| dict.contains(l))
        {
            match hit.contains(&l) {
                true => {}
                false => hit.push(l),
            }
        }
        let bridge = discourse::bridge_translation(self, f.resolved.word);
        let first_labels: Vec<String> = f
            .senses
            .iter()
            .find(|s| s.via != Source::KaikkiEn)
            .map(|s| {
                s.labels_en
                    .iter()
                    .filter(|l| l.chars().next().map(|c| c.is_lowercase()).unwrap_or(false))
                    .map(|l| l.replace('_', " "))
                    .take(2)
                    .collect()
            })
            .unwrap_or_default();
        // Përkthimi ndjek kuptimin që Nura e përkufizon: kur fjala shpjegohet nga leksiku i modelit,
        // merret përkthimi i tij që e dëshmon edhe Wiktionary-t («ankth» = anxiety, jo nightmare).
        let defined: Option<String> = f
            .senses
            .iter()
            .filter(|s| s.via == Source::ModelClaude && s.status == CandStatus::Open)
            .flat_map(|s| s.labels_en.iter().flat_map(|l| l.split([',', ';']).map(|x| x.trim().to_lowercase()).collect::<Vec<_>>()))
            .find(|m| {
                !m.is_empty()
                    && f.senses.iter().filter(|s| s.via == Source::KaikkiEn).any(|s| {
                        s.labels_en
                            .iter()
                            .chain(s.gloss_en.iter())
                            .any(|g| g.to_lowercase().split([',', ';', '(']).any(|x| x.trim() == m.as_str()))
                    })
            });
        let sq_open = f.senses.iter().any(|s| !matches!(s.via, Source::KaikkiEn | Source::ModelClaude) && s.status == CandStatus::Open);
        let bridge = match (sq_open, defined) {
            (false, Some(m)) => vec![m],
            _ => bridge,
        };
        // Ura e Wiktionary-t (kuptimi i parë i pjesës kryesore të ligjëratës) para fjalorit të
        // ConceptNet, ku një glosë me disa fjalë mund të mungojë («teacher, instructor»).
        let (items, src): (Vec<String>, Vec<Source>) = match (hit.is_empty(), bridge.is_empty(), dict.is_empty()) {
            (false, _, _) => (hit.into_iter().take(2).collect(), vec![Source::OmwAls, Source::ConceptNet57]),
            (true, false, _) => (bridge, vec![Source::KaikkiEn]),
            (true, true, false) => (dict.into_iter().take(1).collect(), vec![Source::ConceptNet57]),
            (true, true, true) => (first_labels, vec![Source::OmwAls, Source::Pwn30]),
        };
        let polarity = match items.is_empty() {
            true => Polarity::Neg,
            false => Polarity::Pos,
        };
        Clause {
            claim: Claim::Facet { word: word.clone(), facet: Facet::TranslationEn, items },
            polarity,
            attribution: Some(Attribution { sources: src, note: String::new() }),
        }
    }

    /// Zgjedhja e përdoruesit: kuptimi `sense` për fjalën `lemma` (përgjigje e iZ).
    pub fn choose_sense(&self, lemma: &str, sense_key: &str) -> Result<Analysis, String> {
        let mut f = self.focus(lemma);
        let sid = self.g.get(sense_key).ok_or("kuptimi nuk ekziston në graf")?;
        let idx = f
            .senses
            .iter()
            .position(|s| s.sense == sid)
            .ok_or("ky kuptim nuk i përket kësaj fjale sipas burimit")?;
        for (i, s) in f.senses.iter_mut().enumerate() {
            s.status = match i == idx {
                true => CandStatus::Selected("zgjedhur nga përdoruesi (përgjigje iZ)".into()),
                false => CandStatus::Eliminated("R-E4: përdoruesi zgjodhi një kuptim tjetër".into()),
            };
        }
        let s = f.senses[idx].clone();
        let total = f.senses.len();
        let (gloss, lang) = match (&s.gloss_sq, &s.gloss_en) {
            (Some(g), _) => (g.clone(), Lang::Sq),
            (None, Some(g)) => (g.clone(), Lang::En),
            (None, None) => (s.labels_en.join(", "), Lang::En),
        };
        let word = Self::sense_term(&discourse::term_of(self, f.resolved.word, &f.resolved.lemma), s.pos);
        let mut plan = ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: None,
            core: vec![],
            question: None,
        };
        let def_sq = match lang {
            Lang::Sq => s.gloss_sq.clone(),
            Lang::En => None,
        };
        let facts = discourse::gather(self, &f, s.sense, def_sq.as_deref());
        let budget = match lang {
            Lang::Sq => 2,
            Lang::En => 3,
        };
        let described = discourse::describe(&word, facts, &sense_sources(s.via), budget);
        // Përkufizimi anglisht thuhet vetëm kur s'ka asnjë fakt shqip për kuptimin e zgjedhur.
        match (lang, described.is_empty()) {
            (Lang::En, false) => {}
            _ => plan.core.push(Clause {
                claim: Claim::HasMeaning { word: word.clone(), gloss, gloss_lang: lang, index: idx + 1, of: total },
                polarity: Polarity::Pos,
                attribution: Some(Attribution { sources: sense_sources(s.via), note: format!("synset {}; zgjedhur nga përdoruesi", s.synset) }),
            }),
        }
        plan.core.extend(described);
        let steps = vec![
            ProcessStep { code: "CHOICE", label: format!("përdoruesi zgjodhi kuptimin {}", s.synset), nodes: vec![sid], edges: vec![], event: Some(Event::UserChoseSense) },
            ProcessStep { code: "VERIFY", label: "Y = 1: kuptimi i zgjedhur ekziston në burim".into(), nodes: vec![sid], edges: vec![], event: Some(Event::Verified { evidence_mass: 8_000 }) },
            ProcessStep { code: "SPEAK", label: "shprehja u ndërtua nga plani".into(), nodes: vec![], edges: vec![], event: Some(Event::Realized) },
        ];
        Ok(Analysis {
            input: format!("[zgjedhje] {lemma} → {}", s.synset),
            form: Form::Meaning { focus: lemma.into(), context: vec![] },
            focus: Some(f),
            other: None,
            path: vec![],
            cond_path: vec![],
            bridge_path: vec![],
            plan,
            steps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_covers_the_supported_grammar() {
        assert_eq!(parse("qen").unwrap(), Form::Explore { focus: "qen".into(), context: vec![] });
        assert_eq!(
            parse("A është qeni kafshë?").unwrap(),
            Form::IsA { claim: Simple { subj: "qeni".into(), obj: "kafshë".into(), polarity: Polarity::Pos }, asked: true }
        );
        assert_eq!(
            parse("Qeni nuk është kafshë.").unwrap(),
            Form::IsA { claim: Simple { subj: "qeni".into(), obj: "kafshë".into(), polarity: Polarity::Neg }, asked: false }
        );
        assert_eq!(parse("e kundërta e bardhë").unwrap(), Form::Opposite { focus: "bardhë".into() });
        assert_eq!(parse("ç'është zjarr").unwrap(), Form::Meaning { focus: "zjarr".into(), context: vec![] });
        assert!(matches!(
            parse("Nëse qeni është kafshë, atëherë qeni është organizëm").unwrap(),
            Form::Conditional { .. }
        ));
        assert!(parse("").is_err());
        assert!(parse("nëse qeni").is_err());
    }

    #[test]
    fn explore_keeps_context_words() {
        assert_eq!(
            parse("zjarr në pyll").unwrap(),
            Form::Explore { focus: "zjarr".into(), context: vec!["pyll".into()] }
        );
    }
}
