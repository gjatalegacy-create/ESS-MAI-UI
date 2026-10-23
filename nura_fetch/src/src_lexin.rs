//! Shkarkuesi i LEXIN suedisht–shqip (ISOF, Institutet för språk och folkminnen).
//!
//! - Burimi: `swe_alb.xml` (≈18,7 MB; serveri nuk pranon Range — shkarkohet i plotë)
//!   dhe `metadata.xml` (licenca dhe data e publikimit), të dyja nga
//!   sprakresurser.isof.se. User-Agent: `common::UA`, pa të dhëna personale.
//! - Licenca verifikohet në `metadata.xml` (CC BY 4.0); versioni lexohet nga
//!   atributi `Version` i elementit rrënjë `<Dictionary>`.
//! - Nxjerrja (TSV) merr VETËM anën shqipe (`<TargetLang>`) të çdo `<Word>`:
//!   `Translation`, `Synonym`, `Antonym@Value`, `Derivation`, `Compound`, `Idiom`.
//!   Asnjë tekst suedisht nuk shkruhet: nga ana suedeze përdoren vetëm
//!   `MatchingID` / `VariantID` (si çelës kuptimi) dhe klasa e fjalës `Type`,
//!   e përkthyer në kod POS (n/v/a/r).
//!
//! Formati i TSV-së (`raw/lexin/lexin-sq.tsv`), një rresht për çdo fushë shqipe:
//!   kuptimi ⇥ pos ⇥ fusha ⇥ nën-id ⇥ teksti shqip (i papërpunuar, vetëm i pastruar nga tabulimet)
//! ku fusha ∈ {T (Translation), S (Synonym), A (Antonym), D (Derivation),
//! C (Compound), I (Idiom)}, kuptimi = `MatchingID` ose `v<VariantID>` kur mungon,
//! pos ∈ {n, v, a, r, -}, nën-id = atributi `ID` i elementit (D/C/I) ose `-`.
//! Ndarja në anëtarë (presje, pikëpresje, shënimet në kllapa, `(i,e)`) bëhet
//! nga importuesi pa varësi `nura_lingua::src_lexin`.

use crate::common::{self, Fetched};
use std::path::Path;

pub const SOURCE: &str = "lexin";
pub const URL_XML: &str = "https://sprakresurser.isof.se/lexin/albanska/swe_alb.xml";
pub const URL_META: &str = "https://sprakresurser.isof.se/lexin/albanska/metadata.xml";
pub const RAW_XML: &str = "raw/lexin/swe_alb.xml";
pub const RAW_META: &str = "raw/lexin/metadata.xml";
/// E njëjta rrugë si `nura_lingua::src_lexin::TSV`.
pub const TSV: &str = "raw/lexin/lexin-sq.tsv";
pub const LICENSE: &str = "CC BY 4.0";
pub const ATTRIBUTION: &str = "LEXIN, Institutet för språk och folkminnen (ISOF), CC BY 4.0";

/// Numërimet e nxjerrjes (për raportin dhe për SOURCES.lock).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stats {
    pub words: usize,
    pub without_target: usize,
    pub rows: usize,
    pub per_field: [usize; 6],
}

const FIELDS: [(&str, &str); 6] = [
    ("Translation", "T"),
    ("Synonym", "S"),
    ("Antonym", "A"),
    ("Derivation", "D"),
    ("Compound", "C"),
    ("Idiom", "I"),
];

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    println!("[lexin] shkarkoj {URL_META}");
    let meta = common::get(URL_META)?;
    println!("[lexin] shkarkoj {URL_XML} (≈18,7 MB, i plotë)");
    let xml = common::get(URL_XML)?;
    let meta_text = String::from_utf8_lossy(&meta).to_string();
    let published = meta_published(&meta_text);
    // Licenca duhet të jetë e shpallur nga botuesi; ndryshe nuk regjistrohet asgjë.
    match meta_has_cc_by_4(&meta_text) {
        true => {}
        false => return Err("lexin: metadata.xml nuk e shpall licencën CC BY 4.0 — ndalet".into()),
    }
    let xml_text = String::from_utf8(xml.clone()).map_err(|e| format!("lexin: swe_alb.xml nuk është UTF-8: {e}"))?;
    let (version, tsv, st) = extract(&xml_text, &published)?;
    let version_s = format!("LEXIN swe_alb.xml Version={version}; publikuar {published} (metadata.xml)");
    println!(
        "[lexin] {} hyrje <Word>, {} pa <TargetLang>; {} rreshta shqip (T {}, S {}, A {}, D {}, C {}, I {})",
        st.words, st.without_target, st.rows, st.per_field[0], st.per_field[1], st.per_field[2], st.per_field[3], st.per_field[4], st.per_field[5]
    );
    let check = "ISOF nuk publikon hash; SHA-256 është vetëm lokale";
    Ok(vec![
        common::record(data, SOURCE, RAW_META, &meta, URL_META, &format!("publikuar {published}"), LICENSE, ATTRIBUTION, check, "OK")?,
        common::record(data, SOURCE, RAW_XML, &xml, URL_XML, &version_s, LICENSE, ATTRIBUTION, check, "OK")?,
        common::record(
            data,
            SOURCE,
            TSV,
            tsv.as_bytes(),
            &format!("{URL_XML}#TargetLang"),
            &version_s,
            LICENSE,
            ATTRIBUTION,
            &format!("derivuar lokalisht nga {RAW_XML}: vetëm ana shqipe; {} rreshta", st.rows),
            "OK_EXTRACT_SQ",
        )?,
    ])
}

