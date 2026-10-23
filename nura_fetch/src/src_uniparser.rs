//! Shkarkuesi i burimit «uniparser»: leksiku gramatikor shqip i analizuesit
//! morfologjik `uniparser-grammar-albanian` (Morozova, Rusakov, Arkhangelskiy), MIT.
//!
//! Hapat (pa Python, pa LLM):
//! 1. shkarkon wheel-in 2.3.0 nga PyPI dhe JSON-in e PyPI-së; SHA-256 i wheel-it
//!    duhet të përputhet me `urls[].digests.sha256` të botuar (ndryshe: gabim);
//! 2. e lexon wheel-in si ZIP: `data_strict/lexemes.txt`, `paradigms.txt`,
//!    `derivations.txt` dhe `dist-info/LICENSE`; licenca duhet të jetë teksti MIT
//!    me emrat e autorëve dhe klasifikuesi i PyPI-së duhet të thotë MIT;
//! 3. nxjerr për çdo leksemë: lemën, rrënjët, POS, gjininë, gjallësinë,
//!    kalueshmërinë, mesoren, huazimin + gjuhën, flamujt (deriv, alb, Gheg, nonst…),
//!    paradigmat, `lexref`, `deriv`; `trans_en` mbahet VETËM si kolonë-urë angleze;
//! 4. formën e shquar emërore njëjës (p.sh. kalë → kali) e gjeneron nga paradigma
//!    vetëm kur rregulli është i njëkuptimtë, dhe e VALIDON kundrejt UniMorph sqi
//!    (`N;NOM;SG;DEF`). Paradigmat me < 95 % përputhje ose me pak mostra nuk gjenerojnë.
//!
//! Skedarët e shkruar (të gjithë në SOURCES.lock, burimi "uniparser"):
//! - `raw/uniparser/uniparser_albanian-2.3.0-py3-none-any.whl` (i papërpunuar)
//! - `raw/uniparser/LICENSE` (nga wheel-i)
//! - `raw/uniparser/unimorph-sqi.tsv` (vetëm për validim; CC BY-SA 3.0)
//! - `raw/uniparser/uniparser-defnom-validation.tsv` (raporti i validimit)
//! - `raw/uniparser/uniparser-sq.tsv` (ekstrakti që lexon importuesi)

use crate::common::{self, Fetched};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::Path;

pub const SOURCE: &str = "uniparser";
pub const VERSION: &str = "uniparser-albanian 2.3.0 (PyPI wheel, data_strict)";
pub const WHEEL_NAME: &str = "uniparser_albanian-2.3.0-py3-none-any.whl";
pub const WHEEL_URL: &str = "https://files.pythonhosted.org/packages/45/3e/da3ba84ce435426e0dd094897eb89cedfcf60aa9801cb80c8d25c6ba2d1e/uniparser_albanian-2.3.0-py3-none-any.whl";
pub const PYPI_JSON_URL: &str = "https://pypi.org/pypi/uniparser-albanian/2.3.0/json";
pub const UNIMORPH_URL: &str = "https://raw.githubusercontent.com/unimorph/sqi/master/sqi";
pub const LICENSE: &str = "MIT (dist-info/LICENSE i wheel-it; klasifikuesi PyPI «License :: OSI Approved :: MIT License»)";
pub const ATTRIBUTION: &str = "uniparser-grammar-albanian — Maria Morozova, Alexander Rusakov, Timofey Arkhangelskiy (2011–2025), PyPI uniparser-albanian 2.3.0";
pub const UNIMORPH_LICENSE: &str = "CC BY-SA 3.0 (README i unimorph/sqi) — përdoret VETËM për validim, format nuk importohen";
pub const UNIMORPH_ATTRIBUTION: &str = "UniMorph sqi (github.com/unimorph/sqi), nxjerrë nga Wiktionary";

pub const RAW_DIR: &str = "raw/uniparser";
/// Duhet të jetë identike me `nura_lingua::src_uniparser::TSV`.
pub const TSV_REL: &str = "raw/uniparser/uniparser-sq.tsv";
pub const VALIDATION_REL: &str = "raw/uniparser/uniparser-defnom-validation.tsv";

const ZIP_LEXEMES: &str = "uniparser_albanian/data_strict/lexemes.txt";
const ZIP_PARADIGMS: &str = "uniparser_albanian/data_strict/paradigms.txt";
const ZIP_DERIVATIONS: &str = "uniparser_albanian/data_strict/derivations.txt";
const ZIP_LICENSE: &str = "uniparser_albanian-2.3.0.dist-info/LICENSE";

/// Pragu i përputhjes për të gjeneruar formën e shquar nga një paradigmë.
pub const MIN_AGREEMENT_PCT: usize = 95;
/// Numri minimal i mostrave UniMorph që një paradigmë të quhet e validuar.
pub const MIN_SAMPLES: usize = 5;

