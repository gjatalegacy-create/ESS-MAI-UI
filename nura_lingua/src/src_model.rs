//! Leksiku i shkruar nga modeli gjuhësor i projektit, 2026-10-22, me kërkesë të pronarit
//! («edhe ti je librari … provo ta trajnosh ti, duke e rregulluar në Rust, në sistem direkt»).
//!
//! Si u bë: për 2,598 fjalë të zakonshme që grafi i njihte dobët (pa përkufizim shqip ose pa
//! sinonim), një kalim i modelit shkroi hyrjet dhe një kalim i dytë kundërshtues i kontrolloi,
//! duke hequr çdo fushë të dyshimtë. Përkufizimet janë të shkruara nga modeli me fjalët e veta,
//! jo të kopjuara nga ndonjë fjalor. Në kohën e përdorimit nuk thirret asnjë model: Nura lexon
//! vetëm këtë skedar të fiksuar (SHA-256 në raportin e importit), në mënyrë deterministe.
//!
//! Formati (TSV, `#` = koment): `fjala ⇥ pos ⇥ gjinia ⇥ përkufizimi ⇥ anglisht ⇥ kategoria ⇥
//! sinonimet (|) ⇥ të kundërtat (|) ⇥ baza`.
//!
//! Rregullat e dëshmisë (modeli është NJË dëshmitar, jo autoritet):
//! - Hyrja merret vetëm për një fjalë që e njeh tashmë një burim tjetër; lidhjet (sinonim, e
//!   kundërt, kategori, bazë) vetëm drejt fjalëve që i njeh një burim tjetër.
//! - Përkufizimi: për fjalën pa asnjë kuptim shqip krijohet një kuptim i modelit; për fjalën që
//!   ka kuptime shqip pa përkufizim, përkufizimi i shtohet kuptimit me të njëjtën pjesë ligjërate
//!   dhe të njëjtin përkthim anglisht; ndryshe nuk merret (s'krijohet paqartësi e re).
//! - Lidhjet e modelit thuhen vetëm kur i dëshmon edhe një burim tjetër, ose kur i verifikon një
//!   burim i pavarur (p.sh. kategoria ndër paraardhësit e WordNet-it); shih `discourse::chosen`.
use crate::graph::*;
use crate::import::ImportReport;
use std::collections::BTreeMap;

/// Shtegu i skedarit, nga dosja `ui_ess_mai`.
pub const PATH: &str = "model/sq_model_lexicon.tsv";

/// Fjala në graf: si është, ose pa nyjën e mbiemrit («i madh» → «madh»).
fn existing(g: &Graph, lemma: &str) -> Option<u32> {
    let l = lemma.trim().to_lowercase();
    let bare = ["i ", "e ", "të "].iter().find_map(|a| l.strip_prefix(a)).unwrap_or(&l).to_string();
    g.get(&word_key(Lang::Sq, &l)).or_else(|| g.get(&word_key(Lang::Sq, &bare)))
}

fn edge(from: u32, to: u32, rel: Rel, attrs: Vec<(String, String)>) -> Edge {
    Edge { from, to, rel, source: Source::ModelClaude, lang: Some(Lang::Sq), method: Method::ModelAuthored, attrs }
}

/// Etiketat angleze të kuptimit, të normalizuara («swimming_pool» → «swimming pool»).
fn english_labels(g: &Graph, sid: u32) -> Vec<String> {
    g.node(sid).labels.iter().filter(|t| t.lang == Lang::En).map(|t| t.text.replace('_', " ").to_lowercase()).collect()
}

