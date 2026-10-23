//! Matja e fonologjisë ndaj dëshmive të jashtme: IPA dhe rrokjezimi i Wiktionary-t (kaikki),
//! fjalët e UD-së. Këto lexohen vetëm këtu; moduli nuk i sheh kurrë.

use nura_lingua::gramatika::fonologjia::{self as fon, Hint};
use nura_lingua::gramatika::leksiku::{Lexeme, Lexicon};
use nura_lingua::gramatika::tipet::{split_of, Pos, Split};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
const JSONL: &str = "raw/kaikki_en/kaikki.org-dictionary-Albanian.jsonl";
const UD: [&str; 4] = ["raw/ud_sq_staf/sq_staf-ud-train.conllu", "raw/ud_sq_staf/sq_staf-ud-dev.conllu", "raw/ud_sq_staf/sq_staf-ud-test.conllu", "raw/ud_sq_tsa/sq_tsa-ud-test.conllu"];

#[derive(Debug, Clone)]
enum J {
    Null,
    Bool,
    Num,
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }
    fn arr(&self) -> &[J] {
        match self {
            J::Arr(v) => v,
            _ => &[],
        }
    }
}

struct Parser<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser { s, b: s.as_bytes(), i: 0 }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.i += 1;
        }
    }
    fn value(&mut self) -> Option<J> {
        self.ws();
        match self.peek()? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => self.string().map(J::Str),
            b't' => {
                self.i += 4;
                Some(J::Bool)
            }
            b'f' => {
                self.i += 5;
                Some(J::Bool)
            }
            b'n' => {
                self.i += 4;
                Some(J::Null)
            }
            _ => {
                while matches!(self.peek(), Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')) {
                    self.i += 1;
                }
                Some(J::Num)
            }
        }
    }
    fn hex4(&mut self) -> Option<u32> {
        let h = self.s.get(self.i..self.i + 4)?;
        self.i += 4;
        u32::from_str_radix(h, 16).ok()
    }
    fn string(&mut self) -> Option<String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            match self.peek()? {
                b'"' => {
                    self.i += 1;
                    return Some(out);
                }
                b'\\' => {
                    let e = *self.b.get(self.i + 1)?;
                    self.i += 2;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' | b'f' => out.push(' '),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = match (0xD800..0xDC00).contains(&hi) {
                                true => {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    0x10000 + ((hi - 0xD800) << 10) + lo.saturating_sub(0xDC00)
                                }
                                false => hi,
                            };
                            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                        }
                        other => out.push(other as char),
                    }
                }
                _ => {
                    let start = self.i;
                    while self.peek().map(|c| c != b'"' && c != b'\\').unwrap_or(false) {
                        self.i += 1;
                    }
                    out.push_str(self.s.get(start..self.i)?);
                }
            }
        }
    }
    fn array(&mut self) -> Option<J> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            match self.peek()? {
                b']' => {
                    self.i += 1;
                    return Some(J::Arr(v));
                }
                b',' => self.i += 1,
                _ => v.push(self.value()?),
            }
        }
    }
    fn object(&mut self) -> Option<J> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            match self.peek()? {
                b'}' => {
                    self.i += 1;
                    return Some(J::Obj(v));
                }
                b',' => self.i += 1,
                b'"' => {
                    let k = self.string()?;
                    self.ws();
                    self.i += 1;
                    let val = self.value()?;
                    v.push((k, val));
                }
                _ => return None,
            }
        }
    }
}

struct Entry {
    word: String,
    pos: String,
    lemma_of: Option<String>,
    ipas: Vec<String>,
    hyph: Vec<String>,
}

fn standard_sound(s: &J) -> bool {
    let tags: Vec<&str> = s.get("tags").map(J::arr).unwrap_or(&[]).iter().filter_map(J::str).collect();
    s.get("note").is_none() && (tags.is_empty() || tags.iter().any(|t| *t == "standard" || *t == "Standard"))
}

