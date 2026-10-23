//! Importuesit për formatet reale të paketave të shkarkuara.
//!
//! - WordNet 3.0: `data.noun|verb|adj|adv` (formati "WNDB" i Princeton-it).
//! - Albanet/OMW: `wn-data-als.tab` (offset-pos ⇥ als:lemma|def|exe ⇥ vlerë).
//! - ConceptNet 5.7: CSV me tabulim (uri ⇥ rel ⇥ start ⇥ end ⇥ json).
//! - Universal Dependencies: CoNLL-U (10 kolona, komente `# sent_id`, `# text`).
//!
//! Të tre burimet NUK trajtohen si i njëjti lloj grafi: WordNet jep kuptime
//! dhe marrëdhënie ndërmjet tyre; ConceptNet jep koncepte-terma dhe skaje
//! "commonsense"; UD jep fjali, forma, tipare dhe varësi.

use crate::graph::*;
use crate::morph::{Morph, MorphEntry};

#[derive(Debug, Default, Clone)]
pub struct ImportReport {
    pub lines: Vec<String>,
}

impl ImportReport {
    pub fn note(&mut self, s: String) {
        self.lines.push(s);
    }
}

pub fn normalize_synset(offset_pos: &str) -> Option<String> {
    let (off, pos) = offset_pos.split_once('-')?;
    let p = match pos {
        "s" => "a",
        other => other,
    };
    match (off.len() == 8 && off.bytes().all(|b| b.is_ascii_digit()), Pos::from_wn(p)) {
        (true, Some(_)) => Some(format!("{off}-{p}")),
        _ => None,
    }
}

// ───────────────────────────── WordNet 3.0 ─────────────────────────────

/// Analizon një skedar `data.<pos>` të PWN 3.0.
pub fn import_pwn_data(g: &mut Graph, text: &str, rep: &mut ImportReport) -> Result<usize, String> {
    let mut n = 0usize;
    let mut ptrs = 0usize;
    for line in text.lines() {
        match line.starts_with("  ") || line.trim().is_empty() {
            true => continue, // koka e licencës
            false => {}
        }
        let (head, gloss) = match line.split_once(" | ") {
            Some((h, gl)) => (h, gl.trim()),
            None => (line, ""),
        };
        let f: Vec<&str> = head.split_whitespace().collect();
        let (off, ss_type) = match (f.first(), f.get(2)) {
            (Some(o), Some(t)) => (*o, *t),
            _ => return Err(format!("rresht PWN i pavlefshëm: {}", &line[..line.len().min(60)])),
        };
        let synset = normalize_synset(&format!("{off}-{ss_type}"))
            .ok_or_else(|| format!("synset i pavlefshëm {off}-{ss_type}"))?;
        let pos = Pos::from_wn(ss_type).ok_or("pos")?;
        let w_cnt = usize::from_str_radix(f.get(3).ok_or("w_cnt")?, 16).map_err(|e| e.to_string())?;
        let sid = g.ensure(
            sense_key(Source::Pwn30, &synset),
            NodeKind::Sense { source: Source::Pwn30, synset: synset.clone(), pos },
        );
        let mut i = 4;
        for _ in 0..w_cnt {
            let w = f.get(i).ok_or("word")?;
            let lemma = strip_adj_marker(w).replace('_', " ");
            g.add_label(sid, Text { lang: Lang::En, text: lemma, source: Source::Pwn30 });
            i += 2;
        }
        let p_cnt: usize = f.get(i).ok_or("p_cnt")?.parse().map_err(|_| "p_cnt".to_string())?;
        i += 1;
        for _ in 0..p_cnt {
            let (sym, toff, tpos, st) = match (f.get(i), f.get(i + 1), f.get(i + 2), f.get(i + 3)) {
                (Some(a), Some(b), Some(c), Some(d)) => (*a, *b, *c, *d),
                _ => return Err(format!("tregues PWN i paplotë në {synset}")),
            };
            i += 4;
            let tsyn = normalize_synset(&format!("{toff}-{tpos}")).ok_or("tregues synset")?;
            let tpos_v = Pos::from_wn(tpos).ok_or("tregues pos")?;
            let tid = g.ensure(
                sense_key(Source::Pwn30, &tsyn),
                NodeKind::Sense { source: Source::Pwn30, synset: tsyn, pos: tpos_v },
            );
            let attrs = match st {
                "0000" => vec![],
                lex => vec![("lex".to_string(), lex.to_string())],
            };
            g.add_edge(Edge {
                from: sid,
                to: tid,
                rel: Rel::Wn(sym.to_string()),
                source: Source::Pwn30,
                lang: Some(Lang::En),
                method: Method::Published,
                attrs,
            });
            ptrs += 1;
        }
        match gloss.is_empty() {
            true => {}
            false => {
                let (def, exs) = split_pwn_gloss(gloss);
                let node = &mut g.nodes[sid as usize];
                node.glosses.push(Text { lang: Lang::En, text: def, source: Source::Pwn30 });
                for ex in exs {
                    node.examples.push(Text { lang: Lang::En, text: ex, source: Source::Pwn30 });
                }
            }
        }
        n += 1;
    }
    rep.note(format!("pwn30: {n} synsete, {ptrs} tregues"));
    Ok(n)
}

fn strip_adj_marker(w: &str) -> &str {
    match w.find('(') {
        Some(p) => &w[..p],
        None => w,
    }
}

/// "def; \"ex1\"; \"ex2\"" → (def, [ex1, ex2])
fn split_pwn_gloss(gloss: &str) -> (String, Vec<String>) {
    let mut def = Vec::new();
    let mut exs = Vec::new();
    for part in gloss.split("; ") {
        let p = part.trim();
        match p.starts_with('"') {
            true => exs.push(p.trim_matches('"').to_string()),
            false => def.push(p.to_string()),
        }
    }
    (def.join("; "), exs)
}

// ───────────────────────────── Albanet / OMW ─────────────────────────────

