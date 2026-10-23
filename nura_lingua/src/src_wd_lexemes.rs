//! Importuesi i lexemave shqipe të Wikidata (CC0).
//!
//! TSV (nga nura_fetch), rreshta sipas llojit:
//! - `L ⇥ lexeme ⇥ lema ⇥ kategoria(QID) ⇥ gjinitë(QID,…)`
//! - `F ⇥ lexeme ⇥ forma-id ⇥ përfaqësimi ⇥ tiparet(QID,…)`
//! - `C ⇥ lexeme ⇥ përbërësi ⇥ lema_e_përbërësit ⇥ renditja`
//!
//! Çfarë krijohet:
//! - `Rel::Lex` (vetë-lak) me `pos` dhe `gender` (P5185) për emra, folje, mbiemra, ndajfolje;
//!   mbiemrat «i bukur» ruhen si «bukur» me `("article","i")`.
//! - `MorphEntry` për çdo formë me tipare UD të përkthyera nga QID-të e verifikuara
//!   në Wikidata Query Service (2026-10-22): Q110786 njëjës, Q146786 shumës,
//!   Q131105 emërore, Q146078 kallëzore, Q146233 gjinore, Q145599 dhanore,
//!   Q156986 rrjedhore, Q185077 thirrore, Q53997851 e shquar, Q53997857 e pashquar,
//!   Q499327/Q1775415/Q1775461 gjinia, Q814722 pjesore, Q192613 e tashme,
//!   Q21714344/Q51929049/Q51929074 veta 1/2/3.
//!   Tiparet e tjera (p.sh. aoristi Q216497, e tashmja dëftore Q56682909) nuk shpiken
//!   në UD: hiqen dhe numërohen. Shenja «lemë» Q18514 nuk është tipar: numërohet veçmas.
//!   Format e shënuara si gegërisht/toskërisht/arbërisht, arkaizëm ose të rralla nuk merren.
//!   Rasa nuk shtohet kurrë kur Wikidata nuk e jep.
//! - P5238: përbërës → `Sem("COMPOUND_OF")` (përbërja → përbërësi); kur ndër përbërësit
//!   ka ndajshtesë («-xhi», «mes-»), lidhja është `Sem("DERIVED_FROM")` drejt temës,
//!   me ndajshtesën si atribut (ndajshtesat nuk bëhen fjalë).
use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::{Morph, MorphEntry};
use std::collections::{BTreeMap, HashMap};

pub const TSV: &str = "raw/wd_lexemes/wd_lexemes-sq.tsv";

/// Kategoria leksikore → (kodi pos për Rel::Lex, UPOS).
fn category(q: &str) -> Option<(Option<&'static str>, &'static str)> {
    match q {
        "Q1084" => Some((Some("n"), "NOUN")),
        "Q24905" => Some((Some("v"), "VERB")),
        "Q34698" => Some((Some("a"), "ADJ")),
        "Q380057" => Some((Some("r"), "ADV")),
        "Q147276" => Some((None, "PROPN")),
        _ => None,
    }
}

/// Tipar Wikidata → (çelësi UD, vlera UD). `None` = tipar pa përkthim të sigurt.
fn feature(q: &str) -> Option<(&'static str, &'static str)> {
    match q {
        "Q110786" => Some(("Number", "Sing")),
        "Q146786" => Some(("Number", "Plur")),
        "Q131105" => Some(("Case", "Nom")),
        "Q146078" => Some(("Case", "Acc")),
        "Q146233" => Some(("Case", "Gen")),
        "Q145599" => Some(("Case", "Dat")),
        "Q156986" => Some(("Case", "Abl")),
        "Q185077" => Some(("Case", "Voc")),
        "Q53997851" => Some(("Definite", "Def")),
        "Q53997857" => Some(("Definite", "Ind")),
        "Q499327" => Some(("Gender", "Masc")),
        "Q1775415" => Some(("Gender", "Fem")),
        "Q1775461" => Some(("Gender", "Neut")),
        "Q814722" => Some(("VerbForm", "Part")),
        "Q192613" => Some(("Tense", "Pres")),
        "Q21714344" => Some(("Person", "1")),
        "Q51929049" => Some(("Person", "2")),
        "Q51929074" => Some(("Person", "3")),
        _ => None,
    }
}

/// Tipare që e nxjerrin formën jashtë standardit (dialekt, arkaizëm, e rrallë).
fn non_standard(q: &str) -> bool {
    matches!(q, "Q181037" | "Q180937" | "Q29347" | "Q181970" | "Q58157328")
}