fn entries() -> Vec<Entry> {
    let text = std::fs::read_to_string(Path::new(DATA).join(JSONL)).unwrap();
    text.lines()
        .filter_map(|l| Parser::new(l).value())
        .map(|j| {
            let text_of = |k: &str| j.get(k).and_then(J::str).unwrap_or("").to_string();
            let lemma_of = j
                .get("senses")
                .map(J::arr)
                .unwrap_or(&[])
                .iter()
                .find_map(|s| s.get("form_of").and_then(|f| f.arr().first()).and_then(|f| f.get("word")).and_then(J::str))
                .map(str::to_string);
            let ipas = j.get("sounds").map(J::arr).unwrap_or(&[]).iter().filter(|s| standard_sound(s)).filter_map(|s| s.get("ipa").and_then(J::str)).map(str::to_string).collect();
            let hyph = j
                .get("hyphenations")
                .map(J::arr)
                .unwrap_or(&[])
                .iter()
                .map(|h| h.get("parts").map(J::arr).unwrap_or(&[]).iter().filter_map(J::str).collect::<Vec<_>>().join("-"))
                .filter(|s| !s.is_empty())
                .collect();
            Entry { word: text_of("word"), pos: text_of("pos"), lemma_of, ipas, hyph }
        })
        .collect()
}

const SKIP_POS: [&str; 10] = ["character", "symbol", "suffix", "prefix", "infix", "interfix", "punct", "letter", "affix", "circumfix"];

fn usable(e: &Entry) -> bool {
    !SKIP_POS.contains(&e.pos.as_str()) && !e.word.is_empty() && !e.word.contains(' ')
}

/// Lema që cakton ndarjen Dev/Test: vetë fjala kur ka një hyrje lemë, përndryshe lema e
/// trajtës (`form_of`).
fn split_lemmas(ents: &[Entry]) -> BTreeMap<&str, &str> {
    let mut out: BTreeMap<&str, &str> = BTreeMap::new();
    for e in ents.iter().filter(|e| e.lemma_of.is_none()) {
        out.insert(e.word.as_str(), e.word.as_str());
    }
    for e in ents {
        let lemma = e.lemma_of.as_deref().unwrap_or(e.word.as_str());
        out.entry(e.word.as_str()).or_insert(lemma);
    }
    out
}

fn split_by<'a>(lemmas: &BTreeMap<&'a str, &'a str>, word: &str) -> Split {
    split_of(lemmas.get(word).copied().unwrap_or(word))
}

fn pos_of(p: &str) -> Pos {
    match p {
        "noun" => Pos::Noun,
        "verb" => Pos::Verb,
        "adj" => Pos::Adj,
        "adv" => Pos::Adv,
        "name" => Pos::Propn,
        "pron" => Pos::Pron,
        "num" => Pos::Num,
        "det" | "article" => Pos::Det,
        "prep" | "postp" => Pos::Adp,
        "conj" => Pos::Conj,
        "particle" => Pos::Part,
        "intj" => Pos::Intj,
        _ => Pos::Other,
    }
}

