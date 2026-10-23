//! Theksi i fjalës: rrokja e theksuar, e numëruar nga fundi (0 = rrokja e fundit), dhe
//! zanorja e theksuar, e numëruar nga fillimi i fjalës (0 = zanorja e parë).
//!
//! Theksi i shqipes është leksikor, por i rregullt: bie në rrokjen e fundit të temës dhe
//! mbetet aty në lakim e zgjedhim; mbaresat dhe prapashtesat «-ë», «-ëz», «-ull», «-ur» nuk
//! theksohen. Rregullat:
//! - R-T1 fjala njërrokëshe; R-T2 «-ëj» e foljeve të tipit bëj (për-bëj): rrokja e fundit;
//! - R-T3 «ë»-ja e rrokjes së fundit nuk theksohet (bo-të, mo-tër), as «-ëz-ë» (ca-gë-zë), as
//!   «-ull» (i-shull); në lemë dhe në fjalën pa dëshmi rrokja e mbyllur me «ë» merr R-T3 vetëm
//!   me prapashtesat vendase «-ël», «-ëll», «-ër», «-ërr», «-ëm», «-ëz», «-ës», «-ët», «-ën»
//!   (huazimet va-këf, kë-llëç jo), në trajtën e lakuar me çdo mbaresë (ha-pësh);
//! - R-T4 trajta e lakuar ruan theksin e lemës kur ndan me lemën ose me një temë të saj
//!   zanoren e theksuar (lu-le → lul-ja, pu-noj → pu-nu-am);
//! - R-T5 klasa e paradigmës (uniparser): 1c, 2a, 2b, 3c, 3d mbarojnë me zanore të theksuar
//!   (ba-ba, mu-ze, ka-la, shtë-pi), 3b me «-e» të patheksuar (lu-le, qe-ni-e), 3f me rrokje
//!   fundore të patheksuar (bu-sull);
//! - R-T6 pjesa e ligjëratës së lemës: folja «-em», «-ur» dhe me zanore në fund e parafundit
//!   (la-hem, e-ci), folja e tipit zë, vë, lë, nxë te rrënja (për-zë), folja tjetër, emri, mbiemri e
//!   ndajfolja me bashkëtingëllore në fund te rrokja e fundit (pu-noj, shqip-tar), me
//!   përjashtimet e prapashtesave të patheksuara (i ha-pur, fsheh-tas, tër-tho-ra-zi);
//! - R-T7 mbaresat vetvetore «-hem», «-het», «-hemi», «-heshin» (pu-no-he-mi, du-het);
//! - R-T8 mbaresat njërrokëshe të trajtës së lakuar (ma-lin, pu-no-va, gju-hë-ve, ar-sy-e-ja);
//! - R-T9 fjalët e shërbimit me «ë» fundore të theksuar (`STRESSED_SCHWA`: as-një, as-gjë,
//!   di-kë, kë-të).
//! Rendi sipas dëshmisë: `Unknown` R-T1–3, R-T9; `Inflected` edhe R-T7, R-T8; `Lemma` R-T1–3,
//! R-T9, R-T6; `Lexeme` për lemën edhe R-T5, për trajtën R-T4, R-T7, R-T8.
//! Kur asnjë rregull nuk vlen, kthehet None: fjala nuk theksohet me hamendje. None kthehet
//! edhe për «atë» (përemër a-të ~ emër a-të) pa dëshmi emri, për «ë»-në fundore të përemrave
//! jashtë `STRESSED_SCHWA`, për emrat «-ur» (le-pur ~ fla-mur), për emrat e mbiemrat «-as»
//! (ti-ra-nas), për foljet «-et» pa «h» (gjen-det ~ për-shën-det), për
//! «-ura», «-uri» të trajtave pa lemë (bu-ku-ra ~ kul-tu-ra) dhe kur çiftet e pasigurta
//! ie, ye, ua, ue (py-e-tje ~ die-lli) japin theks tjetër në secilin lexim.

use super::alfabeti::is_vowel;
use super::rrokjet::{canonical_letters, nuclei, pairs, Pair, Reading};
use crate::gramatika::leksiku::Lexeme;
use crate::gramatika::tipet::Pos;

