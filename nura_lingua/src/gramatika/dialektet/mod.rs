//! Dialektet: përkimet e rregullta ndërmjet gegërishtes, toskërishtes dhe gjuhës standarde.
//!
//! Rendi i dëshmisë, i fiksuar:
//! 1. fjala që leksiku e njeh si standarde (lemë, ose temë + mbaresë standarde e klasës së saj)
//!    nuk normalizohet: kthehet Hold, sepse trajta gege që përkon me një trajtë standarde
//!    (vena, kthyem, hapë, mikun) nuk zgjidhet pa fjalinë;
//! 2. tabela e mbyllur e trajtave gege që nuk dalin nga rregulli (pjesoret e shkurtra «bâ»,
//!    «vû», «râ»; «âsht», «tue», «shpi»), secila me lemën standarde që e dëshmon;
//! 3. rregullat e përkimit, secili me emër dhe kosto: rotacizmi (zâni/zëri, gjuni/gjuri, vena/vera),
//!    zanoret hundore (nânë/nënë), diftongu «ue/ua» (grue/grua), pjesoret «-un/-uem/-ue/-ye/-ë»
//!    (hapun/hapur, punue/punuar, kthye/kthyer, ardhë/ardhur), rënia e «ë»-së së patheksuar
//!    (knaq/kënaq), thjeshtimi i grupeve «mb/nd» pas hundores (mênoj/mendoj), «f/h» (njof/njoh),
//!    përkimet toske dhe drejtshkrimi i vjetër; rregullat gege dhe toske nuk përzihen në një rrugë;
//! 4. leksiku i gramatikës (uniparser-grammar-albanian 2.3.0, MIT) si dëshmitar: mbahen vetëm
//!    kandidatët që janë lema standarde; pjesoret gege pranohen vetëm kur kandidati është
//!    pjesore standarde e një foljeje të leksikut sipas klasës së saj.
//!
//! Renditja: kosto e rregullave, numri i tyre, largësia e Levenshtein-it, rendi alfabetik.
//! `to_standard` përgjigjet vetëm kur rruga më e lirë nga të gjitha është një rregull i vetëm
//! i qëndrueshëm dhe asnjë lemë tjetër nuk del brenda `MARGIN` kostoje; rregullat e dobëta
//! (`WEAK`) dhe zinxhirët e rregullave mbeten dëshmi në `scored`.
//! Fjala me shkronjë të madhe pranon vetëm lemë me shkronjë të madhe (emër i përveçëm);
//! shkronjën e madhe në krye të fjalisë e ul fillimisht shtresa e fjalisë.
//! Kur asnjë kandidat nuk dëshmohet, lista mbetet bosh: asnjë hamendje.
//! Wiktionary, UniMorph dhe UD nuk lexohen këtu; shërbejnë vetëm për matje.

mod paradigmat;
mod rregullat;
mod shkronjat;
mod trajtat;

pub use rregullat::{
    CLUSTER_MB, CLUSTER_ND, DIPHTHONG, F_H, INFINITIVE, LEXREF, LONG, MIN_N, MIN_WILSON, MONOPHTHONG, NASAL, NASAL_FINAL, NASAL_LOW, NASAL_NJ, NJI,
    NORTH_PALATAL, O_A, PART_SHORT, PART_UE, PART_UEM, PART_UN, PART_YE, PROTHESIS, ROTACISM, ROTACISM_SCHWA, SCHWA, SCHWA_A, SCHWA_FINAL, SPELLING,
    SPELLING_Z, TABLE, TOSK_SCHWA, UNROUNDING, VELAR, VERB_NJ, WEAK,
};
pub use shkronjat::{fold, letters, levenshtein, sonority};
pub use trajtat::Stems;

use crate::gramatika::leksiku::Lexicon;
use crate::gramatika::tipet::Pos;
use rregullat::{expand, Limits, TO_GHEG, TO_STANDARD};
use std::collections::BTreeSet;
use trajtat::Source;

/// Një lemë standarde e mundshme dhe rregullat që e nxjerrin nga trajta dialektore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub rules: Vec<&'static str>,
}

