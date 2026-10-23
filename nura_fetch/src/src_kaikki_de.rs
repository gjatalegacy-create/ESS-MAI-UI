//! Burimi «kaikki_de»: hyrjet shqipe të Wiktionary-t gjermanisht (de.wiktionary),
//! të nxjerra me wiktextract dhe të botuara nga kaikki.org si JSONL.
//!
//! Çfarë merret (dhe vetëm kaq):
//! - për çdo hyrje: fjala, pjesa e ligjëratës (`pos`), etiketat e gjinisë
//!   (masculine/feminine/neuter) dhe shenja `form-of` (trajtë e lakuar/zgjedhuar);
//! - vargjet e lidhjeve: synonyms, antonyms, hypernyms, hyponyms,
//!   coordinate_terms, derived — me `sense_index` dhe etiketat normalizuara
//!   anglisht (`tags`) të wiktextract.
//! Shpjegimet gjermanisht (glosses), `raw_tags` gjermanisht, përkthimet,
//! etimologjia dhe shembujt NUK merren. Fjalët e lidhura me ä/ö/ü/ß (tituj
//! gjermanë që wiktextract i ka futur si fjalë, p.sh. «Abkürzungen») hidhen:
//! shqipja nuk i ka këto shkronja.
//!
//! Formati i TSV-së (`raw/kaikki_de/kaikki_de-sq.tsv`, UTF-8, rreshtat `#` janë koment):
//! - `E ⇥ nr ⇥ fjala ⇥ pos ⇥ gjinia ⇥ form_of` — gjinia: bashkim i "m","f","n"
//!   sipas radhës (p.sh. "m", "mf", ""), form_of: "1" ose "0";
//! - `L ⇥ nr ⇥ lidhja ⇥ fjala_e_lidhur ⇥ sense_index ⇥ gjatësia_e_listës ⇥ tags`
//!   — `nr` është numri i hyrjes (rreshti në JSONL, nga 1), `lidhja` është emri
//!   i vargut në JSON, `gjatësia_e_listës` sa elemente ka vargu i plotë i hyrjes,
//!   `tags` të ndara me "|".

use crate::common::{self, Fetched};
use serde_json::Value;
use std::path::Path;

pub const SOURCE: &str = "kaikki_de";
pub const URL: &str = "https://kaikki.org/dewiktionary/Albanisch/kaikki.org-dictionary-Albanisch.jsonl";
/// Faqja e kaikki.org që deklaron datën e nxjerrjes dhe dump-in e Wiktionary-t.
pub const INFO_URL: &str = "https://kaikki.org/dewiktionary/index.html";
/// API publike e de.wiktionary për licencën e faqes (siteinfo/rightsinfo).
pub const RIGHTS_URL: &str = "https://de.wiktionary.org/w/api.php";
pub const RAW: &str = "raw/kaikki_de/kaikki.org-dictionary-Albanisch.jsonl";
pub const TSV: &str = "raw/kaikki_de/kaikki_de-sq.tsv";
/// Licenca sipas de.wiktionary (siteinfo/rightsinfo: CC BY-SA 4.0); GFDL-ja përmendet
/// vetëm në shënimin e të drejtave të kaikki.org, jo te rightsinfo e Wiktionary-t.
pub const LICENSE: &str = "CC BY-SA 4.0 (de.wiktionary rightsinfo); kaikki.org përmend edhe GFDL";
pub const ATTRIBUTION: &str = "Wiktionary gjermanisht (de.wiktionary.org), kontribuuesit e Wiktionary; nxjerrë me wiktextract (Tatu Ylonen, LREC 2022) dhe botuar nga kaikki.org";

/// Vargjet e lidhjeve që importohen (emrat si në JSON-in e wiktextract).
pub const LINKAGES: [&str; 6] = ["synonyms", "antonyms", "hypernyms", "hyponyms", "coordinate_terms", "derived"];

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let raw = common::get(URL)?;
    let version = version_from_info(&common::get(INFO_URL).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default());
    let check = publisher_check();
    let text = String::from_utf8(raw.clone()).map_err(|e| format!("{URL}: UTF-8 {e}"))?;
    let (tsv, stats) = extract(&text);
    println!("[nura_fetch] kaikki_de: {stats}");
    let mut out = Vec::new();
    out.push(common::record(data, SOURCE, RAW, &raw, URL, &version, LICENSE, ATTRIBUTION, &check, "FULL")?);
    out.push(common::record(
        data,
        SOURCE,
        TSV,
        tsv.as_bytes(),
        &format!("{URL}#extract=word,pos,gender,form-of,{}", LINKAGES.join(",")),
        &format!("{version} — nxjerrje lokale"),
        LICENSE,
        ATTRIBUTION,
        &format!("derivuar lokalisht nga {RAW}; {stats}"),
        "EXTRACTED",
    )?);
    Ok(out)
}