fn lower(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

#[derive(Default, Clone, Copy)]
struct Tally {
    total: usize,
    answered: usize,
    correct: usize,
    err: usize,
    len: usize,
}

struct Rows {
    rows: Vec<(String, [Tally; 2])>,
    dev: Vec<String>,
    show: String,
}

fn side(s: Split) -> usize {
    match s {
        Split::Dev => 0,
        Split::Test => 1,
    }
}

impl Rows {
    fn tally(&mut self, metric: &str, split: Split) -> &mut Tally {
        let at = self.rows.iter().position(|(m, _)| m == metric).unwrap_or(self.rows.len());
        match at == self.rows.len() {
            true => self.rows.push((metric.to_string(), [Tally::default(); 2])),
            false => (),
        }
        &mut self.rows[at].1[side(split)]
    }

    fn add(&mut self, metric: &str, split: Split, answered: bool, correct: bool, note: impl FnOnce() -> String) {
        let show = !self.show.is_empty() && metric.starts_with(self.show.as_str());
        let t = self.tally(metric, split);
        t.total += 1;
        t.answered += usize::from(answered);
        t.correct += usize::from(answered && correct);
        let wrong = answered && !correct && split == Split::Dev && show;
        wrong.then(note).into_iter().for_each(|n| self.dev.push(format!("{metric}\t{n}")));
    }

    fn get(&self, metric: &str, split: Split) -> Tally {
        self.rows.iter().find(|(m, _)| m == metric).map(|(_, t)| t[side(split)]).unwrap_or_default()
    }
}

/// Kufiri i poshtëm Wilson 95% i saktësisë: (p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n), z = 1.96.
fn wilson_lower(correct: usize, n: usize) -> f64 {
    let z = 1.96f64;
    let nf = n.max(1) as f64;
    let p = correct as f64 / nf;
    let lo = (p + z * z / (2.0 * nf) - z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt()) / (1.0 + z * z / nf);
    match n {
        0 => 0.0,
        _ => lo,
    }
}

fn precision(t: &Tally) -> f64 {
    t.correct as f64 / t.answered.max(1) as f64
}

fn coverage(t: &Tally) -> f64 {
    t.answered as f64 / t.total.max(1) as f64
}

/// Normalizimi i IPA-s: hiqen / [ ] ˈ ˌ . ː, hapësirat, kllapat dhe harku i afrikatës;
/// «g» latine = «ɡ». `keep` ruan shenjat e kufirit që duhen. Në trajtën e gjerë bashkohen
/// edhe variantet e transkriptuesve (e/ɛ, o/ɔ, ɹ/ɾ, cɕ/c, ɟʑ/ɟ, shenjat diakritike) dhe
/// alofonët e /n/-së para qiellzoreve e prapaqiellzoreve (ŋɡ, ŋk, ɲɟ).
fn norm(s: &str, broad: bool, keep: &str) -> String {
    let strict: String = s
        .chars()
        .filter(|c| keep.contains(*c) || !"/[]ˈˌ.ː ()\u{361}\u{35c}".contains(*c))
        .map(|c| match c {
            'g' => 'ɡ',
            other => other,
        })
        .collect();
    let wide: String = strict
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c) && *c != 'ˑ')
        .map(|c| match c {
            'e' => 'ɛ',
            'o' => 'ɔ',
            'ɹ' | 'ɻ' | 'ɽ' => 'ɾ',
            'ä' | 'ɑ' => 'a',
            'ɜ' => 'ə',
            'ɪ' => 'i',
            'ʊ' => 'u',
            other => other,
        })
        .collect();
    match broad {
        true => wide
            .replace("cɕ", "c")
            .replace("cç", "c")
            .replace("ɟʑ", "ɟ")
            .replace("ɟʝ", "ɟ")
            .replace("ŋɡ", "nɡ")
            .replace("ŋk", "nk")
            .replace("ɲɟ", "nɟ"),
        false => strict,
    }
}

