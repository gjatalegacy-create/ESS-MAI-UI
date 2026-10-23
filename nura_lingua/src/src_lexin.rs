//! Importuesi i LEXIN suedisht–shqip (ISOF, CC BY 4.0) — vetëm ana shqipe.
//!
//! Lexon `raw/lexin/lexin-sq.tsv` (nxjerrja e `nura_fetch --only lexin`):
//!   kuptimi ⇥ pos ⇥ fusha(T/S/A/D/C/I) ⇥ nën-id ⇥ teksti shqip
//!
//! Rregullat e deklaruara:
//! - Tekstet ndahen në anëtarë sipas `,` dhe `;` jashtë kllapave; `;` ndan
//!   NËN-KUPTIME: anëtarët e dy segmenteve të ndryshme `;` të të njëjtit rresht
//!   nuk lidhen me njëri-tjetrin. Shënimet në kllapa hidhen, përveç shënuesve të
//!   nyjës së mbiemrit `(i,e)`, `(i)`, `(e)`, që ruhen si atribut `article` dhe
//!   sinjal POS (mbiemër). Në hyrjet foljore `(i)`/`(e)` është përemri i shkurtër
//!   (`(i) nënshtrohem`), jo nyjë: hidhet si shënim.
//! - Shënimi KUFIZUES në kllapa (`motër (në spital)`, `zemër (e frutit)`, `(fig.)`)
//!   e lidh anëtarin vetëm me një kuptim të ngushtë: anëtari mbetet fjalë, por NUK
//!   lidhet me SYNONYM/ANTONYM me grupin. Shënimet gramatikore (nyje, përemra të
//!   shkurtër, valencë: `(më)`, `(për diçka)`, `(me dikë)`) nuk kufizojnë.
//! - Nyja e vetme `i`/`e`/`të` nuk bëhet kurrë anëtar (`i,e rënë shpirtërisht`,
//!   `absolutist,- e`); përjashtim: `e` në hyrjet e fjalëve ndihmëse (klasa `-`),
//!   ku është lidhëza «dhe» (`e, dhe, edhe`). `i,e X` / `i/e X` = shënuesi `(i,e)`.
//! - Shkrimi me vijë të pjerrët: `pranuesh/ëm (i), -me (e)` → `pranueshëm`;
//!   `mbiem/ër-ri` → `mbiemër`; `kif/le-lja` → `kifle`. Mbaresat e trajtës së
//!   shquar të ngjitura me vizë (`fillestar-i`, `modesti-a`, `krah-u`) hiqen.
//!   Copat e gjinisë/mbaresës (`-e`, `-me`, `-ri`), edhe kur LEXIN harron presjen
//!   para tyre (`fuqishëm -me`), hidhen dhe ndajnë anëtarët. Forma femërore
//!   pas mbiemrit me `(i)` (`zoti (i), zonja (e)`, `kuq (i), kuqe (e)`) hidhet.
//! - Lema: me shkronja të vogla, pa nyjën e mbiemrit (`i amësht (i,e)` → `amësht`).
//!   Në hyrjet mbiemërore (klasa LEXIN `a`), `i/e/të` + një fjalë pa shënues
//!   (`e hapur`) jep lemën e zhveshur (`hapur`); në hyrjet emërore (`e drejtë`)
//!   shprehja mbetet frazë (emër i substantivizuar).
//! - Shprehjet me shumë fjalë mbeten nyje-fraza (`Word` me lemë me hapësira),
//!   kurrë nuk copëtohen. Parafrazat më të gjata se `MAX_WORDS_SENSE` fjalë hidhen.
//! - Grupi Translation+Synonym i një kuptimi = një bashkësi shqipe me të njëjtin
//!   kuptim → `Sem("SYNONYM")` në të dy drejtimet, `sense = lexin:<id>`.
//!   Antonym → `Sem("ANTONYM")` ndërmjet anëtarëve të grupit dhe antonimit.
//!   Derivation: alternativat e të njëjtës fjalë të prejardhur → `SYNONYM`
//!   (`sense = lexin:<id>:d<nën-id>`) dhe `Sem("RELATED")` me grupin kryesor
//!   (`via = derivation`): LEXIN e boton prejardhjen nën hyrjen.
//! - Compound dhe Idiom NUK importohen: LEXIN i rendit sipas strukturës së fjalës
//!   SUEDISHTE, jo të shqipes, dhe brenda tyre presja nuk është ndarës i
//!   besueshëm listash. Numërohen në raport si të anashkaluara.
//! - RREGULLI I IMPORTIMIT: LEXIN importohet pas fjalorëve të tjerë. Një skaj
//!   pranohet vetëm kur çdo anëtar me NJË fjalë ekziston tashmë si nyje `Word`
//!   shqipe në graf. Nyjet-fraza krijohen vetëm kur lidhen me një fjalë të
//!   vetme që ekziston; dy fraza nuk lidhen kurrë drejtpërdrejt.
//! - Homografët: një mbiemër me shënues të një gjinie (`zo/ti (i)`, `pari (i)`)
//!   nuk lidhet me një nyje ekzistuese që në graf njihet VETËM si emër.
//! - `Lex` (vetë-lak) me `article=i|e` vetëm për fjalë ekzistuese, një për çdo
//!   (fjalë, nyje); `pos=a` kur shënuesi është `(i,e)` ose hyrja është mbiemër.
//! - LEXIN nuk jep forma të lakuara shqipe: `morph` nuk ndryshohet.

use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;
use std::collections::{BTreeMap, HashMap};

pub const TSV: &str = "raw/lexin/lexin-sq.tsv";

/// Kufiri i fjalëve për anëtarët e grupeve të kuptimit (T/S/A/D).
pub const MAX_WORDS_SENSE: usize = 4;

/// Mbaresa të trajtës së shquar që LEXIN ngjit me vizë pas lemës.
const DEF_SUFFIX: &[&str] = &["i", "u", "a", "ja", "ri", "mi", "ra", "e", "li", "lja", "ia", "ua", "ni"];

/// Një anëtar shqip i një grupi LEXIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub lemma: String,
    /// Nyja e mbiemrit nga shënuesi `(i,e)` / `(i)` / `(e)`.
    pub article: Option<&'static str>,
    /// Shënuesi `(i,e)`: mbiemër me dy gjini (sinjal POS = mbiemër). `(i)` ose `(e)`
    /// vetëm shënon edhe emra me nyjë (`ardhme (e)`), prandaj nuk jep POS.
    pub both_genders: bool,
    /// Shprehje me më shumë se një fjalë.
    pub phrase: bool,
    /// Sinjal mbiemri: shënues nyjeje dhe (`(i,e)` ose hyrje mbiemërore).
    pub adj: bool,
    /// Rreshti i burimit brenda grupit (T, S, …) — vendoset nga importuesi.
    pub row: usize,
    /// Segmenti `;` brenda tekstit (nën-kuptimi).
    pub seg: usize,
    /// Shënim kufizues në kllapa («motër (në spital)»): pa SYNONYM/ANTONYM me grupin.
    pub restricted: bool,
}

/// Arsyet pse një copë teksti nuk u bë anëtar, dhe korrigjimet e bëra.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Drops {
    /// Bosh pas heqjes së shënimeve në kllapa (p.sh. "(paraprakisht)").
    pub note_only: usize,
    /// Copë gjinie/mbaresë ("-e", "-me", "madh-").
    pub fragment: usize,
    /// Parafrazë më e gjatë se kufiri.
    pub long: usize,
    /// Shenja që nuk i përkasin një leme (thonjëza, shifra, pika, "/" pa shënues).
    pub other: usize,
    /// Korrigjim (jo hedhje): mbaresë e shquar e hequr ("fillestar-i" → "fillestar").
    pub suffix_fixed: usize,
    /// Korrigjim (jo hedhje): nyjë e hequr në hyrje mbiemërore ("e hapur" → "hapur").
    pub article_fixed: usize,
    /// Nyjë e vetme «i», «e», «të» si copë ("i,e rënë" pa bashkim, ", - e").
    pub bare_article: usize,
}