/// Fjalët e shërbimit me «ë» fundore të theksuar: të përbërat me një, gjë dhe kallëzoret
/// «-kë» të përemrave pyetës e të pacaktuar, si edhe «këtë».
pub const STRESSED_SCHWA: [&str; 24] = [
    "asnjë", "ndonjë", "gjithnjë", "çdonjë", "kurrnjë", "asndonjë", "asgjë", "kurrgjë", "çdogjë", "hiçgjë", "mosgjë", "askurrgjë", "hiçasgjë", "hiçmosgjë", "dikë", "askë", "gjithkë", "ndokë",
    "kurrkë", "moskë", "sekë", "çdokë", "askurrkë", "këtë",
];

/// Dëshmia që shoqëron fjalën.
#[derive(Debug, Clone, Copy)]
pub enum Hint<'a> {
    /// Fjalë e çfarëdoshme, pa dëshmi tjetër (mund të jetë edhe lemë).
    Unknown,
    /// Trajtë e lakuar ose e zgjedhuar (jo trajta e fjalorit), me lemë të panjohur.
    Inflected,
    /// Trajta e fjalorit (lema) e kësaj pjese të ligjëratës.
    Lemma(Pos),
    /// Leksema e gramatikës: fjala është lema e saj ose një trajtë e saj.
    Lexeme(&'a Lexeme),
}

/// Rrokja e theksuar nga fundi (bërthamat sipas `syllable_count`), zanorja e theksuar nga
/// fillimi dhe rregulli që i dha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stress {
    pub from_end: usize,
    pub vowel: usize,
    pub rule: &'static str,
}

/// Vendimi i një rregulli: None = rregulli nuk vlen; Some(None) = rregulli e mban fjalën pa theks.
type Verdict = Option<Option<Stress>>;

struct Shape {
    low: Vec<&'static str>,
    nuc: Vec<(usize, usize)>,
    reading: Reading,
}

fn vowels_in(s: &[&str]) -> usize {
    s.iter().filter(|l| is_vowel(l)).count()
}

impl Shape {
    fn of(low: &[&'static str], reading: Reading) -> Option<Shape> {
        let nuc = nuclei(low, reading);
        (!nuc.is_empty()).then(|| Shape { low: low.to_vec(), nuc, reading })
    }

    fn n(&self) -> usize {
        self.nuc.len()
    }

    fn ends_with(&self, tail: &[&str]) -> bool {
        self.low.ends_with(tail)
    }

    fn last(&self) -> &'static str {
        self.low.last().copied().unwrap_or("")
    }

    fn last_nucleus(&self) -> &[&'static str] {
        self.nucleus_from_end(0)
    }

    fn ends_in_vowel(&self) -> bool {
        is_vowel(self.last())
    }

    /// Numri i zanoreve para bërthamës së theksuar.
    fn ordinal(&self, from_end: usize) -> Option<usize> {
        let i = self.n().checked_sub(from_end + 1)?;
        let start = self.nuc.get(i)?.0;
        self.low.get(..start).map(vowels_in)
    }

    /// Bërthama që nis me zanoren e `ord`-të, e numëruar nga fundi.
    fn from_ordinal(&self, ord: usize) -> Option<usize> {
        let i = self.nuc.iter().position(|(a, _)| self.low.get(..*a).map(vowels_in) == Some(ord))?;
        self.n().checked_sub(i + 1)
    }

    fn nucleus_from_end(&self, k: usize) -> &[&'static str] {
        self.n().checked_sub(k + 1).and_then(|i| self.nuc.get(i)).and_then(|(a, b)| self.low.get(*a..*b)).unwrap_or(&[])
    }

    /// «-ur»/«-un» e pjesores (hapur, ikur, punun).
    fn participle(&self) -> bool {
        self.n() >= 2 && (self.ends_with(&["u", "r"]) || self.ends_with(&["u", "n"]))
    }

    /// Diftongu «ua» në rrokjen e fundit të mbyllur jashtë pjesores (trotuar, tatuazh, seksual).
    fn ua_closed(&self) -> bool {
        self.nucleus_from_end(0) == ["u", "a"] && !self.ends_in_vowel()
    }

