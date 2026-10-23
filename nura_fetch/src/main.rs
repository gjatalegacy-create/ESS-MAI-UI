//! nura_fetch — shkarkuesi i burimeve gjuhësore për ui_ess_mai.
//!
//! Rregullat:
//! - shkarkon vetëm nga burimet zyrtare të deklaruara më poshtë;
//! - ruan URL-në, versionin (commit-in e fiksuar), licencën, atribuimin,
//!   madhësinë dhe SHA-256 lokale;
//! - kontrolli i botuesit bëhet vetëm aty ku botuesi jep një identifikues:
//!   për GitHub është SHA-1 i objektit git ("blob"), që llogaritet mbi bajtet e
//!   shkarkuara dhe krahasohet me atë që publikon API-ja. SHA-256 lokale NUK
//!   paraqitet si provë autenticiteti nga botuesi;
//! - ConceptNet i plotë (≈498 MB) nuk shkarkohet pa flamurin e qartë
//!   `--conceptnet-full`; si parazgjedhje merret vetëm një prefiks i deklaruar.
//!
//! Stil: pa if/else — vetëm match (disiplina e ESS-MAI).

mod babelnet;
mod common;
mod src_lexin;
mod src_uniparser;
mod src_kaikki_en;
mod src_kaikki_de;
mod src_eurovoc;
mod src_wd_lexemes;
mod src_loanwords;
mod src_gazulli;
mod gazulli_parse;
mod src_dialects;
mod src_conceptnet_en;
mod src_meyer;

use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

// Asnjë e dhënë personale në kërkesa: i njëjti identifikues neutral si te common::UA.
const UA: &str = common::UA;
const CONCEPTNET_URL: &str =
    "https://s3.amazonaws.com/conceptnet/downloads/2019/edges/conceptnet-assertions-5.7.0.csv.gz";
const CONCEPTNET_PREFIX_BYTES: u64 = 8 * 1024 * 1024;

struct GitHubFile {
    repo: &'static str,
    branch: &'static str,
    path: &'static str,
}

struct LockRow {
    source: String,
    file: String,
    url: String,
    version: String,
    license: String,
    attribution: String,
    bytes: u64,
    sha256: String,
    publisher_check: String,
    status: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = data_root();
    let full_conceptnet = args.iter().any(|a| a == "--conceptnet-full");
    let only: Option<String> = args
        .iter()
        .position(|a| a == "--only")
        .and_then(|i| args.get(i + 1).cloned());
    match args.iter().any(|a| a == "--babelnet") {
        true => {
            let budget: u32 = args
                .iter()
                .position(|a| a == "--budget")
                .and_then(|i| args.get(i + 1))
                .and_then(|b| b.parse().ok())
                .unwrap_or(900)
                .min(1000);
            match run_babelnet(&root, budget) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("[nura_fetch] GABIM BabelNet: {e}");
                    std::process::exit(2);
                }
            }
            return;
        }
        false => {}
    }
    match run(&root, full_conceptnet, only.as_deref()) {
        Ok(rows) => {
            println!(
                "\n[nura_fetch] {} skedarë të regjistruar në {}",
                rows,
                root.join("SOURCES.lock").display()
            );
        }
        Err(e) => {
            eprintln!("[nura_fetch] GABIM: {e}");
            std::process::exit(2);
        }
    }
}

fn data_root() -> PathBuf {
    match std::env::var("UI_ESS_MAI_DATA") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("data"),
    }
}

