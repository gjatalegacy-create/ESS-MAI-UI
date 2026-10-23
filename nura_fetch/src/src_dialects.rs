//! FjaloriShqip — format dialektore të Kristoforidhit (1904), `dialekte/kristoforidhi-1904.tsv`.
//!
//! Licencat: të dhënat CC BY-SA 4.0 sipas `DATA-LICENSE.md`, kodi MIT sipas `LICENSE`
//! (të dyja shkarkohen dhe verifikohen para të dhënave).
//!
//! PORTA E PREJARDHJES (rregulli i pronarit: asnjë përmbajtje e shkruar nga AI):
//! para çdo shkarkimi të të dhënave lexohet burimi Atom i commit-eve të skedarit. Nëse ndonjë
//! commit mban trailer `Co-Authored-By:` me një model AI (Claude, GPT/ChatGPT, Copilot,
//! Gemini, Codex, Grok…), burimi REFUZOHET dhe nuk shkruhet asgjë. Më 2026-10-22 të gjitha
//! commit-et e kontrolluara të këtij skedari (transkriptimi nga alfabeti grek, përputhja me
//! formën standarde dhe përkthimi) kishin «Co-Authored-By: Claude Opus 5.5», prandaj
//! `fetch` kthen gabim të shpjeguar.
//!
//! Kur porta kalon: merren VETËM kolonat forma ⇥ dialekti ⇥ klasa ⇥ gjinia ⇥ forma standarde;
//! kuptimi anglisht, faqja dhe krahina nuk merren; `pershkrime.tsv` dhe Gazulli jo.
use crate::common::{self, Fetched};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::path::Path;

const REPO: &str = "EndritRushiti/FjaloriShqip";
const BRANCH: &str = "main";
const FILE: &str = "dialekte/kristoforidhi-1904.tsv";
const REL_TSV: &str = "raw/dialects/dialects-sq.tsv";
const LICENSE: &str = "CC BY-SA 4.0 (të dhënat, DATA-LICENSE.md); MIT (kodi, LICENSE)";
const ATTRIBUTION: &str = "FjaloriShqip (EndritRushiti/FjaloriShqip) — Konstantin Kristoforidhi, Lexikon tis albanikis glossis (Athinë 1904)";
const AI_NAMES: [&str; 10] = ["claude", "anthropic", "gpt", "openai", "copilot", "gemini", "codex", "grok", "llama", "mistral"];

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

/// (commit-e në burim, commit-e me bashkautor AI).
pub fn ai_coauthored(atom: &str) -> (usize, usize) {
    let entries: Vec<String> = atom.split("<entry>").skip(1).map(|e| e.to_lowercase()).collect();
    let flagged = entries
        .iter()
        .filter(|e| {
            e.split("co-authored-by:")
                .skip(1)
                .any(|tail| AI_NAMES.iter().any(|n| tail.chars().take(80).collect::<String>().contains(n)))
        })
        .count();
    (entries.len(), flagged)
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
            false => Err(format!("dialects {path}: blob i botuesit {p} ≠ blob lokal {local}")),
        },
        None => Ok(format!(
            "git-blob-sha1 lokal {local}; API e GitHub-it nuk u arrit (kufi kërkesash?) — pa krahasim me botuesin"
        )),
    }
}

