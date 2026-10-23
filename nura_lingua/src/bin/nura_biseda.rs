//! nura_biseda — provë e bisedës në terminal me indeksin real.
//! Përdorimi: nura_biseda [--nje-deshmitar] "qeni dhe macja" ["pse qan fëmija" …]
//! Me `--nje-deshmitar` thuhen edhe lidhjet me një dëshmitar (vetëm për parapamje, jo për Nurën).

use nura_lingua::biseda::{converse_with, read, spoken};
use nura_lingua::fjalet::Fjalet;
use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::graph::Source;
use nura_lingua::query::Engine;
use nura_lingua::store::{default_data_dir, index_path, load_index, sq_lexicon};
use std::collections::BTreeSet;

fn one(s: &BTreeSet<Source>, _d: &BTreeSet<String>, _strong: bool) -> bool {
    s.contains(&Source::ConceptNet57)
}

fn main() {
    let data = default_data_dir();
    let b = match load_index(&std::env::var("NURA_INDEX").map(std::path::PathBuf::from).unwrap_or_else(|_| index_path(&data))) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("indeksi: {e}");
            std::process::exit(2);
        }
    };
    let lx = Lexicon::load(&data).unwrap_or_default();
    let lex = sq_lexicon(&b.graph);
    let eng = Engine::new(&b.graph, &b.morph, &lex).with_grammar(&lx);
    let forms = Fjalet::new(&eng);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rule = match args.iter().any(|a| a == "--nje-deshmitar") {
        true => one,
        false => spoken,
    };
    for input in args.iter().filter(|a| !a.starts_with("--")) {
        let (intent, words) = read(input);
        println!("\n== {input}   [{} {:?}]", intent.code(), words);
        match converse_with(&eng, &forms, input, rule) {
            Some(r) => {
                println!("   Nura: {}", r.text());
                r.question.iter().for_each(|q| println!("   pyetja: {q}"));
            }
            None => println!("   (asnjë lidhje e folshme: flet rruga e fjalës)"),
        }
    }
}