impl Drops {
    fn add(&mut self, o: Drops) {
        self.note_only += o.note_only;
        self.fragment += o.fragment;
        self.long += o.long;
        self.other += o.other;
        self.suffix_fixed += o.suffix_fixed;
        self.article_fixed += o.article_fixed;
        self.bare_article += o.bare_article;
    }
    /// Copat e hedhura (pa korrigjimet).
    pub fn total(&self) -> usize {
        self.note_only + self.fragment + self.long + self.other + self.bare_article
    }
}

/// Fjalët që në kllapa japin vetëm gramatikë (nyje, përemra të shkurtër, valencë:
/// «(i,e)», «(më)», «(për diçka)», «(me dikë)»), jo kufizim kuptimi.
const NOTE_GRAMMAR: &[&str] = &[
    "i", "e", "u", "më", "të", "na", "ju", "ua", "ia", "ta", "ma", "së", "t'i", "t'ia", "jua", "diçka", "diçkaje",
    "dikë", "dikujt", "dikush", "diku", "dikënd", "ndonjë", "ndokënd", "ndokujt", "për", "me", "në", "nga", "mbi",
    "te", "tek", "ndaj", "para", "pas", "kundër", "prej", "nën", "pa", "një", "bëj",
];

/// Shënim kufizues kuptimi në kllapa («në spital», «e frutit», «fig.»): ka të paktën
/// një fjalë jashtë `NOTE_GRAMMAR`.
fn restricting_note(p: &str) -> bool {
    let low = p.to_lowercase();
    let mut toks = low
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | ';'))
        .map(|t| t.trim_matches(|c: char| !c.is_alphabetic()))
        .filter(|t| !t.is_empty());
    toks.any(|t| !NOTE_GRAMMAR.contains(&t))
}

/// Shënuesi i nyjës sipas klasës së hyrjes: në hyrjet foljore `(i)`/`(e)` është
/// përemri i shkurtër (`(i) nënshtrohem` = «i nënshtrohem»), jo nyjë mbiemri.
fn marker_for(parens: &[String], entry_pos: Option<&str>) -> Option<(&'static str, bool)> {
    match (parens.iter().find_map(|p| article_marker(p)), entry_pos) {
        (Some((_, false)), Some("v")) => None,
        (m, _) => m,
    }
}

fn article_marker(p: &str) -> Option<(&'static str, bool)> {
    let q: String = p.chars().filter(|c| !c.is_whitespace()).collect();
    match q.as_str() {
        "i,e" | "i/e" => Some(("i", true)),
        "i" => Some(("i", false)),
        "e" => Some(("e", false)),
        _ => None,
    }
}

const BAD_CHARS: &[char] = &['"', '“', '”', '„', '«', '»', '.', '=', '?', '!', ':', '[', ']', '+', '&', '*', '…', '|', '<', '>', '/'];

/// "fillestar-i" → ("fillestar", true); "copa-copa", "c-dur" mbeten.
fn strip_def(tok: &str) -> (String, bool) {
    match tok.rsplit_once('-') {
        Some((base, suf)) => match (
            DEF_SUFFIX.contains(&suf),
            base.chars().count() >= 2 && base.chars().all(char::is_alphabetic),
        ) {
            (true, true) => (base.to_string(), true),
            _ => (tok.to_string(), false),
        },
        None => (tok.to_string(), false),
    }
}

/// Vija e pjerrët e LEXIN brenda një fjale të vetme: "mbiem/ër-ri", "anamnez/ë",
/// "kif/le-lja" — pjesa pas "/" (deri te "-") ka ≤ 2 shkronja. "dhe/ose", "ai/ajo" jo.
fn stem_slash(tok: &str) -> Option<String> {
    match tok.split_once('/') {
        Some((stem, rest)) => {
            let (tail, def) = match rest.split_once('-') {
                Some((t, d)) => (t, Some(d)),
                None => (rest, None),
            };
            let def_ok = def.map(|d| !d.is_empty() && d.chars().count() <= 4 && d.chars().all(char::is_alphabetic)).unwrap_or(true);
            let ok = !stem.is_empty()
                && !tail.is_empty()
                && tail.chars().count() <= 2
                && stem.chars().all(char::is_alphabetic)
                && tail.chars().all(char::is_alphabetic)
                && def_ok;
            match ok {
                true => Some(format!("{stem}{tail}")),
                false => None,
            }
        }
        None => None,
    }
}

/// Një copë fjalësh (pa kllapa, pa copa "-…") → anëtar ose arsye hedhjeje.
fn piece_member(
    toks_in: &[String],
    article: Option<&'static str>,
    both_genders: bool,
    entry_pos: Option<&str>,
    max_words: usize,
    fixes: &mut Drops,
) -> Result<Member, Drops> {
    let mut toks: Vec<String> = Vec::new();
    for t in toks_in {
        let (s, fixed) = strip_def(t);
        match fixed {
            true => fixes.suffix_fixed += 1,
            false => {}
        }
        toks.push(s);
    }
    let first_is_article = matches!(toks.first().map(String::as_str), Some("i" | "e" | "të"));
    // Nyja e vetme nuk është lemë; vetëm në hyrjet e fjalëve ndihmëse (klasa «-»)
    // «e» është lidhëza «dhe» («e, dhe, edhe», «e, së»).
    let lone_article = toks.len() == 1 && first_is_article;
    match (lone_article, toks.first().map(String::as_str), entry_pos) {
        (true, Some("e"), Some("-")) => {}
        (true, _, _) => return Err(Drops { bare_article: 1, ..Drops::default() }),
        (false, _, _) => {}
    }
    match (article.is_some(), first_is_article, toks.len(), entry_pos) {
        // Shënuesi (i,e) konfirmon nyjën e mbiemrit.
        (true, true, 2..=usize::MAX, _) => {
            toks.remove(0);
        }
        // Hyrje mbiemërore pa shënues: "e hapur" → "hapur".
        (false, true, 2, Some("a")) => {
            toks.remove(0);
            fixes.article_fixed += 1;
        }
        _ => {}
    }
    let lemma = toks.join(" ");
    let truncated = toks.iter().any(|t| t.ends_with('-'));
    let has_digit = lemma.chars().any(|c| c.is_ascii_digit());
    let has_bad = lemma.contains(BAD_CHARS);
    let has_alpha = lemma.chars().any(|c| c.is_alphabetic());
    match (lemma.is_empty(), truncated, has_digit || has_bad || !has_alpha, toks.len() > max_words) {
        (true, _, _, _) => Err(Drops::default()),
        (false, true, _, _) => Err(Drops { fragment: 1, ..Drops::default() }),
        (false, false, true, _) => Err(Drops { other: 1, ..Drops::default() }),
        (false, false, false, true) => Err(Drops { long: 1, ..Drops::default() }),
        (false, false, false, false) => Ok(Member {
            phrase: toks.len() > 1,
            lemma,
            article,
            both_genders,
            adj: article.is_some() && (both_genders || entry_pos == Some("a")),
            row: 0,
            seg: 0,
            restricted: false,
        }),
    }
}