fn gender_code(qs: &str) -> Option<&'static str> {
    let has = |q: &str| qs.split(',').any(|x| x == q);
    match (has("Q499327"), has("Q1775415"), has("Q1775461")) {
        (true, false, false) => Some("m"),
        (false, true, false) => Some("f"),
        (false, false, true) => Some("n"),
        (true, true, false) => Some("mf"),
        _ => None,
    }
}

fn ud_gender(code: &str) -> Option<&'static str> {
    match code {
        "m" => Some("Masc"),
        "f" => Some("Fem"),
        "n" => Some("Neut"),
        _ => None,
    }
}

/// Lema e fjalës: shkronja të vogla; nyja e mbiemrit («i»/«e») ndahet.
pub fn split_article(lemma: &str, upos: &str) -> (String, Option<&'static str>) {
    let l = lemma.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let art = match upos {
        "ADJ" => match (l.strip_prefix("i "), l.strip_prefix("e ")) {
            (Some(rest), _) => Some((rest.to_string(), "i")),
            (_, Some(rest)) => Some((rest.to_string(), "e")),
            _ => None,
        },
        _ => None,
    };
    match art {
        Some((rest, a)) => match rest.is_empty() {
            false => (rest, Some(a)),
            true => (l, None),
        },
        None => (l, None),
    }
}

struct Lexeme {
    lemma: String,
    upos: &'static str,
    gender: Option<&'static str>,
}

