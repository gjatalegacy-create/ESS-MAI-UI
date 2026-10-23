//! Shënimet e pronarit të projektit: shpjegime popullore të fjalëve që pronari i jep vetë.
//!
//! Nuk janë prejardhje e vërtetuar nga burimet; Nura i thotë gjithmonë si të tilla («Sipas një
//! shpjegimi popullor, …»), pas prejardhjes së dokumentuar, dhe kurrë si fakt.
//!
//! Formati (TSV, `#` = koment): `fjala ⇥ lloji ⇥ teksti`. Lloji i vetëm i njohur:
//! `folk_etymology` — teksti vazhdon fjalinë «Sipas një shpjegimi popullor, «fjala» …».
use crate::graph::*;
use crate::import::ImportReport;

/// Shtegu i skedarit, nga dosja `ui_ess_mai`.
pub const PATH: &str = "model/sq_shenime_pronari.tsv";
pub const FOLK: &str = "FOLK_ETYMOLOGY";

pub fn import(g: &mut Graph, text: &str, rep: &mut ImportReport) -> usize {
    let mut n = 0usize;
    let mut skipped = 0usize;
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty() && !l.starts_with("lemma\t")) {
        let c: Vec<&str> = l.split('\t').map(|x| x.trim()).collect();
        let (lemma, kind, body) = (c.first().copied().unwrap_or(""), c.get(1).copied().unwrap_or(""), c.get(2).copied().unwrap_or(""));
        match (kind, g.get(&word_key(Lang::Sq, &lemma.to_lowercase())), body.is_empty()) {
            ("folk_etymology", Some(w), false) => {
                g.add_edge(Edge {
                    from: w,
                    to: w,
                    rel: Rel::Sem(FOLK.into()),
                    source: Source::OwnerNote,
                    lang: Some(Lang::Sq),
                    method: Method::OwnerStated,
                    attrs: vec![("text".into(), body.to_string()), ("status".into(), "e pavërtetuar".into())],
                });
                n += 1;
            }
            _ => skipped += 1,
        }
    }
    rep.note(format!("owner_note: {n} shpjegime popullore të shënuara si të tilla; {skipped} rreshta të palexueshëm"));
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folk_explanations_attach_only_to_known_words() {
        let mut g = Graph::default();
        g.ensure_word(Lang::Sq, "pishinë");
        let mut rep = ImportReport::default();
        let n = import(&mut g, "pishinë\tfolk_etymology\tlidhet me «pi shina»\nfjalëepanjohur\tfolk_etymology\tx\n", &mut rep);
        assert_eq!(n, 1);
        let w = g.get(&word_key(Lang::Sq, "pishinë")).unwrap();
        assert!(g.out_edges(w).any(|(_, e)| e.rel == Rel::Sem(FOLK.into()) && e.source == Source::OwnerNote));
        assert!(g.get(&word_key(Lang::Sq, "fjalëepanjohur")).is_none());
    }
}
