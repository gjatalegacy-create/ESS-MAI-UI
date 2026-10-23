//! Paraqitja vizuale e grafit dhe e procesit.
//!
//! Organizimi hapësinor ndjek identitetin e ESS-MAI (logoja Nura Legacy:
//! ylli qendror, ari majtas, blu djathtas — i₀ që ndahet në dy hapësira):
//! - qendra (0,0): i₀ — fjala në fokus;
//! - majtas (ari, Xi): dëshmitë e FORMËS — përdorime UD, koncepte/marrëdhënie ConceptNet;
//! - djathtas (blu, Yi): KUPTIMET — synsete Albanet/WordNet dhe marrëdhëniet e tyre.
//!
//! Rregull: çdo vijë në pamje është një skaj real i grafit (me burim, gjuhë,
//! metodë); afërsia vizuale nuk është marrëdhënie. Nyja që mungon në graf
//! (fjalë e pagjetur) shfaqet si `absent` me id të veçantë dhe pa skaje.

use crate::graph::*;
use crate::query::{Analysis, CandStatus};
use std::collections::{BTreeMap, HashSet};

pub const ABSENT_ID: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    X,
    Center,
    Y,
}

impl Side {
    pub fn code(self) -> &'static str {
        match self {
            Side::X => "X",
            Side::Center => "C",
            Side::Y => "Y",
        }
    }
}

#[derive(Debug, Clone)]
pub struct VNode {
    pub id: u32,
    pub key: String,
    pub label: String,
    pub lang: String,
    pub kind: &'static str,
    pub side: Side,
    pub x: i32,
    pub y: i32,
    pub status: String,
    pub note: String,
    pub source: &'static str,
}

#[derive(Debug, Clone)]
pub struct VEdge {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub rel: String,
    pub rel_label: String,
    pub source: &'static str,
    pub source_label: &'static str,
    pub lang: &'static str,
    pub method: &'static str,
    pub attrs: Vec<(String, String)>,
    pub on_path: bool,
}

#[derive(Debug, Clone, Default)]
pub struct View {
    pub center: Option<u32>,
    pub nodes: Vec<VNode>,
    pub edges: Vec<VEdge>,
}

fn kind_of(n: &Node) -> &'static str {
    match n.kind {
        NodeKind::Word { .. } => "word",
        NodeKind::Sense { .. } => "sense",
        NodeKind::Concept { .. } => "concept",
        NodeKind::Sentence { .. } => "sentence",
    }
}

fn node_source(n: &Node) -> &'static str {
    match &n.kind {
        NodeKind::Word { .. } => "lexicon",
        NodeKind::Sense { source, .. } => source.code(),
        NodeKind::Concept { .. } => Source::ConceptNet57.code(),
        NodeKind::Sentence { source, .. } => source.code(),
    }
}

struct Builder<'a> {
    g: &'a Graph,
    v: View,
    placed: HashSet<u32>,
}

impl<'a> Builder<'a> {
    fn new(g: &'a Graph) -> Self {
        Self { g, v: View::default(), placed: HashSet::new() }
    }

    fn place(&mut self, id: u32, side: Side, x: i32, y: i32, status: &str, note: String) {
        match self.placed.insert(id) {
            false => {
                // Nyja ekziston: përditëso vetëm statusin nëse ka më shumë peshë.
                match self.v.nodes.iter_mut().find(|n| n.id == id) {
                    Some(n) => match (n.status.as_str(), status) {
                        (_, "path" | "target" | "selected" | "focus") => n.status = status.to_string(),
                        _ => {}
                    },
                    None => {}
                }
            }
            true => {
                let n = self.g.node(id);
                let (label, lang) = self.g.display_label(id);
                let label = match &n.kind {
                    NodeKind::Sentence { .. } => {
                        let t = self.g.gloss_in(id, Lang::Sq).map(|t| t.text.clone()).unwrap_or(label);
                        let w: Vec<&str> = t.split_whitespace().collect();
                        match w.len() > 6 {
                            true => format!("{} …", w[..6].join(" ")),
                            false => t,
                        }
                    }
                    _ => label,
                };
                self.v.nodes.push(VNode {
                    id,
                    key: n.key.clone(),
                    label,
                    lang: match &n.kind {
                        NodeKind::Concept { uri } => uri.split('/').nth(2).unwrap_or("und").to_string(),
                        _ => lang.code().to_string(),
                    },
                    kind: kind_of(n),
                    side,
                    x,
                    y,
                    status: status.to_string(),
                    note,
                    source: node_source(n),
                });
            }
        }
    }

