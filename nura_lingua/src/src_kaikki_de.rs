//! Importuesi i burimit «kaikki_de»: lidhjet ndërmjet fjalëve shqipe nga
//! Wiktionary gjermanisht (de.wiktionary, nëpërmjet kaikki.org).
//!
//! Merren vetëm: fjala, pjesa e ligjëratës dhe gjinia (→ `Rel::Lex`), si dhe
//! vargjet synonyms / antonyms / hypernyms / hyponyms / coordinate_terms /
//! derived (→ `Rel::Sem`) me `sense_index` e normalizuar te atributi `sense`.
//! Shpjegimet gjermanisht nuk janë në TSV dhe nuk përdoren kurrë.
//!
//! Rregullat (të deklaruara, pa hamendje):
//! - Hyrjet `form-of` (trajta të lakuara/zgjedhuara), emrat e përveçëm (`name`,
//!   ose fjalë me shkronjë të madhe), shkronjat, prapashtesat/parashtesat dhe
//!   shkurtimet nuk bëhen nyje.
//! - Një skaj pranohet vetëm kur të dy skajet janë fjalë shqipe që ekzistojnë
//!   tashmë në graf OSE janë vetë hyrje të këtij skedari (kështu bien gabimet si
//!   «kafschë»).
//! - Nyja e vetme (i/e/të/së) si fjalë e lidhur refuzohet në çdo lidhje (është
//!   copë e një fraze të ndarë nga wiktextract, p.sh. «i/e paafruar»), përveç kur
//!   hyrja është fjalë funksionale (det/pron/conj/particle/prep).
//! - Fjalët e lidhura me etiketë paradigme (vetë; numër jashtë `derived`) janë
//!   trajta të lakuara (p.sh. «mos» → «mosni» [plural]) dhe refuzohen.
//! - «Gegenwörter» e Wiktionary-t gjermanisht përdoret shpesh për bashkë-hiponime
//!   (stinët, rasat, ditët). Një ANTONYM shënohet ("confidence","low") kur:
//!   lista ka më shumë se 6 elemente; ose hyrja, fjala dhe një e tretë janë të
//!   gjitha antonime me njëra-tjetrën (klikë); ose lista ka ≥3 elemente dhe fjala
//!   nuk e rendit hyrjen si antonim (pa reciprocitet). Arsyeja ruhet te
//!   ("low_reason", …).
//! - HYPONYM drejt një fjale që është edhe hiperonim (paraardhës) i hyrjes hidhet
//!   (kontradiktë, p.sh. «mesnatë» → «kohë»); HYPERNYM-i i kundërt mbetet, low.
//! - HYPERNYM drejt një fushe lënde (`FIELD_LABELS`) dhe HYPERNYM/HYPONYM ndërmjet
//!   dy njësive kohe (`TIME_WORDS`, pjesë–tërësi) shënohen low.
//! - `DERIVED_BLOCK`: prejardhje të botuara por etimologjikisht të gabuara, hidhen.
//! - Kuptimet: një hyrje → "1,2a"; e njëjta fjalë me disa hyrje (pos të ndryshme)
//!   → një skaj i vetëm me "adj:1;noun:2a" (numrat janë lokalë për çdo hyrje).
//! - Drejtimet: HYPERNYM hyrja → hiperonimi; HYPONYM hyrja → hiponimi;
//!   DERIVED_FROM fjala e prejardhur → hyrja (baza).

use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const TSV: &str = "raw/kaikki_de/kaikki_de-sq.tsv";

/// Kufiri mbi të cilin një listë antonimesh shënohet me besueshmëri të ulët.
pub const ANTONYM_LIST_MAX: usize = 6;

struct Entry {
    lemma: String,
    pos: String,
    gender: String,
    accepted: bool,
}

/// Pjesa e ligjëratës e wiktextract → kodi i `Rel::Lex`.
fn pos_code(pos: &str) -> Option<&'static str> {
    match pos {
        "noun" => Some("n"),
        "verb" => Some("v"),
        "adj" => Some("a"),
        "adv" => Some("r"),
        _ => None,
    }
}

/// Pjesët e ligjëratës që nuk bëhen nyje fjale shqipe.
fn excluded_pos(pos: &str) -> bool {
    matches!(pos, "name" | "character" | "suffix" | "prefix" | "abbrev" | "unknown" | "")
}

fn rel_name(linkage: &str) -> Option<&'static str> {
    match linkage {
        "synonyms" => Some("SYNONYM"),
        "antonyms" => Some("ANTONYM"),
        "hypernyms" => Some("HYPERNYM"),
        "hyponyms" => Some("HYPONYM"),
        "coordinate_terms" => Some("COORDINATE"),
        "derived" => Some("DERIVED_FROM"),
        _ => None,
    }
}

fn starts_upper(s: &str) -> bool {
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}

