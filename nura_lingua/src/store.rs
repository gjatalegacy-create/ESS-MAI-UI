//! Importimi i plotë nga `data/raw` dhe indeksi lokal `data/index/nura_graph.idx`.
//!
//! Para importimit, çdo skedar verifikohet me SHA-256 kundrejt `SOURCES.lock`
//! (i shkruar nga `nura_fetch`). Kjo është kontroll integriteti lokal, jo
//! provë autenticiteti nga botuesi.

use crate::graph::*;
use crate::import::*;
use crate::morph::{Morph, MorphEntry};
use crate::sha256::sha256_hex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const INDEX_MAGIC: &str = "NURA_GRAPH_IDX v1";

#[derive(Debug, Clone)]
pub struct LockRow {
    pub source: String,
    pub file: String,
    pub url: String,
    pub version: String,
    pub license: String,
    pub attribution: String,
    pub bytes: u64,
    pub sha256: String,
    pub publisher_check: String,
    pub status: String,
}

pub fn read_lock(data: &Path) -> Result<Vec<LockRow>, String> {
    let text = fs::read_to_string(data.join("SOURCES.lock"))
        .map_err(|e| format!("SOURCES.lock mungon ({e}); ekzekuto fillimisht nura_fetch"))?;
    Ok(text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            match c.len() >= 10 {
                true => Some(LockRow {
                    source: c[0].into(),
                    file: c[1].into(),
                    url: c[2].into(),
                    version: c[3].into(),
                    license: c[4].into(),
                    attribution: c[5].into(),
                    bytes: c[6].parse().unwrap_or(0),
                    sha256: c[7].into(),
                    publisher_check: c[8].into(),
                    status: c[9].into(),
                }),
                false => None,
            }
        })
        .collect())
}