/// Trajtat gege që nuk dalin nga rregulli: (trajta gege, trajta standarde, lema standarde dëshmuese).
/// «zgedhje» dhe «shpnesë» janë fjalë të vetme me përkim të veçantë (g/gj pas z, r/n pas shp):
/// qëndrojnë këtu, jo si rregull.
const IRREGULAR: &[(&str, &str, &str)] = &[
    ("âsht", "është", "jam"),
    ("asht", "është", "jam"),
    ("â", "është", "jam"),
    ("osht", "është", "jam"),
    ("bâ", "bërë", "bërë"),
    ("ba", "bërë", "bërë"),
    ("pa", "parë", "parë"),
    ("vû", "vënë", "vënë"),
    ("vu", "vënë", "vënë"),
    ("râ", "rënë", "rënë"),
    ("kênë", "qenë", "qenë"),
    ("ndejë", "ndenjur", "ndenjur"),
    ("hangër", "ngrënë", "ngrënë"),
    ("tue", "duke", "duke"),
    ("tu", "duke", "duke"),
    ("shpi", "shtëpi", "shtëpi"),
    ("çka", "çfarë", "çfarë"),
    ("kurrgjâ", "asgjë", "asgjë"),
    ("kurgjâ", "asgjë", "asgjë"),
    ("zgedhje", "zgjedhje", "zgjedhje"),
    ("shpnesë", "shpresë", "shpresë"),
];

/// Madhësia e tabelës së parregullt, për raportin e matjes.
pub fn irregular_len() -> usize {
    IRREGULAR.len()
}

/// A e përmend tabela e parregullt këtë trajtë (gege ose standarde)?
pub fn in_irregular(word: &str) -> bool {
    let w = fold(word);
    IRREGULAR.iter().any(|(g, s, l)| *g == w || *s == w || *l == w)
}

const TABLE_COST: u32 = 5;

/// Dy lema të ndryshme me kosto që ndryshon jo më shumë se kaq janë të barabarta: rregulli nuk vendos.
pub const MARGIN: u32 = 2;

/// Fjalët më të gjata se kaq nuk janë lema; kërkimi nuk hapet.
const MAX_LETTERS: usize = 30;

const STANDARD_LIMITS: Limits = Limits { apps: 3, cost: 36, schwa: true };
const GHEG_LIMITS: Limits = Limits { apps: 3, cost: 36, schwa: false };

/// Sa trajta gege kthehen për një fjalë standarde.
const MAX_GHEG: usize = 48;

/// Rregullat e pjesores: kandidati duhet të jetë pjesore standarde e një foljeje të leksikut.
const PARTICIPLE: [&str; 5] = [PART_UN, PART_UEM, PART_UE, PART_YE, PART_SHORT];

/// Rotacizmi në drejtimin standard → gegë: «r»-ja standarde vjen nga «n» vetëm në një pjesë
/// të fjalëve, që leksiku nuk i shënon; këto trajta renditen pas të tjerave.
const SPECULATIVE: [&str; 2] = [ROTACISM, ROTACISM_SCHWA];

fn is_standard(lex: &Lexicon, lemma: &str) -> bool {
    lex.entries(lemma).iter().any(|l| l.standard)
}

fn capitalized(s: &str) -> String {
    let mut cs = s.chars();
    cs.next().map(|c| c.to_uppercase().chain(cs).collect()).unwrap_or_default()
}

fn starts_upper(s: &str) -> bool {
    s.trim().chars().next().map(char::is_uppercase).unwrap_or(false)
}

/// Lema standarde e dëshmuar: me shkronjë të madhe kur fjala hyrëse e kishte, përndryshe e vogël.
fn attested(lex: &Lexicon, text: &str, capital: bool) -> Option<String> {
    let form = match capital {
        true => capitalized(text),
        false => text.to_string(),
    };
    is_standard(lex, &form).then_some(form)
}

/// Fjala është vetë lemë standarde e leksikut.
fn own_lemma(lex: &Lexicon, word: &str) -> bool {
    let folded = fold(word.trim());
    is_standard(lex, &folded) || (starts_upper(word) && is_standard(lex, &capitalized(&folded)))
}