/// Një segment teksti (jashtë kllapave) + përmbajtjet e kllapave → anëtarë.
fn segment_members(seg: &str, parens: &[String], entry_pos: Option<&str>, max_words: usize) -> (Vec<Member>, Drops) {
    let mut d = Drops::default();
    let marker = marker_for(parens, entry_pos);
    let article = marker.map(|(a, _)| a);
    let both_genders = marker.map(|(_, b)| b).unwrap_or(false);
    let restricted = parens.iter().any(|p| restricting_note(p));
    let lower = seg.to_lowercase();
    let trimmed = lower.trim().trim_end_matches(['.', ':', ',', ';']).trim().to_string();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    // Vija e pjerrët: me shënues nyjeje hiqet; në një fjalë të vetme me rrënjë+mbaresë hiqet.
    let words: Vec<String> = match (trimmed.contains('/'), article.is_some(), words.len()) {
        (false, _, _) => words.iter().map(|w| w.to_string()).collect(),
        (true, true, _) => words.iter().map(|w| w.replace('/', "")).collect(),
        (true, false, 1) => match stem_slash(words[0]) {
            Some(w) => vec![w],
            None => return (Vec::new(), Drops { other: 1, ..Drops::default() }),
        },
        (true, false, _) => return (Vec::new(), Drops { other: 1, ..Drops::default() }),
    };
    // Copat "-me", "-e", "-" ndajnë anëtarët (LEXIN ndonjëherë harron presjen).
    // Vizë e veçuar + mbaresë ("absolutist,- e", "nervoz, - e", "- me") = një copë e vetme.
    let mut pieces: Vec<Vec<String>> = vec![Vec::new()];
    let mut lone_dash = false;
    for w in words {
        let swallowed = lone_dash && (w == "me" || DEF_SUFFIX.contains(&w.as_str()));
        lone_dash = w == "-";
        match (swallowed, w.starts_with('-')) {
            (true, _) => {}
            (false, true) => {
                d.fragment += 1;
                pieces.push(Vec::new());
            }
            (false, false) => match pieces.last_mut() {
                Some(p) => p.push(w),
                None => {}
            },
        }
    }
    let mut out: Vec<Member> = Vec::new();
    let mut nonempty = 0usize;
    for (i, p) in pieces.iter().enumerate() {
        // Nyja e shënuesit i përket vetëm copës së parë.
        let (art, both) = match i {
            0 => (article, both_genders),
            _ => (None, false),
        };
        match p.is_empty() {
            true => {}
            false => {
                nonempty += 1;
                match piece_member(p, art, both, entry_pos, max_words, &mut d) {
                    Ok(mut m) => {
                        m.restricted = restricted;
                        out.push(m)
                    }
                    Err(x) => d.add(x),
                }
            }
        }
    }
    match (nonempty, d.fragment, parens.is_empty()) {
        (0, 0, false) => d.note_only += 1,
        _ => {}
    }
    (out, d)
}

/// "i,e rënë shpirtërisht": presja ndan nyjën «i» nga «e rënë …». Copa «i» e vetme
/// (pa kllapa) para copës që nis me «e» në të njëjtin segment `;` bashkohen në
/// shënuesin `(i,e)`; po ashtu «i/e X». Kështu «i» nuk bëhet anëtar dhe lema është «X».
fn fold_article_prefix(segs: Vec<(String, Vec<String>, usize)>) -> Vec<(String, Vec<String>, usize)> {
    let mut out: Vec<(String, Vec<String>, usize)> = Vec::new();
    for (text, mut ps, sx) in segs {
        let prev_lone_i = match out.last() {
            Some((t, p, s)) => t.trim().to_lowercase() == "i" && p.is_empty() && *s == sx,
            None => false,
        };
        let words: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
        let lead = words.first().cloned().unwrap_or_default();
        let fold = match (lead.as_str(), words.len() >= 2, prev_lone_i) {
            ("i/e", true, _) => Some(false),
            ("e", true, true) => Some(true),
            _ => None,
        };
        match fold {
            Some(pop_i) => {
                match pop_i {
                    true => {
                        out.pop();
                    }
                    false => {}
                }
                ps.push("i,e".to_string());
                out.push((words[1..].join(" "), ps, sx));
            }
            None => out.push((text, ps, sx)),
        }
    }
    out
}