/// Vetëm kolonat 1–5: forma, dialekti, klasa, gjinia, forma standarde.
pub fn to_tsv_rows(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            match c.len() >= 5 {
                true => {
                    let f: Vec<String> = c[..5].iter().map(|s| common::clean(s)).collect();
                    match f[0].is_empty() {
                        true => None,
                        false => Some(f.join("\t")),
                    }
                }
                false => None,
            }
        })
        .collect()
}

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    // 0. Porta e prejardhjes.
    let feed_url = format!("https://github.com/{REPO}/commits/{BRANCH}/{FILE}.atom");
    let atom = String::from_utf8_lossy(&common::get(&feed_url)?).to_string();
    let (commit, date) = first_commit(&atom).ok_or_else(|| format!("dialects: commit-i nuk u gjet në {feed_url}"))?;
    let (seen, ai) = ai_coauthored(&atom);
    match (seen, ai) {
        (0, _) => return Err(format!("dialects: burimi Atom {feed_url} pa commit-e — prejardhja e paverifikueshme, ndalet")),
        (_, 0) => {}
        (s, a) => {
            return Err(format!(
                "dialects: REFUZUAR — {a} nga {s} commit-et e fundit të {FILE} (deri te {commit}, {date}) kanë «Co-Authored-By» me model AI; \
                 transkriptimi i Kristoforidhit është i shkruar me AI dhe rregulli i pronarit e ndalon. Asgjë nuk u shkruajt."
            ))
        }
    }
    let mut out = Vec::new();
    // 1. Licencat, të verifikuara para të dhënave.
    for (path, needle) in [("DATA-LICENSE.md", "CC BY-SA 4.0"), ("LICENSE", "MIT License")] {
        let url = format!("https://raw.githubusercontent.com/{REPO}/{commit}/{path}");
        let bytes = common::get(&url)?;
        match String::from_utf8_lossy(&bytes).contains(needle) {
            true => {}
            false => return Err(format!("dialects: {path} nuk përmban «{needle}» — ndalet")),
        }
        let check = publisher_check(path, &commit, &bytes)?;
        out.push(common::record(
            data,
            "dialects",
            &format!("raw/dialects/{path}"),
            &bytes,
            &url,
            &format!("{REPO}@{commit}"),
            LICENSE,
            ATTRIBUTION,
            &check,
            "FULL",
        )?);
    }
    // 2. Të dhënat.
    let url = format!("https://raw.githubusercontent.com/{REPO}/{commit}/{FILE}");
    let bytes = common::get(&url)?;
    let check = publisher_check(FILE, &commit, &bytes)?;
    let version = format!("{REPO}@{commit} ({date})");
    out.push(common::record(data, "dialects", "raw/dialects/kristoforidhi-1904.tsv", &bytes, &url, &version, LICENSE, ATTRIBUTION, &check, "FULL")?);
    let rows = to_tsv_rows(&String::from_utf8_lossy(&bytes));
    let mut tsv = format!(
        "# dialects-sq.tsv — {ATTRIBUTION}; {LICENSE}; {version}\n# kolonat: forma_dialektore\tdialekti\tklasa\tgjinia\tforma_standarde\n"
    );
    for r in &rows {
        tsv.push_str(r);
        tsv.push('\n');
    }
    out.push(common::record(data, "dialects", REL_TSV, tsv.as_bytes(), &url, &version, LICENSE, ATTRIBUTION, "nxjerrë nga kristoforidhi-1904.tsv (5 kolona)", "FULL")?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_five_columns() {
        let t = "# Columns: form\tdialect\n\
avlimend\ttosk\tnoun\tm\tavlëmend\tloom\t1\tτ\n\
agjnoj\tgheg\tverb\t\tagjëroj\tto fast\t2\tγ\n\
hutak\tgheg\tnoun\tm\t\tconfused, bewildered person\t481\tγ\n\
short\trow\n";
        let rows = to_tsv_rows(t);
        assert_eq!(rows, vec![
            "avlimend\ttosk\tnoun\tm\tavlëmend".to_string(),
            "agjnoj\tgheg\tverb\t\tagjëroj".to_string(),
            "hutak\tgheg\tnoun\tm\t".to_string(),
        ]);
        assert!(!rows.iter().any(|r| r.contains("loom")));
    }

    #[test]
    fn detects_ai_coauthor_trailer() {
        // Formë reale e burimit Atom të GitHub-it (përmbajtja e escape-uar si HTML).
        let ai = "<feed><entry><id>tag:github.com,2008:Grit::Commit/375b35da3fa4b8787885867ec4021fdc745b9249</id><updated>2026-10-15T08:56:19Z</updated>\
<content type=\"html\">&lt;pre&gt;Kristoforidhi 1904: last dictionary page (482), book complete\n\nCo-Authored-By: Claude Opus 5.5 &amp;lt;noreply@anthropic.com&amp;gt;&lt;/pre&gt;</content></entry>\
<entry><id>tag:github.com,2008:Grit::Commit/cc5e23873b62d18fc27066fe169aeb53c0d85182</id><content>&lt;pre&gt;Fix typo\n\nCo-authored-by: Jane Doe &amp;lt;jane@example.org&amp;gt;&lt;/pre&gt;</content></entry></feed>";
        assert_eq!(ai_coauthored(ai), (2, 1));
        assert_eq!(
            first_commit(ai).map(|c| c.0),
            Some("375b35da3fa4b8787885867ec4021fdc745b9249".to_string())
        );
        assert_eq!(ai_coauthored("<feed></feed>"), (0, 0));
    }
}
