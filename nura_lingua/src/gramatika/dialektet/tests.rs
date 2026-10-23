//! Shembujt e rregullave. Fjalët që janë edhe në arin e matjes merren vetëm nga ndarja `Dev`.

use super::*;

/// Leksik i vogël me rreshta në trajtën e ekstraktit TSV (18 shtylla): lema, pjesa e ligjëratës,
/// statusi, klasat e paradigmës, temat.
fn lexicon(rows: &[(&str, &str, &str, &str, &str)]) -> Lexicon {
    let text: String = rows
        .iter()
        .map(|(lemma, upos, status, par, stems)| format!("{lemma}\t{lemma}\t{upos}\t-\t-\t-\t-\t-\t-\t-\t{status}\t{par}\t{stems}\t-\t-\t-\t-\t-\n"))
        .collect();
    Lexicon::parse(&text)
}

/// Lema standarde pa klasë paradigme.
fn plain(rows: &[(&str, &str)]) -> Lexicon {
    let full: Vec<(&str, &str, &str, &str, &str)> = rows.iter().map(|(l, p)| (*l, *p, "std", "-", "-")).collect();
    lexicon(&full)
}

fn first(word: &str, lex: &Lexicon) -> Option<String> {
    to_standard(word, lex).into_iter().next().map(|c| c.text)
}

fn best(word: &str, lex: &Lexicon) -> Option<(String, Vec<&'static str>)> {
    scored(word, lex).into_iter().next().map(|(_, c)| (c.text, c.rules))
}

#[test]
fn rhotacism_restores_r() {
    let lex = plain(&[("gjuri", "NOUN"), ("trimëri", "NOUN"), ("zëri", "NOUN")]);
    assert_eq!(first("gjuni", &lex).as_deref(), Some("gjuri"));
    let c = to_standard("trimni", &lex);
    assert_eq!(c.first().map(|c| (c.text.as_str(), c.rules.clone())), Some(("trimëri", vec![ROTACISM_SCHWA])));
    assert!(to_standard("zâni", &lex).is_empty(), "dy rregulla (hundorja dhe rotacizmi): dëshmi, jo përgjigje");
    assert_eq!(best("zâni", &lex).map(|b| b.0).as_deref(), Some("zëri"));
}

#[test]
fn single_reflex_nasals_become_oral() {
    let lex = plain(&[("dhen", "NOUN"), ("bli", "NOUN"), ("dru", "NOUN"), ("fryj", "V")]);
    assert_eq!(first("dhên", &lex).as_deref(), Some("dhen"));
    assert_eq!(first("blî", &lex).as_deref(), Some("bli"));
    assert_eq!(first("drû", &lex).as_deref(), Some("dru"));
    assert_eq!(first("frŷj", &lex).as_deref(), Some("fryj"));
}

#[test]
fn low_nasals_and_restorations_are_evidence_not_answers() {
    let lex = plain(&[("gjë", "NOUN"), ("dhëmb", "NOUN"), ("anë", "NOUN"), ("banjë", "NOUN"), ("kënaq", "V"), ("mendoj", "V"), ("varr", "NOUN"), ("mbledhje", "NOUN")]);
    assert_eq!(best("gjâ", &lex), Some(("gjë".to_string(), vec![NASAL_LOW])));
    assert_eq!(best("dhâmb", &lex).map(|b| b.0).as_deref(), Some("dhëmb"));
    assert_eq!(best("ânë", &lex).map(|b| b.0).as_deref(), Some("anë"));
    assert_eq!(best("bâjë", &lex).map(|b| b.0).as_deref(), Some("banjë"));
    assert_eq!(best("knaq", &lex), Some(("kënaq".to_string(), vec![SCHWA])));
    assert_eq!(best("mênoj", &lex).map(|b| b.0).as_deref(), Some("mendoj"));
    assert_eq!(best("vorr", &lex), Some(("varr".to_string(), vec![O_A])));
    assert_eq!(best("mledhje", &lex), Some(("mbledhje".to_string(), vec![CLUSTER_MB])));
    assert!(scored("gjâ", &lex).iter().all(|(_, c)| !is_firm(c)));
    for w in ["gjâ", "dhâmb", "ânë", "bâjë", "knaq", "mênoj", "vorr", "mledhje"] {
        assert!(to_standard(w, &lex).is_empty(), "{w}: vetëm rregulla të dobëta, pra Hold");
    }
}

