//! Trajtat që leksiku i njeh vetë: temat e leksemës (shtylla «stems») me mbaresat e klasës së
//! saj (shtylla «paradigms», tabela `paradigmat`). Prej tyre del a është fjala trajtë e lakuar
//! standarde, cila është pjesorja standarde e një foljeje dhe cilat pjesore gege i përgjigjen.

use super::paradigmat::{Class, ANY, CLASSES};
use crate::gramatika::leksiku::{Lexeme, Lexicon};
use crate::gramatika::tipet::Pos;
use std::collections::HashMap;

/// Klasat e një paradigme sipas emrit (tabela është e renditur).
fn classes(name: &str) -> &'static [Class] {
    let lo = CLASSES.partition_point(|c| c.name < name);
    let hi = CLASSES.partition_point(|c| c.name <= name);
    CLASSES.get(lo..hi).unwrap_or(&[])
}

/// Temat e leksemës me numrin e tyre; variantet e një teme ndahen me «//».
fn stems(l: &Lexeme) -> impl Iterator<Item = (usize, &str)> + '_ {
    l.stems
        .iter()
        .enumerate()
        .flat_map(|(n, s)| s.split("//").map(move |v| (n, v.trim_matches('.'))))
        .filter(|(_, v)| !v.is_empty() && *v != "NOSTEM")
}

/// Klasat që lidhen me temën `n` të leksemës; leksema me një temë të vetme e përdor atë për çdo
/// numër teme të paradigmës (hap: hap-ëm, hap-ur).
fn classes_of(l: &Lexeme, n: usize) -> impl Iterator<Item = &'static Class> + '_ {
    let one = l.stems.len() == 1;
    l.paradigms.iter().flat_map(|p| classes(p).iter()).filter(move |c| one || c.stem == ANY || usize::from(c.stem) == n)
}

/// Pjesa e fjalës pas temës, kur fjala nis me temën; tema e emrit të përveçëm krahasohet pa
/// dallim të shkronjës së madhe.
fn after<'w>(word: &'w str, stem: &str) -> Option<&'w str> {
    let folded = || {
        let mut rest = word;
        for s in stem.chars() {
            let c = rest.chars().next()?;
            rest = s.to_lowercase().eq(c.to_lowercase()).then(|| rest.get(c.len_utf8()..)).flatten()?;
        }
        Some(rest)
    };
    word.strip_prefix(stem).or_else(|| stem.chars().any(char::is_uppercase).then(folded).flatten())
}

/// Treguesi i temave: tema me shkronja të vogla → leksemat standarde që e kanë dhe numri i temës.
/// Ndërtohet një herë për leksikun; pa të, çdo analizë kalon nëpër gjithë leksikun.
#[derive(Debug, Default)]
pub struct Stems<'a> {
    by_stem: HashMap<String, Vec<(&'a Lexeme, usize)>>,
}

impl<'a> Stems<'a> {
    pub fn new(lex: &'a Lexicon) -> Stems<'a> {
        let mut by_stem: HashMap<String, Vec<(&'a Lexeme, usize)>> = HashMap::new();
        for l in lex.iter().filter(|l| l.standard) {
            for (n, s) in stems(l) {
                by_stem.entry(s.to_lowercase()).or_default().push((l, n));
            }
        }
        Stems { by_stem }
    }
}

/// Burimi i leksemave: treguesi i temave ose kalimi nëpër gjithë leksikun.
#[derive(Debug, Clone, Copy)]
pub enum Source<'a> {
    Scan(&'a Lexicon),
    Index(&'a Stems<'a>),
}

/// Leksemat standarde që e nisin fjalën me një nga temat e tyre: (leksema, numri i temës, mbetja).
fn readings<'a, 'w>(src: Source<'a>, word: &'w str) -> Vec<(&'a Lexeme, usize, &'w str)> {
    match src {
        Source::Scan(lex) => {
            lex.iter().filter(|l| l.standard).flat_map(|l| stems(l).filter_map(move |(n, s)| after(word, s).map(|rest| (l, n, rest)))).collect()
        }
        Source::Index(ix) => word
            .char_indices()
            .map(|(i, c)| i + c.len_utf8())
            .filter_map(|i| word.get(..i).zip(word.get(i..)))
            .flat_map(|(stem, rest)| ix.by_stem.get(stem).into_iter().flatten().map(move |(l, n)| (*l, *n, rest)))
            .collect(),
    }
}

/// Mbetja e fjalës është mbaresë e zgjedhur nga `pick` te një klasë e temës.
fn ends(l: &Lexeme, n: usize, rest: &str, pick: fn(&Class) -> &'static [&'static str]) -> bool {
    classes_of(l, n).any(|c| pick(c).contains(&rest))
}

/// Pjesoret standarde të leksemës: temë + mbaresë e pjesores.
fn participles(l: &Lexeme) -> impl Iterator<Item = String> + '_ {
    stems(l).flat_map(move |(n, s)| classes_of(l, n).flat_map(|c| c.participle.iter()).map(move |e| format!("{s}{e}")))
}

