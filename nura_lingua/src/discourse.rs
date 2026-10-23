//! Planifikuesi i tekstit — deterministik dhe logjik.
//!
//! Merr një kuptim të zgjedhur në graf dhe vendos ÇFARË të thuhet dhe NË ÇFARË
//! RENDI, para se realizuesi (`expression`) ta kthejë në shqip.
//!
//! Rregullat (të njëjta për çdo hyrje, pa rastësi):
//! 1. Rëndësia: përgjigjja e pyetjes së parës; pastaj vetëm 1–2 fakte që e
//!    ndihmojnë kuptimin (jo gjithçka që di grafi).
//! 2. Koherenca: kategoria ("është një lloj …") thuhet vetëm kur përputhet me
//!    përkufizimin (fjala e kategorisë ose një prind i saj del në përkufizim),
//!    ose kur nuk ka përkufizim fare. Kështu një lidhje e gabuar në burim
//!    (p.sh. arsim → «kënaqësi») nuk shqiptohet.
//! 3. Pa përsëritje: një fjalë që u tha nuk thuhet sërish në një faset tjetër.
//! 4. Renditje e fiksuar brenda çdo faseti: rendi i burimit, pa dublikata.

use crate::expression::{Attribution, Claim, Clause, Facet, Polarity, TermRef};
use crate::graph::{word_key, Lang, NodeKind, Pos, Rel, Source};
use crate::query::{CandStatus, Engine, Focus};
use std::collections::{BTreeSet, HashSet};

const MAX_ITEMS: usize = 3;

fn tokens(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.chars().count() > 2)
        .map(|w| w.to_string())
        .collect()
}

/// Përputhje e dobët por e qëndrueshme: fjalë e njëjtë ose rrënjë e përbashkët (5 shkronja).
fn mentions(def: &HashSet<String>, label: &str) -> bool {
    label.to_lowercase().split_whitespace().any(|w| {
        let stem: String = w.chars().take(5).collect();
        def.iter().any(|d| d == w || (stem.chars().count() >= 5 && d.starts_with(&stem)))
    })
}

pub struct Facts {
    pub origin: Vec<String>,
    pub native: Vec<String>,
    pub dialect: Vec<String>,
    pub category: Vec<String>,
    pub synonyms: Vec<String>,
    pub parts: Vec<String>,
    pub part_of: Vec<String>,
    pub kinds: Vec<String>,
    pub opposites: Vec<String>,
    pub translation_en: Vec<String>,
    pub example: Vec<String>,
    /// Kuptimi krahinor i Veriut (Gazulli 1941), kur fjala s'ka përkufizim tjetër shqip.
    pub regional: Vec<String>,
    /// Burimet që dëshmojnë çdo faset (për rreshtin e veçantë të burimeve).
    pub src: Vec<(Facet, Vec<Source>)>,
}