#[test]
fn diphthong() {
    let lex = plain(&[("grua", "NOUN"), ("kruaj", "V"), ("shuaj", "V")]);
    assert_eq!(first("grue", &lex).as_deref(), Some("grua"));
    assert_eq!(first("kruej", &lex).as_deref(), Some("kruaj"));
}

#[test]
fn participles_need_a_verb_of_the_lexicon() {
    let rows = [
        ("punuar", "A", "std", "adj-2-2", "punuar.|punuar."),
        ("punoj", "V", "std", "lexoj", "puno.|punua.|punu.|pun."),
        ("pasur", "A", "std", "adj-2-2", "pasur.|pasur."),
        ("kam", "V", "std", "kam", "ka.|ke.|pas.|kish.|pat.|ki.|pa."),
        ("urdhëruar", "A", "std", "adj-2-2", "urdhëruar.|urdhëruar."),
    ];
    let lex = lexicon(&rows);
    assert_eq!(first("punue", &lex).as_deref(), Some("punuar"));
    assert_eq!(first("punuem", &lex).as_deref(), Some("punuar"));
    assert_eq!(first("pasun", &lex).as_deref(), Some("pasur"));
    assert!(to_standard("urdhëruem", &lex).is_empty(), "pa foljen «urdhëroj» në leksik pjesorja nuk dëshmohet");
    assert_eq!(best("urdhëruem", &lex).map(|b| b.0).as_deref(), Some("urdhëruar"));
}

#[test]
fn standard_forms_are_held() {
    let rows = [
        ("plakur", "A", "std", "adj-2-2", "plakur.|plakur."),
        ("plak", "V", "std", "beh", "plak."),
        ("kthyer", "A", "std", "adj-2-2", "kthyer.|kthyer."),
        ("kthej", "V", "std", "kthej", "kthe.|kthye."),
        ("pa", "PR", "std", "unchangeable", "pa."),
        ("parë", "A", "std", "adj-2-1", "parë.|par."),
    ];
    let lex = lexicon(&rows);
    assert_eq!(first("plakun", &lex).as_deref(), Some("plakur"));
    let mut with_adjective = rows.to_vec();
    with_adjective.push(("plak", "A", "std", "plak", "plak.|plakë.|pleq.|plaka.|plak."));
    let lex2 = lexicon(&with_adjective);
    assert!(to_standard("plakun", &lex2).is_empty(), "«plakun» është edhe kallëzore e shquar e «plak»");
    assert!(is_standard_form("plakun", &lex2));
    assert!(to_standard("kthyem", &lex).is_empty(), "«kthyem» është kryera e thjeshtë e «kthej»");
    assert!(to_standard("kthye", &lex).is_empty(), "«u kthye» është trajtë standarde");
    assert!(to_standard("pa", &lex).is_empty(), "«pa» është lemë standarde");
    assert!(!is_standard_form("punue", &lex));
}

#[test]
fn near_equal_paths_to_two_lemmas_are_a_hold() {
    let lex = plain(&[("arena", "NOUN"), ("anera", "NOUN"), ("tharë", "A"), ("thënë", "A"), ("ne", "PRON"), ("në", "ADP"), ("kalaman", "NOUN"), ("kalama", "NOUN")]);
    assert!(to_standard("anena", &lex).is_empty(), "rotacizmi në dy vende jep dy lema");
    assert_eq!(scored("anena", &lex).len(), 2);
    assert!(to_standard("thanë", &lex).is_empty(), "rotacizmi (tharë) dhe «a» para hundores (thënë) janë pothuajse të barabarta");
    assert!(to_standard("nê", &lex).is_empty(), "hundorja fundore: ne apo në");
    assert!(scored("kalamâ", &lex).iter().any(|(_, c)| c.text == "kalaman" && c.rules == vec![NASAL_FINAL]));
}