pub const TSV_HEADER: &str = "lex\tlemma\tupos\tpos\tgender\tanim\ttrans\tmed\tborr\tflags\tstatus\tparadigms\tstems\tlexref\tderiv\tdef_nom_sg\tdef_check\ttrans_en_bridge";

// ───────────────────────────── modelet ─────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lexeme {
    pub lex: String,
    pub stem: String,
    pub gramm: Vec<String>,
    pub paradigms: Vec<String>,
    pub trans_en: String,
    pub lexref: String,
    pub deriv: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flex {
    pub flex: String,
    pub gramm: Vec<String>,
    /// Flex-i vazhdon me një paradigmë tjetër (`paradigm:` brenda flex-it).
    pub chained: bool,
}

pub type Paradigms = HashMap<String, Vec<Flex>>;

/// Tiparet e nxjerra nga `gramm`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gramm {
    pub upos: String,
    pub pos: String,
    pub gender: String,
    pub anim: String,
    pub trans: String,
    pub med: bool,
    pub borr: String,
    pub flags: Vec<String>,
    pub nonst: bool,
    pub gheg: bool,
    pub abbr: bool,
}

// ───────────────────────────── analizuesit ─────────────────────────────

fn kv(line: &str) -> Option<(&str, &str)> {
    line.trim().split_once(':').map(|(k, v)| (k.trim(), v.trim()))
}

/// `lexemes.txt`: blloqe `-lexeme` me rreshta ` key: value`.
pub fn parse_lexemes(text: &str) -> Vec<Lexeme> {
    let mut out: Vec<Lexeme> = Vec::new();
    let mut cur: Option<Lexeme> = None;
    for line in text.lines() {
        let t = line.trim_end_matches('\r');
        match (t.trim() == "-lexeme", kv(t)) {
            (true, _) => {
                match cur.take() {
                    Some(l) => out.push(l),
                    None => {}
                }
                cur = Some(Lexeme::default());
            }
            (false, Some((k, v))) => match cur.as_mut() {
                Some(l) => match k {
                    "lex" => l.lex = v.to_string(),
                    "stem" => l.stem = v.to_string(),
                    "gramm" => l.gramm = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                    "paradigm" => l.paradigms.push(v.to_string()),
                    "trans_en" => l.trans_en = v.to_string(),
                    "lexref" => l.lexref = v.to_string(),
                    "deriv" => l.deriv = v.to_string(),
                    _ => {}
                },
                None => {}
            },
            (false, None) => {}
        }
    }
    match cur {
        Some(l) => out.push(l),
        None => {}
    }
    out
}

/// `paradigms.txt`: `-paradigm: id`, ` -flex: …`, `  gramm: …`, `  paradigm: …`.
pub fn parse_paradigms(text: &str) -> Paradigms {
    let mut out: Paradigms = HashMap::new();
    let mut name: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        match (t.strip_prefix("-paradigm:"), t.strip_prefix("-flex:"), kv(t)) {
            (Some(n), _, _) => {
                let n = n.trim().to_string();
                out.entry(n.clone()).or_default();
                name = Some(n);
            }
            (None, Some(f), _) => match name.as_ref().and_then(|n| out.get_mut(n)) {
                Some(v) => v.push(Flex { flex: f.trim().to_string(), gramm: Vec::new(), chained: false }),
                None => {}
            },
            (None, None, Some((k, v))) => match (name.as_ref().and_then(|n| out.get_mut(n)).and_then(|f| f.last_mut()), k) {
                (Some(f), "gramm") => f.gramm = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                (Some(f), "paradigm") => f.chained = true,
                _ => {}
            },
            _ => {}
        }
    }
    out
}

/// Kodi i gjuhës së huazimit sipas etiketave të uniparser-it.
fn is_lang_tag(t: &str) -> bool {
    let parts: Vec<&str> = t.split('/').collect();
    parts.iter().all(|p| {
        matches!(
            *p,
            "lat" | "rom" | "it" | "turk" | "slav" | "greek" | "fr" | "en" | "germ" | "rus" | "rum" | "arum"
                | "Agreek" | "hung"
        )
    })
}

pub fn parse_gramm(tags: &[String]) -> Gramm {
    let mut g = Gramm::default();
    let mut borrowed = false;
    let mut langs: Vec<String> = Vec::new();
    let mut trans: Vec<&str> = Vec::new();
    for t in tags {
        let s = t.as_str();
        match s {
            "NOUN" | "A" | "V" | "ADV" | "PRO" | "INTJ" | "PR" | "CONJ" | "PART" | "NUM" | "CLIT_PRO" | "VB_PART" => {
                g.upos = s.to_string();
                g.pos = match s {
                    "NOUN" => "n",
                    "V" => "v",
                    "A" => "a",
                    "ADV" => "r",
                    _ => "-",
                }
                .to_string();
            }
            "m" | "f" | "mf" | "n" => g.gender = s.to_string(),
            "anim" | "inanim" => g.anim = s.to_string(),
            "vt" | "vi" => trans.push(s),
            "med" => g.med = true,
            "borr" => borrowed = true,
            "Gheg" => g.gheg = true,
            "abbr" => {
                g.abbr = true;
                g.flags.push(s.to_string());
            }
            other => match (other.starts_with("nonst"), is_lang_tag(other)) {
                (true, _) => {
                    g.nonst = true;
                    g.flags.push(other.to_string());
                }
                (false, true) => langs.push(other.to_string()),
                (false, false) => g.flags.push(other.to_string()),
            },
        }
    }
    match g.gheg {
        true => g.flags.push("Gheg".into()),
        false => {}
    }
    g.trans = trans.join(",");
    // Gjuha e huazimit vetëm kur leksema shënohet `borr`; ndryshe ruhet si flamur.
    match borrowed {
        true => g.borr = langs.join(","),
        false => g.flags.extend(langs.into_iter().map(|l| format!("lang:{l}"))),
    }
    match (borrowed, g.borr.is_empty()) {
        (true, true) => g.borr = "unknwn".into(),
        _ => {}
    }
    g
}