impl Facts {
    fn src_of(&self, facet: Facet) -> Vec<Source> {
        let mut v: Vec<Source> = self.src.iter().filter(|(f, _)| *f == facet).flat_map(|(_, s)| s.iter().copied()).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// Etiketa shqipe e pastruar: pa nënvizime dhe pa nyjën "një" në fillim (p.sh. «një thundrak»).
pub fn clean_label(l: &str) -> String {
    let l = l.replace('_', " ").to_lowercase();
    let l = l.split(',').next().unwrap_or("").trim().to_string();
    match l.strip_prefix("një ") {
        Some(rest) => rest.trim().to_string(),
        None => l.trim().to_string(),
    }
}

fn sq_label_raw(eng: &Engine, id: u32) -> Option<String> {
    eng.g.label_in(id, Lang::Sq).map(clean_label)
}

/// Fjala shqipe është e dëshmuar jashtë AlbaNet-it (gramatikë, UD, Wiktionary, LEXIN, Wikidata…).
fn attested_sq(eng: &Engine, l: &str) -> bool {
    eng.g
        .get(&word_key(Lang::Sq, l))
        .map(|w| {
            eng.g.out_edges(w).any(|(_, e)| independent_sq(e.source)) || eng.g.in_edges(w).any(|(_, e)| independent_sq(e.source))
        })
        .unwrap_or(false)
}

/// Etiketa shqipe e një kuptimi për t'u thënë. Para: etiketa që është fjalë e dëshmuar në një
/// burim shqip të pavarur dhe nuk nis me nyjë (p.sh. «profesion» para «day job»); pastaj e para
/// që nuk është anglishte e papërkthyer (e njëjtë me një etiketë angleze dhe e padëshmuar).
pub fn sq_label(eng: &Engine, id: u32) -> Option<String> {
    let n = eng.g.node(id);
    let en: HashSet<String> =
        n.labels.iter().filter(|t| t.lang == Lang::En).map(|t| t.text.replace('_', " ").to_lowercase()).collect();
    let cands: Vec<String> = n
        .labels
        .iter()
        .filter(|t| t.lang == Lang::Sq)
        .map(|t| clean_label(&t.text))
        .filter(|l| !l.is_empty())
        .collect();
    let attested = cands.iter().find(|l| article_of(l).is_none() && attested_sq(eng, l)).cloned();
    let plain = cands.iter().find(|l| !(en.contains(l.as_str()) && !attested_sq(eng, l))).cloned();
    attested.or(plain)
}

/// Burimet shqipe të pavarura nga WordNet-i (fjalë e dëshmuar në shqip jashtë AlbaNet).
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

/// Kuptimi ka etiketë shqipe, por ajo është anglishte e papërkthyer (p.sh. «liquid»).
fn untranslated(eng: &Engine, id: u32) -> bool {
    matches!((sq_label_raw(eng, id), sq_label(eng, id)), (Some(_), None))
}

/// Përkthimi anglisht nga një glosë e Wiktionary-t: shprehja e parë (deri në 3 fjalë), pa
/// nyjën angleze dhe pa «to» të foljes; jo fjalë funksionale («of», «or», «relating» …).
pub fn bridge_word(gloss: &str) -> Option<String> {
    // «participle of vdes: dead», «plural of X»: pjesa gramatikore nuk është përkthim; kur ka
    // kuptim pas «:», merret ai.
    let (head, tail) = match gloss.split_once(':') {
        Some((h, t)) => (h, Some(t)),
        None => (gloss, None),
    };
    let grammar = head.contains(" of ")
        && matches!(
            head.split_whitespace().next(),
            Some(
                "participle" | "plural" | "feminine" | "masculine" | "definite" | "indefinite" | "inflection" | "form"
                    | "past" | "present" | "singular" | "superlative" | "comparative" | "diminutive"
            )
        );
    let gloss = match (grammar, tail) {
        (true, Some(t)) => t,
        (true, None) => "",
        (false, _) => gloss,
    };
    let first = gloss.split([',', ';', '(', ':']).next().unwrap_or("").trim().to_string();
    let toks: Vec<&str> = first.split_whitespace().collect();
    let toks: &[&str] = match toks.first() {
        Some(&("a" | "an" | "the" | "to")) => &toks[1..],
        _ => &toks[..],
    };
    let function_word = matches!(
        toks.first(),
        Some(
            &("of" | "or" | "relating" | "pertaining" | "any" | "one" | "someone" | "something" | "used" | "having"
                | "in" | "with" | "that" | "which" | "alternative" | "obsolete")
        )
    );
    match (toks.is_empty(), toks.len() <= 3, function_word) {
        (false, true, false) => Some(toks.join(" ")),
        _ => None,
    }
}

/// Kuptimet e Wiktionary-t anglisht të një fjale: (pjesa e ligjëratës, numri i kuptimit,
/// përkthimi). Kuptimet e emrave të përveçëm («kaikki:…:name:…») nuk merren.
pub fn kaikki_bridge(eng: &Engine, w: u32) -> Vec<(Pos, u32, String)> {
    let mut out = Vec::new();
    for (sid, e) in eng.g.senses_of(w) {
        match (e.source, &eng.g.node(sid).kind) {
            (Source::KaikkiEn, NodeKind::Sense { synset, pos, .. }) => {
                let n: u32 = synset
                    .rsplit(':')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(u32::MAX);
                let first = eng.g.node(sid).labels.iter().find(|t| t.lang == Lang::En).and_then(|t| bridge_word(&t.text));
                match (synset.contains(":name:"), first) {
                    (false, Some(b)) => out.push((*pos, n, b)),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

/// Ura anglisht e fjalës për verifikim: përkthimi i kuptimit të parë të çdo pjese ligjërate.
fn english_bridge(eng: &Engine, w: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, n, b) in kaikki_bridge(eng, w) {
        let b = b.to_lowercase();
        match (n == 1, out.contains(&b)) {
            (true, false) => out.push(b),
            _ => {}
        }
    }
    out
}

/// Të gjitha përkthimet anglisht të një fjale (Wiktionary + etiketat angleze të kuptimeve të
/// saj në AlbaNet/BabelNet): për të verifikuar nëse dy fjalë shqipe ndajnë një kuptim.
fn english_all(eng: &Engine, w: u32) -> HashSet<String> {
    let mut out: HashSet<String> = kaikki_bridge(eng, w).into_iter().map(|(_, _, b)| b.to_lowercase()).collect();
    for (sid, e) in eng.g.senses_of(w) {
        match e.source {
            // Përkthimi i modelit nuk vlen si verifikim (modeli s'e verifikon veten).
            Source::KaikkiEn | Source::ModelClaude => {}
            _ => {
                for t in eng.g.node(sid).labels.iter().filter(|t| t.lang == Lang::En) {
                    out.insert(t.text.replace('_', " ").to_lowercase());
                }
            }
        }
    }
    out
}

/// Glosat angleze të plota (pjesa e parë) të një fjale në Wiktionary dhe etiketat angleze të
/// kuptimeve të saj: për të dalluar një lloj («grey or white horse») nga një sinonim («horse»).
fn english_raw(eng: &Engine, w: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (sid, _) in eng.g.senses_of(w).into_iter().filter(|(_, e)| e.source != Source::ModelClaude) {
        for t in eng.g.node(sid).labels.iter().filter(|t| t.lang == Lang::En) {
            let first = t.text.split([';', '(', ',']).next().unwrap_or("").trim().replace('_', " ").to_lowercase();
            match (first.is_empty(), out.contains(&first)) {
                (false, false) => out.push(first),
                _ => {}
            }
        }
    }
    out
}

/// Përkthimet anglisht të fjalës në fjalorin e Wiktionary-t (përmes ConceptNet). (1) Glosa e
/// vetë hyrjes shqipe kur është një fjalë e vetme («qen» → «dog», «lumë» kuptimi 1 →
/// «river»): ConceptNet e ruan si `/r/RelatedTo` nga koncepti i hyrjes. (2) Tabelat e
/// përkthimeve (`/r/Synonym`). Lidhjet që ConceptNet i ka marrë nga WordNet-i nuk merren
/// (janë i njëjti AlbaNet, jo dëshmi tjetër).
pub fn dict_en(eng: &Engine, f: &Focus) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let concept_ok = |c: u32| match &eng.g.node(c).kind {
        NodeKind::Concept { uri } => !uri.contains("/wikt/") || uri.ends_with("/wikt/en_1"),
        _ => false,
    };
    let gloss_word = |r: &&crate::query::FormRel| {
        r.rel == "/r/RelatedTo"
            && r.outgoing
            && r.other_lang_code == "en"
            && r.dataset.starts_with("/d/wiktionary/en")
            && !r.other_term.contains(' ')
            && concept_ok(r.concept)
    };
    // Vetëm kur koncepti ka një fjalë angleze të vetme në glosë (ndryshe glosa është përkufizim);
    // konceptet e pjesës kryesore të ligjëratës së fjalës para («mësues» emër → «teacher»,
    // jo mbiemri «instructive»).
    let main = f.resolved.word.and_then(|w| main_pos(eng, w));
    let concept_pos = |c: u32| match &eng.g.node(c).kind {
        NodeKind::Concept { uri } => uri.split('/').nth(4).and_then(Pos::from_wn),
        _ => None,
    };
    let mut glossed: Vec<(bool, String)> = Vec::new();
    for r in f.form_rels.iter().filter(gloss_word) {
        let n = f.form_rels.iter().filter(gloss_word).filter(|x| x.concept == r.concept).count();
        let p = concept_pos(r.concept);
        let other_pos = p.is_some() && main.is_some() && p != main;
        match (n == 1, glossed.iter().any(|(_, t)| *t == r.other_term)) {
            (true, false) => glossed.push((other_pos, r.other_term.clone())),
            _ => {}
        }
    }
    glossed.sort_by_key(|(o, _)| *o);
    for (_, t) in glossed {
        out.push(t);
    }
    for r in f.form_rels.iter().filter(|r| {
        r.rel == "/r/Synonym"
            && r.other_lang_code == "en"
            && r.dataset.starts_with("/d/wiktionary")
            && !r.other_term.ends_with(" translations")
    }) {
        match out.contains(&r.other_term) {
            true => {}
            false => out.push(r.other_term.clone()),
        }
    }
    out
}

/// Përkthimi anglisht i fjalës nga Wiktionary (urë): kuptimi i parë i pjesës kryesore të
/// ligjëratës (ose, kur ajo s'ka, i pjesës me më shumë kuptime). Vetëm një përkthim, që
/// kuptime të ndryshme të mos bashkohen në një fjali.
pub fn bridge_translation(eng: &Engine, w: Option<u32>) -> Vec<String> {
    match w {
        None => vec![],
        Some(w) => {
            let senses = kaikki_bridge(eng, w);
            let main = main_pos(eng, w);
            let count = |p: Pos| senses.iter().filter(|(q, _, _)| *q == p).count();
            let mut c: Vec<(bool, std::cmp::Reverse<usize>, u32, String)> =
                senses.iter().map(|(p, n, b)| (Some(*p) != main, std::cmp::Reverse(count(*p)), *n, b.clone())).collect();
            c.sort();
            c.into_iter().take(1).map(|x| x.3).collect()
        }
    }
}

/// Kuptimet e Wiktionary-t anglisht të pjesës kryesore japin përkthime të ndryshme
/// (p.sh. «mot» = «weather» dhe «year»): fjala është shumëkuptimëshe.
pub fn bridge_polysemous(eng: &Engine, w: Option<u32>) -> bool {
    match w {
        None => false,
        Some(w) => {
            let main = main_pos(eng, w);
            let mut heads: Vec<String> = kaikki_bridge(eng, w)
                .into_iter()
                .filter(|(p, _, _)| Some(*p) == main)
                .map(|(_, _, b)| b.to_lowercase())
                .collect();
            heads.sort();
            heads.dedup();
            heads.len() > 1
        }
    }
}

/// Paraardhësit në WordNet (etiketat shqip dhe anglisht) të kuptimeve emërore angleze që
/// kanë si lemë një nga fjalët e urës. Shërben vetëm për VERIFIKIM, kurrë për të shtuar fakte.
pub(crate) fn bridge_ancestors(eng: &Engine, bridge: &[String]) -> (HashSet<String>, HashSet<String>) {
    let mut sq = HashSet::new();
    let mut en = HashSet::new();
    let starts: Vec<u32> = eng
        .g
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| match &n.kind {
            NodeKind::Sense { source: Source::Pwn30, pos: Pos::Noun, .. } => {
                n.labels.iter().any(|t| t.lang == Lang::En && bridge.contains(&t.text.replace('_', " ").to_lowercase()))
            }
            _ => false,
        })
        .map(|(i, _)| i as u32)
        .collect();
    for s in starts {
        for (a, _) in ancestors(eng, s).into_iter().skip(1) {
            for t in &eng.g.node(a).labels {
                match t.lang {
                    Lang::Sq => {
                        sq.insert(clean_label(&t.text));
                    }
                    Lang::En => {
                        en.insert(t.text.replace('_', " ").to_lowercase());
                    }
                }
            }
        }
    }
    (sq, en)
}

/// Një sinonim ose e kundërt duhet të jetë fjalë, jo shpjegim: deri në dy fjalë (pa nyjën
/// e mbiemrit) dhe jo një parafrazë që nis me «lloj», «një», «si», «që», «për», ose me një
/// parafjalë («në lindje»); jo shkurtim («etj»).
pub fn lexical(item: &str) -> bool {
    let toks: Vec<&str> = item.split_whitespace().collect();
    let content: &[&str] = match toks.first() {
        Some(&("i" | "e" | "të")) => &toks[1..],
        _ => &toks[..],
    };
    let para = matches!(
        content.first(),
        Some(&("lloj" | "një" | "si" | "që" | "për" | "me" | "pa" | "në" | "nga" | "jashtë" | "te" | "tek" | "mbi" | "nën"))
    );
    let abbr = matches!(content.first(), Some(&("etj" | "etj." | "p.sh." | "psh" | "dmth" | "d.m.th.")));
    !content.is_empty() && content.len() <= 2 && !para && !abbr
}

/// Lidhja i përket kuptimit kryesor (1) të fjalës në burim. Wiktionary anglisht shënon
/// «kaikki:<fjala>:<pos>:<n>»; Wiktionary gjermanisht numëron kuptimet («1», «1,2», «2a»,
/// «adj:1;noun:2a»). Nënkuptimet me shkronjë ndjekin numrin («1b» → 1, «2a» → 2).
/// Burimet pa numër kuptimi (p.sh. grupet e LEXIN) trajtohen si të kuptimit kryesor.
fn primary_sense(src: Source, a: Option<&str>) -> bool {
    let num = |t: &str| t.trim().chars().take_while(|c| c.is_ascii_digit()).collect::<String>();
    match (src, a) {
        (_, None) => true,
        (Source::KaikkiEn, Some(s)) => num(s.rsplit(':').next().unwrap_or("")) == "1",
        (Source::KaikkiDe, Some(s)) => s
            .split(';')
            .flat_map(|p| p.rsplit(':').next().unwrap_or("").split(','))
            .any(|t| num(t) == "1"),
        _ => true,
    }
}

/// Pjesa e ligjëratës së një fjale sipas burimeve të pavarura nga sinsetet e WordNet-it:
/// gramatika (`Lex`), përdorimet UD dhe kuptimet e Wiktionary-t. Kështu një lemë e futur
/// gabim në një sinset (p.sh. folja «lulëzoj» te sinseti emëror «lule») nuk e "vërteton"
/// vetë pjesën e saj të ligjëratës. ConceptNet (`FormOf`) nuk numërohet: konceptet e tij
/// shqipe vijnë pjesërisht nga i njëjti AlbaNet.
pub fn independent_pos(eng: &Engine, w: u32) -> std::collections::BTreeSet<Pos> {
    let mut out = std::collections::BTreeSet::new();
    for (_, e) in eng.g.out_edges(w) {
        match (&e.rel, &eng.g.node(e.to).kind, e.source) {
            (Rel::HasSense, NodeKind::Sense { pos, .. }, Source::KaikkiEn) => {
                out.insert(*pos);
            }
            (Rel::OccursIn, _, _) => match e.attr("upos").and_then(Pos::from_upos) {
                Some(p) => {
                    out.insert(p);
                }
                None => {}
            },
            _ => {}
        }
    }
    for (_, e) in eng.g.in_edges(w) {
        match (&e.rel, e.attr("pos").and_then(Pos::from_wn)) {
            (Rel::Lex, Some(p)) => {
                out.insert(p);
            }
            _ => {}
        }
    }
    out
}

/// Pjesët e ligjëratës janë të pajtueshme kur njëra mungon ose kanë të paktën një të përbashkët.
fn pos_compatible(a: &std::collections::BTreeSet<Pos>, b: &std::collections::BTreeSet<Pos>) -> bool {
    a.is_empty() || b.is_empty() || a.intersection(b).next().is_some()
}



/// Fjala e parë kuptimplote e përkufizimit (pa «një», «i/e/të», «është») përputhet me `label`.
fn is_genus(definition: Option<&str>, label: &str) -> bool {
    let first = definition
        .unwrap_or("")
        .to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty() && !matches!(*w, "një" | "i" | "e" | "të" | "është" | "janë" | "lloj"))
        .map(|w| w.to_string())
        .next();
    let head: String = label.split_whitespace().next().unwrap_or("").chars().take(5).collect();
    match first {
        Some(w) => head.chars().count() >= 3 && w.starts_with(&head),
        None => false,
    }
}

fn push_unique(v: &mut Vec<String>, said: &mut HashSet<String>, item: String) {
    let k = item.to_lowercase();
    match (said.contains(&k), v.len() < MAX_ITEMS, item.trim().is_empty()) {
        (false, true, false) => {
            said.insert(k);
            v.push(item);
        }
        _ => {}
    }
}

fn wn_targets(eng: &Engine, sense: u32, syms: &[&str]) -> Vec<u32> {
    eng.g
        .out_edges(sense)
        .filter(|(_, e)| match &e.rel {
            Rel::Wn(s) => syms.contains(&s.as_str()),
            _ => false,
        })
        .map(|(_, e)| e.to)
        .collect()
}

fn hyper_targets(eng: &Engine, sense: u32) -> Vec<u32> {
    eng.g
        .out_edges(sense)
        .filter(|(_, e)| match &e.rel {
            Rel::Wn(s) => s == "@" || s == "@i",
            Rel::Bn(r) => r == "HYPERNYM",
            _ => false,
        })
        .map(|(_, e)| e.to)
        .collect()
}

/// Nyjet Word të së njëjtës fjalë: «madh», «i madh», «e madhe» nuk janë fjalë të ndryshme.
pub fn aliases(eng: &Engine, f: &Focus) -> Vec<u32> {
    let lemma = f.resolved.lemma.to_lowercase();
    let bare = crate::derive::strip_article(&lemma).to_string();
    let mut keys = vec![lemma.clone(), bare.clone(), format!("i {bare}"), format!("e {bare}")];
    keys.dedup();
    let mut out: Vec<u32> = Vec::new();
    for k in keys {
        match eng.g.get(&crate::graph::word_key(Lang::Sq, &k)) {
            Some(w) => match out.contains(&w) {
                true => {}
                false => out.push(w),
            },
            None => {}
        }
    }
    out
}

fn word_lemma(eng: &Engine, w: u32) -> String {
    match &eng.g.node(w).kind {
        NodeKind::Word { lemma, .. } => clean_label(lemma),
        _ => String::new(),
    }
}

/// Nyja e mbiemrit sipas burimeve: atributi `article` te `Rel::Lex` (Wiktionary, LEXIN)
/// ose fjala me nyjë («i bukur») si nyje në graf. Citimi i mbiemrit bëhet në mashkullore («i»).
fn adj_article(eng: &Engine, w: u32, lemma: &str) -> Option<&'static str> {
    let marked = eng
        .g
        .out_edges(w)
        .filter(|(_, e)| e.rel == Rel::Lex)
        .any(|(_, e)| matches!((e.attr("article"), e.attr("pos")), (Some("i" | "e"), Some("a") | None)));
    let node = eng.g.get(&word_key(Lang::Sq, &format!("i {lemma}"))).is_some();
    match marked || node {
        true => Some("i"),
        false => None,
    }
}

pub fn main_pos(eng: &Engine, w: u32) -> Option<Pos> {
    main_pos_of(eng, &[w])
}

/// Pjesa kryesore e ligjëratës së fjalës (bashkë me trajtat e saj me nyjë) sipas dëshmive të
/// pavarura: një votë për çdo (pjesë ligjërate, burim) — gramatika, UD, Wiktionary. Në barazim
/// vendos (1) pjesa që i jep LEXIN, (2) shënimi «mbiemër me nyjë», (3) emri, mbiemri, folja,
/// ndajfolja. Kur s'ka dëshmi të pavarur, vlen pjesa e ligjëratës e kuptimeve. (Një homograf i
/// rrallë nuk e mund fjalën e zakonshme: «derë» = emër me 4 burime, «i derë» = mbiemër me 1.)
pub fn main_pos_of(eng: &Engine, ws: &[u32]) -> Option<Pos> {
    let mut votes: BTreeSet<(Pos, Source)> = BTreeSet::new();
    let mut lexin: BTreeSet<Pos> = BTreeSet::new();
    let mut art_adj = false;
    for &w in ws {
        for (_, e) in eng.g.out_edges(w) {
            match (&e.rel, &eng.g.node(e.to).kind, e.source) {
                (Rel::HasSense, NodeKind::Sense { pos, .. }, Source::KaikkiEn) => {
                    votes.insert((*pos, Source::KaikkiEn));
                }
                (Rel::OccursIn, _, src) => match e.attr("upos").and_then(Pos::from_upos) {
                    Some(p) => {
                        votes.insert((p, src));
                    }
                    None => {}
                },
                (Rel::Lex, _, src) => match e.attr("pos").and_then(Pos::from_wn) {
                    Some(p) => {
                        votes.insert((p, src));
                        match src {
                            Source::Lexin => {
                                lexin.insert(p);
                            }
                            _ => {}
                        }
                        match (p, e.attr("article")) {
                            (Pos::Adj, Some("i" | "e")) => art_adj = true,
                            _ => {}
                        }
                    }
                    None => {}
                },
                _ => {}
            }
        }
    }
    match votes.is_empty() {
        true => {
            for &w in ws {
                for (sid, _) in eng.g.senses_of(w) {
                    match &eng.g.node(sid).kind {
                        NodeKind::Sense { pos, source, .. } => {
                            votes.insert((*pos, *source));
                        }
                        _ => {}
                    }
                }
            }
        }
        false => {}
    }
    let count = |p: Pos| votes.iter().filter(|(q, _)| *q == p).count();
    let pri = |p: Pos| -> (bool, u8) {
        let base = match p {
            Pos::Noun => 3u8,
            Pos::Adj => match art_adj {
                true => 4,
                false => 2,
            },
            Pos::Verb => 1,
            Pos::Adv => 0,
        };
        (lexin.contains(&p), base)
    };
    [Pos::Noun, Pos::Adj, Pos::Verb, Pos::Adv]
        .into_iter()
        .filter(|p| count(*p) > 0)
        .max_by_key(|p| (count(*p), pri(*p)))
}

/// Forma e cituar e një fjale shqipe dhe nëse ajo citohet ashtu si është (nuk shquhet si emër):
/// mbiemri me nyjën e tij («i bukur»), folja dhe ndajfolja si lemë («vij», «shpejt»);
/// emri si lemë që realizuesi e shquan («qen» → «qeni»).
pub fn display_word(eng: &Engine, w: u32) -> (String, bool) {
    let lemma = word_lemma(eng, w);
    match (article_of(&lemma).is_some(), main_pos(eng, w)) {
        (true, _) => (lemma, true),
        (false, Some(Pos::Adj)) => match adj_article(eng, w, &lemma) {
            Some(a) => (format!("{a} {lemma}"), true),
            None => (lemma, true),
        },
        (false, Some(Pos::Verb) | Some(Pos::Adv)) => (lemma, true),
        (false, _) => (lemma, false),
    }
}

/// Forma e cituar e një fjale të dhënë si tekst (p.sh. nga ConceptNet), me nyjën e saj kur
/// fjala është në graf; trajtat e lakuara kthehen në lemë.
fn cite_term(eng: &Engine, t: &str) -> (String, Option<u32>) {
    match eng.g.get(&word_key(Lang::Sq, t)) {
        Some(w) => {
            let w = lemma_of_form(eng, w);
            (display_word(eng, w).0, Some(w))
        }
        None => (t.to_string(), None),
    }
}

/// Trajta e lakuar e një mbiemri ose emri (femërore, shumës) zëvendësohet me lemën e saj:
/// «ligë» → «lig», «zezë» → «zi». Pjesoret nuk preken (janë fjalë më vete: «pasur», «dashur»).
fn lemma_of_form(eng: &Engine, w: u32) -> u32 {
    eng.g
        .out_edges(w)
        .find(|(_, e)| {
            let tags = e.attr("tags").unwrap_or("");
            let ip = independent_pos(eng, e.to);
            is_sem(&e.rel, "FORM_OF")
                && (tags.contains("feminine") || tags.contains("plural"))
                && !tags.contains("participle")
                && (ip.contains(&Pos::Adj) || ip.contains(&Pos::Noun))
        })
        .map(|(_, e)| e.to)
        .unwrap_or(w)
}



/// Nyja e mbiemrit (nëse ka) në fillim të formës së cituar.
fn article_of(s: &str) -> Option<&str> {
    match s.split_whitespace().next() {
        Some(a @ ("i" | "e" | "të")) => match s.contains(' ') {
            true => Some(a),
            false => None,
        },
        _ => None,
    }
}

/// Mohimi me pa-/mos- e ruan pjesën e ligjëratës: «i bukur» → «i pabukur».
fn agree_article(focus: &str, other: &str) -> String {
    match (article_of(focus), article_of(other)) {
        (Some(a), None) => format!("{a} {other}"),
        _ => other.to_string(),
    }
}

/// Një kandidat i thënshëm (sinonim, e kundërt, kategori…) me dëshmitë e tij.
#[derive(Clone, Debug)]
struct Cand {
    word: String,
    /// Burimet që e dëshmojnë në kuptimin kryesor: një dëshmi = një burim i pavarur.
    sources: BTreeSet<Source>,
    /// Grupet e kuptimit që e dëshmojnë (p.sh. çdo grup përkthimesh i LEXIN), pa dyfishim:
    /// skaji i kundërt i të njëjtit grup nuk numërohet sërish.
    groups: BTreeSet<String>,
    /// Renditja e burimit më të besueshëm.
    rank: u8,
    /// Rendi i parë i takimit (rendi i fjalorit), për barazimet.
    first: usize,
    node: Option<u32>,
}

fn add_cand(m: &mut Vec<Cand>, word: String, node: Option<u32>, src: Source, primary: bool) {
    add_cand_g(m, word, node, src, primary, src.code().to_string())
}

fn add_cand_g(m: &mut Vec<Cand>, word: String, node: Option<u32>, src: Source, primary: bool, group: String) {
    let key = crate::derive::strip_article(&word).to_string();
    let n = m.len();
    match m.iter_mut().find(|x| crate::derive::strip_article(&x.word) == key) {
        Some(x) => {
            match primary {
                true => {
                    x.sources.insert(src);
                    x.groups.insert(group);
                }
                false => {}
            }
            x.rank = x.rank.min(source_rank(src));
            match (article_of(&x.word), article_of(&word)) {
                (None, Some(_)) => x.word = word,
                _ => {}
            }
            match (x.node, node) {
                (None, Some(_)) => x.node = node,
                _ => {}
            }
        }
        None => m.push(Cand {
            word,
            sources: match primary {
                true => BTreeSet::from([src]),
                false => BTreeSet::new(),
            },
            groups: match primary {
                true => BTreeSet::from([group]),
                false => BTreeSet::new(),
            },
            rank: source_rank(src),
            first: n,
            node,
        }),
    }
}



/// Renditja e fiksuar: më shumë burime para; pastaj burimi më i besueshëm; pastaj më shumë
/// skaje; pastaj rendi i fjalorit (jo alfabeti).
fn ranked(mut m: Vec<Cand>) -> Vec<Cand> {
    m.sort_by(|a, b| {
        b.sources
            .len()
            .cmp(&a.sources.len())
            .then(a.rank.cmp(&b.rank))
            .then(b.groups.len().cmp(&a.groups.len()))
            .then(a.first.cmp(&b.first))
    });
    m
}

/// Kandidatët e dëshmuar (në kuptimin kryesor) dhe të thënshëm, të renditur, deri në `cap`.
fn listed(m: Vec<Cand>, cap: usize) -> (Vec<String>, BTreeSet<Source>) {
    let mut r = ranked(m.into_iter().filter(|x| !x.sources.is_empty() && lexical(&x.word)).collect());
    r.truncate(cap);
    let src: BTreeSet<Source> = r.iter().flat_map(|x| x.sources.iter().copied()).collect();
    (r.into_iter().map(|x| x.word).collect(), src)
}

/// Zgjedhja për t'u thënë. Kur disa kandidatë kanë ≥2 burime, thuhen vetëm ata (deri në 3).
/// Ndryshe deri në 2 me një burim të vetëm, vetëm ata që i pranon `verified`.
fn chosen(m: Vec<Cand>, verified: &dyn Fn(&Cand) -> bool) -> (Vec<String>, BTreeSet<Source>) {
    let m: Vec<Cand> = m.into_iter().filter(|x| !x.sources.is_empty() && lexical(&x.word)).collect();
    let best = m.iter().map(|x| x.sources.len()).max().unwrap_or(0);
    // Leksiku i modelit është një dëshmitar: numërohet te «≥2 burime», por vetëm nuk mjafton.
    match best >= 2 {
        true => listed(m.into_iter().filter(|x| x.sources.len() >= 2).collect(), MAX_ITEMS),
        false => listed(m.into_iter().filter(|x| verified(x) && !model_only(x)).collect(), 2),
    }
}

/// Kandidati që e dëshmon vetëm leksiku i modelit (pa asnjë burim tjetër).
fn model_only(c: &Cand) -> bool {
    c.sources.iter().all(|s| *s == Source::ModelClaude)
}

/// Fakti gramatikor i fjalës sipas gramatikës (uniparser, Wiktionary, Wikidata): pjesa kryesore
/// e ligjëratës dhe, për emrat, gjinia që e jep shumica e burimeve. P.sh. ["emër", "femëror"].
pub fn grammar_fact(eng: &Engine, w: u32) -> Vec<String> {
    let lex: Vec<&crate::graph::Edge> = eng.g.out_edges(w).filter(|(_, e)| e.rel == Rel::Lex).map(|(_, e)| e).collect();
    let gender = |g: &str| lex.iter().filter(|e| e.attr("pos") == Some("n") && e.attr("gender") == Some(g)).count();
    let (m, fm, n) = (gender("m"), gender("f"), gender("n"));
    let g = match (m, fm, n) {
        (0, 0, 0) => None,
        _ => [(m, "mashkullor"), (fm, "femëror"), (n, "asnjanës")].into_iter().max_by_key(|(c, _)| *c).map(|x| x.1),
    };
    match (lex.is_empty(), main_pos(eng, w), g) {
        (true, _, _) => vec![],
        (false, Some(Pos::Noun), Some(g)) => vec!["emër".into(), g.into()],
        (false, Some(Pos::Noun), None) => vec!["emër".into()],
        (false, Some(Pos::Adj), _) => vec!["mbiemër".into()],
        (false, Some(Pos::Verb), _) => vec!["folje".into()],
        (false, Some(Pos::Adv), _) => vec!["ndajfolje".into()],
        (false, None, _) => vec![],
    }
}

/// Termi i fjalës për planin e shprehjes: mbiemrat, foljet dhe ndajfoljet citohen, emrat shquhen.
pub fn term_of(eng: &Engine, w: Option<u32>, lemma: &str) -> TermRef {
    match w {
        Some(w) => match display_word(eng, w) {
            (l, true) => TermRef::cited(&l),
            (l, false) => TermRef::word(&l),
        },
        None => TermRef::word(lemma),
    }
}

/// Pjesa e ligjëratës së fokusit (mbi të gjitha trajtat e tij) për filtrimin e kandidatëve.
fn focus_pos_of(eng: &Engine, words: &[u32]) -> BTreeSet<Pos> {
    match main_pos_of(eng, words) {
        Some(p) => BTreeSet::from([p]),
        None => BTreeSet::new(),
    }
}

/// Kandidati ka një pjesë ligjërate të përbashkët me fokusin. Një fjalë që gramatika e njeh,
/// por pa pjesë ligjërate të hapur (parafjalë, lidhëz: «përgjatë»), nuk është sinonim i një
/// mbiemri; një fjalë pa asnjë shënim gramatikor pranohet.
fn pos_ok(eng: &Engine, focus_pos: &BTreeSet<Pos>, o: u32) -> bool {
    let mut ip = independent_pos(eng, o);
    // Një etiketë me nyjë («i mbyllur») është mbiemër, edhe kur gramatika s'e ka si hyrje.
    match article_of(&word_lemma(eng, o)) {
        Some("i" | "e" | "të") => {
            ip.insert(Pos::Adj);
        }
        _ => {}
    }
    let has_lex = eng.g.out_edges(o).any(|(_, e)| e.rel == Rel::Lex);
    match (focus_pos.is_empty(), ip.is_empty(), has_lex) {
        (true, _, _) => true,
        (false, true, true) => false,
        (false, true, false) => true,
        (false, false, _) => ip.intersection(focus_pos).next().is_some(),
    }
}

/// Forma e cituar e kandidatit në pjesën e ligjëratës ku përputhet me fokusin: mbiemri me
/// nyjë kur fokusi është mbiemër, edhe kur kandidati është më shpesh emër («i kripur»).
fn cite_as(eng: &Engine, o: u32, focus_pos: &BTreeSet<Pos>) -> String {
    let lemma = word_lemma(eng, o);
    let ip = independent_pos(eng, o);
    match (focus_pos.contains(&Pos::Adj) && ip.contains(&Pos::Adj), article_of(&lemma).is_some()) {
        (true, false) => match adj_article(eng, o, &lemma) {
            Some(a) => format!("{a} {lemma}"),
            None => lemma,
        },
        (true, true) => lemma,
        (false, _) => display_word(eng, o).0,
    }
}

/// Burimet sipas besueshmërisë për lidhjet në nivel fjale (renditje e fiksuar).
fn source_rank(s: Source) -> u8 {
    match s {
        Source::Lexin => 0,
        Source::KaikkiDe => 1,
        Source::KaikkiEn => 2,
        Source::SqRules => 3,
        Source::WdLexemes => 4,
        Source::Loanwords | Source::Dialects | Source::Gazulli1941 => 5,
        Source::EuroVoc => 6,
        _ => 7,
    }
}

fn is_sem(r: &Rel, name: &str) -> bool {
    match r {
        Rel::Sem(x) => x == name,
        _ => false,
    }
}

/// Fjalët e lidhura me fjalën me një marrëdhënie `Rel::Sem`, të renditura sipas dëshmive.
fn sem_targets(eng: &Engine, words: &[u32], rel: &str, reverse: bool) -> (Vec<String>, BTreeSet<Source>) {
    let mut m: Vec<Cand> = Vec::new();
    sem_cands(eng, words, rel, reverse, false, &focus_pos_of(eng, words), &mut m);
    listed(m, MAX_ITEMS)
}

/// Kandidatët nga skajet `Rel::Sem(rel)` të fjalës (ose drejt saj kur `reverse`), me pjesë
/// ligjërate të pajtueshme me fokusin; trajtat e lakuara kthehen në lemë.
/// `directional`: grupi mban edhe drejtimin («:f» / «:r»), që të dallohet reciprociteti
/// (fjala A e jep B-në si të kundërt dhe B e jep A-në).
fn sem_cands(eng: &Engine, words: &[u32], rel: &str, reverse: bool, directional: bool, focus_pos: &BTreeSet<Pos>, m: &mut Vec<Cand>) {
    let dir = match (directional, reverse) {
        (false, _) => "",
        (true, false) => ":f",
        (true, true) => ":r",
    };
    for &w in words {
        let group = |e: &crate::graph::Edge| format!("{}:{}{dir}", e.source.code(), e.attr("sense").unwrap_or(""));
        let edges: Vec<(u32, Source, bool, String)> = match reverse {
            false => eng
                .g
                .out_edges(w)
                .filter(|(_, e)| is_sem(&e.rel, rel) && e.attr("confidence") != Some("low"))
                .map(|(_, e)| (e.to, e.source, primary_sense(e.source, e.attr("sense")), group(e)))
                .collect(),
            true => eng
                .g
                .in_edges(w)
                .filter(|(_, e)| is_sem(&e.rel, rel) && e.attr("confidence") != Some("low"))
                .map(|(_, e)| (e.from, e.source, primary_sense(e.source, e.attr("sense")), group(e)))
                .collect(),
        };
        for (o, src, primary, grp) in edges {
            let o = lemma_of_form(eng, o);
            match (words.contains(&o), pos_ok(eng, focus_pos, o)) {
                (false, true) => add_cand_g(m, cite_as(eng, o, focus_pos), Some(o), src, primary, grp),
                _ => {}
            }
        }
    }
}

/// Dëshmitari i vërtetë pas një lidhjeje të ConceptNet: lidhjet e nxjerra nga Wiktionary
/// anglisht ose gjermanisht janë i njëjti burim si Wiktionary-t që importohen drejtpërdrejt
/// (nuk numërohen dy herë); të tjerat (p.sh. Wiktionary frëngjisht) janë ConceptNet.
fn cn_witness(dataset: &str) -> Source {
    match (dataset.starts_with("/d/wiktionary/en"), dataset.starts_with("/d/wiktionary/de")) {
        (true, _) => Source::KaikkiEn,
        (_, true) => Source::KaikkiDe,
        _ => Source::ConceptNet57,
    }
}

/// Fjalët shqipe të lidhura me fjalën në ConceptNet (Wiktionary), në formën e cituar dhe me
/// dëshmitarin e tyre. Lidhjet që ConceptNet i ka nga WordNet-i nuk numërohen (janë AlbaNet).
fn cn_terms(eng: &Engine, f: &Focus, words: &[u32], rel: &str, outgoing_only: bool, focus_pos: &BTreeSet<Pos>) -> Vec<(String, Option<u32>, Source)> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for r in f.form_rels.iter().filter(|r| {
        r.rel == rel && r.other_lang == Some(Lang::Sq) && !r.dataset.starts_with("/d/wordnet") && (r.outgoing || !outgoing_only)
    }) {
        let (t, node) = cite_term(eng, &r.other_term.to_lowercase());
        let t = node.map(|n| cite_as(eng, n, focus_pos)).unwrap_or(t);
        // Fjala duhet ta njohë një burim shqip i pavarur (gramatikë, UD, Wiktionary): një trajtë
        // foljore si «shtrëngoi» nuk është sinonim i «akne».
        let ok_pos = match node {
            Some(n) => !independent_pos(eng, n).is_empty() && pos_ok(eng, focus_pos, n),
            None => t.contains(' '),
        };
        let own = node.map(|n| words.contains(&n)).unwrap_or(false);
        let src = cn_witness(&r.dataset);
        match (ok_pos, own, seen.insert(format!("{}:{}", crate::derive::strip_article(&t), src.code()))) {
            (true, false, true) => out.push((t, node, src)),
            _ => {}
        }
    }
    out
}

/// Prejardhjet sipas rregullave: fjali nga shablloni i rregullit ({d}/{b} = e prejardhura dhe
/// baza në formën e cituar; {db}/{bb} = pa nyjë), dhe të kundërtat që rregullat i dëshmojnë.
fn derivations(eng: &Engine, words: &[u32]) -> (Vec<String>, Vec<String>) {
    let mut origin = Vec::new();
    let mut opposites = Vec::new();
    for &w in words {
        for (_, e) in eng.g.out_edges(w) {
            match &e.rel {
                Rel::Deriv(rel) => {
                    let d0 = display_word(eng, w).0;
                    let b0 = display_word(eng, e.to).0;
                    // Mohimi e ruan pjesën e ligjëratës: nyja vlen për të dyja anët.
                    let (d, b) = match (rel.as_str(), article_of(&d0), article_of(&b0)) {
                        ("NEGATION_OF", None, Some(a)) => (format!("{a} {d0}"), b0.clone()),
                        ("NEGATION_OF", Some(_), None) => (d0.clone(), agree_article(&d0, &b0)),
                        _ => (d0.clone(), b0.clone()),
                    };
                    match rel.as_str() {
                        "NEGATION_OF" | "REVERSAL_OF" | "CONTRARY_TO" => opposites.push(b.clone()),
                        _ => {}
                    }
                    let tpl = e.attr("gloss").unwrap_or("");
                    let db = crate::derive::strip_article(&d).to_string();
                    let bb = crate::derive::strip_article(&b).to_string();
                    match tpl.is_empty() {
                        true => {}
                        false => origin.push(
                            tpl.replace("{db}", &db).replace("{bb}", &bb).replace("{d}", &d).replace("{b}", &b),
                        ),
                    }
                }
                _ => {}
            }
        }
        for (_, e) in eng.g.in_edges(w) {
            match &e.rel {
                Rel::Deriv(rel) => match rel.as_str() {
                    "NEGATION_OF" => opposites.push(agree_article(&display_word(eng, w).0, &display_word(eng, e.from).0)),
                    "REVERSAL_OF" | "CONTRARY_TO" => opposites.push(display_word(eng, e.from).0),
                    _ => {}
                },
                _ => {}
            }
        }
    }
    (origin, opposites)
}

/// Fjala është e dëshmuar në një burim shqip jashtë Wiktionary-t (gramatikë, LEXIN, Wikidata,
/// UD, AlbaNet): një etimon i rindërtuar ose i vjetruar («vëharë») nuk thuhet si bazë.
fn attested_beyond_wiktionary(eng: &Engine, n: u32) -> bool {
    let src_ok = |s: Source| {
        matches!(s, Source::Uniparser | Source::Lexin | Source::WdLexemes | Source::UdSqTsa | Source::UdSqStaf | Source::OmwAls)
    };
    eng.g.out_edges(n).any(|(_, e)| src_ok(e.source)) || eng.g.in_edges(n).any(|(_, e)| src_ok(e.source))
}

/// Bazat nga të cilat rrjedh fjala sipas fjalorëve (Wiktionary, LEXIN, Wikidata:
/// `DERIVED_FROM`), deri në dy. Nuk merren: lidhjet nga faqet e emrave të përveçëm
/// («Rexhep» → «xhep»), bazat që vetëm Wiktionary i njeh, dhe `/r/DerivedFrom` i ConceptNet
/// (nxjerrje automatike me drejtim shpesh të gabuar: «borë» → «bjerr»).
fn derived_bases(eng: &Engine, f: &Focus, words: &[u32]) -> (Vec<String>, BTreeSet<Source>) {
    let _ = f;
    let mut c: Vec<Cand> = Vec::new();
    for &w in words {
        for (_, e) in eng.g.out_edges(w).filter(|(_, e)| is_sem(&e.rel, "DERIVED_FROM")) {
            let o = lemma_of_form(eng, e.to);
            let name = e.attr("sense").unwrap_or("").contains(":name:");
            let base = word_lemma(eng, o);
            // Forma e rindërtuar (Wiktionary e shënon «reconstruction») nuk është fjalë e gjuhës.
            let rebuilt = eng.g.out_edges(o).any(|(_, x)| x.rel == Rel::HasSense && x.attr("tags").unwrap_or("").contains("reconstruction"));
            match (words.contains(&o), name || rebuilt, base.contains('*'), attested_beyond_wiktionary(eng, o)) {
                (false, false, false, true) => add_cand(&mut c, display_word(eng, o).0, Some(o), e.source, true),
                _ => {}
            }
        }
    }
    // Baza që e jep vetëm leksiku i modelit nuk thuhet (vlen vetëm si konfirmim i një rregulli).
    listed(c.into_iter().filter(|x| !model_only(x)).collect(), 2)
}

/// Emri shqip (në trajtën e shquar) i gjuhës së prejardhjes: «nga anglishtja».
pub fn lang_def(code: &str) -> Option<&'static str> {
    match code {
        "en" | "english" => Some("anglishtja"),
        "it" | "italian" => Some("italishtja"),
        "fr" | "french" => Some("frëngjishtja"),
        "de" | "german" => Some("gjermanishtja"),
        "tr" | "turk" => Some("turqishtja"),
        "ota" | "ottoman" => Some("turqishtja osmane"),
        "el" | "grc" | "greek" => Some("greqishtja"),
        "la" | "lat" => Some("latinishtja"),
        "es" => Some("spanjishtja"),
        "ru" => Some("rusishtja"),
        "ro" => Some("rumanishtja"),
        "sh" | "sr" | "hr" => Some("serbokroatishtja"),
        "bg" => Some("bullgarishtja"),
        "mk" => Some("maqedonishtja"),
        "ar" => Some("arabishtja"),
        "fa" => Some("persishtja"),
        "hu" => Some("hungarishtja"),
        "pt" => Some("portugalishtja"),
        "slavic" => Some("një gjuhë sllave"),
        _ => None,
    }
}