/// Lexon skedarin vetëm nëse SHA-256 përputhet me SOURCES.lock.
fn read_verified(data: &Path, lock: &[LockRow], rel: &str, rep: &mut ImportReport) -> Result<String, String> {
    let row = lock
        .iter()
        .find(|r| r.file == rel)
        .ok_or_else(|| format!("{rel} nuk është në SOURCES.lock"))?;
    let bytes = fs::read(data.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let h = sha256_hex(&bytes);
    match h == row.sha256 {
        true => {
            rep.note(format!("SHA-256 OK {rel} ({} B)", bytes.len()));
            String::from_utf8(bytes).map_err(|e| format!("{rel}: UTF-8 {e}"))
        }
        false => Err(format!("{rel}: SHA-256 {h} ≠ SOURCES.lock {}", row.sha256)),
    }
}

pub struct Built {
    pub graph: Graph,
    pub morph: Morph,
    pub report: ImportReport,
}

/// Importim nga paketat e shkarkuara.
pub fn import_all(data: &Path) -> Result<Built, String> {
    let lock = read_lock(data)?;
    let mut g = Graph::new();
    let mut m = Morph::default();
    let mut rep = ImportReport::default();
    for f in ["data.noun", "data.verb", "data.adj", "data.adv"] {
        let t = read_verified(data, &lock, &format!("raw/pwn30/{f}"), &mut rep)?;
        import_pwn_data(&mut g, &t, &mut rep)?;
    }
    match lock.iter().any(|r| r.file == "raw/pwn30/index.sense") {
        true => {
            let t = read_verified(data, &lock, "raw/pwn30/index.sense", &mut rep)?;
            pwn_sense_frequency(&mut g, &t, &mut rep);
        }
        false => {}
    }
    let t = read_verified(data, &lock, "raw/omw_als/wn-data-als.tab", &mut rep)?;
    import_omw_tab(&mut g, &t, &mut rep)?;
    let t = read_verified(data, &lock, "raw/conceptnet57/conceptnet-5.7.0.sq-edges.csv", &mut rep)?;
    import_conceptnet(&mut g, &t, &mut rep)?;
    // Indeksi i kuptimeve të WordNet-it: për BabelNet-in dhe për urën e dijes së përbashkët.
    let babelnet = lock.iter().any(|r| r.file == "raw/babelnet53/babelnet-sq.tsv");
    let commonsense = lock.iter().any(|r| r.file == crate::src_commonsense::TSV);
    let sense_index = match (lock.iter().any(|r| r.file == "raw/pwn30/index.sense"), babelnet || commonsense) {
        (true, true) => read_verified(data, &lock, "raw/pwn30/index.sense", &mut rep)?,
        _ => String::new(),
    };
    // BabelNet: vetëm nëse është shkarkuar (cache lokal i API-së, i verifikuar me SHA-256).
    match babelnet {
        true => {
            let keys = match sense_index.is_empty() {
                false => parse_index_sense(&sense_index),
                true => {
                    rep.note("pwn30: index.sense mungon — BabelNet nuk lidhet me WordNet".into());
                    std::collections::HashMap::new()
                }
            };
            let t = read_verified(data, &lock, "raw/babelnet53/babelnet-sq.tsv", &mut rep)?;
            import_babelnet(&mut g, &t, &keys, &mut rep);
        }
        false => rep.note("babelnet53: ende i pashkarkuar".into()),
    }
    // Fjalorët shqip pa Wikipedia (LEXIN, leksiku gramatikor, Wiktionary, EuroVoc, Wikidata, …).
    match lock.iter().any(|r| r.file == crate::src_uniparser::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_uniparser::TSV, &mut rep)?;
            crate::src_uniparser::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("uniparser: ende i pashkarkuar".into()),
    }
    // Teksti i Wiktionary-t anglisht mbahet edhe për urën e përkthimit të dijes së përbashkët.
    let mut kaikki_en = String::new();
    match lock.iter().any(|r| r.file == crate::src_kaikki_en::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_kaikki_en::TSV, &mut rep)?;
            crate::src_kaikki_en::import(&mut g, &mut m, &t, &mut rep);
            kaikki_en = t;
        }
        false => rep.note("kaikki_en: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_kaikki_de::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_kaikki_de::TSV, &mut rep)?;
            crate::src_kaikki_de::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("kaikki_de: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_wd_lexemes::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_wd_lexemes::TSV, &mut rep)?;
            crate::src_wd_lexemes::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("wd_lexemes: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_lexin::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_lexin::TSV, &mut rep)?;
            crate::src_lexin::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("lexin: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_eurovoc::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_eurovoc::TSV, &mut rep)?;
            crate::src_eurovoc::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("eurovoc: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_loanwords::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_loanwords::TSV, &mut rep)?;
            crate::src_loanwords::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("loanwords: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_dialects::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_dialects::TSV, &mut rep)?;
            crate::src_dialects::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("dialects: ende i pashkarkuar".into()),
    }
    match lock.iter().any(|r| r.file == crate::src_gazulli::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_gazulli::TSV, &mut rep)?;
            crate::src_gazulli::import(&mut g, &mut m, &t, &mut rep);
        }
        false => rep.note("gazulli1941: ende i pashkarkuar".into()),
    }
    // Leksiku i modelit (një dëshmitar, jo autoritet): pas të gjitha burimeve me kuptime, që të
    // dihet cila fjalë ka kuptim shqip; para rregullave, që baza e tij të vlejë si konfirmim.
    // Teksti mbahet edhe si dëshmitar i përkthimit për skajet e dobëta të dijes së përbashkët.
    let model_path = data.join("..").join(crate::src_model::PATH);
    let mut model_text = String::new();
    match fs::read(&model_path) {
        Ok(bytes) => {
            rep.note(format!("model_claude: {} (SHA-256 {})", model_path.display(), sha256_hex(&bytes)));
            let text = String::from_utf8(bytes).map_err(|e| format!("leksiku i modelit: UTF-8 {e}"))?;
            crate::src_model::import(&mut g, &text, &mut rep);
            model_text = text;
        }
        Err(_) => rep.note("model_claude: leksiku i modelit mungon".into()),
    }
    // Shënimet e pronarit (shpjegime popullore, të thëna gjithmonë si të tilla).
    let owner_path = data.join("..").join(crate::src_owner::PATH);
    match fs::read(&owner_path) {
        Ok(bytes) => {
            rep.note(format!("owner_note: {} (SHA-256 {})", owner_path.display(), sha256_hex(&bytes)));
            let text = String::from_utf8(bytes).map_err(|e| format!("shënimet e pronarit: UTF-8 {e}"))?;
            crate::src_owner::import(&mut g, &text, &mut rep);
        }
        Err(_) => rep.note("owner_note: s'ka shënime të pronarit".into()),
    }
    let t = read_verified(data, &lock, "raw/ud_sq_tsa/sq_tsa-ud-test.conllu", &mut rep)?;
    import_ud(&mut g, &mut m, Source::UdSqTsa, &t, &mut rep)?;
    for f in ["sq_staf-ud-train.conllu", "sq_staf-ud-dev.conllu", "sq_staf-ud-test.conllu"] {
        let t = read_verified(data, &lock, &format!("raw/ud_sq_staf/{f}"), &mut rep)?;
        import_ud(&mut g, &mut m, Source::UdSqStaf, &t, &mut rep)?;
    }
    // Zgjerime lokale: vetëm nëse ekziston skedari, i etiketuar veçmas.
    let ext = data.join("local_ext").join("extensions.tsv");
    match fs::read_to_string(&ext) {
        Ok(t) => import_local_extensions(&mut g, &t, &mut rep),
        Err(_) => rep.note("local_ext: asnjë zgjerim lokal (skedari mungon)".into()),
    }
    // Rregullat semantike të fjalëformimit të shqipes: zbatohen pasi janë ngarkuar të gjitha fjalët.
    let rules_path = data.join("..").join("rules").join("sq_fjaleformimi.tsv");
    match fs::read(&rules_path) {
        Ok(bytes) => {
            rep.note(format!("sq_rules: tabela {} (SHA-256 {})", rules_path.display(), sha256_hex(&bytes)));
            let text = String::from_utf8(bytes).map_err(|e| format!("rregullat: UTF-8 {e}"))?;
            let rules = crate::derive::parse_rules(&text)?;
            crate::derive::apply(&mut g, &rules, &mut rep);
        }
        Err(_) => rep.note("sq_rules: tabela e rregullave mungon".into()),
    }
    // Etimologjia: Wiktionary (shabllonet e prejardhjes) dhe Meyer 1891 (domen publik).
    match lock.iter().any(|r| r.file == crate::src_etymology::TSV) {
        true => {
            let t = read_verified(data, &lock, crate::src_etymology::TSV, &mut rep)?;
            crate::src_etymology::import(&mut g, &t, &mut rep);
        }
        false => rep.note("etymology: nxjerrja e Wiktionary-t mungon".into()),
    }
    match crate::src_meyer::SCANS.iter().all(|f| lock.iter().any(|r| r.file == *f)) {
        true => {
            let a = read_verified(data, &lock, crate::src_meyer::SCANS[0], &mut rep)?;
            let b = read_verified(data, &lock, crate::src_meyer::SCANS[1], &mut rep)?;
            crate::src_meyer::import(&mut g, &m, &a, &b, &mut rep);
        }
        false => rep.note("meyer1891: skanimet mungojnë".into()),
    }
    // Dija e përbashkët (ConceptNet anglisht → shqip): pas të gjitha burimeve, që urat e
    // përkthimit (Wiktionary, AlbaNet, BabelNet) të jenë të plota.
    match commonsense {
        true => {
            let t = read_verified(data, &lock, crate::src_commonsense::TSV, &mut rep)?;
            let mut index: Vec<(Pos, String)> = Vec::new();
            for (f, p) in crate::src_commonsense::PWN_INDEX {
                index.push((p, read_verified(data, &lock, f, &mut rep)?));
            }
            let refs: Vec<(Pos, &str)> = index.iter().map(|(p, t)| (*p, t.as_str())).collect();
            // Dëshmia e modelit për treshet: dëshmitari i dytë i lidhjes, jo autoritet.
            let deshmi_path = data.join("..").join(crate::src_commonsense::WITNESS);
            let deshmi = match fs::read(&deshmi_path) {
                Ok(bytes) => {
                    rep.note(format!("model_claude_cs: {} (SHA-256 {})", deshmi_path.display(), sha256_hex(&bytes)));
                    String::from_utf8(bytes).map_err(|e| format!("dëshmia e modelit: UTF-8 {e}"))?
                }
                Err(_) => {
                    rep.note("model_claude_cs: dëshmia e modelit për treshet mungon".into());
                    String::new()
                }
            };
            crate::src_commonsense::import(&mut g, &t, &kaikki_en, &refs, &sense_index, &model_text, &deshmi, &mut rep);
        }
        false => rep.note("conceptnet_en: ende i pashkarkuar".into()),
    }
    // Lidhjet sipas përkufizimit: pas të gjitha burimeve, që çdo shpjegim dhe çdo trajtë të jetë në graf.
    crate::src_perkufizime::import(&mut g, &m, &mut rep);
    coverage_report(&g, &mut rep);
    Ok(Built { graph: g, morph: m, report: rep })
}

/// Formati: `lemë_sq ⇥ synset_pwn30 ⇥ autori ⇥ shënim`. Çdo lidhje mbetet
/// e etiketuar `local_ext` dhe nuk përzihet me Albanet-in.
pub fn import_local_extensions(g: &mut Graph, text: &str, rep: &mut ImportReport) {
    let mut n = 0;
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        match (c.first(), c.get(1).and_then(|s| normalize_synset(s)), c.get(2)) {
            (Some(lemma), Some(syn), Some(author)) => match g.get(&sense_key(Source::Pwn30, &syn)) {
                Some(sid) => {
                    let w = g.ensure_word(Lang::Sq, &lemma.to_lowercase());
                    g.add_edge(Edge {
                        from: w,
                        to: sid,
                        rel: Rel::HasSense,
                        source: Source::LocalExtension,
                        lang: Some(Lang::Sq),
                        method: Method::Published,
                        attrs: vec![("author".into(), author.to_string())],
                    });
                    n += 1;
                }
                None => rep.note(format!("local_ext: synset {syn} nuk ekziston — u refuzua")),
            },
            _ => rep.note(format!("local_ext: rresht i pavlefshëm: {l}")),
        }
    }
    rep.note(format!("local_ext: {n} lidhje lokale të etiketuara"));
}

fn coverage_report(g: &Graph, rep: &mut ImportReport) {
    let mut sq_words = 0;
    let mut sq_with_sense = 0;
    let mut sq_only_ud = 0;
    let mut sq_word_rel = 0;
    let mut senses_with_sq = 0;
    let mut senses = 0;
    for (i, n) in g.nodes.iter().enumerate() {
        match &n.kind {
            NodeKind::Word { lang: Lang::Sq, .. } => {
                sq_words += 1;
                let has_sense = !g.senses_of(i as u32).is_empty();
                // Lidhje në nivel fjale: ConceptNet (FormOf), marrëdhënie semantike, fjalëformim.
                let word_rel = |r: &Rel| matches!(r, Rel::FormOf | Rel::Sem(_) | Rel::Deriv(_));
                let has_rel = g.in_edges(i as u32).any(|(_, e)| word_rel(&e.rel))
                    || g.out_edges(i as u32).any(|(_, e)| word_rel(&e.rel));
                match (has_sense, has_rel) {
                    (true, _) => sq_with_sense += 1,
                    (false, true) => sq_word_rel += 1,
                    (false, false) => sq_only_ud += 1,
                }
            }
            NodeKind::Sense { .. } => {
                senses += 1;
                match n.labels.iter().any(|t| t.lang == Lang::Sq) {
                    true => senses_with_sq += 1,
                    false => {}
                }
            }
            _ => {}
        }
    }
    rep.note(format!(
        "MBULIMI: {sq_words} fjalë shqipe gjithsej; {sq_with_sense} me të paktën një kuptim (AlbaNet, BabelNet ose Wiktionary); {sq_word_rel} pa kuptim, por me lidhje fjale (sinonim, të kundërt, prejardhje, ConceptNet); {sq_only_ud} vetëm me formë/gramatikë ose përdorim UD; {senses_with_sq}/{senses} kuptime kanë lemë shqipe"
    ));
}

// ───────────────────────────── indeksi ─────────────────────────────

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "")
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        match c {
            '\\' => match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            },
            o => out.push(o),
        }
    }
    out
}

