//! Lidhjet sipas përkufizimit: fjala lidhet me fjalët e përmbajtjes që përmend shpjegimi i saj
//! shqip. Kështu dy fjalë lidhen sipas kuptimit që kanë në shpjegim: «bari» (njeri që ruan dhe
//! kullot bagëtinë) dhe «bar» (bima) lidhen përmes «kullotë» (tokë me bar ku ushqehen bagëtitë).
//!
//! Rregullat e deklaruara:
//! - Shpjegimet: përkufizimet shqipe të kuptimeve të fjalës (AlbaNet `als:def`, BabelNet, leksiku
//!   i modelit — secili me burimin e vet) dhe shpjegimet krahinore të Gazullit (1941).
//! - Fjalët e shpjegimit ndahen te shenjat e pikësimit dhe te hapësirat; fjalët funksionale
//!   (parafjalë, lidhëza, nyje, përemra, pjesëza, foljet ndihmëse) nuk merren, as kur janë
//!   shkruar pa «ë»/«ç» («qe» = «që», «te» = «të», «per» = «për», «nje» = «një»).
//! - Fjala e shkruar pa «ë»/«ç» provohet me shenjat e rikthyera vetëm te burimet që shkruajnë pa
//!   to (AlbaNet), vetëm kur forma siç është shkruar nuk është e dëshmuar, dhe vetëm kur një variant
//!   i vetëm jep lemë me dëshmi («mjekesine» → «mjekësi»; «politike» s'bëhet «politikë»).
//! - Përemrat, numërorët dhe trajtat e foljeve ndihmëse janë fjalë funksionale.
//! - Çdo fjalë kthehet në lemë vetëm me dëshmi, në këtë radhë: (1) vetë forma është lemë e
//!   leksikut me pjesë ligjërate përmbajtjeje (emër, folje, mbiemër) dhe nuk është trajtë e
//!   atestuar e një leme tjetër («bari» është edhe «bariu», edhe trajta e shquar e «bar» → HOLD);
//!   (2) përndryshe, një trajtë e atestuar (tabelat e Wiktionary-t ose UD) e një leme të vetme të
//!   tillë. Kur trajta i përket disa lemave → HOLD (nuk lidhet; HOLD nuk është mohim).
//! - Skaji: `Word(fjala) → Word(lema e përmendur)`, `Rel::Sem("DEF_MENTIONS")`, me burimin e
//!   shpjegimit, atributet `sense` (kuptimi), `i` (vendi në shpjegim), `genus=1` për emrin e llojit
//!   («njeri», «tokë», «bar»; rregullat te `genus`), dhe `df` = në sa shpjegime përmendet lema. Fjala nuk
//!   lidhet me vetveten; i njëjti çift nga i njëjti burim lidhet një herë.
//! - `df` i lejon bisedës të mos i përdorë fjalët shumë të përgjithshme («njeri», «gjë») si urë
//!   lidhjeje (rregulli i urës është te `biseda`).
//!
//! Stil: pa if/else — vetëm match.

use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;
use std::collections::{BTreeMap, BTreeSet};

pub const REL: &str = "DEF_MENTIONS";

/// Fjalët funksionale të shqipes (klasa të mbyllura): nuk janë fjalë përmbajtjeje.
const FUNCTION_WORDS: &[&str] = &[
    "a", "apo", "as", "asaj", "asnjë", "atë", "ato", "atyre", "ai", "ajo", "aq", "deri", "dhe", "duke", "e", "edhe", "ende",
    "gjatë", "i", "ia", "iu", "jam", "je", "jemi", "janë", "jo", "ka", "kam", "kanë", "kemi", "kë", "këtë", "këto", "kjo",
    "ky", "kur", "ku", "kush", "më", "me", "mbi", "mes", "midis", "mund", "nga", "ndaj", "ndër", "ndërsa", "nën", "në",
    "nëse", "një", "nuk", "o", "ose", "pa", "pas", "pak", "para", "pasi", "për", "përmes", "përpara", "pra", "prej", "që",
    "qoftë", "rreth", "s'", "sa", "se", "si", "sipas", "shumë", "ta", "tani", "te", "tek", "tjetër", "tjera", "tjerë",
    "tij", "saj", "tyre", "të", "u", "vetë", "veç", "ç'", "çdo", "është", "ishte", "kishte", "do", "le", "nëpër", "kundër",
    "drejt", "larg", "afër", "brenda", "jashtë", "sipër", "poshtë", "ndonjë", "disa", "gjithë", "gjitha", "cili", "cila",
    "cilat", "cilët", "atij", "kësaj", "këtij", "këtyre", "dikush", "diçka", "çka", "sikur", "megjithëse", "porse", "por",
    // përemrat «tjetër», «vetë» dhe «asgjë» në trajtat e tyre
    "tjetri", "tjetra", "tjetrin", "tjetrën", "tjetrit", "tjetrës", "tjerët", "tjerat", "tjerëve", "tjerave", "vete", "vetes",
    "vetja", "veten", "vet", "asgjë", "gjithçka", "secili", "secila",
    // numërorët e parë (edhe gegërisht «nji»)
    "nji", "dy", "tre", "tri", "katër", "pesë", "gjashtë", "shtatë", "tetë", "nëntë", "dhjetë",
    // trajtat e foljeve ndihmëse «jam» dhe «kam»
    "jesh", "jeni", "isha", "ishe", "ishim", "ishit", "ishin", "qenë", "qenit", "qenët", "qenkësh", "kisha", "kishe",
    "kishim", "kishin", "keni", "pati", "patën", "paska", "qenka",
];

