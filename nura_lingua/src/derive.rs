//! Rregullat semantike të fjalëformimit të shqipes — motori deterministik.
//!
//! Rregullat NUK janë të shkruara në kod: lexohen nga tabela
//! `ui_ess_mai/rules/sq_fjaleformimi.tsv` (çdo rregull me burim gramatikor).
//! Motori i zbaton mbi fjalorin: një lidhje krijohet VETËM kur
//!   1) fjala e prejardhur dhe fjala bazë ekzistojnë të dyja në fjalor,
//!   2) pjesët e ligjëratës përputhen me rregullin,
//!   3) asnjë kusht përjashtimi nuk vlen,
//!   4) asnjë rregull më lart në tabelë nuk e ka pranuar tashmë të njëjtin çift.
//! Lidhja mban burimin `sq_rules`, metodën `albanian_rule` dhe id-në e rregullit.
//! Saktësia matet kundrejt çifteve /r/DerivedFrom shqip të ConceptNet (Wiktionary).
//!
//! Formati i tabelës (TSV, `#` = koment):
//!   id ⇥ lloji (prefix|suffix) ⇥ ndryshimet ⇥ pos_bazë ⇥ pos_prejardhur ⇥ marrëdhënia ⇥
//!   min_gjatësia_e_bazës ⇥ përjashtime ⇥ burimet [⇥ lista_e_bardhë ⇥ konfirmim ⇥ shablloni_sq]
//! - `lista_e_bardhë`: nëse s'është bosh, rregulli vlen VETËM për këto fjalë të prejardhura;
//! - `konfirmim`: `yes` = lidhja pranohet vetëm kur e njëjta çift (prejardhur, bazë) del
//!   edhe në një burim tjetër (Wiktionary DERIVED_FROM, ConceptNet DerivedFrom, LEXIN);
//! - `shablloni_sq`: si thuhet lidhja shqip, me {d} = fjala e prejardhur, {b} = baza (të cituara,
//!   mbiemri me nyje) dhe {db}, {bb} = të njëjtat pa nyje; i zgjeron shtresa e të folurit.
//!
//! Rendi i tabelës ka rëndësi: për çdo çift (prejardhur, bazë) krijohet VETËM NJË lidhje, nga
//! rregulli i parë që e PRANON çiftin (pjesa e ligjëratës dhe konfirmimi). Rregulli që dështon
//! nuk e zë çiftin, kështu që një rregull i mëvonshëm i vlefshëm mund ta lidhë; dhe një çift
//! nuk merr kurrë dy analiza që kundërshtojnë njëra-tjetrën (p.sh. NEGATION_OF dhe LACKING).
//! `ndryshimet`: lista "prejardhur>bazë" e ndarë me ";" — për prapashtesat zbatohen në
//! fund të fjalës (p.sh. "ues>oj" : mësues → mësoj), për parashtesat në fillim
//! (p.sh. "zb>mb" : zbuloj → mbuloj; "pa>" : pamundur → mundur).

use crate::graph::*;
use crate::import::ImportReport;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Prefix,
    Suffix,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub kind: Kind,
    pub alts: Vec<(String, String)>,
    pub base_pos: Vec<Pos>,
    pub result_pos: Vec<Pos>,
    pub relation: String,
    pub min_base: usize,
    pub exclude: HashSet<String>,
    pub sources: String,
    pub whitelist: HashSet<String>,
    pub confirm: bool,
    pub gloss_sq: String,
}

fn parse_pos_list(s: &str) -> Result<Vec<Pos>, String> {
    s.split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| match p {
            "n" => Ok(Pos::Noun),
            "v" => Ok(Pos::Verb),
            "a" => Ok(Pos::Adj),
            "r" => Ok(Pos::Adv),
            other => Err(format!("pos i panjohur «{other}»")),
        })
        .collect()
}

