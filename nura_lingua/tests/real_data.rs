//! Teste integrimi mbi indeksin real (data/index/nura_graph.idx).
//! Kërkojnë që `nura_fetch` dhe `nura_import` të jenë ekzekutuar më parë.
//! Nëse indeksi mungon, testet dështojnë me mesazh të qartë (nuk anashkalohen).

use nura_lingua::emotion::{EmotionMachine, Phase, Register};
use nura_lingua::expression::{realize, signature, Claim, Polarity, Verdict};
use nura_lingua::graph::{Lang, Method, NodeKind, Rel, Source};
use nura_lingua::layout::{analysis_view, neighborhood_view};
use nura_lingua::query::{CandStatus, Engine};
use nura_lingua::store::{conceptnet_partial, default_data_dir, index_path, load_index, read_lock, sq_lexicon, Built};
use std::collections::HashMap;
use std::sync::OnceLock;

fn data() -> &'static (Built, HashMap<String, u32>) {
    static D: OnceLock<(Built, HashMap<String, u32>)> = OnceLock::new();
    D.get_or_init(|| {
        let b = load_index(&index_path(&default_data_dir()))
            .expect("indeksi mungon: ekzekuto `cargo run -p nura_fetch` dhe `cargo run -p nura_lingua --bin nura_import`");
        let lex = sq_lexicon(&b.graph);
        (b, lex)
    })
}

fn engine() -> Engine<'static> {
    let (b, lex) = data();
    Engine::new(&b.graph, &b.morph, lex).with_conceptnet_partial(cn_partial())
}

fn run_phase(input: &str) -> (nura_lingua::query::Analysis, Phase) {
    let a = engine().analyze(input).expect("hyrje e mbështetur");
    let mut m = EmotionMachine::new();
    for s in &a.steps {
        s.event.map(|e| m.apply(e));
    }
    (a, m.phase)
}

#[test]
fn importer_linked_albanian_lemmas_to_wordnet_senses_with_declared_method() {
    let (b, lex) = data();
    let qen = *lex.get("qen").expect("«qen» në Albanet");
    let senses = b.graph.senses_of(qen);
    assert!(!senses.is_empty());
    for (sid, e) in senses {
        assert_eq!(e.source, Source::OmwAls);
        assert_eq!(e.method, Method::OmwOffsetMap);
        assert!(matches!(b.graph.node(sid).kind, NodeKind::Sense { source: Source::Pwn30, .. }));
    }
}

#[test]
fn word_with_several_meanings_is_not_collapsed_to_one_sense() {
    let (b, lex) = data();
    let zjarr = *lex.get("zjarr").unwrap();
    let senses = b.graph.senses_of(zjarr);
    assert_eq!(senses.len(), 3, "zjarr ka 3 kuptime në Albanet");
    let keys: std::collections::HashSet<_> = senses.iter().map(|(s, _)| *s).collect();
    assert_eq!(keys.len(), 3, "kuptimet mbeten nyje të dallueshme");
}

#[test]
fn isa_question_is_verified_by_a_real_hypernym_path() {
    let (a, phase) = run_phase("A është qeni kafshë?");
    assert_eq!(phase, Phase::NuraSpeaking);
    assert!(a.path.len() >= 2);
    let (b, _) = data();
    for w in a.path.windows(2) {
        let e = &b.graph.edges[w[0].edge.expect("skaj") as usize];
        assert_eq!((e.from, e.to), (w[0].node, w[1].node));
        assert!(match &e.rel { Rel::Wn(s) => s == "@" || s == "@i", _ => false });
    }
    assert!(matches!(a.plan.core[0].claim, Claim::IsA { .. }));
    assert_eq!(a.plan.core[0].polarity, Polarity::Pos);
}

#[test]
fn negated_user_claim_keeps_its_negation_and_is_contradicted() {
    let (a, phase) = run_phase("Qeni nuk është kafshë");
    assert_eq!(phase, Phase::NuraSpeaking);
    assert_eq!(a.plan.verdict, Verdict::UserClaimContradicted);
    assert_eq!(a.plan.user_claim.as_ref().unwrap().polarity, Polarity::Neg);
    let (b, _) = data();
    for r in Register::ALL {
        let x = realize(&a.plan, &b.morph, r);
        assert_eq!(x.core_text, "Jo, qeni është kafshë", "{r:?}");
    }
}

#[test]
fn missing_link_is_absence_not_negation() {
    let (a, phase) = run_phase("A është qeni bimë?");
    assert_eq!(phase, Phase::NegativeWitness);
    assert_eq!(a.plan.verdict, Verdict::Unresolved);
    assert!(matches!(a.plan.core[0].claim, Claim::NoLink { .. }));
    let (b, _) = data();
    let t = realize(&a.plan, &b.morph, Register::Witnessing).core_text;
    assert_eq!(t, "Nuk e di nëse «qen» është «bimë»");
    assert!(!t.contains("qeni nuk është bimë"), "mungesa nuk shndërrohet në mohim faktik");
}

