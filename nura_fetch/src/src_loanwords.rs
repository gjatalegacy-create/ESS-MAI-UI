//! OpenCovenant/loanwords — fjalë të huaja në shqip me njëvlerëset shqipe (MIT).
//!
//! - Commit-i i fundit që preku `loanwords.json` merret nga burimi Atom i GitHub-it
//!   (`github.com/<repo>/commits/<degë>/<rrugë>.atom`, pa API me kufi); bajtet merren nga
//!   URL-ja e fiksuar te ai commit.
//! - Kontrolli i botuesit: git-blob SHA-1 lokal krahasohet me atë të API-së së GitHub-it kur
//!   API-ja përgjigjet; kur jo (kufi kërkesash), kjo shënohet qartë dhe nuk paraqitet si provë.
//! - LICENSE shkarkohet dhe verifikohet («MIT License») para se të shkruhet ndonjë gjë.
//! - TSV: `fjala_e_huaj ⇥ njëvlerësja_shqipe ⇥ origjina ⇥ fjala_burimore` (një rresht për çdo njëvlerëse).
use crate::common::{self, Fetched};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::path::Path;

const REPO: &str = "OpenCovenant/loanwords";
const BRANCH: &str = "main";
const FILE: &str = "loanwords.json";
const REL_TSV: &str = "raw/loanwords/loanwords-sq.tsv";
const LICENSE: &str = "MIT (OpenCovenant/loanwords, LICENSE)";
const ATTRIBUTION: &str = "OpenCovenant — loanwords dataset (Copyright (c) 2022 OpenCovenant), https://github.com/OpenCovenant/loanwords";

