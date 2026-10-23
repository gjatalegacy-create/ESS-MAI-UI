//! Shkronjat e shqipes: dyshkronjëshet, zanoret gojore dhe hundore, tingëllueshmëria.

/// Dyshkronjëshet e alfabetit; secili lexohet si një shkronjë.
const DIGRAPHS: [&str; 9] = ["dh", "gj", "ll", "nj", "rr", "sh", "th", "xh", "zh"];

/// Zanoret hundore të gegërishtes (me «^» në shkrimin letrar, me «~» në transkriptim)
/// dhe zanorja gojore me të njëjtën nyjëtim.
const NASALS: [(&str, &str); 12] = [
    ("â", "a"),
    ("ê", "e"),
    ("î", "i"),
    ("ô", "o"),
    ("û", "u"),
    ("ŷ", "y"),
    ("ã", "a"),
    ("ẽ", "e"),
    ("ĩ", "i"),
    ("õ", "o"),
    ("ũ", "u"),
    ("ỹ", "y"),
];

const ORAL: [&str; 10] = ["a", "e", "ë", "i", "o", "u", "y", "ä", "ö", "ü"];

pub fn is_nasal(t: &str) -> bool {
    NASALS.iter().any(|(n, _)| *n == t)
}

pub fn is_vowel(t: &str) -> bool {
    ORAL.contains(&t) || is_nasal(t)
}

/// Zanore e plotë: çdo zanore përveç «ë»-së, që në gegërisht bie kur s'mban theks.
pub fn is_full_vowel(t: &str) -> bool {
    is_vowel(t) && t != "ë"
}

pub fn is_consonant(t: &str) -> bool {
    !t.is_empty() && !is_vowel(t) && t.chars().all(char::is_alphabetic)
}

/// Bashkëtingëlloret hundore: para tyre zanorja gege hundorëzohet.
pub fn is_nasal_consonant(t: &str) -> bool {
    matches!(t, "m" | "n" | "nj")
}

/// Shenjat e theksit dhe të gjatësisë nuk janë përkim dialektor: hiqen.
fn plain(c: char) -> char {
    match c {
        'á' | 'à' | 'ā' | 'ǎ' => 'a',
        'é' | 'è' | 'ē' | 'ě' => 'e',
        'í' | 'ì' | 'ī' | 'ǐ' => 'i',
        'ó' | 'ò' | 'ō' | 'ǒ' => 'o',
        'ú' | 'ù' | 'ū' | 'ǔ' => 'u',
        'ý' | 'ỳ' | 'ȳ' => 'y',
        c => c,
    }
}

/// Shenja kombinuese mbi zanoren: «^» dhe «~» japin hundoren, «¨» mbi «e» jep «ë».
fn marked(base: char, mark: char) -> Option<char> {
    match (base, mark) {
        ('a', '\u{302}') => Some('â'),
        ('e', '\u{302}') => Some('ê'),
        ('i', '\u{302}') => Some('î'),
        ('o', '\u{302}') => Some('ô'),
        ('u', '\u{302}') => Some('û'),
        ('y', '\u{302}') => Some('ŷ'),
        ('a', '\u{303}') => Some('ã'),
        ('e', '\u{303}') => Some('ẽ'),
        ('i', '\u{303}') => Some('ĩ'),
        ('o', '\u{303}') => Some('õ'),
        ('u', '\u{303}') => Some('ũ'),
        ('y', '\u{303}') => Some('ỹ'),
        ('e', '\u{308}') => Some('ë'),
        _ => None,
    }
}

/// Trajta për krahasim: shkronja të vogla, pa shenja theksi a gjatësie; hundorja ruhet.
pub fn fold(word: &str) -> String {
    let mut out: Vec<char> = Vec::new();
    for c in word.chars().flat_map(char::to_lowercase) {
        match ('\u{300}'..='\u{36f}').contains(&c) {
            true => {
                match out.last().and_then(|b| marked(*b, c)) {
                    Some(j) => {
                        out.pop();
                        out.push(j);
                    }
                    None => {}
                }
            }
            false => out.push(plain(c)),
        }
    }
    out.into_iter().collect()
}

/// Ndarja në shkronja të alfabetit: «shpnesë» → [sh, p, n, e, s, ë].
pub fn letters(word: &str) -> Vec<String> {
    let cs: Vec<char> = word.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let pair: String = cs.iter().skip(i).take(2).collect();
        let take = match DIGRAPHS.contains(&pair.as_str()) {
            true => 2,
            false => 1,
        };
        out.push(cs.iter().skip(i).take(take).collect());
        i += take;
    }
    out
}

/// Shkalla e tingëllueshmërisë: shpërthyeset dhe afrikatet 1, fërkimoret 2, hundoret 3,
/// lëngëzoret 4, gjysmëzanorja «j» 5, zanoret 6.
pub fn sonority(t: &str) -> u8 {
    match t {
        "p" | "b" | "t" | "d" | "k" | "g" | "q" | "gj" | "c" | "ç" | "x" | "xh" => 1,
        "f" | "v" | "s" | "z" | "sh" | "zh" | "th" | "dh" | "h" => 2,
        "m" | "n" | "nj" => 3,
        "l" | "ll" | "r" | "rr" => 4,
        "j" => 5,
        t => match is_vowel(t) {
            true => 6,
            false => 0,
        },
    }
}

/// Parimi i renditjes së tingëllueshmërisë: grupi në krye të rrokjes ngjitet drejt zanores;
/// «s»/«sh» para shpërthyeses është përjashtimi i njohur (shkoj, stinë).
pub fn well_formed_onset(a: &str, b: &str) -> bool {
    sonority(a) < sonority(b) || (matches!(a, "s" | "sh" | "z" | "zh") && sonority(b) == 1)
}

/// Largësia e Levenshtein-it mbi shkronja:
/// d(i, j) = min(d(i−1, j) + 1, d(i, j−1) + 1, d(i−1, j−1) + [aᵢ ≠ bⱼ]).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = Vec::with_capacity(b.len() + 1);
        cur.push(i + 1);
        for (j, cb) in b.iter().enumerate() {
            let sub = prev.get(j).copied().unwrap_or(usize::MAX - 1) + usize::from(ca != *cb);
            let del = prev.get(j + 1).copied().unwrap_or(usize::MAX - 1) + 1;
            let ins = cur.last().copied().unwrap_or(usize::MAX - 1) + 1;
            cur.push(sub.min(del).min(ins));
        }
        prev = cur;
    }
    prev.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_keep_digraphs_whole() {
        assert_eq!(letters("shpnesë"), vec!["sh", "p", "n", "e", "s", "ë"]);
        assert_eq!(letters("rrâjë"), vec!["rr", "â", "j", "ë"]);
        assert_eq!(letters("gjuni"), vec!["gj", "u", "n", "i"]);
    }

    #[test]
    fn fold_drops_stress_but_keeps_nasality() {
        assert_eq!(fold("Kalaqá"), "kalaqa");
        assert_eq!(fold("āt"), "at");
        assert_eq!(fold("Nânë"), "nânë");
        assert_eq!(fold("na\u{302}ne\u{308}"), "nânë");
    }

    #[test]
    fn sonority_scale_orders_the_classes() {
        assert!(well_formed_onset("k", "r"));
        assert!(well_formed_onset("sh", "k"));
        assert!(!well_formed_onset("m", "s"));
        assert!(!well_formed_onset("k", "p"));
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("zâni", "zëri"), 2);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("grue", "grua"), 1);
    }
}
