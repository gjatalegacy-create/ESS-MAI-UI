//! Lexemat shqipe të Wikidata (gjuha Q8748), CC0 1.0, nëpërmjet Wikidata Query Service.
//!
//! - Pikë hyrjeje: https://query.wikidata.org/sparql (User-Agent = `common::UA`, pa email).
//! - Tri pyetje: (1) lexemat: lema shqip (@sq), kategoria leksikore, gjinia P5185;
//!   (2) format: përfaqësimi @sq dhe tiparet gramatikore (QID);
//!   (3) P5238 «combines lexemes»: përbërësit me lemën e tyre shqip dhe renditjen P1545.
//! - P5191 (kryesisht etimologji) dhe glosat NUK merren.
//! - Përgjigjet ruhen të papërpunuara (JSON), pyetjet në `.rq`, nxjerrja në TSV me
//!   QID-të siç janë; përkthimi QID → tipare UD bëhet në importues (nura_lingua).
//! - Asnjë skedar nuk shkruhet para se të tria pyetjet të kenë sukses (pa gjysmë-shkarkime
//!   pa rresht në SOURCES.lock). Kur WDQS kthen 429/5xx, pritet gjithnjë e më gjatë
//!   (15 s → 45 s → 90 s → 180 s → 300 s) para provës tjetër.
use crate::common::{self, Fetched};
use serde_json::Value;
use std::path::Path;

pub const ENDPOINT: &str = "https://query.wikidata.org/sparql";
const ENTITY: &str = "http://www.wikidata.org/entity/";
const REL_TSV: &str = "raw/wd_lexemes/wd_lexemes-sq.tsv";
const REL_RQ: &str = "raw/wd_lexemes/wd_lexemes-sq.rq";
const LICENSE: &str = "CC0 1.0 (Wikidata, të dhëna strukturore)";
const ATTRIBUTION: &str = "Wikidata Lexemes (Albanian, Q8748) — Wikidata contributors";

pub const Q_LEXEMES: &str = r#"SELECT ?l ?lemma ?cat (GROUP_CONCAT(DISTINCT STR(?g); separator=",") AS ?genders) WHERE {
  ?l dct:language wd:Q8748 ; wikibase:lemma ?lemma ; wikibase:lexicalCategory ?cat .
  FILTER(lang(?lemma) = "sq")
  OPTIONAL { ?l wdt:P5185 ?g }
} GROUP BY ?l ?lemma ?cat"#;

pub const Q_FORMS: &str = r#"SELECT ?l ?form ?rep (GROUP_CONCAT(DISTINCT STR(?f); separator=",") AS ?feats) WHERE {
  ?l dct:language wd:Q8748 ; ontolex:lexicalForm ?form .
  ?form ontolex:representation ?rep .
  FILTER(lang(?rep) = "sq")
  OPTIONAL { ?form wikibase:grammaticalFeature ?f }
} GROUP BY ?l ?form ?rep"#;

pub const Q_COMPONENTS: &str = r#"SELECT ?l ?c ?cl ?ord WHERE {
  ?l dct:language wd:Q8748 ; p:P5238 ?st .
  ?st ps:P5238 ?c .
  OPTIONAL { ?st pq:P1545 ?ord }
  OPTIONAL { ?c wikibase:lemma ?cl . FILTER(lang(?cl) = "sq") }
}"#;

/// Data UTC (YYYY-MM-DD) pa varësi: algoritmi "civil from days".
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let z = secs / 86_400 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = match mp < 10 {
        true => mp + 3,
        false => mp - 9,
    };
    let y = yoe + era * 400 + match m <= 2 {
        true => 1,
        false => 0,
    };
    format!("{y:04}-{m:02}-{d:02}")
}

fn val<'a>(b: &'a Value, k: &str) -> &'a str {
    b.get(k).and_then(|x| x.get("value")).and_then(|v| v.as_str()).unwrap_or("")
}

fn short(uri: &str) -> String {
    uri.trim().strip_prefix(ENTITY).unwrap_or(uri.trim()).to_string()
}

fn qids(s: &str) -> String {
    let mut v: Vec<String> = s.split(',').map(short).filter(|x| !x.is_empty()).collect();
    v.sort();
    v.dedup();
    v.join(",")
}

/// Pritjet para provave të përsëritura (sekonda) kur WDQS kufizon ritmin.
const BACKOFF: [u64; 5] = [15, 45, 90, 180, 300];

/// Gabim i përkohshëm i WDQS (kufizim ritmi ose mbingarkesë) që ia vlen të riprovohet.
fn transient(e: &str) -> bool {
    ["429", "500", "502", "503", "504", "timed out", "timeout"].iter().any(|k| e.contains(k))
}

