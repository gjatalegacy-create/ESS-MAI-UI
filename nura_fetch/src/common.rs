//! Mjete të përbashkëta për shkarkuesit e burimeve.
//!
//! RREGULL: asnjë kërkesë nuk mban të dhëna personale (email, emër, çelës në log).
//! User-Agent është gjithmonë `UA` më poshtë.
#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

pub const UA: &str = "ui_ess_mai-nura_fetch/0.1 (local research use; no personal data)";

/// Një skedar i shkarkuar ose i nxjerrë, gati për SOURCES.lock.
#[derive(Debug, Clone)]
pub struct Fetched {
    pub source: String,
    /// Rruga relative ndaj dosjes `data` (p.sh. "raw/lexin/lexin-sq.tsv").
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

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().user_agent(UA).build().into()
}

/// GET i plotë (pa kufi madhësie).
pub fn get(url: &str) -> Result<Vec<u8>, String> {
    get_with(url, &[], None)
}

/// GET me parametra query (p.sh. SPARQL) dhe kokë Accept opsionale.
pub fn get_with(url: &str, query: &[(&str, &str)], accept: Option<&str>) -> Result<Vec<u8>, String> {
    let ag = agent();
    let mut req = ag.get(url);
    for (k, v) in query {
        req = req.query(*k, *v);
    }
    req = match accept {
        Some(a) => req.header("Accept", a),
        None => req,
    };
    let mut resp = req.call().map_err(|e| format!("GET {url}: {e}"))?;
    let mut out = Vec::new();
    resp.body_mut().as_reader().read_to_end(&mut out).map_err(|e| format!("lexim {url}: {e}"))?;
    Ok(out)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match path.parent() {
        Some(p) => fs::create_dir_all(p).map_err(|e| e.to_string())?,
        None => {}
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Shkruan `bytes` te `data/<rel>` dhe kthen rreshtin për SOURCES.lock.
#[allow(clippy::too_many_arguments)]
pub fn record(
    data: &Path,
    source: &str,
    rel: &str,
    bytes: &[u8],
    url: &str,
    version: &str,
    license: &str,
    attribution: &str,
    publisher_check: &str,
    status: &str,
) -> Result<Fetched, String> {
    write(&data.join(rel), bytes)?;
    Ok(Fetched {
        source: source.into(),
        file: rel.into(),
        url: url.into(),
        version: version.into(),
        license: license.into(),
        attribution: attribution.into(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        publisher_check: publisher_check.into(),
        status: status.into(),
    })
}

/// Pastron një fushë për TSV (pa tabulime / rreshta të rinj).
pub fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ").trim().to_string()
}
