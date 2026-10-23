//! Semantika e fjalisë: si LIDHEN fjalët dhe si PËRPUTHEN, që nga dy ose tri fjalë të dalë
//! një fjali e vetme, deterministe, me dëshmitë e saj.
//!
//! Lidhjet vijnë nga grafi: «lloj i» (zinxhiri i hiperonimeve), sinonimi, e kundërta, pjesa,
//! dhe për foljet klasa e kundrinorit (tabela `OBJECT_OF`) e gjallësia e kryefjalës.
//! Përputhjet vijnë nga gramatika: kryefjala në trajtën e shquar, nyja dhe gjinia e mbiemrit,
//! veta e tretë e foljes, kalimtaria, kundrinori në trajtën e pashquar.
//!
//! Kornizat, me radhë fikse: lloj i → sinonim → pjesë → e kundërt → s'është (pa asnjë lidhje
//! kategorie) për dy emra; cilësi për emër + mbiemër; veprim për emër + folje [+ emër].
//! Kur asnjë kornizë nuk vlen, kthehet arsyeja; nuk shpiket asgjë.
//!
//! Materiali (pozitive / negative / dallimi / burimi / kushti rrëzues) ndërtohet për t'u
//! provuar në sistemin e jashtëm të verifikimit (produkti ESS-MAI), që kërkon pikërisht
//! dallimin pozitiv–negativ për çdo fjali; del si një rresht i vetëm ≤ 512 bajte. Shabllonet
//! shmangin fjalët me të cilat ai sistem cakton territorin («jashtë», «mes», «shtëpi» …), që
//! materiali të mbetet në territorin e fjalisë.

use crate::discourse;
use crate::expression::{Facet, Polarity};
use crate::graph::*;
use crate::morph::strip_candidates;
use crate::query::{Engine, Focus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub rel: &'static str,
    pub label: String,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    pub rule: &'static str,
    pub surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Material {
    pub positive: String,
    pub negative: String,
    pub distinction: String,
    pub source: String,
    pub refutation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composed {
    pub text: String,
    pub frame: &'static str,
    pub polarity: Polarity,
    pub links: Vec<Link>,
    pub agreements: Vec<Agreement>,
    /// Fjalia e kundërt e vërtetuar nga mungesa (për korniza «lloj i»).
    pub counter: Option<String>,
    pub material: Material,
}

/// Kufiri i hyrjes së sistemit të jashtëm: një rresht, ≤ 512 bajte.
pub const LINE_LIMIT: usize = 512;

/// Kategoritë e larta për fjalinë e kundërt («X nuk është Y»): merret e para që s'ka asnjë
/// lidhje kategorie me kryefjalën as me kallëzuesin.
const TOP: &[&str] = &["bimë", "kafshë", "send", "vend", "ngjarje", "ndjenjë", "lëng", "ushqim", "mjet", "numër", "njeri"];

/// Klasat e kundrinorit për foljet e zakonshme: kundrinori duhet të jetë njëra nga këto ose
/// lloj i njërës prej tyre.
const OBJECT_OF: &[(&str, &[&str])] = &[
    ("ha", &["ushqim", "mish", "bukë", "frut", "perime", "gjellë", "ëmbëlsirë", "bar"]),
    ("pi", &["lëng", "pije", "ujë", "qumësht", "verë", "kafe", "çaj", "birrë"]),
    ("lexoj", &["libër", "tekst", "letër", "gazetë", "shkrim", "fjalë", "revistë"]),
    ("shkruaj", &["libër", "letër", "tekst", "fjalë", "emër", "shkrim"]),
    ("dëgjoj", &["zë", "muzikë", "këngë", "fjalë", "zhurmë", "radio"]),
    ("shoh", &["send", "njeri", "kafshë", "film", "pamje", "fotografi"]),
    ("blej", &["send", "mall", "ushqim", "libër", "makinë", "shtëpi"]),
    ("ndërtoj", &["ndërtesë", "shtëpi", "urë", "mur", "rrugë"]),
    ("këndoj", &["këngë", "melodi"]),
    ("luaj", &["lojë", "muzikë", "instrument", "futboll"]),
    ("hap", &["derë", "dritare", "libër", "gojë", "sy"]),
    ("mbyll", &["derë", "dritare", "libër", "gojë", "sy"]),
];

/// Kategoritë që e bëjnë kryefjalën të gjallë (vepruese).
const ANIMATE: &[&str] = &["njeri", "kafshë", "gjallesë", "qenie", "organizëm", "person", "shtazë"];

struct Word<'a> {
    typed: String,
    focus: Focus,
    pos: std::collections::BTreeSet<Pos>,
    eng: &'a Engine<'a>,
}

impl Word<'_> {
    fn lemma(&self) -> &str {
        &self.focus.resolved.lemma
    }
    fn node(&self) -> Option<u32> {
        self.focus.resolved.word
    }
    fn is(&self, p: Pos) -> bool {
        self.pos.contains(&p)
    }
    fn only_noun(&self) -> bool {
        self.is(Pos::Noun) && !self.is(Pos::Verb)
    }
    fn lex_attr(&self, key: &str) -> Option<String> {
        let w = self.node()?;
        self.eng
            .g
            .out_edges(w)
            .filter(|(_, e)| e.rel == Rel::Lex && e.source == Source::Uniparser)
            .find_map(|(_, e)| e.attr(key).map(|v| v.to_string()))
    }
}