/// Prejardhja e një huazimi. Gramatika uniparser (`borr`, vetëm për fjalë jo të prejardhura)
/// është dëshmia kryesore: «X» është huazim nga {gjuha}. Etimologjia e Wiktionary-t në
/// ConceptNet («X» vjen nga {gjuha} «etimon») thuhet vetëm kur e pajton gramatika: ConceptNet
/// i ngatërron shpesh fjalët e trashëguara me huazimet (p.sh. «qen» ~ latinisht «canis» është
/// fjalë e afërt, jo burim). Lista e huazimeve jep vetëm «X» është fjalë e huaj.
fn borrow_origin(eng: &Engine, f: &Focus, words: &[u32], d: &str) -> Option<(String, Source)> {
    let lang_key = |c: &str| -> &'static str {
        match c {
            "en" | "english" => "en",
            "it" | "italian" => "it",
            "fr" | "french" => "fr",
            "tr" | "ota" | "turk" | "ottoman" => "tr",
            "el" | "grc" | "greek" => "el",
            _ => "?",
        }
    };
    // Vetëm leksema e pjesës kryesore të ligjëratës së fjalës (jo një homograf: «i lirë» ≠
    // «lirë» emër i huazuar).
    let main = main_pos_of(eng, words);
    let borr: Option<String> = words
        .iter()
        .flat_map(|&w| {
            eng.g
                .out_edges(w)
                .filter(|(_, e)| {
                    e.rel == Rel::Lex
                        && !e.attr("flags").unwrap_or("").contains("deriv")
                        && (main.is_none() || e.attr("pos").and_then(Pos::from_wn) == main)
                })
                .filter_map(|(_, e)| e.attr("borr").map(|b| b.to_string()))
                .collect::<Vec<_>>()
        })
        .next()
        .filter(|b| matches!(b.as_str(), "en" | "it" | "fr" | "turk" | "greek"));
    let listed_loan = words.iter().any(|&w| eng.g.out_edges(w).any(|(_, e)| is_sem(&e.rel, "NATIVE_EQUIVALENT")));
    let ety = borr.as_ref().and_then(|b| {
        f.form_rels.iter().find(|r| {
            r.rel == "/r/EtymologicallyDerivedFrom" && r.outgoing && lang_key(&r.other_lang_code) == lang_key(b)
        })
    });
    match (ety, borr, listed_loan) {
        // Etimoni në shkrim jo-latin (arabisht, greqisht…) nuk thuhet: vetëm gjuha.
        (Some(r), _, _) => match r.other_term.chars().any(|c| c as u32 >= 0x0370) {
            false => Some((
                format!("«{d}» vjen nga {} «{}»", lang_def(&r.other_lang_code).unwrap_or(""), r.other_term),
                Source::Uniparser,
            )),
            true => Some((format!("«{d}» vjen nga {}", lang_def(&r.other_lang_code).unwrap_or("")), Source::Uniparser)),
        },
        (None, Some(b), _) => Some((format!("«{d}» është huazim nga {}", lang_def(&b).unwrap_or("")), Source::Uniparser)),
        (None, None, true) => Some((format!("«{d}» është fjalë e huaj"), Source::Loanwords)),
        (None, None, false) => None,
    }
}



