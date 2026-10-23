//! Rregullat e përkimit si rishkrime mbi shkronja, me kontekst dhe kosto.
//!
//! Çdo rishkrim zbatohet ose jo, në çdo vend ku konteksti i fjalës burimore e lejon;
//! kombinimet kufizohen nga numri i rregullave dhe kostoja e përgjithshme.

use super::shkronjat::{is_consonant, is_full_vowel, is_nasal, is_nasal_consonant, is_vowel, well_formed_onset};
use std::collections::BTreeMap;

pub const TABLE: &str = "D0 trajtë gege e parregullt (tabela e mbyllur)";
pub const ROTACISM: &str = "D1 rotacizmi: n gege → r";
pub const ROTACISM_SCHWA: &str = "D1b rotacizmi pas bashkëtingëlloreje: n → ër";
pub const NASAL: &str = "D2 zanore hundore → zanore gojore";
pub const NASAL_LOW: &str = "D2a hundorja e ulët â/ô → ë, a ose o";
pub const NASAL_NJ: &str = "D2b hundore + j → zanore + nj";
pub const NASAL_FINAL: &str = "D2c hundore fundore ↔ zanore + n (kalamâ/kalaman, nê/në)";
pub const LONG: &str = "D2d zanore e gjatë gege në fund ose para «j» (bli → blî, brej → brêj)";
pub const DIPHTHONG: &str = "D3 diftongu ue → ua";
pub const MONOPHTHONG: &str = "D3b njëzanorja gege u → ua";
pub const PART_UN: &str = "D4 pjesorja -un → -ur";
pub const PART_UEM: &str = "D4b pjesorja -uem → -uar";
pub const PART_UE: &str = "D4c pjesorja -ue → -uar";
pub const PART_SHORT: &str = "D4d pjesorja e shkurtër -ë → -ur";
pub const PART_YE: &str = "D4e pjesorja -ye/-ie → -yer/-ier";
pub const SCHWA: &str = "D5 ë e patheksuar e rënë";
pub const SCHWA_FINAL: &str = "D5b ë fundore e rënë";
pub const SCHWA_A: &str = "D5c a gege para hundores → ë";
pub const CLUSTER_MB: &str = "D6 grupi mb → m";
pub const CLUSTER_ND: &str = "D6b grupi nd → n";
pub const F_H: &str = "D7 f gege → h";
pub const O_A: &str = "D8 o veriore → a";
pub const UNROUNDING: &str = "D9 i gege → y";
pub const VELAR: &str = "D10 k gege → q";
pub const NORTH_PALATAL: &str = "D10b ç/xh veriore → q/gj";
pub const NJI: &str = "D11 nji → një";
pub const VERB_NJ: &str = "D12 veta e parë -nj → -j";
pub const TOSK_SCHWA: &str = "D13 ë toske para grupit hundor → e (mëndje, brënda)";
pub const SPELLING: &str = "D14 drejtshkrim i vjetër: w → v";
pub const SPELLING_Z: &str = "D14b drejtshkrim i vjetër: s midis zanoreve → z";
pub const PROTHESIS: &str = "D15 h protetike gege";
pub const INFINITIVE: &str = "D16 paskajorja gege me + pjesore → për të + pjesore";
pub const LEXREF: &str = "D17 variant i shënuar në leksik";

/// Kushti mbi shkronjat para vendit të rishkrimit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Left {
    Any,
    Start,
    Vowel,
    Consonant,
    /// Bashkëtingëllore, me një zanore diku më përpara: fjala nuk mbetet pa zanore.
    ConsonantAfterVowel,
    Nasal,
    Is(&'static str),
}

/// Kushti mbi shkronjat pas vendit të rishkrimit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Right {
    Any,
    End,
    Vowel,
    VowelOrEnd,
    Consonant,
    /// Bashkëtingëllore, me një zanore të plotë (jo «ë») më pas: «ë»-ja nuk mban theksin.
    ConsonantThenFullVowel,
    TOrEnd,
    NasalConsonant,
    /// Hundore + bashkëtingëllore (nd, mb, nt): rrokje e mbyllur, jo mbaresë.
    NasalCluster,
    EndOfMonosyllable,
    Is(&'static str),
}