#[test]
fn weak_spelling_rules_only_witness() {
    let lex = plain(&[("tym", "NOUN"), ("filozof", "NOUN")]);
    assert!(to_standard("tim", &lex).is_empty(), "i/y është leksikore: «tim» standard nuk bëhet «tym»");
    assert_eq!(best("tim", &lex).map(|b| b.1), Some(vec![UNROUNDING]));
    assert!(to_standard("filosof", &lex).is_empty());
    assert_eq!(best("filosof", &lex).map(|b| b.1), Some(vec![SPELLING_Z]));
}

#[test]
fn proper_names_need_a_proper_lemma() {
    let lex = plain(&[("prerë", "A"), ("Tirana", "PROPN"), ("tirana", "NOUN")]);
    assert!(scored("Prenë", &lex).is_empty(), "emri i përveçëm nuk bëhet fjalë e zakonshme");
    assert!(to_standard("Prenë", &lex).is_empty());
    assert!(scored("Tirona", &lex).iter().all(|(_, c)| c.text == "Tirana"));
}

#[test]
fn consonant_correspondences() {
    let lex = plain(&[("njoh", "V"), ("zgjedhje", "NOUN"), ("vij", "V"), ("asnjë", "PRON")]);
    assert_eq!(first("njof", &lex).as_deref(), Some("njoh"));
    assert_eq!(first("zgedhje", &lex).as_deref(), Some("zgjedhje"));
    assert_eq!(best("vinj", &lex), Some(("vij".to_string(), vec![VERB_NJ])));
    assert_eq!(best("asnji", &lex), Some(("asnjë".to_string(), vec![NJI])));
}

#[test]
fn tosk_schwa_only_in_a_closed_syllable() {
    let lex = plain(&[("mendoj", "V"), ("armen", "NOUN"), ("eter", "NOUN")]);
    let c = to_standard("mëndoj", &lex);
    assert_eq!(c.first().map(|c| (c.text.as_str(), c.rules.clone())), Some(("mendoj", vec![TOSK_SCHWA])));
    assert!(scored("armën", &lex).is_empty(), "«-ën» është mbaresë, jo rrokje e mbyllur");
    assert!(scored("etën", &lex).is_empty(), "ë toske → e dhe rotacizmi gegë s'bashkohen");
}

#[test]
fn irregular_table_needs_its_witness() {
    let lex = plain(&[("jam", "V"), ("duke", "VB_PART")]);
    let c = to_standard("âsht", &lex);
    assert_eq!(c.first().map(|c| (c.text.as_str(), c.rules.clone())), Some(("është", vec![TABLE])));
    assert_eq!(first("tue", &lex).as_deref(), Some("duke"));
    assert!(to_standard("hangër", &lex).is_empty(), "pa lemën «ngrënë» në leksik: asnjë trajtë");
}

#[test]
fn hold_when_nothing_is_attested() {
    let lex = lexicon(&[("dhëmb", "NOUN", "std", "-", "-"), ("dham", "NOUN", "nonst", "-", "-")]);
    assert!(to_standard("xyzq", &lex).is_empty());
    assert!(to_standard("dhëmb", &lex).is_empty(), "fjala standarde nuk kthehet si kandidate e vetvetes");
    assert!(to_standard("", &lex).is_empty());
    assert!(to_standard(&"a".repeat(500), &lex).is_empty());
}

#[test]
fn declared_variant_uses_the_lexref() {
    let text = "x\tdham\tNOUN\t-\t-\t-\t-\t-\t-\tnonst,Gheg\tgheg\t-\t-\tdhëmb\t-\t-\t-\t-\nx\tdhëmb\tNOUN\t-\t-\t-\t-\t-\t-\t-\tstd\t-\t-\t-\t-\t-\t-\t-\n";
    let lex = Lexicon::parse(text);
    assert_eq!(declared("dham", &lex).map(|c| c.text), Some("dhëmb".to_string()));
    assert_eq!(declared("dhëmb", &lex), None);
}

