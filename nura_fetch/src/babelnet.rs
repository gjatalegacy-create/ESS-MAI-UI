//! Shkarkuesi i BabelNet 5.3 përmes API-së HTTP (https://babelnet.io/v9).
//!
//! - Çelësi lexohet nga `data/babelnet.key` ose `BABELNET_KEY`; nuk shkruhet
//!   kurrë në log, raport ose SOURCES.lock.
//! - Kufiri ditor: 1 000 "babelcoins" (1 pyetje = 1). Ky shkarkues përdor si
//!   parazgjedhje 900 në ditë dhe e regjistron konsumin në `quota.tsv`.
//! - Çdo përgjigje ruhet e papërpunuar në `cache/`; shkarkimi vazhdon nga
//!   dita në ditë pa e ripyetur atë që është ruajtur.
//! - Radha: fjalët që UI-ja nuk i gjeti (`data/babelnet_requests.txt`), pastaj
//!   `seed.txt` (lemat shqipe të UD-së, më të shpeshtat para).
//! - Nga BabelNet merren si kuptime vetëm shqisat shqipe me `lemma.type =
//!   HIGH_QUALITY` dhe burim tjetër nga OMWN_SQ (AlbaNet e kemi drejtpërdrejt).
//!   Përkthimet automatike (`AUTOMATIC_TRANSLATION`) ruhen të shënuara, por
//!   importuesi nuk i përdor për të folur.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const BASE: &str = "https://babelnet.io/v9";
const MAX_SYNSETS_PER_WORD: usize = 3;
const MAX_HYPERNYMS_PER_SYNSET: usize = 2;

pub struct Opts {
    pub budget: u32,
}

struct Client {
    key: String,
    agent: ureq::Agent,
    calls: u32,
    budget_left: u32,
}

impl Client {
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<Option<Vec<u8>>, String> {
        match self.budget_left {
            0 => return Ok(None),
            _ => {}
        }
        let mut req = self.agent.get(&format!("{BASE}/{path}")).header("Accept-Encoding", "gzip");
        for (k, v) in query {
            req = req.query(*k, *v);
        }
        req = req.query("key", &self.key);
        self.calls += 1;
        self.budget_left -= 1;
        let mut resp = req.call().map_err(|e| format!("BabelNet {path}: {}", e.to_string().replace(&self.key, "***")))?;
        let mut out = Vec::new();
        resp.body_mut().as_reader().read_to_end(&mut out).map_err(|e| e.to_string())?;
        // Kufiri i tejkaluar kthehet si mesazh JSON, jo si gabim HTTP.
        let text = String::from_utf8_lossy(&out);
        match text.contains("daily requests limit") || text.contains("key is not valid") {
            true => {
                self.budget_left = 0;
                Err("BabelNet: çelësi i pavlefshëm ose kufiri ditor u arrit".into())
            }
            false => Ok(Some(out)),
        }
    }
}

fn hexname(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

fn safe_id(id: &str) -> String {
    id.replace(':', "_")
}

/// Data UTC (YYYY-MM-DD) pa varësi: algoritmi "civil from days".
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let z = secs / 86_400 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + match mp < 10 {
        true => 3,
        false => -9,
    };
    let y = yoe + era * 400 + match m <= 2 {
        true => 1,
        false => 0,
    };
    format!("{y:04}-{m:02}-{d:02}")
}