fn word<'a>(eng: &'a Engine<'a>, typed: &str) -> Word<'a> {
    let focus = eng.focus(typed);
    let mut pos = focus.resolved.word.map(|w| discourse::independent_pos(eng, w)).unwrap_or_default();
    // Fjala me nyjë («i madh», «e kuqe») është mbiemër, edhe kur gramatika s'e ka si hyrje.
    match focus.resolved.lemma.split_once(' ').map(|(a, _)| matches!(a, "i" | "e" | "të")).unwrap_or(false) {
        true => {
            pos.insert(Pos::Adj);
        }
        false => {}
    }
    Word { typed: typed.to_string(), focus, pos, eng }
}

/// Rruga «lloj i» nga `a` te `b`: etiketat shqipe të zinxhirit dhe burimet e skajeve.
fn isa(eng: &Engine, a: &Focus, b: &Focus) -> Option<Link> {
    let (_, path) = eng.isa_path(a, b)?;
    let mut labels: Vec<String> = Vec::new();
    for step in &path {
        match discourse::sq_label(eng, step.node) {
            Some(l) => match labels.last() == Some(&l) {
                true => {}
                false => labels.push(l),
            },
            None => {}
        }
    }
    let mut sources: Vec<Source> = path.iter().filter_map(|s| s.edge).map(|e| eng.g.edges[e as usize].source).collect();
    // Etiketat shqipe të zinxhirit vijnë nga AlbaNet: dëshmitar i zinxhirit bashkë me WordNet-in.
    match labels.is_empty() {
        true => {}
        false => sources.push(Source::OmwAls),
    }
    sources.sort_by_key(|s| s.code());
    sources.dedup();
    Some(Link { rel: "lloj i", label: labels.join(" → "), sources })
}

fn under(eng: &Engine, f: &Focus, category: &str) -> bool {
    eng.isa_path(f, &eng.focus(category)).is_some()
}

/// Kontrolli i dytë «lloj i», përmes urës angleze (Wiktionary → WordNet): `b` del ndër
/// paraardhësit e `a` si fjalë shqipe ose si përkthim i saj.
fn bridge_isa(a: &Word, b: &Word) -> bool {
    let eng = a.eng;
    let bridge = discourse::bridge_translation(eng, a.node());
    let (anc_sq, anc_en) = discourse::bridge_ancestors(eng, &bridge);
    let b_en = discourse::bridge_translation(eng, b.node());
    anc_sq.contains(b.lemma()) || b_en.iter().any(|x| anc_en.contains(x))
}

/// Të dy fjalët ndajnë një kuptim (i njëjti synset): nuk janë kategori të ndara.
fn shares_sense(a: &Word, b: &Word) -> bool {
    a.focus.senses.iter().any(|x| b.focus.senses.iter().any(|y| x.sense == y.sense))
}

/// Fjala hyn në ndonjë kategori të lartë: është emër i klasifikuar, jo i panjohur.
fn classified(w: &Word) -> bool {
    TOP.iter().any(|c| under(w.eng, &w.focus, c))
}

