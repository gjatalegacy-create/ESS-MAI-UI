//! Fonologjia dhe drejtshkrimi i shqipes standarde: alfabeti, shqiptimi (IPA),
//! rrokjezimi, theksi dhe rikthimi i diakritikëve.
//!
//! Dëshmitari leksikor është leksiku i uniparser-grammar-albanian 2.3.0 (MIT —
//! Morozova, Rusakov, Arkhangelskiy): klasat e paradigmës dhe temat e tij japin theksin;
//! lemat e tij dhe trajtat e paradigmave të tij (`klasat`) zgjidhin «e/ë» dhe «c/ç». Kur një
//! rregull nuk është i sigurt, përgjigjja është None.

pub mod alfabeti;
pub mod diakritikat;
mod klasat;
pub mod rrokjet;
pub mod shqiptimi;
pub mod theksi;

pub use alfabeti::{canonical, ipa_of, is_consonant, is_obstruent, is_vowel, letters, manner, sonority, sound, Manner, Sound, ALPHABET, DIGRAPHS};
pub use diakritikat::{candidates, restoration, restore, Restorer, FUNCTION_FORMS, MAX_POSITIONS};
pub use rrokjet::{legal_onset, syllable_count, syllables, Pair, DIPHTHONGS};
pub use shqiptimi::{levenshtein, to_ipa};
pub use theksi::{stress, stressed_syllable, Hint, LexemeStress, Stress, STRESSED_SCHWA};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gramatika::leksiku::Lexicon;
    use crate::gramatika::tipet::Pos;

    #[test]
    fn no_input_makes_a_rule_panic() {
        let lex = Lexicon::parse("");
        let odd = ["", " ", "-", "ë", "\u{308}", "e\u{308}\u{308}", "x\u{301}h", "\u{1f600}", "ççëëççëë", "ngjngjngj", "aeiouyë", "ccccccccccccccc", "eeeeeeeeeeeeeee", "Dh-Ll", "ŉ", "ue", "ie", "rrrr", "ieueuaye", "kuuae", "IE", "UA", "iee", "yeyeye", "uaj", "mbiemmbiemër"];
        for w in odd {
            let _ = (letters(w), to_ipa(w), syllables(w), syllable_count(w), restore(w, &lex));
            for h in [Hint::Unknown, Hint::Inflected, Hint::Lemma(Pos::Noun), Hint::Lemma(Pos::Verb), Hint::Lemma(Pos::Adv), Hint::Lemma(Pos::Pron)] {
                let s = stress(w, h);
                assert!(s.map(|s| syllable_count(w).map(|n| s.from_end < n).unwrap_or(true)).unwrap_or(true));
            }
        }
        assert_eq!(syllables("aeiouyë").map(|s| s.len()), Some(7));
        assert_eq!(syllable_count("grua"), Some(1));
    }
}