    /// Shton vetëm skajet reale ndërmjet nyjeve të vendosura.
    fn connect(&mut self, path_edges: &HashSet<u32>) {
        let mut seen = HashSet::new();
        let ids: Vec<u32> = self.v.nodes.iter().map(|n| n.id).filter(|&i| i != ABSENT_ID).collect();
        let set: HashSet<u32> = ids.iter().copied().collect();
        for id in ids {
            for (eid, e) in self.g.out_edges(id) {
                match (set.contains(&e.to), seen.insert(eid)) {
                    (true, true) => self.v.edges.push(VEdge {
                        id: eid,
                        from: e.from,
                        to: e.to,
                        rel: e.rel.code(),
                        rel_label: e.rel.label_sq(),
                        source: e.source.code(),
                        source_label: e.source.label(),
                        lang: e.lang.map(|l| l.code()).unwrap_or("-"),
                        method: e.method.code(),
                        attrs: e.attrs.clone(),
                        on_path: path_edges.contains(&eid),
                    }),
                    _ => {}
                }
            }
        }
    }
}

fn spread(i: usize, n: usize, span: i32) -> i32 {
    match n {
        0 | 1 => 0,
        _ => -span + (2 * span * i as i32) / (n as i32 - 1),
    }
}

fn status_of(s: &CandStatus) -> (&'static str, String) {
    match s {
        CandStatus::Open => ("open", String::new()),
        CandStatus::Eliminated(r) => ("eliminated", r.clone()),
        CandStatus::Selected(r) => ("selected", r.clone()),
    }
}

/// Pamja e një analize (kolapsi i fjalës).
pub fn analysis_view(g: &Graph, a: &Analysis) -> View {
    let mut b = Builder::new(g);
    let mut path_edges: HashSet<u32> = HashSet::new();
    let f = match &a.focus {
        Some(f) => f,
        None => return b.v,
    };
    match f.resolved.word {
        Some(w) => b.place(w, Side::Center, 0, 0, "focus", format!("i₀ «{}»", f.resolved.input)),
        None => b.v.nodes.push(VNode {
            id: ABSENT_ID,
            key: format!("absent:{}", f.resolved.lemma),
            label: f.resolved.lemma.clone(),
            lang: "sq".to_string(),
            kind: "absent",
            side: Side::Center,
            x: 0,
            y: 0,
            status: "absent".into(),
            note: "nuk ekziston në asnjë burim të importuar — nyje e paraqitur, jo e grafit".into(),
            source: "-",
        }),
    }
    b.v.center = f.resolved.word;
    // Yi — kuptimet
    let ns = f.senses.len();
    for (i, s) in f.senses.iter().enumerate() {
        let (st, note) = status_of(&s.status);
        b.place(s.sense, Side::Y, 380, spread(i, ns, 780), st, note);
    }
    // hiperonimi i drejtpërdrejtë i çdo kuptimi të hapur/zgjedhur (kontekst semantik)
    let parents: Vec<(u32, usize)> = f
        .senses
        .iter()
        .enumerate()
        .filter(|(_, s)| !matches!(s.status, CandStatus::Eliminated(_)))
        .filter_map(|(i, s)| {
            g.out_edges(s.sense)
                .find(|(_, e)| match &e.rel { Rel::Wn(x) => x == "@" || x == "@i", _ => false })
                .map(|(_, e)| (e.to, i))
        })
        .collect();
    for (p, i) in parents {
        b.place(p, Side::Y, 680, spread(i, ns, 780), "context", "hiperonim i drejtpërdrejtë".into());
    }
    // rruga e verifikimit (ISA)
    let np = a.path.len();
    for (i, st) in a.path.iter().enumerate().skip(1) {
        let x = 380 + (580 * i as i32) / (np as i32).max(2) ;
        let status = match i + 1 == np {
            true => "target",
            false => "path",
        };
        b.place(st.node, Side::Y, x.min(940), -620 + (i as i32 * 1240) / (np as i32).max(2), status, format!("hapi {i} i rrugës"));
    }
    path_edges.extend(a.path.iter().filter_map(|p| p.edge));
    for (k, p) in a.cond_path.iter().chain(a.bridge_path.iter()).enumerate() {
        b.place(p.node, Side::Y, 520 + ((k as i32 * 90) % 420), 700 - (k as i32 * 70) % 300, "condition", "rruga e kushtit / urës".into());
    }
    path_edges.extend(a.cond_path.iter().chain(a.bridge_path.iter()).filter_map(|p| p.edge));
    match &a.other {
        Some(o) => match o.resolved.word {
            Some(w) => b.place(w, Side::Y, 960, 760, "object", format!("objekti «{}»", o.resolved.lemma)),
            None => {}
        },
        None => {}
    }
    // Xi — forma dhe përdorimi
    let nu = f.usages.len().min(6);
    let concepts: Vec<u32> = {
        let mut c: Vec<u32> = f.form_rels.iter().map(|r| r.concept).collect();
        c.sort();
        c.dedup();
        c
    };
    let nx = nu + concepts.len();
    for (i, u) in f.usages.iter().take(6).enumerate() {
        b.place(u.sentence, Side::X, -420, spread(i, nx, 780), "usage", format!("forma «{}» · {} · {}", u.form, u.deprel, u.feats));
    }
    for (j, c) in concepts.iter().enumerate() {
        b.place(*c, Side::X, -420, spread(nu + j, nx, 780), "form", "koncept ConceptNet (pa kuptim të caktuar)".into());
    }
    let others: Vec<&crate::query::FormRel> = f.form_rels.iter().take(8).collect();
    let no = others.len();
    for (i, r) in others.iter().enumerate() {
        let on = path_or_rel(&a.steps, r.edge);
        b.place(r.other_concept, Side::X, -780, spread(i, no, 700), match on {
            true => "path",
            false => "form",
        }, format!("{} · {}", r.rel, r.dataset));
        match on {
            true => {
                path_edges.insert(r.edge);
            }
            false => {}
        }
    }
    b.connect(&path_edges);
    b.v
}