    /// Shkronjat pas `k` shkronjave të fundit: (para tyre, fundi).
    fn tail(&self, k: usize) -> (&'static str, &[&'static str]) {
        let start = self.low.len().checked_sub(k);
        let before = start.and_then(|i| i.checked_sub(1)).and_then(|i| self.low.get(i)).copied().unwrap_or("");
        (before, start.and_then(|i| self.low.get(i..)).unwrap_or(&[]))
    }

    /// Mbaresa vetvetore pas zanoreje: rrokjet e patheksuara pas temës (la-hem, pu-no-he-mi).
    fn mediopassive(&self) -> Option<usize> {
        let (b3, t3) = self.tail(3);
        let (b4, t4) = self.tail(4);
        let (b5, t5) = self.tail(5);
        let one = is_vowel(b3) && matches!(t3, ["h", "e", "m" | "t" | "n" | "sh" | "j"]);
        let two = (is_vowel(b4) && matches!(t4, ["h", "e", "m" | "n", "i"] | ["h", "e", "sh", "a" | "e"]))
            || (is_vowel(b5) && matches!(t5, ["h", "e", "sh", "i", "m" | "t" | "n"]));
        match (one, two) {
            (true, _) => Some(1),
            (false, true) => Some(2),
            (false, false) => None,
        }
    }

    /// Rrokja e fundit është e patheksuar; para saj kapërcehen edhe «ë»-ja e patheksuar,
    /// «-e»-ja e patheksuar pas «i», «y» (ar-sy-e-ja) dhe, në trajtat e lakuara, prapashtesa
    /// «-ull» (gju-hë-ve, ca-ku-lli). Në trajtat e lakuara pa lemë «-ur-»/«-un-» para mbaresës
    /// nuk vendoset (bu-ku-ra ~ kul-tu-ra).
    fn before_unstressed_final(&self, inflected: bool) -> Option<usize> {
        let penult = self.nucleus_from_end(1);
        let spans = self.n().checked_sub(2).and_then(|i| self.nuc.get(i)).copied().zip(self.nuc.last().copied());
        let between = spans.and_then(|((_, b), (c, _))| self.low.get(b..c)).unwrap_or(&[]);
        let prior = spans.and_then(|((a, _), _)| a.checked_sub(1)).and_then(|i| self.low.get(i)).copied().unwrap_or("");
        let ull = inflected && penult == ["u"] && between == ["ll"];
        let ur = inflected && penult == ["u"] && matches!(between, ["r"] | ["n"]);
        let hiatus_e = penult == ["e"] && matches!(prior, "i" | "y") && between == ["j"];
        let skip = penult == ["ë"] || ull || hiatus_e;
        match (self.n() >= 3, ur, skip) {
            (true, true, _) => None,
            (true, false, true) => Some(2),
            _ => Some(1),
        }
    }
}

fn at(sh: &Shape, from_end: usize, rule: &'static str) -> Option<Stress> {
    sh.ordinal(from_end).map(|vowel| Stress { from_end, vowel, rule })
}

