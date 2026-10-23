//! nura_cs_export — kandidatët e shtresës së dijes së përbashkët për shqyrtim: lexon indeksin
//! lokal dhe shkruan `data/derived/cs_candidates.tsv`.
//!
//! Rregullat: vetëm skajet `CS_…` të ConceptNet-it me peshë ≥ 1.0; të shumtën 12 për (kokë,
//! marrëdhënie), sipas peshës; renditja: koka me më shumë marrëdhënie së pari, pastaj pesha.
//! Kolona `map` jep kodin e hartimit (`AB`, `AB2`, `A1`, `B1`); `+M` kur përkthimin e dobët e
//! dëshmon edhe leksiku i modelit.
//!
//! Me `--raundi2`: të gjitha skajet që ende kanë vetëm një dëshmitar (pa dëshminë e modelit dhe
//! jo të folshme nga dy grupe të dhënash), pa prag peshe dhe pa kufi për kokë, në
//! `data/derived/cs_round2.tsv`, për raundin e dytë të dëshmisë.

use nura_lingua::graph::{Rel, Source};
use nura_lingua::src_commonsense::{render, spoken, PREFIX};
use nura_lingua::store::{default_data_dir, index_path, load_index};
use std::collections::{BTreeMap, HashMap};

const CAP: usize = 12;
const MIN_WEIGHT: f64 = 1.0;
const HEADER: &str = "cid\trel\thead_sq\ttail_sq\tsq_sentence_hint\ten_head\ten_tail\tweight\tdatasets\trows\tmap";

struct Row {
    cid: String,
    rel: String,
    head: String,
    tail: String,
    hint: String,
    en_head: String,
    en_tail: String,
    weight: f64,
    ds: String,
    rows: String,
    map: String,
}

fn main() {
    let data = default_data_dir();
    let b = match load_index(&index_path(&data)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[nura_cs_export] indeksi: {e}");
            std::process::exit(2);
        }
    };
    let g = &b.graph;
    let round2 = std::env::args().any(|a| a == "--raundi2");
    let mut all: Vec<Row> = Vec::new();
    for e in &g.edges {
        let rel = match (&e.rel, e.source) {
            (Rel::Sem(r), Source::ConceptNet57) => r,
            _ => continue,
        };
        match (rel.starts_with(PREFIX), round2 && spoken(g, e.from, e)) {
            (true, false) => {}
            _ => continue,
        }
        let get = |k: &str| e.attr(k).unwrap_or("").to_string();
        let (en_head, en_tail) = e.attr("en").and_then(|x| x.split_once('|')).unwrap_or(("", ""));
        let (head, tail) = (get("head"), get("tail"));
        all.push(Row {
            cid: get("cid"),
            rel: rel.clone(),
            hint: format!("{} — {}", render(g, &head), render(g, &tail)),
            head,
            tail,
            en_head: en_head.to_string(),
            en_tail: en_tail.to_string(),
            weight: e.attr("w").and_then(|x| x.parse().ok()).unwrap_or(0.0),
            ds: get("ds"),
            rows: get("rows"),
            map: match e.attr("witness") {
                Some(_) => format!("{}+M", get("map")),
                None => get("map"),
            },
        });
    }
    // Rëndësia e kokës: sa marrëdhënie të kësaj shtrese ka (para pragut të peshës).
    let mut importance: HashMap<String, usize> = HashMap::new();
    for r in &all {
        *importance.entry(r.head.clone()).or_default() += 1;
    }
    let total = all.len();
    let mut groups: BTreeMap<(String, String), Vec<Row>> = BTreeMap::new();
    for r in all.into_iter().filter(|r| round2 || r.weight >= MIN_WEIGHT) {
        groups.entry((r.head.clone(), r.rel.clone())).or_default().push(r);
    }
    let mut out: Vec<Row> = Vec::new();
    for (_, mut v) in groups {
        v.sort_by(|a, b| b.weight.total_cmp(&a.weight).then_with(|| a.cid.cmp(&b.cid)));
        match round2 {
            true => {}
            false => v.truncate(CAP),
        }
        out.extend(v);
    }
    out.sort_by(|a, b| {
        let (ia, ib) = (importance.get(&a.head).copied().unwrap_or(0), importance.get(&b.head).copied().unwrap_or(0));
        ib.cmp(&ia)
            .then_with(|| a.head.cmp(&b.head))
            .then_with(|| b.weight.total_cmp(&a.weight))
            .then_with(|| a.rel.cmp(&b.rel))
            .then_with(|| a.cid.cmp(&b.cid))
    });
    let mut text = String::with_capacity(out.len() * 160);
    text.push_str(HEADER);
    text.push('\n');
    let mut by_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_rel: BTreeMap<String, usize> = BTreeMap::new();
    for r in &out {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{}\t{}\n",
            r.cid, r.rel, r.head, r.tail, r.hint, r.en_head, r.en_tail, r.weight, r.ds, r.rows, r.map
        ));
        *by_map.entry(r.map.clone()).or_default() += 1;
        *by_rel.entry(r.rel.clone()).or_default() += 1;
    }
    let dir = data.join("derived");
    let path = dir.join(match round2 {
        true => "cs_round2.tsv",
        false => "cs_candidates.tsv",
    });
    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
        Ok(()) => println!(
            "[nura_cs_export] {} kandidatë nga {total} skaje CS_ → {}; sipas hartimit {by_map:?}; sipas marrëdhënies {by_rel:?}",
            out.len(),
            path.display()
        ),
        Err(e) => {
            eprintln!("[nura_cs_export] shkrimi dështoi: {e}");
            std::process::exit(2);
        }
    }
}