pub fn write_index(b: &Built, path: &Path) -> Result<(), String> {
    let mut s = String::with_capacity(64 * 1024 * 1024);
    s.push_str(INDEX_MAGIC);
    s.push('\n');
    for n in &b.graph.nodes {
        match &n.kind {
            NodeKind::Word { lang, lemma } => s.push_str(&format!("W\t{}\t{}\n", lang.code(), esc(lemma))),
            NodeKind::Sense { source, synset, pos } => {
                s.push_str(&format!("S\t{}\t{}\t{}\n", source.code(), synset, pos.code()))
            }
            NodeKind::Concept { uri } => s.push_str(&format!("C\t{}\n", esc(uri))),
            NodeKind::Sentence { source, sent_id } => {
                s.push_str(&format!("U\t{}\t{}\n", source.code(), esc(sent_id)))
            }
        }
        for (tag, list) in [("L", &n.labels), ("G", &n.glosses), ("X", &n.examples)] {
            for t in list {
                s.push_str(&format!("{tag}\t{}\t{}\t{}\n", t.lang.code(), t.source.code(), esc(&t.text)));
            }
        }
    }
    for e in &b.graph.edges {
        let attrs = e
            .attrs
            .iter()
            .map(|(k, v)| format!("{}={}", esc(k).replace(';', "\\;").replace('=', "\\e"), esc(v).replace(';', "\\;")))
            .collect::<Vec<_>>()
            .join(";");
        s.push_str(&format!(
            "E\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            e.from,
            e.to,
            esc(&e.rel.code()),
            e.source.code(),
            e.lang.map(|l| l.code()).unwrap_or("-"),
            e.method.code(),
            attrs
        ));
    }
    for m in &b.morph.entries {
        s.push_str(&format!(
            "M\t{}\t{}\t{}\t{}\t{}\t{}\n",
            esc(&m.lemma),
            esc(&m.form),
            m.upos,
            esc(&m.feats),
            esc(&m.sent_id),
            m.source.code()
        ));
    }
    for r in &b.report.lines {
        s.push_str(&format!("R\t{}\n", esc(r)));
    }
    match path.parent() {
        Some(p) => fs::create_dir_all(p).map_err(|e| e.to_string())?,
        None => {}
    }
    fs::write(path, s).map_err(|e| e.to_string())
}

