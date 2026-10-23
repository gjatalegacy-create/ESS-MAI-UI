//! Rikthimi i diakritikëve: fjala e shkruar pa «ë» dhe «ç» (tastierë pa shenja).
//!
//! Rendi i dëshmisë:
//! 1. kandidatët janë të gjitha zgjedhjet e/ë dhe c/ç në vendet ku fjala ka «e» ose «c»
//!    pa shenjë (2^k kandidatë për k vende; mbi `MAX_POSITIONS` vende nuk provohet);
//! 2. kandidati është i atestuar kur është lemë e një lekseme standarde të leksikut ose fjalë
//!    shërbimi e `FUNCTION_FORMS` (R-D1), ose trajtë e një lekseme standarde: një temë e saj
//!    + një mbaresë standarde e klasës së saj të paradigmës (R-D2);
//! 3. kthehet kandidati i vetëm i atestuar;
//! 4. dy ose më shumë kandidatë të atestuar, ose asnjë: None (ne ~ në, pune ~ punë,
//!    eshtë ~ është). Variantet jostandarde të leksikut nuk atestojnë asgjë.
//! Shkronjat «ë» dhe «ç» që fjala i ka tashmë nuk ndryshohen; shkronja e madhe ruhet.

use super::klasat::{CLASSES, EVERY};
use crate::gramatika::leksiku::{Lexeme, Lexicon};
use crate::gramatika::tipet::Form;
use std::collections::HashMap;

/// Numri më i madh i vendeve e/c që provohen (2^10 = 1024 kandidatë).
pub const MAX_POSITIONS: usize = 10;

/// Trajtat e nyjës së përparme (i, e, të, së) dhe parafjala «te»: fjalë shërbimi që leksiku
/// i lemave nuk i mban si lemë më vete. Numërohen si të atestuara, që «te ~ të» dhe
/// «se ~ së» të mos zgjidhen me hamendje.
pub const FUNCTION_FORMS: [&str; 5] = ["i", "e", "të", "së", "te"];

/// Mbaresa më e gjatë e klasave të paradigmave, në shkronja.
const LONGEST_ENDING: usize = 7;

const LEMMA_RULE: &str = "R-D1: kandidati i vetëm i atestuar, lemë standarde";
const FORM_RULE: &str = "R-D2: kandidati i vetëm i atestuar, trajtë e paradigmës standarde";

fn marked(c: char) -> Option<char> {
    match c {
        'e' => Some('ë'),
        'E' => Some('Ë'),
        'c' => Some('ç'),
        'C' => Some('Ç'),
        _ => None,
    }
}

fn is_combining(c: char) -> bool {
    ('\u{300}'..='\u{36f}').contains(&c)
}

fn lower(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// Kandidatët me diakritikë: çdo nënbashkësi e vendeve «e»/«c» pa shenjë; None mbi `MAX_POSITIONS`.
pub fn candidates(word: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = word.chars().collect();
    let free: Vec<usize> = (0..chars.len())
        .filter(|i| chars.get(*i).copied().and_then(marked).is_some() && !chars.get(i + 1).copied().map(is_combining).unwrap_or(false))
        .collect();
    (free.len() <= MAX_POSITIONS && !chars.is_empty()).then_some(())?;
    let variant = |mask: usize| -> String {
        chars
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let flip = free.iter().position(|p| *p == i).map(|bit| (mask >> bit) & 1 == 1).unwrap_or(false);
                match flip {
                    true => marked(*c).unwrap_or(*c),
                    false => *c,
                }
            })
            .collect()
    };
    Some((0..(1usize << free.len())).map(variant).collect())
}

/// Mbaresat standarde të një klase për temën me numrin `idx` (fusha bosh = pa mbaresë).
fn endings(class: &str, idx: usize) -> impl Iterator<Item = &'static str> {
    let groups = CLASSES.binary_search_by(|(n, _)| (*n).cmp(class)).ok().and_then(|i| CLASSES.get(i)).map(|(_, g)| *g).unwrap_or(&[]);
    groups.iter().filter(move |(i, _)| *i == EVERY || usize::from(*i) == idx).flat_map(|(_, e)| e.split('|'))
}

