//! Rëndësia e fakteve mbi indeksin real: fakti i vërtetë por rastësor nuk bëhet fakt kryesor,
//! fakti për një nënlloj ose për një kuptim tjetër nuk thuhet për fjalën e zhveshur, dhe folja pa
//! kundrinor merr trajtën që kërkon shqipja.

use nura_lingua::biseda::converse;
use nura_lingua::fjalet::Fjalet;
use nura_lingua::gramatika::leksiku::Lexicon;
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

fn say(input: &str) -> String {
    let (b, lex, gram) = data();
    let eng = Engine::new(&b.graph, &b.morph, lex).with_grammar(gram);
    let forms = Fjalet::new(&eng);
    converse(&eng, &forms, input).map(|r| r.text()).unwrap_or_default()
}

#[test]
fn gold_is_not_presented_by_an_incidental_place() {
    let t = say("ar");
    assert!(t.contains("metal"), "{t}");
    assert!(!t.contains("dhëmb"), "{t}");
}

#[test]
fn iron_is_not_presented_by_what_its_name_already_says() {
    let t = say("hekur");
    assert!(t.contains("metal"), "{t}");
    assert!(!t.contains("hekurosur"), "{t}");
}

#[test]
fn the_car_does_not_take_the_facts_of_the_machine() {
    // Faktet e makinës si mekanizëm thuhen vetëm në fjalinë e atij leximi («Si pajisje, …»),
    // kurrë në fjalinë për veturën.
    let t = say("makina");
    for sentence in t.split(". ") {
        let machine = ["anijen", "njehsuar", "numëruar"].iter().any(|w| sentence.contains(w));
        assert!(!machine || sentence.starts_with("Si "), "{t}");
        assert!(!(sentence.contains("argëtim") && machine), "{t}");
    }
}

#[test]
fn a_subkind_fact_names_the_subkind() {
    let t = say("si lidhet libri me kishën");
    assert!(t.starts_with("Libri i shenjtë"), "{t}");
    assert!(!say("libër").contains("kishë"));
}

#[test]
fn posture_and_change_of_state_take_the_middle_voice() {
    assert!(say("karrigia").contains("për t'u ulur"), "{}", say("karrigia"));
    assert!(say("mali").contains("për t'u ngjitur"), "{}", say("mali"));
    assert!(say("dera").to_lowercase().contains("mbyllet"), "{}", say("dera"));
    // Folja me kundrinor të nënkuptuar mbetet veprore.
    assert!(say("uji").contains("për të pirë"), "{}", say("uji"));
}

#[test]
fn an_abstract_word_has_no_usual_place() {
    let t = say("dashuria");
    assert!(!t.contains("gjendet"), "{t}");
}

#[test]
fn the_answers_are_the_same_every_time() {
    for w in ["ar", "makina", "karrigia", "qeni", "dashuria"] {
        assert_eq!(say(w), say(w));
    }
}