/// GET me pritje kur WDQS kufizon ritmin (HTTP 429) ose është i mbingarkuar (5xx).
/// `common::get_with` nuk e jep kokën Retry-After, ndaj pritjet janë fikse dhe të gjata.
fn get_patient(q: &str, accept: Option<&str>) -> Result<Vec<u8>, String> {
    let first = common::get_with(ENDPOINT, &[("query", q)], accept);
    BACKOFF.iter().enumerate().fold(first, |res, (i, wait)| match res {
        Ok(b) => Ok(b),
        Err(e) => match transient(&e) {
            true => {
                eprintln!(
                    "[nura_fetch] wd_lexemes: {e} — prova {}/{} pas {wait} s",
                    i + 2,
                    BACKOFF.len() + 1
                );
                std::thread::sleep(std::time::Duration::from_secs(*wait));
                common::get_with(ENDPOINT, &[("query", q)], accept)
            }
            false => Err(e),
        },
    })
}

fn bindings(json: &[u8], what: &str) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_slice(json).map_err(|e| format!("wd_lexemes {what}: JSON i pavlefshëm: {e}"))?;
    v.get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .cloned()
        .ok_or_else(|| format!("wd_lexemes {what}: mungon results.bindings"))
}

/// `L ⇥ lexeme ⇥ lema ⇥ kategoria(QID) ⇥ gjinitë(QID,…)`
pub fn lexeme_rows(json: &[u8]) -> Result<Vec<String>, String> {
    let mut rows: Vec<String> = bindings(json, "lexemes")?
        .iter()
        .filter_map(|b| {
            let (l, lemma) = (short(val(b, "l")), common::clean(val(b, "lemma")));
            match (l.is_empty(), lemma.is_empty()) {
                (false, false) => Some(format!("L\t{l}\t{lemma}\t{}\t{}", short(val(b, "cat")), qids(val(b, "genders")))),
                _ => None,
            }
        })
        .collect();
    rows.sort();
    rows.dedup();
    Ok(rows)
}

/// `F ⇥ lexeme ⇥ forma-id ⇥ përfaqësimi ⇥ tiparet(QID,…)`
pub fn form_rows(json: &[u8]) -> Result<Vec<String>, String> {
    let mut rows: Vec<String> = bindings(json, "forms")?
        .iter()
        .filter_map(|b| {
            let (l, f, rep) = (short(val(b, "l")), short(val(b, "form")), common::clean(val(b, "rep")));
            match (l.is_empty(), f.is_empty(), rep.is_empty()) {
                (false, false, false) => Some(format!("F\t{l}\t{f}\t{rep}\t{}", qids(val(b, "feats")))),
                _ => None,
            }
        })
        .collect();
    rows.sort();
    rows.dedup();
    Ok(rows)
}

