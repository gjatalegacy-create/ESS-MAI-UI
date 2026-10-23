//! EuroVoc (Zyra e Publikimeve të BE-së) — etiketat shqipe nëpërmjet SPARQL.
//!
//! - Pikë hyrjeje: https://publications.europa.eu/webapi/rdf/sparql (GET, JSON).
//! - Një pyetje e vetme mbi skemën http://eurovoc.europa.eu/100141: URI e konceptit,
//!   prefLabel shqip, altLabel shqip, skos:broader, skos:related dhe prefLabel anglisht
//!   (vetëm si urë, kurrë si tekst shqip). Konceptet e vjetruara (owl:deprecated) jashtë.
//! - Pyetje e dytë: altLabel anglisht i çdo koncepti (urë), që importuesi të dallojë
//!   etiketat «shqipe» që në fakt janë anglisht e papërkthyer.
//! - Zip-i i plotë (~30 MB) NUK shkarkohet.
//! - Përgjigjet e papërpunuara ruhen si `eurovoc-sq.sparql.json` dhe
//!   `eurovoc-en-alt.sparql.json`, pyetjet si `eurovoc-sq.rq`, nxjerrja si `eurovoc-sq.tsv`
//!   (data e pyetjes në fushën e versionit). Asgjë nuk shkruhet para se të gjitha pyetjet
//!   të kenë sukses.
//! - Licenca: politika e ripërdorimit e BE-së (Vendimi i Komisionit 2011/833/BE).
use crate::common::{self, Fetched};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub const ENDPOINT: &str = "https://publications.europa.eu/webapi/rdf/sparql";
pub const SCHEME: &str = "http://eurovoc.europa.eu/100141";
const PREFIX: &str = "http://eurovoc.europa.eu/";
const REL_JSON: &str = "raw/eurovoc/eurovoc-sq.sparql.json";
const REL_EN_ALT_JSON: &str = "raw/eurovoc/eurovoc-en-alt.sparql.json";
const REL_RQ: &str = "raw/eurovoc/eurovoc-sq.rq";
const REL_TSV: &str = "raw/eurovoc/eurovoc-sq.tsv";
const LICENSE: &str = "EU reuse policy — Commission Decision 2011/833/EU (ripërdorim i lirë me atribuim)";
const ATTRIBUTION: &str = "EuroVoc, Publications Office of the EU";

pub const QUERY: &str = r#"PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?c ?sq (SAMPLE(?en0) AS ?en)
       (GROUP_CONCAT(DISTINCT ?alt; separator="|") AS ?alts)
       (GROUP_CONCAT(DISTINCT STR(?b); separator=" ") AS ?broader)
       (GROUP_CONCAT(DISTINCT STR(?r); separator=" ") AS ?related)
WHERE {
  ?c skos:inScheme <http://eurovoc.europa.eu/100141> ;
     skos:prefLabel ?sq .
  FILTER(lang(?sq) = "sq")
  FILTER NOT EXISTS { ?c owl:deprecated true }
  OPTIONAL { ?c skos:prefLabel ?en0 . FILTER(lang(?en0) = "en") }
  OPTIONAL { ?c skos:altLabel ?alt . FILTER(lang(?alt) = "sq") }
  OPTIONAL { ?c skos:broader ?b }
  OPTIONAL { ?c skos:related ?r }
}
GROUP BY ?c ?sq
ORDER BY ?c
"#;

/// altLabel anglisht për çdo koncept të skemës (vetëm urë për filtrin e anglishtes).
pub const QUERY_EN_ALT: &str = r#"PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
SELECT ?c (GROUP_CONCAT(DISTINCT ?alt; separator="|") AS ?alts)
WHERE {
  ?c skos:inScheme <http://eurovoc.europa.eu/100141> ;
     skos:altLabel ?alt .
  FILTER(lang(?alt) = "en")
  FILTER NOT EXISTS { ?c owl:deprecated true }
}
GROUP BY ?c
ORDER BY ?c
"#;

const VERSION_QUERY: &str =
    "PREFIX owl: <http://www.w3.org/2002/07/owl#> SELECT ?v WHERE { <http://eurovoc.europa.eu/100141> owl:versionInfo ?v }";

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
    uri.strip_prefix(PREFIX).unwrap_or(uri).to_string()
}