fn meta_has_cc_by_4(meta: &str) -> bool {
    let squeezed: String = meta.split_whitespace().collect::<Vec<_>>().join(" ");
    squeezed.contains("(CC BY 4.0)") || squeezed.contains("Attribution 4.0 International")
}

/// "Publicerad: 2020-03-09" → "2020-03-09" (ose "e panjohur").
fn meta_published(meta: &str) -> String {
    meta.split("Publicerad:")
        .nth(1)
        .map(|rest| rest.trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect::<String>())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "e panjohur".into())
}

fn pos_code(t: &str) -> &'static str {
    match t.trim() {
        "subst." => "n",
        "verb" => "v",
        "adj." => "a",
        "adv." => "r",
        _ => "-",
    }
}

/// Teksti i plotë i një elementi (bashkon nyjet tekst të pasardhësve).
fn text_of(n: roxmltree::Node) -> String {
    n.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect::<Vec<_>>().join("")
}

/// XML → (versioni, TSV, numërimet). Vetëm ana shqipe.
pub fn extract(xml: &str, published: &str) -> Result<(String, String, Stats), String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("lexin: XML i pavlefshëm: {e}"))?;
    let root = doc.root_element();
    match (root.tag_name().name(), root.attribute("TargetLang")) {
        ("Dictionary", Some("Albanian")) => {}
        (n, t) => return Err(format!("lexin: rrënja e papritur <{n}> TargetLang={t:?}")),
    }
    let version = root.attribute("Version").unwrap_or("e panjohur").to_string();
    let mut st = Stats::default();
    let mut out = format!(
        "# LEXIN suedisht–shqip (ISOF) — vetëm ana shqipe (TargetLang); Version={version}; publikuar {published}\n\
         # licenca: {LICENSE}; atribuimi: {ATTRIBUTION}\n\
         # kolonat: kuptimi\tpos\tfusha(T/S/A/D/C/I)\tnën-id\tteksti shqip\n"
    );
    for w in root.children().filter(|n| n.has_tag_name("Word")) {
        st.words += 1;
        let sense = match (w.attribute("MatchingID"), w.attribute("VariantID")) {
            (Some(m), _) => common::clean(m),
            (None, Some(v)) => format!("v{}", common::clean(v)),
            (None, None) => format!("w{}", st.words),
        };
        let pos = pos_code(w.attribute("Type").unwrap_or(""));
        let target = match w.children().find(|n| n.has_tag_name("TargetLang")) {
            Some(t) => t,
            None => {
                st.without_target += 1;
                continue;
            }
        };
        for c in target.children().filter(|n| n.is_element()) {
            let field = FIELDS.iter().position(|(tag, _)| c.has_tag_name(*tag));
            let text = match (field, c.has_tag_name("Antonym")) {
                (Some(_), true) => c.attribute("Value").map(|v| v.to_string()).unwrap_or_default(),
                (Some(_), false) => text_of(c),
                (None, _) => continue,
            };
            let text = common::clean(&text);
            match (field, text.is_empty()) {
                (Some(i), false) => {
                    let sub = c.attribute("ID").map(common::clean).unwrap_or_else(|| "-".into());
                    out.push_str(&format!("{sense}\t{pos}\t{}\t{sub}\t{text}\n", FIELDS[i].1));
                    st.per_field[i] += 1;
                    st.rows += 1;
                }
                _ => {}
            }
        }
    }
    Ok((version, out, st))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copëz në formatin real të `swe_alb.xml` (hyrjet «a», «à jour», «abstrakt»).
    const XML: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<Dictionary BaseLang="Swedish" TargetLang="Albanian" Version="2010-07-07" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://lexin.nada.kth.se/schema/LexinSchema.xsd">
  <Word ID="2" MatchingID="2" Type="subst." Value="a" Variant="2" VariantID="4">
    <BaseLang>
      <Meaning MatchingID="9011001">sjätte tonen i C-durskalan</Meaning>
      <Comment>musikterm</Comment>
      <Index Value="a" />
    </BaseLang>
    <TargetLang>
      <Translation>zëri i gjashtë në c-dur</Translation>
      <Synonym>nota, tingulli, toni</Synonym>
      <Comment>"(muz.)"</Comment>
    </TargetLang>
  </Word>
  <Word ID="4" Type="adj." Value="à jour" Variant="" VariantID="6">
    <BaseLang>
      <Meaning />
      <Idiom ID="3" MatchingID="1174829">hålla sig (el. vara) à jour (med något)</Idiom>
    </BaseLang>
    <TargetLang>
      <Translation>në dijeni</Translation>
      <Synonym>azhornuar (i,e), informuar (i,e)</Synonym>
      <Idiom ID="3">azhurnohem, vihem në dijeni (për diçka), informohem</Idiom>
    </TargetLang>
  </Word>
  <Word ID="9" MatchingID="301" Type="adj." Value="abstrakt" Variant="" VariantID="20">
    <BaseLang>
      <Meaning MatchingID="9011300">overklig</Meaning>
      <Antonym Value="konkret" />
    </BaseLang>
    <TargetLang>
      <Translation>abstrakt, -e</Translation>
      <Antonym Value="konkret, -e" />
      <Derivation ID="4">abstraksion</Derivation>
      <Compound ID="11" Description="x">art	abstrakt</Compound>
    </TargetLang>
  </Word>
  <Word ID="10" MatchingID="302" Type="verb" Value="x" Variant="" VariantID="21">
    <BaseLang><Meaning>bara svenska</Meaning></BaseLang>
  </Word>
