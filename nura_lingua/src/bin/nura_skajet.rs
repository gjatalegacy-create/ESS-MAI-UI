//! nura_skajet — shfaq skajet semantike (`Sem`) që dalin nga një fjalë shqipe dhe hyjnë në të,
//! me burimin dhe atributet. Mjet kontrolli për zhvilluesin.
//! Përdorimi: nura_skajet [--hyrje] [--kuptimet] [--rel CS_] fjala …

use nura_lingua::graph::{word_key, Lang, Rel};
use nura_lingua::query::Engine;
use nura_lingua::store::{default_data_dir, index_path, load_index, sq_lexicon};

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
    let lex = sq_lexicon(g);
    let eng = Engine::new(g, &b.morph, &lex);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let incoming = args.iter().any(|a| a == "--hyrje");
    let prefix = args.windows(2).find(|w| w[0] == "--rel").map(|w| w[1].clone()).unwrap_or_default();
    let words = args.iter().enumerate().filter(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i - 1] != "--rel")).map(|(_, a)| a);
    for w in words {
        let id = match g.get(&word_key(Lang::Sq, w)) {
            Some(id) => id,
            None => {
                println!("\n== {w}: nuk është në graf");
                continue;
            }
        };
        println!("\n== {w} (#{id})");
        match args.iter().any(|a| a == "--kuptimet") {
            true => {
                let f = eng.focus(w);
                for s in &f.senses {
                    println!("  kuptim {} {:?} via={} pikë={} en={:?} sq={:?}", s.synset, s.status, s.via.code(), s.score, s.labels_en, s.labels_sq);
                }
            }
            false => {}
        }
        let edges: Vec<_> = match incoming {
            true => g.in_edges(id).collect(),
            false => g.out_edges(id).collect(),
        };
        for (_, e) in edges {
            let name = match &e.rel {
                Rel::Sem(n) => n.clone(),
                _ => continue,
            };
            match name.starts_with(&prefix) {
                true => {}
                false => continue,
            }
            let other = match incoming {
                true => e.from,
                false => e.to,
            };
            let (label, _) = g.display_label(other);
            let attrs: Vec<String> = e.attrs.iter().map(|(k, v)| format!("{k}={v}")).collect();
            println!("  {name} → {label} [{}] {}", e.source.code(), attrs.join(" "));
        }
    }
}