pub fn parse_rules(text: &str) -> Result<Vec<Rule>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let l = line.trim_end_matches('\r');
        match l.starts_with('#') || l.trim().is_empty() {
            true => continue,
            false => {}
        }
        let c: Vec<&str> = l.split('\t').collect();
        match c.len() >= 9 {
            true => {}
            false => return Err(format!("rregulli në rreshtin {} ka {} kolona (duhen 9)", i + 1, c.len())),
        }
        let kind = match c[1] {
            "prefix" => Kind::Prefix,
            "suffix" => Kind::Suffix,
            other => return Err(format!("rreshti {}: lloj i panjohur «{other}»", i + 1)),
        };
        let alts: Vec<(String, String)> = c[2]
            .split(';')
            .filter(|a| !a.trim().is_empty())
            .filter_map(|a| a.split_once('>').map(|(d, b)| (d.trim().to_string(), b.trim().to_string())))
            .collect();
        match alts.is_empty() {
            true => return Err(format!("rreshti {}: pa ndryshime prejardhur>bazë", i + 1)),
            false => {}
        }
        out.push(Rule {
            id: c[0].trim().to_string(),
            kind,
            alts,
            base_pos: parse_pos_list(c[3])?,
            result_pos: parse_pos_list(c[4])?,
            relation: c[5].trim().to_string(),
            min_base: c[6].trim().parse().map_err(|_| format!("rreshti {}: min_gjatësia", i + 1))?,
            exclude: c[7].split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect(),
            sources: c[8].trim().to_string(),
            whitelist: c
                .get(9)
                .map(|w| w.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect())
                .unwrap_or_default(),
            confirm: c.get(10).map(|x| x.trim() == "yes").unwrap_or(false),
            gloss_sq: c.get(11).map(|x| x.trim().to_string()).unwrap_or_default(),
        });
    }
    Ok(out)
}

/// Mbiemrat shqip citohen shpesh me nyje: «i mundur», «e bukur». Për fjalëformim përdoret tema.
pub fn strip_article(w: &str) -> &str {
    match (w.strip_prefix("i "), w.strip_prefix("e "), w.strip_prefix("të ")) {
        (Some(x), _, _) | (_, Some(x), _) | (_, _, Some(x)) => x,
        _ => w,
    }
}

/// Pjesët e ligjëratës të një fjale sipas të gjitha burimeve (kuptime, UD, ConceptNet).
pub fn word_pos(g: &Graph, w: u32) -> BTreeSet<Pos> {
    let mut out = BTreeSet::new();
    for (_, e) in g.out_edges(w) {
        match (&e.rel, &g.node(e.to).kind) {
            (Rel::HasSense, NodeKind::Sense { pos, .. }) => {
                out.insert(*pos);
            }
            (Rel::OccursIn, _) => match e.attr("upos").and_then(Pos::from_upos) {
                Some(p) => {
                    out.insert(p);
                }
                None => {}
            },
            _ => {}
        }
    }
    for (_, e) in g.in_edges(w) {
        match (&e.rel, e.attr("pos").and_then(Pos::from_wn)) {
            (Rel::FormOf, Some(p)) | (Rel::Lex, Some(p)) => {
                out.insert(p);
            }
            _ => {}
        }
    }
    out
}

/// Çiftet (prejardhur, bazë) që i deklaron një burim tjetër: Wiktionary/LEXIN
/// (`Rel::Sem("DERIVED_FROM")` prejardhur → bazë) dhe ConceptNet `/r/DerivedFrom`.
pub fn confirmations(g: &Graph) -> HashSet<(String, String)> {
    let mut out = gold_pairs(g);
    for e in &g.edges {
        match (&e.rel, &g.node(e.from).kind, &g.node(e.to).kind) {
            (Rel::Sem(r), NodeKind::Word { lemma: d, .. }, NodeKind::Word { lemma: b, .. }) => match r.as_str() {
                "DERIVED_FROM" => {
                    out.insert((strip_article(d).to_string(), strip_article(b).to_string()));
                }
                _ => {}
            },
            _ => {}
        }
    }
    out
}