pub fn import(g: &mut Graph, text: &str, rep: &mut ImportReport) -> usize {
    let mut c: BTreeMap<&'static str, usize> = BTreeMap::new();
    let bump = |c: &mut BTreeMap<&'static str, usize>, k: &'static str| *c.entry(k).or_default() += 1;
    let mut rows = 0usize;
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty() && !l.starts_with("lemma\t")) {
        rows += 1;
        let col: Vec<&str> = l.split('\t').collect();
        let get = |i: usize| col.get(i).map(|s| s.trim()).unwrap_or("");
        let (lemma, pos_s, definition, english) = (get(0), get(1), get(3), get(4).to_lowercase());
        let pos = Pos::from_wn(pos_s);
        let w = match existing(g, lemma) {
            Some(w) => w,
            None => {
                bump(&mut c, "fjalë që s'e njeh asnjë burim tjetër (u la jashtë)");
                continue;
            }
        };
        let pos_attr = vec![("src".to_string(), "model_claude".to_string()), ("pos".to_string(), pos_s.to_string())];
        // ── Përkufizimi ──
        let albanian: Vec<u32> = g
            .senses_of(w)
            .iter()
            .filter(|(_, e)| !matches!(e.source, Source::KaikkiEn | Source::ModelClaude))
            .map(|(s, _)| *s)
            .collect();
        let en_items: Vec<String> = english.split([',', ';']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
        // Kuptimi shqip me të njëjtën pjesë ligjërate dhe të njëjtin përkthim, pa përkufizim shqip.
        let mut matching: Vec<(String, u32)> = albanian
            .iter()
            .filter(|&&s| {
                let same_pos = match (&g.node(s).kind, pos) {
                    (NodeKind::Sense { pos: p, .. }, Some(q)) => *p == q,
                    _ => false,
                };
                let same_en = english_labels(g, s).iter().any(|e| en_items.contains(e));
                same_pos && same_en && g.gloss_in(s, Lang::Sq).is_none()
            })
            .map(|&s| (g.node(s).key.clone(), s))
            .collect();
        matching.sort();
        match (definition.is_empty(), albanian.is_empty(), matching.first(), pos) {
            (true, _, _, _) => {}
            (false, true, _, Some(p)) => {
                let synset = format!("model:{}:{}", lemma, pos_s);
                let sid = g.ensure(sense_key(Source::ModelClaude, &synset), NodeKind::Sense { source: Source::ModelClaude, synset, pos: p });
                g.add_label(sid, Text { lang: Lang::Sq, text: lemma.to_string(), source: Source::ModelClaude });
                match english.is_empty() {
                    true => {}
                    false => g.add_label(sid, Text { lang: Lang::En, text: english.clone(), source: Source::ModelClaude }),
                }
                g.nodes[sid as usize].glosses.push(Text { lang: Lang::Sq, text: definition.to_string(), source: Source::ModelClaude });
                g.add_edge(edge(w, sid, Rel::HasSense, pos_attr.clone()));
                bump(&mut c, "kuptime të reja (fjalë pa kuptim shqip)");
            }
            (false, true, _, None) => bump(&mut c, "përkufizim pa pjesë ligjërate (u la jashtë)"),
            (false, false, Some((_, s)), _) => {
                let s = *s;
                g.nodes[s as usize].glosses.push(Text { lang: Lang::Sq, text: definition.to_string(), source: Source::ModelClaude });
                bump(&mut c, "përkufizime të shtuara te kuptimi ekzistues");
            }
            (false, false, None, _) => bump(&mut c, "përkufizim pa kuptim përkatës (u la jashtë)"),
        }
        // ── Lidhjet në nivel fjale, vetëm drejt fjalëve që i njeh një burim tjetër ──
        let links: [(&str, usize, bool); 4] = [("HYPERNYM", 5, false), ("SYNONYM", 6, true), ("ANTONYM", 7, true), ("DERIVED_FROM", 8, false)];
        for (rel, i, many) in links {
            let items: Vec<&str> = match many {
                true => get(i).split('|').map(|x| x.trim()).filter(|x| !x.is_empty()).collect(),
                false => vec![get(i)].into_iter().filter(|x| !x.is_empty()).collect(),
            };
            for t in items {
                match existing(g, t).filter(|&o| o != w) {
                    Some(o) => {
                        let before = g.edges.len();
                        g.add_edge(edge(w, o, Rel::Sem(rel.into()), pos_attr.clone()));
                        match g.edges.len() > before {
                            true => bump(&mut c, "lidhje të shtuara"),
                            false => {}
                        }
                    }
                    None => bump(&mut c, "lidhje drejt fjalës së panjohur (u la jashtë)"),
                }
            }
        }
    }
    rep.note(format!("model_claude: {rows} hyrje; {c:?}"));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> Graph {
        let mut g = Graph::default();
        for w in ["qen", "kafshë", "zagar", "diell", "yll", "madh", "vogël", "zemër"] {
            g.ensure_word(Lang::Sq, w);
        }
        // «zemër» ka një kuptim AlbaNet pa përkufizim shqip, me etiketën angleze «heart».
        let z = g.get(&word_key(Lang::Sq, "zemër")).unwrap();
        let s = g.ensure(sense_key(Source::OmwAls, "05395690-n"), NodeKind::Sense { source: Source::OmwAls, synset: "05395690-n".into(), pos: Pos::Noun });
        g.add_label(s, Text { lang: Lang::En, text: "heart".into(), source: Source::Pwn30 });
        g.add_edge(Edge { from: z, to: s, rel: Rel::HasSense, source: Source::OmwAls, lang: Some(Lang::Sq), method: Method::OmwOffsetMap, attrs: vec![] });
        g
    }

    #[test]
    fn model_entries_link_only_known_words_and_never_compete_with_albanian_senses() {
        let mut g = graph();
        let mut rep = ImportReport::default();
        let tsv = "lemma\tpos\tgender\tdefinition\tenglish\tcategory\tsynonyms\tantonyms\tderived_from\n\
                   qen\tn\tm\tkafshë shtëpiake që ruan shtëpinë\tdog\tkafshë\tzagar|hundëgjatë\t\t\n\
                   madh\ta\t\tqë ka përmasa mbi mesataren\tbig\t\t\tvogël\t\n\
                   zemër\tn\tf\torgan që shtyn gjakun në trup\theart\t\t\t\t\n\
                   fjalëqëskaa\tn\tf\tdiçka\tsomething\t\t\t\t\n";
        import(&mut g, tsv, &mut rep);
        let qen = g.get(&word_key(Lang::Sq, "qen")).unwrap();
        // Kuptim i ri për fjalën pa kuptim shqip.
        let s = g.senses_of(qen);
        assert_eq!(s.len(), 1);
        assert_eq!(g.gloss_in(s[0].0, Lang::Sq).map(|t| t.text.as_str()), Some("kafshë shtëpiake që ruan shtëpinë"));
        // Lidhje vetëm drejt fjalëve të njohura: «zagar» po, «hundëgjatë» jo.
        let syn: Vec<u32> = g.out_edges(qen).filter(|(_, e)| e.rel == Rel::Sem("SYNONYM".into())).map(|(_, e)| e.to).collect();
        assert_eq!(syn, vec![g.get(&word_key(Lang::Sq, "zagar")).unwrap()]);
        assert!(g.out_edges(qen).any(|(_, e)| e.rel == Rel::Sem("HYPERNYM".into()) && e.source == Source::ModelClaude));
        // Përkufizimi i shtohet kuptimit AlbaNet me të njëjtin përkthim; s'krijohet kuptim i ri.
        let z = g.get(&word_key(Lang::Sq, "zemër")).unwrap();
        assert_eq!(g.senses_of(z).len(), 1);
        assert_eq!(g.gloss_in(g.senses_of(z)[0].0, Lang::Sq).map(|t| t.text.as_str()), Some("organ që shtyn gjakun në trup"));
        // Fjala që s'e njeh asnjë burim tjetër nuk hyn në graf.
        assert!(g.get(&word_key(Lang::Sq, "fjalëqëskaa")).is_none());
    }
}