#[test]
fn missing_word_is_reported_per_albanian_source_without_invented_links() {
    // Fjalë që nuk e ka asnjë burim (pas LEXIN, Wiktionary, Wikidata, uniparser…).
    let (a, phase) = run_phase("kërcëllimëz");
    assert_eq!(phase, Phase::NegativeWitness);
    match &a.plan.core[0].claim {
        Claim::NotInSource { sources, partial, .. } => {
            assert!(!sources.contains(&Source::Pwn30), "WordNet nuk ka lema shqipe");
            assert_eq!(*partial, cn_partial(), "shënimi i pjesshmërisë ndjek SOURCES.lock");
        }
        other => panic!("pritej NotInSource, erdhi {other:?}"),
    }
    let (b, _) = data();
    let v = analysis_view(&b.graph, &a);
    assert!(v.edges.is_empty(), "asnjë lidhje e shpikur për fjalën që mungon");
}

#[test]
fn ambiguity_holds_and_asks_then_context_eliminates() {
    // Fjala e vetme merr përgjigjen e bisedës; pyetja e shprehur për kuptimin mban rrugën e
    // kuptimeve: flitet për më të zakonshmin dhe pyetet për të tjerët.
    let (a, phase) = run_phase("ç'kuptim ka zjarr");
    assert!(matches!(phase, Phase::HoldIzQuestion | Phase::NuraSpeaking), "{phase:?}");
    // Pyetja është e shkurtër dhe e natyrshme, jo listë e numëruar kuptimesh.
    let q = a.plan.question.clone().unwrap();
    assert!(q.to_lowercase().contains("e ke fjalën për «zjarr»") && !q.contains("(1)"), "{q}");
    let (a2, phase2) = run_phase("zjarr armë");
    assert_eq!(phase2, Phase::NuraSpeaking);
    let f = a2.focus.unwrap();
    let selected: Vec<_> = f.senses.iter().filter(|s| matches!(s.status, CandStatus::Selected(_))).collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].synset, "00986938-n");
    assert_eq!(f.senses.iter().filter(|s| matches!(s.status, CandStatus::Eliminated(_))).count(), 2);
}

#[test]
fn user_choice_resolves_hold() {
    let a = engine().choose_sense("zjarr", "s:pwn30:07302836-n").unwrap();
    assert!(matches!(a.plan.core[0].claim, Claim::HasMeaning { index: 2, of: 3, .. }));
    assert!(engine().choose_sense("zjarr", "s:pwn30:02083346-n").is_err(), "kuptim i një fjale tjetër refuzohet");
}

#[test]
fn opposites_are_witnessed_and_cited_with_their_article() {
    let (a, phase) = run_phase("e kundërta e bardhë");
    assert_eq!(phase, Phase::NuraSpeaking);
    let opp: Vec<String> = a
        .plan
        .core
        .iter()
        .filter_map(|c| match &c.claim {
            Claim::Opposite { b, .. } => Some(b.lemma.clone()),
            _ => None,
        })
        .collect();
    // Mbiemri citohet me nyjën e tij; e kundërta ka burimet që e dëshmuan.
    assert!(opp.contains(&"i zi".to_string()), "{opp:?}");
    for c in &a.plan.core {
        let at = c.attribution.as_ref().unwrap();
        assert!(!at.sources.is_empty(), "çdo e kundërt ka burim");
    }
}

#[test]
fn conditional_is_preserved_in_every_register() {
    let (a, _) = run_phase("Nëse qeni është kafshë, atëherë qeni është organizëm");
    assert!(a.plan.condition.is_some());
    let (b, _) = data();
    let base = realize(&a.plan, &b.morph, Register::Neutral).core_text;
    assert!(base.contains("nëse qeni është kafshë, atëherë qeni është organizëm"), "{base}");
    for r in Register::ALL {
        assert_eq!(realize(&a.plan, &b.morph, r).core_text, base);
    }
}