fn norm_space(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "2b" → (Some(2), "b"); "b" → (None, "b"); "x1" → None.
fn split_index(t: &str) -> Option<(Option<u32>, String)> {
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest: String = t.chars().skip(digits.chars().count()).collect();
    match (rest.chars().all(|c| c.is_ascii_lowercase()), digits.is_empty(), rest.is_empty()) {
        (false, _, _) => None,
        (true, true, true) => None,
        (true, true, false) => Some((None, rest)),
        (true, false, _) => digits.parse::<u32>().ok().map(|n| (Some(n), rest)),
    }
}

/// Normalizon `sense_index` të Wiktionary-t gjermanisht:
/// "1, 2a, 2b" → "1,2a,2b"; "1–3" → "1,2,3"; "1b–1d" → "1b,1c,1d";
/// "1a, b, c" → "1a,1b,1c"; diapazonet e paqarta ("1–2b") ruhen si "1-2b".
pub fn normalize_sense_index(s: &str) -> String {
    let flat = s.replace(['–', '—', '−'], "-");
    let mut out: Vec<String> = Vec::new();
    let mut last: Option<u32> = None;
    for raw in flat.split(',') {
        let tok: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        let parts: Vec<&str> = tok.split('-').collect();
        let items: Vec<String> = match (tok.is_empty(), parts.as_slice()) {
            (true, _) => Vec::new(),
            (false, [one]) => match split_index(one) {
                Some((Some(n), l)) => {
                    last = Some(n);
                    vec![format!("{n}{l}")]
                }
                Some((None, l)) => match last {
                    Some(n) => vec![format!("{n}{l}")],
                    None => vec![l],
                },
                None => vec![one.to_string()],
            },
            (false, [a, b]) => expand_range(a, b, &mut last),
            (false, _) => vec![tok.clone()],
        };
        for it in items {
            match out.contains(&it) {
                true => {}
                false => out.push(it),
            }
        }
    }
    out.join(",")
}

fn expand_range(a: &str, b: &str, last: &mut Option<u32>) -> Vec<String> {
    let keep = vec![format!("{a}-{b}")];
    match (split_index(a), split_index(b)) {
        (Some((Some(na), la)), Some((nb, lb))) => {
            let nb = nb.unwrap_or(na);
            *last = Some(nb);
            match (la.is_empty(), lb.is_empty(), na == nb, la.chars().count() == 1 && lb.chars().count() == 1) {
                // "1-3"
                (true, true, _, _) => match na <= nb && nb - na <= 50 {
                    true => (na..=nb).map(|n| n.to_string()).collect(),
                    false => keep,
                },
                // "1b-1d" ose "1b-d"
                (false, false, true, true) => {
                    let (x, y) = (la.chars().next().unwrap_or('a'), lb.chars().next().unwrap_or('a'));
                    match x <= y {
                        true => (x..=y).map(|c| format!("{na}{c}")).collect(),
                        false => keep,
                    }
                }
                _ => keep,
            }
        }
        _ => keep,
    }
}

/// Mbiemrat e shqipes mund të shfaqen me nyjën e përparme («i bukur»); identiteti
/// i fjalës është pa nyje, por vetëm kur forma pa nyje është mbiemër i këtij skedari.
fn strip_adj_article(t: &str, adjectives: &HashSet<String>) -> String {
    let rest = ["i ", "e ", "të "].iter().find_map(|a| t.strip_prefix(a));
    match rest {
        Some(r) => match adjectives.contains(r) {
            true => r.to_string(),
            false => t.to_string(),
        },
        None => t.to_string(),
    }
}

/// Fjalë funksionale: vetëm te këto hyrje një nyje e vetme («i», «e», «të», «së»)
/// mund të jetë fjalë e lidhur e vërtetë (p.sh. «edhe» SYNONYM «e»).
fn function_word_pos(pos: &str) -> bool {
    matches!(pos, "det" | "pron" | "conj" | "particle" | "prep")
}

/// Arsyeja pse një fjalë e lidhur refuzohet para kontrollit të ekzistencës.
///
/// - Nyja e vetme (i/e/të/së) refuzohet në çdo lidhje, përveç kur hyrja është
///   fjalë funksionale: wiktextract i ndan «i/e paafruar», «i vendit» në copa.
/// - Etiketat e paradigmës (vetë, numër) në lidhjet jo-`derived` tregojnë një
///   trajtë të lakuar/zgjedhuar (p.sh. «mos» → «mosni» [plural]), jo një fjalë tjetër.
fn reject_reason(target: &str, linkage: &str, tags: &str, entry_pos: &str) -> Option<&'static str> {
    let person = tags.split('|').any(|t| matches!(t, "first-person" | "second-person" | "third-person"));
    let number = linkage != "derived" && tags.split('|').any(|t| matches!(t, "plural" | "singular"));
    let bare_article = matches!(target, "i" | "e" | "të" | "së") && !function_word_pos(entry_pos);
    let noisy = target.chars().any(|c| "()[]{}/;,=:<>|\"".contains(c));
    let morpheme = target.starts_with('-') || target.ends_with('-');
    match (target.is_empty(), starts_upper(target), morpheme, noisy, bare_article, person || number) {
        (true, _, _, _, _, _) => Some("bosh"),
        (_, true, _, _, _, _) => Some("emër i përveçëm / shkronjë e madhe"),
        (_, _, true, _, _, _) => Some("morfemë"),
        (_, _, _, true, _, _) => Some("shenja jo-leksikore"),
        (_, _, _, _, true, _) => Some("nyje e vetme (copë fraze)"),
        (_, _, _, _, _, true) => Some("trajtë paradigme (vetë/numër)"),
        _ => None,
    }
}

/// Lidhje `derived` të botuara që janë etimologjikisht të gabuara: faqja «jam»
/// e de.wiktionary rendit familjen e «jetë»-s nën prejardhjet e saj, ndërsa
/// «jetoj»/«jetohet» vijnë nga «jetë» dhe «jetë» nuk vjen nga «jam».
/// Çifti është (fjala e prejardhur, baza).
pub const DERIVED_BLOCK: [(&str, &str); 3] = [("jetë", "jam"), ("jetoj", "jam"), ("jetohet", "jam")];

