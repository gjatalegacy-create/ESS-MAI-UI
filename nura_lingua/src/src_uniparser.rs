//! Importuesi i burimit «uniparser» (leksiku gramatikor shqip i
//! uniparser-grammar-albanian 2.3.0, MIT — Morozova, Rusakov, Arkhangelskiy).
//!
//! Lexon ekstraktin TSV të `nura_fetch --only uniparser` (i verifikuar me
//! SHA-256 nga `store::import_all`) dhe krijon:
//! - nyje `Word(sq, lemë)` — lema me shkronja të vogla, pa nyje;
//! - një vetë-lak `Rel::Lex` për çdo leksemë me `pos/gender/anim/trans/med/borr/flags`;
//! - hyrje morfologjike me gjininë për emrat (`Gender=Masc|Fem|Neut`, `sent_id = "uniparser"`);
//! - formën e shquar emërore njëjës VETËM kur fetch-i e ka gjeneruar nga një
//!   paradigmë e validuar kundrejt UniMorph (kolona `def_nom_sg`);
//! - `DIALECT_VARIANT_OF` (formë gege/jostandarde → lema standarde nga `lexref`),
//!   `RELATED` (variant standard me `lexref`), `DERIVED_FROM` (fusha `deriv`).
//!
//! Leksemat gege/jostandarde pa `lexref`, shkurtimet dhe format e çrregullta nuk
//! bëhen lema. Kolona `trans_en_bridge` është anglisht dhe NUK importohet si shqip.

use crate::graph::{Edge, Graph, Lang, Method, Rel, Source};
use crate::import::ImportReport;
use crate::morph::{Morph, MorphEntry};
use std::collections::{BTreeMap, HashSet};

pub const TSV: &str = "raw/uniparser/uniparser-sq.tsv";

/// Numri i kolonave të ekstraktit (shih `nura_fetch::src_uniparser::TSV_HEADER`).
const COLS: usize = 18;

struct Row<'a> {
    lex: &'a str,
    lemma: &'a str,
    upos: &'a str,
    pos: &'a str,
    gender: &'a str,
    anim: &'a str,
    trans: &'a str,
    med: &'a str,
    borr: &'a str,
    flags: &'a str,
    status: &'a str,
    lexref: &'a str,
    deriv: &'a str,
    def_nom_sg: &'a str,
    def_check: &'a str,
    /// Përkthimi anglisht i leksemës (urë, kurrë nuk thuhet si shqip): dallon homografët.
    en: &'a str,
    /// Klasa e paradigmës dhe temat (vetëm për foljet): baza e lakimit me rregull.
    paradigm: &'a str,
    stems: &'a str,
}

fn val(s: &str) -> Option<&str> {
    match s.trim() {
        "" | "-" => None,
        v => Some(v),
    }
}

fn parse_row(line: &str) -> Option<Row<'_>> {
    let c: Vec<&str> = line.split('\t').collect();
    match c.len() == COLS {
        true => Some(Row {
            lex: c[0],
            lemma: c[1],
            upos: c[2],
            pos: c[3],
            gender: c[4],
            anim: c[5],
            trans: c[6],
            med: c[7],
            borr: c[8],
            flags: c[9],
            status: c[10],
            lexref: c[13],
            deriv: c[14],
            def_nom_sg: c[15],
            def_check: c[16],
            en: c[17],
            paradigm: c[11],
            stems: c[12],
        }),
        false => None,
    }
}

/// Identiteti i fjalës: shkronja të vogla, pa nyjën e mbiemrit, hapësira të njëfishta.
fn norm_lemma(s: &str) -> Option<String> {
    let l = s.trim().to_lowercase();
    let l = ["i ", "e ", "të ", "së "]
        .iter()
        .find_map(|a| l.strip_prefix(a))
        .map(|x| x.to_string())
        .unwrap_or(l);
    let l = l.split_whitespace().collect::<Vec<_>>().join(" ");
    match l.chars().any(|c| c.is_alphabetic()) {
        true => Some(l),
        false => None,
    }
}

fn gender_feat(g: &str) -> Option<&'static str> {
    match g {
        "m" => Some("Gender=Masc"),
        "f" => Some("Gender=Fem"),
        // «mf»: mashkullor në njëjës, femëror në shumës.
        "mf" => Some("Gender=Masc|Number=Sing"),
        "n" => Some("Gender=Neut"),
        _ => None,
    }
}