/// Fonemat e një vargu IPA të normalizuar: afrikatet (ts, tʃ, dz, dʒ) dhe çiftet qiellzore
/// (cɕ, ɟʑ) janë një njësi; shenjat kombinuese e modifikuese i bashkohen tingullit para tyre.
fn phonemes(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in s.chars() {
        let attach = ('\u{300}'..='\u{36f}').contains(&c) || "ʰʲʷˠˤ".contains(c);
        let last = out.last().map(String::as_str).unwrap_or("");
        let affricate = matches!((last, c), ("t", 's' | 'ʃ' | 'ɕ') | ("d", 'z' | 'ʒ' | 'ʑ') | ("c", 'ɕ' | 'ç') | ("ɟ", 'ʑ' | 'ʝ'));
        match (attach || affricate, out.last_mut()) {
            (true, Some(prev)) => prev.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

const IPA_VOWELS: &str = "aɛəiɔuyeoøæɐɒɨɯʏœɘɵ";

fn clean_gold_ipa(g: &str) -> bool {
    !g.contains(['-', '…', ',', '~']) && !g.trim_matches(['/', '[', ']']).trim().is_empty()
}

fn ipa_rows(ents: &[Entry], lemmas: &BTreeMap<&str, &str>, rows: &mut Rows) {
    let mut by_word: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in ents.iter().filter(|e| usable(e)) {
        by_word.entry(e.word.as_str()).or_default().extend(e.ipas.iter().map(String::as_str).filter(|g| clean_gold_ipa(g)));
    }
    for (word, golds) in by_word.iter().filter(|(_, g)| !g.is_empty()) {
        let split = split_by(lemmas, word);
        let ours = fon::to_ipa(word);
        let drop_final = lower(word).ends_with('ë') && fon::letters(word).iter().filter(|u| fon::is_vowel(u)).count() >= 2;
        for (name, broad, dropped) in [
            ("ipa exact, strict", false, false),
            ("ipa exact, broad", true, false),
            ("ipa exact, strict, final ë dropped", false, true),
            ("ipa exact, broad, final ë dropped", true, true),
        ] {
            let mine = ours.as_ref().map(|o| {
                let o = norm(o, broad, "");
                match dropped && drop_final {
                    true => o.strip_suffix('ə').map(str::to_string).unwrap_or(o),
                    false => o,
                }
            });
            let best = mine.as_ref().and_then(|m| {
                let mp = phonemes(m);
                golds
                    .iter()
                    .map(|g| {
                        let gp = phonemes(&norm(g, broad, ""));
                        (fon::levenshtein(&mp, &gp), gp.len())
                    })
                    .min()
            });
            let ok = best.map(|(d, _)| d == 0).unwrap_or(false);
            let t = rows.tally(name, split);
            t.err += best.map(|b| b.0).unwrap_or(0);
            t.len += best.map(|b| b.1).unwrap_or(0);
            let shown = golds.iter().map(|g| norm(g, broad, "")).collect::<Vec<_>>().join(" | ");
            let mine_text = mine.clone().unwrap_or_default();
            rows.add(name, split, mine.is_some(), ok, || format!("{word}\t{mine_text}\t{shown}"));
        }
    }
}

/// Përputhje e gjerë që pranon të dyja konventat e «ë»-së fundore (e ruajtur ose e rënë).
fn ipa_either_rows(ents: &[Entry], lemmas: &BTreeMap<&str, &str>, rows: &mut Rows) {
    let mut by_word: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in ents.iter().filter(|e| usable(e)) {
        by_word.entry(e.word.as_str()).or_default().extend(e.ipas.iter().map(String::as_str).filter(|g| clean_gold_ipa(g)));
    }
    for (word, golds) in by_word.iter().filter(|(_, g)| !g.is_empty()) {
        let ours = fon::to_ipa(word).map(|o| norm(&o, true, ""));
        let drop_final = lower(word).ends_with('ë') && fon::letters(word).iter().filter(|u| fon::is_vowel(u)).count() >= 2;
        let variants: Vec<String> = ours.iter().flat_map(|o| [o.clone(), o.strip_suffix('ə').filter(|_| drop_final).map(str::to_string).unwrap_or_else(|| o.clone())]).collect();
        let ok = golds.iter().any(|g| variants.contains(&norm(g, true, "")));
        let shown = format!("{word}	{variants:?}");
        rows.add("ipa exact, broad, either final-ë convention", split_by(lemmas, word), ours.is_some(), ok, || shown);
    }
}

fn norm_hyph(s: &str) -> String {
    lower(s)
        .chars()
        .filter(|c| *c != '\u{301}' && *c != '\u{300}')
        .map(|c| match c {
            'á' | 'à' | 'â' => 'a',
            'é' | 'è' | 'ê' => 'e',
            'í' | 'ì' | 'î' => 'i',
            'ó' | 'ò' | 'ô' => 'o',
            'ú' | 'ù' | 'û' => 'u',
            'ý' | 'ỳ' | 'ŷ' => 'y',
            'ɛ' => 'e',
            'ə' => 'ë',
            'ɡ' => 'g',
            '‧' | '·' => '-',
            other => other,
        })
        .collect()
}

const ACCENTED: &str = "áàâéèêíìîóòôúùûýỳŷ";

fn our_hyphenation(head: &str) -> Option<String> {
    let parts: Option<Vec<Vec<String>>> = head.split([' ', '-']).filter(|p| !p.is_empty()).map(fon::syllables).collect();
    parts.map(|p| lower(&p.concat().join("-")))
}

fn hyph_rows(ents: &[Entry], lemmas: &BTreeMap<&str, &str>, rows: &mut Rows) {
    let mut by_word: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for e in ents.iter().filter(|e| !SKIP_POS.contains(&e.pos.as_str())) {
        let key: String = lower(&e.word).chars().filter(|c| *c != ' ' && *c != '-').collect();
        let valid = e.hyph.iter().map(|h| norm_hyph(h)).filter(|h| h.replace('-', "") == key);
        by_word.entry(e.word.as_str()).or_default().extend(valid);
    }
    for (word, golds) in by_word.iter().filter(|(_, g)| !g.is_empty()) {
        let ours = our_hyphenation(word);
        let ok = ours.as_ref().map(|o| golds.contains(o)).unwrap_or(false);
        let shown = format!("{word}\t{}\t{}", ours.clone().unwrap_or_default(), golds.join(" | "));
        rows.add("syllables vs hyphenation", split_by(lemmas, word), ours.is_some(), ok, || shown);
    }
}

/// Kufijtë e rrokjeve të IPA-s si indekse shkronjash, kur IPA përputhet me shkronjat, dhe a
/// janë shënuar të gjithë kufijtë (çdo pjesë mes shenjave ka një zanore të vetme).
fn gold_cuts(word: &str, gold: &str) -> Option<(BTreeSet<usize>, bool)> {
    let marked = norm(gold, true, ".ˈˌ").replace(['ˈ', 'ˌ'], ".");
    let marked = marked.trim_matches('.');
    let phones: Vec<String> = fon::letters(word).iter().map(|u| fon::canonical(u).and_then(fon::ipa_of).map(|p| norm(p, true, ""))).collect::<Option<Vec<_>>>()?;
    let mut edge = vec![0usize];
    for p in &phones {
        edge.push(edge.last().copied().unwrap_or(0) + p.chars().count());
    }
    let flat: String = marked.chars().filter(|c| *c != '.').collect();
    (flat == phones.concat()).then_some(())?;
    let mut cuts = BTreeSet::new();
    let mut at = 0usize;
    for c in marked.chars() {
        match c {
            '.' => {
                cuts.insert(edge.iter().position(|e| *e == at)?);
            }
            _ => at += 1,
        }
    }
    let fully = marked.split('.').all(|chunk| chunk.chars().filter(|c| IPA_VOWELS.contains(*c)).count() == 1);
    (!cuts.is_empty()).then_some((cuts, fully))
}

fn ipa_syllable_rows(ents: &[Entry], lemmas: &BTreeMap<&str, &str>, rows: &mut Rows) {
    let mut by_word: BTreeMap<&str, Vec<(BTreeSet<usize>, bool)>> = BTreeMap::new();
    for e in ents.iter().filter(|e| usable(e)) {
        let cuts = e.ipas.iter().filter(|g| clean_gold_ipa(g)).filter_map(|g| gold_cuts(&e.word, g));
        by_word.entry(e.word.as_str()).or_default().extend(cuts);
    }
    for (word, golds) in by_word.iter().filter(|(_, g)| !g.is_empty()) {
        let split = split_by(lemmas, word);
        let syl = fon::syllables(word);
        let ours: Option<BTreeSet<usize>> = syl.as_ref().map(|s| {
            s.iter()
                .scan(0usize, |acc, x| {
                    *acc += fon::letters(x).len();
                    Some(*acc)
                })
                .filter(|c| *c < fon::letters(word).len())
                .collect()
        });
        let shown = format!("{word}\t{:?}\t{:?}", syl, golds);
        let subset = ours.as_ref().map(|o| golds.iter().any(|(g, _)| g.is_subset(o))).unwrap_or(false);
        rows.add("syllables vs IPA, marked cuts only (gold ⊆ ours)", split, ours.is_some(), subset, || shown.clone());
        let full: Vec<&BTreeSet<usize>> = golds.iter().filter(|(_, f)| *f).map(|(g, _)| g).collect();
        let exact = ours.as_ref().map(|o| full.iter().any(|g| *g == o)).unwrap_or(false);
        match full.is_empty() {
            true => (),
            false => rows.add("syllables vs IPA, all cuts marked (exact)", split, ours.is_some(), exact, || shown),
        }
    }
}

fn vowel_letters(word: &str) -> usize {
    fon::letters(word).iter().filter(|u| fon::is_vowel(u)).count()
}

fn gold_stress_ipa(word: &str, gold: &str) -> Option<usize> {
    let s = norm(gold, true, "ˈ");
    (s.matches('ˈ').count() == 1).then_some(())?;
    let before = s.split('ˈ').next()?.chars().filter(|c| IPA_VOWELS.contains(*c)).count();
    let total = s.chars().filter(|c| IPA_VOWELS.contains(*c)).count();
    let v = vowel_letters(word);
    let dropped = total + 1 == v && lower(word).ends_with('ë') && !s.ends_with('ə');
    let aligned = total == v || dropped;
    (aligned && before < v).then_some(before)
}

fn gold_stress_hyph(word: &str, hyph: &str) -> Option<usize> {
    let low = lower(hyph);
    (low.chars().filter(|c| ACCENTED.contains(*c)).count() == 1).then_some(())?;
    let plain = norm_hyph(&low).replace('-', "");
    (plain == lower(word)).then_some(())?;
    let at = low.replace('-', "").chars().position(|c| ACCENTED.contains(c))?;
    Some(plain.chars().take(at).filter(|c| "aeëiouy".contains(*c)).count())
}

fn lookup<'a>(lex: &'a Lexicon, w: &str, pos: Pos) -> Option<&'a Lexeme> {
    let low = lower(w);
    let noun = match pos {
        Pos::Propn => Pos::Noun,
        other => other,
    };
    lex.standard(w, pos).or_else(|| lex.standard(&low, pos)).or_else(|| lex.standard(&low, noun))
}

fn cc_initial(w: &str) -> bool {
    lower(w).starts_with(['c', 'ç'])
}

fn stress_rows(ents: &[Entry], lex: &Lexicon, rows: &mut Rows) {
    let mut items: BTreeMap<(String, String, Option<String>), BTreeSet<usize>> = BTreeMap::new();
    for e in ents.iter().filter(|e| usable(e)) {
        let from_ipa = e.ipas.iter().filter(|g| clean_gold_ipa(g)).filter_map(|g| gold_stress_ipa(&e.word, g));
        let from_hyph = e.hyph.iter().filter_map(|h| gold_stress_hyph(&e.word, h));
        items.entry((e.word.clone(), e.pos.clone(), e.lemma_of.clone())).or_default().extend(from_ipa.chain(from_hyph));
    }
    for ((word, pos, lemma_of), golds) in items.iter().filter(|(_, g)| !g.is_empty()) {
        let p = pos_of(pos);
        let vowel = |h: Hint| fon::stress(word, h).map(|s| s.vowel);
        let hit = |o: Option<usize>| o.map(|x| golds.contains(&x)).unwrap_or(false);
        let shown = |o: Option<usize>, h: &str| format!("{word}\t{pos}\t{h}\t{o:?}\t{golds:?}");
        let table = fon::STRESSED_SCHWA.contains(&lower(word).as_str());
        match (vowel_letters(word) >= 2, lemma_of) {
            (false, _) => (),
            (true, None) => {
                let split = split_of(word);
                let lx = lookup(lex, word, p);
                let o = vowel(lx.map(Hint::Lexeme).unwrap_or(Hint::Lemma(p)));
                let s = shown(o, &lx.map(|l| l.paradigms.join(",")).unwrap_or_default());
                rows.add("stress, lemmas (lexeme or POS hint)", split, o.is_some(), hit(o), || s.clone());
                match table {
                    true => (),
                    false => rows.add("stress, lemmas without STRESSED_SCHWA words", split, o.is_some(), hit(o), || s.clone()),
                }
                match lx.filter(|l| l.standard).is_some() {
                    true => rows.add("stress, lemmas with a standard uniparser entry", split, o.is_some(), hit(o), || s.clone()),
                    false => (),
                }
                match cc_initial(word) {
                    true => (),
                    false => rows.add("stress, lemmas not starting with c/ç", split, o.is_some(), hit(o), || s.clone()),
                }
                let u = vowel(Hint::Unknown);
                rows.add("stress, lemmas (unknown hint)", split, u.is_some(), hit(u), || shown(u, "unknown"));
            }
            (true, Some(l)) => {
                let split = split_of(l);
                let o = vowel(Hint::Unknown);
                rows.add("stress, inflected forms (unknown hint)", split, o.is_some(), hit(o), || shown(o, "unknown"));
                let o1 = vowel(Hint::Inflected);
                rows.add("stress, inflected forms (inflected hint)", split, o1.is_some(), hit(o1), || shown(o1, "inflected"));
                match table {
                    true => (),
                    false => rows.add("stress, inflected forms without STRESSED_SCHWA words (inflected hint)", split, o1.is_some(), hit(o1), || shown(o1, "inflected")),
                }
                let lx = lookup(lex, l, p);
                let o2 = vowel(lx.map(Hint::Lexeme).unwrap_or(Hint::Inflected));
                let s2 = shown(o2, l);
                rows.add("stress, inflected forms (lexeme of linked lemma)", split, o2.is_some(), hit(o2), || s2.clone());
                match cc_initial(word) {
                    true => (),
                    false => rows.add("stress, inflected forms not starting with c/ç (lexeme of linked lemma)", split, o2.is_some(), hit(o2), || s2),
                }
            }
        }
    }
}

fn strip_diacritics(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ë' => 'e',
            'Ë' => 'E',
            'ç' => 'c',
            'Ç' => 'C',
            other => other,
        })
        .collect()
}

