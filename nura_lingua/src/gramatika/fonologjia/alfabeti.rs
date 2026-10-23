//! Alfabeti i shqipes standarde (Kongresi i Manastirit 1908, Kongresi i Drejtshkrimit 1972):
//! 36 shkronja, nëntë prej tyre dyshkronjëshe (dh gj ll nj rr sh th xh zh) që
//! numërohen si një shkronjë e vetme. Zanore: a e ë i o u y.

/// Mënyra e nyjëtimit të bashkëtingëllores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Manner {
    Stop,
    Affricate,
    Fricative,
    Nasal,
    Liquid,
    Glide,
}

/// Tingulli që shënon një shkronjë: zanore, ose bashkëtingëllore me mënyrë dhe zë.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sound {
    Vowel,
    Consonant { manner: Manner, voiced: bool },
}

const fn cons(manner: Manner, voiced: bool) -> Sound {
    Sound::Consonant { manner, voiced }
}

/// Shkronja, tingulli dhe vlera e saj fonologjike (IPA), sipas rendit të alfabetit.
const TABLE: [(&str, Sound, &str); 36] = [
    ("a", Sound::Vowel, "a"),
    ("b", cons(Manner::Stop, true), "b"),
    ("c", cons(Manner::Affricate, false), "t͡s"),
    ("ç", cons(Manner::Affricate, false), "t͡ʃ"),
    ("d", cons(Manner::Stop, true), "d"),
    ("dh", cons(Manner::Fricative, true), "ð"),
    ("e", Sound::Vowel, "ɛ"),
    ("ë", Sound::Vowel, "ə"),
    ("f", cons(Manner::Fricative, false), "f"),
    ("g", cons(Manner::Stop, true), "ɡ"),
    ("gj", cons(Manner::Stop, true), "ɟ"),
    ("h", cons(Manner::Fricative, false), "h"),
    ("i", Sound::Vowel, "i"),
    ("j", cons(Manner::Glide, true), "j"),
    ("k", cons(Manner::Stop, false), "k"),
    ("l", cons(Manner::Liquid, true), "l"),
    ("ll", cons(Manner::Liquid, true), "ɫ"),
    ("m", cons(Manner::Nasal, true), "m"),
    ("n", cons(Manner::Nasal, true), "n"),
    ("nj", cons(Manner::Nasal, true), "ɲ"),
    ("o", Sound::Vowel, "ɔ"),
    ("p", cons(Manner::Stop, false), "p"),
    ("q", cons(Manner::Stop, false), "c"),
    ("r", cons(Manner::Liquid, true), "ɾ"),
    ("rr", cons(Manner::Liquid, true), "r"),
    ("s", cons(Manner::Fricative, false), "s"),
    ("sh", cons(Manner::Fricative, false), "ʃ"),
    ("t", cons(Manner::Stop, false), "t"),
    ("th", cons(Manner::Fricative, false), "θ"),
    ("u", Sound::Vowel, "u"),
    ("v", cons(Manner::Fricative, true), "v"),
    ("x", cons(Manner::Affricate, true), "d͡z"),
    ("xh", cons(Manner::Affricate, true), "d͡ʒ"),
    ("y", Sound::Vowel, "y"),
    ("z", cons(Manner::Fricative, true), "z"),
    ("zh", cons(Manner::Fricative, true), "ʒ"),
];

/// Të 36 shkronjat, në rendin e alfabetit.
pub const ALPHABET: [&str; 36] = [
    "a", "b", "c", "ç", "d", "dh", "e", "ë", "f", "g", "gj", "h", "i", "j", "k", "l", "ll", "m", "n", "nj", "o", "p", "q", "r", "rr", "s", "sh", "t", "th", "u", "v", "x",
    "xh", "y", "z", "zh",
];

/// Dyshkronjëshat: një shkronjë, një tingull.
pub const DIGRAPHS: [&str; 9] = ["dh", "gj", "ll", "nj", "rr", "sh", "th", "xh", "zh"];

/// Shenjat diakritike kombinuese (U+0300–U+036F) i bashkohen shkronjës para tyre.
fn is_combining(c: char) -> bool {
    ('\u{300}'..='\u{36f}').contains(&c)
}

/// Fundi i njësisë që nis te `start`: një shenjë bazë me shenjat kombinuese pas saj.
fn unit_end(word: &str, start: usize) -> usize {
    let rest = word.get(start..).unwrap_or("");
    rest.char_indices().skip(1).find(|(_, c)| !is_combining(*c)).map(|(i, _)| start + i).unwrap_or(word.len())
}

/// Shkronjat e fjalës, me dyshkronjëshat si një njësi (lexim i lakmueshëm, pa dallim
/// shkronje të madhe/të vogël: «Shqip» → Sh, q, i, p). Çdo shenjë tjetër mbetet njësi më vete.
pub fn letters(word: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < word.len() {
        let one = unit_end(word, start);
        let two = unit_end(word, one);
        let pair = word.get(start..two).unwrap_or("");
        let digraph = two == start + 2 && DIGRAPHS.iter().any(|d| d.eq_ignore_ascii_case(pair));
        let end = match digraph {
            true => two,
            false => one,
        };
        out.push(word.get(start..end).unwrap_or(""));
        start = end;
    }
    out
}

/// Njësia është kjo shkronjë, pa dallim shkronje të madhe/të vogël; «ë» dhe «ç» pranohen
/// edhe të zbërthyera (e + U+0308, c + U+0327).
fn same_letter(unit: &str, letter: &str) -> bool {
    let decomposed = match letter {
        "ë" => "e\u{308}",
        "ç" => "c\u{327}",
        other => other,
    };
    let folded = || unit.chars().flat_map(char::to_lowercase);
    folded().eq(letter.chars()) || folded().eq(decomposed.chars())
}