/// Versioni sipas faqes së kaikki.org: "extracted on <data> from the dewiktionary
/// dump dated <data> using wiktextract (<commit-et>)".
pub fn version_from_info(html: &str) -> String {
    let flat: String = strip_tags(html).split_whitespace().collect::<Vec<_>>().join(" ").replace("( ", "(").replace(" )", ")");
    let found = flat
        .find("extracted on ")
        .map(|i| &flat[i..])
        .and_then(|s| s.find(").").map(|j| s[..j + 1].to_string()).or_else(|| Some(s.chars().take(160).collect())));
    match found {
        Some(s) => format!("kaikki.org dewiktionary — {s}"),
        None => "kaikki.org dewiktionary — data e nxjerrjes e panjohur (faqja informuese nuk u lexua)".into(),
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for c in html.chars() {
        match (c, inside) {
            ('<', _) => inside = true,
            ('>', true) => {
                inside = false;
                out.push(' ');
            }
            (_, false) => out.push(c),
            (_, true) => {}
        }
    }
    out
}

/// Kontroll i licencës te botuesi: rightsinfo e de.wiktionary.
fn publisher_check() -> String {
    let q = [("action", "query"), ("meta", "siteinfo"), ("siprop", "rightsinfo"), ("format", "json")];
    match common::get_with(RIGHTS_URL, &q, Some("application/json")) {
        Ok(b) => match serde_json::from_slice::<Value>(&b) {
            Ok(v) => {
                let r = &v["query"]["rightsinfo"];
                format!(
                    "de.wiktionary siteinfo/rightsinfo: «{}» <{}>; pa checksum botuesi — vetëm SHA-256 lokale",
                    r["text"].as_str().unwrap_or("?"),
                    r["url"].as_str().unwrap_or("?")
                )
            }
            Err(e) => format!("de.wiktionary rightsinfo: JSON i palexueshëm ({e})"),
        },
        Err(e) => format!("de.wiktionary rightsinfo: i paarritshëm ({e})"),
    }
}

fn gender_code(tags: &[&str]) -> String {
    let mut g = String::new();
    for (t, c) in [("masculine", 'm'), ("feminine", 'f'), ("neuter", 'n')] {
        match tags.contains(&t) {
            true => g.push(c),
            false => {}
        }
    }
    g
}

fn str_list(v: &Value) -> Vec<&str> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default()
}

/// JSONL → TSV. Kthen tekstin dhe një përmbledhje me numërime.
pub fn extract(jsonl: &str) -> (String, String) {
    let mut out = String::new();
    out.push_str("# kaikki_de — hyrjet shqipe të de.wiktionary (kaikki.org), vetëm fjala/pos/gjinia/form-of dhe lidhjet\n");
    out.push_str(&format!("# burimi: {URL}\n# licenca: {LICENSE}\n"));
    out.push_str("# E\tnr\tfjala\tpos\tgjinia\tform_of\n# L\tnr\tlidhja\tfjala_e_lidhur\tsense_index\tgjatesia_e_listes\ttags\n");
    let (mut entries, mut links, mut bad, mut german) = (0usize, 0usize, 0usize, 0usize);
    for (i, line) in jsonl.lines().enumerate() {
        let v: Value = match (line.trim().is_empty(), serde_json::from_str::<Value>(line)) {
            (true, _) => continue,
            (false, Ok(v)) => v,
            (false, Err(_)) => {
                bad += 1;
                continue;
            }
        };
        let nr = i + 1;
        let word = common::clean(v["word"].as_str().unwrap_or(""));
        let lang_ok = v["lang_code"].as_str() == Some("sq");
        match (word.is_empty(), lang_ok) {
            (false, true) => {}
            _ => {
                bad += 1;
                continue;
            }
        }
        let tags = str_list(&v["tags"]);
        let form_of = match tags.contains(&"form-of") {
            true => "1",
            false => "0",
        };
        out.push_str(&format!(
            "E\t{nr}\t{word}\t{}\t{}\t{form_of}\n",
            common::clean(v["pos"].as_str().unwrap_or("")),
            gender_code(&tags)
        ));
        entries += 1;
        for rel in LINKAGES {
            let items = v[rel].as_array().cloned().unwrap_or_default();
            let n = items.len();
            for it in &items {
                let target = common::clean(it["word"].as_str().unwrap_or(""));
                match (target.is_empty(), is_german(&target)) {
                    (true, _) => continue,
                    (false, true) => {
                        german += 1;
                        continue;
                    }
                    (false, false) => {}
                }
                let sense = common::clean(it["sense_index"].as_str().unwrap_or(""));
                let itags = str_list(&it["tags"]).iter().map(|t| common::clean(t)).collect::<Vec<_>>().join("|");
                out.push_str(&format!("L\t{nr}\t{rel}\t{target}\t{sense}\t{n}\t{itags}\n"));
                links += 1;
            }
        }
    }
    (out, format!("{entries} hyrje, {links} lidhje, {german} fjalë gjermane të hedhura, {bad} rreshta të papërdorshëm"))
}