/// Etiketa fushash lënde që de.wiktionary i vë te «Oberbegriffe» (p.sh. «iriq» →
/// «zoologji», «lidhor» → «linguistikë»): nuk janë hiperonime (lloj), por fusha;
/// skaji HYPERNYM drejt tyre shënohet ("confidence","low").
pub const FIELD_LABELS: [&str; 13] = [
    "gjuhësi",
    "linguistikë",
    "gramatikë",
    "zoologji",
    "biologji",
    "botanikë",
    "gjeografi",
    "etnografi",
    "mjekësi",
    "astrologji",
    "mitologji",
    "gjeometri",
    "matematikë",
];

/// Njësi kohe dhe pjesë të ditës/kalendarit. Te këto fjalë de.wiktionary rendit
/// pjesët si «Unterbegriffe» (muaj → sekondë, vit → ditë) dhe tërësinë si
/// «Oberbegriffe» (ditë → muaj): kjo është marrëdhënie pjesë–tërësi, jo lloj.
/// HYPERNYM/HYPONYM ndërmjet dy fjalëve të kësaj liste shënohen ("confidence","low").
pub const TIME_WORDS: [&str; 33] = [
    "nanosekondë",
    "mikrosekondë",
    "milisekondë",
    "sekondë",
    "minutë",
    "orë",
    "ditë",
    "natë",
    "javë",
    "muaj",
    "vit",
    "mot",
    "dekadë",
    "shekull",
    "mijëvjeçar",
    "dhjetëvjeçar",
    "tremujor",
    "gjashtëmujor",
    "gjysmëvit",
    "çerek",
    "epokë",
    "mëngjes",
    "paradite",
    "mesditë",
    "pasdite",
    "mbrëmje",
    "mesnatë",
    "sot",
    "dje",
    "pardje",
    "nesër",
    "pasnesër",
    "fundjavë",
];

/// Një lidhje e mundshme pas filtrave të formës (para kontrollit të ekzistencës).
struct Cand {
    nr: String,
    lemma: String,
    pos: String,
    rel: &'static str,
    target: String,
    sense: String,
    len: usize,
}

/// Paraardhësit e `w` sipas HYPERNYM-eve të burimit (BFS, thellësi e kufizuar).
fn ancestors(w: &str, hyper: &HashMap<String, HashSet<String>>) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut front: Vec<String> = vec![w.to_string()];
    for _ in 0..8 {
        let mut next = Vec::new();
        for x in &front {
            for y in hyper.get(x).into_iter().flatten() {
                match seen.insert(y.clone()) {
                    true => next.push(y.clone()),
                    false => {}
                }
            }
        }
        front = next;
    }
    seen
}

fn push_reason(v: &mut Vec<&'static str>, r: &'static str) {
    match v.contains(&r) {
        true => {}
        false => v.push(r),
    }
}