#[test]
fn gheg_generation() {
    let has = |w: &str, g: &str| to_gheg(w).iter().any(|x| x == g);
    assert!(has("grua", "grue"));
    assert!(has("grua", "gru"));
    assert!(has("punuar", "punue"));
    assert!(has("pasur", "pasun"));
    assert!(has("pasur", "pasë"));
    assert!(has("dhëmb", "dhâmb"));
    assert!(has("gjuri", "gjuni"));
    assert!(has("kënaq", "knaq"));
    assert!(has("gjë", "gjâ"));
    assert!(has("bli", "blî"));
    assert!(has("njoh", "njof"));
    assert!(has("trimëri", "trimni"));
    assert!(has("banjë", "bâjë"));
    assert!(has("varr", "vorr"));
    assert!(has("shpresë", "shpnesë"));
    assert!(has("është", "âsht"));
    assert!(has("ngrënë", "hangër"));
    assert!(to_gheg("").is_empty());
    assert!(to_gheg(&"a".repeat(500)).is_empty());
}

#[test]
fn gheg_generation_keeps_the_stressed_vowel() {
    for w in ["bëj", "nënë", "këmbë", "zë", "dhëmb"] {
        let g = to_gheg(w);
        assert!(g.iter().all(|x| letters(x).iter().any(|t| shkronjat::is_vowel(t))), "{w}: {g:?}");
        assert!(!g.iter().any(|x| x == "bj" || x == "nnë" || x == "kmbë"), "{w}: {g:?}");
    }
    let mire = to_gheg("mirë");
    assert!(mire.first().map(|x| x != "minë").unwrap_or(false), "rotacizmi renditet i fundit: {mire:?}");
}

#[test]
fn gheg_infinitive_both_ways() {
    let rows = [
        ("punuar", "A", "std", "adj-2-2", "punuar.|punuar."),
        ("punoj", "V", "std", "lexoj", "puno.|punua.|punu.|pun."),
        ("kthyer", "A", "std", "adj-2-2", "kthyer.|kthyer."),
        ("kthej", "V", "std", "kthej", "kthe.|kthye."),
        ("dalë", "A", "std", "adj-2-1", "dalë.|dal."),
        ("ngrënë", "A", "std", "adj-2-1", "ngrënë.|ngrën."),
        ("pasur", "A", "std", "adj-2-2", "pasur.|pasur."),
        ("kam", "V", "std", "kam", "ka.|ke.|pas.|kish.|pat.|ki.|pa."),
    ];
    let lex = lexicon(&rows);
    let c = infinitive_to_standard("me punue", &lex);
    assert_eq!(c.as_ref().map(|c| c.text.as_str()), Some("për të punuar"));
    assert!(c.map(|c| c.rules.contains(&INFINITIVE) && c.rules.contains(&PART_UE)).unwrap_or(false));
    assert_eq!(infinitive_to_standard("me dalë", &lex).map(|c| c.text), Some("për të dalë".to_string()));
    assert_eq!(infinitive_to_standard("me hangër", &lex).map(|c| c.text), Some("për të ngrënë".to_string()));
    assert_eq!(infinitive_to_standard("me u kthye", &lex).map(|c| c.text), Some("për t'u kthyer".to_string()));
    assert_eq!(infinitive_to_standard("me pasë", &lex).map(|c| c.text), Some("për të pasur".to_string()));
    assert_eq!(infinitive_to_standard("me shokë", &lex), None);
    assert_eq!(infinitive_to_standard("me", &lex), None);
    assert!(infinitive_to_gheg("për të punuar").contains(&"me punue".to_string()));
    assert!(infinitive_to_gheg("për t'u larë").contains(&"me u larë".to_string()));
    assert!(infinitive_to_gheg("për të pasur").contains(&"me pasun".to_string()));
    assert!(infinitive_to_gheg("për të pasur").contains(&"me pasë".to_string()));
    assert!(infinitive_to_gheg("për të ngrënë").contains(&"me hangër".to_string()));
    assert!(infinitive_to_gheg("për t\u{2019}u kthyer").contains(&"me u kthye".to_string()));
    assert!(infinitive_to_gheg("me punue").is_empty());
}