/// A ka shkronja jashtë shqipes standarde (zanoret hundore gege â ê î ô û)?
fn has_gheg_letters(s: &str) -> bool {
    s.chars().any(|c| matches!(c, 'â' | 'ê' | 'î' | 'ô' | 'û' | 'Â' | 'Ê' | 'Î' | 'Ô' | 'Û'))
}

/// Lema e pranueshme: shkronja, hapësirë, vizë, apostrof.
fn well_formed(lemma: &str) -> bool {
    !lemma.is_empty()
        && lemma.chars().any(|c| c.is_alphabetic())
        && lemma.chars().all(|c| c.is_alphabetic() || c == ' ' || c == '-' || c == '\'' || c == '’')
}

pub fn status(l: &Lexeme, g: &Gramm) -> &'static str {
    match (well_formed(&l.lex), g.abbr, g.gheg || has_gheg_letters(&l.lex), g.nonst) {
        (false, _, _, _) => "badform",
        (true, true, _, _) => "abbr",
        (true, false, true, _) => "gheg",
        (true, false, false, true) => "nonst",
        (true, false, false, false) => "std",
    }
}

// ───────────────────────────── gjenerimi ─────────────────────────────

/// `<2>.i` → (Some(2), "i"); `.` → (None, ""); format e tjera → None.
pub fn parse_flex(f: &str) -> Option<(Option<usize>, String)> {
    let (num, rest) = match f.strip_prefix('<') {
        Some(r) => {
            let (n, rest) = r.split_once('>')?;
            (Some(n.parse::<usize>().ok()?), rest)
        }
        None => (None, f),
    };
    let suffix = rest.strip_prefix('.')?;
    match suffix.chars().any(|c| matches!(c, '.' | '<' | '>' | '/' | '[' | ']' | '|')) {
        true => None,
        false => Some((num, suffix.to_string())),
    }
}

/// Rrënja e zgjedhur sipas numrit të flex-it. Rrënja e vetme vlen për çdo numër.
/// Kthen None kur rrënja ka variante (`//`), pika të brendshme ose mungon.
pub fn pick_stem(stem: &str, num: Option<usize>) -> Option<String> {
    let slots: Vec<&str> = stem.split('|').collect();
    let slot = match (slots.len(), num) {
        (1, _) => slots[0],
        (_, Some(n)) => *slots.get(n)?,
        (_, None) => return None,
    };
    let slot = slot.trim();
    match (slot.contains("//"), slot.strip_suffix('.')) {
        (false, Some(body)) => match body.is_empty() || body.chars().any(|c| matches!(c, '.' | '<' | '>' | '[' | ']')) {
            true => None,
            false => Some(body.to_string()),
        },
        _ => None,
    }
}

fn has_all(g: &[String], want: &[&str]) -> bool {
    want.iter().all(|w| g.iter().any(|x| x == w))
}

/// Forma e shquar emërore njëjës sipas një paradigme të vetme.
/// `Err(arsye)` kur paradigma nuk e jep ose rregulli nuk është i njëkuptimtë.
/// Kontroll i brendshëm: forma e pashquar emërore njëjës e gjeneruar nga e njëjta
/// paradigmë duhet të jetë vetë lema (p.sh. «sallon» me rrënjë «sallona.» refuzohet).
pub fn def_nom_sg_in(stem: &str, paradigm: &[Flex], lemma: &str) -> Result<Option<String>, &'static str> {
    let def = form_in(stem, paradigm, &["sg", "nom", "def"])?;
    let indef = form_in(stem, paradigm, &["sg", "nom", "indef"]);
    match (def, indef) {
        (None, _) => Ok(None),
        (Some(d), Ok(None)) => Ok(Some(d)),
        (Some(d), Ok(Some(i))) => match i.to_lowercase() == lemma.to_lowercase() {
            true => Ok(Some(d)),
            false => Err("indef_form_differs_from_lemma"),
        },
        (Some(_), Err(_)) => Err("indef_form_unverifiable"),
    }
}