/// Faktet në nivel FJALE (pa kuptim të caktuar): sinonime, të kundërta, kategori,
/// lloje, prejardhje, fjala shqipe për huazimin, trajta standarde e dialektit.
pub fn gather_word(eng: &Engine, f: &Focus) -> Facts {
    let lemma = crate::derive::strip_article(&f.resolved.lemma.to_lowercase()).to_string();
    let words = aliases(eng, f);
    let mut said: HashSet<String> =
        HashSet::from([lemma.clone(), f.resolved.lemma.to_lowercase(), format!("i {lemma}"), format!("e {lemma}")]);
    // Një frazë që përmban vetë fjalën («pi alkool» për «pi») nuk është sinonim.
    let fill = |items: Vec<String>, said: &mut HashSet<String>| -> Vec<String> {
        let mut v = Vec::new();
        for i in items {
            match i.split_whitespace().any(|t| t == lemma) {
                true => {}
                false => push_unique(&mut v, said, i),
            }
        }
        v
    };
    let focus_pos = focus_pos_of(eng, &words);
    let mut src: Vec<(Facet, Vec<Source>)> = Vec::new();

    // Sinonimet. Një burim i vetëm (p.sh. një grup përkthimesh i LEXIN) nuk mjafton vetëm kur
    // përkthimi anglisht i kandidatit dhe i fjalës nuk takohen: «det» e «liqen» (sjö) nuk
    // ndajnë asnjë përkthim (sea ≠ lake). Kur kandidati s'ka përkthim, pranohet vetëm nga një
    // listë e qartë sinonimesh (Wiktionary, Wikidata, ConceptNet), jo nga bashkëpërkthimi.
    // Sinonimia është simetrike: lidhja «bajrak → flamur» dëshmon edhe «flamur → bajrak».
    let mut syn_c: Vec<Cand> = Vec::new();
    sem_cands(eng, &words, "SYNONYM", false, false, &focus_pos, &mut syn_c);
    sem_cands(eng, &words, "SYNONYM", true, false, &focus_pos, &mut syn_c);
    for (t, node, src) in cn_terms(eng, f, &words, "/r/Synonym", false, &focus_pos) {
        add_cand(&mut syn_c, t, node, src, true);
    }
    // Përkthimet kryesore të fjalës (kuptimi i parë në Wiktionary, fjalori) dhe të gjitha
    // përkthimet e saj (edhe etiketat e kuptimeve në AlbaNet).
    let focus_en: HashSet<String> = words
        .iter()
        .flat_map(|&w| english_bridge(eng, w))
        .chain(dict_en(eng, f).into_iter().map(|x| x.to_lowercase()))
        .collect();
    let focus_all: HashSet<String> = words.iter().flat_map(|&w| english_all(eng, w)).chain(focus_en.iter().cloned()).collect();
    // Fjalët që burimet i lidhin me fjalën si lloj ose kategori nuk janë sinonime («dushk» ⊂ «lis»).
    let kin: HashSet<u32> = words
        .iter()
        .flat_map(|&w| {
            eng.g
                .out_edges(w)
                .filter(|(_, e)| is_sem(&e.rel, "HYPERNYM") || is_sem(&e.rel, "HYPONYM"))
                .map(|(_, e)| e.to)
                .chain(
                    eng.g
                        .in_edges(w)
                        .filter(|(_, e)| is_sem(&e.rel, "HYPERNYM") || is_sem(&e.rel, "HYPONYM"))
                        .map(|(_, e)| e.from),
                )
                .collect::<Vec<_>>()
        })
        .collect();
    // Bashkëpërkthimi në ≥3 grupe të ndryshme të LEXIN (tri fjalë suedeze që përkthehen me
    // të dyja: «bukur» dhe «mirë») është dëshmi e fortë edhe pa përkthim të përbashkët anglisht.
    let hyponym = |c: &Cand| -> bool {
        let raw = c.node.map(|n| english_raw(eng, n)).unwrap_or_default();
        raw.iter().any(|e| focus_en.iter().any(|f| e.ends_with(&format!(" {f}")) && !focus_en.contains(e)))
    };
    // Kontrolli i burimit të vetëm: (1) ≥3 grupe bashkëpërkthimi në LEXIN; (2) përkthimi
    // KRYESOR i kandidatit është edhe përkthim kryesor i fjalës (jo një kuptim i rrallë i një
    // fjale angleze shumëkuptimëshe: «vaj» e «naftë» ndajnë «oil», por jo kuptimin kryesor);
    // (3) një listë e qartë sinonimesh (jo LEXIN) për një fjalë pa përkthim për ta krahasuar.
    let syn_ok = |c: &Cand| -> bool {
        let primary = c.node.map(|n| english_bridge(eng, n)).unwrap_or_default();
        let explicit = c.sources.iter().any(|s| !matches!(s, Source::Lexin));
        let lexin_groups = c.groups.iter().filter(|g| g.starts_with("lexin:")).count();
        match (lexin_groups >= 3, primary.iter().any(|e| focus_en.contains(e)), explicit, primary.is_empty()) {
            (true, _, _, _) | (_, true, _, _) | (_, _, true, true) => true,
            _ => false,
        }
    };
    // Për çdo kandidat (edhe me shumë burime): kur të dyja fjalët kanë përkthim anglisht, ato
    // duhet të ndajnë të paktën një — ndryshe sinonimi i përket një kuptimi tjetër
    // («mal» = pyll në një dialekt; «zemër» = «i dashur» figurativisht).
    let aligned = |c: &Cand| -> bool {
        let cp: HashSet<String> = c.node.map(|n| english_bridge(eng, n).into_iter().collect()).unwrap_or_default();
        let ce = c.node.map(|n| english_all(eng, n)).unwrap_or_default();
        // Kur të dyja fjalët kanë përkthim kryesor, krahasohen ato («qafë» = neck, «jakë» = collar);
        // ndryshe çdo përkthim («vaj» e «naftë» ndajnë «oil»).
        // Përkthimi i fjalës krahasohet me kuptimin e saj KRYESOR kur dihet («zemër» = heart; jo
        // «i dashur» = darling, kuptim figurativ).
        let cand: &HashSet<String> = match cp.is_empty() {
            false => &cp,
            true => &ce,
        };
        let focus: &HashSet<String> = match focus_en.is_empty() {
            false => &focus_en,
            true => &focus_all,
        };
        match (cand.is_empty(), focus.is_empty()) {
            (false, false) => cand.iter().any(|e| focus.contains(e)),
            _ => true,
        }
    };
    // Kandidati që përkthehet «X + fjala» («white horse» për «horse») është lloj i fjalës, jo sinonim.
    let syn_c: Vec<Cand> = syn_c
        .into_iter()
        .filter(|c| !hyponym(c) && aligned(c) && !c.node.map(|n| kin.contains(&n)).unwrap_or(false))
        .collect();
    let (syn, syn_src) = chosen(syn_c, &syn_ok);
    let synonyms = fill(syn, &mut said);
    src.push((Facet::Synonyms, syn_src.into_iter().collect()));

    let (opp, opp_src) = opposites_of(eng, f, &words, &focus_pos);
    let opposites = fill(opp, &mut said);
    src.push((Facet::Opposites, opp_src.into_iter().collect()));

    // Kategoria në nivel fjale: një fjalë ka rrallë më shumë se dy kategori të drejtpërdrejta.
    // Kur burimet japin 3 e më shumë, lista zakonisht është e llojeve (hiponimeve) e futur
    // gabim si hiperonime (p.sh. «kalë» → «balash, bardho, dori»): atëherë pranohen vetëm
    // kategoritë që i dëshmojnë të paktën dy burime.
    let mut cat_c: Vec<Cand> = Vec::new();
    sem_cands(eng, &words, "HYPERNYM", false, false, &focus_pos, &mut cat_c);
    let cat_c: Vec<Cand> = cat_c.into_iter().filter(|x| !x.sources.is_empty()).collect();
    let cat_c: Vec<Cand> = match cat_c.len() >= 3 {
        true => cat_c.into_iter().filter(|x| x.sources.len() >= 2).collect(),
        false => cat_c,
    };
    // Kategoria me një burim të vetëm verifikohet nga një burim tjetër: përkthimi anglisht i
    // fjalës (Wiktionary) → kuptimet e WordNet-it → paraardhësit e tyre. Kategoria thuhet
    // vetëm kur del ndër ata paraardhës (si fjalë shqipe ose përmes përkthimit të saj).
    // P.sh. «nënë» (mother) → prind, i afërm, njeri…: «familje» nuk është ndër ta.
    let bridge: Vec<String> = words.iter().flat_map(|&w| english_bridge(eng, w)).collect();
    let cat_c: Vec<Cand> = match (bridge.is_empty(), cat_c.iter().any(|x| x.sources.len() < 2)) {
        (false, true) => {
            let (anc_sq, anc_en) = bridge_ancestors(eng, &bridge);
            cat_c
                .into_iter()
                .filter(|x| {
                    let bare = crate::derive::strip_article(&x.word).to_string();
                    let via_en = x.node.map(|c| english_bridge(eng, c).iter().any(|b| anc_en.contains(b))).unwrap_or(false);
                    x.sources.len() >= 2 || anc_sq.contains(&bare) || via_en
                })
                .collect()
        }
        // Pa urë për verifikim, kategoria që e jep vetëm modeli nuk thuhet.
        _ => cat_c.into_iter().filter(|x| !model_only(x)).collect(),
    };
    let (cat, cat_src) = listed(cat_c, MAX_ITEMS);
    let category = fill(cat, &mut said);
    src.push((Facet::Category, cat_src.into_iter().collect()));

    // Llojet: hiponimet e dhëna nga burimet, dhe fjalët që e japin këtë fjalë si kategori të
    // tyre (kur ato vetë nuk japin ≥3 kategori nga i njëjti burim), plus «/r/IsA» drejt saj.
    let mut kinds_c: Vec<Cand> = Vec::new();
    sem_cands(eng, &words, "HYPONYM", false, false, &focus_pos, &mut kinds_c);
    for &w in &words {
        for (_, e) in eng.g.in_edges(w).filter(|(_, e)| is_sem(&e.rel, "HYPERNYM") && e.attr("confidence") != Some("low")) {
            let fanout = eng.g.out_edges(e.from).filter(|(_, x)| is_sem(&x.rel, "HYPERNYM") && x.source == e.source).count();
            match (fanout < 3, words.contains(&e.from)) {
                (true, false) => add_cand(
                    &mut kinds_c,
                    display_word(eng, e.from).0,
                    Some(e.from),
                    e.source,
                    primary_sense(e.source, e.attr("sense")),
                ),
                _ => {}
            }
        }
    }
    for r in f.form_rels.iter().filter(|r| {
        r.rel == "/r/IsA" && !r.outgoing && r.other_lang == Some(Lang::Sq) && !r.dataset.starts_with("/d/wordnet")
    }) {
        let (t, node) = cite_term(eng, &r.other_term.to_lowercase());
        match node.map(|n| words.contains(&n)).unwrap_or(false) {
            true => {}
            false => add_cand(&mut kinds_c, t, node, Source::ConceptNet57, true),
        }
    }
    let (kinds_l, kinds_src) = listed(kinds_c.into_iter().filter(|x| !model_only(x)).collect(), MAX_ITEMS);
    let kinds = fill(kinds_l, &mut said);
    src.push((Facet::Kinds, kinds_src.into_iter().collect()));

    let (native_l, native_src) = sem_targets(eng, &words, "NATIVE_EQUIVALENT", false);
    let native = fill(native_l, &mut said);
    src.push((Facet::Native, native_src.into_iter().collect()));
    let (dialect_l, dialect_src) = sem_targets(eng, &words, "DIALECT_VARIANT_OF", false);
    let dialect = fill(dialect_l, &mut said);
    src.push((Facet::Dialect, dialect_src.into_iter().collect()));

    // Prejardhja, sipas përparësisë: rregullat e fjalëformimit; fjalorët (rrjedh nga …);
    // huazimi. Një fjali e vetme, e njëjtë për çdo pyetje.
    let d = words.first().map(|&w| display_word(eng, w).0).unwrap_or_else(|| f.resolved.lemma.clone());
    let (rule_origin, _) = derivations(eng, &words);
    let (bases, bases_src) = derived_bases(eng, f, &words);
    let quoted: Vec<String> = bases.iter().map(|b| format!("«{b}»")).collect();
    // Homografët me prejardhje të ndryshme («pishinë» pyll pishash ← «pishë»; «pishinë» swimming
    // pool, huazim nga italishtja): kur përkthimin kryesor të fjalës e ka vetëm leksema e huazuar,
    // prejardhja thuhet sipas huazimit.
    // Përkthimi kryesor: ura e Wiktionary-t (një fjalë) dhe glosa e parë e plotë («swimming pool»).
    let main_en: Vec<String> = words
        .iter()
        .flat_map(|&w| english_bridge(eng, w).into_iter().chain(english_raw(eng, w).into_iter().take(1)))
        .map(|x| x.to_lowercase())
        .collect();
    let lex_en = |e: &crate::graph::Edge| {
        e.attr("en").map(|x| x.split([',', ';']).any(|t| main_en.contains(&t.trim().to_lowercase()))).unwrap_or(false)
    };
    let lexemes: Vec<&crate::graph::Edge> = words
        .iter()
        .flat_map(|&w| {
            eng.g.out_edges(w).filter(|(_, e)| e.rel == Rel::Lex && e.source == Source::Uniparser).map(|(_, e)| e).collect::<Vec<_>>()
        })
        .collect();
    // Leksema e huazuar drejtpërdrejt (jo e prejardhur në shqip nga një temë e huaj: «mësues»).
    let loan_row = |e: &&crate::graph::Edge| e.attr("borr").is_some() && !e.attr("flags").unwrap_or("").contains("deriv");
    let loan_is_main = lexemes.iter().any(|e| loan_row(e) && lex_en(e))
        && lexemes.iter().any(|e| !loan_row(e))
        && !lexemes.iter().any(|e| !loan_row(e) && lex_en(e));
    let origin_pair: Option<(String, Vec<Source>)> = match (loan_is_main, rule_origin.into_iter().next(), bases.is_empty()) {
        (true, _, _) => borrow_origin(eng, f, &words, &d).map(|(o, s)| (o, vec![s])),
        (false, Some(o), _) => Some((o, vec![Source::SqRules])),
        (false, None, false) => Some((
            format!("«{d}» rrjedh nga {}", quoted.join(" dhe ")),
            bases_src.into_iter().collect(),
        )),
        (false, None, true) => borrow_origin(eng, f, &words, &d).map(|(o, s)| (o, vec![s])),
    };
    let mut origin: Vec<String> = origin_pair.iter().map(|(o, _)| o.clone()).collect();
    let mut origin_src: Vec<Source> = origin_pair.map(|(_, s)| s).unwrap_or_default();
    // Shpjegimi popullor i pronarit: thuhet pas prejardhjes së dokumentuar dhe gjithmonë si i tillë.
    let folk: Vec<String> = words
        .iter()
        .flat_map(|&w| {
            eng.g
                .out_edges(w)
                .filter(|(_, e)| is_sem(&e.rel, crate::src_owner::FOLK) && e.source == Source::OwnerNote)
                .filter_map(|(_, e)| e.attr("text").map(|t| format!("Sipas një shpjegimi popullor, «{d}» {t}")))
                .collect::<Vec<_>>()
        })
        .take(1)
        .collect();
    match folk.is_empty() {
        true => {}
        false => origin_src.push(Source::OwnerNote),
    }
    origin.extend(folk);
    src.push((Facet::Origin, origin_src));
    // Pa përsëritje: e kundërta që e tha tashmë fjalia e prejardhjes nuk thuhet sërish.
    let opposites: Vec<String> =
        opposites.into_iter().filter(|x| !origin.iter().any(|o| o.contains(&format!("«{x}»")))).collect();
    // Kuptimi krahinor i Veriut nga fjalori i Gazullit (shpjegimi i parë që kaloi filtrin e OCR-së),
    // vetëm kur ka të njëjtën pjesë ligjërate me fjalën: «lumë» folje te Gazulli nuk është «lumi».
    // Kur fjala ka pjesë ligjërate nga fjalorët e tjerë, hyrja pa shënim gramatikor nuk thuhet.
    let main = main_pos_of(eng, &words);
    let regional: Vec<String> = words
        .iter()
        .flat_map(|&w| {
            eng.g
                .out_edges(w)
                .filter(|(_, e)| is_sem(&e.rel, "REGIONAL_GLOSS") && e.source == Source::Gazulli1941)
                .filter(|(_, e)| match (e.attr("pos").and_then(Pos::from_wn), main) {
                    (Some(p), Some(m)) => p == m,
                    (None, Some(_)) => false,
                    (_, None) => true,
                })
                .filter_map(|(_, e)| e.attr("gloss").map(|x| x.to_string()))
                .collect::<Vec<_>>()
        })
        .take(1)
        .collect();
    match regional.is_empty() {
        true => {}
        false => src.push((Facet::Regional, vec![Source::Gazulli1941])),
    }
    Facts {
        regional,
        origin,
        native,
        dialect,
        category,
        synonyms,
        parts: vec![],
        part_of: vec![],
        kinds,
        opposites,
        translation_en: vec![],
        example: vec![],
        src,
    }
}