/// Çdo fjalë e UD-së (trajtë e lakuar ose lemë) që pa diakritikë ka të paktën një «e» ose «c».
fn restore_rows(lex: &Lexicon, rows: &mut Rows) {
    let restorer = fon::Restorer::new(lex);
    let mut types: BTreeMap<String, String> = BTreeMap::new();
    for f in UD {
        let text = std::fs::read_to_string(Path::new(DATA).join(f)).unwrap();
        let cols = text.lines().filter(|l| !l.starts_with('#')).map(|l| l.split('\t').collect::<Vec<_>>());
        for c in cols.filter(|c| c.len() == 10 && c[0].parse::<usize>().is_ok() && c[1].chars().all(char::is_alphabetic)) {
            types.entry(c[1].to_string()).or_insert(c[2].to_string());
        }
    }
    for (form, lemma) in &types {
        let input = strip_diacritics(form);
        (input.chars().any(|c| matches!(c, 'e' | 'E' | 'c' | 'C'))).then_some(()).into_iter().for_each(|_| {
            let split = split_of(&lower(lemma));
            let ours = restorer.restore(&input);
            let ok = ours.as_deref() == Some(form.as_str());
            let shown = || format!("{input}\t{}\t{form}\t{lemma}", ours.clone().unwrap_or_default());
            let is_lemma = lower(form) == lower(lemma);
            let closed = fon::FUNCTION_FORMS.contains(&lower(lemma).as_str());
            rows.add("restore, UD types with e/c, all forms", split, ours.is_some(), ok, shown);
            let kind = match is_lemma {
                true => "restore, UD types with e/c, lemma forms",
                false => "restore, UD types with e/c, inflected forms",
            };
            rows.add(kind, split, ours.is_some(), ok, shown);
            match closed {
                true => (),
                false => rows.add("restore, UD types with e/c, without FUNCTION_FORMS lemmas", split, ours.is_some(), ok, shown),
            }
        });
    }
}