pub fn import(g: &mut Graph, _morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let edges_before = g.edges.len();
    // ── Kalimi 1: hyrjet ──
    let mut entries: HashMap<String, Entry> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut skip: BTreeMap<&'static str, usize> = BTreeMap::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = l.split('\t').collect();
        match (c.first().copied(), c.len() >= 6) {
            (Some("E"), true) => {
                let word = norm_space(c[2]);
                let reason = match (c[5] == "1", starts_upper(&word), excluded_pos(c[3]), word.starts_with('-') || word.ends_with('-')) {
                    (true, _, _, _) => Some("form-of (trajtë e lakuar)"),
                    (_, true, _, _) => Some("emër i përveçëm / shkronjë e madhe"),
                    (_, _, true, _) => Some("pos e përjashtuar (emër i përveçëm, shkronjë, morfemë, shkurtim)"),
                    (_, _, _, true) => Some("morfemë"),
                    _ => None,
                };
                match reason {
                    Some(r) => *skip.entry(r).or_default() += 1,
                    None => {}
                }
                order.push(c[1].to_string());
                entries.insert(
                    c[1].to_string(),
                    Entry { lemma: word.to_lowercase(), pos: c[3].to_string(), gender: c[4].to_string(), accepted: reason.is_none() },
                );
            }
            _ => {}
        }
    }
    let lemmas: HashSet<String> = entries.values().filter(|e| e.accepted).map(|e| e.lemma.clone()).collect();
    let adjectives: HashSet<String> =
        entries.values().filter(|e| e.accepted && e.pos == "adj").map(|e| e.lemma.clone()).collect();
    let preexisting = lemmas.iter().filter(|l| g.get(&word_key(Lang::Sq, l)).is_some()).count();
    // Nyjet e fjalëve dhe të dhënat gramatikore.
    let (mut lex, mut gender_odd) = (0usize, 0usize);
    for nr in &order {
        let e = match entries.get(nr) {
            Some(e) => e,
            None => continue,
        };
        match e.accepted {
            true => {}
            false => continue,
        }
        let w = g.ensure_word(Lang::Sq, &e.lemma);
        let p = match pos_code(&e.pos) {
            Some(p) => p,
            None => continue,
        };
        let mut attrs = vec![("pos".to_string(), p.to_string())];
        match (p, e.gender.as_str()) {
            ("n", "m" | "f" | "n" | "mf") => attrs.push(("gender".to_string(), e.gender.clone())),
            ("n", "") => {}
            ("n", _) => gender_odd += 1,
            _ => {}
        }
        let n0 = g.edges.len();
        g.add_edge(Edge { from: w, to: w, rel: Rel::Lex, source: Source::KaikkiDe, lang: Some(Lang::Sq), method: Method::Published, attrs });
        lex += g.edges.len() - n0;
    }
    // ── Kalimi 2a: lidhjet e mundshme pas filtrave të formës ──
    let mut cands: Vec<Cand> = Vec::new();
    let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut rejected: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut from_skipped = 0usize;
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = l.split('\t').collect();
        let (nr, linkage, target, sense, len, tags) = match (c.first().copied(), c.len() >= 7) {
            (Some("L"), true) => (c[1], c[2], norm_space(c[3]), c[4], c[5].parse::<usize>().unwrap_or(0), c[6]),
            _ => continue,
        };
        let rel = match rel_name(linkage) {
            Some(r) => r,
            None => continue,
        };
        *seen.entry(rel).or_default() += 1;
        let entry = match entries.get(nr).filter(|e| e.accepted) {
            Some(e) => e,
            None => {
                from_skipped += 1;
                continue;
            }
        };
        match reject_reason(&target, linkage, tags, &entry.pos) {
            Some(r) => {
                *rejected.entry(r).or_default() += 1;
                continue;
            }
            None => {}
        }
        let t = strip_adj_article(&target.to_lowercase(), &adjectives);
        match t == entry.lemma {
            true => {
                *rejected.entry("vetë-lidhje").or_default() += 1;
                continue;
            }
            false => {}
        }
        cands.push(Cand {
            nr: nr.to_string(),
            lemma: entry.lemma.clone(),
            pos: entry.pos.clone(),
            rel,
            target: t,
            sense: normalize_sense_index(sense),
            len,
        });
    }
    // ── Struktura e burimit (për kontrollet e qëndrueshmërisë) ──
    // antonimet: të drejtuara (fjalë → listë) dhe të padrejtuara (graf)
    let mut ant_dir: HashMap<String, HashSet<String>> = HashMap::new();
    let mut ant_und: HashMap<String, HashSet<String>> = HashMap::new();
    let mut ant_count: HashMap<String, HashSet<String>> = HashMap::new();
    // hiperonimet sipas fjalës; hiperonimet/hiponimet sipas hyrjes
    let mut hyper: HashMap<String, HashSet<String>> = HashMap::new();
    let mut hyper_nr: HashMap<String, HashSet<String>> = HashMap::new();
    let mut hypo_nr: HashMap<String, HashSet<String>> = HashMap::new();
    for k in &cands {
        match k.rel {
            "ANTONYM" => {
                ant_dir.entry(k.lemma.clone()).or_default().insert(k.target.clone());
                ant_und.entry(k.lemma.clone()).or_default().insert(k.target.clone());
                ant_und.entry(k.target.clone()).or_default().insert(k.lemma.clone());
                ant_count.entry(k.nr.clone()).or_default().insert(k.target.clone());
            }
            "HYPERNYM" => {
                hyper.entry(k.lemma.clone()).or_default().insert(k.target.clone());
                hyper_nr.entry(k.nr.clone()).or_default().insert(k.target.clone());
            }
            "HYPONYM" => {
                hypo_nr.entry(k.nr.clone()).or_default().insert(k.target.clone());
            }
            _ => {}
        }
    }
    let empty: HashSet<String> = HashSet::new();
    let time: HashSet<&str> = TIME_WORDS.iter().copied().collect();
    let mut anc_cache: HashMap<String, HashSet<String>> = HashMap::new();
    // ── Kalimi 2b: lidhjet e pranuara, të bashkuara për (nga, lidhja, te) ──
    struct Link {
        from: String,
        to: String,
        rel: &'static str,
        // (pos e hyrjes, kuptimet) — numrat e kuptimeve janë lokalë për çdo hyrje
        groups: Vec<(String, Vec<String>)>,
        low: Vec<&'static str>,
    }
    let mut links: Vec<Link> = Vec::new();
    let mut at: HashMap<(String, &'static str, String), usize> = HashMap::new();
    let mut unknown_samples: Vec<String> = Vec::new();
    let (mut unknown, mut via_existing) = (0usize, 0usize);
    for k in &cands {
        let (from, to) = match k.rel {
            "DERIVED_FROM" => (k.target.clone(), k.lemma.clone()),
            _ => (k.lemma.clone(), k.target.clone()),
        };
        // prejardhje e botuar por e gabuar (listë e deklaruar)
        let blocked = k.rel == "DERIVED_FROM" && DERIVED_BLOCK.iter().any(|(d, b)| *d == from && *b == to);
        // HYPONYM drejt një paraardhësi të hyrjes (p.sh. «mesnatë» → «kohë», ku
        // «kohë» është edhe hiperonim): kontradiktë, skaji HYPONYM hidhet.
        let contradiction = match k.rel {
            "HYPONYM" => {
                let anc = anc_cache.entry(k.lemma.clone()).or_insert_with(|| ancestors(&k.lemma, &hyper));
                anc.contains(&k.target)
            }
            _ => false,
        };
        match (blocked, contradiction) {
            (true, _) => {
                *rejected.entry("prejardhje e gabuar (DERIVED_BLOCK)").or_default() += 1;
                continue;
            }
            (_, true) => {
                *rejected.entry("kontradiktë: hiponimi është edhe hiperonim (paraardhës)").or_default() += 1;
                continue;
            }
            _ => {}
        }
        match (lemmas.contains(&k.target), g.get(&word_key(Lang::Sq, &k.target)).is_some()) {
            (true, _) => {}
            (false, true) => via_existing += 1,
            (false, false) => {
                unknown += 1;
                match unknown_samples.len() < 12 {
                    true => unknown_samples.push(k.target.clone()),
                    false => {}
                }
                continue;
            }
        }
        // ── Besueshmëria ──
        let mut low: Vec<&'static str> = Vec::new();
        match k.rel {
            "ANTONYM" => {
                let mine = ant_und.get(&k.lemma).unwrap_or(&empty);
                let theirs = ant_und.get(&k.target).unwrap_or(&empty);
                // trekëndësh: hyrja, fjala dhe një e tretë janë të gjitha «të kundërta»
                // me njëra-tjetrën → bashkë-hiponime (stinët, rasat, ditët), jo antonime
                let clique = mine.iter().any(|x| x != &k.target && x != &k.lemma && theirs.contains(x));
                let n = ant_count.get(&k.nr).map(|s| s.len()).unwrap_or(0);
                let reciprocal = ant_dir.get(&k.target).map(|s| s.contains(&k.lemma)).unwrap_or(false);
                match k.len > ANTONYM_LIST_MAX {
                    true => push_reason(&mut low, "lista>6"),
                    false => {}
                }
                match clique {
                    true => push_reason(&mut low, "klikë antonimesh (bashkë-hiponime)"),
                    false => {}
                }
                match (n >= 3, reciprocal) {
                    (true, false) => push_reason(&mut low, "listë ≥3 pa reciprocitet"),
                    _ => {}
                }
            }
            "HYPERNYM" => {
                match hypo_nr.get(&k.nr).map(|s| s.contains(&k.target)).unwrap_or(false) {
                    true => push_reason(&mut low, "kontradiktë hiperonim/hiponim"),
                    false => {}
                }
                match FIELD_LABELS.contains(&k.target.as_str()) {
                    true => push_reason(&mut low, "fushë lënde, jo lloj"),
                    false => {}
                }
            }
            _ => {}
        }
        match (matches!(k.rel, "HYPERNYM" | "HYPONYM"), time.contains(k.lemma.as_str()) && time.contains(k.target.as_str())) {
            (true, true) => push_reason(&mut low, "njësi kohe: pjesë–tërësi, jo lloj"),
            _ => {}
        }
        let senses: Vec<String> = k.sense.split(',').filter(|x| !x.is_empty()).map(|x| x.to_string()).collect();
        let key = (from.clone(), k.rel, to.clone());
        match at.get(&key) {
            Some(&i) => {
                let link = &mut links[i];
                match link.groups.iter().position(|(p, _)| p == &k.pos) {
                    Some(j) => {
                        for x in senses {
                            match link.groups[j].1.contains(&x) {
                                true => {}
                                false => link.groups[j].1.push(x),
                            }
                        }
                    }
                    None => link.groups.push((k.pos.clone(), senses)),
                }
                for r in low {
                    push_reason(&mut link.low, r);
                }
            }
            None => {
                at.insert(key, links.len());
                links.push(Link { from, to, rel: k.rel, groups: vec![(k.pos.clone(), senses)], low });
            }
        }
    }
    let mut added: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut low_by: BTreeMap<String, usize> = BTreeMap::new();
    for k in &links {
        let (a, b) = (g.get(&word_key(Lang::Sq, &k.from)), g.get(&word_key(Lang::Sq, &k.to)));
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };
        let mut attrs = Vec::new();
        // Një hyrje: "1,2a". Disa hyrje me pos të ndryshme të së njëjtës fjalë:
        // "adj:1;noun:2a" (numrat e kuptimeve janë lokalë për çdo hyrje).
        let nonempty: Vec<&(String, Vec<String>)> = k.groups.iter().filter(|(_, s)| !s.is_empty()).collect();
        let sense = match (k.groups.len(), nonempty.len()) {
            (_, 0) => None,
            (1, _) => Some(k.groups[0].1.join(",")),
            _ => Some(nonempty.iter().map(|(p, s)| format!("{p}:{}", s.join(","))).collect::<Vec<_>>().join(";")),
        };
        match sense {
            Some(s) => attrs.push(("sense".to_string(), s)),
            None => {}
        }
        match k.low.is_empty() {
            true => {}
            false => {
                attrs.push(("confidence".to_string(), "low".to_string()));
                attrs.push(("low_reason".to_string(), k.low.join("; ")));
            }
        }
        let n0 = g.edges.len();
        g.add_edge(Edge {
            from: a,
            to: b,
            rel: Rel::Sem(k.rel.to_string()),
            source: Source::KaikkiDe,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs,
        });
        let new = g.edges.len() - n0;
        *added.entry(k.rel).or_default() += new;
        match (k.low.is_empty(), new) {
            (false, 1) => *low_by.entry(format!("{} [{}]", k.rel, k.low.join("; "))).or_default() += 1,
            _ => {}
        }
    }
    let sem: usize = added.values().sum();
    let low_n: usize = low_by.values().sum();
    rep.note(format!(
        "kaikki_de: {} hyrje, {} të pranuara si fjalë shqipe ({} ekzistonin tashmë), të anashkaluara {:?}; {lex} skaje gramatikore (Rel::Lex){}",
        entries.len(),
        lemmas.len(),
        preexisting,
        skip,
        match gender_odd {
            0 => String::new(),
            n => format!(", {n} gjini të përziera të pashënuara"),
        }
    ));
    rep.note(format!(
        "kaikki_de: lidhje në TSV {:?}; skaje Rel::Sem të shtuara {:?} = {sem} ({low_n} me besueshmëri të ulët, {via_existing} lidhje drejt fjalëve që ekzistonin vetëm në graf)",
        seen, added
    ));
    rep.note(format!("kaikki_de: besueshmëri e ulët sipas arsyes {:?}", low_by));
    rep.note(format!(
        "kaikki_de: refuzuar — {from_skipped} nga hyrje të anashkaluara, {unknown} fjalë që nuk janë as hyrje as nyje ekzistuese (p.sh. {:?}), sipas formës/qëndrueshmërisë {:?}",
        unknown_samples, rejected
    ));
    g.edges.len() - edges_before
}