/// Zinxhiri i shkurtuar për materialin: fillimi, dy hallkat e fundit.
fn short_chain(label: &str) -> String {
    let parts: Vec<&str> = label.split(" → ").collect();
    match parts.len() > 4 {
        true => format!("{} → {} → … → {} → {}", parts[0], parts[1], parts[parts.len() - 2], parts[parts.len() - 1]),
        false => label.to_string(),
    }
}

fn animate(w: &Word) -> bool {
    w.lex_attr("anim").as_deref() == Some("anim") || ANIMATE.iter().any(|c| under(w.eng, &w.focus, c))
}

fn source_labels(sources: &[Source]) -> String {
    match sources.is_empty() {
        true => "asnjë".to_string(),
        false => sources.iter().map(|s| s.label().split(['(', ',']).next().unwrap_or("").trim().to_string()).collect::<Vec<_>>().join(", "),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Kryefjala në trajtën e shquar emërore (gramatika; ndryshe citim).
fn subject(w: &Word) -> Agreement {
    let r = w.eng.morph.definite_nom_sg(w.lemma());
    Agreement { rule: "kryefjala: emërore e shquar", surface: capitalize(&r.text) }
}

/// Gjinia e emrit sipas gramatikës (UD, pastaj uniparser).
fn gender(w: &Word) -> Option<&'static str> {
    match w.eng.morph.gender(w.lemma()) {
        Some(crate::morph::Gender::Masc) => Some("m"),
        Some(crate::morph::Gender::Fem) => Some("f"),
        _ => match w.lex_attr("gender").as_deref() {
            Some("m") => Some("m"),
            Some("f") => Some("f"),
            _ => None,
        },
    }
}

/// Trajta femërore e mbiemrit: e atestuar kur ekziston; ndryshe rregulli (i madh → e madhe,
/// i bukur → e bukur, i ri → e re, i shëndetshëm → e shëndetshme).
pub fn feminine(morph: &crate::morph::Morph, adj: &str) -> (String, &'static str) {
    let plus_e = format!("{adj}e");
    let attested = [adj.to_string(), plus_e.clone()].into_iter().find(|f| morph.knows_form(f) && f.ends_with('e') != adj.ends_with('e') || morph.knows_form(f) && *f == plus_e);
    let irregular = match adj {
        "ri" => Some("re"),
        "zi" => Some("zezë"),
        "keq" => Some("keqe"),
        "madh" => Some("madhe"),
        _ => None,
    };
    let unchanged = adj.ends_with(['ë', 'e']) || adj.ends_with("ur") || adj.ends_with("ël") || adj.ends_with("ër") || adj.ends_with("ën");
    match (irregular, attested, adj.strip_suffix("shëm"), unchanged) {
        (Some(f), _, _, _) => (f.to_string(), "mbiemër i parregullt (tabela)"),
        (None, _, Some(stem), _) => (format!("{stem}shme"), "rregulli R-A3: «-shëm» → «-shme»"),
        (None, _, None, true) => (adj.to_string(), "rregulli R-A2: trajta e pandryshuar"),
        (None, Some(f), None, false) => (f, "trajtë e atestuar (UD)"),
        (None, None, None, false) => (plus_e, "rregulli R-A1: + «e»"),
    }
}

fn adjective(noun: &Word, adj: &Word) -> Agreement {
    let bare = crate::derive::strip_article(adj.lemma()).to_string();
    match gender(noun) {
        Some("f") => {
            let (f, rule) = feminine(noun.eng.morph, &bare);
            Agreement { rule, surface: format!("e {f}") }
        }
        _ => Agreement { rule: "mbiemri: nyja «i» për gjininë mashkullore", surface: format!("i {bare}") },
    }
}

fn verb3(w: &Word) -> Result<Agreement, String> {
    crate::verb::present_3sg(w.eng.g, w.eng.morph, w.lemma())
        .map(|v| Agreement { rule: "folja: veta e tretë njëjës", surface: v.text })
        .ok_or_else(|| format!("nuk di ta lakoj foljen «{}»", w.lemma()))
}

/// Një rresht ≤ 512 bajte, i prerë në kufi shkronje.
pub fn fit_line(s: &str) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = one;
    loop {
        match out.len() > LINE_LIMIT {
            true => {
                out.pop();
            }
            false => return out,
        }
    }
}