#[test]
fn tone_never_changes_signature_or_core_for_sample_inputs() {
    let (b, _) = data();
    for input in [
        "qen",
        "zjarr",
        "zjarr armë",
        "A është qeni kafshë?",
        "Qeni nuk është kafshë",
        "A është qeni bimë?",
        "e kundërta e bardhë",
        "mace",
        "qyqar",
        "a është arra pemë?",
        "bardhë",
        "Nëse qeni është kafshë, atëherë qeni është organizëm",
    ] {
        let a = engine().analyze(input).unwrap();
        let sig = signature(&a.plan);
        let base = realize(&a.plan, &b.morph, Register::Neutral).core_text;
        for r in Register::ALL {
            let x = realize(&a.plan, &b.morph, r);
            assert_eq!(x.core_text, base, "{input} / {r:?}");
            assert!(x.full.contains(&base));
        }
        assert_eq!(signature(&a.plan), sig);
        // "nuk" në thelb vetëm kur ka një fjali me polaritet negativ
        let has_neg = a.plan.core.iter().chain(a.plan.user_claim.iter()).any(|c| c.polarity == Polarity::Neg);
        // Realizuesi nuk shpik mohim: "nuk" del vetëm kur plani ka një fjali negative.
        // Teksti i një përkufizimi («… dhe nuk ka askënd») vjen nga burimi, jo nga realizuesi.
        let glosses: Vec<String> = a
            .plan
            .core
            .iter()
            .filter_map(|c| match &c.claim {
                Claim::HasMeaning { gloss, .. } => Some(vec![gloss.clone()]),
                Claim::Facet { facet: nura_lingua::expression::Facet::Example, items, .. } => Some(items.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        let own = glosses.iter().fold(base.clone(), |t, g| t.replace(g.trim_end_matches('.'), ""));
        match own.contains("nuk ") { true => assert!(has_neg, "{input}: {base}"), false => {} }
    }
}

#[test]
fn english_text_is_never_presented_as_albanian_context() {
    let (a, _) = run_phase("A është qeni kafshë?");
    let (b, _) = data();
    let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
    assert!(!t.contains("fissiped"), "përkufizimi anglisht nuk hyn si kontekst shqip: {t}");
}

#[test]
fn every_drawn_line_is_a_real_edge_with_provenance() {
    let (b, _) = data();
    for input in ["zjarr armë", "A është qeni kafshë?", "e kundërta e bardhë", "bardhë"] {
        let a = engine().analyze(input).unwrap();
        let v = analysis_view(&b.graph, &a);
        assert!(!v.edges.is_empty(), "{input}");
        for e in &v.edges {
            let g = &b.graph.edges[e.id as usize];
            assert_eq!((g.from, g.to), (e.from, e.to));
            assert_eq!(g.source.code(), e.source);
        }
    }
    let qen = *data().1.get("qen").unwrap();
    let v = neighborhood_view(&b.graph, qen, 14);
    assert!(v.edges.iter().all(|e| e.from == qen || e.to == qen || b.graph.edges[e.id as usize].from == e.from));
}

#[test]
fn ud_contexts_are_attached_as_form_evidence() {
    let (b, lex) = data();
    let w = *lex.get("bardhë").unwrap();
    let occ: Vec<_> = b.graph.out_edges(w).filter(|(_, e)| e.rel == Rel::OccursIn).collect();
    assert!(!occ.is_empty());
    for (_, e) in occ {
        assert!(matches!(e.source, Source::UdSqTsa | Source::UdSqStaf));
        assert!(e.attr("form").is_some() && e.attr("feats").is_some());
        assert_eq!(e.lang, Some(Lang::Sq));
    }
}

fn cn_partial() -> bool {
    conceptnet_partial(&read_lock(&default_data_dir()).expect("SOURCES.lock"))
}

#[test]
fn conceptnet_concepts_in_other_languages_keep_their_language() {
    let (b, _) = data();
    for n in b.graph.nodes.iter() {
        match &n.kind {
            NodeKind::Concept { uri } => {
                let code = uri.split('/').nth(2).unwrap_or("");
                for t in &n.labels {
                    assert_eq!(t.lang.code(), code, "{uri}");
                }
            }
            _ => {}
        }
    }
}

#[test]
fn word_without_albanian_definition_is_described_from_word_facts_with_languages() {
    // «diell» nuk ka përkufizim shqip në AlbaNet: përgjigjja vjen nga faktet e fjalës dhe nuk
    // thuhet "mungon". Biseda, jo fjalor: barazvlerësit në gjuhë të tjera thuhen vetëm kur pyeten.
    let (a, phase) = run_phase("diell");
    assert_eq!(phase, Phase::NuraSpeaking);
    assert!(!a.plan.core.iter().any(|c| matches!(c.claim, Claim::NotInSource { .. })), "pa shënim mungese kur ka përgjigje");
    let foreign = a.plan.core.iter().any(|c| match &c.claim {
        Claim::Related { b_lang, .. } => b_lang != "sq",
        Claim::Facet { facet: nura_lingua::expression::Facet::TranslationEn, .. } => true,
        _ => false,
    });
    assert!(!foreign, "pa përkthime në përgjigjen e zakonshme");
    let (b, _) = data();
    let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
    assert!(!t.contains("anglisht") && !t.contains("frëngjisht"), "{t}");
    assert_eq!(say("si quhet diell në anglisht"), "Dielli në anglisht përkthehet «sun»");
}

#[test]
fn conceptnet_isa_is_used_at_form_level_when_wordnet_has_no_path() {
    let (a, phase) = run_phase("a është arra pemë?");
    assert_eq!(phase, Phase::NuraSpeaking);
    let c = &a.plan.core[0];
    assert!(match &c.claim { Claim::IsA { subj, obj } => subj.lemma == "arrë" && obj.lemma == "pemë", _ => false });
    let at = c.attribution.as_ref().unwrap();
    assert_eq!(at.sources, vec![Source::ConceptNet57]);
    assert!(at.note.contains("pa kuptim të caktuar"));
}

#[test]
fn answers_read_as_plain_albanian_sentences() {
    let (b, _) = data();
    for (input, expected) in [
        ("a është qeni kafshë?", "Po, qeni është kafshë"),
        ("zjarr armë", "Zjarri është veprimi i hapjes së zjarrit me armë apo me artileri drejt armikut"),
        ("e kundërta e mirë", "E kundërta e «i mirë» është «i keq»"),
    ] {
        let a = engine().analyze(input).unwrap();
        let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
        assert!(t.starts_with(expected), "{input}: {t}");
        assert!(!t.contains("synset") && !t.contains("(sipas"), "fjalia nuk përmban kode burimi: {t}");
        assert!(!realize(&a.plan, &b.morph, Register::Neutral).sources.is_empty(), "burimi ruhet veçmas");
    }
}

#[test]
fn why_is_explained_by_a_category_chain_with_a_conclusion() {
    let (b, _) = data();
    let a = engine().analyze("pse qeni është kafshë?").unwrap();
    let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
    assert!(t.starts_with("Sepse qeni është "), "{t}");
    assert!(t.contains("çdo ") && t.ends_with("prandaj qeni është kafshë"), "{t}");
}

#[test]
fn compare_finds_common_category_through_babelnet_wordnet_link() {
    let (b, _) = data();
    let a = engine().analyze("krahaso qen me kalë").unwrap();
    let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
    assert!(t.contains("ashtu si edhe"), "{t}");
    assert!(t.contains("Dallimi:"), "{t}");
}

#[test]
fn facet_questions_answer_only_what_was_asked() {
    let (b, _) = data();
    for (input, start) in [
        ("çfarë pjesësh ka makina?", "Makina ka si pjesë"),
        ("sinonim i flamur", "Flamuri quhet edhe «bajrak»"),
        ("llojet e pemë", "Pema ka lloje si"),
        ("llojet e kërcëllimëz", "Për «kërcëllimëz» nuk di lloje"),
    ] {
        let a = engine().analyze(input).unwrap();
        let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
        assert!(t.starts_with(start), "{input}: {t}");
    }
}

#[test]
fn answers_are_deterministic_across_fresh_engines() {
    let (b, lex) = data();
    for input in ["zjarr", "macja", "pse qeni është kafshë?", "krahaso qen me kalë", "e kundërta e bardhë", "shkollë"] {
        let outs: Vec<String> = (0..3)
            .map(|_| {
                let e = Engine::new(&b.graph, &b.morph, lex).with_conceptnet_partial(cn_partial());
                let a = e.analyze(input).unwrap();
                format!("{}|{:?}", realize(&a.plan, &b.morph, Register::Neutral).full, signature(&a.plan))
            })
            .collect();
        assert!(outs.windows(2).all(|w| w[0] == w[1]), "{input} nuk është deterministik");
    }
}

#[test]
fn wikipedia_titles_and_repeated_subjects_are_cleaned() {
    let (b, _) = data();
    let a = engine().analyze("macja").unwrap();
    let t = realize(&a.plan, &b.morph, Register::Neutral).core_text;
    assert!(!t.contains("macja është macja"), "{t}");
    assert!(!t.contains("Quhet edhe «macja»"), "trajta e shquar nuk është sinonim: {t}");
}

// ───────────── rregullat semantike të shqipes dhe kontrollet ndërmjet burimeve ─────────────

fn say(input: &str) -> String {
    let (b, _) = data();
    let a = engine().analyze(input).expect("hyrje e mbështetur");
    realize(&a.plan, &b.morph, Register::Neutral).core_text
}

#[test]
fn albanian_word_formation_rules_explain_derived_words() {
    for (input, expected) in [
        ("i pamundur", "«I pamundur» do të thotë që nuk është «i mundur»"),
        ("prejardhja e mësues", "«Mësues» është ai që kryen veprimin «mësoj»"),
        ("nga vjen fjala zbuloj", "«Zbuloj» është veprimi i kundërt i «mbuloj»"),
        ("nga vjen fjala bukuri", "«Bukuri» është cilësia e të qenit «i bukur»"),
        ("prejardhja e rishkruaj", "«Rishkruaj» do të thotë «shkruaj» përsëri"),
        // Gjuha e listës së huazimeve nuk thuhet pa një dëshmi tjetër.
        ("nga vjen fjala abandonoj", "«Abandonoj» është fjalë e huaj"),
        ("nga vjen fjala xhami", "«Xhami» është huazim nga turqishtja"),
        ("nga vjen fjala vjetër", "«I vjetër» rrjedh nga «vit»"),
    ] {
        let t = say(input);
        assert!(t.starts_with(expected), "{input}: {t}");
    }
}

#[test]
fn adjectives_are_cited_with_their_article_not_declined_as_nouns() {
    for (input, expected) in [
        ("sinonime të bukur", "«I bukur» quhet edhe"),
        ("e kundërta e i pamundur", "E kundërta e «i pamundur» është «i mundur»"),
    ] {
        let t = say(input);
        assert!(t.starts_with(expected), "{input}: {t}");
        assert!(!t.contains("Bukuri quhet"), "mbiemri nuk shquhet si emër: {t}");
    }
}

#[test]
fn english_only_senses_are_a_translation_bridge_not_a_question() {
    let (a, phase) = run_phase("qyqar");
    assert_eq!(phase, Phase::NuraSpeaking, "pa pyetje me opsione anglisht");
    assert!(a.plan.question.is_none());
    let t = say("qyqar");
    // Përkthimi ndjek kuptimin e përkufizuar (leksiku i modelit: «wretched»; Wiktionary: «poor»).
    assert!(!t.contains("anglisht"), "përkthimi thuhet vetëm kur pyetet: {t}");
    let en = say("si quhet qyqar në anglisht");
    assert!(en.contains("«poor»") || en.contains("«wretched»"), "{en}");
    assert_eq!(say("si quhet mësues në anglisht"), "Mësuesi në anglisht përkthehet «teacher»");
}

#[test]
fn a_general_sense_subsumed_by_a_narrower_one_needs_no_question() {
    // «kalë» (Equus caballus) është lloj i kuptimit tjetër «kalë» (kalitë): R-E7.
    let (a, phase) = run_phase("ç'kuptim ka kalë");
    assert_eq!(phase, Phase::NuraSpeaking);
    assert!(a.steps.iter().any(|s| s.code == "ELIM"));
    assert!(a.plan.question.is_none(), "pa pyetje për kuptime");
    let t = say("ç'kuptim ka kalë");
    assert!(t.starts_with("Kali është"), "trajta e shquar e «kalë» është «kali»: {t}");
    assert!(say("kalë").starts_with("Kali "), "{}", say("kalë"));
}

#[test]
fn spoken_facts_survive_cross_source_checks() {
    for (input, banned) in [
        ("nënë", "lloj familje"),     // kategori që WordNet-i (përmes «mother») nuk e dëshmon
        ("kalë", "balash"),           // hiponime të futura si hiperonime në burim
        ("lule", "lulëzoj"),          // folje në sinset emëror
        ("ujë", "liquid"),            // anglisht e papërkthyer si etiketë shqipe
        ("shkollë", "«etj»"),         // shkurtim si sinonim
        ("qen", "canis familiaris"),  // parafrazë/emër shkencor si sinonim
        ("diell", "«ardhje»"),        // sinonim i një kuptimi dytësor (Wiktionary DE, kuptimi 3)
        ("i bukur", "rrjedh nga «bukuri»"), // drejtim i kundërt i DerivedFrom
    ] {
        let t = say(input);
        assert!(!t.contains(banned), "{input}: {t}");
    }
}

#[test]
fn questions_with_an_adjective_article_keep_the_adjective() {
    for (input, expected) in [
        ("nga vjen fjala i pamundur", "«I pamundur» do të thotë që nuk është «i mundur»"),
        ("e kundërta e fjalës i bukur", "E kundërta e «i bukur» është"),
        ("si quhet i madh në anglisht", "«I madh» në anglisht përkthehet «big»"),
        ("i zi", "E kundërta e «i zi» është «i bardhë»"),
    ] {
        let t = say(input);
        assert!(t.starts_with(expected), "{input}: {t}");
        assert!(!t.starts_with("«I» "), "nyja nuk është fjalë: {t}");
    }
}

#[test]
fn verbs_and_rare_homographs_are_not_declined_as_nouns() {
    let t = say("pi");
    assert!(t.starts_with("«Pi» do të thotë: "), "{t}");
    let t = say("derë");
    assert!(t.starts_with("Dera ") && !t.contains("«door»") && !t.contains("«i derë»"), "{t}");
    // Fjala që e njeh gramatika nuk është «e panjohur»: thuhet të paktën pjesa e ligjëratës.
    let t = say("elegant");
    assert!(t.starts_with("«Elegant» ") && !t.contains("Nuk e njoh"), "{t}");
}

#[test]
fn translations_follow_the_word_not_a_broader_synset() {
    assert_eq!(say("si quhet qen në anglisht"), "Qeni në anglisht përkthehet «dog»");
    assert!(!say("lumë").contains("«river»"), "pa anglisht në përgjigjen e zakonshme");
    assert!(say("si quhet lumë në anglisht").contains("«river»"));
    assert!(say("ç'kuptim ka mace").starts_with("Macja është kafshë shtëpiake. "), "glosa enciklopedike vetëm deri te presja");
}

#[test]
fn rare_northern_words_are_explained_from_gazulli_1941() {
    // Fjalor i 1941 (domen publik), OCR i filtruar: shpjegimi thuhet si kuptim krahinor i rrallë.
    for (input, expected) in [
        ("kakërdhoçë", "Në të folmet e Veriut (fjalë e rrallë, 1941), «kakërdhoçë» do të thotë: nji farë hardhuce qi jeton ndër male"),
        ("lëqëth", "Në të folmet e Veriut (fjalë e rrallë, 1941), «lëqëth» do të thotë: liqë i vogël"),
        ("mugëz", "Në të folmet e Veriut (fjalë e rrallë, 1941), «mugëz» do të thotë: muzg"),
    ] {
        let t = say(input);
        assert!(t.starts_with(expected), "{input}: {t}");
    }
    // Kuptimi krahinor i rrallë nuk zë vendin e kuptimit të zakonshëm, as të një homografi.
    for input in ["lumë", "frutë", "qen"] {
        let t = say(input);
        assert!(!t.contains("Veriut"), "{input}: {t}");
    }
    // Kur fjala ka kuptim të zakonshëm, kuptimi krahinor thuhet pas tij, si shtesë.
    assert!(!say("pishinë").contains("Veriut"), "biseda nuk e nis me kuptimin e rrallë krahinor");
    let t = say("ç'kuptim ka pishinë");
    let common = t.find("Pishina është").expect(&t);
    let regional = t.find("thuhej edhe për").expect(&t);
    assert!(common < regional, "{t}");
    // Prejardhja ndjek kuptimin kryesor: «pishinë» (pishinë noti) është huazim nga italishtja, jo
    // «pishë» (ajo vlen për homografin «pyll pishash»); «mësues» mbetet i prejardhur nga «mësoj».
    let o = say("nga vjen fjala pishinë");
    assert!(o.starts_with("«Pishinë» është huazim nga italishtja. "), "{o}");
    // Shpjegimi popullor i pronarit thuhet pas prejardhjes së dokumentuar dhe si i tillë.
    assert!(o.contains("Sipas një shpjegimi popullor, «pishinë» "), "{o}");
    assert!(say("nga vjen fjala mësues").contains("«mësoj»"));
    // Hyrjet e Gazullit nuk i bëjnë trajtat e lakuara fjalë më vete: «mali» → «mal», «guri» → «gur».
    for (form, lemma) in [("mali", "mal"), ("guri", "gur")] {
        let t = say(form);
        assert!(!t.contains("Veriut") && !t.contains("Nuk e njoh"), "{form}: {t}");
        assert!(t.to_lowercase().contains(lemma), "{form}: {t}");
    }
}

#[test]
fn model_lexicon_defines_words_but_alone_never_links_them() {
    // Përkufizimi i leksikut të modelit për fjalën pa përkufizim shqip; faktet e tjera ruhen.
    let t = say("diell");
    assert!(t.starts_with("Dielli është yll që") && !t.contains("«soleil»"), "{t}");
    // Përkthimi ndjek kuptimin e përkufizuar kur e dëshmon edhe Wiktionary-t («ankth» = anxiety).
    assert!(say("si quhet ankth në anglisht").contains("«anxiety»"));
    // Kategoria e veçantë nuk thuhet pas përkufizimit («arrë»: kokërr, jo «Është pemë»).
    assert!(!say("arrë").contains("Është pemë"));
    // Gjinia femërore e rregullave vetëm për qenie të gjalla («abetare» është libër).
    assert!(!say("abetare").contains("femërore"));
    // Lidhja që e jep vetëm modeli nuk thuhet pa një dëshmitar tjetër.
    assert!(!say("e kundërta e armik").contains("«mik»"));
    assert_eq!(say("sinonim i anketë"), "Për «anketë» nuk di sinonim");
}


#[test]
fn nura_composes_sentences_from_links_and_agreements() {
    // Lidhja «lloj i» nga grafi + përputhja (kryefjala e shquar, kallëzuesi i pashquar).
    let t = say("fjali qen kafshë");
    assert!(t.starts_with("Fjalia: «Qeni është kafshë.» Lidhja: qen → "), "{t}");
    assert!(t.contains("Përkundër: «Qeni nuk është bimë.»"), "{t}");
    // Drejtimi i lidhjes vendos rendin, jo rendi i fjalëve.
    assert!(say("fjali kafshë qen").starts_with("Fjalia: «Qeni është kafshë.»"));
    // Mbiemri merr nyjën dhe gjininë e emrit; nyja e shtypur mbetet me fjalën.
    assert_eq!(say("fjali mace vogël"), "Fjalia: «Macja është e vogël.»");
    assert_eq!(say("fjali i madh qen"), "Fjalia: «Qeni është i madh.»");
    // Veprimi: kryefjalë e gjallë, veta e tretë, kundrinori brenda klasës së foljes.
    assert!(say("fjali qen ha mish").starts_with("Fjalia: «Qeni ha mish.»"));
    assert!(say("fjali fëmijë lexoj libër").starts_with("Fjalia: «Fëmija lexon libër.»"));
    assert!(say("fjali qen pi ujë").starts_with("Fjalia: «Qeni pi ujë.»"));
    assert!(say("fjali njeri punoj").starts_with("Fjalia: «Njeriu punon.»"));
    // Jashtë klasës së kundrinorit ose pa kryefjalë të gjallë: nuk shpiket fjali.
    assert!(say("fjali qen ha gur").starts_with("Nuk ndërtoj dot fjali"), "{}", say("fjali qen ha gur"));
    assert!(say("fjali gur pi ujë").starts_with("Nuk ndërtoj dot fjali"));
    // Mungesa e një rruge në graf nuk është provë mohimi: «mollë» ⊂ «frut» del nga ura angleze.
    let m = say("fjali mollë frut");
    assert!(m.starts_with("Fjalia: «Molla është frut.»") || m.starts_with("Nuk ndërtoj dot fjali"), "{m}");
    assert!(!m.contains("nuk është"), "{m}");
}

#[test]
fn third_person_rules_match_attested_ud_forms() {
    use nura_lingua::morph::feat;
    use nura_lingua::verb::rule_3sg;
    let (b, _) = data();
    // Standardi i artë: trajtat e atestuara në UD, pa zhurmën e etiketimit (pësore pa shenjë,
    // veta e parë e shënuar si e tretë).
    let gold: Vec<(String, String)> = b
        .morph
        .entries
        .iter()
        .filter(|e| {
            e.upos == "VERB"
                && feat(&e.feats, "Person") == Some("3")
                && feat(&e.feats, "Number") == Some("Sing")
                && feat(&e.feats, "Tense") == Some("Pres")
                && feat(&e.feats, "Mood") == Some("Ind")
                && feat(&e.feats, "Voice") != Some("Pass")
        })
        .map(|e| (e.lemma.to_lowercase(), e.form.to_lowercase()))
        .filter(|(l, f)| !(f.ends_with("het") && !l.ends_with("em")) && !(f.ends_with("et") && l.ends_with("oj")) && !(f == l && l.ends_with("oj")))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let (mut ok, mut bad) = (0, 0);
    let mut wrong: Vec<String> = Vec::new();
    for (lemma, form) in &gold {
        match rule_3sg(&b.graph, lemma) {
            Some(v) => match v.text == *form {
                true => ok += 1,
                false => {
                    bad += 1;
                    wrong.push(format!("{lemma}: {form} ≠ {}", v.text));
                }
            },
            None => {}
        }
    }
    assert!(ok >= 25, "shumë pak forma të krahasuara: {ok}");
    assert!(ok * 100 >= (ok + bad) * 80, "saktësia e rregullave {ok}/{} — {wrong:?}", ok + bad);
}

#[test]
fn commonsense_layer_links_albanian_words_through_declared_bridges() {
    let (b, lex) = data();
    let g = &b.graph;
    let imported = read_lock(&default_data_dir())
        .expect("SOURCES.lock")
        .iter()
        .any(|r| r.file == nura_lingua::src_commonsense::TSV);
    let cs: Vec<&nura_lingua::graph::Edge> = g
        .edges
        .iter()
        .filter(|e| {
            e.source == Source::ConceptNet57
                && match &e.rel {
                    Rel::Sem(r) => r.starts_with(nura_lingua::src_commonsense::PREFIX),
                    _ => false,
                }
        })
        .collect();
    match imported {
        false => {
            assert!(cs.is_empty(), "pa skedarin ConceptNet anglisht s'ka shtresë CS_");
            return;
        }
        true => {}
    }
    let lemma = |id: u32| match &g.node(id).kind {
        NodeKind::Word { lang: Lang::Sq, lemma } => Some(lemma.clone()),
        _ => None,
    };
    let main = |structured: &str| structured.split(';').next().and_then(|p| p.split_once('=')).map(|(_, v)| v.to_string());
    let mut by_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut by_rel: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for e in &cs {
        let map = e.attr("map").unwrap_or("");
        let weak = matches!(map, "A1" | "B1");
        // Skaji lidh dy fjalë shqipe: fjalën kryesore të kokës dhe të bishtit.
        assert_eq!(lemma(e.from), e.attr("head").and_then(main), "{:?}", e.attrs);
        assert_eq!(lemma(e.to), e.attr("tail").and_then(main), "{:?}", e.attrs);
        assert_eq!(e.lang, Some(Lang::Sq));
        assert!(matches!(map, "AB" | "AB2" | "A1" | "B1"), "{:?}", e.attrs);
        // I dobëti (një urë) shënohet; dëshmitari i modelit vetëm te i dobëti.
        assert_eq!(e.attr("weak") == Some("1"), weak, "{:?}", e.attrs);
        assert!(e.attr("witness").is_none() || weak, "{:?}", e.attrs);
        let cid = e.attr("cid").unwrap_or("");
        assert!(cid.len() == 12 && cid.chars().all(|c| c.is_ascii_hexdigit()), "{cid}");
        // WordNet-i, OpenCyc dhe DBpedia nuk hyjnë.
        for ds in e.attr("ds").unwrap_or("").split(',') {
            assert!(ds == "/d/conceptnet/4/en" || ds == "/d/verbosity" || ds.starts_with("/d/wiktionary/"), "{ds}");
        }
        assert!(e.attr("en").map(|x| x.contains('|')).unwrap_or(false));
        assert!(e.attr("rows").and_then(|x| x.parse::<usize>().ok()).unwrap_or(0) >= 1);
        *by_map.entry(format!("{map}{}", e.attr("witness").map(|_| "+M").unwrap_or(""))).or_default() += 1;
        *by_rel.entry(match &e.rel {
            Rel::Sem(r) => r.clone(),
            _ => String::new(),
        })
        .or_default() += 1;
    }
    println!("[dija e përbashkët] {} skaje CS_; sipas hartimit {by_map:?}; sipas marrëdhënies {by_rel:?}", cs.len());
    let two = by_map.get("AB").copied().unwrap_or(0) + by_map.get("AB2").copied().unwrap_or(0);
    assert!(cs.len() >= 5000, "shumë pak skaje CS_: {}", cs.len());
    assert!(two >= 3000, "shumë pak skaje me dy ura: {two}");
    let has = |from: &str, rel: &str, to: &str| match (lex.get(from), lex.get(to)) {
        (Some(f), Some(t)) => {
            g.out_edges(*f).any(|(_, e)| e.to == *t && e.rel == Rel::Sem(rel.into()) && e.source == Source::ConceptNet57)
        }
        _ => false,
    };
    // Fakte të zakonshme: qeni leh dhe gjendet diku; thika është në kuzhinë; libri është prej letre.
    assert!(has("qen", "CS_CAPABLE_OF", "leh"));
    assert!(g.out_edges(lex["qen"]).any(|(_, e)| e.rel == Rel::Sem("CS_AT_LOCATION".into())));
    assert!(has("thikë", "CS_AT_LOCATION", "kuzhinë"));
    assert!(has("libër", "CS_MADE_OF", "letër"));
    // Kalket dhe kuptimet e dytësore mbeten jashtë (HOLD): folja e lehtë me emër abstrakt («have fun»),
    // folja si mbiemër («light fire»), pjesorja si folje veprore («punished»), emri në krye të një
    // veprimtarie («smoke» → «tym»), kuptimi i dytësor («letter» → «shkronjë»).
    let activity = ["CS_HAS_PREREQUISITE", "CS_HAS_SUBEVENT", "CS_HAS_FIRST_SUBEVENT", "CS_HAS_LAST_SUBEVENT", "CS_MOTIVATED_BY"];
    for e in &cs {
        let (head, tail, en) = (e.attr("head").unwrap_or(""), e.attr("tail").unwrap_or(""), e.attr("en").unwrap_or(""));
        let rel = match &e.rel {
            Rel::Sem(r) => r.as_str(),
            _ => "",
        };
        assert!(!head.starts_with("v=kam;o=argëtim") && !tail.starts_with("v=kam;o=argëtim"), "{:?}", e.attrs);
        assert_ne!(tail, "n=zjarr;a=çelur", "{:?}", e.attrs);
        assert!(!(en.ends_with("|punished") && tail == "v=ndëshkoj"), "{:?}", e.attrs);
        assert!(!(activity.contains(&rel) && head == "n=tym"), "{:?}", e.attrs);
        assert!(!(en.starts_with("letter") && head == "n=shkronjë"), "{:?}", e.attrs);
    }
}
