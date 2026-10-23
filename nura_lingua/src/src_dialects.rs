//! Importuesi i formave dialektore (FjaloriShqip, Kristoforidhi 1904).
//!
//! TSV (nga nura_fetch): `forma_dialektore ⇥ dialekti ⇥ klasa ⇥ gjinia ⇥ forma_standarde`.
//! - `Sem("DIALECT_VARIANT_OF")` nga forma dialektore te lema standarde, VETËM kur lema
//!   standarde ekziston tashmë si nyje `Word` (asnjë lemë standarde nuk shpiket).
//! - Klasa/gjinia e formës dialektore → `Rel::Lex` mbi formën dialektore.
//! - Kuptimi anglisht nuk vjen fare në këtë TSV.
//!
//! Shënim: nura_fetch e refuzon aktualisht këtë burim (commit-et e transkriptimit kanë
//! bashkautor AI); importuesi mbetet gati për një TSV të të njëjtit format nga një burim i lejuar.
use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;

pub const TSV: &str = "raw/dialects/dialects-sq.tsv";

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn pos_code(p: &str) -> Option<&'static str> {
    match p {
        "noun" => Some("n"),
        "verb" => Some("v"),
        "adj" | "adjective" => Some("a"),
        "adv" | "adverb" => Some("r"),
        _ => None,
    }
}

fn gender_code(g: &str) -> Option<&'static str> {
    match g {
        "m" => Some("m"),
        "f" => Some("f"),
        "n" => Some("n"),
        "mf" | "m/f" => Some("mf"),
        _ => None,
    }
}

pub fn import(g: &mut Graph, _morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let (mut rows_n, mut edges, mut no_std, mut unknown_std) = (0usize, 0usize, 0usize, 0usize);
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        rows_n += 1;
        let c: Vec<&str> = l.split('\t').map(|s| s.trim()).collect();
        let col = |i: usize| c.get(i).copied().unwrap_or("");
        let (form, dialect, pos, gender, std) = (norm(col(0)), col(1).to_lowercase(), col(2).to_lowercase(), col(3).to_lowercase(), norm(col(4)));
        match (form.is_empty() || std.is_empty() || form == std, g.get(&word_key(Lang::Sq, &std))) {
            (true, _) => no_std += 1,
            (false, None) => unknown_std += 1,
            (false, Some(to)) => {
                let from = g.ensure_word(Lang::Sq, &form);
                let mut attrs = vec![("src".to_string(), "fjalorishqip:kristoforidhi-1904".to_string())];
                match dialect.is_empty() {
                    true => {}
                    false => attrs.push(("dialect".into(), dialect.clone())),
                }
                let before = g.edges.len();
                g.add_edge(Edge {
                    from,
                    to,
                    rel: Rel::Sem("DIALECT_VARIANT_OF".into()),
                    source: Source::Dialects,
                    lang: Some(Lang::Sq),
                    method: Method::Published,
                    attrs,
                });
                match g.edges.len() > before {
                    true => edges += 1,
                    false => {}
                }
                let mut lex: Vec<(String, String)> = Vec::new();
                match pos_code(&pos) {
                    Some(p) => lex.push(("pos".into(), p.into())),
                    None => {}
                }
                match gender_code(&gender) {
                    Some(gc) => lex.push(("gender".into(), gc.into())),
                    None => {}
                }
                match lex.is_empty() {
                    true => {}
                    false => {
                        lex.push(("flags".into(), format!("dialect:{dialect}")));
                        g.add_edge(Edge {
                            from,
                            to: from,
                            rel: Rel::Lex,
                            source: Source::Dialects,
                            lang: Some(Lang::Sq),
                            method: Method::Published,
                            attrs: lex,
                        });
                    }
                }
            }
        }
    }
    rep.note(format!(
        "dialects: {rows_n} rreshta; {edges} lidhje DIALECT_VARIANT_OF; {unknown_std} me lemë standarde që nuk ekziston në graf (të anashkaluara); {no_std} pa formë standarde / të njëjta"
    ));
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &str = "# dialects-sq.tsv — test\n\
aguem\tgheg\tnoun\tm\tagim\n\
agjnoj\tgheg\tverb\t\tagjëroj\n\
avlimend\ttosk\tnoun\tm\tavlëmend\n\
hutak\tgheg\tnoun\tm\t\n\
agim\tgheg\tnoun\tm\tagim\n\
gjysmë\n";

    #[test]
    fn links_only_to_known_standard_lemmas() {
        let mut g = Graph::new();
        let agim = g.ensure_word(Lang::Sq, "agim");
        g.ensure_word(Lang::Sq, "agjëroj");
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, FIX, &mut r), 2);
        let aguem = g.get(&word_key(Lang::Sq, "aguem")).unwrap();
        let e = g.out_edges(aguem).find(|(_, e)| e.rel == Rel::Sem("DIALECT_VARIANT_OF".into())).unwrap().1;
        assert_eq!(e.to, agim);
        assert_eq!(e.attr("dialect"), Some("gheg"));
        let lex = g.out_edges(aguem).find(|(_, e)| e.rel == Rel::Lex).unwrap().1;
        assert_eq!((lex.attr("pos"), lex.attr("gender")), (Some("n"), Some("m")));
        // Lema standarde e panjohur: as forma dialektore nuk krijohet.
        assert!(g.get(&word_key(Lang::Sq, "avlimend")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "avlëmend")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "hutak")).is_none());
    }

    #[test]
    fn empty_and_partial() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, "", &mut r), 0);
        assert_eq!(import(&mut g, &mut m, "x\n\t\t\t\t\n", &mut r), 0);
        assert!(g.nodes.is_empty());
    }
}