pub fn import_omw_tab(g: &mut Graph, text: &str, rep: &mut ImportReport) -> Result<usize, String> {
    let mut lemmas = 0usize;
    let mut defs = 0usize;
    let mut exes = 0usize;
    let mut missing_synsets = 0usize;
    let mut reclassified = 0usize;
    for line in text.lines() {
        match line.starts_with('#') || line.trim().is_empty() {
            true => continue,
            false => {}
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let synset = match cols.first().and_then(|s| normalize_synset(s)) {
            Some(s) => s,
            None => continue,
        };
        let key = sense_key(Source::Pwn30, &synset);
        let sid = match g.get(&key) {
            Some(id) => id,
            None => {
                // OMW përmend një synset që mungon në PWN 3.0 — raportohet, nuk shpiket.
                missing_synsets += 1;
                continue;
            }
        };
        match (cols.get(1).copied(), cols.len()) {
            (Some("als:lemma"), 3) => {
                let lemma = cols[2].trim();
                match lemma.is_empty() {
                    true => continue,
                    false => {}
                }
                let w = g.ensure_word(Lang::Sq, &lemma.to_lowercase());
                g.add_label(sid, Text { lang: Lang::Sq, text: lemma.to_string(), source: Source::OmwAls });
                g.add_edge(Edge {
                    from: w,
                    to: sid,
                    rel: Rel::HasSense,
                    source: Source::OmwAls,
                    lang: Some(Lang::Sq),
                    method: Method::OmwOffsetMap,
                    attrs: vec![],
                });
                lemmas += 1;
            }
            (Some("als:def"), 4) => {
                let text = cols[3].trim().to_string();
                let lang = detect_sq_en(&text);
                match lang {
                    Lang::En => reclassified += 1,
                    Lang::Sq => {}
                }
                g.nodes[sid as usize].glosses.push(Text { lang, text, source: Source::OmwAls });
                defs += 1;
            }
            (Some("als:exe"), 4) => {
                let text = cols[3].trim().to_string();
                let lang = detect_sq_en(&text);
                match lang {
                    Lang::En => reclassified += 1,
                    Lang::Sq => {}
                }
                g.nodes[sid as usize].examples.push(Text { lang, text, source: Source::OmwAls });
                exes += 1;
            }
            _ => {}
        }
    }
    rep.note(format!(
        "omw_als: {lemmas} lidhje lemë→kuptim, {defs} përkufizime, {exes} shembuj; {missing_synsets} rreshta me synset që mungon në PWN 3.0"
    ));
    rep.note(format!(
        "omw_als: {reclassified} përkufizime/shembuj të shënuar si shqip në burim u gjetën në anglisht dhe u riklasifikuan lang=en (rregulli i deklaruar detect_sq_en)"
    ));
    Ok(lemmas)
}

/// Zbulim i thjeshtë dhe i deklaruar shqip/anglisht: numëron fjalë funksionale
/// dhe shkronjat ë/ç. Përdoret vetëm për të mos paraqitur anglishten si shqip.
pub fn detect_sq_en(text: &str) -> Lang {
    const EN: &[&str] = &["the", "of", "and", "a", "an", "to", "in", "with", "that", "is", "or", "for", "by", "as", "which", "any", "from", "on"];
    const SQ: &[&str] = &["një", "e", "i", "të", "në", "që", "me", "dhe", "nga", "për", "ose", "është", "së", "si", "apo", "ku"];
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic() && c != 'ë' && c != 'ç')
        .filter(|w| !w.is_empty())
        .collect();
    let en = words.iter().filter(|w| EN.contains(w)).count();
    let sq = words.iter().filter(|w| SQ.contains(w)).count() + lower.matches(['ë', 'ç']).count();
    match en > sq {
        true => Lang::En,
        false => Lang::Sq,
    }
}

// ───────────────────────────── ConceptNet ─────────────────────────────

pub fn import_conceptnet(g: &mut Graph, text: &str, rep: &mut ImportReport) -> Result<usize, String> {
    let mut n = 0usize;
    let mut rels = std::collections::BTreeMap::<String, usize>::new();
    let mut skipped = 0usize;
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (rel, a, b, json) = match (cols.get(1), cols.get(2), cols.get(3), cols.get(4)) {
            (Some(r), Some(a), Some(b), Some(j)) => (*r, *a, *b, *j),
            _ => continue,
        };
        // Vetëm skaje ndërmjet koncepteve (/c/…); /r/ExternalURL drejt faqeve web
        // nuk është marrëdhënie gjuhësore dhe numërohet si e përjashtuar.
        match (a.starts_with("/c/"), b.starts_with("/c/")) {
            (true, true) => {}
            _ => {
                skipped += 1;
                continue;
            }
        }
        let ca = concept_node(g, a);
        let cb = concept_node(g, b);
        let mut attrs = vec![];
        match json_str(json, "dataset") {
            Some(d) => attrs.push(("dataset".to_string(), d)),
            None => {}
        }
        match json_str(json, "license") {
            Some(l) => attrs.push(("license".to_string(), l)),
            None => {}
        }
        match json_num(json, "weight") {
            Some(w) => attrs.push(("weight".to_string(), w)),
            None => {}
        }
        g.add_edge(Edge {
            from: ca,
            to: cb,
            rel: Rel::Cn(rel.to_string()),
            source: Source::ConceptNet57,
            lang: lang_of_uri(a).or_else(|| lang_of_uri(b)),
            method: Method::Published,
            attrs,
        });
        *rels.entry(rel.to_string()).or_default() += 1;
        n += 1;
    }
    rep.note(format!("conceptnet57: {n} skaje me /c/sq/ të importuara ({:?}); {skipped} skaje drejt URL-ve të jashtme (/r/ExternalURL) u përjashtuan", rels));
    Ok(n)
}

fn lang_of_uri(uri: &str) -> Option<Lang> {
    uri.split('/').nth(2).and_then(Lang::parse)
}

fn concept_node(g: &mut Graph, uri: &str) -> u32 {
    let id = g.ensure(concept_key(uri), NodeKind::Concept { uri: uri.to_string() });
    let term = concept_term(uri);
    let lang = lang_of_uri(uri);
    match lang {
        Some(l) => {
            g.add_label(id, Text { lang: l, text: term.clone(), source: Source::ConceptNet57 });
            // Lidhja koncept → fjalë: vetëm me fjalën (formën), kurrë me kuptim.
            let w = g.ensure_word(l, &term.to_lowercase());
            let mut attrs = vec![];
            match concept_pos(uri) {
                Some(p) => attrs.push(("pos".to_string(), p.code().to_string())),
                None => {}
            }
            g.add_edge(Edge {
                from: id,
                to: w,
                rel: Rel::FormOf,
                source: Source::ConceptNet57,
                lang: Some(l),
                method: Method::LemmaStringMatch,
                attrs,
            });
        }
        None => {}
    }
    id
}

fn json_str(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\": \"");
    let s = json.find(&pat)? + pat.len();
    let e = json[s..].find('"')? + s;
    Some(json[s..e].to_string())
}

fn json_num(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\": ");
    let s = json.find(&pat)? + pat.len();
    let v: String = json[s..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    match v.is_empty() {
        true => None,
        false => Some(v),
    }
}

// ───────────────────────────── BabelNet 5.3 ─────────────────────────────