/// Forma e vetme që paradigma jep për tiparet `want` (pa variantet «nonst»).
pub fn form_in(stem: &str, paradigm: &[Flex], want: &[&str]) -> Result<Option<String>, &'static str> {
    let cands: Vec<&Flex> = paradigm
        .iter()
        .filter(|f| has_all(&f.gramm, want))
        .filter(|f| !f.gramm.iter().any(|x| x.starts_with("nonst")))
        .collect();
    match cands.len() {
        0 => Ok(None),
        _ => {
            let forms: BTreeSet<Option<String>> = cands
                .iter()
                .map(|f| match f.chained {
                    true => None,
                    false => parse_flex(&f.flex).and_then(|(n, suf)| pick_stem(stem, n).map(|s| format!("{s}{suf}"))),
                })
                .collect();
            match (forms.len(), forms.iter().next().cloned().flatten()) {
                (1, Some(f)) => Ok(Some(f)),
                (1, None) => Err("unsupported_stem_or_flex"),
                _ => Err("ambiguous_paradigm"),
            }
        }
    }
}

/// Gjenerimi për një leksemë emërore: (paradigma, forma) ose arsyeja e mosgjenerimit.
pub fn generate_def(l: &Lexeme, pars: &Paradigms) -> Result<(String, String), &'static str> {
    let mut found: Vec<(String, String)> = Vec::new();
    for p in &l.paradigms {
        match pars.get(p) {
            Some(flexes) => match def_nom_sg_in(&l.stem, flexes, &l.lex)? {
                Some(f) => found.push((p.clone(), f)),
                None => {}
            },
            None => return Err("unknown_paradigm"),
        }
    }
    let forms: BTreeSet<&String> = found.iter().map(|(_, f)| f).collect();
    match (forms.len(), found.first()) {
        (0, _) => Err("no_def_nom_sg_in_paradigms"),
        (1, Some((p, f))) => Ok((p.clone(), f.to_lowercase())),
        _ => Err("ambiguous_paradigms"),
    }
}

// ───────────────────────────── validimi ─────────────────────────────

/// UniMorph: `lemë ⇥ formë ⇥ N;NOM;SG;DEF` (renditja e tipareve mund të ndryshojë).
pub fn unimorph_def_nom_sg(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in text.lines() {
        let c: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
        match c.len() >= 3 {
            true => {
                let mut f: Vec<&str> = c[2].split(';').collect();
                f.sort();
                match f == ["DEF", "N", "NOM", "SG"] {
                    true => {
                        out.entry(c[0].trim().to_lowercase()).or_default().insert(c[1].trim().to_lowercase());
                    }
                    false => {}
                }
            }
            false => {}
        }
    }
    out
}

#[derive(Debug, Default, Clone)]
pub struct ParStat {
    pub samples: usize,
    pub agree: usize,
    pub mismatches: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Validation {
    pub per_paradigm: BTreeMap<String, ParStat>,
    pub unimorph_lemmas: usize,
    pub matched_lemmas: usize,
    pub not_in_uniparser: usize,
    pub not_generated: usize,
    pub samples: usize,
    pub agree: usize,
}

impl Validation {
    pub fn passes(&self, paradigm: &str) -> bool {
        self.per_paradigm
            .get(paradigm)
            .map(|s| s.samples >= MIN_SAMPLES && s.agree * 100 >= MIN_AGREEMENT_PCT * s.samples)
            .unwrap_or(false)
    }
    pub fn check_label(&self, paradigm: &str) -> String {
        match (self.per_paradigm.get(paradigm), self.passes(paradigm)) {
            (Some(s), true) => format!("validated:{paradigm}:{}/{}", s.agree, s.samples),
            (Some(s), false) => format!("rejected:{paradigm}:{}/{}", s.agree, s.samples),
            (None, _) => format!("unvalidated:{paradigm}:0/0"),
        }
    }
}

fn is_std_noun(l: &Lexeme) -> bool {
    let g = parse_gramm(&l.gramm);
    g.upos == "NOUN" && status(l, &g) == "std"
}

/// Format e gjeneruara (sipas lemës, vetëm emra standardë): lemë → [(paradigmë, formë)].
pub fn generated_by_lemma(lex: &[Lexeme], pars: &Paradigms) -> BTreeMap<String, Vec<(String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for l in lex.iter().filter(|l| is_std_noun(l)) {
        match generate_def(l, pars) {
            Ok(pf) => out.entry(l.lex.to_lowercase()).or_default().push(pf),
            Err(_) => {
                out.entry(l.lex.to_lowercase()).or_default();
            }
        }
    }
    out
}

pub fn validate(lex: &[Lexeme], pars: &Paradigms, unimorph: &BTreeMap<String, BTreeSet<String>>) -> Validation {
    let gen = generated_by_lemma(lex, pars);
    let mut v = Validation { unimorph_lemmas: unimorph.len(), ..Default::default() };
    for (lemma, gold) in unimorph {
        match gen.get(lemma) {
            None => v.not_in_uniparser += 1,
            Some(list) => {
                v.matched_lemmas += 1;
                match list.is_empty() {
                    true => v.not_generated += 1,
                    false => {}
                }
                // Çdo paradigmë që jep një formë për këtë lemë merr një mostër.
                let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
                for (p, f) in list {
                    match seen.insert((p.clone(), f.clone())) {
                        true => {
                            let ok = gold.contains(f);
                            let s = v.per_paradigm.entry(p.clone()).or_default();
                            s.samples += 1;
                            v.samples += 1;
                            match ok {
                                true => {
                                    s.agree += 1;
                                    v.agree += 1;
                                }
                                false => s.mismatches.push(format!(
                                    "{lemma}→{f} (UniMorph: {})",
                                    gold.iter().cloned().collect::<Vec<_>>().join("/")
                                )),
                            }
                        }
                        false => {}
                    }
                }
            }
        }
    }
    v
}

pub fn validation_tsv(v: &Validation) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# uniparser-albanian 2.3.0 — validimi i formës së shquar emërore njëjës kundrejt UniMorph sqi (N;NOM;SG;DEF)\n\
         # lema UniMorph: {}; të gjetura si emra standardë në uniparser: {}; pa leksemë: {}; pa formë të gjeneruar: {}\n\
         # mostra: {}; përputhje: {} ({}); pragu: >= {} mostra dhe >= {} %\n",
        v.unimorph_lemmas,
        v.matched_lemmas,
        v.not_in_uniparser,
        v.not_generated,
        v.samples,
        v.agree,
        pct(v.agree, v.samples),
        MIN_SAMPLES,
        MIN_AGREEMENT_PCT
    ));
    s.push_str("paradigm\tsamples\tagree\trate\tused\tmismatches\n");
    for (p, st) in &v.per_paradigm {
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            common::clean(p),
            st.samples,
            st.agree,
            pct(st.agree, st.samples),
            match v.passes(p) {
                true => "yes",
                false => "no",
            },
            common::clean(&st.mismatches.join("; "))
        ));
    }
    s
}

