//! Shprehja gjuhësore nga grafi.
//!
//! Rrjedha: përmbajtja e grafit → `ExprPlan` (pohime me polaritet, kusht,
//! kontekst, atribuim) → realizim shqip.
//!
//! Ndarja e roleve (grafi emocional ≠ grafi gjuhësor):
//! - THELBI (`core`) ndërtohet vetëm nga plani: kush, çfarë marrëdhënie, me çfarë
//!   polariteti, nën çfarë kushti, në çfarë konteksti, sipas cilit burim.
//! - KORNIZA (`frame_open`/`frame_close`) vjen nga regjistri emocional dhe nuk
//!   prek thelbin. Testet provojnë se thelbi është bajt-për-bajt i njëjtë për
//!   çdo regjistër dhe se mohimi/kushti/konteksti ruhen.

use crate::emotion::Register;
use crate::graph::{Lang, Source};
use crate::morph::{capitalize, FormOrigin, Morph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Pos,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermRef {
    pub lemma: String,
    /// Etiketa e kuptimit të zgjedhur (për kontekstin), nëse ka.
    pub sense_gloss: Option<String>,
    /// Fjala citohet si është (mbiemër «i bukur», folje «vij», ndajfolje «shpejt»): nuk shquhet si emër.
    pub cite: bool,
}

impl TermRef {
    pub fn word(lemma: &str) -> Self {
        Self { lemma: lemma.to_string(), sense_gloss: None, cite: false }
    }
    pub fn cited(lemma: &str) -> Self {
        Self { lemma: lemma.to_string(), sense_gloss: None, cite: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// `subj` është (lloj i) `obj` — nga zinxhiri i hiperonimeve.
    IsA { subj: TermRef, obj: TermRef },
    /// E kundërta e `a` është `b`.
    Opposite { a: TermRef, b: TermRef },
    /// Fjala ka këtë kuptim (përkufizimi citohet nga burimi).
    HasMeaning { word: TermRef, gloss: String, gloss_lang: Lang, index: usize, of: usize },
    /// Fjala ka `open` kuptime kandidate (nga `total`) që burimi nuk i dallon dot pa kontekst.
    SenseCount { word: TermRef, open: usize, total: usize },
    /// Fjala nuk gjendet në këto burime.
    NotInSource { word: TermRef, sources: Vec<Source>, partial: bool },
    /// Burimet nuk e lidhin `subj` me `obj` në këtë marrëdhënie.
    NoLink { subj: TermRef, obj: TermRef, relation: &'static str },
    /// Marrëdhënie forme nga ConceptNet; gjuha e fjalës tjetër ruhet (p.sh. "en").
    Related { a: TermRef, b: TermRef, b_lang: String, relation: String },
    /// Fjala përdoret me këtë formë në një fjali UD.
    Usage { word: TermRef, form: String, sentence: String, sent_id: String, source: Source },
    /// Faset e përshkrimit (plani i tekstit nga grafi). `word` është fjala e fokusit.
    Facet { word: TermRef, facet: Facet, items: Vec<String> },
    /// Shpjegim "pse": zinxhiri i kategorive nga `subj` te `obj` (etiketa shqip).
    /// `conclude`: fjalia mbyllet me përfundimin ("prandaj X është Y").
    Chain { subj: TermRef, chain: Vec<String>, obj: TermRef, conclude: bool },
    /// Krahasim: kategoria e përbashkët dhe kategoritë dalluese.
    Compare { a: TermRef, b: TermRef, common: Option<String>, a_cat: Option<String>, b_cat: Option<String> },
    /// Fjali e ndërtuar nga Nura (kornizë + lidhjet që e mbajnë + e kundërta e saj, kur ka).
    Sentence { text: String, frame: &'static str, support: Vec<String>, counter: Option<String> },
    /// Përgjigje bisede: fjali të lidhura nga lidhjet e dëshmuara (portret, lidhje, shkak …).
    Discourse { text: String, intent: &'static str, links: Vec<String> },
}

/// Faset e një fjale që plani i tekstit mund t'i thotë.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facet {
    Category,
    Synonyms,
    Parts,
    PartOf,
    Kinds,
    Opposites,
    TranslationEn,
    Example,
    /// Prejardhja sipas rregullave të shqipes; items = fjali të gatshme nga shablloni i rregullit.
    Origin,
    /// Fjala shqipe për një huazim.
    Native,
    /// Trajta standarde e një varianti dialektor.
    Dialect,
    /// Pjesa e ligjëratës (dhe gjinia) sipas gramatikës; items = p.sh. ["emër", "femëror"].
    Grammar,
    /// Kuptimi krahinor i Veriut (fjalor i 1941), i thënë si i tillë; items = [shpjegimi].
    Regional,
}

impl Facet {
    pub fn code(self) -> &'static str {
        match self {
            Facet::Category => "category",
            Facet::Synonyms => "synonyms",
            Facet::Parts => "parts",
            Facet::PartOf => "part_of",
            Facet::Kinds => "kinds",
            Facet::Opposites => "opposites",
            Facet::TranslationEn => "translation_en",
            Facet::Example => "example",
            Facet::Origin => "origin",
            Facet::Native => "native",
            Facet::Dialect => "dialect",
            Facet::Grammar => "grammar",
            Facet::Regional => "regional",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribution {
    pub sources: Vec<Source>,
    /// Shënim faktik për metodën (p.sh. "zinxhir hiperonimesh 7 hapa").
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub claim: Claim,
    pub polarity: Polarity,
    pub attribution: Option<Attribution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Pohim i burimit, pa pohim të përdoruesit.
    Stated,
    /// Përgjigje pohuese ndaj pyetjes "a është X Y?" e mbështetur nga burimi.
    Answered,
    /// Pohimi i përdoruesit përputhet me burimin.
    UserClaimSupported,
    /// Pohimi i përdoruesit bie ndesh me burimin.
    UserClaimContradicted,
    /// Burimi nuk mjafton për të gjykuar (mungesë ≠ mohim).
    Unresolved,
}

impl Verdict {
    pub fn code(self) -> &'static str {
        match self {
            Verdict::Stated => "STATED",
            Verdict::Answered => "ANSWERED",
            Verdict::UserClaimSupported => "USER_CLAIM_SUPPORTED",
            Verdict::UserClaimContradicted => "USER_CLAIM_CONTRADICTED",
            Verdict::Unresolved => "UNRESOLVED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprPlan {
    /// Pohimi i përdoruesit, i ruajtur me polaritetin dhe kushtin e tij.
    pub user_claim: Option<Clause>,
    pub user_condition: Option<Clause>,
    pub verdict: Verdict,
    /// Kushti i pohimit të sistemit ("Nëse …").
    pub condition: Option<Clause>,
    pub core: Vec<Clause>,
    /// Pyetja iZ (pjesë e përmbajtjes, jo e tonit).
    pub question: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormNote {
    pub surface: String,
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Realization {
    pub register: Register,
    pub frame_open: String,
    pub core_text: String,
    pub frame_close: String,
    pub question: Option<String>,
    pub full: String,
    pub forms: Vec<FormNote>,
    /// Burimet dhe metoda, të ndara nga fjalia.
    pub sources: String,
}

/// Nënshkrimi semantik: ajo që toni nuk lejohet ta ndryshojë.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub claims: Vec<(String, Polarity)>,
    pub condition: Option<(String, Polarity)>,
    pub user_claim: Option<(String, Polarity)>,
    pub verdict: Verdict,
    pub contexts: Vec<String>,
}

fn claim_key(c: &Claim) -> String {
    match c {
        Claim::IsA { subj, obj } => format!("isa({},{})", subj.lemma, obj.lemma),
        Claim::Opposite { a, b } => format!("opp({},{})", a.lemma, b.lemma),
        Claim::HasMeaning { word, index, .. } => format!("meaning({},{index})", word.lemma),
        Claim::SenseCount { word, open, total } => format!("senses({},{open}/{total})", word.lemma),
        Claim::NotInSource { word, sources, .. } => {
            format!("absent({},{})", word.lemma, sources.iter().map(|s| s.code()).collect::<Vec<_>>().join("+"))
        }
        Claim::NoLink { subj, obj, relation } => format!("nolink({},{},{relation})", subj.lemma, obj.lemma),
        Claim::Related { a, b, b_lang, relation } => format!("rel({},{},{b_lang},{relation})", a.lemma, b.lemma),
        Claim::Usage { word, sent_id, .. } => format!("usage({},{sent_id})", word.lemma),
        Claim::Facet { word, facet, items } => format!("facet({},{},{})", word.lemma, facet.code(), items.join("|")),
        Claim::Sentence { text, frame, .. } => format!("sentence({frame},{text})"),
        Claim::Discourse { text, intent, .. } => format!("discourse({intent},{text})"),
        Claim::Chain { subj, chain, obj, .. } => format!("chain({},{},{})", subj.lemma, chain.join(">"), obj.lemma),
        Claim::Compare { a, b, common, .. } => {
            format!("compare({},{},{})", a.lemma, b.lemma, common.clone().unwrap_or_default())
        }
    }
}

fn claim_context(c: &Claim) -> Option<String> {
    match c {
        Claim::IsA { subj, .. } => subj.sense_gloss.clone(),
        Claim::HasMeaning { gloss, .. } => Some(gloss.clone()),
        _ => None,
    }
}

pub fn signature(p: &ExprPlan) -> Signature {
    Signature {
        claims: p.core.iter().map(|c| (claim_key(&c.claim), c.polarity)).collect(),
        condition: p.condition.as_ref().map(|c| (claim_key(&c.claim), c.polarity)),
        user_claim: p.user_claim.as_ref().map(|c| (claim_key(&c.claim), c.polarity)),
        verdict: p.verdict,
        contexts: p.core.iter().filter_map(|c| claim_context(&c.claim)).collect(),
    }
}

fn source_list(s: &[Source]) -> String {
    let names: Vec<&str> = s.iter().map(|x| x.label()).collect();
    match names.len() {
        0 => "asnjë burim".into(),
        1 => names[0].to_string(),
        n => format!("{} dhe {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

/// Emri shqip i gjuhës nga kodi i ConceptNet.
pub fn lang_name(code: &str) -> String {
    match code {
        "sq" => "shqip".into(),
        "en" => "anglisht".into(),
        "fr" => "frëngjisht".into(),
        "it" => "italisht".into(),
        "de" => "gjermanisht".into(),
        "es" => "spanjisht".into(),
        "pt" => "portugalisht".into(),
        "el" | "grc" => "greqisht".into(),
        "tr" => "turqisht".into(),
        "ru" => "rusisht".into(),
        "la" => "latinisht".into(),
        "mul" => "shumëgjuhësh".into(),
        "sr" | "hr" | "sh" | "bs" => "serbo-kroatisht".into(),
        "ro" => "rumanisht".into(),
        "bg" => "bullgarisht".into(),
        "mk" => "maqedonisht".into(),
        other => format!("gjuha «{other}»"),
    }
}

/// Gjuha ka emër shqip (përndryshe barazvlerësi nuk thuhet: «në gjuha «eo»» nuk është shqip).
pub fn known_lang(code: &str) -> bool {
    !matches!(code, "sq" | "mul") && !lang_name(code).starts_with("gjuha ")
}

/// Lista natyrale: "a", "a dhe b", "a, b dhe c".
fn join_natural(items: &[String], last: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} {last} {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

struct Ctx<'a> {
    morph: &'a Morph,
    forms: Vec<FormNote>,
}

impl Ctx<'_> {
    fn subject(&mut self, lemma: &str) -> String {
        let r = self.morph.definite_nom_sg(lemma);
        self.forms.push(FormNote { surface: r.text.clone(), origin: r.origin.label_sq() });
        r.text
    }
    /// Kryefjala e një termi: emri shquhet (qen → qeni); mbiemri, folja dhe ndajfolja citohen.
    fn subject_of(&mut self, t: &TermRef) -> String {
        match t.cite {
            true => self.cited(&t.lemma),
            false => self.subject(&t.lemma),
        }
    }
    fn cited(&mut self, lemma: &str) -> String {
        self.forms.push(FormNote { surface: format!("«{lemma}»"), origin: FormOrigin::Citation.label_sq() });
        format!("«{lemma}»")
    }
    fn predicate_noun(&mut self, lemma: &str) -> String {
        // Emri kallëzues pas "është" qëndron në emëroren e pashquar = forma e lemës.
        match lemma.contains(' ') {
            true => self.cited(lemma),
            false => {
                self.forms.push(FormNote {
                    surface: lemma.to_string(),
                    origin: "emërore e pashquar = forma e lemës (rregulli R-P1)".into(),
                });
                lemma.to_string()
            }
        }
    }
}

/// Heq kryefjalën e përsëritur në fillim të përkufizimit: «Macja është …», «Dora - është …»,
/// dhe një «është» të zhveshur në fillim («është një kafshë gjitare»).
pub fn strip_subject(lemma: &str, g: &str) -> String {
    let low = g.trim().to_lowercase();
    let first = low.split_whitespace().next().unwrap_or("");
    // Prefiksi i përbashkët (≥ 3 shkronja) mes lemës dhe fjalës së parë: mace/macja, dorë/dora.
    let common = lemma.to_lowercase().chars().zip(first.chars()).take_while(|(a, b)| a == b).count();
    let rest = g.trim().splitn(2, char::is_whitespace).nth(1).unwrap_or("").trim_start();
    let rest = rest.trim_start_matches(['-', '–', ':', ' ']);
    // «Një spital është një institucion …» → «një institucion …».
    let toks: Vec<&str> = low.split_whitespace().collect();
    let nje_x = match toks.as_slice() {
        ["një", x, "është", ..] => lemma.to_lowercase().chars().zip(x.chars()).take_while(|(a, b)| a == b).count() >= 3,
        _ => false,
    };
    let after_nje = g.trim().splitn(4, char::is_whitespace).nth(3).unwrap_or("").trim_start().to_string();
    match (common >= 3, rest.strip_prefix("është "), g.trim().strip_prefix("është "), nje_x) {
        (_, _, _, true) => after_nje,
        (true, Some(r), _, _) => r.to_string(),
        (_, _, Some(r), _) => r.to_string(),
        _ => g.trim().to_string(),
    }
}

/// Përkufizimi nis me vetë fjalën (trajtë e saj, ≥3 shkronja të përbashkëta) dhe vazhdon me
/// një folje tjetër, jo me «është»: «Syri i takon grupit të organeve shqisore …».
/// Teksti nis me një trajtë të vetë fjalës («mishi …» për «mish»).
fn starts_with_word(lemma: &str, g: &str) -> bool {
    let low = g.trim().to_lowercase();
    let first = low.split_whitespace().next().unwrap_or("");
    let n = lemma.chars().count();
    let common = lemma.to_lowercase().chars().zip(first.chars()).take_while(|(a, b)| a == b).count();
    common >= n.min(3) && common >= 2 && first.chars().count() >= n
}

fn opens_with_subject(lemma: &str, g: &str) -> bool {
    let low = g.trim().to_lowercase();
    let mut words = low.split_whitespace();
    let first = words.next().unwrap_or("");
    let second = words.next().unwrap_or("");
    // Duhet një folje e trajtuar pak më tej («Syri i takon …»); ndryshe është frazë emërore
    // («Mishi i kafshëve i përdorur si ushqim») dhe thuhet me «do të thotë:».
    let finite = low.split_whitespace().skip(1).take(4).any(|w| {
        matches!(
            w,
            "takon" | "quhet" | "quhen" | "përbëhet" | "përbëhen" | "ka" | "kanë" | "përdoret" | "përdoren" | "shërben"
                | "bën" | "jeton" | "rritet" | "ndodhet" | "gjendet"
        )
    });
    let common = lemma.to_lowercase().chars().zip(first.chars()).take_while(|(a, b)| a == b).count();
    let n = lemma.chars().count();
    common >= n.min(3) && common >= 2 && first.chars().count() >= n && finite && !matches!(second, "është" | "-" | "–" | ":" | "" | "janë")
}

/// Përkufizimi i burimit për t'u thënë: vetëm kuptimi i parë (deri te «;» i parë), pa pikë në
/// fund, me shkronjë të vogël në fillim, pa hapësira para shenjave dhe pa tekst të ngjitur
/// («heksadecimaleE bardha …» → pritet te kufiri i fjalisë së ngjitur).
fn clean_gloss(g: &str) -> String {
    let first = g.split(';').next().unwrap_or("").replace(" ,", ",").replace(" .", ".");
    let chars: Vec<char> = first.chars().collect();
    let glued = chars.windows(2).position(|w| w[0].is_lowercase() && w[1].is_uppercase());
    let cut: String = match glued {
        Some(i) => chars[..=i].iter().collect(),
        None => first.clone(),
    };
    let t = cut.trim().trim_end_matches(['.', ';', ',', ' ']);
    let mut c = t.chars();
    match c.next() {
        Some(f) => format!("{}{}", f.to_lowercase(), c.as_str()),
        None => String::new(),
    }
}

/// Konteksti i kuptimit është i lexueshëm vetëm kur është përkufizim shqip.
fn readable_context(g: &Option<String>) -> Option<&str> {
    match g {
        Some(s) => match (s.starts_with("synset "), s.strip_prefix("është ")) {
            (true, _) => None,
            (false, Some(r)) => Some(r),
            (false, None) => Some(s.as_str()),
        },
        None => None,
    }
}

/// Fjali e vetme e lexueshme për një pohim. `prev_lemma` lejon lidhje të natyrshme
/// kur fjalia e mëparshme foli për të njëjtën fjalë.
fn clause_sentence(c: &Clause, cx: &mut Ctx, prev_meaning_of: Option<&str>) -> String {
    let neg = match c.polarity {
        Polarity::Pos => "",
        Polarity::Neg => "nuk ",
    };
    match &c.claim {
        Claim::IsA { subj, obj } => match (prev_meaning_of == Some(subj.lemma.as_str()), c.polarity) {
            (true, Polarity::Pos) => {
                let o = cx.predicate_noun(&obj.lemma);
                format!("Është {o}")
            }
            _ => {
                let s = cx.subject_of(subj);
                let ctx = match readable_context(&subj.sense_gloss) {
                    Some(g) => format!(", në kuptimin «{g}»,"),
                    None => String::new(),
                };
                let o = cx.predicate_noun(&obj.lemma);
                capitalize(&format!("{s}{ctx} {neg}është {o}"))
            }
        },
        Claim::Opposite { a, b } => {
            let (x, y) = (cx.cited(&a.lemma), cx.cited(&b.lemma));
            format!("E kundërta e {x} {neg}është {y}")
        }
        Claim::HasMeaning { word, gloss, gloss_lang, .. } => match (gloss_lang, c.polarity, opens_with_subject(&word.lemma, gloss) && !word.cite) {
            // Përkufizimi nis vetë me fjalën dhe një folje tjetër («Syri i takon grupit …»):
            // thuhet ashtu siç është, pa «X është».
            (Lang::Sq, Polarity::Pos, true) => capitalize(&clean_gloss(gloss)),
            (Lang::Sq, Polarity::Pos, false) => {
                let g = clean_gloss(&strip_subject(&word.lemma, gloss));
                // Përkufizimi nis me vetë fjalën pa folje («mishi i kafshëve i përdorur si ushqim»):
                // «X është mishi …» do të ishte tautologji, prandaj «X do të thotë: …».
                let repeats = starts_with_word(&word.lemma, &g);
                match g.starts_with("të ") || g.starts_with("me ") || g.starts_with("t'") || word.lemma.contains(' ') || word.cite || repeats {
                    true => {
                        let w = cx.cited(&word.lemma);
                        format!("{} do të thotë: {g}", capitalize(&w))
                    }
                    false => {
                        let s = cx.subject_of(word);
                        capitalize(&format!("{s} është {g}"))
                    }
                }
            }
            (Lang::En, Polarity::Pos, _) => {
                let w = cx.cited(&word.lemma);
                format!("{}: “{}” (në anglisht)", capitalize(&w), clean_gloss(gloss))
            }
            (_, Polarity::Neg, _) => {
                let w = cx.cited(&word.lemma);
                format!("{} nuk ka kuptimin «{}»", capitalize(&w), clean_gloss(gloss))
            }
        },
        Claim::SenseCount { word, total, .. } => {
            let w = cx.cited(&word.lemma);
            let _ = total;
            format!("Me {w} mund të kesh parasysh disa gjëra")
        }
        // Përgjigje e thjeshtë: pa emra burimesh dhe pa meta-fjalë për sistemin.
        Claim::NotInSource { word, sources, .. } => {
            let w = cx.cited(&word.lemma);
            match sources.first() {
                Some(Source::ConceptNet57) => format!("Për {w} {neg}di të kundërt"),
                _ => format!("Nuk e njoh ende fjalën {w}"),
            }
        }
        Claim::NoLink { subj, obj, relation } => {
            let (x, y) = (cx.cited(&subj.lemma), cx.cited(&obj.lemma));
            let why = c.attribution.as_ref().map(|a| a.note.clone()).unwrap_or_default();
            match (*relation, why.is_empty()) {
                ("fjali", false) => format!("Nuk ndërtoj dot fjali nga {x} dhe {y}: {why}"),
                ("fjali", true) => format!("Nuk ndërtoj dot fjali nga {x} dhe {y}"),
                _ => format!("Nuk e di nëse {x} është {y}"),
            }
        }
        Claim::Related { a, b, b_lang, relation } => {
            let x = cx.subject_of(a);
            related_sentence(&x, relation, &[(b.lemma.clone(), b_lang.clone())], c.polarity)
        }
        Claim::Facet { word, facet, items } => facet_sentence(word, *facet, items, prev_meaning_of, cx),
        Claim::Chain { subj, chain, obj, conclude } => {
            let s = cx.subject_of(subj);
            let mut links: Vec<String> = Vec::new();
            let mut steps: Vec<String> = chain.clone();
            steps.push(obj.lemma.clone());
            let first = cx.predicate_noun(&steps[0]);
            for w in steps.windows(2) {
                let (x, y) = (cx.predicate_noun(&w[0]), cx.predicate_noun(&w[1]));
                links.push(format!("çdo {x} është {y}"));
            }
            let o = cx.predicate_noun(&obj.lemma);
            let body = match (links.is_empty(), chain.is_empty()) {
                (_, true) => format!("Sepse {s} është drejtpërdrejt {o}"),
                (true, false) => format!("Sepse {s} është {first}"),
                (false, false) => format!("Sepse {s} është {first}, {}", join_natural(&links, "dhe")),
            };
            match (conclude, chain.is_empty()) {
                (true, false) => format!("{body}; prandaj {s} është {o}"),
                _ => body,
            }
        }
        Claim::Compare { a, b, common, a_cat, b_cat } => {
            let (x, y) = (cx.subject_of(a), cx.subject_of(b));
            let first = match common {
                Some(c) => {
                    let c = cx.predicate_noun(c);
                    format!("{} është {c}, ashtu si edhe {y}", capitalize(&x))
                }
                None => format!("Nuk gjej diçka të përbashkët mes «{}» dhe «{}»", a.lemma, b.lemma),
            };
            match (a_cat, b_cat, common.is_some()) {
                (Some(ca), Some(cb), _) => match ca == cb {
                    true => format!("{first}. Dallimin mes tyre nuk e di"),
                    false => {
                        let (ca, cb) = (cx.predicate_noun(ca), cx.predicate_noun(cb));
                        format!("{first}. Dallimi: {x} është {ca}, ndërsa {y} është {cb}")
                    }
                },
                (_, _, true) => format!("{first}. Dallimin mes tyre nuk e di"),
                _ => first,
            }
        }
        Claim::Usage { word, form, sentence, .. } => {
            let w = cx.cited(&word.lemma);
            format!("Përdoret si «{form}», p.sh.: «{sentence}» ({})", w)
        }
        Claim::Sentence { text, support, counter, .. } => {
            let link = match support.is_empty() {
                true => String::new(),
                false => format!(" Lidhja: {}.", support.join(" → ")),
            };
            let against = counter.as_ref().map(|c| format!(" Përkundër: «{c}»")).unwrap_or_default();
            format!("Fjalia: «{text}»{link}{against}")
        }
        Claim::Discourse { text, .. } => text.clone(),
    }
}

/// Fjali për një faset. Kur fjalia e mëparshme foli për të njëjtën fjalë,
/// kryefjala nënkuptohet (shqipja e lejon: "Është…", "Quhet…", "Ka…").
fn facet_sentence(word: &TermRef, facet: Facet, items: &[String], prev: Option<&str>, cx: &mut Ctx) -> String {
    let implied = prev == Some(word.lemma.as_str());
    let subj = match implied {
        true => String::new(),
        false => cx.subject_of(word),
    };
    let quoted: Vec<String> = items.iter().map(|i| format!("«{i}»")).collect();
    let nouns: Vec<String> = items.iter().map(|i| cx.predicate_noun(i)).collect();
    let lead = |verb: &str| match implied {
        true => capitalize(verb),
        false => format!("{} {verb}", capitalize(&subj)),
    };
    match (facet, items.is_empty()) {
        (_, true) => {
            let w = &word.lemma;
            match facet {
                Facet::Category => format!("Për «{w}» nuk di në çfarë kategorie hyn"),
                Facet::Synonyms => format!("Për «{w}» nuk di sinonim"),
                Facet::Parts => format!("Për «{w}» nuk di nga çfarë pjesësh përbëhet"),
                Facet::PartOf => format!("Për «{w}» nuk di pjesë e çfarë është"),
                Facet::Kinds => format!("Për «{w}» nuk di lloje"),
                Facet::Opposites => format!("Për «{w}» nuk di të kundërt"),
                Facet::TranslationEn => format!("Për «{w}» nuk di si quhet në anglisht"),
                Facet::Example => format!("Për «{w}» nuk kam shembull"),
                Facet::Origin => format!("Për «{w}» nuk di nga cila fjalë është formuar"),
                Facet::Native => format!("Për «{w}» nuk di një fjalë tjetër shqipe"),
                Facet::Dialect => format!("Për «{w}» nuk di trajtë standarde"),
                Facet::Grammar => format!("Për «{w}» nuk di pjesën e ligjëratës"),
                Facet::Regional => format!("Për «{w}» nuk di kuptim krahinor"),
            }
        }
        (Facet::Category, false) => format!("{} {}", lead("është"), join_natural(&nouns, "dhe")),
        (Facet::Synonyms, false) => format!("{} edhe {}", lead("quhet"), join_natural(&quoted, "ose")),
        (Facet::Parts, false) => format!("{} si pjesë {}", lead("ka"), join_natural(&nouns, "dhe")),
        (Facet::PartOf, false) => format!("{} pjesë e {}", lead("është"), join_natural(&quoted, "ose")),
        (Facet::Kinds, false) => format!("{} lloje si {}", lead("ka"), join_natural(&nouns, "dhe")),
        (Facet::Opposites, false) => format!("E kundërta e «{}» është {}", word.lemma, join_natural(&quoted, "ose")),
        // Kur fjalia nuk vjen pas përkufizimit, kryefjala thuhet (nuk nënkuptohet).
        (Facet::TranslationEn, false) => match implied {
            true => format!("Në anglisht quhet {}", join_natural(&quoted, "ose")),
            false => format!("{} në anglisht përkthehet {}", capitalize(&subj), join_natural(&quoted, "ose")),
        },
        (Facet::Example, false) => format!("Për shembull: «{}»", items[0].trim_end_matches('.')),
        (Facet::Origin, false) => items.iter().map(|i| capitalize(i.trim_end_matches('.'))).collect::<Vec<_>>().join(". "),
        (Facet::Native, false) => match implied {
            true => format!("Në shqip mund të thuhet {}", join_natural(&quoted, "ose")),
            false => format!("Në vend të «{}», në shqip mund të thuhet {}", word.lemma, join_natural(&quoted, "ose")),
        },
        (Facet::Grammar, false) => format!("«{}» është {}", capitalize(&word.lemma), items.join(" ")),
        // Fjalë e rrallë e Veriut: shpjegimi thuhet si i tillë, me ortografinë e autorit (1941).
        // Pas kuptimit të zakonshëm të së njëjtës fjalë thuhet si shtesë.
        (Facet::Regional, false) => {
            let gloss = items[0].trim_end_matches(['.', ':', ',']);
            match implied {
                false => format!("Në të folmet e Veriut (fjalë e rrallë, 1941), «{}» do të thotë: {gloss}", word.lemma),
                true => format!("Në të folmet e Veriut (fjalë e rrallë, 1941), «{}» thuhej edhe për: {gloss}", word.lemma),
            }
        }
        (Facet::Dialect, false) => match implied {
            true => format!("Është trajtë dialektore e {}", join_natural(&quoted, "ose")),
            false => format!("{} është trajtë dialektore e {}", capitalize(&subj), join_natural(&quoted, "ose")),
        },
    }
}

/// Fjali natyrale për marrëdhëniet e formës nga ConceptNet. Fjalët shqip dhe ato
/// në gjuhë të tjera thuhen veçmas, me emrin e gjuhës — kurrë si shqip.
fn related_sentence(word: &str, relation: &str, items: &[(String, String)], pol: Polarity) -> String {
    let w = capitalize(word);
    let sq: Vec<String> = items.iter().filter(|(_, l)| l == "sq").map(|(x, _)| format!("«{x}»")).collect();
    // Përkthimet: "në anglisht «flag», në frëngjisht «drapeau»".
    let mut langs: Vec<String> = Vec::new();
    for (_, l) in items.iter().filter(|(_, l)| l != "sq") {
        match langs.contains(l) {
            true => {}
            false => langs.push(l.clone()),
        }
    }
    // Anglishtja e para (gjuha e urës); të tjerat në rendin e burimit.
    langs.sort_by_key(|l| l != "en");
    let other: Vec<String> = langs
        .iter()
        .map(|l| {
            let ws: Vec<String> = items.iter().filter(|(_, x)| x == l).map(|(x, _)| format!("«{x}»")).collect();
            format!("në {} {}", lang_name(l), join_natural(&ws, "ose"))
        })
        .collect();
    let all: Vec<String> = items.iter().map(|(x, _)| format!("«{x}»")).collect();
    let kind = relation.split(" (").last().unwrap_or("").trim_end_matches(')');
    match (pol, kind) {
        (Polarity::Neg, _) => format!("{w} nuk lidhet me {}", join_natural(&all, "dhe")),
        (Polarity::Pos, "Synonym") => {
            let a = match sq.is_empty() {
                true => String::new(),
                false => format!("{w} ka si sinonim {}", join_natural(&sq, "dhe")),
            };
            let b = other.join(", ");
            match (a.is_empty(), b.is_empty()) {
                (false, false) => format!("{a}. {}", capitalize(&b)),
                (false, true) => a,
                _ => format!("{w} thuhet {b}"),
            }
        }
        (Polarity::Pos, "IsA") => format!("{w} është {}", join_natural(&all, "ose")),
        (Polarity::Pos, "IsA, drejtim i kundërt") => format!("{w} ka lloje si {}", join_natural(&all, "dhe")),
        (Polarity::Pos, "FormOf") => format!("{w} është formë e {}", join_natural(&all, "ose")),
        (Polarity::Pos, "DerivedFrom") => format!("{w} rrjedh nga {}", join_natural(&all, "dhe")),
        (Polarity::Pos, _) => format!("{w} lidhet me {}", join_natural(&all, "dhe")),
    }
}

/// Bashkon pohime të njëpasnjëshme të të njëjtit lloj në një fjali (p.sh. disa antonime).
fn aggregate(clauses: &[Clause], cx: &mut Ctx, start: usize) -> (String, usize) {
    let first = &clauses[start];
    let run_len = clauses[start..]
        .iter()
        .take_while(|c| match (&c.claim, &first.claim, c.polarity == first.polarity) {
            (Claim::Opposite { a, .. }, Claim::Opposite { a: a0, .. }, true) => a.lemma == a0.lemma,
            (Claim::Related { a, relation, .. }, Claim::Related { a: a0, relation: r0, .. }, true) => {
                a.lemma == a0.lemma && relation == r0
            }
            _ => false,
        })
        .count();
    match (run_len > 1, &first.claim) {
        (true, Claim::Opposite { a, .. }) => {
            let x = cx.cited(&a.lemma);
            let items: Vec<String> = clauses[start..start + run_len]
                .iter()
                .filter_map(|c| match &c.claim {
                    Claim::Opposite { b, .. } => Some(cx.cited(&b.lemma)),
                    _ => None,
                })
                .collect();
            (format!("E kundërta e {x} është {}", join_natural(&items, "ose")), run_len)
        }
        (true, Claim::Related { a, relation, .. }) => {
            let x = cx.subject_of(a);
            let items: Vec<(String, String)> = clauses[start..start + run_len]
                .iter()
                .filter_map(|c| match &c.claim {
                    Claim::Related { b, b_lang, .. } => Some((b.lemma.clone(), b_lang.clone())),
                    _ => None,
                })
                .collect();
            (related_sentence(&x, relation, &items, first.polarity), run_len)
        }
        _ => (String::new(), 0),
    }
}

/// Thelbi i pavarur nga toni: fjali të qarta shqip. Burimet shkojnë te `sources_text`.
pub fn realize_core(p: &ExprPlan, morph: &Morph) -> (String, Vec<FormNote>) {
    let mut cx = Ctx { morph, forms: Vec::new() };
    let mut sentences: Vec<String> = Vec::new();
    let mut skip_first = false;
    // 1. Përgjigjja ndaj pohimit ose pyetjes së përdoruesit.
    match (&p.user_claim, p.verdict, p.core.first()) {
        (Some(uc), Verdict::UserClaimSupported, Some(c0)) => {
            let body = clause_sentence(c0, &mut cx, None);
            let cond = match &p.condition {
                Some(cc) => format!("nëse {}, atëherë ", lower_first(&clause_sentence(cc, &mut cx, None))),
                None => String::new(),
            };
            let _ = uc;
            sentences.push(format!("Po, {cond}{}", lower_first(&body)));
            skip_first = true;
        }
        (Some(uc), Verdict::UserClaimContradicted, Some(c0)) => {
            let said = lower_first(&clause_sentence(uc, &mut cx, None));
            let body = clause_sentence(c0, &mut cx, None);
            let _ = said;
            sentences.push(format!("Jo, {}", lower_first(&body)));
            skip_first = true;
        }
        (Some(uc), _, _) => {
            let said = lower_first(&clause_sentence(uc, &mut cx, None));
            let cond = match &p.user_condition {
                Some(cc) => format!("nëse {}, atëherë ", lower_first(&clause_sentence(cc, &mut cx, None))),
                None => String::new(),
            };
            let _ = (cond, said);
        }
        (None, Verdict::Answered, Some(c0)) => {
            let body = clause_sentence(c0, &mut cx, None);
            let cond = match &p.condition {
                Some(cc) => format!("nëse {}, atëherë ", lower_first(&clause_sentence(cc, &mut cx, None))),
                None => String::new(),
            };
            sentences.push(format!("Po, {cond}{}", lower_first(&body)));
            skip_first = true;
        }
        _ => {}
    }
    // 2. Pohimet e tjera të planit, me kushtin e ruajtur.
    let mut i = match skip_first {
        true => 1,
        false => 0,
    };
    let mut prev_meaning: Option<String> = None;
    // Kur ka përgjigje me përmbajtje, shënimet e mungesës dhe shembujt e përdorimit
    // nuk thuhen: Nura jep përgjigjen, jo procesin.
    let has_content = p
        .core
        .iter()
        .any(|c| !matches!(c.claim, Claim::NotInSource { .. } | Claim::Usage { .. }));
    while i < p.core.len() {
        let quiet = match (&p.core[i].claim, has_content) {
            (Claim::Usage { .. }, true) => true,
            (Claim::NotInSource { sources, .. }, true) => sources.first() != Some(&Source::ConceptNet57),
            _ => false,
        };
        match quiet {
            true => {
                i += 1;
                continue;
            }
            false => {}
        }
        let (agg, used) = aggregate(&p.core, &mut cx, i);
        match used {
            0 => {
                let c = &p.core[i];
                // Kuptimi krahinor thuhet si shtesë kur kuptimi i zakonshëm i fjalës u tha më parë
                // në të njëjtën përgjigje, edhe nëse ndërmjet tyre ra prejardhja.
                let said_before = |lemma: &str| {
                    p.core[..i].iter().any(|x| match &x.claim {
                        Claim::HasMeaning { word, .. } => word.lemma == lemma,
                        _ => false,
                    })
                };
                let prev_arg = match &c.claim {
                    Claim::Facet { facet: Facet::Regional, word, .. } => match said_before(&word.lemma) {
                        true => Some(word.lemma.clone()),
                        false => prev_meaning.clone(),
                    },
                    _ => prev_meaning.clone(),
                };
                let s = clause_sentence(c, &mut cx, prev_arg.as_deref());
                let s = match (i == 0, &p.condition) {
                    (true, Some(cc)) => {
                        format!("Nëse {}, atëherë {}", lower_first(&clause_sentence(cc, &mut cx, None)), lower_first(&s))
                    }
                    _ => s,
                };
                prev_meaning = match &c.claim {
                    // Pas «E kundërta e X është Y», «Në shqip mund të thuhet …», shembullit dhe
                    // prejardhjes, kryefjala nuk nënkuptohet: fjalia tjetër e thotë përsëri.
                    Claim::Facet { facet: Facet::Opposites | Facet::Native | Facet::Example | Facet::Origin | Facet::Regional, .. } => None,
                    Claim::HasMeaning { word, .. } | Claim::Facet { word, .. } => Some(word.lemma.clone()),
                    Claim::IsA { subj, .. } => match prev_meaning.as_deref() == Some(subj.lemma.as_str()) {
                        true => prev_meaning.clone(),
                        false => None,
                    },
                    _ => None,
                };
                sentences.push(s);
                i += 1;
            }
            n => {
                sentences.push(agg);
                prev_meaning = None;
                i += n;
            }
        }
    }
    let mut forms: Vec<FormNote> = Vec::new();
    for f in cx.forms {
        match forms.contains(&f) {
            true => {}
            false => forms.push(f),
        }
    }
    let text = sentences
        .iter()
        .map(|s| s.trim_end_matches(['.', ':']).to_string())
        .collect::<Vec<_>>()
        .join(". ");
    (text, forms)
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some('«') => s.to_string(),
        Some(f) => format!("{}{}", f.to_lowercase(), c.as_str()),
        None => String::new(),
    }
}

/// Burimet dhe metoda, të ndara nga fjalia: "Burimi: …". Nuk varen nga toni.
pub fn sources_text(p: &ExprPlan) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in p.core.iter().chain(p.condition.iter()) {
        match &c.attribution {
            Some(a) => {
                let s = match a.note.is_empty() {
                    true => source_list(&a.sources),
                    false => format!("{} — {}", source_list(&a.sources), a.note),
                };
                match parts.contains(&s) {
                    true => {}
                    false => parts.push(s),
                }
            }
            None => {}
        }
    }
    match parts.is_empty() {
        true => String::new(),
        false => format!("Burimi: {}", parts.join(" · ")),
    }
}

/// Korniza sipas regjistrit — nuk përmban pohime, mohime apo kushte.
pub fn frame(register: Register) -> (&'static str, &'static str) {
    match register {
        Register::Neutral => ("", "."),
        Register::Assertive => ("", "."),
        // Toni shprehet përmes animacionit të ndërfaqes, jo me fjalë shtesë.
        Register::Measured => ("", "."),
        Register::Questioning => ("", "."),
        Register::Witnessing => ("", "."),
        Register::Corrective => ("", "."),
        Register::Technical => ("[teknik] ", "."),
    }
}

pub fn realize(p: &ExprPlan, morph: &Morph, register: Register) -> Realization {
    let (core, forms) = realize_core(p, morph);
    let (open, close) = frame(register);
    let full = match &p.question {
        Some(q) => format!("{open}{core}{close} {q}"),
        None => format!("{open}{core}{close}"),
    };
    Realization {
        register,
        frame_open: open.to_string(),
        core_text: core,
        frame_close: close.to_string(),
        question: p.question.clone(),
        full,
        forms,
        sources: sources_text(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_isa(pol: Polarity, cond: bool) -> ExprPlan {
        let subj = TermRef { lemma: "qen".into(), sense_gloss: Some("mishngrënës me këmbë".into()), cite: false };
        ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: match cond {
                true => Some(Clause {
                    claim: Claim::IsA { subj: TermRef::word("qen"), obj: TermRef::word("kafshë") },
                    polarity: Polarity::Pos,
                    attribution: None,
                }),
                false => None,
            },
            core: vec![Clause {
                claim: Claim::IsA { subj, obj: TermRef::word("organizëm") },
                polarity: pol,
                attribution: Some(Attribution { sources: vec![Source::OmwAls, Source::Pwn30], note: "zinxhir 3 hapa".into() }),
            }],
            question: None,
        }
    }

    #[test]
    fn core_is_identical_across_all_registers() {
        let m = Morph::default();
        for p in [plan_isa(Polarity::Pos, false), plan_isa(Polarity::Neg, true)] {
            let base = realize(&p, &m, Register::Neutral);
            for r in Register::ALL {
                let x = realize(&p, &m, r);
                assert_eq!(x.core_text, base.core_text, "regjistri {r:?} ndryshoi thelbin");
                assert_eq!(x.sources, base.sources);
                assert!(x.full.contains(&base.core_text));
            }
        }
    }

    #[test]
    fn frames_never_carry_negation_or_condition() {
        for r in Register::ALL {
            let (o, c) = frame(r);
            for w in ["nuk", "nëse", "jo ", "s'", "po,"] {
                assert!(!o.to_lowercase().contains(w) && !c.to_lowercase().contains(w), "{r:?} korniza përmban {w}");
            }
        }
    }

    #[test]
    fn negation_condition_and_context_survive_realization() {
        let m = Morph::default();
        let x = realize(&plan_isa(Polarity::Neg, true), &m, Register::Assertive);
        assert!(x.core_text.starts_with("Nëse qeni është kafshë, atëherë qeni"), "{}", x.core_text);
        assert!(x.core_text.contains("nuk është organizëm"));
        assert!(x.core_text.contains("në kuptimin «mishngrënës me këmbë»"));
        assert!(x.sources.contains("Albanet / OMW dhe WordNet 3.0"));
    }

    #[test]
    fn sentence_is_plain_language_without_source_brackets() {
        let m = Morph::default();
        let x = realize(&plan_isa(Polarity::Pos, false), &m, Register::Neutral);
        assert_eq!(x.core_text, "Qeni, në kuptimin «mishngrënës me këmbë», është organizëm");
        assert!(!x.core_text.contains("sipas") && !x.core_text.contains("synset"));
        assert_eq!(x.sources, "Burimi: Albanet / OMW dhe WordNet 3.0 — zinxhir 3 hapa");
    }

    #[test]
    fn answered_question_starts_with_po() {
        let m = Morph::default();
        let mut p = plan_isa(Polarity::Pos, false);
        p.verdict = Verdict::Answered;
        p.core[0].claim = Claim::IsA {
            subj: TermRef { lemma: "qen".into(), sense_gloss: Some("synset 02083346-n të WordNet".into()), cite: false },
            obj: TermRef::word("kafshë"),
        };
        assert_eq!(realize(&p, &m, Register::Neutral).core_text, "Po, qeni është kafshë");
    }

    #[test]
    fn user_claim_polarity_is_preserved_in_contradiction() {
        let m = Morph::default();
        let p = ExprPlan {
            user_claim: Some(Clause {
                claim: Claim::IsA { subj: TermRef::word("qen"), obj: TermRef::word("kafshë") },
                polarity: Polarity::Neg,
                attribution: None,
            }),
            user_condition: None,
            verdict: Verdict::UserClaimContradicted,
            condition: None,
            core: vec![Clause {
                claim: Claim::IsA { subj: TermRef::word("qen"), obj: TermRef::word("kafshë") },
                polarity: Polarity::Pos,
                attribution: None,
            }],
            question: None,
        };
        let x = realize(&p, &m, Register::Corrective);
        assert_eq!(x.core_text, "Jo, qeni është kafshë");
        assert_eq!(p.user_claim.as_ref().unwrap().polarity, Polarity::Neg, "mohimi i përdoruesit ruhet në plan");
    }

    #[test]
    fn several_opposites_become_one_sentence() {
        let m = Morph::default();
        let opp = |b: &str| Clause {
            claim: Claim::Opposite { a: TermRef::word("bardhë"), b: TermRef::word(b) },
            polarity: Polarity::Pos,
            attribution: None,
        };
        let p = ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: None,
            core: vec![opp("zezë"), opp("zi")],
            question: None,
        };
        assert_eq!(realize(&p, &m, Register::Neutral).core_text, "E kundërta e «bardhë» është «zezë» ose «zi»");
    }

    #[test]
    fn meaning_reads_as_a_definition() {
        let m = Morph::default();
        let p = ExprPlan {
            user_claim: None,
            user_condition: None,
            verdict: Verdict::Stated,
            condition: None,
            core: vec![Clause {
                claim: Claim::HasMeaning {
                    word: TermRef::word("zjarr"),
                    gloss: "një ngjarje ku diçka digjet".into(),
                    gloss_lang: Lang::Sq,
                    index: 2,
                    of: 3,
                },
                polarity: Polarity::Pos,
                attribution: None,
            }],
            question: None,
        };
        assert_eq!(realize(&p, &m, Register::Neutral).core_text, "Zjarri është një ngjarje ku diçka digjet");
    }

    #[test]
    fn forms_record_their_origin() {
        let m = Morph::default();
        let x = realize(&plan_isa(Polarity::Pos, false), &m, Register::Neutral);
        assert!(x.forms.iter().any(|f| f.surface == "qeni" && f.origin.contains("rregulli")));
    }
}
