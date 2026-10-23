//! nura_import — importon paketat e shkarkuara (data/raw) dhe shkruan indeksin
//! lokal data/index/nura_graph.idx + data/index/IMPORT_REPORT.txt.

use nura_lingua::store::{default_data_dir, import_all, index_path, write_index};

fn main() {
    let data = default_data_dir();
    let t0 = std::time::Instant::now();
    match import_all(&data) {
        Ok(b) => {
            let p = index_path(&data);
            match write_index(&b, &p) {
                Ok(()) => {
                    let report = b.report.lines.join("\n");
                    let _ = std::fs::write(data.join("index").join("IMPORT_REPORT.txt"), format!("{report}\n"));
                    println!("{report}");
                    println!(
                        "[nura_import] {} nyje, {} skaje, {} hyrje morfologjike → {} ({} ms)",
                        b.graph.nodes.len(),
                        b.graph.edges.len(),
                        b.morph.entries.len(),
                        p.display(),
                        t0.elapsed().as_millis()
                    );
                }
                Err(e) => {
                    eprintln!("[nura_import] shkrimi i indeksit dështoi: {e}");
                    std::process::exit(2);
                }
            }
        }
        Err(e) => {
            eprintln!("[nura_import] GABIM: {e}");
            std::process::exit(2);
        }
    }
}
