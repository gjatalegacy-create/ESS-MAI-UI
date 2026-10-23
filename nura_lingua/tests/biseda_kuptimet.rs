//! Fjalët me disa lexime dhe lidhjet sipas përkufizimit, mbi indeksin real.
//! «Bari» është bima (trajta e shquar e «bar») dhe njeriu që ruan bagëtinë; Nura i thotë të dyja
//! dhe i lidh sipas shpjegimeve, pa probabilitet. Fjala e shkruar pa «ë»/«ç» kuptohet.

use nura_lingua::biseda::{converse, readings};
use nura_lingua::fjalet::Fjalet;
use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::graph::{word_key, Lang, Rel};
use nura_lingua::query::Engine;
use nura_lingua::store::{default_data_dir, index_path, load_index, sq_lexicon, Built};
use std::collections::HashMap;
use std::sync::OnceLock;

fn data() -> &'static (Built, HashMap<String, u32>, Lexicon) {
    static D: OnceLock<(Built, HashMap<String, u32>, Lexicon)> = OnceLock::new();
    D.get_or_init(|| {
        let dir = default_data_dir();
        let b = load_index(&index_path(&dir)).expect("indeksi mungon: ekzekuto `cargo run -p nura_lingua --bin nura_import`");
        let lex = sq_lexicon(&b.graph);
        let gram = Lexicon::load(&dir).expect("leksiku gramatikor");
        (b, lex, gram)
    })
}

fn engine() -> Engine<'static> {
    let (b, lex, gram) = data();
    Engine::new(&b.graph, &b.morph, lex).with_grammar(gram)
}

fn say(input: &str) -> (String, Option<String>) {
    let eng = engine();
    let forms = Fjalet::new(&eng);
    converse(&eng, &forms, input).map(|r| (r.text(), r.question.clone())).unwrap_or_default()
}

fn mentions(from: &str) -> Vec<String> {
    let (b, _, _) = data();
    let g = &b.graph;
    let w = g.get(&word_key(Lang::Sq, from)).expect("fjala në graf");
    g.out_edges(w)
        .filter(|(_, e)| e.rel == Rel::Sem("DEF_MENTIONS".into()))
        .map(|(_, e)| g.display_label(e.to).0)
        .collect()
}

#[test]
fn bari_is_both_the_grass_and_the_shepherd_linked_by_their_definitions() {
    let (t, q) = say("bari");
    assert!(t.contains("Bariu është njeri që ruan"), "{t}");
    assert!(t.contains("bimë"), "{t}");
    assert!(t.contains("lidhen përmes"), "lidhja sipas shpjegimeve: {t}");
    let q = q.unwrap_or_default();
    assert!(q.contains("bariun") && q.contains("barin"), "{q}");
}

#[test]
fn ari_is_both_gold_and_the_bear() {
    let (t, q) = say("ari");
    assert!(t.contains("metal") && t.contains("Ariu është gjitar"), "{t}");
    assert!(q.unwrap_or_default().contains("ariun"));
}

#[test]
fn a_word_written_without_diacritics_is_understood() {
    assert!(say("vere").0.contains("Vera"), "{}", say("vere").0);
    assert!(say("dere").0.starts_with("Dera"), "{}", say("dere").0);
    assert!(say("kale").0.starts_with("Kali"), "{}", say("kale").0);
    let eng = engine();
    assert!(readings(&eng, "dere").iter().any(|(l, _)| l == "derë"));
}

#[test]
fn definitions_link_words_by_what_they_mention() {
    let k = mentions("kullotë");
    assert!(k.contains(&"bar".to_string()) && k.contains(&"bagëti".to_string()), "{k:?}");
    // «bari» në një përkufizim mund të jetë bariu ose bari (bima): HOLD, jo lidhje e gabuar.
    let d = mentions("derrar");
    assert!(!d.contains(&"bari".to_string()) && !d.contains(&"bar".to_string()), "{d:?}");
}

#[test]
fn several_readings_are_said_the_same_way_every_time() {
    for w in ["bari", "ari", "vere", "gjuha"] {
        assert_eq!(say(w), say(w));
    }
}