fn bn_pos(p: &str, id: &str) -> Option<Pos> {
    match p {
        "NOUN" => Some(Pos::Noun),
        "VERB" => Some(Pos::Verb),
        "ADJ" => Some(Pos::Adj),
        "ADV" => Some(Pos::Adv),
        _ => id.chars().last().map(|c| c.to_string()).and_then(|c| Pos::from_wn(&c)),
    }
}

fn bn_sense(g: &mut Graph, id: &str, pos: Option<Pos>) -> Option<u32> {
    let pos = pos.or_else(|| bn_pos("", id))?;
    Some(g.ensure(
        sense_key(Source::BabelNet53, id),
        NodeKind::Sense { source: Source::BabelNet53, synset: id.to_string(), pos },
    ))
}

/// Rregulla shtesë heqjeje VETËM për titujt e Wikipedia/Wikidata, pas `strip_candidates`
/// dhe gjithmonë të verifikuara me fjalorin: «-ja → ∅» (gruaja → grua, arsyeja → arsye),
/// «-ja → -je» (mendja → mendje), «-ra → -ër» (femra → femër), «-ri → ∅» (syri → sy),
/// «-ët → ∅» (lotët → lot).
const TITLE_RULES: &[(&str, &str)] = &[("ja", ""), ("ja", "je"), ("ra", "ër"), ("ri", ""), ("ët", "")];

/// Lemat e mundshme të një titulli (me shkronja të vogla), sipas renditjes: së pari
/// rregullat e `morph::strip_candidates`, pastaj `TITLE_RULES`, pastaj «-ra → -r»
/// (flutura → flutur) dhe «-të → ∅» pas -i/-y (sytë → sy, bashkësitë → bashkësi).
fn title_candidates(t: &str) -> Vec<String> {
    let mut out = crate::morph::strip_candidates(t);
    for &(suf, repl) in TITLE_RULES {
        match t.strip_suffix(suf) {
            Some(stem) => match stem.chars().count() >= 2 {
                true => out.push(format!("{stem}{repl}")),
                false => {}
            },
            None => {}
        }
    }
    match t.strip_suffix("ra") {
        Some(stem) => match stem.chars().count() >= 2 {
            true => out.push(format!("{stem}r")),
            false => {}
        },
        None => {}
    }
    match t.strip_suffix("të") {
        Some(stem) => match (stem.chars().count() >= 2, stem.chars().last()) {
            (true, Some('i' | 'y')) => out.push(stem.to_string()),
            _ => {}
        },
        None => {}
    }
    out
}

/// Mbaresë tipike e trajtës së shquar emërore (-a, -i, -u njëjës; -ët shumës).
fn looks_definite(t: &str) -> bool {
    ["a", "i", "u", "ët"]
        .iter()
        .any(|&s| t.strip_suffix(s).map(|stem| stem.chars().count() >= 2).unwrap_or(false))
}

/// A njihet vetë titulli si lemë nga një burim leksikografik? Fjala ka kuptim (AlbaNet,
/// BabelNet), është lemë e një shqise jo-Wikipedia të këtij skedari, ose ConceptNet e
/// njeh nga një burim jashtë DBpedia (DBpedia = vetë titujt e Wikipedia-s, si «gruaja»).
fn lexicographic_lemma(g: &Graph, known: &std::collections::HashSet<String>, t: &str) -> bool {
    match (known.contains(t), g.get(&word_key(Lang::Sq, t))) {
        (true, _) => true,
        (false, None) => false,
        (false, Some(w)) => {
            let non_dbpedia = |c: u32| {
                g.out_edges(c).chain(g.in_edges(c)).any(|(_, x)| {
                    matches!(x.rel, Rel::Cn(_)) && x.attr("dataset").map(|d| !d.starts_with("/d/dbpedia")).unwrap_or(false)
                })
            };
            !g.senses_of(w).is_empty()
                || g.in_edges(w).any(|(_, e)| e.rel == Rel::FormOf && e.source == Source::ConceptNet57 && non_dbpedia(e.from))
        }
    }
}

/// Titulli Wikipedia/Wikidata me një fjalë → lema e cituar. Rregull i deklaruar:
/// - kandidati i parë nga `title_candidates` që ekziston si fjalë shqipe (ose si lemë
///   e një shqise jo-Wikipedia të skedarit) → ajo lemë («Ushqimi» → «ushqim»);
/// - pa kandidat: titulli që s'duket i shquar mbetet («fukadrugë»), po ashtu titulli që
///   vetë njihet si lemë («dukuri»);
/// - `None`: trajtë e shquar pa lemë të njohur («Didaktika») → hidhet (as kuptim, as etiketë).
fn title_lemma(g: &Graph, known: &std::collections::HashSet<String>, title: &str) -> Option<String> {
    let exists = |w: &str| g.get(&word_key(Lang::Sq, w)).is_some() || known.contains(w);
    match title_candidates(title).into_iter().find(|c| exists(c.as_str())) {
        Some(l) => Some(l),
        None => match (looks_definite(title), lexicographic_lemma(g, known, title)) {
            (true, false) => None,
            _ => Some(title.to_string()),
        },
    }
}

/// Etiketë shqipe pa dyfishim sipas tekstit (pa marrë parasysh burimin dhe shkronjat
/// e mëdha): «Vendbanimi» → «vendbanim» nuk shtohet kur AlbaNet e ka «vendbanim».
fn add_sq_label(g: &mut Graph, sid: u32, text: String) {
    let low = text.to_lowercase();
    let dup = g.node(sid).labels.iter().any(|t| t.lang == Lang::Sq && t.text.to_lowercase() == low);
    match dup {
        true => {}
        false => g.add_label(sid, Text { lang: Lang::Sq, text, source: Source::BabelNet53 }),
    }
}