#[cfg(test)]
mod tests {
    use super::*;

    // TSV në formatin e saktë të `nura_fetch::src_kaikki_de::extract`, me të dhëna
    // reale (të shkurtuara) nga kaikki.org-dictionary-Albanisch.jsonl.
    const FIX: &str = "# kaikki_de — test\n\
E\t1\tdem\tnoun\tm\t0\n\
L\t1\tsynonyms\tka\t1\t1\t\n\
L\t1\tantonyms\tlopë\t1\t1\t\n\
L\t1\thypernyms\tkafschë\t1\t6\t\n\
L\t1\thypernyms\tshtazë\t1\t6\t\n\
L\t1\thypernyms\tzodiak\t2\t6\t\n\
L\t1\thyponyms\tdem race\t1\t1\t\n\
L\t1\tcoordinate_terms\tviç\t1\t1\t\n\
E\t2\tlopë\tnoun\tf\t0\n\
L\t2\tantonyms\tdem\t1\t1\t\n\
E\t3\tshtazë\tnoun\tf\t0\n\
E\t4\tmal\tnoun\tm\t0\n\
L\t4\tderived\tMalësi\t1a, 1b\t3\t\n\
L\t4\tderived\tmalësor\t1a, 1b\t3\t\n\
L\t4\tderived\tmalor\t1–3\t3\t\n\
E\t5\tmalësor\tnoun\tm\t0\n\
E\t6\tmalor\tadj\tm\t0\n\
E\t7\tekonomi\tnoun\tf\t0\n\
L\t7\tantonyms\tbiologji\t1\t8\t\n\
E\t8\tbiologji\tnoun\tf\t0\n\
E\t9\tlopës\tnoun\tf\t1\n\
L\t9\tsynonyms\tlopë\t1\t1\t\n\
E\t10\tMehmet\tname\tm\t0\n\
L\t10\tderived\tMehmeti\t1\t1\t\n\
E\t11\t-i\tsuffix\tm\t0\n\
L\t11\tantonyms\t-a\t1a\t20\tfirst-person|singular\n\
E\t12\tdua\tverb\t\t0\n\
L\t12\tderived\ti\t1\t3\tadjective\n\
L\t12\tderived\tdashur\t1\t3\tparticiple\n\
L\t12\tderived\tdashuroj\t1\t3\t\n\
E\t13\tdashuroj\tverb\t\t0\n\
E\t14\tmirë\tnoun\tmn\t0\n";