/// Burimet që i shkruajnë shpjegimet pa «ë»/«ç»: vetëm te këto rikthehen shenjat.
const WITHOUT_DIACRITICS: &[Source] = &[Source::OmwAls];

/// Fjalët para të cilave nuk ka emër lloji: lidhëza, parafjalë, nyje, fjalë lidhore.
const NO_GENUS: &[&str] = &[
    "që", "qe", "të", "te", "në", "ne", "me", "nga", "për", "per", "pa", "sipas", "përtej", "nëpër", "mbi", "ndërmjet", "ku",
    "kur", "si", "i", "e", "së", "se", "duke", "kush", "cili", "cila",
];

/// Pjesët e ligjëratës së përmbajtjes sipas leksikut gramatikor.
const CONTENT_POS: &[&str] = &["n", "v", "adj"];

fn content_pos(g: &Graph, w: u32) -> Option<&'static str> {
    g.out_edges(w)
        .filter(|(_, e)| e.rel == Rel::Lex)
        .filter_map(|(_, e)| e.attr("pos"))
        .find_map(|p| CONTENT_POS.iter().copied().find(|c| *c == p))
}

/// Lema e një fjale të shpjegimit, me dëshmi; `None` = HOLD.
fn lemma_of(g: &Graph, m: &Morph, token: &str) -> Option<(u32, &'static str)> {
    let as_lemma = g.get(&word_key(Lang::Sq, token)).and_then(|w| content_pos(g, w).map(|p| (w, p)));
    let attested = || -> Option<(u32, &'static str)> {
        let from_wiktionary: BTreeSet<u32> = g
            .get(&word_key(Lang::Sq, token))
            .map(|w| {
                g.out_edges(w)
                    .filter(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()) && e.source != Source::ModelClaude && e.to != w)
                    .map(|(_, e)| e.to)
                    .collect()
            })
            .unwrap_or_default();
        let from_ud: BTreeSet<u32> = m.attested_lemmas(token).iter().filter_map(|l| g.get(&word_key(Lang::Sq, l))).collect();
        let all: Vec<(u32, &'static str)> =
            from_wiktionary.union(&from_ud).filter_map(|&w| content_pos(g, w).map(|p| (w, p))).collect::<BTreeSet<_>>().into_iter().collect();
        match all.as_slice() {
            [one] => Some(*one),
            _ => None,
        }
    };
    // Forma që është vetë lemë, por edhe trajtë e atestuar e një leme tjetër («bari»: bariu, ose
    // trajta e shquar e «bar»), është e paqartë → HOLD.
    let other_lemma = || -> bool {
        let w = match g.get(&word_key(Lang::Sq, token)) {
            Some(w) => w,
            None => return false,
        };
        g.out_edges(w)
            .any(|(_, e)| e.rel == Rel::Sem("FORM_OF".into()) && e.source != Source::ModelClaude && e.to != w && content_pos(g, e.to).is_some())
            || m.attested_lemmas(token).iter().any(|l| l != token && g.get(&word_key(Lang::Sq, l)).and_then(|x| content_pos(g, x)).is_some())
    };
    match (as_lemma, other_lemma()) {
        (Some(_), true) => None,
        (Some(x), false) => Some(x),
        (None, _) => attested(),
    }
}

/// Fjalë funksionale, edhe kur është shkruar pa «ë»/«ç».
fn function_word(t: &str) -> bool {
    FUNCTION_WORDS.contains(&t)
        || crate::gramatika::fonologjia::diakritikat::candidates(t)
            .map(|vs| vs.iter().any(|v| FUNCTION_WORDS.contains(&v.as_str())))
            .unwrap_or(false)
}

pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphabetic() || c == '-' || c == '\''))
        .map(|t| t.trim_matches(|c| c == '-' || c == '\'').to_lowercase())
        .filter(|t| t.chars().count() >= 2 && !function_word(t))
        .collect()
}

/// Forma është e dëshmuar siç është shkruar (lemë ose trajtë e atestuar e ndonjë leme): atëherë
/// nuk i mungojnë shenjat, edhe kur leximi i saj është i paqartë («politike»).
fn attested_as_written(g: &Graph, m: &Morph, token: &str) -> bool {
    g.get(&word_key(Lang::Sq, token))
        .map(|w| content_pos(g, w).is_some() || g.out_edges(w).any(|(_, e)| e.rel == Rel::Sem("FORM_OF".into())))
        .unwrap_or(false)
        || !m.attested_lemmas(token).is_empty()
}

/// Lema me dëshmi për fjalën siç është shkruar; te burimet pa «ë»/«ç», edhe për variantin e
/// vetëm me shenjat e rikthyera, kur forma siç është shkruar nuk është e dëshmuar.
fn lemma_restored(g: &Graph, m: &Morph, token: &str, src: Source) -> Option<(u32, &'static str)> {
    let may_restore = WITHOUT_DIACRITICS.contains(&src) && !attested_as_written(g, m, token);
    let restored = || -> Option<(u32, &'static str)> {
        match may_restore {
            true => {}
            false => return None,
        }
        let found: BTreeSet<(u32, &'static str)> = crate::gramatika::fonologjia::diakritikat::candidates(token)?
            .iter()
            .filter(|v| v.as_str() != token)
            .filter_map(|v| lemma_of(g, m, v))
            .collect();
        match found.len() {
            1 => found.into_iter().next(),
            _ => None,
        }
    };
    lemma_of(g, m, token).or_else(restored)
}

/// Emri i llojit në përkufizim, me rregulla të deklaruara: (a) hiqet etiketa e fushës në kllapa
/// në fillim; (b) në përkufizimin në formë fjalie («X është një Y») kërkohet pas «është/janë»;
/// (c) kapërcehet «një»; (d) fjala e parë është emri i llojit vetëm kur është emër i leksikut; kur
/// fillon me lidhëz, parafjalë, nyje, folje ose mbiemër, përkufizimi s'ka emër lloji.
fn genus(g: &Graph, m: &Morph, text: &str, src: Source) -> Option<String> {
    let t = text.trim();
    let t = match t.starts_with('(') {
        true => t.split_once(')').map(|(_, r)| r).unwrap_or(t),
        false => t,
    };
    let words: Vec<String> = t
        .split(|c: char| !(c.is_alphabetic() || c == '-' || c == '\''))
        .map(|w| w.trim_matches(|c| c == '-' || c == '\'').to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();
    let start = words.iter().position(|w| w == "është" || w == "janë" || w == "eshte" || w == "jane").map(|p| p + 1).unwrap_or(0);
    let first = words.iter().skip(start).find(|w| !matches!(w.as_str(), "një" | "nji" | "nje"))?;
    match NO_GENUS.contains(&first.as_str()) {
        true => None,
        false => match lemma_restored(g, m, first, src) {
            Some((_, "n")) => Some(first.clone()),
            _ => None,
        },
    }
}

/// Shpjegimet shqipe të një fjale: (burimi, kuptimi, teksti).
fn glosses(g: &Graph, w: u32) -> Vec<(Source, String, String)> {
    let mut out: Vec<(Source, String, String)> = Vec::new();
    for (s, _) in g.senses_of(w) {
        let key = match &g.node(s).kind {
            NodeKind::Sense { synset, source, .. } => format!("{}:{synset}", source.code()),
            _ => continue,
        };
        g.node(s).glosses.iter().filter(|t| t.lang == Lang::Sq).for_each(|t| out.push((t.source, key.clone(), t.text.clone())));
    }
    g.out_edges(w)
        .filter(|(_, e)| e.rel == Rel::Sem(crate::src_gazulli::REL.into()) && e.source == Source::Gazulli1941)
        .filter_map(|(_, e)| e.attr("gloss").map(|t| (Source::Gazulli1941, "gazulli1941".to_string(), t.to_string())))
        .for_each(|x| out.push(x));
    out.sort();
    out.dedup();
    out
}

pub fn import(g: &mut Graph, m: &Morph, rep: &mut ImportReport) {
    let words: Vec<u32> = (0..g.nodes.len() as u32)
        .filter(|&i| matches!(&g.node(i).kind, NodeKind::Word { lang: Lang::Sq, .. }))
        .collect();
    // (fjala, lema e përmendur, burimi) → (kuptimi, vendi, lloji)
    let mut links: BTreeMap<(u32, u32, Source), (String, usize, bool)> = BTreeMap::new();
    let mut held = 0usize;
    let mut glossed = 0usize;
    for &w in &words {
        let gl = glosses(g, w);
        glossed += usize::from(!gl.is_empty());
        for (src, sense, text) in gl {
            let kind = genus(g, m, &text, src);
            let mut genus_seen = false;
            for (i, t) in tokens(&text).iter().enumerate() {
                match lemma_restored(g, m, t, src) {
                    Some((l, _)) => {
                        let genus = !genus_seen && kind.as_deref() == Some(t.as_str());
                        genus_seen = genus_seen || genus;
                        match l == w {
                            true => {}
                            false => {
                                links.entry((w, l, src)).or_insert((sense.clone(), i, genus));
                            }
                        }
                    }
                    None => held += 1,
                }
            }
        }
    }
    let mut df: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    links.keys().for_each(|(w, l, _)| {
        df.entry(*l).or_default().insert(*w);
    });
    let n = links.len();
    for ((w, l, src), (sense, i, genus)) in links {
        let mut attrs = vec![("sense".to_string(), sense), ("i".to_string(), i.to_string())];
        match genus {
            true => attrs.push(("genus".into(), "1".into())),
            false => {}
        }
        attrs.push(("df".into(), df.get(&l).map(|s| s.len()).unwrap_or(0).to_string()));
        g.add_edge(Edge { from: w, to: l, rel: Rel::Sem(REL.into()), source: src, lang: Some(Lang::Sq), method: Method::GlossLemma, attrs });
    }
    rep.note(format!(
        "përkufizimet: {glossed} fjalë me shpjegim shqip; {n} lidhje DEF_MENTIONS drejt {} lemave; {held} fjalë shpjegimi në HOLD (pa lemë të vetme të dëshmuar)",
        df.len()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(g: &mut Graph, lemma: &str, pos: &str) -> u32 {
        let w = g.ensure_word(Lang::Sq, lemma);
        g.add_edge(Edge {
            from: w,
            to: w,
            rel: Rel::Lex,
            source: Source::Uniparser,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![("pos".into(), pos.into())],
        });
        w
    }

    #[test]
    fn a_definition_links_the_word_to_the_content_words_it_mentions() {
        let mut g = Graph::new();
        let kullote = lex(&mut g, "kullotë", "n");
        let toke = lex(&mut g, "tokë", "n");
        let bar = lex(&mut g, "bar", "n");
        let bageti = lex(&mut g, "bagëti", "n");
        let ushqej = lex(&mut g, "ushqej", "v");
        let s = g.ensure(sense_key(Source::ModelClaude, "model:kullotë:n"), NodeKind::Sense { source: Source::ModelClaude, synset: "model:kullotë:n".into(), pos: Pos::Noun });
        g.nodes[s as usize].glosses.push(Text { lang: Lang::Sq, text: "tokë me bar ku ushqehen bagëtitë".into(), source: Source::ModelClaude });
        g.add_edge(Edge { from: kullote, to: s, rel: Rel::HasSense, source: Source::ModelClaude, lang: Some(Lang::Sq), method: Method::ModelAuthored, attrs: vec![] });
        // «bagëtitë» është trajtë e atestuar e «bagëti»; «ushqehen» s'ka dëshmi → HOLD.
        let form = g.ensure_word(Lang::Sq, "bagëtitë");
        g.add_edge(Edge { from: form, to: bageti, rel: Rel::Sem("FORM_OF".into()), source: Source::KaikkiEn, lang: Some(Lang::Sq), method: Method::Published, attrs: vec![] });
        let mut rep = ImportReport::default();
        import(&mut g, &Morph::default(), &mut rep);
        let to: Vec<(u32, Option<String>)> = g
            .out_edges(kullote)
            .filter(|(_, e)| e.rel == Rel::Sem(REL.into()))
            .map(|(_, e)| (e.to, e.attr("genus").map(str::to_string)))
            .collect();
        assert!(to.iter().any(|(l, g)| *l == toke && g.as_deref() == Some("1")), "lloji: «tokë»");
        assert!(to.iter().any(|(l, _)| *l == bar));
        assert!(to.iter().any(|(l, _)| *l == bageti));
        assert!(!to.iter().any(|(l, _)| *l == ushqej));
        assert_eq!(to.len(), 3);
    }
}