pub fn pct(a: usize, n: usize) -> String {
    match n {
        0 => "-".into(),
        _ => format!("{}.{:02}%", a * 100 / n, (a * 10_000 / n) % 100),
    }
}

// ───────────────────────────── TSV ─────────────────────────────

fn dash(s: &str) -> String {
    match s.trim().is_empty() {
        true => "-".into(),
        false => common::clean(s),
    }
}

/// Ekstrakti TSV. Forma e shquar shkruhet vetëm për emra standardë, me paradigmë
/// të validuar dhe pa konflikt ndërmjet leksemave të së njëjtës lemë.
pub fn build_tsv(lex: &[Lexeme], pars: &Paradigms, v: &Validation) -> (String, BTreeMap<&'static str, usize>) {
    let gen = generated_by_lemma(lex, pars);
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut s = String::with_capacity(16 * 1024 * 1024);
    s.push_str("# uniparser-albanian 2.3.0 (data_strict) — MIT — Morozova, Rusakov, Arkhangelskiy\n");
    s.push_str("# trans_en_bridge është anglisht: urë vetëm, kurrë nuk paraqitet si shqip\n");
    s.push_str(TSV_HEADER);
    s.push('\n');
    for l in lex {
        let g = parse_gramm(&l.gramm);
        let st = status(l, &g);
        *counts.entry(st).or_default() += 1;
        let lemma = l.lex.to_lowercase();
        let distinct: BTreeSet<&String> =
            gen.get(&lemma).map(|v| v.iter().map(|(_, f)| f).collect()).unwrap_or_default();
        let (def, check) = match (g.upos == "NOUN", st == "std") {
            (true, true) => match generate_def(l, pars) {
                Ok((p, f)) => match (distinct.len() > 1, v.passes(&p)) {
                    (true, _) => ("-".to_string(), "conflict_between_lexemes".to_string()),
                    (false, true) => (f, v.check_label(&p)),
                    (false, false) => ("-".to_string(), v.check_label(&p)),
                },
                Err(e) => ("-".to_string(), e.to_string()),
            },
            (true, false) => ("-".to_string(), "not_standard".to_string()),
            (false, _) => ("-".to_string(), "-".to_string()),
        };
        match def != "-" {
            true => *counts.entry("def_nom_sg").or_default() += 1,
            false => {}
        }
        let mut flags = g.flags.clone();
        match (g.upos.is_empty(), g.pos == "-") {
            (false, true) => flags.insert(0, g.upos.clone()),
            _ => {}
        }
        let row = [
            dash(&l.lex),
            dash(&lemma),
            dash(&g.upos),
            dash(&g.pos),
            dash(&g.gender),
            dash(&g.anim),
            dash(&g.trans),
            match g.med {
                true => "yes".into(),
                false => "-".into(),
            },
            dash(&g.borr),
            dash(&flags.join(",")),
            st.to_string(),
            dash(&l.paradigms.join(",")),
            dash(&l.stem),
            dash(&l.lexref),
            dash(&l.deriv),
            dash(&def),
            dash(&check),
            dash(&l.trans_en),
        ];
        s.push_str(&row.join("\t"));
        s.push('\n');
    }
    (s, counts)
}