fn git_blob_sha1(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(format!("blob {}\0", bytes.len()).as_bytes());
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// (commit, data) e hyrjes së parë të një burimi Atom të commit-eve të GitHub-it.
pub fn first_commit(atom: &str) -> Option<(String, String)> {
    let entry = atom.split("<entry>").nth(1)?;
    let sha: String = entry.split("Grit::Commit/").nth(1)?.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    let updated = entry
        .split("<updated>")
        .nth(1)
        .and_then(|s| s.split('<').next())
        .unwrap_or("")
        .to_string();
    match sha.len() {
        40 => Some((sha, updated)),
        _ => None,
    }
}

fn latest_commit(path: &str) -> Result<(String, String), String> {
    let url = format!("https://github.com/{REPO}/commits/{BRANCH}/{path}.atom");
    let atom = String::from_utf8_lossy(&common::get(&url)?).to_string();
    first_commit(&atom).ok_or_else(|| format!("loanwords: commit-i për {path} nuk u gjet në {url}"))
}

fn publisher_check(path: &str, commit: &str, bytes: &[u8]) -> Result<String, String> {
    let local = git_blob_sha1(bytes);
    let api = format!("https://api.github.com/repos/{REPO}/contents/{path}?ref={commit}");
    let published = common::get_with(&api, &[], Some("application/vnd.github+json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v.get("sha").and_then(|s| s.as_str()).map(|s| s.to_string()));
    match published {
        Some(p) => match p == local {
            true => Ok(format!("git-blob-sha1 PËRPUTHET ({p})")),
            false => Err(format!("loanwords {path}: blob i botuesit {p} ≠ blob lokal {local}")),
        },
        None => Ok(format!(
            "git-blob-sha1 lokal {local}; API e GitHub-it nuk u arrit (kufi kërkesash?) — pa krahasim me botuesin"
        )),
    }
}

/// Rreshtat TSV nga `loanwords.json`.
pub fn to_tsv_rows(json: &[u8]) -> Result<Vec<String>, String> {
    let v: Value = serde_json::from_slice(json).map_err(|e| format!("loanwords: JSON i pavlefshëm: {e}"))?;
    let obj = v.as_object().ok_or_else(|| "loanwords: pritej objekt JSON".to_string())?;
    let mut rows = Vec::new();
    for (key, entry) in obj {
        let loan = common::clean(key);
        let origin = common::clean(entry.get("origin").and_then(|x| x.as_str()).unwrap_or(""));
        let etymon = common::clean(entry.get("word").and_then(|x| x.as_str()).unwrap_or(""));
        let alts = entry.get("alternatives").and_then(|a| a.as_array()).cloned().unwrap_or_default();
        for a in alts.iter().filter_map(|a| a.as_str()).map(common::clean) {
            match (loan.is_empty(), a.is_empty()) {
                (false, false) => rows.push(format!("{loan}\t{a}\t{origin}\t{etymon}")),
                _ => {}
            }
        }
    }
    rows.sort();
    rows.dedup();
    Ok(rows)
}

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let mut out = Vec::new();
    // 1. Licenca, e verifikuar para të dhënave.
    let (lic_commit, _) = latest_commit("LICENSE")?;
    let lic_url = format!("https://raw.githubusercontent.com/{REPO}/{lic_commit}/LICENSE");
    let lic = common::get(&lic_url)?;
    match String::from_utf8_lossy(&lic).contains("MIT License") {
        true => {}
        false => return Err("loanwords: LICENSE nuk është MIT — ndalet".into()),
    }
    let check = publisher_check("LICENSE", &lic_commit, &lic)?;
    out.push(common::record(
        data,
        "loanwords",
        "raw/loanwords/LICENSE",
        &lic,
        &lic_url,
        &format!("{REPO}@{lic_commit}"),
        LICENSE,
        ATTRIBUTION,
        &check,
        "FULL",
    )?);
    // 2. Të dhënat.
    let (commit, date) = latest_commit(FILE)?;
    let url = format!("https://raw.githubusercontent.com/{REPO}/{commit}/{FILE}");
    print!("[nura_fetch] loanwords: {FILE}@{commit} … ");
    let bytes = common::get(&url)?;
    let rows = to_tsv_rows(&bytes)?;
    let check = publisher_check(FILE, &commit, &bytes)?;
    println!("{} B, {} çifte, {check}", bytes.len(), rows.len());
    let version = format!("{REPO}@{commit} ({date})");
    out.push(common::record(data, "loanwords", "raw/loanwords/loanwords.json", &bytes, &url, &version, LICENSE, ATTRIBUTION, &check, "FULL")?);
    let mut tsv = format!(
        "# loanwords-sq.tsv — {ATTRIBUTION}; {LICENSE}; {version}\n# kolonat: fjala_e_huaj\tnjëvlerësja_shqipe\torigjina\tfjala_burimore(gjuhë e huaj, jo shqip)\n"
    );
    for r in &rows {
        tsv.push_str(r);
        tsv.push('\n');
    }
    out.push(common::record(data, "loanwords", REL_TSV, tsv.as_bytes(), &url, &version, LICENSE, ATTRIBUTION, "nxjerrë nga loanwords.json", "FULL")?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hyrje të vërteta nga loanwords.json (formati origjinal).
    const SAMPLE: &str = r#"{
  "abandonoj": { "alternatives": [ "braktis" ], "origin": "english", "word": "abandon" },
  "bekgraund": { "alternatives": [ "sfond", "prapaskenë", "prapavijë" ], "origin": "english", "word": "background" },
  "adekuat": { "alternatives": [ "i përshtatshëm", "i mjaftueshem" ], "origin": "italian", "word": "adeguato" },
  "bosh": { "alternatives": [], "origin": "ottoman", "word": "boş" }
}"#;

    #[test]
    fn json_to_rows() {
        let rows = to_tsv_rows(SAMPLE.as_bytes()).unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0], "abandonoj\tbraktis\tenglish\tabandon");
        assert!(rows.contains(&"adekuat\ti përshtatshëm\titalian\tadeguato".to_string()));
        assert!(to_tsv_rows(b"[]").is_err());
    }

    #[test]
    fn atom_first_commit() {
        let atom = "<feed><updated>2025-01-28T18:48:20Z</updated><entry>\n<id>tag:github.com,2008:Grit::Commit/98ed839ac863701873a603a0af94670c71724d87</id>\n<updated>2025-01-28T18:48:20Z</updated></entry><entry><id>tag:github.com,2008:Grit::Commit/b939bc7814b4cb147a169f56f204840444d9013d</id></entry></feed>";
        assert_eq!(
            first_commit(atom),
            Some(("98ed839ac863701873a603a0af94670c71724d87".to_string(), "2025-01-28T18:48:20Z".to_string()))
        );
        assert_eq!(first_commit("<feed></feed>"), None);
    }

    #[test]
    fn blob_sha1_matches_git() {
        // `git hash-object` i një skedari bosh.
        assert_eq!(git_blob_sha1(b""), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }
}