#[test]
fn gold_measurement() {
    let lex = Lexicon::load(Path::new(DATA)).unwrap();
    let ents = entries();
    let lemmas = split_lemmas(&ents);
    let mut rows = Rows { rows: Vec::new(), dev: Vec::new(), show: std::env::var("FON_DEV").unwrap_or_default() };
    ipa_rows(&ents, &lemmas, &mut rows);
    ipa_either_rows(&ents, &lemmas, &mut rows);
    hyph_rows(&ents, &lemmas, &mut rows);
    ipa_syllable_rows(&ents, &lemmas, &mut rows);
    stress_rows(&ents, &lex, &mut rows);
    restore_rows(&lex, &mut rows);

    rows.dev.iter().take(1500).for_each(|l| eprintln!("DEV\t{l}"));
    eprintln!("{:<72} {:<5} {:>6} {:>6} {:>7} {:>9} {:>9} {:>9} {:>8}", "metric", "split", "total", "answ", "correct", "precision", "coverage", "wilson95", "PER");
    for (m, t) in &rows.rows {
        for (s, x) in [("Dev", t[0]), ("Test", t[1])] {
            let per = match x.len {
                0 => "-".to_string(),
                l => format!("{:.4}", x.err as f64 / l as f64),
            };
            eprintln!("{:<72} {:<5} {:>6} {:>6} {:>7} {:>9.4} {:>9.4} {:>9.4} {:>8}", m, s, x.total, x.answered, x.correct, precision(&x), coverage(&x), wilson_lower(x.correct, x.answered), per);
        }
    }
    eprintln!(
        "tables: STRESSED_SCHWA = {} function words (stress R-T9); FUNCTION_FORMS = {} closed-class words (restoration); paradigm classes = uniparser endings, not word lists",
        fon::STRESSED_SCHWA.len(),
        fon::FUNCTION_FORMS.len()
    );

    let floor = |m: &str, p: f64, c: f64| {
        let t = rows.get(m, Split::Test);
        assert!(precision(&t) >= p, "{m}: Test precision {:.4} < {p}", precision(&t));
        assert!(coverage(&t) >= c, "{m}: Test coverage {:.4} < {c}", coverage(&t));
    };
    floor("ipa exact, strict", 0.715, 0.99);
    floor("ipa exact, broad", 0.90, 0.99);
    floor("ipa exact, broad, either final-ë convention", 0.925, 0.99);
    floor("syllables vs hyphenation", 0.97, 0.92);
    floor("syllables vs IPA, marked cuts only (gold ⊆ ours)", 0.965, 0.90);
    floor("syllables vs IPA, all cuts marked (exact)", 0.96, 0.92);
    floor("stress, lemmas (lexeme or POS hint)", 0.965, 0.895);
    floor("stress, lemmas without STRESSED_SCHWA words", 0.965, 0.895);
    floor("stress, lemmas with a standard uniparser entry", 0.97, 0.955);
    floor("stress, lemmas not starting with c/ç", 0.96, 0.885);
    floor("stress, lemmas (unknown hint)", 0.955, 0.32);
    floor("stress, inflected forms (unknown hint)", 0.99, 0.145);
    floor("stress, inflected forms (inflected hint)", 0.98, 0.935);
    floor("stress, inflected forms without STRESSED_SCHWA words (inflected hint)", 0.98, 0.935);
    floor("stress, inflected forms (lexeme of linked lemma)", 0.98, 0.955);
    floor("stress, inflected forms not starting with c/ç (lexeme of linked lemma)", 0.965, 0.945);
    floor("restore, UD types with e/c, all forms", 0.99, 0.695);
    floor("restore, UD types with e/c, lemma forms", 0.99, 0.57);
    floor("restore, UD types with e/c, inflected forms", 0.985, 0.76);
    floor("restore, UD types with e/c, without FUNCTION_FORMS lemmas", 0.99, 0.70);
    let per = rows.get("ipa exact, broad", Split::Test);
    assert!((per.err as f64) / (per.len.max(1) as f64) <= 0.025, "broad phoneme error rate above 2.5%");
}
