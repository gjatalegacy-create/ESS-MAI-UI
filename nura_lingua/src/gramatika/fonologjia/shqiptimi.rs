//! Shqiptimi: shkronjë → fonemë (IPA), sipas vlerave standarde të alfabetit
//! (c /t͡s/, ç /t͡ʃ/, dh /ð/, e /ɛ/, ë /ə/, gj /ɟ/, ll /ɫ/, nj /ɲ/, o /ɔ/, q /c/,
//! r /ɾ/, rr /r/, sh /ʃ/, th /θ/, x /d͡z/, xh /d͡ʒ/, zh /ʒ/).
//!
//! Transkriptimi është fonologjik: një shkronjë, një fonemë. Nuk shënohen
//! shurdhimi në fund të fjalës, asimilimi i hundores (n + k/g) as rënia e «ë»-së.
//! «ë»-ja e patheksuar në fund të fjalës ruhet si /ə/, sepse drejtshkrimi e shënon
//! dhe ligjërimi i kujdesshëm e shqipton; rënia e saj (/ˈpat/ për «patë») është
//! dukuri e ligjërimit të shpejtë dhe e dialekteve.

use super::alfabeti::{canonical, ipa_of, letters};

/// Transkriptimi fonologjik i një fjale; None kur fjala ka shenja jashtë alfabetit.
pub fn to_ipa(word: &str) -> Option<String> {
    let units = letters(word);
    (!units.is_empty()).then_some(())?;
    units.iter().map(|u| canonical(u).and_then(ipa_of)).collect::<Option<Vec<&str>>>().map(|v| v.concat())
}

/// Largesa e Levenshtein-it (Wagner–Fischer 1974): numri më i vogël i futjeve, fshirjeve dhe zëvendësimeve.
pub fn levenshtein<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut diag = i;
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = (diag + usize::from(x != y)).min(row[j] + 1).min(up + 1);
            diag = up;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_correspondences() {
        assert_eq!(to_ipa("shqip").as_deref(), Some("ʃcip"));
        assert_eq!(to_ipa("dhjetë").as_deref(), Some("ðjɛtə"));
        assert_eq!(to_ipa("rrallë").as_deref(), Some("raɫə"));
        assert_eq!(to_ipa("gjuhë").as_deref(), Some("ɟuhə"));
        assert_eq!(to_ipa("njeri").as_deref(), Some("ɲɛɾi"));
        assert_eq!(to_ipa("xhaxha").as_deref(), Some("d͡ʒad͡ʒa"));
        assert_eq!(to_ipa("Zog").as_deref(), Some("zɔɡ"));
        assert_eq!(to_ipa("cicërimë").as_deref(), Some("t͡sit͡səɾimə"));
        assert_eq!(to_ipa("çelës").as_deref(), Some("t͡ʃɛləs"));
        assert_eq!(to_ipa("thjeshtë").as_deref(), Some("θjɛʃtə"));
        assert_eq!(to_ipa("zhurmë").as_deref(), Some("ʒuɾmə"));
        assert_eq!(to_ipa("xixë").as_deref(), Some("d͡zid͡zə"));
        assert_eq!(to_ipa("dy"), Some("dy".to_string()));
    }

    #[test]
    fn foreign_signs_are_held() {
        assert_eq!(to_ipa("watt"), None);
        assert_eq!(to_ipa("e hënë"), None);
        assert_eq!(to_ipa(""), None);
    }

    #[test]
    fn edit_distance() {
        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        assert_eq!(levenshtein(&a, &b), 3);
        assert_eq!(levenshtein::<char>(&[], &b), 7);
        assert_eq!(levenshtein(&a, &a), 0);
    }
}