/// Variantet e pastra të një teme (pa pikat, pa «NOSTEM»).
fn variants(stem: &str) -> impl Iterator<Item = &str> {
    stem.split("//").map(|v| v.trim_matches('.')).filter(|v| !v.is_empty() && *v != "NOSTEM")
}

/// Leksema ka trajtën `stem + ending`: tema është në vendin `idx` dhe klasa e lejon mbaresën.
fn has_form(lx: &Lexeme, stem: &str, ending: &str) -> bool {
    let places = lx.stems.iter().enumerate().filter(|(_, s)| variants(s).any(|v| v.chars().flat_map(char::to_lowercase).eq(stem.chars()))).map(|(i, _)| i);
    places.into_iter().any(|idx| lx.paradigms.iter().any(|p| endings(p, idx).any(|e| e == ending)))
}

/// Dëshmia e leksikut për rikthimin: temat e lekseme standarde, të ndërtuara një herë.
pub struct Restorer<'a> {
    lex: &'a Lexicon,
    stems: HashMap<String, Vec<&'a Lexeme>>,
}

impl<'a> Restorer<'a> {
    pub fn new(lex: &'a Lexicon) -> Restorer<'a> {
        let mut stems: HashMap<String, Vec<&'a Lexeme>> = HashMap::new();
        for lx in lex.iter().filter(|l| l.standard) {
            let mut seen: Vec<String> = lx.stems.iter().flat_map(|s| variants(s)).map(lower).collect();
            seen.sort();
            seen.dedup();
            seen.into_iter().for_each(|v| stems.entry(v).or_default().push(lx));
        }
        Restorer { lex, stems }
    }

    fn lemma(&self, cand: &str) -> bool {
        let low = lower(cand);
        let standard = |w: &str| self.lex.entries(w).iter().any(|l| l.standard);
        FUNCTION_FORMS.contains(&low.as_str()) || standard(cand) || standard(&low)
    }

    /// Trajta i përket paradigmës së një lekseme standarde: temë + mbaresë e klasës.
    fn form(&self, cand: &str) -> bool {
        let low = lower(cand);
        let cuts: Vec<usize> = low.char_indices().map(|(b, _)| b).chain(std::iter::once(low.len())).collect();
        cuts.iter().rev().take(LONGEST_ENDING + 1).filter(|b| **b > 0).any(|b| {
            let (stem, ending) = (low.get(..*b).unwrap_or(""), low.get(*b..).unwrap_or(""));
            self.stems.get(stem).map(|ls| ls.iter().any(|lx| has_form(lx, stem, ending))).unwrap_or(false)
        })
    }

    fn evidence(&self, cand: &str) -> Option<&'static str> {
        match self.lemma(cand) {
            true => Some(LEMMA_RULE),
            false => self.form(cand).then_some(FORM_RULE),
        }
    }