fn used_today(dir: &Path, day: &str) -> u32 {
    fs::read_to_string(dir.join("quota.tsv"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(d, _)| *d == day)
        .filter_map(|(_, n)| n.trim().parse::<u32>().ok())
        .sum()
}

fn read_key(data: &Path) -> Result<String, String> {
    match std::env::var("BABELNET_KEY") {
        Ok(k) => Ok(k.trim().to_string()),
        Err(_) => fs::read_to_string(data.join("babelnet.key"))
            .map(|k| k.trim().to_string())
            .map_err(|_| "mungon çelësi: vendose te data/babelnet.key ose BABELNET_KEY".to_string()),
    }
}

fn read_lines(p: &Path) -> Vec<String> {
    fs::read_to_string(p)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

pub struct Outcome {
    pub calls: u32,
    pub words_done: usize,
    pub words_pending: usize,
    pub tsv: PathBuf,
}

pub fn run(data: &Path, opts: &Opts) -> Result<Outcome, String> {
    let dir = data.join("raw").join("babelnet53");
    for sub in ["cache/senses", "cache/synsets", "cache/edges"] {
        fs::create_dir_all(dir.join(sub)).map_err(|e| e.to_string())?;
    }
    let day = today();
    let used = used_today(&dir, &day);
    let key = read_key(data)?;
    let mut c = Client {
        key,
        agent: ureq::Agent::config_builder().user_agent(crate::common::UA).build().into(),
        calls: 0,
        budget_left: opts.budget.saturating_sub(used),
    };
    println!("[babelnet] sot ({day}) janë përdorur {used} pyetje; mbeten {} nga kufiri i zgjedhur {}", c.budget_left, opts.budget);

    // Radha: kërkesat nga UI-ja, pastaj seed-i.
    let mut queue: Vec<String> = read_lines(&data.join("babelnet_requests.txt"));
    queue.extend(read_lines(&dir.join("seed.txt")));
    let mut seen = BTreeSet::new();
    queue.retain(|w| seen.insert(w.clone()));

    let mut done = 0usize;
    let mut error: Option<String> = None;
    for lemma in &queue {
        match c.budget_left {
            0 => break,
            _ => {}
        }
        let sp = dir.join("cache/senses").join(format!("{}.json", hexname(lemma)));
        let senses_json = match fs::read(&sp) {
            Ok(b) => b,
            Err(_) => match c.get("getSenses", &[("lemma", lemma), ("searchLang", "SQ")]) {
                Ok(Some(b)) => {
                    fs::write(&sp, &b).map_err(|e| e.to_string())?;
                    b
                }
                Ok(None) => break,
                Err(e) => {
                    error = Some(e);
                    break;
                }
            },
        };
        // Synset-et me shqisë shqipe të saktë që nuk vijnë nga AlbaNet.
        let v: Value = serde_json::from_slice(&senses_json).unwrap_or(Value::Null);
        let mut wanted: Vec<String> = Vec::new();
        for s in v.as_array().cloned().unwrap_or_default() {
            let p = &s["properties"];
            let quality = p["lemma"]["type"].as_str().unwrap_or("");
            let src = p["source"].as_str().unwrap_or("");
            let id = p["synsetID"]["id"].as_str().unwrap_or("").to_string();
            match (quality == "HIGH_QUALITY", src != "OMWN_SQ", id.is_empty(), wanted.contains(&id)) {
                (true, true, false, false) => wanted.push(id),
                _ => {}
            }
        }
        wanted.truncate(MAX_SYNSETS_PER_WORD);
        let mut complete = true;
        for id in &wanted {
            match fetch_synset(&mut c, &dir, id, true) {
                Ok(true) => {}
                Ok(false) => complete = false,
                Err(e) => {
                    error = Some(e);
                    complete = false;
                }
            }
            match error.is_some() {
                true => break,
                false => {}
            }
        }
        match (complete, error.is_some()) {
            (true, false) => done += 1,
            _ => break,
        }
    }
    // Konsumi i sotëm regjistrohet edhe kur ndalet nga gabimi.
    let mut q = fs::read_to_string(dir.join("quota.tsv")).unwrap_or_default();
    q.push_str(&format!("{day}\t{}\n", c.calls));
    fs::write(dir.join("quota.tsv"), q).map_err(|e| e.to_string())?;

    let tsv = build_tsv(&dir)?;
    let pending = queue
        .iter()
        .filter(|w| !dir.join("cache/senses").join(format!("{}.json", hexname(w))).exists())
        .count();
    match error {
        Some(e) => println!("[babelnet] u ndal: {e}"),
        None => {}
    }
    Ok(Outcome { calls: c.calls, words_done: done, words_pending: pending, tsv })
}

/// Merr synset-in (SQ+EN) dhe lidhjet e tij; për hiperonimet merr edhe etiketat SQ.
/// Kthen false kur buxheti mbaroi para se të plotësohej.
fn fetch_synset(c: &mut Client, dir: &Path, id: &str, with_edges: bool) -> Result<bool, String> {
    let syn_p = dir.join("cache/synsets").join(format!("{}.json", safe_id(id)));
    match syn_p.exists() {
        true => {}
        false => match c.get("getSynset", &[("id", id), ("targetLang", "SQ"), ("targetLang", "EN")])? {
            Some(b) => fs::write(&syn_p, b).map_err(|e| e.to_string())?,
            None => return Ok(false),
        },
    }
    match with_edges {
        false => return Ok(true),
        true => {}
    }
    let edge_p = dir.join("cache/edges").join(format!("{}.json", safe_id(id)));
    let edges = match fs::read(&edge_p) {
        Ok(b) => b,
        Err(_) => match c.get("getOutgoingEdges", &[("id", id)])? {
            Some(b) => {
                // Ruhen vetëm lidhjet që përdoren (përgjigja e plotë ka mijëra lidhje në shumë gjuhë).
                let v: Value = serde_json::from_slice(&b).unwrap_or(Value::Null);
                let kept: Vec<Value> = v
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|x| matches!(x["pointer"]["relationGroup"].as_str(), Some("HYPERNYM" | "HOLONYM" | "MERONYM" | "ANTONYM")))
                    .collect();
                let slim = serde_json::to_vec(&kept).map_err(|e| e.to_string())?;
                fs::write(&edge_p, &slim).map_err(|e| e.to_string())?;
                slim
            }
            None => return Ok(false),
        },
    };
    let v: Value = serde_json::from_slice(&edges).unwrap_or(Value::Null);
    let hypers: Vec<String> = v
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|e| e["pointer"]["relationGroup"].as_str() == Some("HYPERNYM"))
        .filter_map(|e| e["target"].as_str().map(|s| s.to_string()))
        .take(MAX_HYPERNYMS_PER_SYNSET)
        .collect();
    for h in hypers {
        match fetch_synset(c, dir, &h, false)? {
            true => {}
            false => return Ok(false),
        }
    }
    Ok(true)
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// Shndërron cache-in në një TSV të thjeshtë që importuesi pa varësi e lexon:
///   Q ⇥ lemë                                  (lema e pyetur)
///   S ⇥ bn:id ⇥ POS ⇥ lemë ⇥ burim ⇥ cilësi    (shqisë shqipe)
///   L ⇥ bn:id ⇥ gjuhë ⇥ lemë ⇥ burim           (etiketë e synset-it)
///   G ⇥ bn:id ⇥ gjuhë ⇥ burim ⇥ përkufizim
///   E ⇥ bn:id ⇥ relationGroup ⇥ emri ⇥ objektivi ⇥ gjuhë
///   T ⇥ bn:id ⇥ synsetType (CONCEPT / NAMED_ENTITY)
pub fn build_tsv(dir: &Path) -> Result<PathBuf, String> {
    let mut out = String::from("# BabelNet 5.3 — nxjerrje lokale nga cache i API-së (ui_ess_mai)\n");
    let mut names: Vec<PathBuf> = fs::read_dir(dir.join("cache/senses"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    names.sort();
    for p in names {
        let v: Value = serde_json::from_slice(&fs::read(&p).map_err(|e| e.to_string())?).unwrap_or(Value::Null);
        let lemma_hex = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let bytes: Vec<u8> = (0..lemma_hex.len() / 2)
            .filter_map(|i| u8::from_str_radix(&lemma_hex[2 * i..2 * i + 2], 16).ok())
            .collect();
        out.push_str(&format!("Q\t{}\n", clean(&String::from_utf8_lossy(&bytes))));
        for s in v.as_array().cloned().unwrap_or_default() {
            let p = &s["properties"];
            out.push_str(&format!(
                "S\t{}\t{}\t{}\t{}\t{}\n",
                p["synsetID"]["id"].as_str().unwrap_or(""),
                p["pos"].as_str().unwrap_or(""),
                clean(p["fullLemma"].as_str().unwrap_or("")),
                p["source"].as_str().unwrap_or(""),
                p["lemma"]["type"].as_str().unwrap_or("")
            ));
        }
    }
    let mut syns: Vec<PathBuf> = fs::read_dir(dir.join("cache/synsets"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    syns.sort();
    for p in syns {
        let id = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").replacen('_', ":", 1);
        let v: Value = serde_json::from_slice(&fs::read(&p).map_err(|e| e.to_string())?).unwrap_or(Value::Null);
        out.push_str(&format!("T\t{id}\t{}\n", v["synsetType"].as_str().unwrap_or("")));
        let mut seen = BTreeSet::new();
        for s in v["senses"].as_array().cloned().unwrap_or_default() {
            let pr = &s["properties"];
            let quality = pr["lemma"]["type"].as_str().unwrap_or("");
            let lang = pr["language"].as_str().unwrap_or("");
            let lemma = clean(pr["fullLemma"].as_str().unwrap_or(""));
            match (quality == "HIGH_QUALITY", seen.insert((lang.to_string(), lemma.to_lowercase()))) {
                (true, true) => out.push_str(&format!(
                    "L\t{id}\t{lang}\t{lemma}\t{}\n",
                    pr["source"].as_str().unwrap_or("")
                )),
                _ => {}
            }
        }
        // Çelësat e WordNet (senseKey) që vetë BabelNet i jep për shqisat angleze WN:
        // bëjnë të mundur lidhjen e deklaruar me synset-et e WordNet 3.0.
        let mut keys = BTreeSet::new();
        for s in v["senses"].as_array().cloned().unwrap_or_default() {
            let pr = &s["properties"];
            match (pr["language"].as_str(), pr["source"].as_str(), pr["senseKey"].as_str()) {
                (Some("EN"), Some("WN"), Some(k)) => {
                    keys.insert(k.to_string());
                }
                _ => {}
            }
        }
        for k in keys {
            out.push_str(&format!("K\t{id}\t{}\n", clean(&k)));
        }
        for g in v["glosses"].as_array().cloned().unwrap_or_default() {
            out.push_str(&format!(
                "G\t{id}\t{}\t{}\t{}\n",
                g["language"].as_str().unwrap_or(""),
                g["source"].as_str().unwrap_or(""),
                clean(g["gloss"].as_str().unwrap_or(""))
            ));
        }
        let ep = dir.join("cache/edges").join(p.file_name().unwrap_or_default());
        match fs::read(&ep) {
            Ok(b) => {
                let e: Value = serde_json::from_slice(&b).unwrap_or(Value::Null);
                // Vetëm lidhjet që përdoren për shprehje dhe që botuesi NUK i shënon
                // si automatike (`isAutomatic`); "instance_of" (entitete) përjashtohet.
                for x in e.as_array().cloned().unwrap_or_default().into_iter().filter(|x| {
                    let group = matches!(
                        x["pointer"]["relationGroup"].as_str(),
                        Some("HYPERNYM" | "HOLONYM" | "MERONYM" | "ANTONYM")
                    );
                    let manual = x["pointer"]["isAutomatic"].as_bool() == Some(false);
                    let not_instance = x["pointer"]["name"].as_str() != Some("instance_of");
                    group && manual && not_instance
                }) {
                    out.push_str(&format!(
                        "E\t{id}\t{}\t{}\t{}\t{}\n",
                        x["pointer"]["relationGroup"].as_str().unwrap_or(""),
                        clean(x["pointer"]["name"].as_str().unwrap_or("")),
                        x["target"].as_str().unwrap_or(""),
                        x["language"].as_str().unwrap_or("")
                    ));
                }
            }
            Err(_) => {}
        }
    }
    let tsv = dir.join("babelnet-sq.tsv");
    fs::write(&tsv, out).map_err(|e| e.to_string())?;
    Ok(tsv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_is_iso_date() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
    }

    #[test]
    fn hexname_roundtrip_is_stable() {
        assert_eq!(hexname("zjarr"), "7a6a617272");
        assert_eq!(safe_id("bn:00034625n"), "bn_00034625n");
    }

    #[test]
    fn tsv_keeps_quality_marker_and_skips_nothing_silently() {
        let dir = std::env::temp_dir().join("nura_bn_tsv_test");
        let _ = fs::remove_dir_all(&dir);
        for sub in ["cache/senses", "cache/synsets", "cache/edges"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let senses = r#"[{"type":"BabelSense","properties":{"fullLemma":"Zjarri","source":"WIKI","language":"SQ","pos":"NOUN","synsetID":{"id":"bn:00034625n","pos":"NOUN","source":"BABELNET"},"lemma":{"lemma":"Zjarri","type":"HIGH_QUALITY"}}},
{"type":"BabelSense","properties":{"fullLemma":"impetuousness","source":"WNTR","language":"SQ","pos":"NOUN","synsetID":{"id":"bn:00046118n","pos":"NOUN","source":"BABELNET"},"lemma":{"lemma":"impetuousness","type":"AUTOMATIC_TRANSLATION"}}}]"#;
        fs::write(dir.join("cache/senses").join(format!("{}.json", hexname("zjarr"))), senses).unwrap();
        let syn = r#"{"senses":[{"type":"BabelSense","properties":{"fullLemma":"Zjarri","source":"WIKI","language":"SQ","lemma":{"lemma":"Zjarri","type":"HIGH_QUALITY"}}}],"glosses":[{"source":"WIKI","language":"EN","gloss":"Fire is the rapid oxidation of a material."}]}"#;
        fs::write(dir.join("cache/synsets/bn_00034625n.json"), syn).unwrap();
        let edges = r#"[{"language":"EN","pointer":{"fSymbol":"@","name":"hypernym","shortName":"is-a","relationGroup":"HYPERNYM","isAutomatic":false},"target":"bn:00030879n","weight":0.0,"normalizedWeight":0.0}]"#;
        fs::write(dir.join("cache/edges/bn_00034625n.json"), edges).unwrap();
        let t = fs::read_to_string(build_tsv(&dir).unwrap()).unwrap();
        assert!(t.contains("Q\tzjarr\n"));
        assert!(t.contains("S\tbn:00034625n\tNOUN\tZjarri\tWIKI\tHIGH_QUALITY\n"));
        assert!(t.contains("S\tbn:00046118n\tNOUN\timpetuousness\tWNTR\tAUTOMATIC_TRANSLATION\n"));
        assert!(t.contains("L\tbn:00034625n\tSQ\tZjarri\tWIKI\n"));
        assert!(t.contains("G\tbn:00034625n\tEN\tWIKI\tFire is the rapid oxidation of a material.\n"));
        assert!(t.contains("E\tbn:00034625n\tHYPERNYM\thypernym\tbn:00030879n\tEN\n"));
        let _ = fs::remove_dir_all(&dir);
    }
}