impl Left {
    fn holds(self, src: &[String], i: usize) -> bool {
        let prev = i.checked_sub(1).and_then(|k| src.get(k)).map(String::as_str);
        match (self, prev) {
            (Left::Any, _) => true,
            (Left::Start, p) => p.is_none(),
            (Left::Vowel, Some(p)) => is_vowel(p),
            (Left::Consonant, Some(p)) => is_consonant(p),
            (Left::ConsonantAfterVowel, Some(p)) => is_consonant(p) && src.iter().take(i).any(|t| is_vowel(t)),
            (Left::Nasal, Some(p)) => is_nasal(p),
            (Left::Is(x), Some(p)) => x == p,
            (_, None) => false,
        }
    }
}

impl Right {
    fn holds(self, src: &[String], j: usize) -> bool {
        let next = src.get(j).map(String::as_str);
        let then = src.get(j + 1).map(String::as_str);
        match (self, next) {
            (Right::Any, _) => true,
            (Right::End, n) => n.is_none(),
            (Right::Vowel, Some(n)) => is_vowel(n),
            (Right::VowelOrEnd, n) => n.map(is_vowel).unwrap_or(true),
            (Right::Consonant, Some(n)) => is_consonant(n),
            (Right::ConsonantThenFullVowel, Some(n)) => is_consonant(n) && src.iter().skip(j).any(|t| is_full_vowel(t)),
            (Right::TOrEnd, n) => n.map(|t| t == "t").unwrap_or(true),
            (Right::NasalConsonant, Some(n)) => is_nasal_consonant(n),
            (Right::NasalCluster, Some(n)) => is_nasal_consonant(n) && then.map(is_consonant).unwrap_or(false),
            (Right::EndOfMonosyllable, None) => src.iter().filter(|t| is_vowel(t)).count() == 1,
            (Right::Is(x), Some(n)) => x == n,
            (_, _) => false,
        }
    }
}

/// Një rishkrim: shkronjat `from` të fjalës burimore bëhen `to`, kur kushtet `left`/`right` vlejnë.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rewrite {
    pub name: &'static str,
    pub from: &'static [&'static str],
    pub to: &'static [&'static str],
    pub left: Left,
    pub right: Right,
    pub cost: u32,
    /// Gjatësia më e vogël e fjalës burimore (në shkronja): në fjalët e shkurtra fqinjët
    /// leksikorë janë të dendur dhe rregulli jep goditje të rastit.
    pub min: usize,
}