/// Shkronjat ä/ö/ü/ß nuk janë në alfabetin e shqipes: një «fjalë e lidhur» me to
/// është tekst gjerman (titull seksioni) i rrjedhur nga wiktextract.
fn is_german(s: &str) -> bool {
    s.chars().any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'ß' | 'Ä' | 'Ö' | 'Ü'))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Rreshta të vërtetë (të shkurtuar) nga kaikki.org-dictionary-Albanisch.jsonl.
    const FIX: &str = concat!(
        r#"{"word": "dem", "pos": "noun", "pos_title": "Substantiv", "lang_code": "sq", "lang": "Albanisch", "synonyms": [{"word": "ka", "sense_index": "1"}], "antonyms": [{"word": "lopë", "sense_index": "1"}], "hypernyms": [{"word": "kafschë", "sense_index": "1"}, {"word": "shtazë", "sense_index": "1"}], "hyponyms": [{"word": "dem race", "sense_index": "1"}], "coordinate_terms": [{"word": "viç", "sense_index": "1"}], "tags": ["masculine"], "senses": [{"glosses": ["männliches Rind, Stier"], "sense_index": "1"}]}"#,
        "\n",
        r#"{"word": "dua", "pos": "verb", "lang_code": "sq", "lang": "Albanisch", "synonyms": [{"word": "Abkürzungen", "sense_index": "1"}], "derived": [{"word": "dashas", "sense_index": "1", "raw_tags": ["Adverbien"]}, {"word": "dashur", "sense_index": "1", "tags": ["participle"]}], "senses": [{"glosses": ["lieben"], "sense_index": "1"}]}"#,
        "\n",
        r#"{"word": "lopës", "pos": "noun", "lang_code": "sq", "lang": "Albanisch", "tags": ["feminine", "form-of"], "senses": [{"glosses": ["Dativ Singular von lopë"]}]}"#,
        "\n",
        "nuk-është-json\n"
    );

    #[test]
    fn extract_keeps_linkages_and_drops_german() {
        let (tsv, stats) = extract(FIX);
        assert!(tsv.contains("E\t1\tdem\tnoun\tm\t0\n"));
        assert!(tsv.contains("L\t1\tantonyms\tlopë\t1\t1\t\n"));
        assert!(tsv.contains("L\t1\thypernyms\tkafschë\t1\t2\t\n"));
        assert!(tsv.contains("L\t1\tcoordinate_terms\tviç\t1\t1\t\n"));
        assert!(tsv.contains("L\t2\tderived\tdashur\t1\t2\tparticiple\n"));
        assert!(tsv.contains("E\t3\tlopës\tnoun\tf\t1\n"));
        // Asnjë shpjegim apo raw_tag gjermanisht.
        assert!(!tsv.contains("Rind"));
        assert!(!tsv.contains("lieben"));
        assert!(!tsv.contains("Adverbien"));
        assert!(!tsv.contains("Abkürzungen"));
        assert_eq!(stats, "3 hyrje, 8 lidhje, 1 fjalë gjermane të hedhura, 1 rreshta të papërdorshëm");
    }

    #[test]
    fn version_is_read_from_info_page() {
        let html = "<p>This dictionary is based on structured data extracted on 2026-10-20 from the dewiktionary dump dated 2026-10-01 using <a href=x>wiktextract</a> (<a href=y>c1bb342</a> and <a href=z>6f96ddf</a>). If you use</p>";
        assert_eq!(
            version_from_info(html),
            "kaikki.org dewiktionary — extracted on 2026-10-20 from the dewiktionary dump dated 2026-10-01 using wiktextract (c1bb342 and 6f96ddf)"
        );
        assert!(version_from_info("").contains("e panjohur"));
    }

    #[test]
    fn gender_codes() {
        assert_eq!(gender_code(&["masculine"]), "m");
        assert_eq!(gender_code(&["feminine", "masculine"]), "mf");
        assert_eq!(gender_code(&["noun"]), "");
    }

    #[test]
    fn german_words_detected() {
        assert!(is_german("Abkürzungen"));
        assert!(is_german("Straße"));
        assert!(!is_german("çelës"));
        assert!(!is_german("shtëpi"));
    }
}