/// Importon `babelnet-sq.tsv` (nxjerrja lokale e `nura_fetch --babelnet`).
/// Rregullat e deklaruara:
/// - kuptim i ri për një fjalë shqipe vetëm nga shqisat `HIGH_QUALITY` me burim
///   jashtë `OMWN_SQ` (AlbaNet është importuar drejtpërdrejt; s'dyfishohet);
/// - `AUTOMATIC_TRANSLATION` dhe `POTENTIAL_NEAR_SYNONYM_OR_WORSE` nuk hyjnë në
///   graf (mbeten vetëm në cache, të numëruara në raport);
/// - lidhjet: vetëm HYPERNYM, HOLONYM, MERONYM, ANTONYM;
/// - titujt Wikipedia/Wikidata me një fjalë (S dhe L) → lema e cituar (`title_lemma`),
///   si fjalë edhe si etiketë; trajta e shquar pa lemë të njohur nuk hyn në graf.
/// Shpeshtësia e kuptimeve sipas `index.sense` të WordNet 3.0: për çdo synset, shuma e numrave
/// të etiketimit (tag_cnt, nga korpusi SemCor) dhe rendi më i mirë i kuptimit te lemat e tij.
/// Ruhet si vetë-lak `Rel::Sem("SENSE_FREQ")` me atributet `tag` dhe `rank`; shërben vetëm për
/// të gjetur kuptimin më të zakonshëm, kurrë si fakt që thuhet. Po aty ruhen edhe klasa
/// leksikografike e synset-it (`lex`, numri i skedarit `lexnames` të WordNet-it: 05 kafshë,
/// 06 send i bërë, 27 lëndë …) dhe rendi i kuptimit për secilën lemë angleze
/// (`en` = «lema=rendi;…», p.sh. te synset-i i veturës «car=1;machine=6;…»).
pub fn pwn_sense_frequency(g: &mut Graph, text: &str, rep: &mut ImportReport) {
    // synset → (shuma e etiketimeve, rendi më i mirë, klasa leksikografike, lemë → rendi)
    type Row = (u32, u32, String, std::collections::BTreeMap<String, u32>);
    let mut freq: std::collections::BTreeMap<String, Row> = std::collections::BTreeMap::new();
    for l in text.lines() {
        let c: Vec<&str> = l.split(' ').collect();
        let parsed = match c.as_slice() {
            [key, off, rank, tag, ..] => {
                let (lemma, rest) = key.split_once('%').unwrap_or((key, ""));
                let fields: Vec<&str> = rest.split(':').collect();
                let pos = match fields.first().and_then(|f| f.chars().next()) {
                    Some('1') => Some("n"),
                    Some('2') => Some("v"),
                    Some('3' | '5') => Some("a"),
                    Some('4') => Some("r"),
                    _ => None,
                };
                let lexfile = fields.get(1).copied().unwrap_or("").to_string();
                match (pos.and_then(|p| normalize_synset(&format!("{off}-{p}"))), rank.parse::<u32>(), tag.parse::<u32>()) {
                    (Some(s), Ok(r), Ok(t)) => Some((s, r, t, lexfile, lemma.replace('_', " ").to_lowercase())),
                    _ => None,
                }
            }
            _ => None,
        };
        match parsed {
            Some((s, r, t, lexfile, lemma)) => {
                let e = freq.entry(s).or_insert((0, u32::MAX, lexfile, std::collections::BTreeMap::new()));
                e.0 += t;
                e.1 = e.1.min(r);
                let n = e.3.entry(lemma).or_insert(r);
                *n = (*n).min(r);
            }
            None => {}
        }
    }
    let mut n = 0u32;
    for (synset, (tag, rank, lexfile, lemmas)) in freq {
        let en: Vec<String> = lemmas.iter().map(|(l, r)| format!("{l}={r}")).collect();
        match g.get(&sense_key(Source::Pwn30, &synset)) {
            Some(sid) => {
                g.add_edge(Edge {
                    from: sid,
                    to: sid,
                    rel: Rel::Sem("SENSE_FREQ".into()),
                    source: Source::Pwn30,
                    lang: Some(Lang::En),
                    method: Method::Published,
                    attrs: vec![
                        ("tag".into(), tag.to_string()),
                        ("rank".into(), rank.to_string()),
                        ("lex".into(), lexfile),
                        ("en".into(), en.join(";")),
                    ],
                });
                n += 1;
            }
            None => {}
        }
    }
    rep.note(format!("pwn30: shpeshtësia e {n} synset-eve nga index.sense (SemCor tag_cnt)"));
}

/// `index.sense` i WordNet 3.0: senseKey → synset ("offset-pos").
pub fn parse_index_sense(text: &str) -> std::collections::HashMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let mut it = l.split(' ');
            let key = it.next()?;
            let off = it.next()?;
            let ss = key.split_once('%')?.1.chars().next()?;
            let pos = match ss {
                '1' => "n",
                '2' => "v",
                '3' | '5' => "a",
                '4' => "r",
                _ => return None,
            };
            Some((key.to_lowercase(), format!("{off}-{pos}")))
        })
        .collect()
}



/// Fjalia kryesore e një përkufizimi enciklopedik: pjesa para presjes së parë, kur ka të
/// paktën 3 fjalë («Macja është kafshë shtëpiake»); ndryshe fjalia e parë e plotë.
fn wiki_first_clause(t: &str) -> String {
    let first_sentence = t.split(". ").next().unwrap_or(t).trim().trim_end_matches('.');
    let clause = first_sentence.split(", ").next().unwrap_or(first_sentence).trim();
    match clause.split_whitespace().count() >= 3 {
        true => format!("{clause}."),
        false => format!("{first_sentence}."),
    }
}