/// Të kundërtat e thënshme. Kur dy burime pajtohen, thuhen ato. Një burim i vetëm mjafton kur
/// është fjalor me antonime të shënuara (LEXIN), rregull mohimi, WordNet (shenja «!» mes
/// kuptimeve me etiketë shqipe), ose kur lidhja është reciproke në të njëjtin burim (A e jep B
/// dhe B e jep A). Te emrat Wiktionary/ConceptNet shënojnë shpesh çiftin gjinor («qen» /
/// «kudër») si «të kundërt», prandaj për emrat reciprociteti nuk mjafton.
fn opposites_of(eng: &Engine, f: &Focus, words: &[u32], focus_pos: &BTreeSet<Pos>) -> (Vec<String>, BTreeSet<Source>) {
    let noun = focus_pos.contains(&Pos::Noun);
    let ok = |c: &Cand| -> bool {
        let trusted = c.sources.iter().any(|s| matches!(s, Source::SqRules | Source::Lexin | Source::OmwAls));
        let reciprocal = c.sources.iter().any(|s| {
            let p = format!("{}:", s.code());
            c.groups.iter().any(|g| g.starts_with(&p) && g.ends_with(":f")) && c.groups.iter().any(|g| g.starts_with(&p) && g.ends_with(":r"))
        });
        trusted || (reciprocal && !noun)
    };
    chosen(opposite_cands(eng, f, words, focus_pos), &ok)
}

