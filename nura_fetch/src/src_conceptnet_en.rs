//! conceptnet_en — dija e përbashkët (commonsense) nga pjesa ANGLISHT e ConceptNet 5.7,
//! e nxjerrë nga arkivi i plotë tashmë i shkarkuar dhe i fiksuar në `SOURCES.lock`
//! (`raw/conceptnet57/conceptnet-assertions-5.7.0.csv.gz`). Nuk shkarkon asgjë.
//!
//! Mbahen vetëm skajet ku të dy anët janë `/c/en/` dhe relacioni është një nga
//! marrëdhëniet e dijes së përbashkët më poshtë. Rezultati është një TSV kompakt:
//!
//! ```text
//! rel ⇥ koka ⇥ bishti ⇥ pesha ⇥ dataset
//! ```
//!
//! `koka`/`bishti` janë termat anglisht me nënvizime (p.sh. `guard_house`); `dataset`
//! dallon dëshmitarët brenda ConceptNet-it (OMCS, Verbosity, Wiktionary, WordNet …).
//! Kalimi në shqip bëhet nga importuesi i `nura_lingua`, përmes urave të përkthimit
//! që grafi i ka tashmë.
//!
//! Stil: pa if/else — vetëm match.

use crate::common::{sha256_hex, Fetched};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

const ARCHIVE: &str = "raw/conceptnet57/conceptnet-assertions-5.7.0.csv.gz";
const OUT: &str = "raw/conceptnet57/conceptnet-5.7.0.en-commonsense.tsv";

const RELATIONS: &[&str] = &[
    "/r/IsA",
    "/r/PartOf",
    "/r/HasA",
    "/r/MadeOf",
    "/r/UsedFor",
    "/r/CapableOf",
    "/r/AtLocation",
    "/r/LocatedNear",
    "/r/HasProperty",
    "/r/Causes",
    "/r/CausesDesire",
    "/r/HasPrerequisite",
    "/r/HasSubevent",
    "/r/HasFirstSubevent",
    "/r/HasLastSubevent",
    "/r/MotivatedByGoal",
    "/r/Desires",
    "/r/NotDesires",
    "/r/ReceivesAction",
    "/r/CreatedBy",
    "/r/Antonym",
    "/r/MannerOf",
    "/r/ObstructedBy",
    "/r/Entails",
    "/r/SymbolOf",
    "/r/NotCapableOf",
    "/r/NotHasProperty",
    "/r/NotUsedFor",
];

/// Termi pa prefiksin dhe pa etiketën e pjesës së ligjëratës: `/c/en/guard_house/v` → `guard_house`.
fn term(uri: &str) -> Option<&str> {
    uri.strip_prefix("/c/en/").and_then(|r| r.split('/').next()).filter(|t| !t.is_empty())
}

/// Vlera numerike e një fushe JSON të thjeshtë (`"weight": 2.0`), pa analizues JSON.
fn json_number(meta: &str, key: &str) -> Option<f64> {
    let k = format!("\"{key}\":");
    let rest = meta.split_once(&k)?.1.trim_start();
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    rest[..end].trim().parse().ok()
}

fn json_string<'a>(meta: &'a str, key: &str) -> Option<&'a str> {
    let k = format!("\"{key}\":");
    let rest = meta.split_once(&k)?.1.trim_start().strip_prefix('"')?;
    rest.split_once('"').map(|(v, _)| v)
}

pub fn fetch(root: &Path) -> Result<Vec<Fetched>, String> {
    let archive = root.join(ARCHIVE);
    let file = fs::File::open(&archive).map_err(|e| format!("{}: {e} (shkarko më parë --conceptnet-full)", archive.display()))?;
    let mut reader = BufReader::new(flate2::read::GzDecoder::new(file));
    let mut out: Vec<u8> = Vec::with_capacity(64 << 20);
    out.extend_from_slice("# conceptnet_en — ConceptNet 5.7.0, skajet /c/en/ → /c/en/ të dijes së përbashkët\n".as_bytes());
    out.extend_from_slice("# licenca: CC BY-SA 4.0; atribuimi: ConceptNet 5.7 — Speer, Chin & Havasi (2017)\n".as_bytes());
    out.extend_from_slice("# rel\tkoka\tbishti\tpesha\tdataset\n".as_bytes());
    let mut seen: u64 = 0;
    let mut kept: u64 = 0;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => return Err(format!("{ARCHIVE}: {e}")),
        }
        seen += 1;
        let c: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
        let row = match c.as_slice() {
            [_, rel, a, b, meta, ..] => match (RELATIONS.contains(rel), term(a), term(b)) {
                (true, Some(h), Some(t)) => Some((*rel, h, t, *meta)),
                _ => None,
            },
            _ => None,
        };
        match row {
            Some((rel, h, t, meta)) => {
                kept += 1;
                let w = json_number(meta, "weight").unwrap_or(1.0);
                let ds = json_string(meta, "dataset").unwrap_or("");
                out.extend_from_slice(format!("{rel}\t{h}\t{t}\t{w:.3}\t{ds}\n").as_bytes());
            }
            None => {}
        }
    }
    fs::write(root.join(OUT), &out).map_err(|e| format!("{OUT}: {e}"))?;
    println!("[nura_fetch] conceptnet_en: {seen} rreshta të lexuar, {kept} skaje të dijes së përbashkët");
    Ok(vec![Fetched {
        source: "conceptnet57_en".into(),
        file: OUT.into(),
        url: "https://s3.amazonaws.com/conceptnet/downloads/2019/edges/conceptnet-assertions-5.7.0.csv.gz#filter=/c/en/commonsense".into(),
        version: "ConceptNet 5.7.0 — nxjerrje lokale nga arkivi i plotë".into(),
        license: "CC BY-SA 4.0".into(),
        attribution: "ConceptNet 5.7 — Speer, Chin & Havasi (2017), commonsense/conceptnet5".into(),
        bytes: out.len() as u64,
        sha256: sha256_hex(&out),
        publisher_check: format!("derivuar lokalisht nga {ARCHIVE}; {kept} skaje nga {seen} rreshta"),
        status: "FULL_FILTERED_EN_COMMONSENSE".into(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_and_json_fields() {
        assert_eq!(term("/c/en/guard_house/v"), Some("guard_house"));
        assert_eq!(term("/c/sq/qen"), None);
        let meta = r#"{"dataset": "/d/conceptnet/4/en", "license": "cc:by/4.0", "weight": 2.828}"#;
        assert_eq!(json_number(meta, "weight"), Some(2.828));
        assert_eq!(json_string(meta, "dataset"), Some("/d/conceptnet/4/en"));
    }
}