pub fn import(g: &mut Graph, morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let rows: Vec<Vec<&str>> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split('\t').map(|c| c.trim()).collect())
        .collect();
    let col = |r: &Vec<&str>, i: usize| -> String { r.get(i).map(|s| s.to_string()).unwrap_or_default() };
    let mut lex: HashMap<String, Lexeme> = HashMap::new();
    let (mut lex_edges, mut other_cat) = (0usize, 0usize);
    for r in rows.iter().filter(|r| r.first() == Some(&"L")) {
        let (id, lemma, cat, genders) = (col(r, 1), col(r, 2), col(r, 3), col(r, 4));
        match (id.is_empty() || lemma.is_empty(), category(&cat)) {
            (false, Some((pos, upos))) => {
                let (lemma, article) = split_article(&lemma, upos);
                let gender = gender_code(&genders);
                let w = g.ensure_word(Lang::Sq, &lemma);
                let mut attrs: Vec<(String, String)> = Vec::new();
                match pos {
                    Some(p) => attrs.push(("pos".into(), p.into())),
                    None => {}
                }
                match gender {
                    Some(gc) => attrs.push(("gender".into(), gc.into())),
                    None => {}
                }
                match article {
                    Some(a) => attrs.push(("article".into(), a.into())),
                    None => {}
                }
                match attrs.is_empty() {
                    true => {}
                    false => {
                        attrs.push(("src".into(), format!("wikidata:{id}")));
                        g.add_edge(Edge {
                            from: w,
                            to: w,
                            rel: Rel::Lex,
                            source: Source::WdLexemes,
                            lang: Some(Lang::Sq),
                            method: Method::Published,
                            attrs,
                        });
                        lex_edges += 1;
                    }
                }
                lex.insert(id, Lexeme { lemma, upos, gender });
            }
            (false, None) => other_cat += 1,
            _ => {}
        }
    }
    // Format.
    let (mut forms, mut skipped_forms, mut dropped_feats, mut lemma_marks) = (0usize, 0usize, 0usize, 0usize);
    for r in rows.iter().filter(|r| r.first() == Some(&"F")) {
        let (id, fid, rep_form, feats) = (col(r, 1), col(r, 2), col(r, 3), col(r, 4));
        let lx = match (lex.get(&id), rep_form.is_empty()) {
            (Some(lx), false) => lx,
            _ => {
                skipped_forms += 1;
                continue;
            }
        };
        let qs: Vec<&str> = feats.split(',').filter(|q| !q.is_empty()).collect();
        match qs.iter().any(|q| non_standard(q)) {
            true => {
                skipped_forms += 1;
                continue;
            }
            false => {}
        }
        let mut ud: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for q in &qs {
            match (feature(q), *q == "Q18514") {
                (Some((k, v)), _) => ud.entry(k).or_default().push(v),
                (None, true) => lemma_marks += 1,
                (None, false) => dropped_feats += 1,
            }
        }
        match ud.is_empty() {
            true => {
                skipped_forms += 1;
                continue;
            }
            false => {}
        }
        // Gjinia e lexemës (P5185) vlen për format e emrit kur forma vetë nuk e jep.
        match (matches!(lx.upos, "NOUN" | "PROPN"), ud.contains_key("Gender"), lx.gender.and_then(ud_gender)) {
            (true, false, Some(gv)) => {
                ud.insert("Gender", vec![gv]);
            }
            _ => {}
        }
        let feats_ud = ud
            .iter_mut()
            .map(|(k, v)| {
                v.sort();
                v.dedup();
                format!("{k}={}", v.join(","))
            })
            .collect::<Vec<_>>()
            .join("|");
        morph.add(MorphEntry {
            lemma: lx.lemma.clone(),
            form: rep_form,
            upos: lx.upos.to_string(),
            feats: feats_ud,
            sent_id: format!("wd_lexemes:{fid}"),
            source: Source::WdLexemes,
        });
        forms += 1;
    }
    // Përbërësit (P5238), të grupuar sipas lexemës.
    let mut comps: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.first() == Some(&"C")) {
        let (id, cid, cl) = (col(r, 1), col(r, 2), col(r, 3));
        match (id.is_empty(), cl.is_empty()) {
            (false, false) => comps.entry(id).or_default().push((cid, cl.to_lowercase())),
            _ => {}
        }
    }
    let (mut compound, mut derived, mut comp_missing) = (0usize, 0usize, 0usize);
    for (id, parts) in &comps {
        let lx = match lex.get(id) {
            Some(lx) => lx,
            None => {
                comp_missing += 1;
                continue;
            }
        };
        let is_affix = |s: &str| s.starts_with('-') || s.ends_with('-');
        let affixes: Vec<&str> = parts.iter().filter(|(_, l)| is_affix(l)).map(|(_, l)| l.as_str()).collect();
        let rel = match affixes.is_empty() {
            true => "COMPOUND_OF",
            false => "DERIVED_FROM",
        };
        let from = g.ensure_word(Lang::Sq, &lx.lemma);
        for (cid, cl) in parts.iter().filter(|(_, l)| !is_affix(l)) {
            let to = g.ensure_word(Lang::Sq, cl);
            match from == to {
                true => continue,
                false => {}
            }
            let mut attrs = vec![("src".to_string(), format!("wikidata:{id}")), ("component".to_string(), format!("wikidata:{cid}"))];
            match affixes.is_empty() {
                true => {}
                false => attrs.push(("affix".into(), affixes.join(" "))),
            }
            g.add_edge(Edge {
                from,
                to,
                rel: Rel::Sem(rel.to_string()),
                source: Source::WdLexemes,
                lang: Some(Lang::Sq),
                method: Method::Published,
                attrs,
            });
            match rel {
                "COMPOUND_OF" => compound += 1,
                _ => derived += 1,
            }
        }
    }
    rep.note(format!(
        "wd_lexemes: {} lexema shqipe (emër/folje/mbiemër/ndajfolje/emër i përveçëm), {other_cat} të kategorive të tjera të pa-importuara; {lex_edges} skaje Lex (pos/gjini); {forms} forma morfologjike, {skipped_forms} forma të anashkaluara (jo standarde / pa tipare), {dropped_feats} tipare pa përkthim UD të hequra, {lemma_marks} shenja «lemë» (Q18514) të anashkaluara; {compound} COMPOUND_OF, {derived} DERIVED_FROM (me ndajshtesë) nga P5238; {comp_missing} përbërje pa lexemë të njohur",
        lex.len()
    ));
    lex_edges + forms + compound + derived
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morph::Gender;

    // Rreshta të vërtetë të Wikidata (L590679 «mal», L591630 «lule», L1364563 «duhanxhi»).
    const FIX: &str = "# wd_lexemes-sq.tsv — test\n\
L\tL590679\tmal\tQ1084\tQ499327\n\
L\tL591630\tlule\tQ1084\tQ1775415\n\
L\tL1\ti bukur\tQ34698\t\n\
L\tL1364563\tduhanxhi\tQ1084\t\n\
L\tL1358785\tduhan\tQ1084\t\n\
L\tL1362039\tmesnatë\tQ1084\tQ1775415\n\
L\tL9\tdhe\tQ36484\t\n\
F\tL590679\tL590679-F6\tmali\tQ110786,Q131105,Q53997851\n\
F\tL590679\tL590679-F8\tmalit\tQ110786,Q145599,Q146233,Q156986,Q53997851\n\
F\tL590679\tL590679-F9\tmalet\tQ131105,Q146078,Q146786,Q53997851\n\
F\tL591630\tL591630-F1\tlulja\tQ110786,Q131105,Q53997851\n\
F\tL591630\tL591630-F9\tlulja\tQ110786,Q131105,Q181037,Q53997851\n\
F\tL591630\tL591630-F4\tluleve\tQ145599,Q146786\n\
F\tL1\tL1-F1\tbukurit\tQ216497\n\
F\tL404\tL404-F1\tasgjë\tQ110786\n\
C\tL1364563\tL1358785\tduhan\t1\n\
C\tL1364563\tL1408052\t-xhi\t\n\
C\tL1362039\tL1359041\tnatë\t\n\
C\tL1362039\tL1417683\tmes-\t\n\
C\tL77\tL1\tbukur\t\n\
F\tbroken\n";

    #[test]
    fn imports_lex_forms_compounds() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        import(&mut g, &mut m, FIX, &mut r);
        let mal = g.get(&word_key(Lang::Sq, "mal")).unwrap();
        let lex = g.out_edges(mal).find(|(_, e)| e.rel == Rel::Lex).unwrap().1;
        assert_eq!(lex.attr("pos"), Some("n"));
        assert_eq!(lex.attr("gender"), Some("m"));
        assert_eq!(lex.from, lex.to);
        // Mbiemri pa nyjë, nyja si atribut.
        let bukur = g.get(&word_key(Lang::Sq, "bukur")).unwrap();
        assert!(g.get(&word_key(Lang::Sq, "i bukur")).is_none());
        let bl = g.out_edges(bukur).find(|(_, e)| e.rel == Rel::Lex).unwrap().1;
        assert_eq!(bl.attr("article"), Some("i"));
        assert_eq!(bl.attr("pos"), Some("a"));
        // Kategoritë e tjera nuk importohen.
        assert!(g.get(&word_key(Lang::Sq, "dhe")).is_none());
        // Morfologjia: forma e shquar emërore njëjës e atestuar.
        let d = m.definite_nom_sg("mal");
        assert_eq!(d.text, "mali");
        assert_eq!(m.gender("mal"), Some(Gender::Masc));
        assert_eq!(m.definite_nom_sg("lule").text, "lulja");
        let malit = m.entries_of("mal").into_iter().find(|e| e.form == "malit").unwrap();
        assert_eq!(malit.feats, "Case=Abl,Dat,Gen|Definite=Def|Gender=Masc|Number=Sing");
        // Rasa nuk shpiket: «luleve» pa shquarsi.
        let luleve = m.entries_of("lule").into_iter().find(|e| e.form == "luleve").unwrap();
        assert_eq!(luleve.feats, "Case=Dat|Gender=Fem|Number=Plur");
        // Forma gege dhe forma vetëm me aorist nuk merren.
        assert_eq!(m.entries_of("lule").len(), 2);
        assert!(m.entries_of("bukur").is_empty());
        // P5238 me ndajshtesë → DERIVED_FROM; pa ndajshtesë do të ishte COMPOUND_OF.
        let dx = g.get(&word_key(Lang::Sq, "duhanxhi")).unwrap();
        let duhan = g.get(&word_key(Lang::Sq, "duhan")).unwrap();
        let e = g.out_edges(dx).find(|(_, e)| e.to == duhan).unwrap().1;
        assert_eq!(e.rel, Rel::Sem("DERIVED_FROM".into()));
        assert_eq!(e.attr("affix"), Some("-xhi"));
        assert!(g.get(&word_key(Lang::Sq, "-xhi")).is_none());
        let mn = g.get(&word_key(Lang::Sq, "mesnatë")).unwrap();
        assert!(g.out_edges(mn).any(|(_, e)| e.rel == Rel::Sem("DERIVED_FROM".into())));
    }

    #[test]
    fn pure_compound() {
        let t = "L\tL5\tkryeqytet\tQ1084\tQ499327\nC\tL5\tL6\tkrye\t1\nC\tL5\tL7\tqytet\t2\n";
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        import(&mut g, &mut m, t, &mut r);
        let k = g.get(&word_key(Lang::Sq, "kryeqytet")).unwrap();
        assert_eq!(g.out_edges(k).filter(|(_, e)| e.rel == Rel::Sem("COMPOUND_OF".into())).count(), 2);
    }

    #[test]
    fn empty_and_partial() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, "", &mut r), 0);
        assert_eq!(import(&mut g, &mut m, "L\nF\t\t\t\nC\tL1\n# x\n", &mut r), 0);
        assert!(g.nodes.is_empty());
    }
}