impl Material {
    pub fn line(&self) -> String {
        fit_line(&format!(
            "Pozitive: {}. Negative: {}. Dallimi: {}. Burimi: {}. Kushti: {}.",
            self.positive, self.negative, self.distinction, self.source, self.refutation
        ))
    }
}

/// Fjalia e kundërt për «lloj i»: kategoria e parë e lartë pa lidhje me kryefjalën as me kallëzuesin.
fn counter(eng: &Engine, subj: &Word, obj: &Word) -> Option<String> {
    TOP.iter()
        .filter(|c| **c != obj.lemma() && **c != subj.lemma())
        .map(|c| (c, eng.focus(c)))
        .filter(|(_, cf)| !cf.senses.is_empty())
        .find(|(_, cf)| {
            eng.isa_path(&subj.focus, cf).is_none()
                && eng.isa_path(cf, &subj.focus).is_none()
                && eng.isa_path(&obj.focus, cf).is_none()
                && eng.isa_path(cf, &obj.focus).is_none()
        })
        .map(|(c, _)| c.to_string())
}

fn frame_isa(subj: &Word, obj: &Word, link: Link) -> Composed {
    let s = subject(subj);
    let o = Agreement { rule: "kallëzuesi emëror: emërore e pashquar", surface: obj.lemma().to_string() };
    let text = format!("{} është {}.", s.surface, o.surface);
    let against = counter(subj.eng, subj, obj);
    let against_text = against.as_ref().map(|c| format!("{} nuk është {c}.", s.surface));
    let material = Material {
        positive: format!("{} ({})", short_chain(&link.label), source_labels(&link.sources)),
        negative: against
            .as_ref()
            .map(|c| format!("{} nuk është {c}; asnjë lidhje «lloj i» ndërmjet «{}» dhe «{c}»", s.surface, subj.lemma()))
            .unwrap_or_else(|| "asnjë kategori e kundërt e gjetur".into()),
        distinction: format!("Xi = «{}» (lloji), Yi = «{}» (kategoria që e përfshin)", subj.lemma(), obj.lemma()),
        source: source_labels(&link.sources),
        refutation: format!("rrëzohet nëse një burim e jep «{}» si lloj të «{}»", obj.lemma(), subj.lemma()),
    };
    Composed { text, frame: "lloj i", polarity: Polarity::Pos, links: vec![link], agreements: vec![s, o], counter: against_text, material }
}

fn frame_negative(subj: &Word, obj: &Word) -> Composed {
    let s = subject(subj);
    let text = format!("{} nuk është {}.", s.surface, obj.lemma());
    let material = Material {
        positive: format!("«{}» dhe «{}» janë fjalë të dëshmuara, secila me kuptimet e veta", subj.lemma(), obj.lemma()),
        negative: format!("asnjë lidhje «lloj i» në asnjë drejtim ndërmjet «{}» dhe «{}»", subj.lemma(), obj.lemma()),
        distinction: format!("Xi = «{}», Yi = «{}»: kategori të ndara", subj.lemma(), obj.lemma()),
        source: "AlbaNet/WordNet, Wiktionary, leksiku i modelit (mungesë e lidhjes)".into(),
        refutation: format!("rrëzohet nëse gjendet një zinxhir kategorish nga «{}» te «{}»", subj.lemma(), obj.lemma()),
    };
    Composed { text, frame: "s'është", polarity: Polarity::Neg, links: vec![], agreements: vec![s], counter: None, material }
}

fn frame_synonym(subj: &Word, other: &Word, sources: Vec<Source>) -> Composed {
    let s = subject(subj);
    let text = format!("{} quhet edhe «{}».", s.surface, other.lemma());
    let material = Material {
        positive: format!("«{}» = «{}» ({})", subj.lemma(), other.lemma(), source_labels(&sources)),
        negative: format!("«{}» nuk është e kundërta dhe as lloj i «{}»", other.lemma(), subj.lemma()),
        distinction: "Xi dhe Yi janë e njëjta fjalë me dy trajta".into(),
        source: source_labels(&sources),
        refutation: format!("rrëzohet nëse një burim i ndan «{}» dhe «{}» në kuptime të ndryshme", subj.lemma(), other.lemma()),
    };
    Composed {
        text,
        frame: "sinonim",
        polarity: Polarity::Pos,
        links: vec![Link { rel: "sinonim", label: format!("{} = {}", subj.lemma(), other.lemma()), sources }],
        agreements: vec![s],
        counter: None,
        material,
    }
}

