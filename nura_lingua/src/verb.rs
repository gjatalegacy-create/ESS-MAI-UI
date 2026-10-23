//! Veta e tretë njëjës e së tashmes dëftore — për fjalitë që ndërton Nura.
//!
//! Rendi i dëshmisë, i fiksuar:
//! 1. trajtë e atestuar në UD (Person=3, Number=Sing, Tense=Pres, Mood=Ind, jo pësore);
//! 2. folje e parregullt e deklaruar (tabela `IRREGULAR`);
//! 3. rregull sipas klasës së paradigmës së gramatikës (uniparser: atributet `par`, `stems`):
//!    klasat me lemë në «-j» (lexoj, laj, kthej, brej, ndiej, shkruaj, arrij) marrin temën e parë + «n»;
//!    klasat me ndërrim zanoreje (flas→flet, shoh→sheh, dal→del, marr→merr, pres→pret) marrin
//!    temën e dytë; klasat me bashkëtingëllore (hap, mat, heq, vjel, zhvesh) mbajnë temën e parë;
//!    vetvetoret «-em» marrin «-et»;
//! 4. rregull sipas trajtës së lemës, kur gramatika s'e ka paradigmën.
//! Kur asnjë nuk vlen, kthehet None: folja nuk lakohet me hamendje.

use crate::graph::*;
use crate::morph::{feat, Morph};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verb3 {
    pub text: String,
    /// Prejardhja e trajtës, për rreshtin e formave.
    pub origin: String,
}

const IRREGULAR: &[(&str, &str)] = &[
    ("jam", "është"),
    ("kam", "ka"),
    ("them", "thotë"),
    ("dua", "do"),
    ("vij", "vjen"),
    ("jap", "jep"),
    ("bie", "bie"),
    ("di", "di"),
    ("lë", "lë"),
    ("vë", "vë"),
    ("zë", "zë"),
    ("rri", "rri"),
    ("iki", "ikën"),
    ("eci", "ecën"),
    ("hyj", "hyn"),
    ("ha", "ha"),
    ("pi", "pi"),
    ("fle", "fle"),
    ("shoh", "sheh"),
    ("flas", "flet"),
    ("dal", "del"),
    ("marr", "merr"),
    ("vdes", "vdes"),
    ("vras", "vret"),
    ("pres", "pret"),
    ("shes", "shet"),
    ("ngas", "nget"),
    ("njoh", "njeh"),
    ("bëj", "bën"),
    ("mbaj", "mban"),
    ("shkoj", "shkon"),
    ("sjell", "sjell"),
    ("ndjek", "ndjek"),
    ("hedh", "hedh"),
    ("djeg", "djeg"),
    ("bredh", "bredh"),
    ("mbyll", "mbyll"),
    ("hap", "hap"),
    ("heq", "heq"),
    ("vesh", "vesh"),
    ("rris", "rrit"),
    ("pyes", "pyet"),
    ("mbes", "mbetet"),
    ("fryj", "fryn"),
];

enum Rule {
    Stem0N,
    Stem1,
    Stem0,
}

fn class_rule(par: &str) -> Option<Rule> {
    match par.trim_end_matches("-1") {
        "lexoj" | "laj" | "kthej" | "brej" | "ndiej" | "shkruaj" | "shkruej" | "arrij" => Some(Rule::Stem0N),
        "pres-wait" | "flas" | "shoh" | "dal" | "marr" | "jap" | "vras" | "pres" | "shes" | "vdes" | "ngas" => Some(Rule::Stem1),
        "hap" | "mat" | "beh" | "heq" | "vjel" | "zhvesh" | "ec" | "ik" => Some(Rule::Stem0),
        _ => None,
    }
}

/// Temat e gramatikës: «flas.|flet.|flit.//flis.|fol.» → [flas, flet, flit, fol].
fn stems(raw: &str) -> Vec<String> {
    raw.split('|')
        .map(|s| s.split("//").next().unwrap_or("").trim_matches('.').to_string())
        .filter(|s| !s.is_empty() && s != "NOSTEM")
        .collect()
}

fn attested(morph: &Morph, lemma: &str) -> Option<String> {
    morph
        .entries_of(lemma)
        .into_iter()
        .find(|e| {
            e.upos == "VERB"
                && feat(&e.feats, "Person") == Some("3")
                && feat(&e.feats, "Number") == Some("Sing")
                && feat(&e.feats, "Tense") == Some("Pres")
                && feat(&e.feats, "Mood") == Some("Ind")
                && feat(&e.feats, "Voice") != Some("Pass")
        })
        .map(|e| e.form.to_lowercase())
}

/// Paradigma dhe temat e foljes sipas gramatikës (uniparser), kur ekzistojnë.
fn paradigm(g: &Graph, lemma: &str) -> Option<(String, Vec<String>)> {
    let w = g.get(&word_key(Lang::Sq, lemma))?;
    g.out_edges(w)
        .filter(|(_, e)| e.rel == Rel::Lex && e.source == Source::Uniparser && e.attr("pos") == Some("v"))
        .find_map(|(_, e)| match (e.attr("par"), e.attr("stems")) {
            (Some(p), Some(s)) => Some((p.to_string(), stems(s))),
            _ => None,
        })
}