fn sing_gender(g: &str) -> Option<&'static str> {
    match g {
        "m" | "mf" => Some("Masc"),
        "f" => Some("Fem"),
        "n" => Some("Neut"),
        _ => None,
    }
}

fn sem(g: &mut Graph, from: u32, to: u32, rel: &str, attrs: Vec<(String, String)>) -> bool {
    let before = g.edges.len();
    g.add_edge(Edge {
        from,
        to,
        rel: Rel::Sem(rel.into()),
        source: Source::Uniparser,
        lang: Some(Lang::Sq),
        method: Method::Published,
        attrs,
    });
    g.edges.len() > before
}

fn lex_attrs(r: &Row) -> Vec<(String, String)> {
    let mut a: Vec<(String, String)> = Vec::new();
    for (k, v) in [
        ("pos", r.pos),
        ("gender", r.gender),
        ("anim", r.anim),
        ("trans", r.trans),
        ("med", r.med),
        ("borr", r.borr),
        ("flags", r.flags),
        ("en", r.en),
    ] {
        match val(v) {
            Some(x) => a.push((k.into(), x.into())),
            None => {}
        }
    }
    match (r.upos, val(r.paradigm), val(r.stems)) {
        ("V", Some(p), Some(s)) => {
            a.push(("par".into(), p.into()));
            a.push(("stems".into(), s.into()));
        }
        _ => {}
    }
    a.push(("src".into(), "uniparser".into()));
    a
}

