//! nura_def_export — mostër deterministe e lidhjeve sipas përkufizimit (`DEF_MENTIONS`) për
//! kontroll të pavarur: çdo i k-ti skaj, me shpjegimin, fjalën e shpjegimit dhe lemën e zgjedhur.
//! Përdorimi: nura_def_export [sa=300] > data/derived/def_audit.tsv

use nura_lingua::graph::{sense_key, Lang, NodeKind, Rel, Source};
use nura_lingua::src_perkufizime::{tokens, REL};
use nura_lingua::store::{default_data_dir, index_path, load_index};

fn main() {
    let data = default_data_dir();
    let b = match load_index(&index_path(&data)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("indeksi: {e}");
            std::process::exit(2);
        }
    };
    let g = &b.graph;
    let want: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(300);
    let rel = Rel::Sem(REL.into());
    let mut rows: Vec<(String, String, String, String, String, String)> = Vec::new();
    for i in 0..g.nodes.len() as u32 {
        let from = match &g.node(i).kind {
            NodeKind::Word { lemma, lang: Lang::Sq, .. } => lemma.clone(),
            _ => continue,
        };
        for (_, e) in g.out_edges(i).filter(|(_, e)| e.rel == rel) {
            let sense = e.attr("sense").unwrap_or("").to_string();
            let gloss = match sense.split_once(':') {
                Some(("gazulli1941", _)) | None => g
                    .out_edges(i)
                    .find(|(_, x)| x.rel == Rel::Sem("REGIONAL_GLOSS".into()))
                    .and_then(|(_, x)| x.attr("gloss"))
                    .unwrap_or("")
                    .to_string(),
                Some((code, syn)) => Source::parse(code)
                    .and_then(|s| g.get(&sense_key(s, syn)))
                    .and_then(|n| g.node(n).glosses.iter().find(|t| t.lang == Lang::Sq && t.source == e.source))
                    .map(|t| t.text.clone())
                    .unwrap_or_default(),
            };
            let pos: usize = e.attr("i").and_then(|x| x.parse().ok()).unwrap_or(0);
            let token = tokens(&gloss).get(pos).cloned().unwrap_or_default();
            let to = g.display_label(e.to).0;
            rows.push((from.clone(), e.source.code().to_string(), gloss.replace('\t', " "), token, to, e.attr("genus").unwrap_or("").to_string()));
        }
    }
    rows.sort();
    let step = (rows.len() / want.max(1)).max(1);
    println!("# mostër e lidhjeve DEF_MENTIONS: çdo e {step}-ta nga {} (e renditur sipas fjalës)", rows.len());
    println!("fjala\tburimi\tshpjegimi\tfjala_në_shpjegim\tlema_e_zgjedhur\tlloji");
    for r in rows.iter().step_by(step).take(want) {
        println!("{}\t{}\t{}\t{}\t{}\t{}", r.0, r.1, r.2, r.3, r.4, r.5);
    }
}