/// Të gjithë kandidatët e dëshmuar me koston e rrugës më të lirë, të renditur, pa politikën e mbajtjes.
pub fn scored(word: &str, lex: &Lexicon) -> Vec<(u32, Candidate)> {
    let folded = fold(word.trim());
    let capital = starts_upper(word);
    let src = letters(&folded);
    let paths = match src.len() <= MAX_LETTERS {
        true => expand(&src, TO_STANDARD, STANDARD_LIMITS),
        false => Default::default(),
    };
    let table = IRREGULAR
        .iter()
        .filter(|(g, _, lemma)| *g == folded && is_standard(lex, lemma))
        .map(|(_, s, _)| (TABLE_COST, 1, 0, Candidate { text: s.to_string(), rules: vec![TABLE] }));
    let ruled = paths.into_iter().filter(|(text, _)| *text != folded).filter_map(|(text, (cost, rules))| {
        attested(lex, &text, capital).map(|t| (cost, rules.len(), levenshtein(&folded, &text), Candidate { text: t, rules }))
    });
    let mut ranked: Vec<(u32, usize, usize, Candidate)> = table.chain(ruled).collect();
    ranked.sort_by(|a, b| (a.0, a.1, a.2, &a.3.text).cmp(&(b.0, b.1, b.2, &b.3.text)));
    let mut seen = BTreeSet::new();
    ranked.into_iter().filter(|(_, _, _, c)| seen.insert(c.text.clone())).map(|(cost, _, _, c)| (cost, c)).collect()
}

/// Kandidati mbështetet vetëm në rregulla të qëndrueshme (asnjë nga `WEAK`).
pub fn is_firm(c: &Candidate) -> bool {
    rregullat::firm(&c.rules)
}

/// Rregulli është i qëndrueshëm (nuk është në `WEAK`).
fn not_weak(r: &'static str) -> bool {
    !WEAK.contains(&r)
}

/// Mbaresat e pjesores standarde.
const PARTICIPLE_ENDINGS: [&str; 6] = ["ur", "ar", "er", "rë", "në", "ë"];

/// Rregulli i pjesores gege sipas mbaresës së saj.
fn participle_rule(w: &str) -> &'static str {
    let by_end: [(&str, &'static str); 9] = [
        ("uem", PART_UEM),
        ("yem", PART_YE),
        ("iem", PART_YE),
        ("ue", PART_UE),
        ("ye", PART_YE),
        ("ie", PART_YE),
        ("un", PART_UN),
        ("m", PART_UEM),
        ("ë", PART_SHORT),
    ];
    by_end.iter().find(|(e, _)| w.ends_with(e)).map(|(_, r)| *r).unwrap_or(PART_SHORT)
}

/// Normalizuesi: leksiku dëshmitar dhe, kur kërkohet shpejtësi, treguesi i temave të tij.
/// `Normalizer::new` e ndërton treguesin një herë (për tekst); `Normalizer::scan` nuk e ndërton
/// dhe çdo analizë e trajtës kalon nëpër gjithë leksikun (për pyetje të rralla).
#[derive(Debug)]
pub struct Normalizer<'a> {
    lex: &'a Lexicon,
    stems: Option<Stems<'a>>,
}