    /// Fjala me diakritikët e rikthyer dhe rregulli që e dha.
    pub fn restoration(&self, word: &str) -> Option<Form> {
        let all = candidates(word)?;
        let found: Vec<(String, &'static str)> = all.into_iter().filter_map(|c| self.evidence(&c).map(|r| (c, r))).take(2).collect();
        match found.len() {
            1 => found.into_iter().next().map(|(t, r)| Form::one(t, r)),
            _ => None,
        }
    }

    /// Fjala me diakritikët e rikthyer; None kur leksiku nuk jep një kandidat të vetëm.
    pub fn restore(&self, word: &str) -> Option<String> {
        self.restoration(word).map(|f| f.text)
    }
}

/// Rikthimi për një fjalë të vetme; për tekst të gjatë `Restorer` ndërtohet një herë.
pub fn restoration(word: &str, lex: &Lexicon) -> Option<Form> {
    Restorer::new(lex).restoration(word)
}

/// Fjala me diakritikët e rikthyer; None kur leksiku nuk jep një kandidat të vetëm.
pub fn restore(word: &str, lex: &Lexicon) -> Option<String> {
    restoration(word, lex).map(|f| f.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex() -> Lexicon {
        let row = |lemma: &str, upos: &str, status: &str, par: &str, stems: &str| format!("x\t{lemma}\t{upos}\t-\tf\t-\t-\t-\t-\t-\t{status}\t{par}\t{stems}\t-\t-\t-\t-\t-");
        let text = [
            row("këngë", "NOUN", "std", "3a,5a", "këngë.|këngë.|këng."),
            row("shkollë", "NOUN", "std", "3a,5a", "shkolla.|shkollë.|shkoll."),
            row("çelës", "NOUN", "std", "1a,5a", "çelësa.|çelës."),
            row("pe", "NOUN", "std", "1c", "pe.|pe."),
            row("pë", "NOUN", "std", "3a", "pë.|pë.|p."),
            row("cep", "NOUN", "std", "1a,5a", "cepa.|cep."),
            row("cepë", "NOUN", "nonst", "3a", "cepë.|cepë.|cep."),
            row("çë", "PRO", "nonst", "unchangeable", "çë."),
            row("eshtë", "NOUN", "std", "3a,5a", "eshta.|eshtë.|esht."),
            row("jam", "V", "std", "jam", "ja.|je.//jë.|ësh.//ish.|qen.//klen.|ish.|qe.//kle.|ji.|qo.//klo."),
            row("dhe", "CONJ", "std", "unchangeable", "dhe."),
            row("se", "CONJ", "std", "unchangeable", "se."),
            row("të", "PART", "std", "unchangeable", "të."),
        ]
        .join("\n");
        Lexicon::parse(&text)
    }

    #[test]
    fn unique_lemma_or_form_is_restored() {
        let lx = lex();
        let r = Restorer::new(&lx);
        assert_eq!(r.restore("Celes").as_deref(), Some("Çelës"));
        assert_eq!(r.restore("dhe").as_deref(), Some("dhe"));
        assert_eq!(r.restoration("celes").map(|f| f.rule), Some(LEMMA_RULE));
        assert_eq!(r.restoration("shkolles").map(|f| (f.text, f.rule)), Some(("shkollës".to_string(), FORM_RULE)));
        assert_eq!(r.restore("celesat").as_deref(), Some("çelësat"));
        assert_eq!(restore("kenges", &lx).as_deref(), Some("këngës"));
    }

    #[test]
    fn valid_inflected_input_is_never_rewritten() {
        let lx = lex();
        let r = Restorer::new(&lx);
        assert_eq!(r.restore("shkolle"), None);
        assert_eq!(r.restore("eshte"), None);
        assert_eq!(r.restore("kenge"), None);
        assert_eq!(r.restore("kënge"), None);
    }

    #[test]
    fn variants_attest_nothing_and_ambiguity_is_held() {
        let lx = lex();
        let r = Restorer::new(&lx);
        assert_eq!(r.restore("cepe"), None);
        assert_eq!(r.restore("ce"), None);
        assert_eq!(r.restore("cep").as_deref(), Some("cep"));
        assert_eq!(r.restore("pe"), None);
        assert_eq!(r.restore("se"), None);
        assert_eq!(r.restore("Te"), None);
        assert_eq!(r.restore("xyz"), None);
        assert_eq!(r.restore("këngës").as_deref(), Some("këngës"));
    }

    #[test]
    fn candidate_space_is_bounded() {
        assert_eq!(candidates("ce").map(|c| c.len()), Some(4));
        assert_eq!(candidates("eeeeeeeeeee"), None);
        assert_eq!(candidates(""), None);
    }
}