// ───────────────────────────── PyPI / ZIP ─────────────────────────────

/// SHA-256 i botuar nga PyPI për skedarin `filename`.
pub fn pypi_sha256(json: &[u8], filename: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_slice(json).map_err(|e| format!("PyPI JSON: {e}"))?;
    v.get("urls")
        .and_then(|u| u.as_array())
        .and_then(|a| a.iter().find(|x| x.get("filename").and_then(|f| f.as_str()) == Some(filename)))
        .and_then(|x| x.pointer("/digests/sha256"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| format!("PyPI JSON: mungon urls[].digests.sha256 për {filename}"))
}

/// A thotë PyPI se licenca është MIT (klasifikues ose fusha `license`)?
pub fn pypi_says_mit(json: &[u8]) -> bool {
    let v: serde_json::Value = serde_json::from_slice(json).unwrap_or(serde_json::Value::Null);
    let classifier = v
        .pointer("/info/classifiers")
        .and_then(|c| c.as_array())
        .map(|a| a.iter().any(|x| x.as_str() == Some("License :: OSI Approved :: MIT License")))
        .unwrap_or(false);
    let field = v
        .pointer("/info/license")
        .and_then(|l| l.as_str())
        .map(|l| l.to_uppercase().contains("MIT"))
        .unwrap_or(false);
    classifier || field
}

/// Teksti MIT me autorët e pritur.
pub fn license_ok(text: &str) -> bool {
    [
        "Permission is hereby granted, free of charge",
        "THE SOFTWARE IS PROVIDED \"AS IS\"",
        "Morozova",
        "Rusakov",
        "Arkhangelskiy",
    ]
    .iter()
    .all(|w| text.contains(w))
}

pub fn read_zip_member(zip_bytes: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| format!("ZIP: {e}"))?;
    let mut f = z.by_name(name).map_err(|e| format!("ZIP {name}: {e}"))?;
    let mut out = Vec::new();
    f.read_to_end(&mut out).map_err(|e| format!("ZIP {name}: {e}"))?;
    Ok(out)
}

fn utf8(b: Vec<u8>, what: &str) -> Result<String, String> {
    String::from_utf8(b).map_err(|e| format!("{what}: UTF-8 i pavlefshëm ({e})"))
}

// ───────────────────────────── fetch ─────────────────────────────

pub fn fetch(data: &Path) -> Result<Vec<Fetched>, String> {
    let wheel = common::get(WHEEL_URL)?;
    let pypi = common::get(PYPI_JSON_URL)?;
    let published = pypi_sha256(&pypi, WHEEL_NAME)?;
    let local = common::sha256_hex(&wheel);
    match published == local {
        true => {}
        false => return Err(format!("uniparser: SHA-256 i wheel-it {local} ≠ PyPI {published}")),
    }
    let lic_text = utf8(read_zip_member(&wheel, ZIP_LICENSE)?, "LICENSE")?;
    match (license_ok(&lic_text), pypi_says_mit(&pypi)) {
        (true, true) => {}
        (a, b) => return Err(format!("uniparser: licenca nuk u verifikua (teksti MIT: {a}, PyPI MIT: {b})")),
    }
    let lexemes = parse_lexemes(&utf8(read_zip_member(&wheel, ZIP_LEXEMES)?, "lexemes.txt")?);
    let pars = parse_paradigms(&utf8(read_zip_member(&wheel, ZIP_PARADIGMS)?, "paradigms.txt")?);
    let derivations = utf8(read_zip_member(&wheel, ZIP_DERIVATIONS)?, "derivations.txt")?;
    let deriv_types = derivations.lines().filter(|l| l.trim().starts_with("-deriv-type")).count();

    let um_bytes = common::get(UNIMORPH_URL)?;
    let um_text = String::from_utf8_lossy(&um_bytes).to_string();
    let gold = unimorph_def_nom_sg(&um_text);
    let v = validate(&lexemes, &pars, &gold);
    let (tsv, counts) = build_tsv(&lexemes, &pars, &v);
    let vtsv = validation_tsv(&v);

    println!(
        "[uniparser] wheel {} B, sha256 {local} = PyPI ✓; leksema {}, paradigma {}, tipe derivimi {}",
        wheel.len(),
        lexemes.len(),
        pars.len(),
        deriv_types
    );
    println!(
        "[uniparser] statusi: {}",
        counts.iter().map(|(k, n)| format!("{k}={n}")).collect::<Vec<_>>().join(", ")
    );
    println!(
        "[uniparser] validim UniMorph: {} mostra, {} përputhje ({}); paradigma të pranuara: {}",
        v.samples,
        v.agree,
        pct(v.agree, v.samples),
        v.per_paradigm.keys().filter(|p| v.passes(p)).cloned().collect::<Vec<_>>().join(",")
    );

    let pub_check = format!("PyPI JSON urls[].digests.sha256 = {published} (përputhet)");
    let derived = format!("nxjerrë nga wheel-i sha256 {local} (i verifikuar me PyPI)");
    Ok(vec![
        common::record(
            data,
            SOURCE,
            &format!("{RAW_DIR}/{WHEEL_NAME}"),
            &wheel,
            WHEEL_URL,
            VERSION,
            LICENSE,
            ATTRIBUTION,
            &pub_check,
            "ok",
        )?,
        common::record(
            data,
            SOURCE,
            &format!("{RAW_DIR}/LICENSE"),
            lic_text.as_bytes(),
            WHEEL_URL,
            VERSION,
            LICENSE,
            ATTRIBUTION,
            &derived,
            "ok",
        )?,
        common::record(
            data,
            SOURCE,
            &format!("{RAW_DIR}/unimorph-sqi.tsv"),
            &um_bytes,
            UNIMORPH_URL,
            "unimorph/sqi master (shkarkuar në kohën e fetch-it)",
            UNIMORPH_LICENSE,
            UNIMORPH_ATTRIBUTION,
            "pa kontroll botuesi (GitHub raw)",
            "validation_only",
        )?,
        common::record(
            data,
            SOURCE,
            VALIDATION_REL,
            vtsv.as_bytes(),
            WHEEL_URL,
            VERSION,
            LICENSE,
            ATTRIBUTION,
            &derived,
            "ok",
        )?,
        common::record(data, SOURCE, TSV_REL, tsv.as_bytes(), WHEEL_URL, VERSION, LICENSE, ATTRIBUTION, &derived, "ok")?,
    ])
}