/// Fjala është trajtë e lakuar e një leme standarde: temë + mbaresë standarde e klasës.
pub fn standard_form(src: Source<'_>, word: &str) -> bool {
    readings(src, word).into_iter().any(|(l, n, rest)| ends(l, n, rest, |c| c.endings))
}

/// Fjala është pjesore standarde e një foljeje standarde.
pub fn standard_participle(src: Source<'_>, word: &str) -> bool {
    readings(src, word).into_iter().any(|(l, n, rest)| l.pos == Pos::Verb && ends(l, n, rest, |c| c.participle))
}

/// Pjesoret standarde të foljeve standarde që e kanë fjalën pjesore gege sipas klasës
/// (hapun/hapë → hapur, punue → punuar, ardhë → ardhur), të renditura, pa përsëritje.
pub fn gheg_participle(src: Source<'_>, word: &str) -> Vec<String> {
    let mut out: Vec<String> = readings(src, word)
        .into_iter()
        .filter(|(l, n, rest)| l.pos == Pos::Verb && ends(l, *n, rest, |c| c.gheg))
        .flat_map(|(l, _, _)| participles(l))
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon(rows: &[(&str, &str, &str, &str)]) -> Lexicon {
        let text: String = rows
            .iter()
            .map(|(lemma, upos, par, stems)| format!("{lemma}\t{lemma}\t{upos}\t-\t-\t-\t-\t-\t-\t-\tstd\t{par}\t{stems}\t-\t-\t-\t-\t-\n"))
            .collect();
        Lexicon::parse(&text)
    }

    #[test]
    fn inflected_standard_forms() {
        let lex = lexicon(&[
            ("mik", "NOUN", "2a,5b", "miq.|mik."),
            ("shtëpi", "NOUN", "3d,5b", "shtëpi.|shtëpi."),
            ("punoj", "V", "lexoj", "puno.|punua.|punu.|pun."),
            ("Kinë", "PROPN", "3a", "Kina.|Kinë.|Kin."),
        ]);
        let ix = Stems::new(&lex);
        for src in [Source::Scan(&lex), Source::Index(&ix)] {
            for w in ["mikun", "miqtë", "shtëpisë", "shtëpinë", "punoj", "punuam", "punonte", "punuar", "kinës"] {
                assert!(standard_form(src, w), "{w}");
            }
            for w in ["mikur", "punue", "punuem", "shtëpiza", ""] {
                assert!(!standard_form(src, w), "{w}");
            }
        }
    }

    #[test]
    fn gheg_participles_follow_the_class() {
        let lex = lexicon(&[("hap", "V", "hap", "hap."), ("punoj", "V", "lexoj", "puno.|punua.|punu.|pun."), ("vij", "V", "vij", "vi.|vje.|ardh.|erdh.//erth.|eja.")]);
        let ix = Stems::new(&lex);
        for src in [Source::Scan(&lex), Source::Index(&ix)] {
            assert_eq!(gheg_participle(src, "hapun"), vec!["hapur".to_string()]);
            assert_eq!(gheg_participle(src, "hapë"), vec!["hapur".to_string()]);
            assert!(gheg_participle(src, "punue").contains(&"punuar".to_string()));
            assert!(gheg_participle(src, "ardhë").contains(&"ardhur".to_string()));
            assert!(gheg_participle(src, "hapur").is_empty());
            assert!(standard_participle(src, "hapur"));
            assert!(standard_participle(src, "punuar"));
            assert!(!standard_participle(src, "hapun"));
        }
    }
}