fn split_attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut parts = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match (c, it.peek()) {
            ('\\', Some(';')) => {
                cur.push(';');
                it.next();
            }
            (';', _) => parts.push(std::mem::take(&mut cur)),
            (o, _) => cur.push(o),
        }
    }
    match cur.is_empty() {
        true => {}
        false => parts.push(cur),
    }
    for p in parts {
        match p.split_once('=') {
            Some((k, v)) => out.push((unesc(&k.replace("\\e", "=")), unesc(v))),
            None => {}
        }
    }
    out
}

pub fn load_index(path: &Path) -> Result<Built, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("indeksi {}: {e}", path.display()))?;
    let mut lines = text.lines();
    match lines.next() {
        Some(INDEX_MAGIC) => {}
        _ => return Err("indeksi nuk ka kokën NURA_GRAPH_IDX v1".into()),
    }
    let mut g = Graph::new();
    let mut m = Morph::default();
    let mut rep = ImportReport::default();
    let mut cur: Option<u32> = None;
    for l in lines {
        let c: Vec<&str> = l.split('\t').collect();
        let bad = || format!("rresht indeksi i pavlefshëm: {}", l.chars().take(80).collect::<String>());
        match c[0] {
            "W" => {
                let lang = Lang::parse(c[1]).ok_or_else(bad)?;
                let lemma = unesc(c[2]);
                cur = Some(g.ensure(word_key(lang, &lemma), NodeKind::Word { lang, lemma }));
            }
            "S" => {
                let source = Source::parse(c[1]).ok_or_else(bad)?;
                let pos = Pos::from_wn(c[3]).ok_or_else(bad)?;
                cur = Some(g.ensure(
                    sense_key(source, c[2]),
                    NodeKind::Sense { source, synset: c[2].to_string(), pos },
                ));
            }
            "C" => {
                let uri = unesc(c[1]);
                cur = Some(g.ensure(concept_key(&uri), NodeKind::Concept { uri }));
            }
            "U" => {
                let source = Source::parse(c[1]).ok_or_else(bad)?;
                let sent_id = unesc(c[2]);
                cur = Some(g.ensure(sentence_key(source, &sent_id), NodeKind::Sentence { source, sent_id }));
            }
            "L" | "G" | "X" => {
                let id = cur.ok_or_else(bad)?;
                let t = Text {
                    lang: Lang::parse(c[1]).ok_or_else(bad)?,
                    source: Source::parse(c[2]).ok_or_else(bad)?,
                    text: unesc(c.get(3).copied().unwrap_or("")),
                };
                let n = &mut g.nodes[id as usize];
                match c[0] {
                    "L" => n.labels.push(t),
                    "G" => n.glosses.push(t),
                    _ => n.examples.push(t),
                }
            }
            "E" => {
                let e = Edge {
                    from: c[1].parse().map_err(|_| bad())?,
                    to: c[2].parse().map_err(|_| bad())?,
                    rel: Rel::parse(&unesc(c[3])).ok_or_else(bad)?,
                    source: Source::parse(c[4]).ok_or_else(bad)?,
                    lang: Lang::parse(c[5]),
                    method: Method::parse(c[6]).ok_or_else(bad)?,
                    attrs: split_attrs(c.get(7).copied().unwrap_or("")),
                };
                let id = g.edges.len() as u32;
                g.out[e.from as usize].push(id);
                g.inc[e.to as usize].push(id);
                g.edges.push(e);
            }
            "M" => m.add(MorphEntry {
                lemma: unesc(c[1]),
                form: unesc(c[2]),
                upos: c[3].to_string(),
                feats: unesc(c[4]),
                sent_id: unesc(c[5]),
                source: Source::parse(c[6]).ok_or_else(bad)?,
            }),
            "R" => rep.note(unesc(c[1])),
            _ => return Err(bad()),
        }
    }
    Ok(Built { graph: g, morph: m, report: rep })
}

