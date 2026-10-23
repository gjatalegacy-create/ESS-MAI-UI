//! Leksiku gramatikor: lema, klasa e paradigmës, temat dhe gjinia, nga
//! ekstrakti TSV i uniparser-grammar-albanian 2.3.0 (MIT — Morozova, Rusakov,
//! Arkhangelskiy). Ky është dëshmitari leksikor i gramatikës; trajtat e
//! Wiktionary-t, UniMorph-it dhe UD-së mbeten vetëm për matje.

use crate::gramatika::tipet::{Gender, Number, Pos};
use std::collections::HashMap;

/// Shtegu relativ ndaj dosjes `data`.
pub const TSV: &str = "raw/uniparser/uniparser-sq.tsv";

const COLS: usize = 18;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexeme {
    pub lemma: String,
    pub pos: Pos,
    /// Gjinia në njëjës dhe në shumës («mf»: mashkullore në njëjës, femërore në shumës).
    pub gender_sg: Option<Gender>,
    pub gender_pl: Option<Gender>,
    pub animate: Option<bool>,
    /// Klasat e paradigmës, p.sh. «1a», «5b», «lexoj», «adj-2-2».
    pub paradigms: Vec<String>,
    /// Temat e papërpunuara, të ndara me «|»; variantet brenda një teme me «//».
    pub stems: Vec<String>,
    pub standard: bool,
    pub flags: Vec<String>,
    /// Lema standarde, kur kjo është variant.
    pub lexref: Option<String>,
    /// Forma e shquar emërore njëjës, kur paradigma u validua.
    pub def_nom_sg: Option<String>,
}

impl Lexeme {
    pub fn gender(&self, n: Number) -> Option<Gender> {
        match n {
            Number::Sing => self.gender_sg,
            Number::Plur => self.gender_pl,
        }
    }

    /// Temat e pastra: pa pikat e fundit, pa «NOSTEM», variantet e para.
    pub fn clean_stems(&self) -> Vec<Vec<String>> {
        self.stems
            .iter()
            .map(|s| s.split("//").map(|v| v.trim_matches('.').to_string()).filter(|v| !v.is_empty() && v != "NOSTEM").collect())
            .collect()
    }
}

fn val(s: &str) -> Option<&str> {
    match s.trim() {
        "" | "-" => None,
        v => Some(v),
    }
}

fn pos_of(upos: &str) -> Pos {
    match upos {
        "NOUN" => Pos::Noun,
        "PROPN" => Pos::Propn,
        "A" | "ADJ" => Pos::Adj,
        "V" | "VERB" => Pos::Verb,
        "ADV" => Pos::Adv,
        "PRON" => Pos::Pron,
        "DET" => Pos::Det,
        "NUM" => Pos::Num,
        "ADP" | "PREP" => Pos::Adp,
        "CONJ" | "CCONJ" | "SCONJ" => Pos::Conj,
        "PART" => Pos::Part,
        "INTJ" => Pos::Intj,
        _ => Pos::Other,
    }
}

fn genders(g: &str) -> (Option<Gender>, Option<Gender>) {
    match g {
        "m" => (Some(Gender::Masc), Some(Gender::Masc)),
        "f" => (Some(Gender::Fem), Some(Gender::Fem)),
        "n" => (Some(Gender::Neut), Some(Gender::Neut)),
        "mf" => (Some(Gender::Masc), Some(Gender::Fem)),
        _ => (None, None),
    }
}

pub fn parse_line(line: &str) -> Option<Lexeme> {
    let c: Vec<&str> = line.split('\t').collect();
    match (c.len() == COLS, line.starts_with('#') || line.starts_with("lex\t")) {
        (true, false) => {
            let (gender_sg, gender_pl) = genders(c[4].trim());
            Some(Lexeme {
                lemma: c[1].trim().to_string(),
                pos: pos_of(c[2].trim()),
                gender_sg,
                gender_pl,
                animate: match c[5].trim() {
                    "anim" => Some(true),
                    "inanim" => Some(false),
                    _ => None,
                },
                paradigms: val(c[11]).map(|p| p.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default(),
                stems: val(c[12]).map(|s| s.split('|').map(str::to_string).collect()).unwrap_or_default(),
                standard: c[10].trim() == "std",
                flags: val(c[9]).map(|f| f.split(',').map(str::to_string).collect()).unwrap_or_default(),
                lexref: val(c[13]).map(str::to_string),
                def_nom_sg: val(c[15]).map(str::to_string),
            })
        }
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct Lexicon {
    by_lemma: HashMap<String, Vec<Lexeme>>,
}

impl Lexicon {
    pub fn parse(text: &str) -> Lexicon {
        let mut by_lemma: HashMap<String, Vec<Lexeme>> = HashMap::new();
        for lx in text.lines().filter_map(parse_line) {
            by_lemma.entry(lx.lemma.clone()).or_default().push(lx);
        }
        Lexicon { by_lemma }
    }

    pub fn load(data_dir: &std::path::Path) -> std::io::Result<Lexicon> {
        std::fs::read_to_string(data_dir.join(TSV)).map(|t| Lexicon::parse(&t))
    }

    pub fn entries(&self, lemma: &str) -> &[Lexeme] {
        self.by_lemma.get(lemma).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Leksema standarde e parë e një pjese ligjërate.
    pub fn standard(&self, lemma: &str, pos: Pos) -> Option<&Lexeme> {
        let all = self.entries(lemma);
        all.iter().find(|l| l.pos == pos && l.standard).or_else(|| all.iter().find(|l| l.pos == pos))
    }

    pub fn len(&self) -> usize {
        self.by_lemma.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_lemma.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Lexeme> {
        self.by_lemma.values().flatten()
    }
}