/// Vendi i shkronjës kanonike (e vogël, e përbërë) në alfabet.
fn exact(unit: &str) -> Option<usize> {
    match unit {
        "a" => Some(0),
        "b" => Some(1),
        "c" => Some(2),
        "ç" => Some(3),
        "d" => Some(4),
        "dh" => Some(5),
        "e" => Some(6),
        "ë" => Some(7),
        "f" => Some(8),
        "g" => Some(9),
        "gj" => Some(10),
        "h" => Some(11),
        "i" => Some(12),
        "j" => Some(13),
        "k" => Some(14),
        "l" => Some(15),
        "ll" => Some(16),
        "m" => Some(17),
        "n" => Some(18),
        "nj" => Some(19),
        "o" => Some(20),
        "p" => Some(21),
        "q" => Some(22),
        "r" => Some(23),
        "rr" => Some(24),
        "s" => Some(25),
        "sh" => Some(26),
        "t" => Some(27),
        "th" => Some(28),
        "u" => Some(29),
        "v" => Some(30),
        "x" => Some(31),
        "xh" => Some(32),
        "y" => Some(33),
        "z" => Some(34),
        "zh" => Some(35),
        _ => None,
    }
}

/// Vendi i njësisë në alfabet.
fn index(unit: &str) -> Option<usize> {
    exact(unit).or_else(|| ALPHABET.iter().position(|l| same_letter(unit, l)))
}

/// Shkronja kanonike (e vogël, e përbërë) e një njësie; None për çdo shenjë jashtë alfabetit.
pub fn canonical(unit: &str) -> Option<&'static str> {
    index(unit).and_then(|i| ALPHABET.get(i)).copied()
}

fn entry(letter: &str) -> Option<&'static (&'static str, Sound, &'static str)> {
    index(letter).and_then(|i| TABLE.get(i))
}

/// Tingulli i një shkronje.
pub fn sound(letter: &str) -> Option<Sound> {
    entry(letter).map(|(_, s, _)| *s)
}

/// Vlera fonologjike (IPA) e një shkronje.
pub fn ipa_of(letter: &str) -> Option<&'static str> {
    entry(letter).map(|(_, _, p)| *p)
}

pub fn is_vowel(letter: &str) -> bool {
    sound(letter) == Some(Sound::Vowel)
}

pub fn is_consonant(letter: &str) -> bool {
    matches!(sound(letter), Some(Sound::Consonant { .. }))
}

pub fn manner(letter: &str) -> Option<Manner> {
    match sound(letter)? {
        Sound::Consonant { manner, .. } => Some(manner),
        Sound::Vowel => None,
    }
}

/// Pengoret: të plota, afrikatet dhe fërkimoret.
pub fn is_obstruent(letter: &str) -> bool {
    matches!(manner(letter), Some(Manner::Stop | Manner::Affricate | Manner::Fricative))
}

/// Shkalla e sonoritetit (Clements 1990): pengore 1, hundore 2, lëngëzore 3, gjysmëzanore 4, zanore 5.
pub fn sonority(letter: &str) -> Option<u8> {
    sound(letter).map(|s| match s {
        Sound::Vowel => 5,
        Sound::Consonant { manner, .. } => match manner {
            Manner::Stop | Manner::Affricate | Manner::Fricative => 1,
            Manner::Nasal => 2,
            Manner::Liquid => 3,
            Manner::Glide => 4,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digraphs_are_single_letters() {
        assert_eq!(letters("shqipëri"), vec!["sh", "q", "i", "p", "ë", "r", "i"]);
        assert_eq!(letters("Gjergj"), vec!["Gj", "e", "r", "gj"]);
        assert_eq!(letters("DHJETË"), vec!["DH", "J", "E", "T", "Ë"]);
        assert_eq!(letters("rrallë"), vec!["rr", "a", "ll", "ë"]);
        assert_eq!(letters("xhaxha"), vec!["xh", "a", "xh", "a"]);
        assert_eq!(letters("nj-ë"), vec!["nj", "-", "ë"]);
        assert_eq!(letters("ne\u{308}"), vec!["n", "e\u{308}"]);
        assert_eq!(letters(""), Vec::<&str>::new());
        assert_eq!(ALPHABET.len(), 36);
    }

    #[test]
    fn letter_classes() {
        assert!(is_vowel("ë") && is_vowel("Y") && !is_vowel("j"));
        assert_eq!(sound("xh"), Some(Sound::Consonant { manner: Manner::Affricate, voiced: true }));
        assert_eq!(sound("q"), Some(Sound::Consonant { manner: Manner::Stop, voiced: false }));
        assert_eq!(manner("rr"), Some(Manner::Liquid));
        assert_eq!(canonical("E\u{308}"), Some("ë"));
        assert_eq!(canonical("w"), None);
        assert_eq!(sonority("t"), Some(1));
        assert_eq!(sonority("n"), Some(2));
        assert_eq!(sonority("ll"), Some(3));
        assert_eq!(sonority("j"), Some(4));
        assert_eq!(sonority("a"), Some(5));
        let vowels = ALPHABET.iter().filter(|l| is_vowel(l)).count();
        assert_eq!(vowels, 7);
        assert!(TABLE.iter().zip(ALPHABET.iter()).all(|((l, _, _), a)| l == a));
        assert!(ALPHABET.iter().enumerate().all(|(i, l)| exact(l) == Some(i)));
        assert_eq!(canonical("Ç"), Some("ç"));
        assert_eq!(canonical("C\u{327}"), Some("ç"));
        assert_eq!(canonical("Dh"), Some("dh"));
    }
}