pub fn default_data_dir() -> PathBuf {
    match std::env::var("UI_ESS_MAI_DATA") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("data"),
    }
}

pub fn index_path(data: &Path) -> PathBuf {
    data.join("index").join("nura_graph.idx")
}

/// Indeks fjalësh shqipe për kërkim (lemë me shkronja të vogla → nyje Word).
pub fn sq_lexicon(g: &Graph) -> HashMap<String, u32> {
    g.nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match &n.kind {
            NodeKind::Word { lang: Lang::Sq, lemma } => Some((lemma.clone(), i as u32)),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_roundtrip_preserves_nodes_edges_attrs() {
        let mut g = Graph::new();
        let w = g.ensure_word(Lang::Sq, "bardhë\tx");
        let c = g.ensure(concept_key("/c/sq/zi"), NodeKind::Concept { uri: "/c/sq/zi".into() });
        g.add_label(c, Text { lang: Lang::Sq, text: "zi".into(), source: Source::ConceptNet57 });
        g.add_edge(Edge {
            from: c,
            to: w,
            rel: Rel::Cn("/r/Antonym".into()),
            source: Source::ConceptNet57,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![("license".into(), "cc:by-sa/4.0".into()), ("k;x".into(), "v=1;2".into())],
        });
        let b = Built { graph: g, morph: Morph::default(), report: ImportReport::default() };
        let p = std::env::temp_dir().join("nura_idx_roundtrip_test.idx");
        write_index(&b, &p).unwrap();
        let l = load_index(&p).unwrap();
        let _ = fs::remove_file(&p);
        assert_eq!(l.graph.nodes.len(), 2);
        assert_eq!(l.graph.node(0).key, "w:sq:bardhë\tx");
        assert_eq!(l.graph.edges[0].attrs, b.graph.edges[0].attrs);
        assert_eq!(l.graph.label_in(1, Lang::Sq), Some("zi"));
    }
}

/// ConceptNet konsiderohet i pjesshëm nëse çdo rresht i tij në SOURCES.lock
/// ka status PARTIAL_*, ose nëse mungon fare.
pub fn conceptnet_partial(lock: &[LockRow]) -> bool {
    let rows: Vec<&LockRow> = lock.iter().filter(|r| r.source == "conceptnet57").collect();
    match rows.is_empty() {
        true => true,
        false => rows.iter().any(|r| r.status.starts_with("PARTIAL")),
    }
}

#[cfg(test)]
mod partial_tests {
    use super::*;

    fn row(status: &str) -> LockRow {
        LockRow {
            source: "conceptnet57".into(),
            file: String::new(),
            url: String::new(),
            version: String::new(),
            license: String::new(),
            attribution: String::new(),
            bytes: 0,
            sha256: String::new(),
            publisher_check: String::new(),
            status: status.into(),
        }
    }

    #[test]
    fn conceptnet_partial_follows_lock_status() {
        assert!(conceptnet_partial(&[]));
        assert!(conceptnet_partial(&[row("PARTIAL_PREFIX"), row("PARTIAL_PREFIX_FILTERED_SQ")]));
        assert!(!conceptnet_partial(&[row("FULL"), row("FULL_FILTERED_SQ")]));
    }
}
