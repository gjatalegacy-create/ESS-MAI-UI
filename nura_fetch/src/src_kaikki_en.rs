//! kaikki_en — hyrjet SHQIPE të Wiktionary-t anglisht, të nxjerra nga Wiktextract
//! dhe të publikuara nga kaikki.org (JSONL, një hyrje për rresht).
//!
//! - Burimi: https://kaikki.org/dictionary/Albanian/kaikki.org-dictionary-Albanian.jsonl
//! - Licenca: CC BY-SA 4.0 dhe GFDL (si Wiktionary); atribuimi: kontribuuesit e
//!   Wiktionary + Wiktextract (Tatu Ylonen) / kaikki.org.
//! - Versioni: data e nxjerrjes dhe data e dump-it lexohen nga faqja zyrtare
//!   https://kaikki.org/dictionary/Albanian/ (p.sh. "extracted on 2026-10-03 from
//!   the enwiktionary dump dated 2026-09-02").
//! - kaikki.org nuk publikon hash për skedarin: SHA-256 këtu është vetëm lokal.
//!
//! JSONL-i lexohet rresht për rresht me serde_json dhe kthehet në një TSV kompakt
//! që importuesi pa varësi (nura_lingua::src_kaikki_en) e lexon me dorë:
//!
//! ```text
//! E ⇥ fjala ⇥ pos ⇥ gjinia_e_papërpunuar ⇥ forma_kanonike ⇥ nr_etimologjie   (fillon një hyrje)
//! F ⇥ forma ⇥ etiketat,… ⇥ burimi_i_formës                                   (forms[])
//! G ⇥ n ⇥ etiketat,… ⇥ glosa_angleze                                          (senses[n-1].glosses)
//! O ⇥ n ⇥ form_of|alt_of ⇥ fjala_objektiv ⇥ etiketat,…                        (senses[n-1].form_of / alt_of)
//! R ⇥ n ⇥ REL ⇥ fjala ⇥ etiketat,… ⇥ lang_code ⇥ shënim_anglisht             (n = 0: niveli i hyrjes)
//! ```
//!
//! REL ∈ SYNONYM, ANTONYM, HYPERNYM, HYPONYM, DERIVED, RELATED, COORDINATE.
//! Rreshtat e formave me etiketat `error-unrecognized-form`, `table-tags`,
//! `inflection-template` hidhen; fushat `_dis1` nuk shkruhen. Glosat janë ANGLISHT
//! dhe mbeten të shënuara si të tilla nga importuesi.
//!
//! Stil: pa if/else — vetëm match.

use crate::common::{self, Fetched};
use serde_json::Value;
use std::path::Path;

pub const SOURCE: &str = "kaikki_en";
pub const URL: &str = "https://kaikki.org/dictionary/Albanian/kaikki.org-dictionary-Albanian.jsonl";
pub const INDEX_URL: &str = "https://kaikki.org/dictionary/Albanian/";
pub const RAW: &str = "raw/kaikki_en/kaikki.org-dictionary-Albanian.jsonl";
/// E njëjta rrugë si `nura_lingua::src_kaikki_en::TSV`.
pub const TSV: &str = "raw/kaikki_en/kaikki_en-sq.tsv";
const LICENSE: &str = "CC BY-SA 4.0 + GFDL (Wiktionary)";
const ATTRIBUTION: &str = "Wiktionary (en.wiktionary.org) contributors; extracted by Wiktextract (Tatu Ylonen, LREC 2022) and published by kaikki.org";

/// Fushat e marrëdhënieve (në hyrje dhe në kuptim) → emri në TSV.
const RELS: [(&str, &str); 7] = [
    ("synonyms", "SYNONYM"),
    ("antonyms", "ANTONYM"),
    ("hypernyms", "HYPERNYM"),
    ("hyponyms", "HYPONYM"),
    ("derived", "DERIVED"),
    ("related", "RELATED"),
    ("coordinate_terms", "COORDINATE"),
];