impl<'a> Normalizer<'a> {
    pub fn new(lex: &'a Lexicon) -> Normalizer<'a> {
        Normalizer { lex, stems: Some(Stems::new(lex)) }
    }

    pub fn scan(lex: &'a Lexicon) -> Normalizer<'a> {
        Normalizer { lex, stems: None }
    }

    fn source(&self) -> Source<'_> {
        match &self.stems {
            Some(s) => Source::Index(s),
            None => Source::Scan(self.lex),
        }
    }

    /// Fjala është standarde sipas leksikut: lemë, ose temë e një leme standarde me mbaresë
    /// standarde të klasës së saj (shtëpisë, mikun, kthyem, punonte).
    pub fn is_standard_form(&self, word: &str) -> bool {
        own_lemma(self.lex, word) || trajtat::standard_form(self.source(), &fold(word.trim()))
    }

    /// Trajta dialektore → lemat standarde të dëshmuara në leksik, të renditura; bosh = Hold.
    pub fn to_standard(&self, word: &str) -> Vec<Candidate> {
        self.to_standard_by(word, &not_weak)
    }

    /// Politika e përgjigjes me një gjykim të dhënë për qëndrueshmërinë e rregullave (`firm`).
    /// Kandidati i një rregulli të pjesores lexohet vetëm kur është pjesore standarde e një foljeje
    /// të leksikut sipas klasës së saj. Përgjigjet vetëm kur: fjala nuk është standarde; rruga më e
    /// lirë nga të gjitha është një rregull i vetëm i qëndrueshëm; asnjë lemë tjetër nuk del brenda
    /// `MARGIN`. Përndryshe Hold.
    pub fn to_standard_by(&self, word: &str, firm: &dyn Fn(&'static str) -> bool) -> Vec<Candidate> {
        let folded = fold(word.trim());
        let all = match own_lemma(self.lex, word) {
            true => Vec::new(),
            false => scored(word, self.lex),
        };
        let reading = |c: &Candidate| !c.rules.iter().any(|r| PARTICIPLE.contains(r)) || trajtat::standard_participle(self.source(), &c.text);
        let s: Vec<(u32, Candidate)> = all.into_iter().filter(|(_, c)| reading(c)).collect();
        let single = |c: &Candidate| c.rules.len() == 1 && c.rules.iter().all(|r| firm(r));
        let clear = s.first().filter(|(cost, c)| single(c) && s.iter().skip(1).all(|(k, _)| *k > cost + MARGIN));
        let answer = clear.filter(|_| !trajtat::standard_form(self.source(), &folded));
        match answer {
            Some(_) => s.into_iter().map(|(_, c)| c).filter(|c| single(c)).collect(),
            None => Vec::new(),
        }
    }

    /// Pjesore standarde e dëshmuar: lemë standarde, mbiemër me mbaresë pjesoreje ose pjesore e
    /// një foljeje standarde sipas klasës së saj (qenë nga «jam»).
    fn standard_participle(&self, w: &str) -> bool {
        let entries = self.lex.entries(w);
        let shape = PARTICIPLE_ENDINGS.iter().any(|e| w.ends_with(e));
        let adjective = shape && entries.iter().any(|l| l.standard && l.pos == Pos::Adj);
        adjective || (entries.iter().any(|l| l.standard) && trajtat::standard_participle(self.source(), w))
    }

    /// Pjesorja pas «me» (gege ose standarde) → pjesorja standarde, me rregullin që e jep.
    /// Rendi: pjesore standarde vetë; tabela; pjesorja gege e klasës së foljes në leksik (një
    /// përgjigje e vetme); rregullat e pjesores me kandidat pjesore standarde, pa rival brenda `MARGIN`.
    pub fn participle_to_standard(&self, part: &str) -> Option<Candidate> {
        let w = fold(part.trim());
        let direct = || self.standard_participle(&w).then(|| Candidate { text: w.clone(), rules: Vec::new() });
        let table = || {
            IRREGULAR
                .iter()
                .find(|(g, s, _)| *g == w && self.standard_participle(s))
                .map(|(_, s, _)| Candidate { text: s.to_string(), rules: vec![TABLE] })
        };
        let paradigm = || {
            let found: Vec<String> = trajtat::gheg_participle(self.source(), &w).into_iter().filter(|p| is_standard(self.lex, p)).collect();
            match found.as_slice() {
                [one] => Some(Candidate { text: one.clone(), rules: vec![participle_rule(&w)] }),
                _ => None,
            }
        };
        let ruled = || {
            let s: Vec<(u32, Candidate)> = scored(&w, self.lex)
                .into_iter()
                .filter(|(_, c)| c.rules.len() == 1 && c.rules.iter().all(|r| PARTICIPLE.contains(r)) && self.standard_participle(&c.text))
                .collect();
            let alone = match (s.first(), s.get(1)) {
                (Some((c1, _)), Some((c2, _))) => *c2 > c1 + MARGIN,
                _ => true,
            };
            s.into_iter().next().filter(|_| alone).map(|(_, c)| c)
        };
        direct().or_else(table).or_else(paradigm).or_else(ruled)
    }

    /// Paskajorja gege «me + pjesore» → «për të + pjesore» (me punue → për të punuar;
    /// me u kthye → për t'u kthyer). Jepet vetëm leximi i qëllimit («për të»); pas foljeve modale
    /// dhe fazore standardi do «të» + lidhore (due me shkue → dua të shkoj), që e zgjedh fjalia.
    pub fn infinitive_to_standard(&self, phrase: &str) -> Option<Candidate> {
        let low = fold(phrase);
        let words: Vec<&str> = low.split_whitespace().collect();
        let (reflexive, part) = match words.as_slice() {
            ["me", "u", p] => Some((true, *p)),
            ["me", p] => Some((false, *p)),
            _ => None,
        }?;
        let p = self.participle_to_standard(part)?;
        let head = match reflexive {
            true => "për t'u",
            false => "për të",
        };
        let mut rules = vec![INFINITIVE];
        rules.extend(p.rules);
        Some(Candidate { text: format!("{head} {}", p.text), rules })
    }
}

/// Fjala është standarde sipas leksikut (pa tregues temash; shih `Normalizer`).
pub fn is_standard_form(word: &str, lex: &Lexicon) -> bool {
    Normalizer::scan(lex).is_standard_form(word)
}

/// Trajta dialektore → lemat standarde të dëshmuara në leksik, të renditura; bosh = Hold.
pub fn to_standard(word: &str, lex: &Lexicon) -> Vec<Candidate> {
    Normalizer::scan(lex).to_standard(word)
}

/// `to_standard` me gjykim të dhënë për qëndrueshmërinë e rregullave.
pub fn to_standard_by(word: &str, lex: &Lexicon, firm: &dyn Fn(&'static str) -> bool) -> Vec<Candidate> {
    Normalizer::scan(lex).to_standard_by(word, firm)
}

/// Pjesorja pas «me» → pjesorja standarde (shih `Normalizer::participle_to_standard`).
pub fn participle_to_standard(part: &str, lex: &Lexicon) -> Option<Candidate> {
    Normalizer::scan(lex).participle_to_standard(part)
}

/// Paskajorja gege «me + pjesore» → «për të + pjesore» (shih `Normalizer::infinitive_to_standard`).
pub fn infinitive_to_standard(phrase: &str, lex: &Lexicon) -> Option<Candidate> {
    Normalizer::scan(lex).infinitive_to_standard(phrase)
}

/// Lema standarde që leksiku ia shënon vetë një varianti jostandard («lexref»).
pub fn declared(word: &str, lex: &Lexicon) -> Option<Candidate> {
    let w = word.trim();
    lex.entries(w)
        .iter()
        .filter(|l| !l.standard)
        .filter_map(|l| l.lexref.clone())
        .find(|s| s != w && is_standard(lex, s))
        .map(|text| Candidate { text, rules: vec![LEXREF] })
}

/// Fjala standarde → trajtat gege që japin rregullat (pa renditje kuptimore): më përpara ato pa
/// rotacizëm, pastaj sipas numrit të rregullave dhe kostos.
pub fn to_gheg(standard: &str) -> Vec<String> {
    let folded = fold(standard.trim());
    let src = letters(&folded);
    let paths = match src.len() <= MAX_LETTERS {
        true => expand(&src, TO_GHEG, GHEG_LIMITS),
        false => Default::default(),
    };
    let table = IRREGULAR.iter().filter(|(_, s, _)| *s == folded).map(|(g, _, _)| (0, 0, 0, g.to_string()));
    let mut ruled: Vec<(usize, usize, u32, String)> = paths
        .into_iter()
        .map(|(t, (c, r))| (r.iter().filter(|x| SPECULATIVE.contains(x)).count(), r.len(), c, t))
        .collect();
    ruled.sort();
    let mut seen = BTreeSet::new();
    table
        .chain(ruled)
        .map(|(_, _, _, t)| t)
        .filter(|t| *t != folded && letters(t).iter().any(|x| shkronjat::is_vowel(x)) && seen.insert(t.clone()))
        .take(MAX_GHEG)
        .collect()
}

/// Pjesorja standarde → pjesoret gege sipas rregullave të pjesores dhe tabelës.
fn participle_to_gheg(p: &str) -> Vec<String> {
    let table = IRREGULAR.iter().filter(|(_, s, _)| *s == p).map(|(g, _, _)| g.to_string());
    let endings: [(&str, &[&str]); 5] = [("uar", &["ue", "uem"]), ("yer", &["ye", "yem"]), ("ier", &["ie", "iem"]), ("ur", &["un", "ë"]), ("ë", &["ë"])];
    let ruled = endings.iter().find(|(e, _)| p.ends_with(e)).map(|(e, gs)| {
        let stem = p.strip_suffix(e).unwrap_or(p);
        gs.iter().map(|g| format!("{stem}{g}")).collect::<Vec<_>>()
    });
    let mut seen = BTreeSet::new();
    table.chain(ruled.unwrap_or_default()).filter(|g| seen.insert(g.clone())).collect()
}

/// Paskajorja standarde «për të + pjesore» → paskajoret gege «me + pjesore».
pub fn infinitive_to_gheg(phrase: &str) -> Vec<String> {
    let low = fold(phrase).replace('’', "'");
    let words: Vec<&str> = low.split_whitespace().collect();
    let parsed = match words.as_slice() {
        ["për", "t'u", p] => Some(("me u", *p)),
        ["për", "të", p] => Some(("me", *p)),
        _ => None,
    };
    parsed.map(|(head, p)| participle_to_gheg(p).into_iter().map(|g| format!("{head} {g}")).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests;