fn frame_opposite(subj: &Word, other: &Word, sources: Vec<Source>) -> Composed {
    let text = format!("E kundërta e «{}» është «{}».", subj.lemma(), other.lemma());
    let material = Material {
        positive: format!("«{}» ↔ «{}» të kundërta ({})", subj.lemma(), other.lemma(), source_labels(&sources)),
        negative: format!("«{}» nuk është sinonim i «{}»", other.lemma(), subj.lemma()),
        distinction: "Xi dhe Yi qëndrojnë në skaje të kundërta të së njëjtës shkallë".into(),
        source: source_labels(&sources),
        refutation: format!("rrëzohet nëse një burim i jep «{}» dhe «{}» si sinonime", subj.lemma(), other.lemma()),
    };
    Composed {
        text,
        frame: "e kundërt",
        polarity: Polarity::Pos,
        links: vec![Link { rel: "e kundërt", label: format!("{} ↔ {}", subj.lemma(), other.lemma()), sources }],
        agreements: vec![],
        counter: None,
        material,
    }
}

fn frame_part(subj: &Word, part: &Word, sources: Vec<Source>) -> Composed {
    let s = subject(subj);
    let text = format!("{} ka {}.", s.surface, part.lemma());
    let material = Material {
        positive: format!("«{}» është pjesë e «{}» ({})", part.lemma(), subj.lemma(), source_labels(&sources)),
        negative: format!("«{}» nuk është lloj i «{}»", part.lemma(), subj.lemma()),
        distinction: "Xi është e tëra, Yi pjesa e saj".into(),
        source: source_labels(&sources),
        refutation: format!("rrëzohet nëse «{}» del e tërë dhe «{}» pjesë e saj", part.lemma(), subj.lemma()),
    };
    Composed {
        text,
        frame: "pjesë",
        polarity: Polarity::Pos,
        links: vec![Link { rel: "pjesë", label: format!("{} ⊃ {}", subj.lemma(), part.lemma()), sources }],
        agreements: vec![s],
        counter: None,
        material,
    }
}

fn frame_quality(noun: &Word, adj: &Word) -> Composed {
    let s = subject(noun);
    let a = adjective(noun, adj);
    let text = format!("{} është {}.", s.surface, a.surface);
    let material = Material {
        positive: format!("fjali e përputhur gramatikisht: {} / {}", s.rule, a.rule),
        negative: "burimet nuk e lidhin cilësinë me emrin: pa dëshmi të vërtetësisë".into(),
        distinction: format!("Xi = «{}» (emri), Yi = «{}» (cilësia)", noun.lemma(), adj.lemma()),
        source: "gramatika (UD, uniparser); pa burim për vërtetësinë".into(),
        refutation: "rrëzohet nëse gjinia ose nyja nuk përputhen".into(),
    };
    Composed { text, frame: "cilësi", polarity: Polarity::Pos, links: vec![], agreements: vec![s, a], counter: None, material }
}

fn object_fits(verb: &Word, obj: &Word) -> Result<Option<Link>, String> {
    let table = OBJECT_OF.iter().find(|(v, _)| *v == verb.lemma()).map(|(_, c)| *c);
    match table {
        Some(classes) => {
            let hit = classes.iter().find(|c| **c == obj.lemma() || under(obj.eng, &obj.focus, c));
            match hit {
                Some(c) => Ok(Some(Link {
                    rel: "kundrinor",
                    label: format!("{} merr «{}» ({} ⊂ {c})", verb.lemma(), obj.lemma(), obj.lemma()),
                    sources: vec![Source::SqRules],
                })),
                None => Err(format!("«{}» nuk përputhet si kundrinor i «{}»", obj.lemma(), verb.lemma())),
            }
        }
        None => match verb.lex_attr("trans").map(|t| t.contains("vt")).unwrap_or(true) {
            true => Ok(None),
            false => Err(format!("folja «{}» nuk merr kundrinor", verb.lemma())),
        },
    }
}