pub fn import(g: &mut Graph, morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let mut c: BTreeMap<&'static str, usize> = BTreeMap::new();
    let bump = |c: &mut BTreeMap<&'static str, usize>, k: &'static str| *c.entry(k).or_default() += 1;
    let mut seen_morph: HashSet<(String, String, String)> = HashSet::new();
    let mut words: HashSet<u32> = HashSet::new();
    let nodes_before = g.nodes.len();
    let edges_before = g.edges.len();
    let morph_before = morph.entries.len();

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        match (line.starts_with('#') || line.trim().is_empty(), line.starts_with("lex\tlemma\t")) {
            (true, _) | (false, true) => continue,
            _ => {}
        }
        let r = match parse_row(line) {
            Some(r) => r,
            None => {
                bump(&mut c, "malformed");
                continue;
            }
        };
        bump(&mut c, "rows");
        let lemma = match norm_lemma(r.lemma) {
            Some(l) => l,
            None => {
                bump(&mut c, "skip_empty_lemma");
                continue;
            }
        };
        let standard_ref = val(r.lexref).and_then(norm_lemma).filter(|x| *x != lemma);
        match (r.status, standard_ref.is_some()) {
            ("std", _) => {}
            ("gheg" | "nonst", true) => {}
            ("gheg", false) => {
                bump(&mut c, "skip_gheg_no_lexref");
                continue;
            }
            ("nonst", false) => {
                bump(&mut c, "skip_nonst_no_lexref");
                continue;
            }
            ("abbr", _) => {
                bump(&mut c, "skip_abbr");
                continue;
            }
            _ => {
                bump(&mut c, "skip_badform");
                continue;
            }
        }
        let w = g.ensure_word(Lang::Sq, &lemma);
        words.insert(w);
        let mut attrs = lex_attrs(&r);
        match r.status {
            "std" => {}
            other => attrs.push(("variety".into(), other.into())),
        }
        let before = g.edges.len();
        g.add_edge(Edge {
            from: w,
            to: w,
            rel: Rel::Lex,
            source: Source::Uniparser,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs,
        });
        match g.edges.len() > before {
            true => bump(&mut c, "lex_edges"),
            false => {}
        }

        // Format gege/jostandarde: vetëm lidhja me lemën standarde, pa morfologji.
        match (r.status, &standard_ref) {
            ("gheg" | "nonst", Some(std_lemma)) => {
                let t = g.ensure_word(Lang::Sq, std_lemma);
                match sem(g, w, t, "DIALECT_VARIANT_OF", vec![
                    ("variety".into(), r.status.into()),
                    ("src".into(), "uniparser lexref".into()),
                ]) {
                    true => bump(&mut c, "dialect_variant_edges"),
                    false => {}
                }
                continue;
            }
            ("std", Some(std_lemma)) => {
                let t = g.ensure_word(Lang::Sq, std_lemma);
                match sem(g, w, t, "RELATED", vec![
                    ("kind".into(), "variant".into()),
                    ("src".into(), "uniparser lexref".into()),
                ]) {
                    true => bump(&mut c, "variant_related_edges"),
                    false => {}
                }
            }
            _ => {}
        }
        match val(r.deriv).and_then(norm_lemma).filter(|b| *b != lemma) {
            Some(base) => {
                let t = g.ensure_word(Lang::Sq, &base);
                match sem(g, w, t, "DERIVED_FROM", vec![("src".into(), "uniparser deriv".into())]) {
                    true => bump(&mut c, "derived_from_edges"),
                    false => {}
                }
            }
            None => {}
        }

        // Morfologjia e emrave: gjinia dhe (kur është e validuar) forma e shquar.
        let proper = r.lex.chars().next().map(|ch| ch.is_uppercase()).unwrap_or(false);
        let upos = match proper {
            true => "PROPN",
            false => "NOUN",
        };
        let form_base = match proper {
            true => r.lex.trim().to_string(),
            false => lemma.clone(),
        };
        match (r.upos == "NOUN", gender_feat(r.gender)) {
            (true, Some(feats)) => {
                match seen_morph.insert((lemma.clone(), form_base.clone(), feats.to_string())) {
                    true => {
                        morph.add(MorphEntry {
                            lemma: lemma.clone(),
                            form: form_base.clone(),
                            upos: upos.into(),
                            feats: feats.into(),
                            sent_id: "uniparser".into(),
                            source: Source::Uniparser,
                        });
                        bump(&mut c, "morph_gender");
                    }
                    false => {}
                }
                match (
                    val(r.def_nom_sg).filter(|d| plausible_definite(&lemma, d)),
                    r.def_check.starts_with("validated:"),
                    sing_gender(r.gender),
                ) {
                    (Some(def), true, Some(gs)) => {
                        let feats = format!("Case=Nom|Definite=Def|Gender={gs}|Number=Sing");
                        let form = match proper {
                            true => capitalize_first(def),
                            false => def.to_lowercase(),
                        };
                        match seen_morph.insert((lemma.clone(), form.clone(), feats.clone())) {
                            true => {
                                morph.add(MorphEntry {
                                    lemma: lemma.clone(),
                                    form,
                                    upos: upos.into(),
                                    feats,
                                    sent_id: "uniparser".into(),
                                    source: Source::Uniparser,
                                });
                                bump(&mut c, "morph_def_nom_sg");
                            }
                            false => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let words_n = words.len();
    rep.note(format!(
        "uniparser: {} rreshta; {} fjalë (lema) të prekura, {} nyje të reja, {} skaje të reja, {} hyrje morfologjike të reja",
        c.get("rows").copied().unwrap_or(0),
        words_n,
        g.nodes.len() - nodes_before,
        g.edges.len() - edges_before,
        morph.entries.len() - morph_before
    ));
    rep.note(format!(
        "uniparser: {}",
        c.iter().map(|(k, n)| format!("{k}={n}")).collect::<Vec<_>>().join(", ")
    ));
    words_n
}

fn capitalize_first(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        Some(f) => format!("{}{}", f.to_uppercase(), it.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::word_key;
    use crate::morph::FormOrigin;

    // Rreshta në formatin e saktë të ekstraktit të nura_fetch (18 kolona).
    const TSV_FIXTURE: &str = "# uniparser-albanian 2.3.0 (data_strict) — MIT\n\
lex\tlemma\tupos\tpos\tgender\tanim\ttrans\tmed\tborr\tflags\tstatus\tparadigms\tstems\tlexref\tderiv\tdef_nom_sg\tdef_check\ttrans_en_bridge\n\
kalë\tkalë\tNOUN\tn\tm\tanim\t-\t-\tlat\t-\tstd\t1b,5a\tkuaj.|kalë.|kal.\t-\t-\tkali\tvalidated:1b:120/122\thorse\n\
lule\tlule\tNOUN\tn\tf\tinanim\t-\t-\t-\talb\tstd\t3b,5a\tlule.|lule.|lulej.|lulj.\t-\t-\tlulja\tvalidated:3b:40/41\tflower\n\
mendim\tmendim\tNOUN\tn\tmf\tinanim\t-\t-\t-\talb,deriv\tstd\t1a,5a\tmendime.|mendim.\t-\t-\t-\trejected:1a:10/12\tthought\n\
djalë\tdjalë\tNOUN\tn\tm\tanim\t-\t-\t-\tnonst,alb\tnonst\t5a,5b\tdjelm.\t-\t-\t-\tnot_standard\tboy, son\n\
Shqypni\tshqypni\tNOUN\tn\tf\tinanim\t-\t-\tlat\tnonst,deriv,Gheg\tgheg\t3d,5b\tshqypni.|shqypni.\tShqipëri\t-\t-\tnot_standard\tAlbania\n\
Shqipëri\tshqipëri\tNOUN\tn\tf\tinanim\t-\t-\tlat\tderiv\tstd\t3d,5b\tshqipëri.|shqipëri.\t-\t-\tshqipëria\tvalidated:3d:30/30\tAlbania\n\
punoj\tpunoj\tV\tv\t-\t-\tvt,vi\t-\t-\talb,deriv\tstd\tlexoj\tpuno.|punua.|punu.|pun.\t-\t-\t-\t-\twork\n\
bukur\tbukur\tA\ta\t-\t-\t-\t-\t-\talb\tstd\tadj-2-2\tbukur.|bukur.\t-\t-\t-\t-\tbeautiful\n\
anësisht\tanësisht\tADV\tr\t-\t-\t-\t-\t-\tnonst,alb,deriv\tstd\tunchangeable\tanësisht.\t-\tanë\t-\t-\tlaterally\n\
AFP\tafp\tNOUN\tn\tf\t-\t-\t-\t-\tabbr\tabbr\tabbreviation-f-1\tafp.//afp-.\t-\t-\t-\tnot_standard\tAgence France-Presse\n\
kalë\tkalë\tNOUN\tn\tm\tanim\t-\t-\tlat\t-\tstd\t1b,5a\tkuaj.|kalë.|kal.\t-\t-\tkali\tvalidated:1b:120/122\thorse\n";

    fn run() -> (Graph, Morph, ImportReport, usize) {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let n = import(&mut g, &mut m, TSV_FIXTURE, &mut r);
        (g, m, r, n)
    }

    #[test]
    fn words_and_lex_self_loops() {
        let (g, _, _, n) = run();
        // kalë, lule, mendim, shqypni, shqipëri, punoj, bukur, anësisht (+ anë si bazë, jo e prekur si lemë)
        assert_eq!(n, 8);
        let w = g.get(&word_key(Lang::Sq, "kalë")).unwrap();
        let lex: Vec<&Edge> = g.out_edges(w).filter(|(_, e)| e.rel == Rel::Lex).map(|(_, e)| e).collect();
        assert_eq!(lex.len(), 1, "rreshti i dyfishtë nuk krijon skaj të dytë");
        assert_eq!(lex[0].to, w);
        assert_eq!(lex[0].attr("pos"), Some("n"));
        assert_eq!(lex[0].attr("gender"), Some("m"));
        assert_eq!(lex[0].attr("anim"), Some("anim"));
        assert_eq!(lex[0].attr("borr"), Some("lat"));
        let p = g.get(&word_key(Lang::Sq, "punoj")).unwrap();
        let e = g.out_edges(p).find(|(_, e)| e.rel == Rel::Lex).unwrap().1;
        assert_eq!(e.attr("trans"), Some("vt,vi"));
        assert!(g.get(&word_key(Lang::Sq, "djalë")).is_none(), "jostandard pa lexref nuk bëhet lemë");
        assert!(g.get(&word_key(Lang::Sq, "afp")).is_none(), "shkurtimet nuk bëhen lema");
    }

    #[test]
    fn gender_and_validated_definite_form() {
        let (_, m, _, _) = run();
        assert_eq!(m.gender("kalë"), Some(crate::morph::Gender::Masc));
        assert_eq!(m.gender("lule"), Some(crate::morph::Gender::Fem));
        let r = m.definite_nom_sg("kalë");
        assert_eq!(r.text, "kali");
        assert!(matches!(r.origin, FormOrigin::Attested { source: Source::Uniparser, .. }));
        assert_eq!(m.definite_nom_sg("lule").text, "lulja");
        // Paradigma e refuzuar: asnjë formë e shquar nga uniparser, vetëm gjinia.
        assert!(!m.entries_of("mendim").iter().any(|e| e.feats.contains("Definite=Def")));
        // «mf»: mashkullor në njëjës.
        assert_eq!(m.gender("mendim"), Some(crate::morph::Gender::Masc));
    }

    #[test]
    fn dialect_variant_and_derivation_edges() {
        let (g, m, _, _) = run();
        let gh = g.get(&word_key(Lang::Sq, "shqypni")).unwrap();
        let st = g.get(&word_key(Lang::Sq, "shqipëri")).unwrap();
        assert!(g.out_edges(gh).any(|(_, e)| e.rel == Rel::Sem("DIALECT_VARIANT_OF".into()) && e.to == st));
        let flagged = g.out_edges(gh).find(|(_, e)| e.rel == Rel::Lex).unwrap().1;
        assert_eq!(flagged.attr("variety"), Some("gheg"));
        assert!(m.entries_of("shqypni").is_empty(), "format gege nuk marrin morfologji");
        // Emër i përveçëm: PROPN me formën me shkronjë të madhe.
        assert!(m.entries_of("shqipëri").iter().any(|e| e.upos == "PROPN" && e.form == "Shqipëria"));
        let a = g.get(&word_key(Lang::Sq, "anësisht")).unwrap();
        let b = g.get(&word_key(Lang::Sq, "anë")).unwrap();
        assert!(g.out_edges(a).any(|(_, e)| e.rel == Rel::Sem("DERIVED_FROM".into()) && e.to == b));
    }

    #[test]
    fn english_bridge_is_never_imported() {
        let (g, m, _, _) = run();
        assert!(g.get(&word_key(Lang::Sq, "horse")).is_none());
        assert!(g.nodes.iter().all(|n| n.labels.iter().all(|t| t.text != "horse")));
        assert!(m.entries.iter().all(|e| e.form != "horse"));
    }

    #[test]
    fn malformed_rows_are_counted_not_fatal() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let n = import(&mut g, &mut m, "a\tb\tc\n", &mut r);
        assert_eq!(n, 0);
        assert!(r.lines.iter().any(|l| l.contains("malformed=1")));
    }
}

/// Kontroll i thjeshtë forme për trajtën e shquar emërore njëjës të gjeneruar nga paradigma:
/// hedh poshtë gabimet e temës në burim (p.sh. «heshtaa», «cibëi», «adrelnalina», «evropa» për «europë»).
/// Forma duhet të ruajë temën e lemës (pa -ë/-e fundore) dhe të mos mbarojë me "aa", "ëi" ose "ëa".
pub fn plausible_definite(lemma: &str, form: &str) -> bool {
    let l: Vec<char> = lemma.to_lowercase().chars().collect();
    let f = form.to_lowercase();
    // -ë/-e fundore bie (kalë → kali); -ë- para bashkëtingëllores fundore bie (libër → libri).
    let n = l.len();
    let stem_len = match (l.last(), n >= 2 && l[n - 2] == 'ë') {
        (Some('ë') | Some('e'), _) => n.saturating_sub(1),
        (_, true) => n.saturating_sub(2),
        _ => n,
    };
    let common = l.iter().zip(f.chars()).take_while(|(a, b)| *a == b).count();
    let bad_end = f.ends_with("aa") || f.ends_with("ëi") || f.ends_with("ëa");
    common + 1 >= stem_len && !bad_end && f.chars().count() > l.len().saturating_sub(1)
}

#[cfg(test)]
mod plausible_tests {
    use super::plausible_definite;

    #[test]
    fn rejects_known_source_errors_and_keeps_regular_forms() {
        for (l, f) in [("heshtë", "heshtaa"), ("cibë", "cibëi"), ("adrenalinë", "adrelnalina"), ("europë", "evropa"), ("ikë", "ikaa")] {
            assert!(!plausible_definite(l, f), "{l} → {f}");
        }
        for (l, f) in [("kalë", "kali"), ("qen", "qeni"), ("lule", "lulja"), ("shtëpi", "shtëpia"), ("mik", "miku"), ("libër", "libri"), ("vajzë", "vajza")] {
            assert!(plausible_definite(l, f), "{l} → {f}");
        }
    }
}