/// `C ⇥ lexeme ⇥ përbërësi ⇥ lema_e_përbërësit ⇥ renditja`
pub fn component_rows(json: &[u8]) -> Result<Vec<String>, String> {
    let mut rows: Vec<String> = bindings(json, "components")?
        .iter()
        .filter_map(|b| {
            let (l, c) = (short(val(b, "l")), short(val(b, "c")));
            match (l.is_empty(), c.is_empty()) {
                (false, false) => Some(format!("C\t{l}\t{c}\t{}\t{}", common::clean(val(b, "cl")), common::clean(val(b, "ord")))),
                _ => None,
            }
        })
        .collect();
    rows.sort();
    rows.dedup();
    Ok(rows)
}

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let date = today();
    let accept = Some("application/sparql-results+json");
    let version = format!("Wikidata Query Service, pyetur më {date} (UTC)");
    let check = "pa identifikues botuesi (SPARQL i gjallë); SHA-256 vetëm lokale";
    let mut out = Vec::new();
    let mut tsv = format!(
        "# wd_lexemes-sq.tsv — {ATTRIBUTION}; {LICENSE}; {ENDPOINT}, pyetur më {date} (UTC)\n# L\tlexeme\tlema\tkategoria\tgjinitë(P5185)\n# F\tlexeme\tforma\tpërfaqësimi\ttiparet\n# C\tlexeme\tpërbërësi(P5238)\tlema_e_përbërësit\trenditja(P1545)\n"
    );
    let parts: [(&str, &str, fn(&[u8]) -> Result<Vec<String>, String>); 3] = [
        ("lexemes", Q_LEXEMES, lexeme_rows),
        ("forms", Q_FORMS, form_rows),
        ("components", Q_COMPONENTS, component_rows),
    ];
    let mut rq = String::new();
    // 1) Shkarko dhe kontrollo të tria; asgjë në disk ende.
    let mut got: Vec<(&str, Vec<u8>)> = Vec::new();
    for (name, q, conv) in parts {
        print!("[nura_fetch] wd_lexemes: {name} … ");
        std::thread::sleep(std::time::Duration::from_secs(5));
        let bytes = get_patient(q, accept)?;
        let rows = conv(&bytes)?;
        println!("{} B, {} rreshta", bytes.len(), rows.len());
        for r in rows {
            tsv.push_str(&r);
            tsv.push('\n');
        }
        rq.push_str(&format!("# {name}\n{q}\n\n"));
        got.push((name, bytes));
    }
    // 2) Vetëm tani shkruhen skedarët.
    for (name, bytes) in &got {
        out.push(common::record(
            data,
            "wd_lexemes",
            &format!("raw/wd_lexemes/wd_lexemes-sq.{name}.json"),
            bytes,
            ENDPOINT,
            &version,
            LICENSE,
            ATTRIBUTION,
            check,
            "FULL",
        )?);
    }
    out.push(common::record(data, "wd_lexemes", REL_RQ, rq.as_bytes(), ENDPOINT, &version, LICENSE, ATTRIBUTION, "pyetjet e përdorura (tekst)", "FULL")?);
    out.push(common::record(
        data,
        "wd_lexemes",
        REL_TSV,
        tsv.as_bytes(),
        ENDPOINT,
        &version,
        LICENSE,
        ATTRIBUTION,
        "nxjerrë nga wd_lexemes-sq.{lexemes,forms,components}.json",
        "FULL",
    )?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEX: &str = r#"{"head":{"vars":["l","lemma","cat","genders"]},"results":{"bindings":[
 {"l":{"type":"uri","value":"http://www.wikidata.org/entity/L590679"},"lemma":{"xml:lang":"sq","type":"literal","value":"mal"},"cat":{"type":"uri","value":"http://www.wikidata.org/entity/Q1084"},"genders":{"type":"literal","value":"http://www.wikidata.org/entity/Q499327"}},
 {"l":{"type":"uri","value":"http://www.wikidata.org/entity/L1209636"},"lemma":{"xml:lang":"sq","type":"literal","value":"mace"},"cat":{"type":"uri","value":"http://www.wikidata.org/entity/Q1084"},"genders":{"type":"literal","value":""}}]}}"#;
    const FORMS: &str = r#"{"head":{"vars":["l","form","rep","feats"]},"results":{"bindings":[
 {"l":{"type":"uri","value":"http://www.wikidata.org/entity/L590679"},"form":{"type":"uri","value":"http://www.wikidata.org/entity/L590679-F6"},"rep":{"xml:lang":"sq","type":"literal","value":"mali"},"feats":{"type":"literal","value":"http://www.wikidata.org/entity/Q53997851,http://www.wikidata.org/entity/Q110786,http://www.wikidata.org/entity/Q131105"}}]}}"#;
    const COMP: &str = r#"{"head":{"vars":["l","c","cl","ord"]},"results":{"bindings":[
 {"l":{"type":"uri","value":"http://www.wikidata.org/entity/L1364563"},"c":{"type":"uri","value":"http://www.wikidata.org/entity/L1408052"},"cl":{"xml:lang":"sq","type":"literal","value":"-xhi"}},
 {"l":{"type":"uri","value":"http://www.wikidata.org/entity/L1364563"},"c":{"type":"uri","value":"http://www.wikidata.org/entity/L1358785"},"cl":{"xml:lang":"sq","type":"literal","value":"duhan"},"ord":{"type":"literal","value":"1"}}]}}"#;

    #[test]
    fn converts_real_shapes() {
        assert_eq!(
            lexeme_rows(LEX.as_bytes()).unwrap(),
            vec!["L\tL1209636\tmace\tQ1084\t".to_string(), "L\tL590679\tmal\tQ1084\tQ499327".to_string()]
        );
        assert_eq!(form_rows(FORMS.as_bytes()).unwrap(), vec!["F\tL590679\tL590679-F6\tmali\tQ110786,Q131105,Q53997851".to_string()]);
        assert_eq!(
            component_rows(COMP.as_bytes()).unwrap(),
            vec!["C\tL1364563\tL1358785\tduhan\t1".to_string(), "C\tL1364563\tL1408052\t-xhi\t".to_string()]
        );
    }

    #[test]
    fn transient_errors() {
        assert!(transient("GET https://query.wikidata.org/sparql: http status: 429"));
        assert!(transient("GET x: http status: 503"));
        assert!(!transient("GET x: http status: 400"));
        assert_eq!(BACKOFF.iter().sum::<u64>(), 630);
    }

    #[test]
    fn bad_json_is_error() {
        assert!(lexeme_rows(b"[]").is_err());
        assert!(form_rows(b"x").is_err());
    }
}