    // Rastet e rishikimit: stinët (klikë), «i/e paafruar» i ndarë, «kohë» si
    // hiperonim dhe hiponim, njësitë e kohës, «jam» → familja e «jetë»,
    // fushë lënde si hiperonim, trajtë shumësi si sinonim, fjalë funksionale.
    const FIX2: &str = "# kaikki_de — rastet e rishikimit\n\
E\t20\tdimër\tnoun\tm\t0\n\
L\t20\tantonyms\tpranverë\t1\t3\t\n\
L\t20\tantonyms\tverë\t1\t3\t\n\
L\t20\tantonyms\tvjeshtë\t1\t3\t\n\
E\t21\tverë\tnoun\tf\t0\n\
L\t21\tantonyms\tpranverë\t1\t3\t\n\
L\t21\tantonyms\tvjeshtë\t1\t3\t\n\
L\t21\tantonyms\tdimër\t1\t3\t\n\
E\t22\tvjeshtë\tnoun\tf\t0\n\
L\t22\tantonyms\tpranverë\t1\t3\t\n\
L\t22\tantonyms\tverë\t1\t3\t\n\
L\t22\tantonyms\tdimër\t1\t3\t\n\
E\t23\tpranverë\tnoun\tf\t0\n\
E\t24\tzi\tadj\tm\t0\n\
L\t24\tantonyms\tbardhë\t1\t1\t\n\
E\t25\tbardhë\tadj\tm\t0\n\
L\t25\tantonyms\tzi\t1\t1\t\n\
E\t26\tafruar\tadj\tn\t0\n\
L\t26\tantonyms\ti\t1, 2\t5\t\n\
L\t26\tantonyms\te\t1, 2\t5\t\n\
L\t26\tantonyms\tpaafruar\t1, 2\t5\t\n\
E\t27\tpaafruar\tadj\tm\t0\n\
E\t28\ti\tdet\t\t0\n\
E\t29\te\tdet\t\t0\n\
E\t30\tedhe\tconj\t\t0\n\
L\t30\tsynonyms\te\t1\t2\t\n\
E\t31\tmesnatë\tnoun\tf\t0\n\
L\t31\thypernyms\tkohë\t1\t1\t\n\
L\t31\thyponyms\tkohë\t1\t2\t\n\
L\t31\thyponyms\torë\t1\t2\t\n\
E\t32\tkohë\tnoun\tf\t0\n\
E\t33\torë\tnoun\tf\t0\n\
L\t33\thypernyms\tditë\t1\t2\t\n\
L\t33\thypernyms\tkohë\t1\t2\t\n\
E\t34\tditë\tnoun\tf\t0\n\
E\t35\tjam\tverb\t\t0\n\
L\t35\tderived\tjetë\t\t3\tnoun\n\
L\t35\tderived\tjetoj\t\t3\t\n\
L\t35\tderived\tqenie\t\t3\t\n\
E\t36\tjetë\tnoun\tf\t0\n\
E\t37\tjetoj\tverb\t\t0\n\
E\t38\tqenie\tnoun\tf\t0\n\
E\t39\tiriq\tnoun\tm\t0\n\
L\t39\thypernyms\tgjitar\t1\t2\t\n\
L\t39\thypernyms\tzoologji\t1\t2\t\n\
E\t40\tgjitar\tnoun\tm\t0\n\
E\t41\tzoologji\tnoun\tf\t0\n\
E\t42\tmos\tparticle\t\t0\n\
L\t42\tsynonyms\tmosni\t2\t1\tplural\n\
E\t43\tmosni\tparticle\t\t0\n\
E\t44\tshqiptar\tadj\tm\t0\n\
L\t44\tsynonyms\tshqip\t1\t1\t\n\
E\t45\tshqiptar\tnoun\tm\t0\n\
L\t45\tsynonyms\tshqip\t2a, 2b\t1\t\n\
E\t46\tshqip\tadj\t\t0\n\
E\t47\tvendës\tnoun\tm\t0\n\
L\t47\tantonyms\tardhës\t1\t3\t\n\
L\t47\tantonyms\tardhur\t1, 2\t3\t\n\
L\t47\tantonyms\tmik\t2\t3\t\n\
E\t48\tardhur\tadj\tm\t0\n\
L\t48\tantonyms\tvendës\t1\t1\t\n\
E\t49\tmik\tnoun\tm\t0\n";