fn frame_action(subj: &Word, verb: &Word, obj: Option<&Word>) -> Result<Composed, String> {
    let s = subject(subj);
    let v = verb3(verb)?;
    let lively = animate(subj);
    let object_link = obj.map(|o| object_fits(verb, o)).transpose()?.flatten();
    let intransitive_ok = obj.is_some() || verb.lex_attr("trans").map(|t| t.contains("vi") || t == "-").unwrap_or(true) || verb.lex_attr("med").is_some();
    match (lively, intransitive_ok) {
        (false, _) => Err(format!("«{}» nuk është kryefjalë e gjallë për «{}»", subj.lemma(), verb.lemma())),
        (_, false) => Err(format!("folja «{}» kërkon kundrinor", verb.lemma())),
        (true, true) => {
            let o = obj.map(|o| Agreement { rule: "kundrinori: kallëzore e pashquar", surface: o.lemma().to_string() });
            let text = match &o {
                Some(o) => format!("{} {} {}.", s.surface, v.surface, o.surface),
                None => format!("{} {}.", s.surface, v.surface),
            };
            let mut links = vec![Link { rel: "kryefjalë e gjallë", label: format!("{} është qenie e gjallë", subj.lemma()), sources: vec![Source::Uniparser] }];
            links.extend(object_link);
            let checked = links.len() > 1;
            let material = Material {
                positive: format!("{} ({})", links.iter().map(|l| l.label.clone()).collect::<Vec<_>>().join("; "), source_labels(&[Source::Uniparser, Source::SqRules])),
                negative: match checked {
                    true => "kundrinori përtej klasës së foljes do ta prishte fjalinë".into(),
                    false => "përputhja e kundrinorit nuk u kontrollua (folje jashtë tabelës)".into(),
                },
                distinction: format!("Xi = «{}» (vepruesi), Yi = «{}» (veprimi)", subj.lemma(), verb.lemma()),
                source: "gramatika (uniparser), rregullat e fjalisë".into(),
                refutation: "rrëzohet nëse kryefjala del e pajetë ose kundrinori përtej klasës".into(),
            };
            let mut agreements = vec![s, v];
            agreements.extend(o);
            Ok(Composed { text, frame: "veprim", polarity: Polarity::Pos, links, agreements, counter: None, material })
        }
    }
}

/// Sinonimet dhe të kundërtat e dëshmuara të fjalës (si i thotë Nura).
fn spoken(eng: &Engine, w: &Word, facet: Facet) -> (Vec<String>, Vec<Source>) {
    let facts = discourse::gather_word(eng, &w.focus);
    let items = match facet {
        Facet::Synonyms => facts.synonyms.clone(),
        Facet::Opposites => discourse::word_opposites(eng, &w.focus).0,
        _ => vec![],
    };
    let src = facts.src.iter().find(|(f, _)| *f == facet).map(|(_, s)| s.clone()).unwrap_or_default();
    (items, src)
}

fn parts_of(eng: &Engine, w: &Word) -> (Vec<String>, Vec<Source>) {
    match discourse::sense_with_facet(eng, &w.focus, Facet::Parts) {
        Some(s) => {
            let facts = discourse::gather(eng, &w.focus, s, None);
            let src = facts.src.iter().find(|(f, _)| *f == Facet::Parts).map(|(_, s)| s.clone()).unwrap_or_default();
            (facts.parts, src)
        }
        None => (vec![], vec![]),
    }
}

fn same(items: &[String], lemma: &str) -> bool {
    let bare = crate::derive::strip_article(lemma).to_lowercase();
    items.iter().any(|i| {
        let b = crate::derive::strip_article(&i.to_lowercase()).to_string();
        b == bare || strip_candidates(&b).contains(&bare)
    })
}

fn bridge_link(a: &Word, b: &Word) -> Link {
    Link { rel: "lloj i", label: format!("{} → … → {} (përmes anglishtes)", a.lemma(), b.lemma()), sources: vec![Source::KaikkiEn, Source::Pwn30] }
}