/// Rregull në listë vendimi: kushti, rrokja nga fundi (None = nuk vendoset) dhe emri.
type Step = (bool, Option<usize>, &'static str);

/// Kushti i parë i vërtetë vendos; një hap me None e mban fjalën pa theks.
fn decide(sh: &Shape, steps: &[Step]) -> Verdict {
    steps.iter().find(|(c, _, _)| *c).map(|(_, f, r)| f.and_then(|f| at(sh, f, r)))
}

/// Çfarë dihet për fjalën kur zbatohen rregullat e përgjithshme.
#[derive(Clone, Copy)]
struct Ctx {
    pos: Option<Pos>,
    lemma: bool,
    form: bool,
}

/// Rregullat që vlejnë për çdo trajtë.
fn general(sh: &Shape, ctx: Ctx) -> Verdict {
    let schwa = sh.last_nucleus() == ["ë"];
    let word = schwa.then(|| sh.low.concat()).unwrap_or_default();
    let closed = matches!(ctx.pos, Some(Pos::Pron | Pos::Det | Pos::Other));
    let noun = matches!(ctx.pos, Some(Pos::Noun | Pos::Propn));
    let coda = sh.nuc.last().and_then(|(_, b)| sh.low.get(*b..)).unwrap_or(&[]);
    let native_coda = matches!(coda, [] | ["l" | "ll" | "r" | "rr" | "m" | "z" | "s" | "t" | "n"]);
    let root = ["z", "v", "l", "nx"].iter().any(|c| sh.ends_with(&[*c, "ë"]));
    let root_verb = ctx.lemma && ctx.pos == Some(Pos::Verb) && root;
    decide(
        sh,
        &[
            (sh.n() == 1, Some(0), "R-T1: fjalë njërrokëshe"),
            (sh.ends_with(&["ë", "j"]), Some(0), "R-T2: «-ëj» e foljeve të tipit bëj"),
            (STRESSED_SCHWA.contains(&word.as_str()), Some(0), "R-T9: fjalë shërbimi me «ë» fundore të theksuar"),
            (word == "atë" && !noun, None, ""),
            (closed && schwa, None, ""),
            (root_verb, Some(0), "R-T6: folje e tipit zë, vë, lë, nxë: rrënja e theksuar"),
            (schwa && !native_coda && !ctx.form, None, ""),
            (sh.n() >= 3 && sh.ends_with(&["ë", "z", "ë"]), Some(2), "R-T3: prapashtesat «-ëz» dhe «-ë» nuk theksohen"),
            (schwa, Some(1), "R-T3: «ë» e rrokjes së fundit nuk theksohet"),
            (sh.n() >= 2 && sh.ends_with(&["u", "ll"]), Some(1), "R-T3: prapashtesa «-ull» nuk theksohet"),
        ],
    )
}

const FINAL_VOWEL_CLASSES: [&str; 6] = ["1c", "2a", "2b", "3c", "3c-1", "3d"];

fn by_class(sh: &Shape, lx: &Lexeme) -> Verdict {
    let has = |c: &str| lx.paradigms.iter().any(|p| p == c);
    let final_vowel = FINAL_VOWEL_CLASSES.iter().any(|c| has(c));
    let vowel = sh.ends_in_vowel();
    decide(
        sh,
        &[
            (vowel && final_vowel && !has("3b"), Some(0), "R-T5: klasë me zanore të theksuar në fund (1c, 2a, 2b, 3c, 3d)"),
            (vowel && has("3b") && !final_vowel && sh.last() == "e", sh.before_unstressed_final(false), "R-T5: klasa 3b, «-e» e patheksuar"),
            (!vowel && has("3f") && !sh.ua_closed(), Some(1), "R-T5: klasa 3f, rrokja e fundit e patheksuar"),
        ],
    )
}

/// Mbaresat vetvetore pas zanoreje nuk theksohen (la-hem, pu-no-het, pu-no-he-mi).
fn mediopassive(sh: &Shape) -> Verdict {
    sh.mediopassive().map(|f| at(sh, f, "R-T7: mbaresat vetvetore «-hem», «-het», «-hemi», «-heshin» nuk theksohen"))
}

/// Trajta e lakuar: mbaresat njërrokëshe janë të patheksuara, theksi mbetet në temë.
fn inflected(sh: &Shape) -> Verdict {
    let last = sh.nucleus_from_end(0);
    let coda = sh.nuc.last().and_then(|(_, b)| sh.low.get(*b..)).unwrap_or(&[]);
    let simple = matches!(last, ["a"] | ["e"] | ["i"] | ["u"]);
    let ending = matches!(coda, [] | ["t"] | ["n"] | ["s"] | ["sh"]) || (coda == ["m"] && last != ["i"]);
    decide(
        sh,
        &[
            (last == ["u", "a"] && matches!(coda, ["r"] | ["m"] | ["t"] | ["n"]), Some(0), "R-T8: «-uar», «-uam», «-uat», «-uan»: theksi te «u»-ja e temës"),
            (sh.participle(), Some(1), "R-T8: pjesore «-ur», «-un»"),
            (sh.n() >= 3 && (sh.ends_with(&["ë", "v", "e"]) || sh.ends_with(&["ë", "v", "e", "t"])), Some(2), "R-T8: «-ëve», «-ëvet»: dy rrokje të patheksuara"),
            ((sh.ends_with(&["v", "e"]) || sh.ends_with(&["v", "e", "t"])) && !sh.ends_with(&["o", "v", "e"]), None, ""),
            (last == ["o"] && matches!(coda, ["n"] | ["j"]), Some(0), "R-T8: «-on», «-oj» e foljes: theksi në temë"),
            (simple && ending, sh.before_unstressed_final(true), "R-T8: mbaresë njërrokëshe e patheksuar"),
        ],
    )
}

fn verb(sh: &Shape) -> Verdict {
    decide(
        sh,
        &[
            (sh.ends_with(&["e", "m"]), Some(1), "R-T6: folje vetvetore «-em»"),
            (sh.mediopassive().is_some(), sh.mediopassive(), "R-T7: folje vetvetore «-het», «-hem»"),
            (sh.n() >= 2 && sh.ends_with(&["e", "t"]), None, ""),
            (sh.participle(), Some(1), "R-T6: pjesore «-ur», «-un»"),
            (sh.ends_in_vowel(), Some(1), "R-T6: folje me zanore në fund (eci, iki)"),
            (true, Some(0), "R-T6: folje me bashkëtingëllore në fund (punoj, kërcas)"),
        ],
    )
}

fn noun(sh: &Shape) -> Verdict {
    let after_vowel = sh.low.len().checked_sub(2).and_then(|i| sh.low.get(i)).map(|l| is_vowel(l)).unwrap_or(false);
    let diphthong_end = sh.ends_in_vowel() && sh.last_nucleus().len() == 2;
    decide(
        sh,
        &[
            (sh.ua_closed(), None, ""),
            (sh.participle(), None, ""),
            (sh.n() >= 2 && sh.ends_with(&["a", "s"]), None, ""),
            (diphthong_end, Some(0), "R-T6: emër me diftong të theksuar në fund (për-rua)"),
            (!sh.ends_in_vowel(), Some(0), "R-T6: emër me bashkëtingëllore në fund"),
            (sh.last() == "e", sh.before_unstressed_final(false), "R-T6: emër me «-e» të patheksuar"),
            (sh.last() == "i", Some(0), "R-T6: emër me «-i» të theksuar (liri, shtëpi)"),
            (sh.last() == "o" && !after_vowel, Some(1), "R-T6: emër me «-o» të patheksuar (fjongo, piano)"),
        ],
    )
}

fn adjective(sh: &Shape) -> Verdict {
    decide(
        sh,
        &[
            (sh.ends_in_vowel(), None, ""),
            (sh.participle(), Some(1), "R-T6: mbiemër prej pjesoreje «-ur», «-un»"),
            (sh.ua_closed() && sh.last() != "r", None, ""),
            (sh.n() >= 2 && sh.ends_with(&["a", "s"]), None, ""),
            (true, Some(0), "R-T6: mbiemër me bashkëtingëllore në fund"),
        ],
    )
}

fn adverb(sh: &Shape) -> Verdict {
    decide(
        sh,
        &[
            (sh.n() >= 3 && sh.ends_with(&["a", "z", "i"]), Some(2), "R-T6: ndajfolje «-azi», prapashtesë e patheksuar"),
            (sh.ends_with(&["th", "i"]), Some(1), "R-T6: ndajfolje «-thi», prapashtesë e patheksuar"),
            (sh.ends_in_vowel(), None, ""),
            (sh.ends_with(&["a", "s"]), Some(1), "R-T6: ndajfolje «-as», prapashtesë e patheksuar"),
            (sh.ends_with(&["a", "sh"]), None, ""),
            (sh.participle(), Some(1), "R-T6: ndajfolje prej pjesoreje «-ur»"),
            (true, Some(0), "R-T6: ndajfolje me bashkëtingëllore në fund"),
        ],
    )
}

fn numeral(sh: &Shape) -> Verdict {
    decide(sh, &[(!sh.ends_in_vowel(), Some(0), "R-T6: numëror me bashkëtingëllore në fund")])
}

fn by_pos(sh: &Shape, pos: Pos) -> Verdict {
    match pos {
        Pos::Verb => verb(sh),
        Pos::Noun => noun(sh),
        Pos::Adj => adjective(sh),
        Pos::Adv => adverb(sh),
        Pos::Num => numeral(sh),
        _ => None,
    }
}

fn lemma_stress(sh: &Shape, pos: Pos, lx: Option<&Lexeme>) -> Option<Stress> {
    let surname = lx.map(|l| l.paradigms.iter().any(|p| p == "surname")).unwrap_or(false);
    let lexical = || match surname {
        true => Some(None),
        false => lx.and_then(|l| by_class(sh, l)).or_else(|| by_pos(sh, pos)),
    };
    general(sh, Ctx { pos: Some(pos), lemma: true, form: false }).or_else(lexical).flatten()
}

/// Theksi në të dy leximet e çifteve të pasigurta ie, ye, ua, ue; jepet vetëm kur të dy
/// leximet bien në të njëjtën zanore dhe në të njëjtën rrokje nga fundi.
fn resolve(low: &[&'static str], rule: impl Fn(&Shape) -> Option<Stress>) -> Option<Stress> {
    let join = Shape::of(low, Reading::Join).and_then(|sh| rule(&sh));
    match pairs(low).contains(&Pair::Unsure) {
        false => join,
        true => {
            let split = Shape::of(low, Reading::Split).and_then(|sh| rule(&sh));
            join.filter(|j| split.map(|s| s.from_end == j.from_end && s.vowel == j.vowel).unwrap_or(false))
        }
    }
}

/// Lema në një lexim të çifteve dhe theksi i saj.
type LemmaStress = Option<(Shape, Option<Stress>)>;

/// Theksi i lemës dhe i trajtave të një lekseme, me lemën dhe temat e përgatitura një herë.
pub struct LexemeStress<'a> {
    lx: &'a Lexeme,
    lemma: Option<Vec<&'static str>>,
    stems: Vec<Vec<&'static str>>,
    join: LemmaStress,
    split: LemmaStress,
}

impl<'a> LexemeStress<'a> {
    pub fn new(lx: &'a Lexeme) -> LexemeStress<'a> {
        let stems = lx.stems.iter().flat_map(|s| s.split("//")).map(|v| v.trim_matches('.')).filter(|v| !v.is_empty() && *v != "NOSTEM").filter_map(canonical_letters).collect();
        let lemma = canonical_letters(&lx.lemma);
        let read = |r: Reading| lemma.as_deref().and_then(|l| Shape::of(l, r)).map(|sh| {
            let s = lemma_stress(&sh, lx.pos, Some(lx));
            (sh, s)
        });
        let (join, split) = (read(Reading::Join), read(Reading::Split));
        LexemeStress { lx, lemma, stems, join, split }
    }

    fn lemma_in(&self, reading: Reading) -> Option<&(Shape, Option<Stress>)> {
        match reading {
            Reading::Join => self.join.as_ref(),
            Reading::Split => self.split.as_ref(),
        }
    }

    /// Rrokja e theksuar e lemës ose e një trajte të leksemës.
    pub fn stress(&self, word: &str) -> Option<Stress> {
        let low = canonical_letters(word)?;
        resolve(&low, |sh| self.verdict(sh))
    }

    fn verdict(&self, sh: &Shape) -> Option<Stress> {
        let ctx = Ctx { pos: Some(self.lx.pos), lemma: false, form: true };
        match self.lemma.as_deref() == Some(&sh.low[..]) {
            true => self.lemma_in(sh.reading).and_then(|(_, s)| s.clone()),
            false => general(sh, ctx).or_else(|| self.projected(sh)).or_else(|| mediopassive(sh)).or_else(|| inflected(sh)).flatten(),
        }
    }

    /// Trajta e lakuar merr theksin e lemës kur ndan me lemën ose me një temë zanoren e
    /// theksuar; «ë»-ja e një teme nuk merr theksin e një zanoreje tjetër të lemës (njeri → njerëz).
    fn projected(&self, sh: &Shape) -> Verdict {
        let (lemma, s) = self.lemma_in(sh.reading)?;
        let s = s.as_ref()?;
        let covered = std::iter::once(&lemma.low).chain(self.stems.iter()).any(|c| {
            let common = sh.low.iter().zip(c.iter()).take_while(|(a, b)| a == b).count();
            sh.low.get(..common).map(vowels_in).unwrap_or(0) > s.vowel
        });
        let from_end = covered.then(|| sh.from_ordinal(s.vowel)).flatten()?;
        let theirs = lemma.nucleus_from_end(s.from_end).first().copied().unwrap_or("");
        let ours = sh.nucleus_from_end(from_end).first().copied().unwrap_or("");
        (ours != "ë" || theirs == "ë").then(|| at(sh, from_end, "R-T4: theksi i lemës mbetet në temë"))
    }
}

fn plain(sh: &Shape, hint: Hint) -> Option<Stress> {
    let bare = Ctx { pos: None, lemma: false, form: false };
    let form = Ctx { pos: None, lemma: false, form: true };
    match hint {
        Hint::Unknown => general(sh, bare).flatten(),
        Hint::Inflected => general(sh, form).or_else(|| mediopassive(sh)).or_else(|| inflected(sh)).flatten(),
        Hint::Lemma(pos) => lemma_stress(sh, pos, None),
        Hint::Lexeme(lx) => LexemeStress::new(lx).verdict(sh),
    }
}

/// Rrokja e theksuar dhe rregulli që e dha.
pub fn stress(word: &str, hint: Hint) -> Option<Stress> {
    match hint {
        Hint::Lexeme(lx) => LexemeStress::new(lx).stress(word),
        other => canonical_letters(word).and_then(|low| resolve(&low, |sh| plain(sh, other))),
    }
}

/// Indeksi i rrokjes së theksuar nga fundi (0 = e fundit); None kur nuk dihet me siguri.
pub fn stressed_syllable(word: &str, hint: Hint) -> Option<usize> {
    stress(word, hint).map(|s| s.from_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gramatika::leksiku::parse_line;

    fn lexeme(lemma: &str, upos: &str, par: &str, stems: &str) -> Lexeme {
        let line = format!("x\t{lemma}\t{upos}\t-\tf\t-\t-\t-\t-\t-\tstd\t{par}\t{stems}\t-\t-\t-\t-\t-");
        parse_line(&line).unwrap()
    }

    #[test]
    fn final_schwa_is_never_stressed() {
        assert_eq!(stressed_syllable("botë", Hint::Unknown), Some(1));
        assert_eq!(stressed_syllable("kuçedër", Hint::Unknown), Some(1));
        assert_eq!(stressed_syllable("ndjeshëm", Hint::Unknown), Some(1));
        assert_eq!(stressed_syllable("mal", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("përbëj", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("shqiptar", Hint::Unknown), None);
        assert_eq!(stressed_syllable("vakëf", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("hapësh", Hint::Inflected), Some(1));
    }

    #[test]
    fn function_words_with_stressed_final_schwa() {
        assert_eq!(stressed_syllable("asnjë", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("ndonjë", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("gjithnjë", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("asgjë", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("dikë", Hint::Unknown), Some(0));
        assert_eq!(stressed_syllable("këtë", Hint::Inflected), Some(0));
        assert_eq!(stressed_syllable("atë", Hint::Unknown), None);
        assert_eq!(stressed_syllable("atë", Hint::Lemma(Pos::Noun)), Some(1));
        assert_eq!(stressed_syllable("unë", Hint::Lemma(Pos::Pron)), None);
        assert_eq!(stress("asnjë", Hint::Unknown).map(|s| s.rule), Some("R-T9: fjalë shërbimi me «ë» fundore të theksuar"));
    }

    #[test]
    fn lemma_rules_by_part_of_speech() {
        assert_eq!(stressed_syllable("shqiptar", Hint::Lemma(Pos::Noun)), Some(0));
        assert_eq!(stressed_syllable("ishull", Hint::Lemma(Pos::Noun)), Some(1));
        assert_eq!(stressed_syllable("lule", Hint::Lemma(Pos::Noun)), Some(1));
        assert_eq!(stressed_syllable("liri", Hint::Lemma(Pos::Noun)), Some(0));
        assert_eq!(stressed_syllable("punoj", Hint::Lemma(Pos::Verb)), Some(0));
        assert_eq!(stressed_syllable("lahem", Hint::Lemma(Pos::Verb)), Some(1));
        assert_eq!(stressed_syllable("eci", Hint::Lemma(Pos::Verb)), Some(1));
        assert_eq!(stressed_syllable("hapur", Hint::Lemma(Pos::Adj)), Some(1));
        assert_eq!(stressed_syllable("radio", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("duhet", Hint::Lemma(Pos::Verb)), Some(1));
        assert_eq!(stressed_syllable("punohet", Hint::Lemma(Pos::Verb)), Some(1));
        assert_eq!(stressed_syllable("përzë", Hint::Lemma(Pos::Verb)), Some(0));
        assert_eq!(stressed_syllable("gjendet", Hint::Lemma(Pos::Verb)), None);
        assert_eq!(stressed_syllable("qeshë", Hint::Lemma(Pos::Verb)), Some(1));
    }

    #[test]
    fn unstressed_suffix_classes_are_held() {
        assert_eq!(stressed_syllable("lepur", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("flutur", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("tiranas", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("elbasanas", Hint::Lemma(Pos::Adj)), None);
        assert_eq!(stressed_syllable("bukura", Hint::Inflected), None);
        assert_eq!(stressed_syllable("hapurit", Hint::Inflected), None);
        assert_eq!(stressed_syllable("bukur", Hint::Lemma(Pos::Adj)), Some(1));
    }

    #[test]
    fn vowel_pairs_follow_the_reading() {
        assert_eq!(stress("qenie", Hint::Lemma(Pos::Noun)).map(|s| (s.from_end, s.vowel)), Some((1, 1)));
        assert_eq!(stress("arsye", Hint::Lemma(Pos::Noun)).map(|s| (s.from_end, s.vowel)), Some((1, 1)));
        assert_eq!(stress("arsyeja", Hint::Inflected).map(|s| s.vowel), Some(1));
        assert_eq!(stress("mësues", Hint::Lemma(Pos::Noun)).map(|s| (s.from_end, s.vowel)), Some((0, 1)));
        assert_eq!(stress("kamerier", Hint::Lemma(Pos::Noun)).map(|s| s.vowel), Some(3));
        assert_eq!(stress("situatë", Hint::Unknown).map(|s| s.vowel), Some(2));
        assert_eq!(stress("përrua", Hint::Lemma(Pos::Noun)).map(|s| (s.from_end, s.vowel)), Some((0, 1)));
        assert_eq!(stressed_syllable("mbiemër", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("pyetje", Hint::Lemma(Pos::Noun)), None);
        assert_eq!(stressed_syllable("tryezë", Hint::Unknown), None);
        assert_eq!(stress("kryetar", Hint::Lemma(Pos::Noun)).map(|s| (s.from_end, s.vowel)), Some((0, 2)));
    }

    #[test]
    fn paradigm_class_and_stem_projection() {
        let kafe = lexeme("kafe", "NOUN", "3c", "kafe.|kafe.|kafej.");
        assert_eq!(stressed_syllable("kafe", Hint::Lexeme(&kafe)), Some(0));
        let lule = lexeme("lule", "NOUN", "3b,5a", "lule.|lule.|lulej.|lulj.");
        assert_eq!(stressed_syllable("lule", Hint::Lexeme(&lule)), Some(1));
        assert_eq!(stressed_syllable("lulja", Hint::Lexeme(&lule)), Some(1));
        let gazete = lexeme("gazetë", "NOUN", "3a,5a", "gazeta.|gazetë.|gazet.");
        assert_eq!(stressed_syllable("gazetave", Hint::Lexeme(&gazete)), Some(2));
        let punoj = lexeme("punoj", "V", "lexoj", "puno.|punua.|punu.|pun.");
        let s = stress("punuam", Hint::Lexeme(&punoj)).unwrap();
        assert_eq!(s.from_end, 0);
        assert_eq!(s.rule, "R-T4: theksi i lemës mbetet në temë");
        assert_eq!(stressed_syllable("punonim", Hint::Lexeme(&punoj)), Some(1));
        let njeri = lexeme("njeri", "NOUN", "1a,5b", "njerëz.|njeri.");
        assert_eq!(stressed_syllable("njerëz", Hint::Lexeme(&njeri)), Some(1));
        let bukur = lexeme("bukur", "A", "adj-1-1", "bukur.|bukur.");
        assert_eq!(stressed_syllable("bukura", Hint::Lexeme(&bukur)), Some(2));
        let diell = lexeme("diell", "NOUN", "1a,5a", "diej.|diell.");
        assert_eq!(stress("dielli", Hint::Lexeme(&diell)).map(|s| (s.from_end, s.vowel)), Some((1, 0)));
        let ky = lexeme("ky", "PRO", "dem-pro-sg-pl", "ky.|këtij.|këtë.|kjo.");
        assert_eq!(stressed_syllable("këtë", Hint::Lexeme(&ky)), Some(0));
        let once = LexemeStress::new(&punoj);
        assert_eq!(once.stress("punuam").map(|s| s.from_end), Some(0));
        assert_eq!(once.stress("punoj").map(|s| s.from_end), Some(0));
    }

    #[test]
    fn inflectional_endings_are_unstressed() {
        assert_eq!(stressed_syllable("malin", Hint::Inflected), Some(1));
        assert_eq!(stressed_syllable("punova", Hint::Inflected), Some(1));
        assert_eq!(stressed_syllable("punuam", Hint::Inflected), Some(0));
        assert_eq!(stressed_syllable("kërkon", Hint::Inflected), Some(0));
        assert_eq!(stressed_syllable("gjuhëve", Hint::Inflected), Some(2));
        assert_eq!(stressed_syllable("punohet", Hint::Inflected), Some(1));
        assert_eq!(stressed_syllable("punohemi", Hint::Inflected), Some(2));
        assert_eq!(stressed_syllable("cakulli", Hint::Inflected), Some(2));
        assert_eq!(stressed_syllable("maleve", Hint::Inflected), None);
        assert_eq!(stressed_syllable("malin", Hint::Unknown), None);
    }

    #[test]
    fn adverb_suffixes() {
        assert_eq!(stressed_syllable("zakonisht", Hint::Lemma(Pos::Adv)), Some(0));
        assert_eq!(stressed_syllable("fshehtas", Hint::Lemma(Pos::Adv)), Some(1));
        assert_eq!(stressed_syllable("tërthorazi", Hint::Lemma(Pos::Adv)), Some(2));
        assert_eq!(stressed_syllable("vrapthi", Hint::Lemma(Pos::Adv)), Some(1));
        assert_eq!(stressed_syllable("akull", Hint::Unknown), Some(1));
    }
}
