//! Morfologjia shqipe e kufizuar për realizimin e shprehjes.
//!
//! Burimi i parë është gjithmonë forma e ATESTUAR në UD (CoNLL-U). Kur forma
//! mungon, përdoret një rregull i deklaruar dhe forma etiketohet si e tillë;
//! kur as rregulli nuk është i sigurt, përdoret forma e cituar «lemë».
//! Asnjë formë nuk paraqitet si e atestuar kur nuk është.

use crate::graph::Source;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MorphEntry {
    pub lemma: String,
    pub form: String,
    pub upos: String,
    pub feats: String,
    pub sent_id: String,
    pub source: Source,
}

#[derive(Default)]
pub struct Morph {
    pub entries: Vec<MorphEntry>,
    by_lemma: HashMap<String, Vec<usize>>,
    by_form: HashMap<String, Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Masc,
    Fem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormOrigin {
    /// Forma u gjet e anotuar në UD.
    Attested { source: Source, sent_id: String },
    /// Forma u ndërtua nga rregulli `rule`; gjinia është e atestuar ose e hamendësuar.
    Rule { rule: &'static str, gender_attested: bool },
    /// Forma e cituar e lemës, në thonjëza, pa lakim.
    Citation,
}

impl FormOrigin {
    pub fn label_sq(&self) -> String {
        match self {
            FormOrigin::Attested { source, sent_id } => {
                format!("formë e atestuar ({} · {})", source.label(), sent_id)
            }
            FormOrigin::Rule { rule, gender_attested } => format!(
                "formë nga rregulli {rule} (gjinia {})",
                match gender_attested {
                    true => "e atestuar në UD",
                    false => "e hamendësuar nga mbaresa",
                }
            ),
            FormOrigin::Citation => "formë e cituar (pa lakim)".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Realized {
    pub text: String,
    pub origin: FormOrigin,
}

pub fn feat<'a>(feats: &'a str, key: &str) -> Option<&'a str> {
    feats
        .split('|')
        .find_map(|kv| kv.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
}

impl Morph {
    pub fn add(&mut self, e: MorphEntry) {
        let i = self.entries.len();
        self.by_lemma.entry(e.lemma.clone()).or_default().push(i);
        self.by_form.entry(e.form.to_lowercase()).or_default().push(i);
        self.entries.push(e);
    }

    /// Forma (e lakuar ose jo) është e njohur nga një burim morfologjik.
    pub fn knows_form(&self, form: &str) -> bool {
        self.by_form.contains_key(&form.to_lowercase())
    }

    pub fn knows_lemma(&self, lemma: &str) -> bool {
        self.by_lemma.contains_key(lemma)
    }

    pub fn entries_of(&self, lemma: &str) -> Vec<&MorphEntry> {
        self.by_lemma
            .get(lemma)
            .map(|v| v.iter().map(|&i| &self.entries[i]).collect())
            .unwrap_or_default()
    }

    /// Gjinia e emrit sipas shumicës së anotimeve UD.
    pub fn gender(&self, lemma: &str) -> Option<Gender> {
        let (m, f) = self.entries_of(lemma).iter().filter(|e| e.upos == "NOUN").fold(
            (0u32, 0u32),
            |(m, f), e| match feat(&e.feats, "Gender") {
                Some("Masc") => (m + 1, f),
                Some("Fem") => (m, f + 1),
                _ => (m, f),
            },
        );
        match (m, f) {
            (0, 0) => None,
            (m, f) => match m >= f {
                true => Some(Gender::Masc),
                false => Some(Gender::Fem),
            },
        }
    }

    /// Forma e shquar, emërore, njëjës (p.sh. qen → qeni, kafshë → kafsha).
    pub fn definite_nom_sg(&self, lemma: &str) -> Realized {
        let attested = self.entries_of(lemma).into_iter().find(|e| {
            e.upos == "NOUN"
                && feat(&e.feats, "Case") == Some("Nom")
                && feat(&e.feats, "Definite") == Some("Def")
                && feat(&e.feats, "Number") == Some("Sing")
        });
        match (attested, lemma.contains(' ')) {
            (Some(e), _) => Realized {
                text: e.form.to_lowercase(),
                origin: FormOrigin::Attested { source: e.source, sent_id: e.sent_id.clone() },
            },
            (None, true) => Realized { text: format!("«{lemma}»"), origin: FormOrigin::Citation },
            (None, false) => {
                let known = self.gender(lemma);
                let g = known.or_else(|| guess_gender(lemma));
                match g.and_then(|g| rule_definite(lemma, g)) {
                    Some((text, rule)) => Realized {
                        text,
                        origin: FormOrigin::Rule { rule, gender_attested: known.is_some() },
                    },
                    None => Realized { text: format!("«{lemma}»"), origin: FormOrigin::Citation },
                }
            }
        }
    }

    /// Lemat e atestuara (UD, tabelat) për një trajtë, pa rregullat e heqjes së mbaresave.
    pub fn attested_lemmas(&self, form: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .by_form
            .get(&form.to_lowercase())
            .map(|ix| ix.iter().map(|&i| self.entries[i].lemma.clone()).collect())
            .unwrap_or_default();
        out.sort();
        out.dedup();
        out
    }

    /// Lemat e mundshme për një formë të shtypur nga përdoruesi.
    /// Renditja: forma e atestuar → heqje mbaresash me rregull.
    /// Thirrësi i filtron sipas leksikonit; këtu nuk shpiket ekzistenca.
    pub fn lemma_candidates(&self, form: &str) -> Vec<(String, &'static str)> {
        let f = form.to_lowercase();
        let mut out: Vec<(String, &'static str)> = vec![(f.clone(), "forma si lemë")];
        match self.by_form.get(&f) {
            Some(ix) => {
                for &i in ix {
                    let l = self.entries[i].lemma.clone();
                    match out.iter().any(|(x, _)| *x == l) {
                        true => {}
                        false => out.push((l, "formë e atestuar në UD")),
                    }
                }
            }
            None => {}
        }
        for (suffix, repl, rule) in STRIP_RULES {
            match f.strip_suffix(suffix) {
                Some(stem) => match stem.chars().count() >= 2 {
                    true => {
                        let cand = format!("{stem}{repl}");
                        match out.iter().any(|(x, _)| *x == cand) {
                            true => {}
                            false => out.push((cand, rule)),
                        }
                    }
                    false => {}
                },
                None => {}
            }
        }
        out
    }
}

/// Lemat e mundshme vetëm nga rregullat e heqjes së mbaresave (pa leksikon).
pub fn strip_candidates(form: &str) -> Vec<String> {
    let f = form.to_lowercase();
    STRIP_RULES
        .iter()
        .filter_map(|(suffix, repl, _)| f.strip_suffix(suffix).map(|stem| (stem.to_string(), repl)))
        .filter(|(stem, _)| stem.chars().count() >= 2)
        .map(|(stem, repl)| format!("{stem}{repl}"))
        .collect()
}

/// (mbaresa, zëvendësimi, emri i rregullit) — rregulla heqjeje për formën e shquar.
const STRIP_RULES: &[(&str, &str, &str)] = &[
    ("ja", "e", "R-L1 -ja → -e (lulja → lule)"),
    ("ia", "i", "R-L2 -ia → -i (shtëpia → shtëpi)"),
    ("ri", "ër", "R-L3 -ri → -ër (libri → libër)"),
    ("a", "ë", "R-L4 -a → -ë (kafsha → kafshë)"),
    ("u", "", "R-L5 -u → ∅ (miku → mik)"),
    ("i", "", "R-L6 -i → ∅ (qeni → qen)"),
    ("i", "ë", "R-L7 -i → -ë (djali → djalë)"),
    ("in", "", "R-L8 -in → ∅ (qenin → qen)"),
    ("në", "ë", "R-L9 -në → -ë (kafshën → kafshë)"),
    ("t", "", "R-L10 -t → ∅ (shumës i shquar)"),
];

fn guess_gender(lemma: &str) -> Option<Gender> {
    let last = lemma.chars().last()?;
    match last {
        'ë' | 'e' => Some(Gender::Fem),
        'i' | 'a' | 'o' | 'u' | 'y' => None,
        _ => Some(Gender::Masc),
    }
}

/// Rregullat e shquarjes (emërore njëjës). Kthen None kur rregulli nuk është i sigurt.
fn rule_definite(lemma: &str, g: Gender) -> Option<(String, &'static str)> {
    let chars: Vec<char> = lemma.chars().collect();
    let n = chars.len();
    let last = *chars.last()?;
    let prev = match n >= 2 {
        true => Some(chars[n - 2]),
        false => None,
    };
    let stem = |k: usize| chars[..n - k].iter().collect::<String>();
    match (g, last, prev) {
        (Gender::Masc, 'r', Some('ë')) => Some((format!("{}ri", stem(2)), "R-D1 m. -ër → -ri")),
        (Gender::Masc, 'k' | 'g' | 'h', _) => Some((format!("{lemma}u"), "R-D2 m. -k/-g/-h → +u")),
        (Gender::Masc, 'ë', _) => Some((format!("{}i", stem(1)), "R-D3 m. -ë → -i")),
        (Gender::Masc, 'a' | 'e' | 'i' | 'o' | 'u' | 'y', _) => None,
        (Gender::Masc, _, _) => Some((format!("{lemma}i"), "R-D4 m. bashkëtingëllore → +i")),
        (Gender::Fem, 'ë', _) => Some((format!("{}a", stem(1)), "R-D5 f. -ë → -a")),
        (Gender::Fem, 'e', _) => Some((format!("{}ja", stem(1)), "R-D6 f. -e → -ja")),
        (Gender::Fem, 'i', _) => Some((format!("{lemma}a"), "R-D7 f. -i → +a")),
        (Gender::Fem, _, _) => None,
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some('«') => match c.next() {
            Some(f) => format!("«{}{}", f.to_uppercase(), c.as_str()),
            None => "«".into(),
        },
        Some(f) => format!("{}{}", f.to_uppercase(), c.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn morph_with(lemma: &str, form: &str, feats: &str) -> Morph {
        let mut m = Morph::default();
        m.add(MorphEntry {
            lemma: lemma.into(),
            form: form.into(),
            upos: "NOUN".into(),
            feats: feats.into(),
            sent_id: "S1".into(),
            source: Source::UdSqStaf,
        });
        m
    }

    #[test]
    fn attested_form_wins_over_rule() {
        let m = morph_with("nënë", "Nëna", "Case=Nom|Definite=Def|Gender=Fem|Number=Sing");
        let r = m.definite_nom_sg("nënë");
        assert_eq!(r.text, "nëna");
        assert!(matches!(r.origin, FormOrigin::Attested { .. }));
    }

    #[test]
    fn rules_with_attested_gender() {
        let m = morph_with("burg", "burg", "Case=Acc|Definite=Ind|Gender=Masc|Number=Sing");
        let r = m.definite_nom_sg("burg");
        assert_eq!(r.text, "burgu");
        assert_eq!(r.origin, FormOrigin::Rule { rule: "R-D2 m. -k/-g/-h → +u", gender_attested: true });
    }

    #[test]
    fn rules_with_guessed_gender_are_marked_as_guessed() {
        let m = Morph::default();
        assert_eq!(m.definite_nom_sg("qen").text, "qeni");
        assert_eq!(m.definite_nom_sg("kafshë").text, "kafsha");
        assert_eq!(m.definite_nom_sg("lule").text, "lulja");
        assert!(matches!(
            m.definite_nom_sg("qen").origin,
            FormOrigin::Rule { gender_attested: false, .. }
        ));
    }

    #[test]
    fn uncertain_and_multiword_fall_back_to_citation() {
        let m = Morph::default();
        assert_eq!(m.definite_nom_sg("baba").origin, FormOrigin::Citation);
        assert_eq!(m.definite_nom_sg("i gjithë").text, "«i gjithë»");
    }

    #[test]
    fn lemma_candidates_include_rule_stripped_forms() {
        let m = Morph::default();
        let c: Vec<String> = m.lemma_candidates("Kafsha").into_iter().map(|x| x.0).collect();
        assert!(c.contains(&"kafshë".to_string()));
        let c: Vec<String> = m.lemma_candidates("qeni").into_iter().map(|x| x.0).collect();
        assert!(c.contains(&"qen".to_string()));
    }

    #[test]
    fn capitalize_handles_quotes() {
        assert_eq!(capitalize("qeni"), "Qeni");
        assert_eq!(capitalize("«zi»"), "«Zi»");
    }
}