fn path_or_rel(steps: &[crate::query::ProcessStep], edge: u32) -> bool {
    steps.iter().any(|s| s.edges.contains(&edge))
}

/// Rendi i marrëdhënieve për eksplorim (më informueset para).
fn rel_rank(r: &Rel) -> u8 {
    match r {
        Rel::HasSense => 0,
        Rel::Wn(s) => match s.as_str() {
            "@" | "@i" => 1,
            "~" | "~i" => 2,
            "#p" | "%p" | "#m" | "%m" | "#s" | "%s" => 3,
            "!" => 4,
            _ => 6,
        },
        Rel::Cn(_) => 2,
        Rel::FormOf => 1,
        Rel::OccursIn => 3,
        Rel::DependsOn(_) => 5,
        Rel::Bn(_) => 1,
        Rel::Deriv(_) => 2,
        Rel::Sem(_) => 2,
        Rel::Lex => 9,
    }
}

/// Fqinjësia e një nyjeje për eksplorim të lirë.
pub fn neighborhood_view(g: &Graph, center: u32, per_side: usize) -> View {
    let mut b = Builder::new(g);
    b.place(center, Side::Center, 0, 0, "focus", "nyja në eksplorim".into());
    b.v.center = Some(center);
    let mut cands: Vec<(u8, u32, Side)> = Vec::new();
    for (_, e) in g.out_edges(center) {
        cands.push((rel_rank(&e.rel), e.to, side_for(g, e.to)));
    }
    for (_, e) in g.in_edges(center) {
        cands.push((rel_rank(&e.rel), e.from, side_for(g, e.from)));
    }
    cands.sort_by_key(|c| (c.0, c.1));
    let mut by_side: BTreeMap<&'static str, Vec<u32>> = BTreeMap::new();
    let mut seen = HashSet::from([center]);
    for (_, id, side) in cands {
        match seen.insert(id) {
            true => {
                let list = by_side.entry(side.code()).or_default();
                match list.len() < per_side {
                    true => list.push(id),
                    false => {}
                }
            }
            false => {}
        }
    }
    for (side_code, ids) in by_side {
        let (side, x) = match side_code {
            "X" => (Side::X, -480),
            _ => (Side::Y, 480),
        };
        let n = ids.len();
        for (i, id) in ids.into_iter().enumerate() {
            let col = match n > 9 {
                true => (i % 2) as i32 * 300,
                false => 0,
            };
            let xx = match side {
                Side::X => x - col,
                _ => x + col,
            };
            b.place(id, side, xx, spread(i, n, 820), "neighbor", String::new());
        }
    }
    b.connect(&HashSet::new());
    b.v
}

/// Ana e hapësirës: kuptimet → Yi; fjalët, fjalitë, konceptet → Xi.
fn side_for(g: &Graph, id: u32) -> Side {
    match g.node(id).kind {
        NodeKind::Sense { .. } => Side::Y,
        _ => Side::X,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighborhood_draws_only_real_edges() {
        let mut g = Graph::new();
        let w = g.ensure_word(Lang::Sq, "qen");
        let s = g.ensure("s:pwn30:1-n".into(), NodeKind::Sense { source: Source::Pwn30, synset: "1-n".into(), pos: Pos::Noun });
        let t = g.ensure("s:pwn30:2-n".into(), NodeKind::Sense { source: Source::Pwn30, synset: "2-n".into(), pos: Pos::Noun });
        g.add_edge(Edge { from: w, to: s, rel: Rel::HasSense, source: Source::OmwAls, lang: Some(Lang::Sq), method: Method::OmwOffsetMap, attrs: vec![] });
        g.add_edge(Edge { from: s, to: t, rel: Rel::Wn("@".into()), source: Source::Pwn30, lang: Some(Lang::En), method: Method::Published, attrs: vec![] });
        let v = neighborhood_view(&g, s, 10);
        assert_eq!(v.nodes.len(), 3);
        assert_eq!(v.edges.len(), 2);
        // fjala në anën X, kuptimi fqinj në anën Y
        assert_eq!(v.nodes.iter().find(|n| n.id == w).unwrap().side, Side::X);
        assert_eq!(v.nodes.iter().find(|n| n.id == t).unwrap().side, Side::Y);
        for e in &v.edges {
            assert!(g.edges[e.id as usize].from == e.from && g.edges[e.id as usize].to == e.to);
        }
    }
}