    fn run(t: &str) -> (Graph, ImportReport, usize) {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut rep = ImportReport::default();
        let n = import(&mut g, &mut m, t, &mut rep);
        (g, rep, n)
    }

    fn sem(g: &Graph, a: &str, rel: &str, b: &str) -> Option<Edge> {
        let (x, y) = (g.get(&word_key(Lang::Sq, a))?, g.get(&word_key(Lang::Sq, b))?);
        g.out_edges(x).find(|(_, e)| e.to == y && e.rel == Rel::Sem(rel.into())).map(|(_, e)| e.clone())
    }

    fn lex(g: &Graph, a: &str) -> Vec<Vec<(String, String)>> {
        let x = g.get(&word_key(Lang::Sq, a)).unwrap();
        g.out_edges(x).filter(|(_, e)| e.rel == Rel::Lex).map(|(_, e)| e.attrs.clone()).collect()
    }

    #[test]
    fn imports_linkages_with_filters() {
        let mut g = Graph::new();
        // «ka» ekziston tashmë nga një burim tjetër; «viç» jo.
        g.ensure_word(Lang::Sq, "ka");
        let mut m = Morph::default();
        let mut rep = ImportReport::default();
        let n = import(&mut g, &mut m, FIX, &mut rep);
        assert!(n > 0);
        // dem ↔ lopë, dem ⊂ shtazë
        let e = sem(&g, "dem", "ANTONYM", "lopë").unwrap();
        assert_eq!(e.attr("sense"), Some("1"));
        assert_eq!(e.source, Source::KaikkiDe);
        assert_eq!(e.method, Method::Published);
        assert!(sem(&g, "lopë", "ANTONYM", "dem").is_some());
        assert!(sem(&g, "dem", "HYPERNYM", "shtazë").is_some());
        // nyje ekzistuese pranohet
        assert!(sem(&g, "dem", "SYNONYM", "ka").is_some());
        // gabimet e shkrimit dhe fjalët e panjohura bien
        assert!(g.get(&word_key(Lang::Sq, "kafschë")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "zodiak")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "viç")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "dem race")).is_none());
        // prejardhja: e prejardhura → baza, me sense të normalizuar
        let d = sem(&g, "malësor", "DERIVED_FROM", "mal").unwrap();
        assert_eq!(d.attr("sense"), Some("1a,1b"));
        assert_eq!(sem(&g, "malor", "DERIVED_FROM", "mal").unwrap().attr("sense"), Some("1,2,3"));
        assert!(sem(&g, "mal", "DERIVED_FROM", "malësor").is_none());
        assert!(sem(&g, "dashuroj", "DERIVED_FROM", "dua").is_some());
        // emrat e përveçëm nuk ulen në shkronja të vogla
        assert!(g.get(&word_key(Lang::Sq, "malësi")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "mehmet")).is_none());
        // nyja e vetme «i» nuk është fjalë e prejardhur
        assert!(g.get(&word_key(Lang::Sq, "i")).is_none());
        // listë e gjatë antonimesh → besueshmëri e ulët
        let ek = sem(&g, "ekonomi", "ANTONYM", "biologji").unwrap();
        assert_eq!(ek.attr("confidence"), Some("low"));
        assert_eq!(ek.attr("low_reason"), Some("lista>6"));
        assert_eq!(e.attr("confidence"), None);
        // hyrjet form-of dhe morfemat nuk bëhen nyje
        assert!(g.get(&word_key(Lang::Sq, "lopës")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "-i")).is_none());
    }

    #[test]
    fn co_hyponym_antonyms_are_low() {
        let (g, _, _) = run(FIX2);
        // stinët: secila «e kundërt» me të tjerat → klikë → low (edhe dimër–verë)
        for (a, b) in [("dimër", "vjeshtë"), ("vjeshtë", "verë"), ("verë", "pranverë"), ("dimër", "verë")] {
            let e = sem(&g, a, "ANTONYM", b).unwrap();
            assert_eq!(e.attr("confidence"), Some("low"), "{a}–{b}");
            assert!(e.attr("low_reason").unwrap().contains("klikë"));
        }
        // çift i vërtetë, reciprok, pa të tretë → i besueshëm
        assert_eq!(sem(&g, "zi", "ANTONYM", "bardhë").unwrap().attr("confidence"), None);
        assert_eq!(sem(&g, "bardhë", "ANTONYM", "zi").unwrap().attr("confidence"), None);
        // «i», «e» hiqen si copa; lista efektive ka 1 element → pa shënim
        let e = sem(&g, "afruar", "ANTONYM", "paafruar").unwrap();
        assert_eq!(e.attr("confidence"), None);
        // listë me 3 elemente: «ardhur» reciprok → i besueshëm; «mik» jo → low
        assert_eq!(sem(&g, "vendës", "ANTONYM", "ardhur").unwrap().attr("confidence"), None);
        let m = sem(&g, "vendës", "ANTONYM", "mik").unwrap();
        assert_eq!(m.attr("low_reason"), Some("listë ≥3 pa reciprocitet"));
    }

    #[test]
    fn article_fragments_rejected_except_function_words() {
        let (g, rep, _) = run(FIX2);
        assert!(sem(&g, "afruar", "ANTONYM", "i").is_none());
        assert!(sem(&g, "afruar", "ANTONYM", "e").is_none());
        // «edhe» është lidhëz: «e» si sinonim është e vërtetë
        assert!(sem(&g, "edhe", "SYNONYM", "e").is_some());
        assert!(rep.lines.iter().any(|l| l.contains("nyje e vetme (copë fraze)")));
    }

    #[test]
    fn hypernym_hyponym_consistency() {
        let (g, rep, _) = run(FIX2);
        // «kohë» është edhe hiperonim edhe «hiponim» i «mesnatë»-s: HYPONYM hidhet,
        // HYPERNYM mbetet por me besueshmëri të ulët
        assert!(sem(&g, "mesnatë", "HYPONYM", "kohë").is_none());
        let h = sem(&g, "mesnatë", "HYPERNYM", "kohë").unwrap();
        assert_eq!(h.attr("confidence"), Some("low"));
        assert!(rep.lines.iter().any(|l| l.contains("kontradiktë: hiponimi")));
        // njësitë e kohës: pjesë–tërësi, jo lloj
        let p = sem(&g, "mesnatë", "HYPONYM", "orë").unwrap();
        assert_eq!(p.attr("confidence"), Some("low"));
        assert_eq!(sem(&g, "orë", "HYPERNYM", "ditë").unwrap().attr("confidence"), Some("low"));
        assert_eq!(sem(&g, "orë", "HYPERNYM", "kohë").unwrap().attr("confidence"), None);
        // fushë lënde si hiperonim → low; hiperonim i vërtetë → jo
        assert_eq!(sem(&g, "iriq", "HYPERNYM", "zoologji").unwrap().attr("confidence"), Some("low"));
        assert_eq!(sem(&g, "iriq", "HYPERNYM", "gjitar").unwrap().attr("confidence"), None);
    }

    #[test]
    fn blocked_derivations_forms_and_multi_pos_senses() {
        let (g, _, _) = run(FIX2);
        assert!(sem(&g, "jetë", "DERIVED_FROM", "jam").is_none());
        assert!(sem(&g, "jetoj", "DERIVED_FROM", "jam").is_none());
        assert!(sem(&g, "qenie", "DERIVED_FROM", "jam").is_some());
        // «mosni» [plural] është trajtë, jo sinonim
        assert!(sem(&g, "mos", "SYNONYM", "mosni").is_none());
        // dy hyrje (adj, noun) të së njëjtës fjalë → një skaj, kuptimet sipas pos
        let s = sem(&g, "shqiptar", "SYNONYM", "shqip").unwrap();
        assert_eq!(s.attr("sense"), Some("adj:1;noun:2a,2b"));
        let x = g.get(&word_key(Lang::Sq, "shqiptar")).unwrap();
        assert_eq!(g.out_edges(x).filter(|(_, e)| e.rel == Rel::Sem("SYNONYM".into())).count(), 1);
    }

    #[test]
    fn lex_pos_and_gender() {
        let (g, rep, _) = run(FIX);
        let kv = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(lex(&g, "dem"), vec![vec![kv("pos", "n"), kv("gender", "m")]]);
        assert_eq!(lex(&g, "lopë"), vec![vec![kv("pos", "n"), kv("gender", "f")]]);
        assert_eq!(lex(&g, "malor"), vec![vec![kv("pos", "a")]]);
        assert_eq!(lex(&g, "dua"), vec![vec![kv("pos", "v")]]);
        // gjini e përzier jo-standarde: pos po, gjini jo
        assert_eq!(lex(&g, "mirë"), vec![vec![kv("pos", "n")]]);
        let e = g.edges.iter().find(|e| e.rel == Rel::Lex).unwrap();
        assert_eq!(e.from, e.to);
        assert!(rep.lines.iter().any(|l| l.contains("kafschë")));
    }

    #[test]
    fn sense_index_forms() {
        assert_eq!(normalize_sense_index("1"), "1");
        assert_eq!(normalize_sense_index("1, 2a, 2b"), "1,2a,2b");
        assert_eq!(normalize_sense_index("1–3"), "1,2,3");
        assert_eq!(normalize_sense_index("1b–1d"), "1b,1c,1d");
        assert_eq!(normalize_sense_index("1a, b, c"), "1a,1b,1c");
        assert_eq!(normalize_sense_index("1–2b"), "1-2b");
        assert_eq!(normalize_sense_index("9a, 11b"), "9a,11b");
        assert_eq!(normalize_sense_index("1, 1b, 3"), "1,1b,3");
        assert_eq!(normalize_sense_index(""), "");
    }

    #[test]
    fn idempotent_on_reimport() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut rep = ImportReport::default();
        let a = import(&mut g, &mut m, FIX, &mut rep);
        let b = import(&mut g, &mut m, FIX, &mut rep);
        let c = import(&mut g, &mut m, FIX2, &mut rep);
        let d = import(&mut g, &mut m, FIX2, &mut rep);
        assert!(a > 0 && c > 0);
        assert_eq!(b, 0);
        assert_eq!(d, 0);
    }
}