// ───────────────────────────── testet ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Copëza reale (format i pandryshuar) nga data_strict/lexemes.txt dhe paradigms.txt.
    const LEX: &str = "-lexeme\n lex: kalë\n stem: kuaj.|kalë.|kal.\n gramm: NOUN,m,anim,borr,lat\n paradigm: 1b\n paradigm: 5a\n trans_en: horse\n\n\
-lexeme\n lex: djalë\n stem: djelm.\n gramm: NOUN,m,nonst,anim,alb\n paradigm: 5a\n paradigm: 5b\n trans_en: boy, son\n\n\
-lexeme\n lex: lule\n stem: lule.|lule.|lulej.|lulj.\n gramm: NOUN,f,inanim,alb\n paradigm: 3b\n paradigm: 5a\n trans_en: flower\n\n\
-lexeme\n lex: qen\n stem: qen.//qenër.|qen.\n gramm: NOUN,m,anim,alb\n paradigm: 1a\n paradigm: 5b\n trans_en: dog\n\n\
-lexeme\n lex: Shqypni\n stem: shqypni.|shqypni.\n gramm: NOUN,f,inanim,nonst,Gheg,borr,lat,deriv\n lexref: Shqipëri\n paradigm: 3d\n paradigm: 5b\n trans_en: Albania\n\n\
-lexeme\n lex: punoj\n stem: puno.|punua.|punu.|pun.\n gramm: V,vt,vi,alb,deriv\n paradigm: lexoj\n trans_en: work\n\n\
-lexeme\n lex: AFP\n stem: afp.//afp-.\n gramm: NOUN,abbr,f\n paradigm: abbreviation-f-1\n trans_en: Agence France-Presse, AFP\n";

    const PAR: &str = "-paradigm: 1a\n -flex: <1>.\n  gramm: sg,nom,indef\n -flex: <1>.i\n  gramm: sg,nom,def\n -flex: <1>.në\n  gramm: sg,acc,def,nonst\n\n\
-paradigm: 1b\n -flex: <1>.\n  gramm: sg,nom,indef\n -flex: <2>.i\n  gramm: sg,gen_dat,indef\n -flex: <2>.i\n  gramm: sg,nom,def\n\n\
-paradigm: 3b\n -flex: <1>.\n  gramm: sg,nom,indef\n -flex: <3>.a\n  gramm: sg,nom,def\n -flex: <1>.s\n  gramm: sg,gen_dat,def\n\n\
-paradigm: 3d\n -flex: <1>.a\n  gramm: sg,nom,def\n -flex: <1>.ja\n  gramm: sg,nom,def,nonst\n\n\
-paradigm: 5a\n -flex: <0>.\n  gramm: pl,nom,indef\n\n\
-paradigm: 5b\n -flex: <0>.\n  gramm: pl,nom,indef\n";

    #[test]
    fn parses_real_lexeme_blocks() {
        let l = parse_lexemes(LEX);
        assert_eq!(l.len(), 7);
        assert_eq!(l[0].lex, "kalë");
        assert_eq!(l[0].paradigms, vec!["1b", "5a"]);
        assert_eq!(l[4].lexref, "Shqipëri");
        let g = parse_gramm(&l[0].gramm);
        assert_eq!((g.pos.as_str(), g.gender.as_str(), g.anim.as_str(), g.borr.as_str()), ("n", "m", "anim", "lat"));
        let g = parse_gramm(&l[5].gramm);
        assert_eq!((g.pos.as_str(), g.trans.as_str()), ("v", "vt,vi"));
        assert!(g.flags.contains(&"deriv".to_string()) && g.flags.contains(&"lang:alb".to_string()) == false);
        assert!(g.flags.contains(&"alb".to_string()));
    }

    #[test]
    fn status_flags_gheg_nonst_abbr() {
        let l = parse_lexemes(LEX);
        let st: Vec<&str> = l.iter().map(|x| status(x, &parse_gramm(&x.gramm))).collect();
        assert_eq!(st, vec!["std", "nonst", "std", "std", "gheg", "std", "abbr"]);
    }

    #[test]
    fn generates_definite_nominative_from_paradigms() {
        let l = parse_lexemes(LEX);
        let p = parse_paradigms(PAR);
        assert_eq!(generate_def(&l[0], &p), Ok(("1b".into(), "kali".into())));
        assert_eq!(generate_def(&l[2], &p), Ok(("3b".into(), "lulja".into())));
        // Rrënja 1 e «qen» nuk ka variante, prandaj forma është e njëkuptimtë.
        assert_eq!(generate_def(&l[3], &p), Ok(("1a".into(), "qeni".into())));
        // 3d ka edhe -ja «nonst», që përjashtohet.
        assert_eq!(generate_def(&l[4], &p), Ok(("3d".into(), "shqypnia".into())));
        // Leksemë reale me rrënjë të gabuar njëjësi: forma e pashquar ≠ lema → refuzohet.
        let s = parse_lexemes("-lexeme\n lex: sallon\n stem: sallone.|sallona.\n gramm: NOUN,mf,inanim,borr,it\n paradigm: 1a\n paradigm: 5a\n trans_en: lobby; showroom; salon\n");
        assert_eq!(generate_def(&s[0], &p), Err("indef_form_differs_from_lemma"));
    }

    #[test]
    fn stems_and_flexes_are_strict() {
        assert_eq!(parse_flex("<2>.it"), Some((Some(2), "it".into())));
        assert_eq!(parse_flex("."), Some((None, "".into())));
        assert_eq!(parse_flex(".//<1>.a"), None);
        assert_eq!(pick_stem("bukur.", Some(1)), Some("bukur".into()));
        assert_eq!(pick_stem("qen.//qenër.|qen.", Some(0)), None);
        assert_eq!(pick_stem("homoseksualizma.|homoseksualizëm.homoseksualizm.", Some(1)), None);
        assert_eq!(pick_stem("windows.|windows", Some(1)), None);
    }

    #[test]
    fn validation_and_tsv_respect_threshold() {
        let l = parse_lexemes(LEX);
        let p = parse_paradigms(PAR);
        let um = unimorph_def_nom_sg("kalë\tkali\tN;NOM;SG;DEF\nlule\tlulja\tN;DEF;NOM;SG\nqen\tqenin\tN;ACC;SG;DEF\n");
        assert_eq!(um.len(), 2);
        let v = validate(&l, &p, &um);
        assert_eq!((v.samples, v.agree), (2, 2));
        // Vetëm 1 mostër për paradigmë < MIN_SAMPLES: asnjë formë nuk shkruhet.
        assert!(!v.passes("1b"));
        let (tsv, counts) = build_tsv(&l, &p, &v);
        assert_eq!(counts.get("def_nom_sg"), None);
        let kale = tsv.lines().find(|r| r.starts_with("kalë\t")).unwrap();
        let c: Vec<&str> = kale.split('\t').collect();
        assert_eq!(c.len(), TSV_HEADER.split('\t').count());
        assert_eq!(c[15], "-");
        assert_eq!(c[16], "rejected:1b:1/1");
        assert_eq!(c[17], "horse");
        // Me prag të plotësuar forma shkruhet.
        let mut v2 = Validation::default();
        v2.per_paradigm.insert("1b".into(), ParStat { samples: 20, agree: 20, mismatches: vec![] });
        let (tsv2, _) = build_tsv(&l, &p, &v2);
        let c: Vec<String> =
            tsv2.lines().find(|r| r.starts_with("kalë\t")).unwrap().split('\t').map(String::from).collect();
        assert_eq!((c[15].as_str(), c[16].as_str()), ("kali", "validated:1b:20/20"));
    }

    #[test]
    fn pypi_digest_and_license_checks() {
        let j = br#"{"info":{"classifiers":["License :: OSI Approved :: MIT License"],"license":null},
            "urls":[{"filename":"x.tar.gz","digests":{"sha256":"aa"}},
                    {"filename":"uniparser_albanian-2.3.0-py3-none-any.whl","digests":{"sha256":"ABCD"}}]}"#;
        assert_eq!(pypi_sha256(j, WHEEL_NAME), Ok("abcd".into()));
        assert!(pypi_says_mit(j));
        assert!(!license_ok("GPL"));
    }
}