/// Bazat e mundshme sipas rregullit (pa kontrollin e fjalorit).
pub fn base_candidates(rule: &Rule, stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (d, b) in &rule.alts {
        let cand = match rule.kind {
            Kind::Suffix => stem.strip_suffix(d.as_str()).map(|s| format!("{s}{b}")),
            Kind::Prefix => stem.strip_prefix(d.as_str()).map(|s| format!("{b}{s}")),
        };
        match cand {
            Some(c) => match (c.chars().count() >= rule.min_base, c != stem, out.contains(&c)) {
                (true, true, false) => out.push(c),
                _ => {}
            },
            None => {}
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub derived: u32,
    pub base: u32,
    pub rule: String,
    pub relation: String,
}

/// Zbaton rregullat mbi fjalët shqipe. Kthen lidhjet (pa i shtuar ende në graf).
pub fn find_links(g: &Graph, rules: &[Rule]) -> Vec<Link> {
    let confirmed = confirmations(g);
    // Indeks: tema pa nyje → nyjet Word (p.sh. «mundur» → «i mundur» dhe «mundur»).
    let mut by_stem: HashMap<String, Vec<u32>> = HashMap::new();
    let mut words: Vec<(u32, String)> = Vec::new();
    for (i, n) in g.nodes.iter().enumerate() {
        match &n.kind {
            NodeKind::Word { lang: Lang::Sq, lemma } => {
                let stem = strip_article(lemma).to_string();
                match stem.contains(' ') || stem.chars().count() < 3 {
                    true => {}
                    false => {
                        by_stem.entry(stem.clone()).or_default().push(i as u32);
                        words.push((i as u32, stem));
                    }
                }
            }
            _ => {}
        }
    }
    words.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut pos_cache: HashMap<u32, BTreeSet<Pos>> = HashMap::new();
    let mut pos_of = |g: &Graph, w: u32| -> BTreeSet<Pos> { pos_cache.entry(w).or_insert_with(|| word_pos(g, w)).clone() };
    let mut out = Vec::new();
    // Çiftet (prejardhur, bazë) të zëna: një çift, një analizë.
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    for (w, stem) in &words {
        for r in rules {
            match (r.exclude.contains(stem), r.whitelist.is_empty() || r.whitelist.contains(stem)) {
                (false, true) => {}
                _ => continue,
            }
            let dpos = pos_of(g, *w);
            let d_ok = r.result_pos.is_empty() || dpos.is_empty() || r.result_pos.iter().any(|p| dpos.contains(p));
            match d_ok {
                true => {}
                false => continue,
            }
            for cand in base_candidates(r, stem) {
                for &b in by_stem.get(&cand).map(|v| v.as_slice()).unwrap_or(&[]) {
                    let bpos = pos_of(g, b);
                    // Baza duhet të ketë pjesë ligjërate të njohur dhe të pranuar nga rregulli.
                    let b_ok = r.base_pos.is_empty() || r.base_pos.iter().any(|p| bpos.contains(p));
                    let c_ok = !r.confirm || confirmed.contains(&(stem.clone(), cand.clone()));
                    // Gjinia femërore e një emri vetie («mësuese» ← «mësues») vlen vetëm për qenie të
                    // gjalla: kur gramatika e shënon njërën anë si send («abetare», «margaritare»,
                    // «mesatare»), rregulli nuk zbatohet.
                    let a_ok = r.relation != "FEMININE_OF" || (!inanimate(g, *w) && !inanimate(g, b));
                    let c_ok = c_ok && a_ok;
                    // Çifti "zihet" vetëm kur rregulli e pranon (ndryshe një rregull i dështuar
                    // do të bllokonte një rregull të mëvonshëm të vlefshëm), dhe vetëm një herë:
                    // fiton rregulli i parë i tabelës, pavarësisht nga marrëdhënia.
                    match (b_ok && c_ok, b != *w) {
                        (true, true) => match seen.insert((*w, b)) {
                            true => out.push(Link { derived: *w, base: b, rule: r.id.clone(), relation: r.relation.clone() }),
                            false => {}
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    out
}

/// Gramatika (uniparser) e shënon fjalën vetëm si send (`anim=inanim`), asnjëherë si qenie.
fn inanimate(g: &Graph, w: u32) -> bool {
    let anim: Vec<&str> =
        g.out_edges(w).filter(|(_, e)| e.rel == Rel::Lex && e.source == Source::Uniparser).filter_map(|(_, e)| e.attr("anim")).collect();
    anim.contains(&"inanim") && !anim.contains(&"anim")
}

/// Çiftet e artë (prejardhur → bazë) nga ConceptNet /r/DerivedFrom shqip.
pub fn gold_pairs(g: &Graph) -> HashSet<(String, String)> {
    let mut out = HashSet::new();
    for e in &g.edges {
        match (&e.rel, &g.node(e.from).kind, &g.node(e.to).kind) {
            (Rel::Cn(r), NodeKind::Concept { uri: a }, NodeKind::Concept { uri: b }) => {
                match (r.as_str(), a.starts_with("/c/sq/"), b.starts_with("/c/sq/")) {
                    ("/r/DerivedFrom", true, true) => {
                        out.insert((concept_term(a).to_lowercase(), concept_term(b).to_lowercase()));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct RuleScore {
    pub links: usize,
    pub gold_hit: usize,
    /// Fjala e prejardhur është në standardin e artë, por me bazë tjetër.
    pub gold_miss: usize,
}

/// Saktësia për rregull: lidhjet që standardi i artë i konfirmon / i kundërshton.
pub fn evaluate(g: &Graph, links: &[Link], gold: &HashSet<(String, String)>) -> BTreeMap<String, RuleScore> {
    let derived_in_gold: HashSet<&String> = gold.iter().map(|(d, _)| d).collect();
    let mut out: BTreeMap<String, RuleScore> = BTreeMap::new();
    let lemma = |id: u32| match &g.node(id).kind {
        NodeKind::Word { lemma, .. } => strip_article(lemma).to_string(),
        _ => String::new(),
    };
    for l in links {
        let (d, b) = (lemma(l.derived), lemma(l.base));
        let s = out.entry(l.rule.clone()).or_default();
        s.links += 1;
        match (gold.contains(&(d.clone(), b.clone())), derived_in_gold.contains(&d)) {
            (true, _) => s.gold_hit += 1,
            (false, true) => s.gold_miss += 1,
            _ => {}
        }
    }
    out
}

/// Shton lidhjet në graf si skaje `Rel::Deriv(relation)` prejardhur → bazë.
pub fn apply(g: &mut Graph, rules: &[Rule], rep: &mut ImportReport) -> usize {
    let gold = gold_pairs(g);
    let links = find_links(g, rules);
    let scores = evaluate(g, &links, &gold);
    for (id, s) in &scores {
        let checked = s.gold_hit + s.gold_miss;
        rep.note(format!(
            "sq_rules {id}: {} lidhje; standardi i artë: {} të konfirmuara, {} me bazë tjetër{}",
            s.links,
            s.gold_hit,
            s.gold_miss,
            match checked {
                0 => String::new(),
                n => format!(" (saktësi {:.0}%)", 100.0 * s.gold_hit as f64 / n as f64),
            }
        ));
    }
    let by_id: HashMap<&str, &Rule> = rules.iter().map(|r| (r.id.as_str(), r)).collect();
    for l in &links {
        let r = by_id.get(l.rule.as_str());
        g.add_edge(Edge {
            from: l.derived,
            to: l.base,
            rel: Rel::Deriv(l.relation.clone()),
            source: Source::SqRules,
            lang: Some(Lang::Sq),
            method: Method::AlbanianRule,
            attrs: vec![
                ("rule".into(), l.rule.clone()),
                ("ref".into(), r.map(|r| r.sources.clone()).unwrap_or_default()),
                ("gloss".into(), r.map(|r| r.gloss_sq.clone()).unwrap_or_default()),
            ],
        });
    }
    rep.note(format!("sq_rules: {} lidhje semantike nga {} rregulla të fjalëformimit; standardi i artë ka {} çifte", links.len(), rules.len(), gold.len()));
    links.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = "# test\n\
D-PA-NEG\tprefix\tpa>\ta\ta\tNEGATION_OF\t3\tpallat,pastë\tGramatika 2002\n\
D-UES-AGENT\tsuffix\tues>oj;yes>ej\tv\tn\tAGENT_OF\t3\t\tGramatika 2002\n\
D-REV-ZB\tprefix\tzb>mb;sh>n\tv\tv\tREVERSAL_OF\t3\t\tGramatika 2002\n";

    /// Shton një fjalë shqipe me një kuptim të pjesës së ligjëratës `pos`.
    fn add(g: &mut Graph, lemma: &str, pos: Pos, key: &str) {
        let w = g.ensure_word(Lang::Sq, lemma);
        let s = g.ensure(sense_key(Source::Pwn30, key), NodeKind::Sense { source: Source::Pwn30, synset: key.into(), pos });
        g.add_edge(Edge { from: w, to: s, rel: Rel::HasSense, source: Source::OmwAls, lang: Some(Lang::Sq), method: Method::OmwOffsetMap, attrs: vec![] });
    }

    /// Lidhjet si (prejardhur, bazë, rregull, marrëdhënie), me fjalët siç citohen.
    fn described(g: &Graph, links: &[Link]) -> Vec<(String, String, String, String)> {
        links
            .iter()
            .map(|l| (g.display_label(l.derived).0, g.display_label(l.base).0, l.rule.clone(), l.relation.clone()))
            .collect()
    }

    fn graph() -> Graph {
        let mut g = Graph::new();
        add(&mut g, "i mundur", Pos::Adj, "1-a");
        add(&mut g, "i pamundur", Pos::Adj, "2-a");
        add(&mut g, "mësoj", Pos::Verb, "3-v");
        add(&mut g, "mësues", Pos::Noun, "4-n");
        add(&mut g, "mbuloj", Pos::Verb, "5-v");
        add(&mut g, "zbuloj", Pos::Verb, "6-v");
        add(&mut g, "pallat", Pos::Noun, "7-n");
        add(&mut g, "llat", Pos::Noun, "8-n");
        g
    }

    #[test]
    fn rules_parse_and_link_only_existing_bases_with_right_pos() {
        let rules = parse_rules(RULES).unwrap();
        assert_eq!(rules.len(), 3);
        let g = graph();
        let links = find_links(&g, &rules);
        let pairs: Vec<(String, String, String)> = links
            .iter()
            .map(|l| (g.display_label(l.derived).0, g.display_label(l.base).0, l.relation.clone()))
            .collect();
        assert!(pairs.contains(&("i pamundur".into(), "i mundur".into(), "NEGATION_OF".into())));
        assert!(pairs.contains(&("mësues".into(), "mësoj".into(), "AGENT_OF".into())));
        assert!(pairs.contains(&("zbuloj".into(), "mbuloj".into(), "REVERSAL_OF".into())));
        // «pallat» është në listën e përjashtimeve dhe s'ka POS mbiemër.
        assert!(!pairs.iter().any(|p| p.0 == "pallat"));
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn malformed_rule_rows_are_rejected() {
        assert!(parse_rules("X\tprefix\tpa>\ta\n").is_err());
        assert!(parse_rules("X\tinfix\tpa>\ta\ta\tR\t3\t\ts\n").is_err());
    }

    #[test]
    fn evaluation_counts_gold_confirmations() {
        let rules = parse_rules(RULES).unwrap();
        let g = graph();
        let links = find_links(&g, &rules);
        let gold: HashSet<(String, String)> = [("mësues".to_string(), "mësoj".to_string())].into_iter().collect();
        let s = evaluate(&g, &links, &gold);
        assert_eq!(s["D-UES-AGENT"].gold_hit, 1);
        assert_eq!(s["D-PA-NEG"].gold_hit, 0);
    }

    #[test]
    fn one_pair_gets_only_the_first_accepting_rule() {
        // Dy rregulla e pranojnë të njëjtin çift me marrëdhënie të ndryshme: fiton i pari.
        let first = "A\tprefix\tpa>\ta\ta\tNEGATION_OF\t3\t\ts\n\
B\tprefix\tpa>\ta,n\ta\tLACKING\t3\t\ts\n";
        let g = graph();
        let links = find_links(&g, &parse_rules(first).unwrap());
        assert_eq!(
            described(&g, &links),
            vec![("i pamundur".to_string(), "i mundur".to_string(), "A".to_string(), "NEGATION_OF".to_string())]
        );
        // Me rend të kundërt fiton tjetri; kurrë të dy.
        let swapped = "B\tprefix\tpa>\ta,n\ta\tLACKING\t3\t\ts\n\
A\tprefix\tpa>\ta\ta\tNEGATION_OF\t3\t\ts\n";
        let links = find_links(&g, &parse_rules(swapped).unwrap());
        assert_eq!(
            described(&g, &links),
            vec![("i pamundur".to_string(), "i mundur".to_string(), "B".to_string(), "LACKING".to_string())]
        );
    }

    #[test]
    fn a_rule_that_fails_does_not_claim_the_pair() {
        // A dështon te pjesa e ligjëratës (baza s'është folje), B te konfirmimi (s'ka burim
        // tjetër); asnjëri s'e zë çiftin, prandaj C e lidh.
        let t = "A\tprefix\tpa>\tv\ta\tNEGATION_OF\t3\t\ts\n\
B\tprefix\tpa>\ta\ta\tNEGATION_OF\t3\t\ts\t\tyes\n\
C\tprefix\tpa>\ta\ta\tLACKING\t3\t\ts\n";
        let g = graph();
        let links = find_links(&g, &parse_rules(t).unwrap());
        assert_eq!(
            described(&g, &links),
            vec![("i pamundur".to_string(), "i mundur".to_string(), "C".to_string(), "LACKING".to_string())]
        );
    }

    /// Tabela e vërtetë: lexohet e tëra, id-të janë të veçanta, çdo shabllon citon fjalën e
    /// prejardhur, përdor vetëm {d}, {b}, {db}, {bb} dhe s'ka shënime gramatikore («(…)», «-ar/-tar»).
    #[test]
    fn the_real_rule_table_is_well_formed() {
        let rules = parse_rules(include_str!("../../rules/sq_fjaleformimi.tsv")).unwrap();
        let ids: HashSet<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids.len(), rules.len(), "id e përsëritur në tabelë");
        for r in &rules {
            let t = r.gloss_sq.as_str();
            assert!(t.contains("{d}"), "{}: shablloni s'e citon fjalën e prejardhur", r.id);
            assert!(!t.contains(|c: char| matches!(c, '(' | ')' | '/')), "{}: shënim gramatikor në shabllon", r.id);
            let mut rest = t;
            loop {
                match rest.split_once('{') {
                    Some((_, tail)) => {
                        let (name, after) = tail.split_once('}').expect("kllapë e pambyllur në shabllon");
                        assert!(matches!(name, "d" | "b" | "db" | "bb"), "{}: vend i panjohur {{{name}}}", r.id);
                        rest = after;
                    }
                    None => break,
                }
            }
            assert!(r.whitelist.is_disjoint(&r.exclude), "{}: fjalë edhe në listën e bardhë, edhe te përjashtimet", r.id);
        }
        // Emri dhe folja me kundër- kanë rreshta të ndarë.
        let by_id: HashMap<&str, &Rule> = rules.iter().map(|r| (r.id.as_str(), r)).collect();
        assert_eq!(by_id["D-KUNDER.N"].base_pos, vec![Pos::Noun]);
        assert_eq!(by_id["D-KUNDER.V"].base_pos, vec![Pos::Verb]);
    }

    /// Vendimet e tabelës mbi një fjalor të vogël.
    #[test]
    fn real_table_decisions_on_a_small_lexicon() {
        let rules = parse_rules(include_str!("../../rules/sq_fjaleformimi.tsv")).unwrap();
        let mut g = Graph::new();
        // «derë» është emër, por një burim i jep edhe një homograf mbiemëror dialektor.
        add(&mut g, "derë", Pos::Noun, "1-n");
        add(&mut g, "derë", Pos::Adj, "2-a");
        add(&mut g, "i paderë", Pos::Adj, "3-a");
        // «i lumtur» ka edhe një kuptim emëror (i lumturi).
        add(&mut g, "i lumtur", Pos::Adj, "4-a");
        add(&mut g, "i lumtur", Pos::Noun, "5-n");
        add(&mut g, "i palumtur", Pos::Adj, "6-a");
        add(&mut g, "sulmoj", Pos::Verb, "7-v");
        add(&mut g, "kundërsulmoj", Pos::Verb, "8-v");
        add(&mut g, "sulm", Pos::Noun, "9-n");
        add(&mut g, "kundërsulm", Pos::Noun, "10-n");
        add(&mut g, "ardhje", Pos::Noun, "11-n");
        add(&mut g, "mosardhje", Pos::Noun, "12-n");
        add(&mut g, "besim", Pos::Noun, "13-n");
        add(&mut g, "mosbesim", Pos::Noun, "14-n");
        add(&mut g, "i bardhë", Pos::Adj, "15-a");
        add(&mut g, "zbardh", Pos::Verb, "16-v");
        add(&mut g, "ditë", Pos::Noun, "17-n");
        add(&mut g, "pasdite", Pos::Noun, "18-n");
        add(&mut g, "kuptim", Pos::Noun, "19-n");
        add(&mut g, "mirëkuptim", Pos::Noun, "20-n");
        add(&mut g, "grup", Pos::Noun, "21-n");
        add(&mut g, "nëngrup", Pos::Noun, "22-n");
        add(&mut g, "tokë", Pos::Noun, "23-n");
        add(&mut g, "nëntokë", Pos::Noun, "24-n");
        let all = described(&g, &find_links(&g, &rules));
        let of = |d: &str| -> Vec<(String, String, String)> {
            all.iter().filter(|x| x.0 == d).map(|x| (x.1.clone(), x.2.clone(), x.3.clone())).collect()
        };
        let one = |b: &str, rule: &str, rel: &str| vec![(b.to_string(), rule.to_string(), rel.to_string())];
        // «i paderë» = pa derë: mungesë, jo e kundërta e «derë».
        assert_eq!(of("i paderë"), one("derë", "D-PA-PRIV-N", "LACKING"));
        // Një çift, një analizë: vetëm NEGATION_OF, jo edhe LACKING.
        assert_eq!(of("i palumtur"), one("i lumtur", "D-PA-NEG-ADJ", "NEGATION_OF"));
        // Folja dhe emri me kundër- marrin secili rreshtin e vet.
        assert_eq!(of("kundërsulmoj"), one("sulmoj", "D-KUNDER.V", "COUNTER_OF"));
        assert_eq!(of("kundërsulm"), one("sulm", "D-KUNDER.N", "COUNTER_OF"));
        // mos- + emër veprimi = mungesë; vetëm antonimet e ngulitura janë "e kundërta".
        assert_eq!(of("mosardhje"), one("ardhje", "D-MOS-NEG", "ABSENCE_OF"));
        assert_eq!(of("mosbesim"), one("besim", "D-MOS-NEG.ANT", "NEGATION_OF"));
        assert_eq!(of("zbardh"), one("i bardhë", "D-CAUS-Z-ADJ", "CAUSATIVE_OF"));
        // «pasdite» = pas mesditës, jo "pas ditës": s'lidhet me «ditë».
        assert!(of("pasdite").is_empty());
        assert_eq!(of("mirëkuptim"), one("kuptim", "D-KEQ-MIRE.LEX", "DERIVED_FROM"));
        assert_eq!(of("nëngrup"), one("grup", "D-NEN.PART", "SUBDIVISION_OF"));
        assert_eq!(of("nëntokë"), one("tokë", "D-NEN.PLACE", "UNDER_OF"));
    }
}