const fn rw(name: &'static str, from: &'static [&'static str], to: &'static [&'static str], left: Left, right: Right, cost: u32) -> Rewrite {
    Rewrite { name, from, to, left, right, cost, min: 0 }
}

impl Rewrite {
    const fn at_least(self, min: usize) -> Rewrite {
        Rewrite { min, ..self }
    }

    fn at(&self, src: &[String], i: usize) -> bool {
        let n = self.from.len();
        let same = src.get(i..i + n).map(|w| w.iter().zip(self.from).all(|(a, b)| a == b)).unwrap_or(false);
        n > 0 && src.len() >= self.min && same && self.left.holds(src, i) && self.right.holds(src, i + n)
    }
}

/// Kostoja e «ë»-së së rikthyer midis dy bashkëtingëlloreve: më e lirë kur grupi gege
/// shkel parimin e tingëllueshmërisë (msues, mkat), sepse aty rënia e «ë»-së është më e mundshme.
fn schwa_cost(a: &str, b: &str) -> u32 {
    match well_formed_onset(a, b) {
        true => 10,
        false => 9,
    }
}

/// Gegë/toskë → standard.
pub const TO_STANDARD: &[Rewrite] = &[
    rw(ROTACISM, &["n"], &["r"], Left::Vowel, Right::Vowel, 10),
    rw(ROTACISM, &["n"], &["r"], Left::Is("ë"), Right::End, 10),
    rw(ROTACISM, &["n"], &["r"], Left::Consonant, Right::Vowel, 12),
    rw(ROTACISM_SCHWA, &["n"], &["ë", "r"], Left::Consonant, Right::Vowel, 11),
    rw(NASAL_LOW, &["â"], &["ë"], Left::Any, Right::Any, 9),
    rw(NASAL_LOW, &["â"], &["a"], Left::Any, Right::Any, 10),
    rw(NASAL_LOW, &["ã"], &["ë"], Left::Any, Right::Any, 9),
    rw(NASAL_LOW, &["ã"], &["a"], Left::Any, Right::Any, 10),
    rw(NASAL, &["ê"], &["e"], Left::Any, Right::Any, 9),
    rw(NASAL, &["ẽ"], &["e"], Left::Any, Right::Any, 9),
    rw(NASAL, &["î"], &["i"], Left::Any, Right::Any, 9),
    rw(NASAL, &["ĩ"], &["i"], Left::Any, Right::Any, 9),
    rw(NASAL_LOW, &["ô"], &["ë"], Left::Any, Right::Any, 10),
    rw(NASAL_LOW, &["ô"], &["a"], Left::Any, Right::Any, 11),
    rw(NASAL_LOW, &["ô"], &["o"], Left::Any, Right::Any, 12),
    rw(NASAL_LOW, &["õ"], &["ë"], Left::Any, Right::Any, 10),
    rw(NASAL_LOW, &["õ"], &["o"], Left::Any, Right::Any, 12),
    rw(NASAL, &["û"], &["u"], Left::Any, Right::Any, 9),
    rw(NASAL, &["ũ"], &["u"], Left::Any, Right::Any, 9),
    rw(NASAL, &["ŷ"], &["y"], Left::Any, Right::Any, 9),
    rw(NASAL, &["ỹ"], &["y"], Left::Any, Right::Any, 9),
    rw(NASAL_FINAL, &["â"], &["a", "n"], Left::Any, Right::End, 9),
    rw(NASAL_FINAL, &["ê"], &["e", "n"], Left::Any, Right::End, 9),
    rw(NASAL_FINAL, &["ê"], &["ë"], Left::Any, Right::End, 9),
    rw(NASAL_FINAL, &["î"], &["i", "n"], Left::Any, Right::End, 9),
    rw(NASAL_FINAL, &["ô"], &["o", "n"], Left::Any, Right::End, 9),
    rw(NASAL_FINAL, &["û"], &["u", "n"], Left::Any, Right::End, 9),
    rw(NASAL_FINAL, &["ŷ"], &["y", "n"], Left::Any, Right::End, 9),
    rw(NASAL_NJ, &["j"], &["nj"], Left::Nasal, Right::Any, 5),
    rw(DIPHTHONG, &["u", "e"], &["u", "a"], Left::Any, Right::Any, 10),
    rw(MONOPHTHONG, &["u"], &["u", "a"], Left::Consonant, Right::End, 11),
    rw(MONOPHTHONG, &["u", "j"], &["u", "a", "j"], Left::Consonant, Right::End, 11),
    rw(PART_UN, &["u", "n"], &["u", "r"], Left::Consonant, Right::End, 8).at_least(5),
    rw(PART_UEM, &["u", "e", "m"], &["u", "a", "r"], Left::Any, Right::End, 8),
    rw(PART_UE, &["u", "e"], &["u", "a", "r"], Left::Any, Right::End, 10),
    rw(PART_SHORT, &["ë"], &["u", "r"], Left::Consonant, Right::End, 10).at_least(4),
    rw(PART_YE, &["y", "e"], &["y", "e", "r"], Left::Any, Right::End, 10).at_least(4),
    rw(PART_YE, &["y", "e", "m"], &["y", "e", "r"], Left::Any, Right::End, 8).at_least(4),
    rw(PART_YE, &["i", "e"], &["i", "e", "r"], Left::Any, Right::End, 10).at_least(4),
    rw(PART_YE, &["i", "e", "m"], &["i", "e", "r"], Left::Any, Right::End, 8).at_least(4),
    rw(SCHWA_A, &["a"], &["ë"], Left::Any, Right::NasalConsonant, 11),
    rw(CLUSTER_MB, &["m"], &["m", "b"], Left::Nasal, Right::VowelOrEnd, 10),
    rw(CLUSTER_MB, &["m"], &["m", "b"], Left::Start, Right::Consonant, 10),
    rw(CLUSTER_ND, &["n"], &["n", "d"], Left::Nasal, Right::VowelOrEnd, 10),
    rw(F_H, &["f"], &["h"], Left::Vowel, Right::TOrEnd, 10),
    rw(O_A, &["o"], &["a"], Left::Is("v"), Right::Any, 10),
    rw(UNROUNDING, &["i"], &["y"], Left::Consonant, Right::Any, 12),
    rw(VELAR, &["k"], &["q"], Left::Any, Right::Any, 12).at_least(5),
    rw(NORTH_PALATAL, &["ç"], &["q"], Left::Any, Right::End, 10),
    rw(NORTH_PALATAL, &["xh"], &["gj"], Left::Any, Right::Any, 10),
    rw(NJI, &["nj", "i"], &["nj", "ë"], Left::Any, Right::Any, 8),
    rw(VERB_NJ, &["nj"], &["j"], Left::Vowel, Right::End, 10),
    rw(TOSK_SCHWA, &["ë"], &["e"], Left::Any, Right::NasalCluster, 11),
    rw(SPELLING_Z, &["s"], &["z"], Left::Vowel, Right::Vowel, 12),
    rw(SPELLING, &["w"], &["v"], Left::Any, Right::Any, 10),
    rw(PROTHESIS, &["h"], &[], Left::Start, Right::Vowel, 12).at_least(4),
];

/// Standard → gegë.
pub const TO_GHEG: &[Rewrite] = &[
    rw(ROTACISM, &["r"], &["n"], Left::Vowel, Right::VowelOrEnd, 10),
    rw(ROTACISM_SCHWA, &["ë", "r"], &["n"], Left::Consonant, Right::Vowel, 10),
    rw(NASAL, &["ë"], &["â"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["ë"], &["a"], Left::Any, Right::NasalConsonant, 11),
    rw(NASAL, &["ë"], &["ô"], Left::Any, Right::NasalConsonant, 12),
    rw(NASAL, &["a"], &["â"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["e"], &["ê"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["i"], &["î"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["o"], &["ô"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["u"], &["û"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["y"], &["ŷ"], Left::Any, Right::NasalConsonant, 10),
    rw(NASAL, &["ë"], &["â"], Left::Any, Right::EndOfMonosyllable, 10),
    rw(NASAL_NJ, &["ë", "nj"], &["â", "j"], Left::Any, Right::Any, 10),
    rw(NASAL_NJ, &["e", "nj"], &["ê", "j"], Left::Any, Right::Any, 10),
    rw(NASAL_NJ, &["a", "nj"], &["â", "j"], Left::Any, Right::Any, 10),
    rw(NASAL_FINAL, &["a", "n"], &["â"], Left::Any, Right::End, 12),
    rw(LONG, &["a"], &["â"], Left::Consonant, Right::End, 11),
    rw(LONG, &["e"], &["ê"], Left::Consonant, Right::End, 11),
    rw(LONG, &["i"], &["î"], Left::Consonant, Right::End, 11),
    rw(LONG, &["o"], &["ô"], Left::Consonant, Right::End, 11),
    rw(LONG, &["u"], &["û"], Left::Consonant, Right::End, 11),
    rw(LONG, &["y"], &["ŷ"], Left::Consonant, Right::End, 11),
    rw(LONG, &["a"], &["â"], Left::Consonant, Right::Is("j"), 11),
    rw(LONG, &["e"], &["ê"], Left::Consonant, Right::Is("j"), 11),
    rw(LONG, &["i"], &["î"], Left::Consonant, Right::Is("j"), 11),
    rw(LONG, &["u"], &["û"], Left::Consonant, Right::Is("j"), 11),
    rw(LONG, &["y"], &["ŷ"], Left::Consonant, Right::Is("j"), 11),
    rw(DIPHTHONG, &["u", "a"], &["u", "e"], Left::Any, Right::Any, 10),
    rw(MONOPHTHONG, &["u", "a"], &["u"], Left::Consonant, Right::End, 11),
    rw(MONOPHTHONG, &["u", "a", "j"], &["u", "j"], Left::Consonant, Right::End, 11),
    rw(PART_UE, &["u", "a", "r"], &["u", "e"], Left::Any, Right::End, 10),
    rw(PART_UEM, &["u", "a", "r"], &["u", "e", "m"], Left::Any, Right::End, 10),
    rw(PART_UN, &["u", "r"], &["u", "n"], Left::Any, Right::End, 10),
    rw(PART_YE, &["y", "e", "r"], &["y", "e"], Left::Any, Right::End, 10),
    rw(PART_YE, &["y", "e", "r"], &["y", "e", "m"], Left::Any, Right::End, 10),
    rw(PART_YE, &["i", "e", "r"], &["i", "e"], Left::Any, Right::End, 10),
    rw(PART_YE, &["i", "e", "r"], &["i", "e", "m"], Left::Any, Right::End, 10),
    rw(PART_SHORT, &["u", "r"], &["ë"], Left::Consonant, Right::End, 12),
    rw(SCHWA, &["ë"], &[], Left::Consonant, Right::ConsonantThenFullVowel, 10),
    rw(SCHWA_FINAL, &["ë"], &[], Left::ConsonantAfterVowel, Right::End, 10),
    rw(CLUSTER_MB, &["m", "b"], &["m"], Left::Vowel, Right::VowelOrEnd, 11),
    rw(CLUSTER_ND, &["n", "d"], &["n"], Left::Vowel, Right::VowelOrEnd, 11),
    rw(F_H, &["h"], &["f"], Left::Vowel, Right::TOrEnd, 10),
    rw(O_A, &["a"], &["o"], Left::Is("v"), Right::Any, 10),
    rw(UNROUNDING, &["y"], &["i"], Left::Consonant, Right::Any, 12),
    rw(NJI, &["nj", "ë"], &["nj", "i"], Left::Any, Right::Any, 8),
    rw(VERB_NJ, &["j"], &["nj"], Left::Is("i"), Right::End, 12),
    rw(VERB_NJ, &["j"], &["nj"], Left::Is("a"), Right::End, 12),
];

/// Rregullat toske; një fjalë është ose gege ose toske, kurrë të dyja.
const TOSK: [&str; 1] = [TOSK_SCHWA];

/// Rregullat që vlejnë për çdo dialekt (drejtshkrimi, tabela).
const SHARED: [&str; 3] = [SPELLING, SPELLING_Z, TABLE];

/// Rregullat e dobëta. Rregulli është i qëndrueshëm vetëm kur mbi ndarjen `Dev` ka të paktën
/// `MIN_N` përgjigje dhe kufiri i poshtëm Wilson 95% i saktësisë së tij arrin `MIN_WILSON`, pra
/// rregulli ka më shumë gjasë të ketë të drejtë se gabim (tabela e rregullave në matjen e arit).
/// Rregulli i dobët mbetet dëshmi në `scored`, pa përgjigje. Rregullat e pjesores nuk varen nga ky
/// prag: dëshmitari i tyre është klasa e foljes në leksik.
pub const WEAK: [&str; 15] = [
    NASAL_LOW,
    NASAL_NJ,
    NASAL_FINAL,
    SCHWA,
    CLUSTER_MB,
    CLUSTER_ND,
    O_A,
    UNROUNDING,
    VELAR,
    NORTH_PALATAL,
    NJI,
    VERB_NJ,
    SPELLING,
    SPELLING_Z,
    PROTHESIS,
];

/// Pragu i përgjigjeve në `Dev` për një rregull të qëndrueshëm.
pub const MIN_N: usize = 5;

/// Pragu i kufirit të poshtëm Wilson 95% për një rregull të qëndrueshëm.
pub const MIN_WILSON: f64 = 0.5;

/// Rruga mbështetet vetëm në rregulla të qëndrueshme.
pub fn firm(rules: &[&'static str]) -> bool {
    rules.iter().all(|r| !WEAK.contains(r))
}

/// Një rrugë nuk përzien rregulla gege me rregulla toske.
fn one_dialect(used: &[&'static str]) -> bool {
    let tosk = used.iter().any(|r| TOSK.contains(r));
    let gheg = used.iter().any(|r| !TOSK.contains(r) && !SHARED.contains(r));
    !(tosk && gheg)
}

/// Kufijtë e kërkimit: numri i rregullave, kostoja e përgjithshme, rikthimi i «ë»-së midis
/// bashkëtingëlloreve. «Ë»-ja fundore e rënë nuk rikthehet: trajta me «-ë» është zakonisht
/// fjalë tjetër (lip/lipë, ves/vesë, uj/ujë).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub apps: usize,
    pub cost: u32,
    pub schwa: bool,
}

/// Rruga më e lirë deri te çdo trajtë: kostoja dhe rregullat e zbatuara.
pub type Paths = BTreeMap<String, (u32, Vec<&'static str>)>;

struct Walk<'a> {
    src: &'a [String],
    rules: &'a [Rewrite],
    limits: Limits,
    found: Paths,
}

/// Të gjitha trajtat që dalin nga `src` me të paktën një rregull, brenda kufijve.
pub fn expand(src: &[String], rules: &[Rewrite], limits: Limits) -> Paths {
    let mut walk = Walk { src, rules, limits, found: BTreeMap::new() };
    walk.step(0, &mut Vec::new(), 0, &mut Vec::new(), false);
    walk.found
}

impl Walk<'_> {
    fn room(&self, cost: u32, used: &[&'static str], add: u32) -> bool {
        used.len() < self.limits.apps && cost + add <= self.limits.cost
    }

    fn record(&mut self, out: &[String], cost: u32, used: &[&'static str]) {
        let text: String = out.concat();
        let better = self.found.get(&text).map(|(c, u)| (cost, used.len()) < (*c, u.len())).unwrap_or(true);
        match better && !used.is_empty() && one_dialect(used) {
            true => {
                self.found.insert(text, (cost, used.to_vec()));
            }
            false => {}
        }
    }

    fn step(&mut self, i: usize, out: &mut Vec<String>, cost: u32, used: &mut Vec<&'static str>, gap_done: bool) {
        let src = self.src;
        let rules = self.rules;
        match src.get(i) {
            None => self.record(out, cost, used),
            Some(cur) => {
                let prev = i.checked_sub(1).and_then(|k| src.get(k));
                let gap = prev.filter(|p| is_consonant(p) && is_consonant(cur)).map(|p| schwa_cost(p, cur));
                match (self.limits.schwa && !gap_done, gap) {
                    (true, Some(add)) => match self.room(cost, used, add) {
                        true => {
                            out.push("ë".to_string());
                            used.push(SCHWA);
                            self.step(i, out, cost + add, used, true);
                            out.pop();
                            used.pop();
                        }
                        false => {}
                    },
                    _ => {}
                }
                out.push(cur.clone());
                self.step(i + 1, out, cost, used, false);
                out.pop();
                for r in rules.iter().filter(|r| r.at(src, i)) {
                    match self.room(cost, used, r.cost) {
                        true => {
                            let keep = out.len();
                            out.extend(r.to.iter().map(|s| s.to_string()));
                            used.push(r.name);
                            self.step(i + r.from.len(), out, cost + r.cost, used, false);
                            out.truncate(keep);
                            used.pop();
                        }
                        false => {}
                    }
                }
            }
        }
    }
}