fn run(root: &Path, full_conceptnet: bool, only: Option<&str>) -> Result<usize, String> {
    let raw = root.join("raw");
    fs::create_dir_all(&raw).map_err(|e| e.to_string())?;
    let mut rows: Vec<LockRow> = Vec::new();
    let want = |id: &str| only.map(|o| o == id).unwrap_or(true);

    match want("pwn30") {
        true => rows.extend(fetch_pwn30(&raw.join("pwn30"))?),
        false => {}
    }
    match want("omw_als") {
        true => rows.extend(fetch_github_set(
            &raw.join("omw_als"),
            "omw_als",
            &[
                GitHubFile { repo: "omwn/omw-data", branch: "main", path: "wns/als/wn-data-als.tab" },
                GitHubFile { repo: "omwn/omw-data", branch: "main", path: "wns/als/LICENSE" },
                GitHubFile { repo: "omwn/omw-data", branch: "main", path: "wns/als/citation.bib" },
                GitHubFile { repo: "omwn/omw-data", branch: "main", path: "wns/als/README" },
            ],
            "CC BY 3.0 (ose GPL 3, licencë e dyfishtë sipas wns/als/LICENSE)",
            "Albanet — Ervin Ruci (2008), University of Vlora; shpërndarë nga Open Multilingual Wordnet (omwn/omw-data)",
        )?),
        false => {}
    }
    match want("ud_sq_tsa") {
        true => rows.extend(fetch_github_set(
            &raw.join("ud_sq_tsa"),
            "ud_sq_tsa",
            &[
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-TSA", branch: "master", path: "sq_tsa-ud-test.conllu" },
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-TSA", branch: "master", path: "LICENSE.txt" },
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-TSA", branch: "master", path: "README.md" },
            ],
            "CC BY-SA 4.0",
            "UD_Albanian-TSA — Universal Dependencies",
        )?),
        false => {}
    }
    match want("ud_sq_staf") {
        true => rows.extend(fetch_github_set(
            &raw.join("ud_sq_staf"),
            "ud_sq_staf",
            &[
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-STAF", branch: "master", path: "sq_staf-ud-train.conllu" },
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-STAF", branch: "master", path: "sq_staf-ud-dev.conllu" },
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-STAF", branch: "master", path: "sq_staf-ud-test.conllu" },
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-STAF", branch: "master", path: "LICENSE.txt" },
                GitHubFile { repo: "UniversalDependencies/UD_Albanian-STAF", branch: "master", path: "README.md" },
            ],
            "CC BY-SA 4.0",
            "UD_Albanian-STAF — Universal Dependencies",
        )?),
        false => {}
    }
    match only == Some("lexin") {
        true => rows.extend(src_lexin::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("uniparser") {
        true => rows.extend(src_uniparser::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("kaikki_en") {
        true => rows.extend(src_kaikki_en::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("kaikki_de") {
        true => rows.extend(src_kaikki_de::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("eurovoc") {
        true => rows.extend(src_eurovoc::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("wd_lexemes") {
        true => rows.extend(src_wd_lexemes::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("loanwords") {
        true => rows.extend(src_loanwords::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("gazulli1941") {
        true => rows.extend(src_gazulli::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("dialects") {
        true => rows.extend(src_dialects::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("conceptnet_en") {
        true => rows.extend(src_conceptnet_en::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match only == Some("meyer1891") {
        true => rows.extend(src_meyer::fetch(root)?.into_iter().map(from_fetched)),
        false => {}
    }
    match want("conceptnet57") {
        true => rows.extend(fetch_conceptnet(&raw.join("conceptnet57"), full_conceptnet)?),
        false => {}
    }

    write_lock(root, &rows)?;
    Ok(rows.len())
}

// ───────────────────────────── BabelNet ─────────────────────────────

/// Radha fillestare: lemat e fjalëve të përmbajtjes në UD shqip, sipas shpeshtësisë.
fn build_seed(root: &Path) -> Result<(), String> {
    let seed = root.join("raw").join("babelnet53").join("seed.txt");
    match seed.exists() {
        true => return Ok(()),
        false => {}
    }
    let mut freq: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for f in [
        "raw/ud_sq_tsa/sq_tsa-ud-test.conllu",
        "raw/ud_sq_staf/sq_staf-ud-train.conllu",
        "raw/ud_sq_staf/sq_staf-ud-dev.conllu",
        "raw/ud_sq_staf/sq_staf-ud-test.conllu",
    ] {
        let t = fs::read_to_string(root.join(f)).map_err(|e| format!("{f}: {e}"))?;
        for l in t.lines() {
            let c: Vec<&str> = l.split('\t').collect();
            match (c.len() == 10, c.get(3).copied()) {
                (true, Some("NOUN" | "VERB" | "ADJ" | "ADV")) => {
                    *freq.entry(c[2].to_lowercase()).or_default() += 1;
                }
                _ => {}
            }
        }
    }
    let mut v: Vec<(String, u32)> = freq.into_iter().filter(|(w, _)| w.chars().count() > 2).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let text = v.iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>().join("\n");
    fs::create_dir_all(seed.parent().unwrap_or(root)).map_err(|e| e.to_string())?;
    fs::write(&seed, format!("# lemat shqipe të UD (TSA+STAF), sipas shpeshtësisë\n{text}\n")).map_err(|e| e.to_string())
}

fn run_babelnet(root: &Path, budget: u32) -> Result<(), String> {
    fs::create_dir_all(root.join("raw").join("babelnet53")).map_err(|e| e.to_string())?;
    build_seed(root)?;
    let o = babelnet::run(root, &babelnet::Opts { budget })?;
    let bytes = fs::read(&o.tsv).map_err(|e| e.to_string())?;
    println!(
        "[babelnet] pyetje këtë herë: {}; fjalë të plotësuara: {}; në radhë: {}",
        o.calls, o.words_done, o.words_pending
    );
    // Shenjë ASCII për skriptin e natës: radha mbaroi.
    match o.words_pending {
        0 => println!("BABELNET_QUEUE_EMPTY"),
        _ => {}
    }
    write_lock(
        root,
        &[LockRow {
            source: "babelnet53".into(),
            file: "raw/babelnet53/babelnet-sq.tsv".into(),
            url: "https://babelnet.io/v9 (getSenses, getSynset, getOutgoingEdges)".into(),
            version: "BabelNet 5.3 (V5_3)".into(),
            license: "BabelNet Non-Commercial License (babelnet.org/full-license) — vetëm përdorim kërkimor jo-tregtar; mos e shpërndaj".into(),
            attribution: "BabelNet 5.3 — Navigli & Ponzetto (2012), babelnet.org; burimet: WIKI, WIKIDATA, WIKT, OMWIKI, WN/OEWN".into(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            publisher_check: "NUK KA — API; nxjerrje lokale nga përgjigjet e ruajtura".into(),
            status: format!("PARTIAL_API_CACHE (në radhë: {})", o.words_pending),
        }],
    )
}

fn from_fetched(f: common::Fetched) -> LockRow {
    LockRow {
        source: f.source,
        file: f.file,
        url: f.url,
        version: f.version,
        license: f.license,
        attribution: f.attribution,
        bytes: f.bytes,
        sha256: f.sha256,
        publisher_check: f.publisher_check,
        status: f.status,
    }
}

// ───────────────────────────── HTTP ─────────────────────────────

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().user_agent(UA).build().into()
}

fn http_get(url: &str, range: Option<(u64, u64)>) -> Result<Vec<u8>, String> {
    let ag = agent();
    let req = ag.get(url);
    let req = match range {
        Some((a, b)) => req.header("Range", &format!("bytes={a}-{b}")),
        None => req,
    };
    let mut resp = req.call().map_err(|e| format!("GET {url}: {e}"))?;
    let mut out = Vec::new();
    resp.body_mut()
        .as_reader()
        .read_to_end(&mut out)
        .map_err(|e| format!("lexim {url}: {e}"))?;
    Ok(out)
}

fn http_head_len(url: &str) -> Result<Option<u64>, String> {
    let ag = agent();
    let resp = ag.head(url).call().map_err(|e| format!("HEAD {url}: {e}"))?;
    Ok(resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok()))
}

// ───────────────────────────── hash ─────────────────────────────

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-1 i objektit git: sha1("blob <len>\0" + përmbajtja).
fn git_blob_sha1(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(format!("blob {}\0", bytes.len()).as_bytes());
    h.update(bytes);
    hex(&h.finalize())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Nxjerr vlerën e parë të `"key":"value"` nga JSON i GitHub-it pa varësi JSON.
fn json_first_string(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let start = json.find(&pat)? + pat.len();
    let rest = &json[start..];
    let q1 = rest.find('"')? + 1;
    let q2 = rest[q1..].find('"')? + q1;
    Some(rest[q1..q2].to_string())
}

// ───────────────────────────── GitHub ─────────────────────────────

fn fetch_github_set(
    dir: &Path,
    source: &str,
    files: &[GitHubFile],
    license: &str,
    attribution: &str,
) -> Result<Vec<LockRow>, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for f in files {
        // 1. Fiksohet commit-i i fundit që preku skedarin (versioni i saktë).
        let commits_url = format!(
            "https://api.github.com/repos/{}/commits?sha={}&path={}&per_page=1",
            f.repo, f.branch, f.path
        );
        let commits = String::from_utf8_lossy(&http_get(&commits_url, None)?).to_string();
        let commit = json_first_string(&commits, "sha")
            .ok_or_else(|| format!("commit-i për {} nuk u gjet", f.path))?;
        // 2. Identifikuesi i botuesit: blob SHA-1 në atë commit.
        let meta_url = format!(
            "https://api.github.com/repos/{}/contents/{}?ref={}",
            f.repo, f.path, commit
        );
        let meta = String::from_utf8_lossy(&http_get(&meta_url, None)?).to_string();
        let published_blob = json_first_string(&meta, "sha")
            .ok_or_else(|| format!("blob sha për {} mungon", f.path))?;
        // 3. Bajtet nga URL-ja e fiksuar te commit-i.
        let raw_url = format!("https://raw.githubusercontent.com/{}/{}/{}", f.repo, commit, f.path);
        print!("[nura_fetch] {source}: {} … ", f.path);
        let _ = std::io::stdout().flush();
        let bytes = http_get(&raw_url, None)?;
        let local_blob = git_blob_sha1(&bytes);
        let check = match local_blob == published_blob {
            true => format!("git-blob-sha1 PËRPUTHET ({published_blob})"),
            false => {
                return Err(format!(
                    "{}: blob i botuesit {published_blob} ≠ blob lokal {local_blob}",
                    f.path
                ))
            }
        };
        let name = f.path.rsplit('/').next().unwrap_or(f.path);
        fs::write(dir.join(name), &bytes).map_err(|e| e.to_string())?;
        println!("{} B, {}", bytes.len(), check);
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or(source);
        rows.push(LockRow {
            source: source.to_string(),
            file: format!("raw/{dir_name}/{name}"),
            url: raw_url,
            version: format!("{}@{}", f.repo, commit),
            license: license.to_string(),
            attribution: attribution.to_string(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            publisher_check: check,
            status: "FULL".to_string(),
        });
    }
    Ok(rows)
}

// ───────────────────────────── WordNet 3.0 ─────────────────────────────

fn fetch_pwn30(dir: &Path) -> Result<Vec<LockRow>, String> {
    let url = "https://wordnetcode.princeton.edu/3.0/WNdb-3.0.tar.gz";
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    print!("[nura_fetch] pwn30: WNdb-3.0.tar.gz … ");
    let _ = std::io::stdout().flush();
    let bytes = http_get(url, None)?;
    println!("{} B", bytes.len());
    let archive_name = "WNdb-3.0.tar.gz";
    fs::write(dir.join(archive_name), &bytes).map_err(|e| e.to_string())?;
    let license = "WordNet 3.0 License (Princeton University) — leje përdorimi/kopjimi me ruajtjen e njoftimit të të drejtave";
    let attribution = "Princeton University \"About WordNet.\" WordNet. Princeton University. 2010.";
    let mut rows = vec![LockRow {
        source: "pwn30".into(),
        file: format!("raw/pwn30/{archive_name}"),
        url: url.into(),
        version: "WordNet 3.0 (WNdb-3.0, Last-Modified 2007-05-07)".into(),
        license: license.into(),
        attribution: attribution.into(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        publisher_check: "NUK KA — Princeton nuk publikon checksum; vetëm SHA-256 lokale".into(),
        status: "FULL".into(),
    }];
    // Nxjerrja e dict/ (data.*, index.*, LICENSE) nga arkivi.
    let gz = flate2::read::GzDecoder::new(&bytes[..]);
    let mut ar = tar::Archive::new(gz);
    let entries = ar.entries().map_err(|e| e.to_string())?;
    for entry in entries {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let keep = matches!(
            name.as_str(),
            "data.noun" | "data.verb" | "data.adj" | "data.adv" | "index.noun" | "index.verb"
                | "index.adj" | "index.adv" | "index.sense" | "LICENSE"
        );
        match keep {
            true => {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                fs::write(dir.join(&name), &buf).map_err(|e| e.to_string())?;
                rows.push(LockRow {
                    source: "pwn30".into(),
                    file: format!("raw/pwn30/{name}"),
                    url: format!("{url}#{}", path.display()),
                    version: "WordNet 3.0".into(),
                    license: license.into(),
                    attribution: attribution.into(),
                    bytes: buf.len() as u64,
                    sha256: sha256_hex(&buf),
                    publisher_check: "nxjerrë nga arkivi më sipër (pa checksum botuesi)".into(),
                    status: "EXTRACTED".into(),
                });
            }
            false => {}
        }
    }
    Ok(rows)
}

// ───────────────────────────── ConceptNet 5.7 ─────────────────────────────

fn fetch_conceptnet(dir: &Path, full: bool) -> Result<Vec<LockRow>, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let total = http_head_len(CONCEPTNET_URL)?.unwrap_or(0);
    println!(
        "[nura_fetch] conceptnet57: arkivi zyrtar i plotë = {total} B (≈{} MB), gzip i pandashëm",
        total / 1_000_000
    );
    let (bytes, status) = match full {
        true => (http_get(CONCEPTNET_URL, None)?, "FULL"),
        false => (
            http_get(CONCEPTNET_URL, Some((0, CONCEPTNET_PREFIX_BYTES - 1)))?,
            "PARTIAL_PREFIX",
        ),
    };
    println!("[nura_fetch] conceptnet57: u morën {} B ({status})", bytes.len());
    let gz_name = match full {
        true => "conceptnet-assertions-5.7.0.csv.gz",
        false => "conceptnet-assertions-5.7.0.prefix-8MiB.csv.gz",
    };
    fs::write(dir.join(gz_name), &bytes).map_err(|e| e.to_string())?;

    // Dekompresim i rrjedhës deri ku mbërrijnë bajtet; ruhen vetëm rreshtat e
    // plotë ku të paktën një skaj është /c/sq/ (filtër i deklaruar).
    let gz = flate2::read::GzDecoder::new(&bytes[..]);
    let mut reader = BufReader::new(gz);
    let mut out = Vec::new();
    let mut lines_total: u64 = 0;
    let mut lines_sq: u64 = 0;
    let mut last_rel = String::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => match line.last() {
                Some(b'\n') => {
                    lines_total += 1;
                    let s = String::from_utf8_lossy(&line);
                    let mut cols = s.split('\t');
                    let _uri = cols.next();
                    let rel = cols.next().unwrap_or("").to_string();
                    let a = cols.next().unwrap_or("");
                    let b = cols.next().unwrap_or("");
                    let is_sq = a.starts_with("/c/sq/") || b.starts_with("/c/sq/");
                    last_rel = rel;
                    match is_sq {
                        true => {
                            lines_sq += 1;
                            out.extend_from_slice(&line);
                        }
                        false => {}
                    }
                }
                _ => break, // rresht i prerë nga fundi i prefiksit — hidhet
            },
            Err(_) => break, // fundi i prefiksit (gzip i paplotë) — ndalim i pritur
        }
    }
    let extract_name = "conceptnet-5.7.0.sq-edges.csv";
    fs::write(dir.join(extract_name), &out).map_err(|e| e.to_string())?;
    let scope = match full {
        true => "arkiv",
        false => "prefiks",
    };
    let coverage = format!(
        "rreshta të plotë në {scope}: {lines_total}; me /c/sq/: {lines_sq}; relacioni i fundit i arritur: {last_rel}"
    );
    println!("[nura_fetch] conceptnet57: {coverage}");
    fs::write(
        dir.join("COVERAGE.txt"),
        format!("{coverage}\narkivi i plotë: {total} B\nstatus: {status}\n"),
    )
    .map_err(|e| e.to_string())?;
    let license = "CC BY-SA 4.0 (për çdo skaj, licenca e saktë në kolonën JSON 'license')";
    let attribution =
        "ConceptNet 5.7 — Speer, Chin & Havasi (2017), commonsense/conceptnet5; skajet sq vijnë nga Wiktionary";
    Ok(vec![
        LockRow {
            source: "conceptnet57".into(),
            file: format!("raw/conceptnet57/{gz_name}"),
            url: CONCEPTNET_URL.into(),
            version: "ConceptNet 5.7.0 assertions (Last-Modified 2019-07-03)".into(),
            license: license.into(),
            attribution: attribution.into(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            publisher_check: "NUK KA — ETag i S3 për ngarkim me shumë pjesë nuk është checksum i skedarit".into(),
            status: status.into(),
        },
        LockRow {
            source: "conceptnet57".into(),
            file: format!("raw/conceptnet57/{extract_name}"),
            url: format!("{CONCEPTNET_URL}#filter=/c/sq/"),
            version: "ConceptNet 5.7.0 — nxjerrje lokale".into(),
            license: license.into(),
            attribution: attribution.into(),
            bytes: out.len() as u64,
            sha256: sha256_hex(&out),
            publisher_check: format!("derivuar lokalisht; {coverage}"),
            status: format!("{status}_FILTERED_SQ"),
        },
    ])
}

// ───────────────────────────── lock ─────────────────────────────

fn write_lock(root: &Path, new_rows: &[LockRow]) -> Result<(), String> {
    let path = root.join("SOURCES.lock");
    // Ruan rreshtat e burimeve që nuk u rishkarkuan këtë herë.
    let fresh: std::collections::BTreeSet<&str> = new_rows.iter().map(|r| r.source.as_str()).collect();
    let old = fs::read_to_string(&path).unwrap_or_default();
    let mut text = String::from(
        "# ui_ess_mai — SOURCES.lock\n# kolonat: source\tfile\turl\tversion\tlicense\tattribution\tbytes\tsha256_local\tpublisher_check\tstatus\n",
    );
    for l in old.lines() {
        let src = l.split('\t').next().unwrap_or("");
        match l.starts_with('#') || l.trim().is_empty() || fresh.contains(src) {
            true => {}
            false => {
                text.push_str(l);
                text.push('\n');
            }
        }
    }
    for r in new_rows {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.source, r.file, r.url, r.version, r.license, r.attribution, r.bytes, r.sha256,
            r.publisher_check, r.status
        ));
    }
    fs::write(&path, text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_blob_sha1_matches_git_definition() {
        // `printf 'hello\n' | git hash-object --stdin`
        assert_eq!(git_blob_sha1(b"hello\n"), "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn json_first_string_reads_sha() {
        let j = r#"[{"sha": "abc123", "node_id": "x"}]"#;
        assert_eq!(json_first_string(j, "sha").as_deref(), Some("abc123"));
    }
}