fn bindings(json: &[u8]) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_slice(json).map_err(|e| format!("eurovoc: JSON i pavlefshëm: {e}"))?;
    v.get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .cloned()
        .ok_or_else(|| "eurovoc: mungon results.bindings".to_string())
}

/// Koncepti → altLabel-et anglisht (të pastruara, të renditura, të bashkuara me «|»).
pub fn en_alt_map(json: &[u8]) -> Result<HashMap<String, String>, String> {
    Ok(bindings(json)?
        .iter()
        .filter_map(|b| {
            let c = short(val(b, "c"));
            let mut alts: Vec<String> = val(b, "alts").split('|').map(common::clean).filter(|a| !a.is_empty()).collect();
            alts.sort();
            alts.dedup();
            match (c.is_empty(), alts.is_empty()) {
                (false, false) => Some((c, alts.join("|"))),
                _ => None,
            }
        })
        .collect())
}

/// Rreshtat TSV nga përgjigja SPARQL JSON:
/// `koncepti ⇥ prefLabel_sq ⇥ altLabel_sq (|) ⇥ broader (hapësirë) ⇥ related (hapësirë) ⇥ prefLabel_en ⇥ altLabel_en (|)`.
pub fn to_tsv_rows(json: &[u8], en_alt: &HashMap<String, String>) -> Result<Vec<String>, String> {
    let bindings = bindings(json)?;
    let mut rows: Vec<String> = bindings
        .iter()
        .filter_map(|b| {
            let c = short(val(b, "c"));
            let sq = common::clean(val(b, "sq"));
            match (c.is_empty(), sq.is_empty()) {
                (false, false) => {
                    let mut alts: Vec<String> = val(b, "alts")
                        .split('|')
                        .map(common::clean)
                        .filter(|a| !a.is_empty())
                        .collect();
                    alts.sort();
                    alts.dedup();
                    let ids = |k: &str| -> String {
                        let mut v: Vec<String> = val(b, k).split_whitespace().map(short).collect();
                        v.sort();
                        v.dedup();
                        v.join(" ")
                    };
                    let ena = en_alt.get(&c).map(|s| s.as_str()).unwrap_or("");
                    Some(format!(
                        "{c}\t{sq}\t{}\t{}\t{}\t{}\t{ena}",
                        alts.join("|"),
                        ids("broader"),
                        ids("related"),
                        common::clean(val(b, "en"))
                    ))
                }
                _ => None,
            }
        })
        .collect();
    rows.sort();
    Ok(rows)
}

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let date = today();
    let accept = Some("application/sparql-results+json");
    print!("[nura_fetch] eurovoc: versioni i skemës … ");
    let vjson = common::get_with(ENDPOINT, &[("query", VERSION_QUERY)], accept)?;
    let vv: Value = serde_json::from_slice(&vjson).map_err(|e| format!("eurovoc: JSON i versionit: {e}"))?;
    let thesaurus = vv
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .and_then(|a| a.first())
        .map(|b| val(b, "v").to_string())
        .unwrap_or_default();
    let thesaurus = match thesaurus.is_empty() {
        true => "e panjohur".to_string(),
        false => thesaurus,
    };
    println!("{thesaurus}");
    print!("[nura_fetch] eurovoc: pyetja SPARQL (skema {SCHEME}) … ");
    let bytes = common::get_with(ENDPOINT, &[("query", QUERY)], accept)?;
    println!("{} B", bytes.len());
    print!("[nura_fetch] eurovoc: altLabel anglisht (urë) … ");
    let en_bytes = common::get_with(ENDPOINT, &[("query", QUERY_EN_ALT)], accept)?;
    let en_alt = en_alt_map(&en_bytes)?;
    println!("{} B, {} koncepte me altLabel anglisht", en_bytes.len(), en_alt.len());
    let rows = to_tsv_rows(&bytes, &en_alt)?;
    println!("[nura_fetch] eurovoc: {} koncepte me etiketë shqipe", rows.len());
    let version = format!("EuroVoc {thesaurus}; SPARQL i pyetur më {date} (UTC)");
    let check = "pa identifikues botuesi (SPARQL i gjallë); SHA-256 vetëm lokale";
    let mut tsv = format!(
        "# eurovoc-sq.tsv — EuroVoc {thesaurus}, skema {SCHEME}, pyetur më {date} (UTC) nga {ENDPOINT}\n# {ATTRIBUTION}; {LICENSE}\n# kolonat: koncepti\tprefLabel_sq\taltLabel_sq(|)\tbroader\trelated\tprefLabel_en(urë, jo shqip)\taltLabel_en(|)(urë, jo shqip)\n"
    );
    for r in &rows {
        tsv.push_str(r);
        tsv.push('\n');
    }
    let rq = format!("# eurovoc-sq (etiketat shqipe)\n{QUERY}\n# eurovoc-en-alt (altLabel anglisht, urë)\n{QUERY_EN_ALT}");
    Ok(vec![
        common::record(data, "eurovoc", REL_RQ, rq.as_bytes(), ENDPOINT, &version, LICENSE, ATTRIBUTION, "pyetjet e përdorura (tekst)", "FULL")?,
        common::record(data, "eurovoc", REL_JSON, &bytes, ENDPOINT, &version, LICENSE, ATTRIBUTION, check, "FULL")?,
        common::record(data, "eurovoc", REL_EN_ALT_JSON, &en_bytes, ENDPOINT, &version, LICENSE, ATTRIBUTION, check, "FULL")?,
        common::record(data, "eurovoc", REL_TSV, tsv.as_bytes(), ENDPOINT, &version, LICENSE, ATTRIBUTION, "nxjerrë nga eurovoc-sq.sparql.json + eurovoc-en-alt.sparql.json", "FULL")?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{ "head": { "link": [], "vars": ["c", "sq", "en", "alts", "broader", "related"] },
  "results": { "distinct": false, "ordered": true, "bindings": [
    { "c": { "type": "uri", "value": "http://eurovoc.europa.eu/10" }	, "sq": { "type": "literal", "xml:lang": "sq", "value": "tregti e brendshme" }	, "en": { "type": "literal", "xml:lang": "en", "value": "domestic trade" }	, "alts": { "type": "literal", "value": "" }	, "broader": { "type": "literal", "value": "http://eurovoc.europa.eu/2449" }	, "related": { "type": "literal", "value": "http://eurovoc.europa.eu/1806" }},
    { "c": { "type": "uri", "value": "http://eurovoc.europa.eu/1000" }	, "sq": { "type": "literal", "xml:lang": "sq", "value": "financim" }	, "en": { "type": "literal", "xml:lang": "en", "value": "financing" }	, "alts": { "type": "literal", "value": "financat\tbashkiake|financim i Bashkimit Europian" }	, "broader": { "type": "literal", "value": "http://eurovoc.europa.eu/b2 http://eurovoc.europa.eu/b1" }	, "related": { "type": "literal", "value": "" }}
  ] } }"#;

    // Formë reale e përgjigjes së pyetjes së dytë (altLabel anglisht).
    const EN_ALT: &str = r#"{ "head": { "link": [], "vars": ["c", "alts"] },
  "results": { "distinct": false, "ordered": true, "bindings": [
    { "c": { "type": "uri", "value": "http://eurovoc.europa.eu/1000" }	, "alts": { "type": "literal", "value": "financing of the EU|EU financing" }},
    { "c": { "type": "uri", "value": "http://eurovoc.europa.eu/5" }	, "alts": { "type": "literal", "value": "" }}
  ] } }"#;

    #[test]
    fn json_to_tsv_rows() {
        let en = en_alt_map(EN_ALT.as_bytes()).unwrap();
        assert_eq!(en.len(), 1);
        let rows = to_tsv_rows(SAMPLE.as_bytes(), &en).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], "10\ttregti e brendshme\t\t2449\t1806\tdomestic trade\t");
        assert_eq!(
            rows[1],
            "1000\tfinancim\tfinancat bashkiake|financim i Bashkimit Europian\tb1 b2\t\tfinancing\tEU financing|financing of the EU"
        );
    }

    #[test]
    fn bad_json_is_error() {
        let en = HashMap::new();
        assert!(to_tsv_rows(b"{}", &en).is_err());
        assert!(to_tsv_rows(b"not json", &en).is_err());
        assert!(en_alt_map(b"[]").is_err());
    }

    #[test]
    fn date_format() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert_eq!(&d[4..5], "-");
    }
}