/// Ndan tekstin shqip të LEXIN në anëtarë: `,` dhe `;` jashtë kllapave.
/// `entry_pos`: klasa e hyrjes LEXIN (n/v/a/r, ose «-» për fjalët ndihmëse) për rregullin e nyjës.
/// `seg` i anëtarit = numri i segmentit `;`.
pub fn split_members(text: &str, entry_pos: Option<&str>, max_words: usize) -> (Vec<Member>, Drops) {
    let mut segs: Vec<(String, Vec<String>, usize)> = Vec::new();
    let mut cur = String::new();
    let mut parens: Vec<String> = Vec::new();
    let mut pbuf = String::new();
    let mut depth = 0usize;
    let mut semi = 0usize;
    for ch in text.chars() {
        match (ch, depth) {
            ('(', 0) => {
                depth = 1;
                pbuf.clear();
            }
            ('(', _) => {
                depth += 1;
                pbuf.push(ch);
            }
            // Kllapë mbyllëse pa çelëse: injorohet.
            (')', 0) => {}
            (')', 1) => {
                depth = 0;
                parens.push(std::mem::take(&mut pbuf));
            }
            (')', _) => {
                depth -= 1;
                pbuf.push(ch);
            }
            (',', 0) => segs.push((std::mem::take(&mut cur), std::mem::take(&mut parens), semi)),
            (';', 0) => {
                segs.push((std::mem::take(&mut cur), std::mem::take(&mut parens), semi));
                semi += 1;
            }
            (_, 0) => cur.push(ch),
            (_, _) => pbuf.push(ch),
        }
    }
    segs.push((cur, parens, semi));
    let segs = fold_article_prefix(segs);
    let mut out: Vec<Member> = Vec::new();
    let mut drops = Drops::default();
    let mut prev: Option<(&'static str, bool)> = None;
    for (seg, ps, sx) in &segs {
        let marker = marker_for(ps, entry_pos);
        // "zoti (i), zonja (e)", "kuq (i), kuqe (e)": forma femërore e mbiemrit të mëparshëm.
        let feminine = matches!((prev, marker), (Some(("i", false)), Some(("e", false))));
        prev = marker;
        match feminine {
            true => {
                drops.fragment += 1;
                continue;
            }
            false => {}
        }
        let (ms, d) = segment_members(seg, ps, entry_pos, max_words);
        drops.add(d);
        for mut m in ms {
            m.seg = *sx;
            match out.iter().any(|x| x.lemma == m.lemma) {
                true => {}
                false => out.push(m),
            }
        }
    }
    (out, drops)
}

#[derive(Default)]
struct SenseRows {
    pos: String,
    /// (fusha T/S, teksti)
    main: Vec<(char, String)>,
    ant: Vec<String>,
    /// nën-id → tekstet e prejardhjes
    deriv: BTreeMap<String, Vec<String>>,
    /// rreshta Compound/Idiom (të paimportuar)
    comp_rows: usize,
    idiom_rows: usize,
}

#[derive(Debug, Default, Clone, Copy)]
struct GroupStat {
    total: usize,
    /// Me të paktën dy anëtarë të vlefshëm (ose grup + antonim).
    pairable: usize,
    /// Me të paktën një skaj të pranuar.
    linked: usize,
}

#[derive(Debug, Default)]
struct Counts {
    rows: usize,
    bad_rows: usize,
    senses: usize,
    main: GroupStat,
    ant: GroupStat,
    deriv: GroupStat,
    comp_rows: usize,
    idiom_rows: usize,
    pairs_ok: usize,
    skip_missing: usize,
    skip_phrase_pair: usize,
    skip_pos_conflict: usize,
    skip_subsense: usize,
    /// Çifte SYNONYM/ANTONYM të pamarra: një anëtar ka shënim kufizues në kllapa.
    skip_restricted: usize,
    /// Anëtarë me shënim kufizues (mbeten fjalë, pa SYNONYM/ANTONYM).
    restricted_members: usize,
    phrases_created: usize,
    edges_syn: usize,
    edges_ant: usize,
    edges_rel: usize,
    edges_lex: usize,
    drops: Drops,
}

/// Dëshmia POS e një nyjeje Word në graf: (mbiemër, emër) — nga kuptimet
/// (HasSense → Sense.pos), vetë-lakët Lex (`pos`) dhe konceptet ConceptNet.
fn pos_evidence(g: &Graph, id: u32) -> (bool, bool) {
    let mut adj = false;
    let mut noun = false;
    for (_, e) in g.out_edges(id) {
        let p = match &e.rel {
            Rel::HasSense => match &g.node(e.to).kind {
                NodeKind::Sense { pos, .. } => Some(*pos),
                _ => None,
            },
            Rel::Lex => e.attr("pos").and_then(Pos::from_wn),
            _ => None,
        };
        match p {
            Some(Pos::Adj) => adj = true,
            Some(Pos::Noun) => noun = true,
            _ => {}
        }
    }
    // ConceptNet: Concept(/c/sq/<lemë>/<pos>) → Word me FormOf.
    for (_, e) in g.in_edges(id) {
        let p = match (&e.rel, &g.node(e.from).kind) {
            (Rel::FormOf, NodeKind::Concept { uri }) => concept_pos(uri),
            _ => None,
        };
        match p {
            Some(Pos::Adj) => adj = true,
            Some(Pos::Noun) => noun = true,
            _ => {}
        }
    }
    (adj, noun)
}

/// Nyja ekzistuese e një anëtari me një fjalë. `Err(true)` = homograf: anëtari
/// është mbiemër me shënues të një gjinie vetëm (`zo/ti (i)`, `pari (i)` — trajta
/// e substantivizuar/rendore), kurse nyja njihet në graf vetëm si emër. Shënuesi
/// `(i,e)` (`fortë (i,e)`) e identifikon lemën e mbiemrit qartë dhe nuk bllokohet.
fn existing(g: &Graph, m: &Member) -> Result<u32, bool> {
    match g.get(&word_key(Lang::Sq, &m.lemma)) {
        Some(id) => match (m.adj && !m.both_genders, pos_evidence(g, id)) {
            (true, (false, true)) => Err(true),
            _ => Ok(id),
        },
        None => Err(false),
    }
}

fn phrase_node(g: &mut Graph, m: &Member, c: &mut Counts) -> u32 {
    match g.get(&word_key(Lang::Sq, &m.lemma)) {
        Some(id) => id,
        None => {
            c.phrases_created += 1;
            g.ensure_word(Lang::Sq, &m.lemma)
        }
    }
}

fn sem_edge(from: u32, to: u32, rel: &str, attrs: &[(String, String)]) -> Edge {
    Edge {
        from,
        to,
        rel: Rel::Sem(rel.to_string()),
        source: Source::Lexin,
        lang: Some(Lang::Sq),
        method: Method::Published,
        attrs: attrs.to_vec(),
    }
}

fn miss(c: &mut Counts, homograph: bool) {
    match homograph {
        true => c.skip_pos_conflict += 1,
        false => c.skip_missing += 1,
    }
}

/// Lidh dy anëtarë në të dy drejtimet sipas rregullit të importimit.
/// Kthen numrin e skajeve të reja (≥1 kur çifti pranohet).
fn link(g: &mut Graph, a: &Member, b: &Member, rel: &str, attrs: &[(String, String)], c: &mut Counts) -> usize {
    // «motër (në spital)» nuk është sinonim i përgjithshëm i «infermiere»: shënimi
    // kufizues e ndalon SYNONYM/ANTONYM (RELATED i prejardhjes mbetet).
    let restricted = (a.restricted || b.restricted) && matches!(rel, "SYNONYM" | "ANTONYM");
    let ends = match (a.lemma == b.lemma, restricted, a.phrase, b.phrase) {
        (true, _, _, _) => None,
        (false, true, _, _) => {
            c.skip_restricted += 1;
            None
        }
        (false, false, true, true) => {
            c.skip_phrase_pair += 1;
            None
        }
        (false, false, false, false) => match (existing(g, a), existing(g, b)) {
            (Ok(x), Ok(y)) => Some((x, y)),
            (Err(h), _) | (_, Err(h)) => {
                miss(c, h);
                None
            }
        },
        (false, false, true, false) => match existing(g, b) {
            Ok(y) => Some((phrase_node(g, a, c), y)),
            Err(h) => {
                miss(c, h);
                None
            }
        },
        (false, false, false, true) => match existing(g, a) {
            Ok(x) => Some((x, phrase_node(g, b, c))),
            Err(h) => {
                miss(c, h);
                None
            }
        },
    };
    match ends {
        Some((x, y)) => {
            c.pairs_ok += 1;
            let before = g.edges.len();
            g.add_edge(sem_edge(x, y, rel, attrs));
            g.add_edge(sem_edge(y, x, rel, attrs));
            let n = g.edges.len() - before;
            match rel {
                "SYNONYM" => c.edges_syn += n,
                "ANTONYM" => c.edges_ant += n,
                _ => c.edges_rel += n,
            }
            n.max(1)
        }
        None => 0,
    }
}

/// A janë dy anëtarë të të njëjtit nën-kuptim? Brenda një rreshti: i njëjti
/// segment `;`. Ndërmjet rreshtave: po, përveç kur rreshti T ka disa nën-kuptime.
fn same_subsense(a: &Member, b: &Member, multi_t: &[bool]) -> bool {
    let multi = |r: usize| multi_t.get(r).copied().unwrap_or(false);
    match a.row == b.row {
        true => a.seg == b.seg,
        false => !multi(a.row) && !multi(b.row),
    }
}

/// Lidh çiftet e të njëjtit nën-kuptim me SYNONYM; kthen numrin e çifteve të pranuara.
fn link_within(g: &mut Graph, ms: &[Member], multi_t: &[bool], attrs: &[(String, String)], c: &mut Counts) -> usize {
    let mut ok = 0;
    for i in 0..ms.len() {
        for j in (i + 1)..ms.len() {
            ok += match same_subsense(&ms[i], &ms[j], multi_t) {
                true => match link(g, &ms[i], &ms[j], "SYNONYM", attrs, c) {
                    0 => 0,
                    _ => 1,
                },
                false => {
                    c.skip_subsense += 1;
                    0
                }
            };
        }
    }
    ok
}

fn link_across(g: &mut Graph, xs: &[Member], ys: &[Member], rel: &str, attrs: &[(String, String)], c: &mut Counts) -> usize {
    let mut ok = 0;
    for x in xs {
        for y in ys {
            ok += match link(g, x, y, rel, attrs, c) {
                0 => 0,
                _ => 1,
            };
        }
    }
    ok
}

/// Mbledh shenjat e nyjës për fjalët ekzistuese: (id, nyja) → mbiemër?
/// Një vetë-lak për çdo (fjalë, nyje): `i` (mbiemër) dhe `e` (p.sh. emri
/// i substantivizuar «e drejtë (e)») janë të dhëna të ndryshme.
fn lex_marks(g: &Graph, ms: &[Member], acc: &mut BTreeMap<(u32, &'static str), bool>) {
    for m in ms {
        match (m.phrase, m.article, existing(g, m)) {
            (false, Some(art), Ok(id)) => {
                let e = acc.entry((id, art)).or_default();
                *e = *e || m.adj;
            }
            _ => {}
        }
    }
}

fn emit_lex(g: &mut Graph, acc: &BTreeMap<(u32, &'static str), bool>, c: &mut Counts) {
    for ((id, art), adj) in acc {
        let mut attrs: Vec<(String, String)> = Vec::new();
        match adj {
            true => attrs.push(("pos".into(), "a".into())),
            false => {}
        }
        attrs.push(("article".into(), (*art).into()));
        attrs.push(("src".into(), "lexin".into()));
        let before = g.edges.len();
        g.add_edge(Edge {
            from: *id,
            to: *id,
            rel: Rel::Lex,
            source: Source::Lexin,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs,
        });
        c.edges_lex += g.edges.len() - before;
    }
}

fn stat(s: &mut GroupStat, pairable: bool, linked: usize) {
    s.total += 1;
    match pairable {
        true => s.pairable += 1,
        false => {}
    }
    match linked {
        0 => {}
        _ => s.linked += 1,
    }
}

/// Anëtarët e disa rreshtave; `row` = indeksi i rreshtit. Dyfishet hiqen.
fn members_of(texts: &[&str], entry_pos: Option<&str>, c: &mut Counts) -> Vec<Member> {
    let mut out: Vec<Member> = Vec::new();
    for (row, t) in texts.iter().enumerate() {
        let (ms, d) = split_members(t, entry_pos, MAX_WORDS_SENSE);
        c.drops.add(d);
        for mut m in ms {
            m.row = row;
            match (out.iter().any(|x| x.lemma == m.lemma), m.restricted) {
                (true, _) => {}
                (false, true) => {
                    c.restricted_members += 1;
                    out.push(m)
                }
                (false, false) => out.push(m),
            }
        }
    }
    out
}

/// Numri i segmenteve `;` jashtë kllapave.
fn semis(text: &str) -> usize {
    let mut depth = 0usize;
    let mut n = 0usize;
    for ch in text.chars() {
        match (ch, depth) {
            ('(', _) => depth += 1,
            (')', 0) => {}
            (')', _) => depth -= 1,
            (';', 0) => n += 1,
            _ => {}
        }
    }
    n
}

fn parse_rows(text: &str, c: &mut Counts) -> Vec<(String, SenseRows)> {
    let mut order: Vec<(String, SenseRows)> = Vec::new();
    let mut idx: HashMap<String, usize> = HashMap::new();
    for line in text.lines() {
        let l = line.trim_end_matches('\r');
        match l.starts_with('#') || l.trim().is_empty() {
            true => continue,
            false => {}
        }
        let cols: Vec<&str> = l.splitn(5, '\t').collect();
        let (sense, pos, field, sub, txt) = match (cols.first(), cols.get(1), cols.get(2), cols.get(3), cols.get(4)) {
            (Some(a), Some(b), Some(f), Some(s), Some(t)) => (*a, *b, *f, *s, *t),
            _ => {
                c.bad_rows += 1;
                continue;
            }
        };
        c.rows += 1;
        let i = match idx.get(sense) {
            Some(i) => *i,
            None => {
                order.push((sense.to_string(), SenseRows { pos: pos.to_string(), ..SenseRows::default() }));
                idx.insert(sense.to_string(), order.len() - 1);
                order.len() - 1
            }
        };
        let r = &mut order[i].1;
        match field {
            "T" => r.main.push(('T', txt.to_string())),
            "S" => r.main.push(('S', txt.to_string())),
            "A" => r.ant.push(txt.to_string()),
            "D" => r.deriv.entry(sub.to_string()).or_default().push(txt.to_string()),
            "C" => r.comp_rows += 1,
            "I" => r.idiom_rows += 1,
            _ => c.bad_rows += 1,
        }
    }
    order
}

fn attrs(sense: &str, pos: Option<&str>, via: Option<&str>) -> Vec<(String, String)> {
    let mut a = vec![("sense".to_string(), format!("lexin:{sense}")), ("src".to_string(), "lexin".to_string())];
    match pos {
        Some(p) => a.push(("pos".into(), p.into())),
        None => {}
    }
    match via {
        Some(v) => a.push(("via".into(), v.into())),
        None => {}
    }
    a
}

pub fn import(g: &mut Graph, _morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let mut c = Counts::default();
    let edges0 = g.edges.len();
    let senses = parse_rows(text, &mut c);
    c.senses = senses.len();
    let mut lex: BTreeMap<(u32, &'static str), bool> = BTreeMap::new();
    for (sid, r) in &senses {
        let pos = match r.pos.as_str() {
            "n" | "v" | "a" | "r" => Some(r.pos.as_str()),
            _ => None,
        };
        // Klasa e hyrjes për ndarjen: edhe «-» (fjalë ndihmëse), ku «e» = lidhëza «dhe».
        let class = match r.pos.as_str() {
            "-" => Some("-"),
            _ => pos,
        };
        c.comp_rows += r.comp_rows;
        c.idiom_rows += r.idiom_rows;
        // 1. Grupi Translation + Synonym.
        let texts: Vec<&str> = r.main.iter().map(|(_, t)| t.as_str()).collect();
        let multi_t: Vec<bool> = r.main.iter().map(|(f, t)| *f == 'T' && semis(t) > 0).collect();
        let main = members_of(&texts, class, &mut c);
        lex_marks(g, &main, &mut lex);
        let ok = link_within(g, &main, &multi_t, &attrs(sid, pos, None), &mut c);
        match r.main.is_empty() {
            true => {}
            false => stat(&mut c.main, main.len() >= 2, ok),
        }
        // 2. Antonimet.
        match r.ant.is_empty() {
            true => {}
            false => {
                let at: Vec<&str> = r.ant.iter().map(String::as_str).collect();
                let ant = members_of(&at, class, &mut c);
                lex_marks(g, &ant, &mut lex);
                // Antonimi i përket fjalës së përkthimit (T); rreshtat S shpjegojnë një fjalë tjetër
                // suedeze dhe nuk e trashëgojnë atë («i shpejtë» ≠ e kundërta e «kronik»).
                let t_texts: Vec<&str> = r.main.iter().filter(|(k, _)| *k == 'T').map(|(_, t)| t.as_str()).collect();
                let mut scratch = Counts::default();
                let main_t = members_of(&t_texts, class, &mut scratch);
                let ok = link_across(g, &main_t, &ant, "ANTONYM", &attrs(sid, pos, None), &mut c);
                stat(&mut c.ant, !main.is_empty() && !ant.is_empty(), ok);
            }
        }
        // 3. Prejardhjet (Compound/Idiom nuk importohen).
        for (sub, ts) in &r.deriv {
            let dt: Vec<&str> = ts.iter().map(String::as_str).collect();
            let ms = members_of(&dt, None, &mut c);
            lex_marks(g, &ms, &mut lex);
            let sk = format!("{sid}:d{sub}");
            let within = link_within(g, &ms, &[], &attrs(&sk, None, None), &mut c);
            let across = link_across(g, &ms, &main, "RELATED", &attrs(&sk, None, Some("derivation")), &mut c);
            let pairable = ms.len() >= 2 || (!ms.is_empty() && !main.is_empty());
            stat(&mut c.deriv, pairable, within + across);
        }
    }
    emit_lex(g, &lex, &mut c);
    let added = g.edges.len() - edges0;
    rep.note(format!(
        "lexin: {} rreshta shqip, {} kuptime (MatchingID); {} rreshta të pavlefshëm",
        c.rows, c.senses, c.bad_rows
    ));
    rep.note(format!(
        "lexin: grupe Translation+Synonym {} (me ≥2 anëtarë {}; të pranuar {}; të anashkaluar {}); antonime {} (të pranuar {}); prejardhje {} (të pranuara {}); të paimportuara me qëllim: {} rreshta Compound, {} rreshta Idiom",
        c.main.total,
        c.main.pairable,
        c.main.linked,
        c.main.total - c.main.linked,
        c.ant.total,
        c.ant.linked,
        c.deriv.total,
        c.deriv.linked,
        c.comp_rows,
        c.idiom_rows
    ));
    rep.note(format!(
        "lexin: çifte të pranuara {}; të anashkaluara {} (fjala e vetme mungon në graf) + {} (frazë–frazë) + {} (homograf: shënues mbiemri te nyje vetëm-emër) + {} (nën-kuptime të ndryshme «;») + {} (shënim kufizues në kllapa, p.sh. «motër (në spital)»; {} anëtarë të tillë mbeten fjalë pa SYNONYM/ANTONYM); skaje të reja: {} SYNONYM, {} ANTONYM, {} RELATED (derivation), {} Lex (nyja e mbiemrit); {} nyje-fraza të krijuara",
        c.pairs_ok,
        c.skip_missing,
        c.skip_phrase_pair,
        c.skip_pos_conflict,
        c.skip_subsense,
        c.skip_restricted,
        c.restricted_members,
        c.edges_syn,
        c.edges_ant,
        c.edges_rel,
        c.edges_lex,
        c.phrases_created
    ));
    rep.note(format!(
        "lexin: copa të hedhura {} ({} vetëm shënim në kllapa, {} copa gjinie/mbaresë si «-e» ose «- e», {} parafraza të gjata, {} me shenja jo-leme, {} nyje «i/e/të» të vetme); korrigjime: {} mbaresa të shquara «-i/-a/-u» të hequra, {} nyje «i/e/të» të hequra në hyrje mbiemërore",
        c.drops.total(),
        c.drops.note_only,
        c.drops.fragment,
        c.drops.long,
        c.drops.other,
        c.drops.bare_article,
        c.drops.suffix_fixed,
        c.drops.article_fixed
    ));
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rreshta në formatin real të `lexin-sq.tsv` (nxjerrë nga swe_alb.xml, Version 2010-07-07).
    const TSV_FIX: &str = "# LEXIN suedisht–shqip (ISOF) — vetëm ana shqipe\n\
2\tn\tT\t-\tzëri i gjashtë në c-dur\n\
2\tn\tS\t-\tnota, tingulli, toni\n\
161\ta\tT\t-\tnë dijeni\n\
161\ta\tS\t-\tazhornuar (i,e), informuar (i,e)\n\
161\ta\tI\t3\tazhurnohem, vihem në dijeni (për diçka), informohem, jam i informuar, jam në dijeni\n\
301\ta\tT\t-\tabstrakt, -e\n\
301\ta\tA\t-\tkonkret, -e\n\
301\ta\tD\t4\tabstraksion\n\
400\ta\tT\t-\tpranuesh/ëm (i), -me (e)\n\
400\ta\tS\t-\tpërshtatsh/ëm (i), -me (e)\n\
64\tr\tT\t-\tme paradhënie\n\
64\tr\tS\t-\t(paraprakisht)\n\
500\tv\tS\t-\tsajesë, fjalë që nuk ekziston\n\
2860\tr\tI\t188\ttani, gjithçka është pothuajse e qartë\n\
5496\tn\tC\t1948\tfryt i mendimit\n\
6666\tn\tT\t-\tshtypje\n\
6666\tn\tD\t2373\tshtypës-i, tiran-i\n\
bad row without tabs\n";

    fn graph_with(words: &[&str]) -> Graph {
        let mut g = Graph::new();
        for w in words {
            g.ensure_word(Lang::Sq, w);
        }
        g
    }

    fn sem(g: &Graph, a: &str, b: &str, rel: &str) -> Vec<Edge> {
        let (x, y) = (g.get(&word_key(Lang::Sq, a)), g.get(&word_key(Lang::Sq, b)));
        match (x, y) {
            (Some(x), Some(y)) => g
                .out_edges(x)
                .filter(|(_, e)| e.to == y && e.rel == Rel::Sem(rel.into()))
                .map(|(_, e)| e.clone())
                .collect(),
            _ => vec![],
        }
    }

    fn lemmas(ms: &[Member]) -> Vec<&str> {
        ms.iter().map(|x| x.lemma.as_str()).collect()
    }

    fn mem(lemma: &str, article: Option<&'static str>, both: bool, phrase: bool) -> Member {
        Member { lemma: lemma.into(), article, both_genders: both, phrase, adj: both, row: 0, seg: 0, restricted: false }
    }

    #[test]
    fn split_handles_markers_notes_slashes_and_fragments() {
        let (m, d) = split_members("azhornuar (i,e), informuar (i,e)", None, 4);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0], mem("azhornuar", Some("i"), true, false));
        assert_eq!(d.total(), 0);
        let (m, d) = split_members("pranuesh/ëm (i), -me (e)", None, 4);
        assert_eq!(m, vec![mem("pranueshëm", Some("i"), false, false)]);
        assert_eq!(d.fragment, 1);
        let (m, _) = split_members("kronik, -e; afatgjatë", None, 4);
        assert_eq!(lemmas(&m), vec!["kronik", "afatgjatë"]);
        assert_eq!((m[0].seg, m[1].seg), (0, 1));
        let (m, d) = split_members("vihem në dijeni (për diçka), (paraprakisht)", None, 4);
        assert_eq!(m, vec![mem("vihem në dijeni", None, false, true)]);
        assert_eq!(d.note_only, 1);
        let (m, d) = split_members("i amësht (i,e), bajat, -e", None, 4);
        assert_eq!(lemmas(&m), vec!["amësht", "bajat"]);
        assert_eq!(d.fragment, 1);
        let (m, d) = split_members("pakuptim (i,e), që është në kundërshtim me arsyen e shëndoshë", None, 4);
        assert_eq!(m.len(), 1);
        assert_eq!(d.long, 1);
        let (m, d) = split_members("pjesë e ligjëratës (p.sh. \"vogël (i,e), kuq (i)\")", None, 4);
        assert_eq!(m[0].lemma, "pjesë e ligjëratës");
        assert_eq!(d.total(), 0);
        let (m, d) = split_members("dhe/ose, Suedi", None, 4);
        assert_eq!(m[0].lemma, "suedi");
        assert_eq!(d.other, 1);
    }

    #[test]
    fn split_fixes_definite_endings_slashes_and_missing_commas() {
        // Mbaresa e shquar e ngjitur me vizë (rreshta realë D/S të LEXIN).
        let (m, d) = split_members("shtypës-i, tiran-i", None, 4);
        assert_eq!(lemmas(&m), vec!["shtypës", "tiran"]);
        assert_eq!(d.suffix_fixed, 2);
        let (m, _) = split_members("modesti-a, krah-u, abëcë-ja, hi-ri", None, 4);
        assert_eq!(lemmas(&m), vec!["modesti", "krah", "abëcë", "hi"]);
        // Fjalë me vizë që nuk janë mbaresa mbeten.
        let (m, _) = split_members("copa-copa, njëri-tjetrin", None, 4);
        assert_eq!(lemmas(&m), vec!["copa-copa", "njëri-tjetrin"]);
        // Vija e pjerrët në emra.
        let (m, _) = split_members("mbiem/ër-ri", None, 4);
        assert_eq!(lemmas(&m), vec!["mbiemër"]);
        let (m, _) = split_members("kif/le-lja", None, 4);
        assert_eq!(lemmas(&m), vec!["kifle"]);
        let (m, d) = split_members("anamnez/ë, -a", None, 4);
        assert_eq!(lemmas(&m), vec!["anamnezë"]);
        assert_eq!(d.fragment, 1);
        let (m, d) = split_members("ai/ajo", None, 4);
        assert!(m.is_empty());
        assert_eq!(d.other, 1);
        // Presja e harruar para copës së gjinisë.
        let (m, d) = split_members("fuqishëm -me", None, 4);
        assert_eq!(lemmas(&m), vec!["fuqishëm"]);
        assert_eq!(d.fragment, 1);
        let (m, _) = split_members("lakueshëm -me i përkulshëm", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["lakueshëm", "përkulshëm"]);
        let (m, _) = split_members("gji -ri, shtatëdhjeti -", None, 4);
        assert_eq!(lemmas(&m), vec!["gji", "shtatëdhjeti"]);
        let (m, d) = split_members("jashtëzakonisht madh-", None, 4);
        assert!(m.is_empty());
        assert_eq!(d.fragment, 1);
    }

    #[test]
    fn leading_article_is_stripped_only_in_adjective_entries() {
        let (m, d) = split_members("e hapur, i ardhur", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["hapur", "ardhur"]);
        assert_eq!(d.article_fixed, 2);
        assert!(m.iter().all(|x| x.article.is_none() && !x.phrase));
        // Emër i substantivizuar në hyrje emërore: mbetet frazë.
        let (m, d) = split_members("e drejtë", Some("n"), 4);
        assert_eq!(m, vec![mem("e drejtë", None, false, true)]);
        assert_eq!(d.article_fixed, 0);
    }

    #[test]
    fn edges_only_between_words_that_already_exist() {
        let mut g = graph_with(&[
            "azhornuar", "informuar", "abstrakt", "konkret", "pranueshëm", "përshtatshëm", "informohem", "sajesë",
            "tani", "fryt", "shtypje", "shtypës", "tiran", "abstraksion",
        ]);
        let nodes0 = g.nodes.len();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let n = import(&mut g, &mut m, TSV_FIX, &mut r);
        assert!(n > 0);
        // sinonime në të dy drejtimet, me çelësin e kuptimit
        let s = sem(&g, "azhornuar", "informuar", "SYNONYM");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].attr("sense"), Some("lexin:161"));
        assert_eq!(s[0].source, Source::Lexin);
        assert_eq!(s[0].method, Method::Published);
        assert_eq!(sem(&g, "informuar", "azhornuar", "SYNONYM").len(), 1);
        assert_eq!(sem(&g, "pranueshëm", "përshtatshëm", "SYNONYM").len(), 1);
        // antonimi
        assert_eq!(sem(&g, "abstrakt", "konkret", "ANTONYM").len(), 1);
        assert_eq!(sem(&g, "konkret", "abstrakt", "ANTONYM").len(), 1);
        // fraza krijohet vetëm sepse partneri me një fjalë ekziston
        assert_eq!(sem(&g, "në dijeni", "azhornuar", "SYNONYM").len(), 1);
        // Idiomat dhe fjalët e përbëra NUK importohen
        assert!(sem(&g, "vihem në dijeni", "informohem", "SYNONYM").is_empty());
        assert!(sem(&g, "informohem", "informuar", "RELATED").is_empty());
        assert!(g.get(&word_key(Lang::Sq, "gjithçka është pothuajse e qartë")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "fryt i mendimit")).is_none());
        // Prejardhja: alternativat → SYNONYM; lidhja me grupin → RELATED via=derivation
        assert_eq!(sem(&g, "shtypës", "tiran", "SYNONYM")[0].attr("sense"), Some("lexin:6666:d2373"));
        let rel = sem(&g, "shtypës", "shtypje", "RELATED");
        assert_eq!(rel[0].attr("via"), Some("derivation"));
        assert_eq!(sem(&g, "abstraksion", "abstrakt", "RELATED").len(), 1);
        // fjalët që mungojnë nuk krijohen
        for w in ["nota", "tingulli", "toni", "kronik"] {
            assert!(g.get(&word_key(Lang::Sq, w)).is_none(), "{w}");
        }
        // fraza pa partner ekzistues nuk krijohet
        assert!(g.get(&word_key(Lang::Sq, "me paradhënie")).is_none());
        // fraza me partner ekzistues («sajesë») krijohet dhe lidhet
        assert_eq!(sem(&g, "fjalë që nuk ekziston", "sajesë", "SYNONYM").len(), 1);
        // në dijeni, fjalë që nuk ekziston
        assert_eq!(g.nodes.len(), nodes0 + 2);
        // Lex: nyja e mbiemrit, një vetë-lak
        let az = g.get(&word_key(Lang::Sq, "azhornuar")).unwrap();
        let lex: Vec<_> = g.out_edges(az).filter(|(_, e)| e.rel == Rel::Lex).collect();
        assert_eq!(lex.len(), 1);
        assert_eq!(lex[0].1.to, az);
        assert_eq!(lex[0].1.attr("article"), Some("i"));
        assert_eq!(lex[0].1.attr("pos"), Some("a"));
        // `(e)` vetëm në një hyrje emërore: nyja ruhet, POS jo
        let mut g2 = graph_with(&["ardhme", "ardhmja"]);
        let tsv2 = "9\tn\tT\t-\tardhme (e)\n9\tn\tS\t-\tardhmja\n";
        import(&mut g2, &mut m, tsv2, &mut r);
        let ar = g2.get(&word_key(Lang::Sq, "ardhme")).unwrap();
        let lex2: Vec<_> = g2.out_edges(ar).filter(|(_, e)| e.rel == Rel::Lex).collect();
        assert_eq!(lex2.len(), 1);
        assert_eq!(lex2[0].1.attr("article"), Some("e"));
        assert_eq!(lex2[0].1.attr("pos"), None);
        // raporti
        assert!(r.lines.iter().any(|l| l.contains("1 rreshta të pavlefshëm")));
        assert!(r.lines.iter().any(|l| l.contains("1 rreshta Compound, 2 rreshta Idiom")));
    }

    #[test]
    fn one_lex_loop_per_word_even_when_entries_disagree() {
        let mut g = graph_with(&["hollë", "imët", "fortë"]);
        let tsv = "1\tn\tT\t-\thollë (i), imët (i,e)\n2\ta\tT\t-\thollë (i,e), fortë\n3\tv\tS\t-\thollë (i)\n";
        let mut r = ImportReport::default();
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        let h = g.get(&word_key(Lang::Sq, "hollë")).unwrap();
        let lex: Vec<_> = g.out_edges(h).filter(|(_, e)| e.rel == Rel::Lex).collect();
        assert_eq!(lex.len(), 1);
        assert_eq!(lex[0].1.attr("pos"), Some("a"));
        // «e drejtë (e)» në hyrje emërore ≠ «drejtë (i,e)»: dy vetë-lakë të ndryshëm, pa dyfishim.
        let mut g = graph_with(&["drejtë", "ligj", "saktë"]);
        let tsv = "1	n	T	-	e drejtë (e), ligj
2	a	T	-	drejtë (i,e), saktë (i,e)
3	a	S	-	drejtë (i,e)
";
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        let d = g.get(&word_key(Lang::Sq, "drejtë")).unwrap();
        let mut lex: Vec<(Option<&str>, Option<&str>)> =
            g.out_edges(d).filter(|(_, e)| e.rel == Rel::Lex).map(|(_, e)| (e.attr("article"), e.attr("pos"))).collect();
        lex.sort();
        assert_eq!(lex, vec![(Some("e"), None), (Some("i"), Some("a"))]);
    }

    #[test]
    fn verb_entries_read_i_as_clitic_not_article() {
        // Rreshta realë 23430 / 5392: "(i) nënshtrohem", "pajtohem, nënshtrohem (i)"
        let (m, _) = split_members("pajtohem, nënshtrohem (i)", Some("v"), 4);
        assert_eq!(m[1], mem("nënshtrohem", None, false, false));
        // "(i,e)" mbetet mbiemër edhe në hyrje foljore.
        let (m, _) = split_members("informuar (i,e)", Some("v"), 4);
        assert_eq!(m[0].article, Some("i"));
        // Në hyrje emërore "(e)" është nyjë, por jo sinjal mbiemri ("e shtunë").
        let (m, _) = split_members("e shtunë (e)", Some("n"), 4);
        assert_eq!((m[0].lemma.as_str(), m[0].article, m[0].adj), ("shtunë", Some("e"), false));
    }

    #[test]
    fn feminine_form_after_masculine_marker_is_dropped() {
        // Rresht real 4064: "zoti (i), zonja (e), kompetent, -e"
        let (m, d) = split_members("zoti (i), zonja (e), kompetent, -e", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["zoti", "kompetent"]);
        assert_eq!(d.fragment, 2);
        let (m, _) = split_members("kuq (i), kuqe (e)", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["kuq"]);
        // "(e)" pa "(i)" para tij mbetet ("ardhme (e)").
        let (m, _) = split_members("ardhme (e), e ardhmja", Some("n"), 4);
        assert_eq!(m[0].lemma, "ardhme");
    }

    #[test]
    fn semicolon_subsenses_are_not_merged() {
        // 5496 (real): Translation "embrion; bërthamë, zanafillë"-lloj; S me «;».
        let mut g = graph_with(&["embrion", "bërthamë", "zanafillë", "gol", "portë", "fillim"]);
        let tsv = "5496\tn\tT\t-\tembrion; bërthamë, zanafillë\n13377\tn\tT\t-\tgol; portë\n13377\tn\tS\t-\tfillim\n77\tn\tT\t-\tfillim\n77\tn\tS\t-\tembrion; zanafillë\n";
        let mut r = ImportReport::default();
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        assert!(sem(&g, "embrion", "bërthamë", "SYNONYM").is_empty());
        assert_eq!(sem(&g, "bërthamë", "zanafillë", "SYNONYM").len(), 1);
        assert!(sem(&g, "gol", "portë", "SYNONYM").is_empty());
        // T me disa nën-kuptime: nuk lidhet me S
        assert!(sem(&g, "gol", "fillim", "SYNONYM").is_empty());
        // S me «;» lidhet me T (një nën-kuptim), por segmentet e S jo ndërmjet tyre
        assert_eq!(sem(&g, "fillim", "embrion", "SYNONYM").len(), 1);
        assert_eq!(sem(&g, "fillim", "zanafillë", "SYNONYM").len(), 1);
        assert!(r.lines.iter().any(|l| l.contains("nën-kuptime të ndryshme")));
    }

    #[test]
    fn adjective_marker_does_not_attach_to_noun_only_homograph() {
        // LEXIN 7425: "zo/ti (i), -nja (e)" = i zoti (i aftë); në graf "zoti" = Zot (emër).
        let mut g = graph_with(&["zoti", "shkëlqyer", "aftë"]);
        let zoti = g.get(&word_key(Lang::Sq, "zoti")).unwrap();
        let s = g.ensure(sense_key(Source::Pwn30, "09505418-n"), NodeKind::Sense { source: Source::Pwn30, synset: "09505418-n".into(), pos: Pos::Noun });
        g.add_edge(Edge { from: zoti, to: s, rel: Rel::HasSense, source: Source::OmwAls, lang: Some(Lang::Sq), method: Method::OmwOffsetMap, attrs: vec![] });
        let tsv = "7425\ta\tT\t-\tzo/ti (i), -nja (e), shkëlqyer (i,e), aftë (i,e)\n";
        let mut r = ImportReport::default();
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        assert!(sem(&g, "zoti", "shkëlqyer", "SYNONYM").is_empty());
        assert_eq!(sem(&g, "shkëlqyer", "aftë", "SYNONYM").len(), 1);
        assert_eq!(g.out_edges(zoti).filter(|(_, e)| e.rel == Rel::Lex).count(), 0);
        assert!(r.lines.iter().any(|l| l.contains("+ 2 (homograf")));
        // "(i,e)" e identifikon mbiemrin qartë: lidhet edhe me nyje që njihet vetëm si emër.
        let mut g = graph_with(&["fortë", "fuqishëm"]);
        let f = g.get(&word_key(Lang::Sq, "fortë")).unwrap();
        let s2 = g.ensure(sense_key(Source::Pwn30, "05035353-n"), NodeKind::Sense { source: Source::Pwn30, synset: "05035353-n".into(), pos: Pos::Noun });
        g.add_edge(Edge { from: f, to: s2, rel: Rel::HasSense, source: Source::OmwAls, lang: Some(Lang::Sq), method: Method::OmwOffsetMap, attrs: vec![] });
        import(&mut g, &mut Morph::default(), "1	a	T	-	fortë (i,e), fuqishëm (i), -me (e)
", &mut r);
        assert_eq!(sem(&g, "fortë", "fuqishëm", "SYNONYM").len(), 1);
    }

    #[test]
    fn bare_articles_never_become_members() {
        // Rresht real 13721: "i,e rënë shpirtërisht" = shënuesi (i,e) + «rënë shpirtërisht».
        let (m, d) = split_members("tepër i dëshpëruar, shumë i hidhëruar, i,e rënë shpirtërisht", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["tepër i dëshpëruar", "shumë i hidhëruar", "rënë shpirtërisht"]);
        assert_eq!((m[2].article, m[2].both_genders, m[2].adj), (Some("i"), true, true));
        assert_eq!(d.bare_article, 0);
        let (m, _) = split_members("i/e rënë shpirtërisht", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["rënë shpirtërisht"]);
        // Rreshta realë 984 / 18184 / 16454: vizë e veçuar + mbaresë = një copë gjinie.
        let (m, d) = split_members("absolutist,- e, autokratik, -e, diktatorial, -e", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["absolutist", "autokratik", "diktatorial"]);
        assert_eq!(d.fragment, 3);
        let (m, _) = split_members("nervoz, - e", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["nervoz"]);
        let (m, _) = split_members("pleonastik,- e (formulim), parazitar, -e (fjalë)", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["pleonastik", "parazitar"]);
        // Nyja e vetme hidhet në hyrjet n/v/a/r dhe në prejardhje (klasa None).
        let (m, d) = split_members("e, dhe", Some("a"), 4);
        assert_eq!(lemmas(&m), vec!["dhe"]);
        assert_eq!(d.bare_article, 1);
        let (m, d) = split_members("i, të", None, 4);
        assert!(m.is_empty());
        assert_eq!(d.bare_article, 2);
        // Lidhëza «e» në hyrjet e fjalëve ndihmëse (rreshta realë 3027, 21595) mbetet; «i», «të» jo.
        let (m, _) = split_members("e, dhe, edhe", Some("-"), 4);
        assert_eq!(lemmas(&m), vec!["e", "dhe", "edhe"]);
        let (m, d) = split_members("i, të", Some("-"), 4);
        assert!(m.is_empty());
        assert_eq!(d.bare_article, 2);
    }

    #[test]
    fn article_nodes_get_no_lexin_edges_from_split_markers() {
        let mut g = graph_with(&["i", "e", "dhe", "edhe", "madje", "pikëlluar", "absolutist", "autokratik", "diktatorial", "nervoz"]);
        let tsv = "984\ta\tS\t-\tabsolutist,- e, autokratik, -e, diktatorial, -e\n\
3027\t-\tT\t-\te, dhe, edhe\n\
3027\t-\tS\t-\tmadje\n\
13721\ta\tT\t-\tpikëlluar (i,e)\n\
13721\ta\tS\t-\ttepër i dëshpëruar, shumë i hidhëruar, i,e rënë shpirtërisht\n\
18184\ta\tT\t-\tnervoz, - e\n";
        let mut r = ImportReport::default();
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        let i = g.get(&word_key(Lang::Sq, "i")).unwrap();
        assert_eq!(g.out_edges(i).filter(|(_, e)| e.source == Source::Lexin).count(), 0);
        for adj in ["absolutist", "autokratik", "diktatorial", "nervoz", "pikëlluar"] {
            assert!(sem(&g, "e", adj, "SYNONYM").is_empty(), "{adj}");
        }
        assert_eq!(sem(&g, "absolutist", "autokratik", "SYNONYM").len(), 1);
        // Lidhëza «e» = «dhe» mbetet.
        assert_eq!(sem(&g, "e", "dhe", "SYNONYM").len(), 1);
        assert_eq!(sem(&g, "e", "madje", "SYNONYM").len(), 1);
        // «i,e rënë shpirtërisht» → fraza «rënë shpirtërisht», jo «e rënë shpirtërisht».
        assert_eq!(sem(&g, "rënë shpirtërisht", "pikëlluar", "SYNONYM").len(), 1);
        assert!(g.get(&word_key(Lang::Sq, "e rënë shpirtërisht")).is_none());
    }

    #[test]
    fn restricting_note_keeps_word_but_blocks_synonyms() {
        let (m, _) = split_members("zemër (e frutit), farë (e mollës)", Some("n"), 4);
        assert_eq!(lemmas(&m), vec!["zemër", "farë"]);
        assert!(m.iter().all(|x| x.restricted));
        // Shënimet gramatikore (nyje, përemër i shkurtër, valencë) nuk kufizojnë.
        for (t, p) in [
            ("vihem në dijeni (për diçka)", None),
            ("nënshtrohem (i)", Some("v")),
            ("zjarrtë (i,e,)", Some("a")),
            ("kujdesem (për dikë)", Some("v")),
            ("ardhme (e)", Some("n")),
        ] {
            let (m, _) = split_members(t, p, 4);
            assert!(!m[0].restricted, "{t}");
        }
        // Rreshta realë 18062 / 18533.
        let mut g = graph_with(&["infermiere", "motër", "bërthamë", "zemër", "farë", "fillim", "nismë"]);
        let tsv = "18062\tn\tT\t-\tinfermiere\n18062\tn\tS\t-\tmotër (në spital)\n18533\tn\tT\t-\tbërthamë\n18533\tn\tS\t-\tzemër (e frutit), farë (e mollës)\n9\tn\tT\t-\tfillim (për diçka)\n9\tn\tS\t-\tnismë\n";
        let mut r = ImportReport::default();
        import(&mut g, &mut Morph::default(), tsv, &mut r);
        assert!(sem(&g, "motër", "infermiere", "SYNONYM").is_empty());
        assert!(sem(&g, "zemër", "bërthamë", "SYNONYM").is_empty());
        assert!(sem(&g, "farë", "bërthamë", "SYNONYM").is_empty());
        assert!(sem(&g, "zemër", "farë", "SYNONYM").is_empty());
        assert_eq!(sem(&g, "fillim", "nismë", "SYNONYM").len(), 1);
        assert!(g.get(&word_key(Lang::Sq, "motër")).is_some());
        assert!(r.lines.iter().any(|l| l.contains("+ 4 (shënim kufizues në kllapa") && l.contains("3 anëtarë të tillë")));
    }

    #[test]
    fn empty_graph_accepts_nothing_and_creates_nothing() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, TSV_FIX, &mut r), 0);
        assert_eq!(g.nodes.len(), 0);
        assert!(m.entries.is_empty());
    }

    #[test]
    fn importing_twice_adds_no_duplicates() {
        let mut g = graph_with(&["azhornuar", "informuar"]);
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let first = import(&mut g, &mut m, TSV_FIX, &mut r);
        assert!(first > 0);
        assert_eq!(import(&mut g, &mut m, TSV_FIX, &mut r), 0);
    }
}