/// Kandidatët për të kundërtën: antonimet e burimeve (në të dy drejtimet: lidhja e kundërt
/// është dëshmi e njëjtë), mohimet sipas rregullave dhe antonimet e ConceptNet.
fn opposite_cands(eng: &Engine, f: &Focus, words: &[u32], focus_pos: &BTreeSet<Pos>) -> Vec<Cand> {
    let mut m: Vec<Cand> = Vec::new();
    sem_cands(eng, words, "ANTONYM", false, true, focus_pos, &mut m);
    sem_cands(eng, words, "ANTONYM", true, true, focus_pos, &mut m);
    // WordNet: antonimia («!») mes kuptimeve të fjalës dhe kuptimeve me etiketë shqipe.
    for &w in words {
        for (sid, e) in eng.g.senses_of(w) {
            match e.source {
                Source::KaikkiEn => {}
                _ => {
                    for t in wn_targets(eng, sid, &["!"]) {
                        for l in eng.g.node(t).labels.iter().filter(|l| l.lang == Lang::Sq) {
                            let (c, node) = cite_term(eng, &clean_label(&l.text));
                            let ok = node.map(|n| !words.contains(&n) && pos_ok(eng, focus_pos, n)).unwrap_or(false);
                            match ok {
                                true => add_cand_g(&mut m, c, node, Source::OmwAls, true, "omw_als:!".into()),
                                false => {}
                            }
                        }
                    }
                }
            }
        }
    }
    let (_, deriv_opp) = derivations(eng, words);
    for x in deriv_opp {
        let node = eng.g.get(&word_key(Lang::Sq, crate::derive::strip_article(&x)));
        add_cand(&mut m, x, node, Source::SqRules, true);
    }
    for (t, node, src) in cn_terms(eng, f, words, "/r/Antonym", false, focus_pos) {
        add_cand(&mut m, t, node, src, true);
    }
    m
}