/// Etiketat e formave që nuk janë forma të vërteta (sipas Wiktextract).
const DROP_FORM_TAGS: [&str; 3] = ["error-unrecognized-form", "table-tags", "inflection-template"];

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let page = common::get(INDEX_URL)?;
    let version = dump_version(&String::from_utf8_lossy(&page));
    println!("[kaikki_en] versioni: {version}");
    let raw = common::get(URL)?;
    let (tsv, st) = build_tsv(&raw, &version);
    match st.entries {
        0 => return Err(format!("kaikki_en: asnjë hyrje shqipe në {URL} ({} rreshta të pavlefshëm)", st.bad_lines)),
        _ => {}
    }
    let r1 = common::record(
        data,
        SOURCE,
        RAW,
        &raw,
        URL,
        &version,
        LICENSE,
        ATTRIBUTION,
        "kaikki.org nuk publikon hash; SHA-256 vetëm lokale",
        "OK",
    )?;
    let summary = st.summary();
    let r2 = common::record(
        data,
        SOURCE,
        TSV,
        tsv.as_bytes(),
        &format!("{URL}#tsv"),
        &version,
        LICENSE,
        ATTRIBUTION,
        &format!("derivuar lokalisht nga {RAW} (sha256 {}); {summary}", r1.sha256),
        "OK_DERIVED_TSV",
    )?;
    println!("[kaikki_en] {} bajte JSONL → {} bajte TSV; {summary}", r1.bytes, r2.bytes);
    Ok(vec![r1, r2])
}