pub fn import_babelnet(
    g: &mut Graph,
    text: &str,
    wn_keys: &std::collections::HashMap<String, String>,
    rep: &mut ImportReport,
) -> usize {
    let (mut queried, mut senses, mut dup, mut auto, mut weak, mut glosses, mut edges) = (0, 0, 0, 0, 0, 0, 0);
    let mut named = 0;
    // Titujt Wikipedia/Wikidata: të kthyer në lemë, dhe të hedhur (trajtë e shquar pa lemë të njohur).
    let (mut titles_fixed, mut def_senses, mut def_labels) = (0, 0, 0);
    // Lemat e shqisave jo-Wikipedia të këtij skedari (Wiktionary, OMW, AlbaNet …): titujt
    // krahasohen edhe me to, pavarësisht renditjes së rreshtave.
    let known: std::collections::HashSet<String> = text
        .lines()
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            match (c.first().copied(), c.len(), c.get(4).copied(), c.get(5).copied()) {
                (Some("S"), 6, Some("WIKI" | "WIKIDATA" | "GEONM"), _) => None,
                (Some("S"), 6, Some(_), Some("HIGH_QUALITY")) => Some(c[3].replace('_', " ").to_lowercase()),
                _ => None,
            }
        })
        .collect();
    // Kalimi i parë: lloji i synset-it. Vetëm konceptet bëhen kuptime fjalësh;
    // entitetet me emër (filma, mbiemra, vende) dhe synset-et e pashkarkuara jo.
    let concepts: std::collections::HashSet<String> = text
        .lines()
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            match (c.first().copied(), c.get(2).copied()) {
                (Some("T"), Some("CONCEPT")) => c.get(1).map(|x| x.to_string()),
                _ => None,
            }
        })
        .collect();
    // Kalimi i dytë: synset BabelNet → synset WordNet 3.0 (njëvlershmëri e deklaruar).
    let mut to_wn: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for l in text.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        match (c.first().copied(), c.get(1), c.get(2)) {
            (Some("K"), Some(id), Some(k)) => match (to_wn.contains_key(*id), wn_keys.get(&k.to_lowercase())) {
                (false, Some(syn)) => match g.get(&sense_key(Source::Pwn30, syn)) {
                    Some(sid) => {
                        to_wn.insert(id.to_string(), sid);
                    }
                    None => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
    let mut mapped = 0;
    let target = |g: &mut Graph, id: &str, pos: Option<Pos>| -> Option<u32> {
        match to_wn.get(id) {
            Some(sid) => Some(*sid),
            None => bn_sense(g, id, pos),
        }
    };
    for line in text.lines() {
        let c: Vec<&str> = line.split('\t').collect();
        match (c.first().copied(), c.len()) {
            (Some("Q"), 2) => queried += 1,
            (Some("S"), 6) => match (c[5], c[4], concepts.contains(c[1]) && !c[3].contains('(')) {
                ("HIGH_QUALITY", "OMWN_SQ", _) => dup += 1,
                ("HIGH_QUALITY", _, false) => named += 1,
                ("HIGH_QUALITY", src, true) => {
                    let raw = c[3].replace('_', " ");
                    let title = raw.to_lowercase();
                    // Titujt e Wikipedia/Wikidata shqip janë zakonisht në trajtën e shquar
                    // («Macja», «Flutura»). Rregull i deklaruar (`title_lemma`): kthehen në lemën
                    // e cituar që njihet në fjalor; kjo lemë bëhet edhe etiketa (sinonimi) e
                    // kuptimit. Trajta e shquar pa lemë të njohur hidhet (as kuptim, as etiketë).
                    // (lema, etiketa, titull i zgjidhur?, lema ≠ titulli?)
                    let (lemma, label, titled, changed) = match (src, title.contains(' ')) {
                        ("WIKI" | "WIKIDATA", false) => match title_lemma(g, &known, &title) {
                            Some(l) => {
                                let changed = l != title;
                                (l.clone(), l, true, changed)
                            }
                            None => {
                                def_senses += 1;
                                continue;
                            }
                        },
                        _ => (title, raw.clone(), false, false),
                    };
                    match (lemma.is_empty(), target(g, c[1], bn_pos(c[2], c[1]))) {
                        (false, Some(sid)) => {
                            let w = g.ensure_word(Lang::Sq, &lemma);
                            // E njëjta fjalë → i njëjti kuptim nga një burim tjetër: nuk dyfishohet.
                            let already = g.senses_of(w).iter().any(|(s, _)| *s == sid);
                            match already {
                                true => {
                                    dup += 1;
                                    continue;
                                }
                                false => {}
                            }
                            let is_mapped = to_wn.contains_key(c[1]);
                            match is_mapped {
                                true => mapped += 1,
                                false => {}
                            }
                            let mut attrs = vec![("bn_source".to_string(), src.to_string()), ("bn_id".to_string(), c[1].to_string())];
                            match changed {
                                // Titulli origjinal ruhet si provë kur lema ndryshon («Ushqimi» → «ushqim»).
                                true => {
                                    titles_fixed += 1;
                                    attrs.push(("bn_title".to_string(), raw.clone()));
                                }
                                false => {}
                            }
                            match titled {
                                true => add_sq_label(g, sid, label),
                                false => g.add_label(sid, Text { lang: Lang::Sq, text: label, source: Source::BabelNet53 }),
                            }
                            g.add_edge(Edge {
                                from: w,
                                to: sid,
                                rel: Rel::HasSense,
                                source: Source::BabelNet53,
                                lang: Some(Lang::Sq),
                                method: match is_mapped {
                                    true => Method::BnSenseKeyMap,
                                    false => Method::Published,
                                },
                                attrs,
                            });
                            senses += 1;
                        }
                        _ => {}
                    }
                }
                ("AUTOMATIC_TRANSLATION", _, _) => auto += 1,
                _ => weak += 1,
            },
            (Some("L"), 5) => {
                let node = match to_wn.get(c[1]) {
                    Some(sid) => Some(*sid),
                    None => g.get(&sense_key(Source::BabelNet53, c[1])),
                };
                let lang = Lang::parse(&c[2].to_lowercase());
                let raw = c[3].replace('_', " ");
                let title = raw.to_lowercase();
                // Etiketa shqipe nga titulli Wikipedia/Wikidata (një fjalë) i një koncepti:
                // e njëjta zgjidhje si te rreshtat S — lema e cituar, ose asgjë.
                let titled = matches!(
                    (lang, c[4], concepts.contains(c[1]), title.contains(' ')),
                    (Some(Lang::Sq), "WIKI" | "WIKIDATA", true, false)
                );
                // Te synset-et e WordNet shtohen vetëm etiketat shqipe (ato angleze ekzistojnë).
                match (lang, node, to_wn.contains_key(c[1]), titled) {
                    (Some(Lang::Sq), Some(sid), _, true) => match title_lemma(g, &known, &title) {
                        Some(l) => {
                            match l == title {
                                true => {}
                                false => titles_fixed += 1,
                            }
                            add_sq_label(g, sid, l)
                        }
                        None => def_labels += 1,
                    },
                    (Some(Lang::Sq), Some(sid), _, false) | (Some(Lang::En), Some(sid), false, _) => {
                        g.add_label(sid, Text { lang: lang.unwrap_or(Lang::En), text: raw, source: Source::BabelNet53 })
                    }
                    _ => {}
                }
            }
            (Some("G"), 5) => match (Lang::parse(&c[2].to_lowercase()), target(g, c[1], None)) {
                (Some(l), Some(sid)) => {
                    // Nga Wikipedia shqip merret vetëm fjalia kryesore e përkufizimit (deri te presja
                    // e parë): shtesat enciklopedike sjellin pohime të paverifikuara
                    // («Macja është kafshë shtëpiake, bën pjesë te Insektët»).
                    let text = match (l, c[3]) {
                        (Lang::Sq, "WIKI" | "WIKIDATA") => wiki_first_clause(c[4]),
                        _ => c[4].to_string(),
                    };
                    let t = Text { lang: l, text, source: Source::BabelNet53 };
                    let n = &mut g.nodes[sid as usize];
                    match n.glosses.contains(&t) {
                        true => {}
                        false => {
                            n.glosses.push(t);
                            glosses += 1;
                        }
                    }
                }
                _ => {}
            },
            (Some("E"), 6) => match c[2] {
                // Synset-et e lidhura me WordNet përdorin hierarkinë e WordNet; lidhjet BabelNet vetëm për të tjerët.
                "HYPERNYM" | "HOLONYM" | "MERONYM" | "ANTONYM" => match (to_wn.contains_key(c[1]), target(g, c[1], None), target(g, c[4], None)) {
                    (false, Some(a), Some(b)) => {
                        g.add_edge(Edge {
                            from: a,
                            to: b,
                            rel: Rel::Bn(c[2].to_string()),
                            source: Source::BabelNet53,
                            lang: Lang::parse(&c[5].to_lowercase()),
                            method: Method::Published,
                            attrs: vec![("name".into(), c[3].to_string())],
                        });
                        edges += 1;
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
    rep.note(format!(
        "babelnet53: {queried} fjalë të pyetura; {senses} kuptime shqipe të reja (HIGH_QUALITY, jashtë AlbaNet); {dup} shqisa OMWN_SQ të anashkaluara si dublikatë e AlbaNet; {auto} përkthime automatike, {weak} sinonime të dobëta dhe {named} entitete me emër / synset-e pa koncept të papërdorura; {glosses} përkufizime; {edges} lidhje; {mapped} kuptime të lidhura me WordNet 3.0 përmes senseKey ({} synset-e të njëvlershme)", to_wn.len()
    ));
    rep.note(format!(
        "babelnet53: {titles_fixed} tituj Wikipedia/Wikidata në trajtë të shquar u kthyen në lemën e cituar (kuptim dhe etiketë); u hodhën {def_senses} kuptime dhe {def_labels} etiketa nga tituj në trajtë të shquar pa lemë të njohur në fjalor"
    ));
    senses
}

// ───────────────────────────── CoNLL-U ─────────────────────────────

#[derive(Debug, Clone)]
pub struct UdToken {
    pub id: usize,
    pub form: String,
    pub lemma: String,
    pub upos: String,
    pub feats: String,
    pub head: usize,
    pub deprel: String,
}

#[derive(Debug, Clone)]
pub struct UdSentence {
    pub sent_id: String,
    pub text: String,
    pub tokens: Vec<UdToken>,
}

/// Analizues CoNLL-U: injoron rreshtat me shumë fjalë ("1-2") dhe nyjet bosh ("1.1").
pub fn parse_conllu(text: &str) -> Result<Vec<UdSentence>, String> {
    let mut out = Vec::new();
    let mut cur = UdSentence { sent_id: String::new(), text: String::new(), tokens: Vec::new() };
    for (ln, line) in text.lines().enumerate() {
        let l = line.trim_end_matches('\r');
        match (l.is_empty(), l.starts_with('#')) {
            (true, _) => {
                match cur.tokens.is_empty() {
                    true => {}
                    false => out.push(std::mem::replace(
                        &mut cur,
                        UdSentence { sent_id: String::new(), text: String::new(), tokens: Vec::new() },
                    )),
                }
            }
            (false, true) => {
                match (l.strip_prefix("# sent_id = "), l.strip_prefix("# text = ")) {
                    (Some(id), _) => cur.sent_id = id.trim().to_string(),
                    (_, Some(t)) => cur.text = t.trim().to_string(),
                    _ => {}
                }
            }
            (false, false) => {
                let c: Vec<&str> = l.split('\t').collect();
                match c.len() == 10 {
                    true => {}
                    false => return Err(format!("CoNLL-U rreshti {} ka {} kolona", ln + 1, c.len())),
                }
                match c[0].contains('-') || c[0].contains('.') {
                    true => continue,
                    false => {}
                }
                let id: usize = c[0].parse().map_err(|_| format!("ID e pavlefshme në rreshtin {}", ln + 1))?;
                let head: usize = c[6].parse().unwrap_or(0);
                cur.tokens.push(UdToken {
                    id,
                    form: c[1].to_string(),
                    lemma: c[2].to_string(),
                    upos: c[3].to_string(),
                    feats: c[5].to_string(),
                    head,
                    deprel: c[7].to_string(),
                });
            }
        }
    }
    match cur.tokens.is_empty() {
        true => {}
        false => out.push(cur),
    }
    Ok(out)
}

pub fn import_ud(
    g: &mut Graph,
    morph: &mut Morph,
    source: Source,
    text: &str,
    rep: &mut ImportReport,
) -> Result<usize, String> {
    let sents = parse_conllu(text)?;
    let mut occ = 0usize;
    let mut deps = 0usize;
    for s in &sents {
        let sk = sentence_key(source, &s.sent_id);
        let sid = g.ensure(sk, NodeKind::Sentence { source, sent_id: s.sent_id.clone() });
        g.nodes[sid as usize].glosses.push(Text { lang: Lang::Sq, text: s.text.clone(), source });
        // fjalë përmbajtjeje
        let mut word_of: Vec<Option<u32>> = vec![None; s.tokens.len() + 1];
        for t in &s.tokens {
            morph.add(MorphEntry {
                lemma: t.lemma.to_lowercase(),
                form: t.form.clone(),
                upos: t.upos.clone(),
                feats: t.feats.clone(),
                sent_id: s.sent_id.clone(),
                source,
            });
            match Pos::from_upos(&t.upos) {
                Some(_) => {
                    let w = g.ensure_word(Lang::Sq, &t.lemma.to_lowercase());
                    word_of[t.id.min(s.tokens.len())] = Some(w);
                    g.add_edge(Edge {
                        from: w,
                        to: sid,
                        rel: Rel::OccursIn,
                        source,
                        lang: Some(Lang::Sq),
                        method: Method::UdAnnotation,
                        attrs: vec![
                            ("form".into(), t.form.clone()),
                            ("upos".into(), t.upos.clone()),
                            ("feats".into(), t.feats.clone()),
                            ("deprel".into(), t.deprel.clone()),
                            ("tok".into(), t.id.to_string()),
                        ],
                    });
                    occ += 1;
                }
                None => {}
            }
        }
        for t in &s.tokens {
            match (word_of.get(t.id).copied().flatten(), word_of.get(t.head).copied().flatten()) {
                (Some(dep), Some(head)) => {
                    g.add_edge(Edge {
                        from: dep,
                        to: head,
                        rel: Rel::DependsOn(t.deprel.clone()),
                        source,
                        lang: Some(Lang::Sq),
                        method: Method::UdAnnotation,
                        attrs: vec![("sent".into(), s.sent_id.clone())],
                    });
                    deps += 1;
                }
                _ => {}
            }
        }
    }
    rep.note(format!(
        "{}: {} fjali, {} përdorime fjalësh përmbajtjeje, {} varësi ndërmjet tyre",
        source.code(),
        sents.len(),
        occ,
        deps
    ));
    Ok(sents.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PWN: &str = "  1 This software and database is being provided to you, the LICENSEE, by  \n\
02084071 05 n 03 dog 0 domestic_dog 0 Canis_familiaris 0 002 @ 02083346 n 0000 ~ 01322604 n 0000 | a member of the genus Canis; \"the dog barked all night\"\n\
02083346 05 n 02 canine 0 canid 0 001 ~ 02084071 n 0000 | any of various fissiped mammals\n\
01322604 05 n 01 puppy 0 001 @ 02084071 n 0000 | a young dog\n";

    #[test]
    fn pwn_parser_reads_synsets_lemmas_pointers_and_examples() {
        let mut g = Graph::new();
        let mut r = ImportReport::default();
        assert_eq!(import_pwn_data(&mut g, PWN, &mut r).unwrap(), 3);
        let dog = g.get("s:pwn30:02084071-n").unwrap();
        assert_eq!(g.label_in(dog, Lang::En), Some("dog"));
        assert_eq!(g.node(dog).labels.len(), 3);
        assert_eq!(g.node(dog).examples[0].text, "the dog barked all night");
        let hyper: Vec<_> = g.out_edges(dog).filter(|(_, e)| e.rel == Rel::Wn("@".into())).collect();
        assert_eq!(hyper.len(), 1);
        assert_eq!(g.node(hyper[0].1.to).key, "s:pwn30:02083346-n");
    }

    #[test]
    fn omw_links_albanian_lemma_to_existing_sense_and_reports_missing() {
        let mut g = Graph::new();
        let mut r = ImportReport::default();
        import_pwn_data(&mut g, PWN, &mut r).unwrap();
        let tab = "# Albanet\tals\thttp://fjalnet.com/\tCC BY 3.0\n\
02083346-n\tals:lemma\tqen\n\
02083346-n\tals:def\t0\tkafshë mishngrënëse\n\
99999999-n\tals:lemma\tsajesë\n";
        import_omw_tab(&mut g, tab, &mut r).unwrap();
        let qen = g.get("w:sq:qen").unwrap();
        let senses = g.senses_of(qen);
        assert_eq!(senses.len(), 1);
        assert_eq!(senses[0].1.method, Method::OmwOffsetMap);
        // synset-i i panjohur nuk krijon nyje të shpikur
        assert!(g.get("w:sq:sajesë").is_none());
        assert!(r.lines.iter().any(|l| l.contains("1 rreshta me synset që mungon")));
    }

    #[test]
    fn conceptnet_concepts_link_to_words_not_senses() {
        let mut g = Graph::new();
        let mut r = ImportReport::default();
        let csv = "/a/[/r/Antonym/,/c/sq/bardhë/a/,/c/sq/zi/]\t/r/Antonym\t/c/sq/bardhë/a\t/c/sq/zi\t{\"dataset\": \"/d/wiktionary/en\", \"license\": \"cc:by-sa/4.0\", \"weight\": 1.0}\n";
        import_conceptnet(&mut g, csv, &mut r).unwrap();
        let c = g.get("c:/c/sq/bardhë/a").unwrap();
        let form: Vec<_> = g.out_edges(c).filter(|(_, e)| e.rel == Rel::FormOf).collect();
        assert_eq!(form.len(), 1);
        assert_eq!(form[0].1.method, Method::LemmaStringMatch);
        assert!(matches!(g.node(form[0].1.to).kind, NodeKind::Word { .. }));
        let ant: Vec<_> = g.out_edges(c).filter(|(_, e)| e.rel == Rel::Cn("/r/Antonym".into())).collect();
        assert_eq!(ant[0].1.attr("license"), Some("cc:by-sa/4.0"));
        assert_eq!(ant[0].1.attr("dataset"), Some("/d/wiktionary/en"));
    }

    #[test]
    fn conllu_parser_handles_comment_order_and_multiword_lines() {
        let t = "# text = Qeni leh.\n# sent_id = T1\n1-2\tQeni\t_\t_\t_\t_\t_\t_\t_\t_\n1\tQeni\tqen\tNOUN\t_\tCase=Nom|Definite=Def|Gender=Masc|Number=Sing\t2\tnsubj\t_\t_\n2\tleh\tleh\tVERB\t_\t_\t0\troot\t_\t_\n3\t.\t.\tPUNCT\t_\t_\t2\tpunct\t_\t_\n\n";
        let s = parse_conllu(t).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].sent_id, "T1");
        assert_eq!(s[0].text, "Qeni leh.");
        assert_eq!(s[0].tokens.len(), 3);
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        import_ud(&mut g, &mut m, Source::UdSqTsa, t, &mut r).unwrap();
        let qen = g.get("w:sq:qen").unwrap();
        let dep: Vec<_> = g.out_edges(qen).filter(|(_, e)| matches!(e.rel, Rel::DependsOn(_))).collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].1.rel, Rel::DependsOn("nsubj".into()));
        assert_eq!(dep[0].1.attr("sent"), Some("T1"));
    }

    #[test]
    fn english_text_labelled_albanian_is_detected() {
        assert_eq!(detect_sq_en("any of various fissiped mammals with nonretractile claws and typically long muzzles"), Lang::En);
        assert_eq!(detect_sq_en("një ngjarje ku diçka digjet"), Lang::Sq);
        assert_eq!(detect_sq_en("organizëm i gjallë i cili nuk mund të lëviz"), Lang::Sq);
    }

    #[test]
    fn conllu_rejects_malformed_rows() {
        assert!(parse_conllu("1\tbad\n").is_err());
    }

    fn sq_labels(g: &Graph, id: u32) -> Vec<String> {
        g.node(id).labels.iter().filter(|t| t.lang == Lang::Sq).map(|t| t.text.clone()).collect()
    }

    /// Rreshta në formatin e `babelnet-sq.tsv` (titujt realë: Flutura, Ushqimi, Vendbanimi …).
    const BN: &str = "# BabelNet 5.3 test\n\
T\tbn:00014271n\tCONCEPT\n\
K\tbn:00014271n\tbutterfly%1:05:00::\n\
S\tbn:00014271n\tNOUN\tFlutura\tWIKI\tHIGH_QUALITY\n\
S\tbn:00014271n\tNOUN\tfukadrugë\tWIKT\tHIGH_QUALITY\n\
L\tbn:00014271n\tSQ\tFlutura\tWIKI\n\
T\tbn:00035650n\tCONCEPT\n\
K\tbn:00035650n\tfood%1:13:00::\n\
S\tbn:00035650n\tNOUN\tUshqimi\tWIKI\tHIGH_QUALITY\n\
T\tbn:00070724n\tCONCEPT\n\
K\tbn:00070724n\tvillage%1:15:00::\n\
L\tbn:00070724n\tSQ\tVendbanimi\tWIKI\n\
T\tbn:00027000n\tCONCEPT\n\
K\tbn:00027000n\tdidactics%1:09:00::\n\
S\tbn:00027000n\tNOUN\tDidaktika\tWIKI\tHIGH_QUALITY\n\
L\tbn:00027000n\tSQ\tDidaktika\tWIKI\n\
T\tbn:00081000n\tCONCEPT\n\
K\tbn:00081000n\twoman%1:18:00::\n\
S\tbn:00081000n\tNOUN\tGruaja\tWIKI\tHIGH_QUALITY\n\
T\tbn:00012000n\tCONCEPT\n\
K\tbn:00012000n\tbotany%1:09:00::\n\
S\tbn:00012000n\tNOUN\tBotanika\tWIKI\tHIGH_QUALITY\n\
T\tbn:00041000n\tCONCEPT\n\
K\tbn:00041000n\tguild%1:14:00::\n\
S\tbn:00041000n\tNOUN\tEsnafi\tWIKI\tHIGH_QUALITY\n\
L\tbn:00041000n\tSQ\tGilda_e_tregtarëve\tWIKI\n";

    #[test]
    fn babelnet_definite_titles_become_citation_lemmas_or_are_dropped() {
        let mut g = Graph::new();
        let mut r = ImportReport::default();
        let syns = ["02274259-n", "07555863-n", "08672199-n", "05996646-n", "10787470-n", "06001021-n", "08053576-n"];
        for syn in syns {
            g.ensure(sense_key(Source::Pwn30, syn), NodeKind::Sense { source: Source::Pwn30, synset: syn.into(), pos: Pos::Noun });
        }
        for w in ["flutur", "ushqim", "vendbanim", "grua"] {
            g.ensure_word(Lang::Sq, w);
        }
        let sid = |g: &Graph, syn: &str| g.get(&sense_key(Source::Pwn30, syn)).unwrap();
        // AlbaNet e ka tashmë «vendbanim» si etiketë të fshatit.
        let v = sid(&g, "08672199-n");
        g.add_label(v, Text { lang: Lang::Sq, text: "vendbanim".into(), source: Source::OmwAls });
        // ConceptNet: «esnafi» vetëm nga DBpedia (titull), «botanika» edhe nga Wiktionary.
        let csv = "/a/1\t/r/Synonym\t/c/sq/esnafi\t/c/en/guild\t{\"dataset\": \"/d/dbpedia/en\", \"license\": \"cc:by-sa/4.0\", \"weight\": 0.5}\n\
/a/2\t/r/Synonym\t/c/en/botany/n\t/c/sq/botanika\t{\"dataset\": \"/d/wiktionary/en\", \"license\": \"cc:by-sa/4.0\", \"weight\": 1.0}\n";
        import_conceptnet(&mut g, csv, &mut r).unwrap();
        let keys: std::collections::HashMap<String, String> = [
            ("butterfly%1:05:00::", "02274259-n"),
            ("food%1:13:00::", "07555863-n"),
            ("village%1:15:00::", "08672199-n"),
            ("didactics%1:09:00::", "05996646-n"),
            ("woman%1:18:00::", "10787470-n"),
            ("botany%1:09:00::", "06001021-n"),
            ("guild%1:14:00::", "08053576-n"),
        ]
        .iter()
        .map(|(k, s)| (k.to_string(), s.to_string()))
        .collect();
        import_babelnet(&mut g, BN, &keys, &mut r);
        let word = |g: &Graph, l: &str| g.get(&word_key(Lang::Sq, l));
        let has = |g: &Graph, l: &str, syn: &str| match word(g, l) {
            Some(w) => g.senses_of(w).iter().any(|(s, _)| *s == sid(g, syn)),
            None => false,
        };
        // «Flutura» → «flutur» (-ra → -r): lema dhe etiketa; titulli ruhet si `bn_title`.
        assert!(has(&g, "flutur", "02274259-n"));
        assert!(word(&g, "flutura").is_none());
        assert_eq!(sq_labels(&g, sid(&g, "02274259-n")), vec!["flutur".to_string(), "fukadrugë".to_string()]);
        let fl = word(&g, "flutur").unwrap();
        assert_eq!(g.senses_of(fl)[0].1.attr("bn_title"), Some("Flutura"));
        // «Ushqimi» → «ushqim»: etiketa nuk është më trajta e shquar.
        assert!(has(&g, "ushqim", "07555863-n"));
        assert_eq!(sq_labels(&g, sid(&g, "07555863-n")), vec!["ushqim".to_string()]);
        // L «Vendbanimi» → «vendbanim», që AlbaNet e ka: pa dyfishim.
        assert_eq!(sq_labels(&g, v), vec!["vendbanim".to_string()]);
        // «Gruaja» → «grua» (-ja → ∅).
        assert!(has(&g, "grua", "10787470-n"));
        // «Didaktika»: trajtë e shquar pa lemë të njohur → as fjalë, as kuptim, as etiketë.
        assert!(word(&g, "didaktika").is_none());
        assert!(sq_labels(&g, sid(&g, "05996646-n")).is_empty());
        // «Esnafi»: ConceptNet e njeh vetëm nga DBpedia (titull) → hidhet.
        assert!(!has(&g, "esnafi", "08053576-n"));
        // «Botanika»: Wiktionary e njeh si fjalë → mbetet.
        assert!(has(&g, "botanika", "06001021-n"));
        // Tituj me disa fjalë: sjellja e mëparshme.
        assert_eq!(sq_labels(&g, sid(&g, "08053576-n")), vec!["Gilda e tregtarëve".to_string()]);
        assert!(r.lines.iter().any(|l| l.contains("babelnet53: 5 tituj") && l.contains("u hodhën 2 kuptime dhe 1 etiketa")));
    }

    #[test]
    fn title_candidates_extend_strip_rules() {
        let c = title_candidates("flutura");
        assert!(c.contains(&"flutur".to_string()));
        assert!(c.contains(&"fluturë".to_string()));
        assert!(title_candidates("mendja").contains(&"mendje".to_string()));
        assert!(title_candidates("syri").contains(&"sy".to_string()));
        assert!(title_candidates("sytë").contains(&"sy".to_string()));
        assert!(title_candidates("lotët").contains(&"lot".to_string()));
        // «-të → ∅» vetëm pas -i/-y: «botë» nuk jep «bo».
        assert!(!title_candidates("botë").contains(&"bo".to_string()));
        assert!(looks_definite("didaktika") && looks_definite("esnafi") && looks_definite("lotët"));
        assert!(!looks_definite("fukadrugë") && !looks_definite("intensitet"));
    }
}