/// Të kundërtat në nivel fjale, të gatshme për t'u thënë, me burimet e tyre: e njëjta
/// zgjedhje si te përshkrimi i fjalës (pa hequr ato që i thotë fjalia e prejardhjes, sepse
/// këtu pyetja është vetë e kundërta).
pub fn word_opposites(eng: &Engine, f: &Focus) -> (Vec<String>, Vec<Source>) {
    let words = aliases(eng, f);
    let focus_pos = focus_pos_of(eng, &words);
    let lemma = crate::derive::strip_article(&f.resolved.lemma.to_lowercase()).to_string();
    let (opp, src) = opposites_of(eng, f, &words, &focus_pos);
    let opp: Vec<String> = opp.into_iter().filter(|x| !x.split_whitespace().any(|t| t == lemma)).collect();
    (opp, src.into_iter().collect())
}

/// Bashkon faktet e kuptimit me ato të fjalës (pa përsëritje, as me nyjë as pa të; kuptimi
/// ka përparësi). Burimet e fasetit ndjekin atë që u shtua.
fn merge(mut a: Facts, b: Facts) -> Facts {
    let key = |s: &str| crate::derive::strip_article(&s.to_lowercase()).to_string();
    let extend = |x: &mut Vec<String>, y: &[String]| -> bool {
        let mut added = false;
        for i in y {
            match (x.iter().any(|e| key(e) == key(i)), x.len() < MAX_ITEMS) {
                (false, true) => {
                    x.push(i.clone());
                    added = true;
                }
                _ => {}
            }
        }
        added
    };
    for facet in [Facet::Synonyms, Facet::Opposites, Facet::Kinds] {
        let added = match facet {
            Facet::Synonyms => extend(&mut a.synonyms, &b.synonyms),
            Facet::Opposites => extend(&mut a.opposites, &b.opposites),
            _ => extend(&mut a.kinds, &b.kinds),
        };
        match added {
            true => a.src.push((facet, b.src_of(facet))),
            false => {}
        }
    }
    match a.category.is_empty() {
        true => {
            a.category = b.category.clone();
            a.src.push((Facet::Category, b.src_of(Facet::Category)));
        }
        false => {}
    }
    for facet in [Facet::Origin, Facet::Native, Facet::Dialect] {
        a.src.push((facet, b.src_of(facet)));
    }
    a.origin = b.origin;
    a.native = b.native;
    a.dialect = b.dialect;
    a
}

/// Mbledh faktet e kuptimit `sense` për fjalën e fokusit (plus faktet në nivel fjale).
/// Kur kuptimi u zgjodh nga konteksti (R-E2) ose nga përdoruesi (R-E4), mund të jetë një
/// kuptim dytësor: atëherë sinonimet, të kundërtat dhe kategoria e fjalës (që i përkasin
/// kuptimit kryesor) nuk i shtohen; prejardhja, huazimi dhe dialekti mbeten (janë të formës).
fn chosen_specifically(f: &Focus) -> bool {
    f.senses.iter().any(|s| match &s.status {
        CandStatus::Eliminated(r) => r.starts_with("R-E2") || r.starts_with("R-E4"),
        _ => false,
    })
}

pub fn gather(eng: &Engine, f: &Focus, sense: u32, definition: Option<&str>) -> Facts {
    let word = gather_word(eng, f);
    let word = match chosen_specifically(f) {
        true => Facts { synonyms: vec![], opposites: vec![], category: vec![], kinds: vec![], ..word },
        false => word,
    };
    // Sinonimet e fjalës i përkasin shpesh një kuptimi tjetër («pemë» = frutë në gegërisht):
    // i bashkohen përshkrimit të këtij kuptimi vetëm kur ndajnë një përkthim anglisht me të.
    let sense_en: HashSet<String> = eng
        .g
        .node(sense)
        .labels
        .iter()
        .filter(|t| t.lang == Lang::En)
        .map(|t| t.text.replace('_', " ").to_lowercase())
        .collect();
    let fits_sense = |x: &String| {
        let n = eng.g.get(&word_key(Lang::Sq, crate::derive::strip_article(x)));
        let ce = n.map(|n| english_all(eng, n)).unwrap_or_default();
        match (ce.is_empty(), sense_en.is_empty()) {
            (false, false) => ce.iter().any(|e| sense_en.contains(e)),
            (true, false) => false,
            _ => true,
        }
    };
    let synonyms: Vec<String> = word.synonyms.iter().filter(|x| fits_sense(x)).cloned().collect();
    // Kuptimi krahinor i rrallë nuk thuhet pranë kuptimit të zakonshëm të fjalës.
    let word = Facts { synonyms, regional: vec![], ..word };
    merge(gather_sense(eng, f, sense, definition), word)
}

fn gather_sense(eng: &Engine, f: &Focus, sense: u32, definition: Option<&str>) -> Facts {
    let lemma = crate::derive::strip_article(&f.resolved.lemma.to_lowercase()).to_string();
    let mut said: HashSet<String> = HashSet::from([lemma.clone(), format!("i {lemma}"), format!("e {lemma}")]);
    let def_tokens = definition.map(tokens).unwrap_or_default();
    let via = f.senses.iter().find(|s| s.sense == sense).map(|s| s.via).unwrap_or(Source::OmwAls);
    let ssrc = vec![via, Source::Pwn30];

    // Kategoria: hiperonimet e drejtpërdrejta me etiketë shqipe; kontroll koherence.
    let mut category = Vec::new();
    // Kur hiperonimi mban po atë fjalë si etiketë («kalë» ⊂ «kalë») ose vetëm anglisht të
    // papërkthyer («liquid»), kategoria merret një nivel më lart.
    let mut hypers: Vec<u32> = Vec::new();
    // Kategoria («Është …») ka kuptim vetëm për emrat: hiperonimi i një foljeje nuk thuhet si
    // emër kallëzues («Vrapoj është lëviz»).
    let noun_sense = matches!(eng.g.node(sense).kind, NodeKind::Sense { pos: Pos::Noun, .. });
    let hyper_list: Vec<u32> = match noun_sense {
        true => hyper_targets(eng, sense),
        false => vec![],
    };
    for h in hyper_list {
        match (sq_label(eng, h).map(|l| l == lemma), untranslated(eng, h)) {
            (Some(true), _) | (_, true) => hypers.extend(hyper_targets(eng, h)),
            _ => hypers.push(h),
        }
    }
    for h in hypers {
        let parents: Vec<u32> = hyper_targets(eng, h);
        let coherent = match def_tokens.is_empty() {
            true => true,
            false => {
                sq_label(eng, h).map(|l| mentions(&def_tokens, &l)).unwrap_or(false)
                    || parents.iter().filter_map(|p| sq_label(eng, *p)).any(|l| mentions(&def_tokens, &l))
            }
        };
        // Përkufizimi zakonisht nis me gjininë («Uji është lëng …»): kur kategoria është
        // pikërisht ajo fjalë, përsëritja nuk thuhet.
        let genus = sq_label(eng, h).map(|l| is_genus(definition, &l)).unwrap_or(false);
        match (coherent, genus, sq_label(eng, h)) {
            (true, false, Some(l)) => push_unique(&mut category, &mut said, l),
            _ => {}
        }
    }
    // Sinonimet: lemat e tjera shqipe të të njëjtit kuptim (AlbaNet). Nuk thuhen: trajtat e
    // vetë fjalës (edhe pa «ë»: «buke»), vetë përkufizimi, emrat shkencorë latinë, frazat që
    // përmbajnë fjalën, dhe fjalët që asnjë burim tjetër shqip nuk i njeh ose që kanë pjesë
    // tjetër ligjërate (folja «lulëzoj» në sinsetin emëror «lule»).
    let mut synonyms = Vec::new();
    let same_word = |w: &str| crate::morph::strip_candidates(w).iter().any(|c| *c == lemma);
    let fold = |x: &str| x.to_lowercase().replace('ë', "e").replace('ç', "c");
    let def_clean = definition.map(|d| clean_label(d.split(';').next().unwrap_or(""))).unwrap_or_default();
    let sense_pos: BTreeSet<Pos> = match &eng.g.node(sense).kind {
        NodeKind::Sense { pos, .. } => BTreeSet::from([*pos]),
        _ => BTreeSet::new(),
    };
    // Bashkanëtarët e sinsetit të AlbaNet-it thuhen si sinonime vetëm kur i lidh si sinonime
    // edhe një fjalor (Wiktionary, LEXIN, Wikidata): AlbaNet bashkon shpesh fjalë të ndryshme
    // nën një kuptim anglisht më të gjerë («xhaxha» e «dajë» nën «uncle»).
    let words = aliases(eng, f);
    let dict_syn: HashSet<String> = words
        .iter()
        .flat_map(|&w| {
            eng.g
                .out_edges(w)
                .filter(|(_, e)| is_sem(&e.rel, "SYNONYM"))
                .map(|(_, e)| e.to)
                .chain(eng.g.in_edges(w).filter(|(_, e)| is_sem(&e.rel, "SYNONYM")).map(|(_, e)| e.from))
                .collect::<Vec<_>>()
        })
        .map(|n| crate::derive::strip_article(&word_lemma(eng, n)).to_string())
        .chain(
            f.form_rels
                .iter()
                .filter(|r| r.rel == "/r/Synonym" && r.other_lang == Some(Lang::Sq) && !r.dataset.starts_with("/d/wordnet"))
                .map(|r| crate::derive::strip_article(&r.other_term.to_lowercase()).to_string()),
        )
        .collect();
    let fits = |w: &str| match eng.g.get(&word_key(Lang::Sq, w)) {
        Some(id) => {
            let ip = independent_pos(eng, id);
            !ip.is_empty() && pos_compatible(&sense_pos, &ip)
        }
        None => w.contains(' '),
    };
    for t in eng.g.node(sense).labels.iter().filter(|t| t.lang == Lang::Sq) {
        let orig = t.text.replace('_', " ");
        let w = match orig.chars().any(|c| c.is_ascii_digit()) {
            true => orig.clone(),
            false => orig.to_lowercase(),
        };
        let latin = orig.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && orig.split_whitespace().count() >= 2;
        let own = same_word(&w) || fold(&w) == fold(&lemma) || w.split_whitespace().any(|x| x == lemma);
        let confirmed = dict_syn.contains(crate::derive::strip_article(&w));
        match (own, lexical(&w), fits(&w), latin, w == def_clean, confirmed) {
            (false, true, true, false, false, true) => push_unique(&mut synonyms, &mut said, w),
            _ => {}
        }
    }
    // Pjesët, e tëra, llojet (WordNet; vetëm me etiketë shqipe).
    let mut parts = Vec::new();
    for t in wn_targets(eng, sense, &["%p", "%m", "%s"]) {
        match sq_label(eng, t) {
            Some(l) => push_unique(&mut parts, &mut said, l),
            None => {}
        }
    }
    let mut part_of = Vec::new();
    for t in wn_targets(eng, sense, &["#p", "#m", "#s"]) {
        match sq_label(eng, t) {
            Some(l) => push_unique(&mut part_of, &mut said, l),
            None => {}
        }
    }
    let mut kinds = Vec::new();
    for t in wn_targets(eng, sense, &["~", "~i"]) {
        match sq_label(eng, t) {
            Some(l) => push_unique(&mut kinds, &mut said, l),
            None => {}
        }
    }
    // Përkthimi anglisht. Etiketat e sinsetit mund të jenë më të gjera se fjala (p.sh. «qen»
    // te sinseti «canine, canid»), ndërsa fjalori i Wiktionary-t përkthen fjalën. Rendi:
    // (1) etiketat që i dëshmojnë të dy; (2) kur kuptimi nuk u zgjodh si kuptim dytësor,
    // përkthimi i fjalorit; (3) etiketat e sinsetit. Emrat shkencorë latinë («Equus caballus»)
    // nuk thuhen kur ka etiketë tjetër.
    let labels: Vec<String> =
        eng.g.node(sense).labels.iter().filter(|t| t.lang == Lang::En).map(|t| t.text.replace('_', " ")).collect();
    let lower: Vec<String> =
        labels.iter().filter(|l| l.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)).cloned().collect();
    let labels = match lower.is_empty() {
        true => labels,
        false => lower,
    };
    let dict = dict_en(eng, f);
    let both: Vec<String> = labels.iter().filter(|l| dict.contains(l)).cloned().collect();
    let pick = match (both.is_empty(), dict.is_empty(), chosen_specifically(f)) {
        (false, _, _) => both,
        (true, false, false) => dict,
        _ => labels,
    };
    let mut translation_en = Vec::new();
    let mut said_en: HashSet<String> = HashSet::new();
    for t in pick {
        push_unique(&mut translation_en, &mut said_en, t);
    }
    translation_en.truncate(2);
    // Shembulli: shembull shqip i kuptimit, ndryshe një fjali UD me fjalën.
    let mut example = Vec::new();
    let mut said_ex: HashSet<String> = HashSet::new();
    for t in eng.g.node(sense).examples.iter().filter(|t| t.lang == Lang::Sq) {
        push_unique(&mut example, &mut said_ex, t.text.clone());
    }
    match (example.is_empty(), f.usages.first()) {
        (true, Some(u)) => example.push(u.text.clone()),
        _ => {}
    }
    // Shembulli thuhet vetëm kur është i shkurtër: një fjali e gjatë e një korpusi nuk sqaron.
    let example: Vec<String> = example.into_iter().filter(|e| e.split_whitespace().count() <= 20).take(1).collect();
    let src = [Facet::Category, Facet::Synonyms, Facet::Parts, Facet::PartOf, Facet::Kinds, Facet::TranslationEn, Facet::Example]
        .into_iter()
        .map(|fc| (fc, ssrc.clone()))
        .collect();
    Facts {
        regional: vec![],
        origin: vec![],
        native: vec![],
        dialect: vec![],
        category,
        synonyms,
        parts,
        part_of,
        kinds,
        opposites: vec![],
        translation_en,
        example,
        src,
    }
}