/// "extracted on 2026-10-03 from the enwiktionary dump dated 2026-09-02 using wiktextract (1a05e46 and e3d6d4e)"
pub fn dump_version(html: &str) -> String {
    let text = strip_tags(html);
    let start = text.find("extracted on ");
    match start {
        Some(i) => {
            let rest = &text[i..];
            let end = rest.find(')').map(|e| e + 1).unwrap_or(rest.len().min(200));
            format!("kaikki.org {}", rest[..end].split_whitespace().collect::<Vec<_>>().join(" "))
        }
        None => "kaikki.org (data e dump-it nuk u gjet në faqe)".into(),
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for c in html.chars() {
        match (c, inside) {
            ('<', _) => inside = true,
            ('>', true) => inside = false,
            (_, true) => {}
            (c, false) => out.push(c),
        }
    }
    out
}

#[derive(Default, Debug)]
pub struct Stats {
    pub entries: usize,
    pub bad_lines: usize,
    pub other_lang: usize,
    pub forms: usize,
    pub forms_dropped: usize,
    pub glosses: usize,
    pub links: usize,
    pub rels: usize,
}

impl Stats {
    fn summary(&self) -> String {
        format!(
            "{} hyrje sq; {} forma (+{} të hedhura: error/table-tags/inflection-template); {} kuptime me glosë angleze; {} form_of/alt_of; {} marrëdhënie; {} rreshta të pavlefshëm; {} hyrje në gjuhë tjetër",
            self.entries, self.forms, self.forms_dropped, self.glosses, self.links, self.rels, self.bad_lines, self.other_lang
        )
    }
}

fn s(v: &Value) -> String {
    common::clean(v.as_str().unwrap_or(""))
}

fn tags(v: &Value) -> String {
    v["tags"]
        .as_array()
        .map(|a| a.iter().filter_map(|t| t.as_str()).map(common::clean).collect::<Vec<_>>().join(","))
        .unwrap_or_default()
}

fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

/// Gjinia nga shabllonet e kokës: `head` (g, g2), `sq-noun` (1), `sq-proper noun` (1 ose g).
fn gender_raw(v: &Value) -> String {
    let mut out: Vec<String> = Vec::new();
    for h in arr(&v["head_templates"]) {
        let a = &h["args"];
        let cands: Vec<&Value> = match h["name"].as_str() {
            Some("head") => vec![&a["g"], &a["g2"]],
            Some("sq-noun") | Some("sq-proper noun") => vec![&a["1"], &a["g"]],
            _ => vec![],
        };
        for c in cands {
            let g = s(c);
            match (g.is_empty(), out.contains(&g)) {
                (false, false) => out.push(g),
                _ => {}
            }
        }
    }
    out.join(",")
}

fn rel_rows(out: &mut String, st: &mut Stats, n: usize, holder: &Value) {
    for (key, rel) in RELS {
        for it in arr(&holder[key]) {
            let w = s(&it["word"]);
            match w.is_empty() {
                true => {}
                false => {
                    st.rels += 1;
                    out.push_str(&format!(
                        "R\t{n}\t{rel}\t{w}\t{}\t{}\t{}\n",
                        tags(&it),
                        s(&it["lang_code"]),
                        s(&it["sense"])
                    ));
                }
            }
        }
    }
}

/// Një hyrje JSON → rreshtat e TSV-së.
fn entry_rows(v: &Value, out: &mut String, st: &mut Stats) {
    let word = s(&v["word"]);
    let canonical = arr(&v["forms"])
        .iter()
        .find(|f| arr(&f["tags"]).iter().any(|t| t.as_str() == Some("canonical")))
        .map(|f| s(&f["form"]))
        .unwrap_or_default();
    out.push_str(&format!(
        "E\t{word}\t{}\t{}\t{canonical}\t{}\n",
        s(&v["pos"]),
        gender_raw(v),
        v["etymology_number"].as_u64().map(|n| n.to_string()).unwrap_or_default()
    ));
    for f in arr(&v["forms"]) {
        let drop = arr(&f["tags"]).iter().any(|t| DROP_FORM_TAGS.contains(&t.as_str().unwrap_or("")));
        let form = s(&f["form"]);
        match (drop, form.is_empty()) {
            (false, false) => {
                st.forms += 1;
                out.push_str(&format!("F\t{form}\t{}\t{}\n", tags(&f), s(&f["source"])));
            }
            _ => st.forms_dropped += 1,
        }
    }
    rel_rows(out, st, 0, v);
    for (i, sense) in arr(&v["senses"]).iter().enumerate() {
        let n = i + 1;
        let gl: Vec<String> = arr(&sense["glosses"]).iter().map(s).filter(|g| !g.is_empty()).collect();
        match gl.is_empty() {
            true => {}
            false => {
                st.glosses += 1;
                out.push_str(&format!("G\t{n}\t{}\t{}\n", tags(sense), gl.join(": ")));
            }
        }
        for (key, kind) in [("form_of", "form_of"), ("alt_of", "alt_of")] {
            for it in arr(&sense[key]) {
                let w = s(&it["word"]);
                match w.is_empty() {
                    true => {}
                    false => {
                        st.links += 1;
                        out.push_str(&format!("O\t{n}\t{kind}\t{w}\t{}\n", tags(sense)));
                    }
                }
            }
        }
        rel_rows(out, st, n, sense);
    }
}

/// Lexon JSONL-in rresht për rresht dhe ndërton TSV-në.
pub fn build_tsv(raw: &[u8], version: &str) -> (String, Stats) {
    let mut st = Stats::default();
    let mut out = String::with_capacity(raw.len() / 3);
    out.push_str(&format!(
        "# kaikki_en — hyrjet shqipe të Wiktionary-t anglisht (Wiktextract / kaikki.org)\n# burimi: {URL}\n# versioni: {version}\n# licenca: {LICENSE}; atribuimi: {ATTRIBUTION}\n# glosat (G) janë ANGLISHT\n"
    ));
    for line in raw.split(|b| *b == b'\n') {
        let trimmed = line.strip_suffix(b"\r").unwrap_or(line);
        match trimmed.iter().all(|b| b.is_ascii_whitespace()) {
            true => continue,
            false => {}
        }
        match serde_json::from_slice::<Value>(trimmed) {
            Ok(v) => match v["lang_code"].as_str() {
                Some("sq") => {
                    st.entries += 1;
                    entry_rows(&v, &mut out, &mut st);
                }
                _ => st.other_lang += 1,
            },
            Err(_) => st.bad_lines += 1,
        }
    }
    (out, st)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hyrje reale të shkurtuara (formati i kaikki.org, 2026-10-03).
    const KALE: &str = r#"{"pos": "noun", "head_templates": [{"name": "head", "args": {"1": "sq", "2": "noun", "head": "", "g": "m", "3": "plural", "4": "kuaj", "5": "definite", "6": "kali"}, "expansion": "kalë m (plural kuaj, definite kali, definite plural kuajt)"}, {"name": "sq-noun", "args": {"1": "m", "2": "kuaj", "3": "kali", "4": "kuajt"}, "expansion": "kalë m"}], "forms": [{"form": "kuaj", "tags": ["plural"]}, {"form": "kali", "tags": ["definite"]}, {"form": "no-table-tags", "source": "declension", "tags": ["table-tags"]}, {"form": "sq-noun-m", "source": "declension", "tags": ["inflection-template"]}, {"form": "kali", "tags": ["definite", "nominative", "singular"], "source": "declension"}, {"form": "kalin", "tags": ["accusative", "definite", "singular"], "source": "declension"}, {"form": "kalëx", "tags": ["error-unrecognized-form", "indefinite", "singular"], "source": "declension"}], "word": "kalë", "lang": "Albanian", "lang_code": "sq", "derived": [{"word": "kalorës", "_dis1": "0 0"}], "senses": [{"glosses": ["horse"], "tags": ["masculine"], "id": "en-kalë-sq-noun-x", "synonyms": [{"word": "at"}]}]}"#;
    const KALIN: &str = r#"{"pos": "noun", "head_templates": [{"name": "head", "args": {"1": "sq", "2": "noun form"}, "expansion": "kalin"}], "word": "kalin", "lang": "Albanian", "lang_code": "sq", "senses": [{"glosses": ["definite accusative singular of kalë"], "tags": ["accusative", "definite", "form-of", "singular"], "form_of": [{"word": "kalë"}], "id": "en-kalin-sq-noun-A5ISMrF1"}], "forms": []}"#;
    const BUKUR: &str = r#"{"pos": "adj", "head_templates": [{"name": "head", "args": {"1": "sq", "2": "adjective", "head": "i bukur", "3": "feminine plural", "4": "të bukura"}, "expansion": "i bukur (feminine plural të bukura)"}], "forms": [{"form": "i bukur", "tags": ["canonical"]}, {"form": "të bukura", "tags": ["feminine", "plural"]}], "word": "bukur", "lang": "Albanian", "lang_code": "sq", "senses": [{"glosses": ["beautiful"], "id": "en-bukur-sq-adj-xqFCfXEZ"}]}"#;

    #[test]
    fn version_is_read_from_index_page() {
        let html = r#"<p>This dictionary is based on structured data extracted on 2026-10-03 from the enwiktionary dump dated 2026-09-02 using <a href="https://github.com/tatuylonen/wiktextract">wiktextract</a> (<a href="x">1a05e46</a> and <a href="y">e3d6d4e</a>).</p>"#;
        assert_eq!(
            dump_version(html),
            "kaikki.org extracted on 2026-10-03 from the enwiktionary dump dated 2026-09-02 using wiktextract (1a05e46 and e3d6d4e)"
        );
        assert!(dump_version("<html></html>").contains("nuk u gjet"));
    }

    #[test]
    fn tsv_keeps_real_forms_and_drops_template_rows() {
        let raw = format!("{KALE}\n{KALIN}\n{BUKUR}\nnot json\n");
        let (t, st) = build_tsv(raw.as_bytes(), "v");
        assert_eq!(st.entries, 3);
        assert_eq!(st.bad_lines, 1);
        assert!(t.contains("E\tkalë\tnoun\tm\t\t\n"));
        assert!(t.contains("F\tkali\tdefinite,nominative,singular\tdeclension\n"));
        assert!(t.contains("F\tkuaj\tplural\t\n"));
        assert!(!t.contains("no-table-tags"));
        assert!(!t.contains("sq-noun-m"));
        assert!(!t.contains("kalëx"));
        assert_eq!(st.forms_dropped, 3);
        assert!(t.contains("R\t0\tDERIVED\tkalorës\t\t\t\n"));
        assert!(!t.contains("_dis1"));
        assert!(!t.contains("0 0"));
        assert!(t.contains("G\t1\tmasculine\thorse\n"));
        assert!(t.contains("R\t1\tSYNONYM\tat\t\t\t\n"));
        assert!(t.contains("O\t1\tform_of\tkalë\taccusative,definite,form-of,singular\n"));
        assert!(t.contains("E\tbukur\tadj\t\ti bukur\t\n"));
        assert!(t.contains("F\ttë bukura\tfeminine,plural\t\n"));
    }

    #[test]
    fn other_languages_are_not_extracted() {
        let raw = r#"{"word": "Hund", "pos": "noun", "lang_code": "de", "senses": []}"#;
        let (t, st) = build_tsv(raw.as_bytes(), "v");
        assert_eq!((st.entries, st.other_lang), (0, 1));
        assert!(!t.contains("Hund"));
    }
}