fn two_nouns(a: &Word, b: &Word) -> Result<Composed, String> {
    let eng = a.eng;
    match (isa(eng, &a.focus, &b.focus), isa(eng, &b.focus, &a.focus)) {
        (Some(l), _) => return Ok(frame_isa(a, b, l)),
        (None, Some(l)) => return Ok(frame_isa(b, a, l)),
        (None, None) => {}
    }
    let (syn, syn_src) = spoken(eng, a, Facet::Synonyms);
    let (parts, parts_src) = parts_of(eng, a);
    let (opp, opp_src) = spoken(eng, a, Facet::Opposites);
    match (same(&syn, b.lemma()), same(&parts, b.lemma()), same(&opp, b.lemma())) {
        (true, _, _) => return Ok(frame_synonym(a, b, syn_src)),
        (_, true, _) => return Ok(frame_part(a, b, parts_src)),
        (_, _, true) => return Ok(frame_opposite(a, b, opp_src)),
        _ => {}
    }
    // Kontrolli i dytë «lloj i» përmes anglishtes, para se të thuhet ndonjë mohim.
    match (bridge_isa(a, b), bridge_isa(b, a)) {
        (true, _) => return Ok(frame_isa(a, b, bridge_link(a, b))),
        (_, true) => return Ok(frame_isa(b, a, bridge_link(b, a))),
        _ => {}
    }
    // Mohimi vetëm kur të dyja janë emra të klasifikuar, pa asnjë kuptim të përbashkët.
    match (b.focus.senses.is_empty(), shares_sense(a, b), classified(a) && classified(b)) {
        (true, _, _) => Err(format!("«{}» s'ka kuptim të dëshmuar: nuk e lidh dot me «{}»", b.typed, a.typed)),
        (_, true, _) => Err(format!("«{}» dhe «{}» ndajnë një kuptim: nuk i ndaj dot", a.typed, b.typed)),
        (_, _, false) => Err(format!("nuk gjej asnjë lidhje të dëshmuar mes «{}» dhe «{}»", a.typed, b.typed)),
        (false, false, true) => Ok(frame_negative(a, b)),
    }
}

/// Fjalia nga dy ose tri fjalë.
pub fn compose(eng: &Engine, words: &[String]) -> Result<Composed, String> {
    let ws: Vec<Word> = words.iter().map(|w| word(eng, w)).collect();
    match ws.iter().find(|w| w.node().is_none()) {
        Some(w) => return Err(format!("nuk e njoh fjalën «{}»", w.typed)),
        None => {}
    }
    match ws.as_slice() {
        [a, b] => match (a.is(Pos::Noun), b.only_noun(), a.is(Pos::Adj), b.is(Pos::Adj), b.is(Pos::Verb), a.is(Pos::Verb)) {
            (true, true, _, _, _, _) => two_nouns(a, b),
            (true, _, _, true, _, _) => Ok(frame_quality(a, b)),
            (_, true, true, _, _, _) => Ok(frame_quality(b, a)),
            (true, _, _, _, true, _) => frame_action(a, b, None),
            (_, true, _, _, _, true) => Err(format!("kryefjala duhet të vijë para foljes: «fjali {} {}»", b.typed, a.typed)),
            _ => Err(format!("«{}» dhe «{}» nuk formojnë fjali: duhen dy emra, emër + mbiemër ose emër + folje", a.typed, b.typed)),
        },
        [a, v, o] => match (a.is(Pos::Noun), v.is(Pos::Verb), o.is(Pos::Noun)) {
            (true, true, true) => frame_action(a, v, Some(o)),
            _ => Err("tri fjalë formojnë fjali vetëm si emër + folje + emër".into()),
        },
        _ => Err("fjalia ndërtohet nga dy ose tri fjalë".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_fit_the_external_limit_on_char_boundaries() {
        let long = "ë".repeat(600);
        let out = fit_line(&long);
        assert!(out.len() <= LINE_LIMIT && out.chars().all(|c| c == 'ë'));
        assert_eq!(fit_line("  a   b \n c "), "a b c");
    }

    #[test]
    fn feminine_adjective_rules() {
        let m = crate::morph::Morph::default();
        assert_eq!(feminine(&m, "madh").0, "madhe");
        assert_eq!(feminine(&m, "bukur").0, "bukur");
        assert_eq!(feminine(&m, "ri").0, "re");
        assert_eq!(feminine(&m, "shëndetshëm").0, "shëndetshme");
        assert_eq!(feminine(&m, "kuq").0, "kuqe");
        assert_eq!(feminine(&m, "mirë").0, "mirë");
    }

    #[test]
    fn material_is_one_line() {
        let m = Material { positive: "a".into(), negative: "b".into(), distinction: "c".into(), source: "d".into(), refutation: "e".into() };
        assert_eq!(m.line(), "Pozitive: a. Negative: b. Dallimi: c. Burimi: d. Kushti: e.");
    }
}