fn facet_clause(word: &TermRef, facet: Facet, items: Vec<String>, sources: Vec<Source>) -> Clause {
    let polarity = match items.is_empty() {
        true => Polarity::Neg,
        false => Polarity::Pos,
    };
    Clause {
        claim: Claim::Facet { word: word.clone(), facet, items },
        polarity,
        attribution: Some(Attribution { sources, note: String::new() }),
    }
}

/// Plani i përshkrimit: përkufizimi (nga thirrësi) + deri në `budget` fasete,
/// sipas rëndësisë: kategoria, sinonimet, e kundërta, pjesët/llojet, përkthimi, shembulli.
/// Burimet e çdo faseti janë ato që e dëshmuan; `sources` vlen vetëm kur mungojnë.
pub fn describe(word: &TermRef, facts: Facts, sources: &[Source], budget: usize) -> Vec<Clause> {
    let srcs: Vec<(Facet, Vec<Source>)> = [
        Facet::Regional,
        Facet::Dialect,
        Facet::Category,
        Facet::Origin,
        Facet::Synonyms,
        Facet::Native,
        Facet::Opposites,
        Facet::TranslationEn,
        Facet::Example,
    ]
    .into_iter()
    .map(|fc| (fc, facts.src_of(fc)))
    .collect();
    // Pjesët dhe llojet jepen vetëm kur pyeten drejtpërdrejt (te përshkrimi i
    // përgjithshëm etiketat e tyre janë shpesh të zhurmshme).
    let order: Vec<(Facet, Vec<String>)> = vec![
        (Facet::Regional, facts.regional),
        (Facet::Dialect, facts.dialect),
        (Facet::Category, facts.category),
        (Facet::Origin, facts.origin),
        (Facet::Synonyms, facts.synonyms),
        (Facet::Native, facts.native),
        (Facet::Opposites, facts.opposites),
        (Facet::TranslationEn, facts.translation_en),
        (Facet::Example, facts.example),
    ];
    order
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .take(budget)
        .map(|(fc, v)| {
            let s = srcs.iter().find(|(x, _)| *x == fc).map(|(_, s)| s.clone()).unwrap_or_default();
            let s = match s.is_empty() {
                true => sources.to_vec(),
                false => s,
            };
            facet_clause(word, fc, v, s)
        })
        .collect()
}

/// Një faset i vetëm, për pyetjet e drejtpërdrejta ("çfarë pjesësh ka …").
pub fn single(word: &TermRef, facts: Facts, facet: Facet, sources: &[Source]) -> Clause {
    let s = facts.src_of(facet);
    let s = match s.is_empty() {
        true => sources.to_vec(),
        false => s,
    };
    let items = match facet {
        Facet::Category => facts.category,
        Facet::Synonyms => facts.synonyms,
        Facet::Parts => facts.parts,
        Facet::PartOf => facts.part_of,
        Facet::Kinds => facts.kinds,
        Facet::Opposites => facts.opposites,
        Facet::TranslationEn => facts.translation_en,
        Facet::Example => facts.example,
        Facet::Origin => facts.origin,
        Facet::Native => facts.native,
        Facet::Dialect => facts.dialect,
        Facet::Grammar => vec![],
        Facet::Regional => facts.regional,
    };
    facet_clause(word, facet, items, s)
}

/// Zgjedh kuptimin e parë (sipas rendit të fiksuar) që ka të dhëna për fasetin.
pub fn sense_with_facet(eng: &Engine, f: &Focus, facet: Facet) -> Option<u32> {
    f.senses
        .iter()
        .filter(|s| matches!(s.status, CandStatus::Open | CandStatus::Selected(_)))
        .map(|s| s.sense)
        .find(|&s| !single(&TermRef::word(""), gather(eng, f, s, None), facet, &[]).claim_items_empty())
}

impl Clause {
    pub fn claim_items_empty(&self) -> bool {
        match &self.claim {
            Claim::Facet { items, .. } => items.is_empty(),
            _ => false,
        }
    }
}

/// Paraardhësit (hiperonimet) me distancë, deri në 12 hapa.
fn ancestors(eng: &Engine, start: u32) -> Vec<(u32, usize)> {
    let mut out = vec![(start, 0usize)];
    let mut seen = HashSet::from([start]);
    let mut i = 0;
    while i < out.len() {
        let (n, d) = out[i];
        match d < 12 {
            true => {
                for h in hyper_targets(eng, n) {
                    match seen.insert(h) {
                        true => out.push((h, d + 1)),
                        false => {}
                    }
                }
            }
            false => {}
        }
        i += 1;
    }
    out
}

/// Krahasimi i dy fjalëve: paraardhësi i përbashkët më i afërt me etiketë shqipe (jo me nyjë
/// mbiemri), dhe për dallimin kategoria e secilës që qëndron POSHTË paraardhësit të përbashkët
/// (një kategori mbi të do të ishte e përbashkët, jo dallim).
pub fn compare(eng: &Engine, a: &Focus, b: &Focus) -> Clause {
    // Vetëm kuptimet emërore (krahasohen sende, jo veprime) dhe paraardhës jo shumë afër rrënjës
    // («njësi», «objekt», «entitet» janë të përbashkët për gjithçka).
    let senses = |f: &Focus| -> Vec<u32> {
        f.senses
            .iter()
            .filter(|s| s.pos == Pos::Noun && !matches!(s.status, CandStatus::Eliminated(_)))
            .map(|s| s.sense)
            .collect()
    };
    let deep = |n: u32| ancestors(eng, n).iter().map(|(_, d)| *d).max().unwrap_or(0) >= 4;
    let mut best: Option<(usize, u32, u32, u32, usize, usize)> = None;
    for sa in senses(a) {
        let anc_a = ancestors(eng, sa);
        for sb in senses(b) {
            for (nb, db) in ancestors(eng, sb) {
                match anc_a.iter().find(|(na, _)| *na == nb) {
                    Some((_, da)) => {
                        let score = da + db;
                        let labelled =
                            sq_label(eng, nb).map(|l| article_of(&l).is_none()).unwrap_or(false) && nb != sa && nb != sb && deep(nb);
                        match (labelled, best.map(|x| score < x.0).unwrap_or(true)) {
                            (true, true) => best = Some((score, nb, sa, sb, *da, db)),
                            _ => {}
                        }
                    }
                    None => {}
                }
            }
        }
    }
    let below = |s: u32, limit: usize| -> Option<String> {
        ancestors(eng, s).into_iter().skip(1).filter(|(_, d)| *d < limit).find_map(|(n, _)| sq_label(eng, n))
    };
    let (common, a_cat, b_cat) = match best {
        Some((_, c, sa, sb, da, db)) => (sq_label(eng, c), below(sa, da), below(sb, db)),
        None => (None, None, None),
    };
    Clause {
        claim: Claim::Compare {
            a: term_of(eng, a.resolved.word, &a.resolved.lemma),
            b: term_of(eng, b.resolved.word, &b.resolved.lemma),
            common: common.clone(),
            a_cat,
            b_cat,
        },
        polarity: match common {
            Some(_) => Polarity::Pos,
            None => Polarity::Neg,
        },
        attribution: Some(Attribution { sources: vec![Source::OmwAls, Source::Pwn30], note: String::new() }),
    }
}