/// Vetvetorja «-em» → «-et» (bëhem → bëhet, gjendem → gjendet).
fn mediopassive(lemma: &str) -> Option<String> {
    lemma.strip_suffix("em").map(|stem| format!("{stem}et"))
}

/// Rregulli sipas klasës së paradigmës.
fn by_paradigm(g: &Graph, lemma: &str) -> Option<Verb3> {
    let (par, st) = paradigm(g, lemma)?;
    let first = st.first().cloned()?;
    match (mediopassive(lemma), class_rule(&par)) {
        (Some(m), _) => Some(Verb3 { text: m, origin: format!("rregulli R-V4: vetvetore «-em» → «-et» (klasa {par})") }),
        (None, Some(Rule::Stem0N)) => Some(Verb3 { text: format!("{first}n"), origin: format!("rregulli R-V1: tema + «n» (klasa {par})") }),
        (None, Some(Rule::Stem1)) => st
            .get(1)
            .map(|s| Verb3 { text: s.clone(), origin: format!("rregulli R-V2: tema e dytë (klasa {par})") }),
        // Lema me «-s» në klasën e bashkëtingëllores merr temën e dytë me «-t» (pyes → pyet).
        (None, Some(Rule::Stem0)) => match (lemma.ends_with('s'), st.get(1).filter(|s| s.ends_with('t'))) {
            (true, Some(s)) => Some(Verb3 { text: s.clone(), origin: format!("rregulli R-V2: tema e dytë (klasa {par})") }),
            _ => Some(Verb3 { text: first, origin: format!("rregulli R-V3: tema e parë (klasa {par})") }),
        },
        (None, None) => None,
    }
}

/// Rregulli sipas trajtës së lemës, pa gramatikë.
fn by_shape(lemma: &str) -> Option<Verb3> {
    let j = lemma.strip_suffix('j').filter(|s| s.ends_with(['o', 'e', 'a', 'i', 'y', 'u']));
    match (mediopassive(lemma), j) {
        (Some(m), _) => Some(Verb3 { text: m, origin: "rregulli R-V4: vetvetore «-em» → «-et»".into() }),
        (None, Some(stem)) => Some(Verb3 { text: format!("{stem}n"), origin: "rregulli R-V1: tema + «n» (sipas lemës)".into() }),
        (None, None) => None,
    }
}

/// Veta e tretë vetëm me rregulla (pa trajtat e atestuara): për matjen e rregullave.
pub fn rule_3sg(g: &Graph, lemma: &str) -> Option<Verb3> {
    let irregular = IRREGULAR.iter().find(|(l, _)| *l == lemma).map(|(_, f)| Verb3 { text: f.to_string(), origin: "folje e parregullt (tabela)".into() });
    irregular.or_else(|| by_paradigm(g, lemma)).or_else(|| by_shape(lemma))
}

/// Veta e tretë njëjës e së tashmes, me dëshminë e saj.
pub fn present_3sg(g: &Graph, morph: &Morph, lemma: &str) -> Option<Verb3> {
    attested(morph, lemma).map(|text| Verb3 { text, origin: "trajtë e atestuar (UD)".into() }).or_else(|| rule_3sg(g, lemma))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_verb(lemma: &str, par: &str, st: &str) -> Graph {
        let mut g = Graph::default();
        let w = g.ensure_word(Lang::Sq, lemma);
        g.add_edge(Edge {
            from: w,
            to: w,
            rel: Rel::Lex,
            source: Source::Uniparser,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![("pos".into(), "v".into()), ("par".into(), par.into()), ("stems".into(), st.into())],
        });
        g
    }

    #[test]
    fn paradigm_classes_give_the_third_person() {
        assert_eq!(rule_3sg(&with_verb("punoj", "lexoj", "puno.|punua.|punu.|pun."), "punoj").unwrap().text, "punon");
        assert_eq!(rule_3sg(&with_verb("gërgas", "flas", "gërgas.|gërget.|gërgit.//gërgis.|gërga."), "gërgas").unwrap().text, "gërget");
        assert_eq!(rule_3sg(&with_verb("ajros", "hap", "ajros."), "ajros").unwrap().text, "ajros");
        assert_eq!(rule_3sg(&with_verb("aftosem", "hap-1", "aftos."), "aftosem").unwrap().text, "aftoset");
        assert_eq!(rule_3sg(&with_verb("bluaj", "shkruaj", "blua.|blu.|blo."), "bluaj").unwrap().text, "bluan");
        assert_eq!(rule_3sg(&with_verb("bares", "pres-wait", "bares.|baret.|barit."), "bares").unwrap().text, "baret");
    }

    #[test]
    fn irregular_and_shape_rules() {
        let g = Graph::default();
        assert_eq!(rule_3sg(&g, "jam").unwrap().text, "është");
        assert_eq!(rule_3sg(&g, "dua").unwrap().text, "do");
        assert_eq!(rule_3sg(&g, "mësoj").unwrap().text, "mëson");
        assert_eq!(rule_3sg(&g, "bëhem").unwrap().text, "bëhet");
        assert!(rule_3sg(&g, "sodis").is_none(), "pa gramatikë dhe pa rregull të sigurt: asgjë me hamendje");
    }
}
