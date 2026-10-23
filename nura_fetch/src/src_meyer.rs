//! Gustav Meyer, «Etymologisches Wörterbuch der albanesischen Sprache» (Strassburg: K. J. Trübner,
//! 1891; Sammlung indogermanischer Wörterbücher III).
//!
//! - Statusi: DOMEN PUBLIK. Autori vdiq më 28 gusht 1900; vepra u botua më 1891.
//! - Dy skanime të pavarura të Google Books në Internet Archive. Merret vetëm shtresa OCR
//!   (`…_djvu.txt`). Dy skanimet krahasohen nga parseri për të ndrequr gabimet e leximit.
//! - Çdo skedar verifikohet me SHA-1 që shpall Internet Archive: kur nuk përputhet, shkarkimi ndalet.
//! - Teksti është gjermanisht, me format shqipe të shkruara sipas drejtshkrimit të Meyer-it.

use crate::common::{self, Fetched};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::path::Path;

/// Skanimi më i plotë (972 KB OCR) dhe skanimi i dytë (613 KB OCR).
pub const SCANS: [&str; 2] = ["etymologischesw01meyegoog", "etymologischesw00meyegoog"];
pub const DIR: &str = "raw/meyer1891";
const LICENSE: &str = "Domen publik (autori † 1900; vepra 1891)";
const ATTRIBUTION: &str =
    "Gustav Meyer, «Etymologisches Wörterbuch der albanesischen Sprache», Strassburg: K. J. Trübner, 1891; skanim Google Books, OCR: Internet Archive";

fn djvu_entry(meta: &Value) -> Option<&Value> {
    meta.get("files")?
        .as_array()?
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()).map(|n| n.ends_with("_djvu.txt")).unwrap_or(false))
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    std::fs::create_dir_all(data.join(DIR)).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for id in SCANS {
        let meta_bytes = common::get(&format!("https://archive.org/metadata/{id}"))?;
        let meta: Value = serde_json::from_slice(&meta_bytes).map_err(|e| format!("meyer1891 {id}: metadata JSON: {e}"))?;
        let entry = djvu_entry(&meta).ok_or_else(|| format!("meyer1891 {id}: skedari _djvu.txt mungon"))?;
        let name = entry.get("name").and_then(|n| n.as_str()).ok_or("meyer1891: emri mungon")?.to_string();
        let published = entry.get("sha1").and_then(|s| s.as_str()).ok_or("meyer1891: IA nuk shpall SHA-1")?.to_string();
        let url = format!("https://archive.org/download/{id}/{name}");
        let bytes = common::get(&url)?;
        let local = sha1_hex(&bytes);
        match local == published {
            true => {}
            false => return Err(format!("meyer1891 {id}: SHA-1 nuk përputhet (IA {published}, lokale {local})")),
        }
        let rel = format!("{DIR}/{name}");
        std::fs::write(data.join(&rel), &bytes).map_err(|e| format!("{rel}: {e}"))?;
        println!("[nura_fetch] meyer1891: {name} {} B, SHA-1 e IA përputhet", bytes.len());
        rows.push(Fetched {
            source: "meyer1891".into(),
            file: rel,
            url,
            version: format!("Internet Archive {id} (Google Books scan), OCR djvu.txt"),
            license: LICENSE.into(),
            attribution: ATTRIBUTION.into(),
            bytes: bytes.len() as u64,
            sha256: common::sha256_hex(&bytes),
            publisher_check: format!("SHA-1 e IA PËRPUTHET ({published})"),
            status: "FULL".into(),
        });
    }
    Ok(rows)
}
