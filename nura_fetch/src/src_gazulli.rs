//! Nikollë Gazulli, «Fjalorth i ri – Fjalë të rralla të përdoruna në Veri të Shqipnis»
//! (Tiranë: Shtypshkroja «Gurakuqi», 1941 — kështu në faqen e titullit; disa katalogë japin 1942;
//! seria «Visaret e Kombit»: Internet Archive e numëron 9, katalogë të tjerë vëll. XI).
//!
//! - Statusi: DOMEN PUBLIK sipas afatit. Autori vdiq më 23 mars 1946; jeta + 70 vjet → i lirë në
//!   Shqipëri dhe në BE që nga 1 janari 2017. Internet Archive nuk jep licencë të shkruar për
//!   kopjen; vendimi për ta marrë si domen publik u dha nga pronari i projektit (2026-10-22).
//! - Teksti është shtresa OCR automatike (Tesseract, `-l sqi`) e Internet Archive për skanimin e
//!   botimit origjinal. Nuk është tekst i shkruar me AI; ka gabime leximi, ndaj importuesi e
//!   filtron (shih `nura_lingua/src/src_gazulli.rs`).
//! - Skedari fiksohet: madhësia dhe SHA-256 duhet të përputhen, ndryshe shkarkimi ndalet.
use crate::common::{self, Fetched};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::path::Path;

pub const IDENTIFIER: &str = "id-9-fjalorth-i-ri-fjale-te-rralla-te-perdoruna-ne-veri-te-shqipnis-visaret-e-kombit";
pub const REL_TXT: &str = "raw/gazulli1941/fjalorth-i-ri-1941.djvu.txt";
pub const REL_TSV: &str = "raw/gazulli1941/gazulli1941-sq.tsv";
/// Kopja e verifikuar më 2026-10-22 (651,514 bajte).
pub const PINNED_BYTES: usize = 651_514;
pub const PINNED_SHA256: &str = "b17dbaa5eac5937ef24cb529b500c72307a6d3aa4fd4395f7e9fa2d37a22329a";
const LICENSE: &str = "Domen publik sipas afatit (autori † 1946; jeta+70: Shqipëri/BE që nga 2017); IA nuk jep licencë të shkruar";
const ATTRIBUTION: &str = "Nikollë Gazulli, «Fjalorth i ri – Fjalë të rralla të përdoruna në Veri të Shqipnis», Tiranë: Shtypshkroja «Gurakuqi», 1941 (seria «Visaret e Kombit»); skanim dhe OCR: Internet Archive";

/// Emri i skedarit të tekstit OCR (`…_djvu.txt`) sipas metadatave të Internet Archive.
pub fn djvu_name(meta: &Value) -> Option<String> {
    djvu_entry(meta).and_then(|f| f.get("name")?.as_str().map(|n| n.to_string()))
}

fn djvu_entry(meta: &Value) -> Option<&Value> {
    meta.get("files")?
        .as_array()?
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()).map(|n| n.ends_with("_djvu.txt")).unwrap_or(false))
}

/// SHA-1 që Internet Archive shpall për skedarin, kundrejt asaj të llogaritur këtu.
fn ia_sha1_check(meta: &Value, bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(bytes);
    let local: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let published = djvu_entry(meta).and_then(|f| f.get("sha1")?.as_str().map(|s| s.to_string()));
    match published {
        Some(p) => match p == local {
            true => format!("SHA-1 e IA PËRPUTHET ({p})"),
            false => format!("SHA-1 e IA NUK përputhet (IA {p}, lokale {local})"),
        },
        None => format!("IA nuk shpall SHA-1 (lokale {local})"),
    }
}

/// Fushat e të drejtave në metadatat e IA (`licenseurl`, `rights`); për këtë skanim mungojnë.
fn ia_rights(meta: &Value) -> String {
    let field = |k: &str| meta.get("metadata").and_then(|m| m.get(k)).and_then(|v| v.as_str()).map(|s| s.to_string());
    match (field("licenseurl"), field("rights")) {
        (None, None) => "IA: pa fushë licence (licenseurl/rights mungojnë)".to_string(),
        (l, r) => format!("IA: licenseurl={} rights={}", l.unwrap_or("-".into()), r.unwrap_or("-".into())),
    }
}

/// Kodimi i emrit të skedarit për URL (hapësira dhe shkronja jo-ASCII).
fn url_path(name: &str) -> String {
    name.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let meta_url = format!("https://archive.org/metadata/{IDENTIFIER}");
    let meta_bytes = common::get(&meta_url)?;
    let meta: Value = serde_json::from_slice(&meta_bytes).map_err(|e| format!("gazulli: metadata JSON: {e}"))?;
    let name = djvu_name(&meta).ok_or("gazulli: skedari _djvu.txt nuk u gjet në metadata")?;
    let url = format!("https://archive.org/download/{IDENTIFIER}/{}", url_path(&name));
    print!("[nura_fetch] gazulli1941: {name} … ");
    let bytes = common::get(&url)?;
    let sha = common::sha256_hex(&bytes);
    let pinned = bytes.len() == PINNED_BYTES && sha == PINNED_SHA256;
    match pinned {
        true => {}
        false => {
            return Err(format!(
                "gazulli1941: teksti nuk përputhet me kopjen e fiksuar ({} B, sha256 {sha}); ndalet pa shkruar asgjë",
                bytes.len()
            ))
        }
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let rows = crate::gazulli_parse::entries(&text);
    let check = format!("SHA-256 e fiksuar përputhet; {}; {}", ia_sha1_check(&meta, &bytes), ia_rights(&meta));
    println!("{} B, SHA-256 e fiksuar PËRPUTHET, {} hyrje", bytes.len(), rows.len());
    let version = format!("archive.org/{IDENTIFIER} (OCR, sha256 {PINNED_SHA256})");
    let mut out = vec![common::record(
        data,
        "gazulli1941",
        REL_TXT,
        &bytes,
        &url,
        &version,
        LICENSE,
        ATTRIBUTION,
        &check,
        "FULL",
    )?];
    let mut tsv = format!(
        "# gazulli1941-sq.tsv — {ATTRIBUTION}; {LICENSE}; {version}\n# kolonat: fjala\tshënimi_gramatikor\tshpjegimi (shqip, ortografia e 1941)\n"
    );
    for r in &rows {
        tsv.push_str(&format!("{}\t{}\t{}\n", common::clean(&r.word), common::clean(&r.grammar), common::clean(&r.gloss)));
    }
    out.push(common::record(
        data,
        "gazulli1941",
        REL_TSV,
        tsv.as_bytes(),
        &url,
        &version,
        LICENSE,
        ATTRIBUTION,
        "nxjerrë nga teksti OCR me rregulla të deklaruara",
        "FULL",
    )?);
    Ok(out)
}