</Dictionary>"#;

    #[test]
    fn extract_keeps_only_albanian_side() {
        let (v, tsv, st) = extract(XML, "2020-03-09").unwrap();
        assert_eq!(v, "2010-07-07");
        assert_eq!(st.words, 4);
        assert_eq!(st.without_target, 1);
        assert_eq!(st.rows, 9);
        assert_eq!(st.per_field, [3, 2, 1, 1, 1, 1]);
        assert!(tsv.contains("2\tn\tT\t-\tzëri i gjashtë në c-dur\n"));
        assert!(tsv.contains("2\tn\tS\t-\tnota, tingulli, toni\n"));
        // pa MatchingID → v<VariantID>
        assert!(tsv.contains("v6\ta\tS\t-\tazhornuar (i,e), informuar (i,e)\n"));
        assert!(tsv.contains("v6\ta\tI\t3\tazhurnohem, vihem në dijeni (për diçka), informohem\n"));
        assert!(tsv.contains("301\ta\tA\t-\tkonkret, -e\n"));
        assert!(tsv.contains("301\ta\tD\t4\tabstraksion\n"));
        // tabulimi brenda tekstit pastrohet
        assert!(tsv.contains("301\ta\tC\t11\tart abstrakt\n"));
        // asnjë tekst suedisht, asnjë koment
        for sv in ["sjätte", "musikterm", "hålla sig", "overklig", "bara svenska", "(muz.)"] {
            assert!(!tsv.contains(sv), "{sv}");
        }
    }

    #[test]
    fn extract_rejects_wrong_target_language() {
        let x = r#"<Dictionary BaseLang="Swedish" TargetLang="Bosnian" Version="1"></Dictionary>"#;
        assert!(extract(x, "-").is_err());
        assert!(extract("<nope", "-").is_err());
    }

    #[test]
    fn metadata_license_and_date_are_read() {
        let m = "<resurs>\n  <publication>\n   Publicerad: 2020-03-09\n  </publication>\n  <lc>\n   Creative Commons  Attribution 4.0 International (CC BY 4.0)\n  </lc>\n</resurs>";
        assert!(meta_has_cc_by_4(m));
        assert_eq!(meta_published(m), "2020-03-09");
        assert!(!meta_has_cc_by_4("<lc>CC BY-NC 4.0</lc>"));
        assert_eq!(meta_published("<x/>"), "e panjohur");
    }
}
