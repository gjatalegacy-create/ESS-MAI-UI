//! Numërorët e shqipes standarde: themelorë, rreshtorë, thyesorë, dhjetorë dhe vitet.
//!
//! Burimet: uniparser-grammar-albanian 2.3.0, MIT (leksiku i numërorëve dhe i rreshtorëve;
//! gjinia femërore e «mijë» dhe mashkullore e «milion», «miliard»; paradigma «adj-2-1» për trajtat
//! e shquara femërore të thyesave); Drejtshkrimi i gjuhës shqipe (1973).
//!
//! Rendi i dëshmisë, i fiksuar:
//! 1. leksiku i gramatikës (uniparser), kur jepet, dëshmon fjalën rreshtore (`ordinal_in`);
//! 2. tabelat e deklaruara: `IRREGULAR` (20, 30, 40), `FIRST` («i parë»), `BARE` (qind, mijë),
//!    `BARE_ORDINAL` (qindtë, mijtë, miliontë, miliardtë);
//! 3. zbërthimi pozicional n = Σ dᵢ·10ⁱ dhe rregullat R-N, R-O, R-T, R-D, R-VT;
//! 4. çdo lexim pranon vlerën vetëm kur rigjenerimi, nga rregulli ose nga tabela, jep të njëjtën trajtë.
//!
//! Kur asnjë nuk vlen me siguri, kthehet None: numri nuk shkruhet e nuk lexohet me hamendje.
//! Lakimi i numërorëve (treve, pesave), rreshtorët e shquar (i pesti, e dyta), shumësi i
//! përafërt (qindra, mijëra) dhe rreshtorët me më shumë se dy përbërës (i njëqind e dhjetë…)
//! nuk trajtohen: kthejnë None.
//!
//! Numërimi me bazë 20: «njëzet» = një·20 dhe «dyzet» = dy·20 janë mbetje të sistemit
//! vigesimal. Trajtat dialektore me bazë 20 (u·20, u·20 + 10) nuk dëshmohen në leksikun e
//! gramatikës dhe nuk lexohen: kthejnë None.

use crate::gramatika::leksiku::Lexicon;
use crate::gramatika::tipet::{Form, Gender, Pos};

/// Kufiri i sipërm i numërorëve: 999 999 999 999.
pub const MAX: u64 = 999_999_999_999;

/// Gjatësia më e madhe e hyrjes që lexohet (në bajtë).
const MAX_INPUT: usize = 512;

/// Gjatësia më e madhe e një fjale të shkruar bashkë që ndahet (në bajtë);
/// më e gjata e rregullt, «gjashtëqindegjashtëdhjetegjashtë», ka 35.
const MAX_JOINED: usize = 48;

/// Pjesa dhjetore më e gjatë që lexohet si numër (në shifra).
const MAX_FRACTION_DIGITS: usize = 3;

pub const R_ZERO: &str = "R-N0: zero";
pub const R_SIMPLE: &str = "R-N1: numërorët e thjeshtë një–dhjetë; «tri» me emrat femërorë";
pub const R_TEEN: &str = "R-N2: njësia + «mbë» + «dhjetë» (11–19)";
pub const R_TENS: &str = "R-N3: njësia + «dhjetë»; 20 = një·20, 40 = dy·20 (vigesimale); 30 me temën «tri»";
pub const R_HUNDREDS: &str = "R-N4: njësia + «qind», shkruhen bashkë";
pub const R_LARGE: &str =
    "R-N5: «mijë», «milion», «miliard» shkruhen ndaras; shumësi «milionë», «miliardë»; «tri mijë» se «mijë» është femërore (pranohet edhe «tre mijë»)";
pub const R_COMPOUND: &str = "R-N6: n = Σ dᵢ·10ⁱ; përbërësit lidhen me «e»";
pub const R_ORD_FIRST: &str = "R-O1: «i parë», trajtë supletive";
pub const R_ORD_TE: &str = "R-O2: prapashtesa «-të»; «-ë» e patheksuar bie (pesë → pestë), «t + të» → «të» (gjashtë, njëzetë)";
pub const R_ORD_T: &str = "R-O3: pas «-r» prapashtesa «-t» (katër → katërt)";
pub const R_ORD_JOINED: &str = "R-O4: rreshtori i përbërë shkruhet bashkë me «-e-» (njëzetenjëtë, njëqindedytë)";
pub const R_ORD_LARGE: &str = "R-O5: rreshtorët e mijëshes, milionit dhe miliardit shkruhen bashkë (dymijtë, njëmiliontë)";
pub const R_ORD_BARE: &str = "R-O6: rreshtor pa «një» i tabelës `BARE_ORDINAL` (i qindtë, i mijtë)";
pub const R_ORD_WITNESS: &str = "L-O: rreshtor i dëshmuar në leksikun e gramatikës (uniparser)";
pub const R_FRACTION: &str =
    "R-T1: numëruesi themelor në gjininë femërore + emëruesi rreshtor femëror i shquar shumës (tri të katërtat; pranohet edhe «tre»)";
pub const R_UNIT_FRACTION: &str = "R-T2: «një e» + rreshtori femëror i shquar njëjës (një e treta); 1/2 edhe «gjysmë»";
pub const R_FRACTION_BARE: &str = "R-T3: emëruesi pa «një» i tabelës `BARE_ORDINAL` (një e qindta, dy të mijtat)";
pub const R_DECIMAL: &str =
    "R-D1: pjesa e plotë + «presje» + pjesa dhjetore si numër, deri në tri shifra; zerot e para një nga një; më shumë shifra: None";
pub const R_YEAR: &str = "R-VT1: viti lexohet si numëror themelor";
pub const R_READ: &str = "R-L1: lexim i verifikuar me rigjenerim";
pub const R_READ_JOINED: &str = "R-L2: shkrimi i bashkuar i vjetër («njëzetenjë» = «njëzet e një»)";
pub const R_READ_BARE: &str = "R-L4: trajtë pa «një» e tabelës `BARE` (qind, mijë)";
pub const R_ORD_READ: &str = "R-OL1: lexim rreshtori i verifikuar me rigjenerim";
pub const R_ORD_READ_SPLIT: &str = "R-OL2: rreshtori i shkallës i shkruar ndaras, vetëm për lexim («i një mijtë» = «i njëmijtë»)";

const UNITS: [&str; 10] = ["zero", "një", "dy", "tre", "katër", "pesë", "gjashtë", "shtatë", "tetë", "nëntë"];

/// Të parregulltat e dhjetësheve: (vlera, trajta, arsyeja).
pub const IRREGULAR: &[(u64, &str, &str)] = &[
    (20, "njëzet", "një·20, numërim vigesimal"),
    (30, "tridhjetë", "tema femërore «tri», jo «tre»"),
    (40, "dyzet", "dy·20, numërim vigesimal"),
];

/// Rreshtori supletiv i njëshit.
pub const FIRST: &str = "parë";

/// Numërorët themelorë pa «një»: (vlera, trajta, arsyeja).
pub const BARE: &[(u64, &str, &str)] = &[(100, "qind", "«njëqind» pa «një»"), (1_000, "mijë", "«një mijë» pa «një»")];

/// Rreshtorët e shkallëve pa «një»: (vlera, fjala, arsyeja).
pub const BARE_ORDINAL: &[(u64, &str, &str)] = &[
    (100, "qindtë", "«i njëqindtë» pa «një»"),
    (1_000, "mijtë", "«i njëmijtë» pa «një»"),
    (1_000_000, "miliontë", "«i njëmiliontë» pa «një»"),
    (1_000_000_000, "miliardtë", "«i njëmiliardtë» pa «një»"),
];

/// Shkallët e grupeve treshifrore: (njëjës, shumës, trajta e treshit) për 1000ᵏ.
/// Treshi përshtatet me gjininë e shkallës: «mijë» femërore (tri mijë), «milion», «miliard» mashkullore.
const SCALES: [(&str, &str, &str); 4] =
    [("", "", ""), ("mijë", "mijë", "tri"), ("milion", "milionë", "tre"), ("miliard", "miliardë", "tre")];

/// Shkallët me vlerën e tyre, për leximin e rreshtorëve të bashkuar (dymijtë, njëmiliontë).
const SCALE_WORDS: [(&str, u64); 3] = [("mijë", 1_000), ("milion", 1_000_000), ("miliard", 1_000_000_000)];

const VOWELS: &str = "aeëiouy";

/// Leximi i një numërori me rregullin që e dha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub value: u64,
    pub rule: &'static str,
}

/// Zbërthimi pozicional n = Σ dᵢ·10ⁱ: shifrat dᵢ, nga njësitë lart.
pub fn digits(n: u64) -> Vec<u8> {
    std::iter::successors(Some(n), |m| (*m >= 10).then_some(m / 10)).map(|m| (m % 10) as u8).collect()
}

/// Rindërtimi n = Σ dᵢ·10ⁱ me skemën e Hornerit, me kontroll tejkalimi.
pub fn compose(d: &[u8]) -> Option<u64> {
    d.iter().rev().try_fold(0u64, |acc, &x| acc.checked_mul(10)?.checked_add(u64::from(x)))
}

/// Grupet treshifrore n = Σ gₖ·1000ᵏ, ku gₖ = Σᵢ d₃ₖ₊ᵢ·10ⁱ; k = 0 njësitë, k = 3 miliardët.
fn groups(n: u64) -> [u64; 4] {
    let d = digits(n);
    let at = |i: usize| u64::from(d.get(i).copied().unwrap_or(0));
    [0usize, 1, 2, 3].map(|k| at(3 * k) + 10 * at(3 * k + 1) + 100 * at(3 * k + 2))
}

fn unit(u: u64) -> &'static str {
    usize::try_from(u).ok().and_then(|i| UNITS.get(i)).copied().unwrap_or("")
}

fn irregular(v: u64) -> Option<&'static str> {
    IRREGULAR.iter().find(|(n, _, _)| *n == v).map(|(_, w, _)| *w)
}

/// Dhjetëshja t·10, për t = 1..9.
fn tens(t: u64) -> String {
    match t {
        1 => "dhjetë".to_string(),
        _ => irregular(t * 10).map(str::to_string).unwrap_or_else(|| format!("{}dhjetë", unit(t))),
    }
}

/// Numri r < 100; `three` është «tre» ose «tri» kur njësia 3 qëndron më vete.
fn below_hundred(r: u64, three: &'static str) -> Vec<String> {
    let u = r % 10;
    let last = match u {
        3 => three,
        _ => unit(u),
    };
    match (r / 10, u) {
        (0, 0) => Vec::new(),
        (0, _) => vec![last.to_string()],
        (1, 0) => vec![tens(1)],
        (1, _) => vec![format!("{}mbëdhjetë", unit(u))],
        (t, 0) => vec![tens(t)],
        (t, _) => vec![tens(t), "e".to_string(), last.to_string()],
    }
}

/// Grupi g < 1000: qindëshja shkruhet bashkë, pjesa tjetër lidhet me «e».
fn group_words(g: u64, three: &'static str) -> Vec<String> {
    let hundreds: Vec<String> = (g >= 100).then(|| format!("{}qind", unit(g / 100))).into_iter().collect();
    let rest = below_hundred(g % 100, three);
    let link: Vec<String> = (!hundreds.is_empty() && !rest.is_empty()).then(|| "e".to_string()).into_iter().collect();
    [hundreds, link, rest].concat()
}

/// Fjalët e numrit n ≤ MAX; `three` vlen për grupin e njësive (gjinia e emrit),
/// grupet e tjera marrin treshin e shkallës së tyre.
fn words(n: u64, three: &'static str) -> Vec<String> {
    let g = groups(n);
    let parts: Vec<Vec<String>> = [3usize, 2, 1, 0]
        .iter()
        .filter_map(|&k| {
            let v = g.get(k).copied().unwrap_or(0);
            let (one, many, scale_three) = SCALES.get(k).copied().unwrap_or(("", "", "tre"));
            let t = match k {
                0 => three,
                _ => scale_three,
            };
            let scale: Vec<String> = match (k, v) {
                (0, _) => Vec::new(),
                (_, 1) => vec![one.to_string()],
                _ => vec![many.to_string()],
            };
            (v > 0).then(|| [group_words(v, t), scale].concat())
        })
        .collect();
    match parts.is_empty() {
        true => vec!["zero".to_string()],
        false => parts.join(&"e".to_string()),
    }
}

fn cardinal_rule(n: u64, w: &[String]) -> &'static str {
    match (n, w.iter().any(|t| t == "e")) {
        (0, _) => R_ZERO,
        (_, true) => R_COMPOUND,
        (1..=10, false) => R_SIMPLE,
        (11..=19, false) => R_TEEN,
        (20..=99, false) => R_TENS,
        (100..=999, false) => R_HUNDREDS,
        _ => R_LARGE,
    }
}

/// Variantet drejtshkrimore pa tabelë: «tre mijë» i përdorimit, shumësi «miliona»/«miliarda».
fn spelling_variants(text: &str) -> Vec<String> {
    let mut out: Vec<String> = [text.replace("tri mijë", "tre mijë"), text.replace("milionë", "miliona").replace("miliardë", "miliarda")]
        .into_iter()
        .filter(|v| v != text)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Variantet e pranuara: ato drejtshkrimore dhe trajtat pa «një» të tabelës `BARE`.
fn cardinal_variants(n: u64, text: &str) -> Vec<String> {
    let bare = BARE.iter().filter(|(v, _, _)| *v == n).map(|(_, w, _)| w.to_string());
    let mut out: Vec<String> = spelling_variants(text).into_iter().chain(bare).filter(|v| v != text).collect();
    out.sort();
    out.dedup();
    out
}

/// Numërori themelor për n ≤ MAX, në përputhje me gjininë e emrit (tre/tri);
/// para «mijë» treshi është «tri» në çdo gjini. Për asnjanësen kthehet None vetëm kur trajta varet nga gjinia.
pub fn cardinal_form(n: u64, gender: Gender) -> Option<Form> {
    (n <= MAX).then_some(())?;
    let masc = words(n, "tre");
    let fem = words(n, "tri");
    let chosen = match gender {
        Gender::Masc => Some(masc),
        Gender::Fem => Some(fem),
        Gender::Neut => (masc == fem).then_some(masc),
    }?;
    let text = chosen.join(" ");
    Some(Form { variants: cardinal_variants(n, &text), rule: cardinal_rule(n, &chosen), text })
}

/// Numërori themelor si tekst.
pub fn cardinal(n: u64, gender: Gender) -> Option<String> {
    cardinal_form(n, gender).map(|f| f.text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Atom {
    Link,
    Small(u64),
    Scale(u64),
}

fn unit_value(t: &str) -> Option<u64> {
    match t {
        "tri" => Some(3),
        _ => UNITS.iter().position(|u| *u == t).map(|i| i as u64),
    }
}

/// Vlera e një fjale nën 1000: njësitë, 11–19, dhjetëshet, qindëshet.
fn small_value(t: &str) -> Option<u64> {
    let teen = || t.strip_suffix("mbëdhjetë").and_then(unit_value).filter(|u| (1..=9).contains(u)).map(|u| 10 + u);
    let ten = || {
        IRREGULAR
            .iter()
            .find(|(_, w, _)| *w == t)
            .map(|(v, _, _)| *v)
            .or_else(|| t.strip_suffix("dhjetë").and_then(unit_value).filter(|u| (5..=9).contains(u)).map(|u| 10 * u))
    };
    let hundred = || t.strip_suffix("qind").and_then(unit_value).filter(|u| (1..=9).contains(u)).map(|u| 100 * u);
    unit_value(t).or_else(teen).or_else(ten).or_else(hundred)
}

fn atom(t: &str) -> Option<Atom> {
    match t {
        "e" => Some(Atom::Link),
        "dhjetë" => Some(Atom::Small(10)),
        "qind" => Some(Atom::Small(100)),
        "mijë" => Some(Atom::Scale(1_000)),
        "milion" | "milionë" | "miliona" => Some(Atom::Scale(1_000_000)),
        "miliard" | "miliardë" | "miliarda" => Some(Atom::Scale(1_000_000_000)),
        _ => small_value(t).map(Atom::Small),
    }
}

/// Vlera e vargut: n = Σ gₖ·sₖ, ku gₖ është shuma e fjalëve para shkallës sₖ.
fn value_of(tokens: &[String]) -> Option<u64> {
    let mut total: u64 = 0;
    let mut group: u64 = 0;
    for t in tokens {
        match atom(t)? {
            Atom::Link => {}
            Atom::Small(v) => group = group.checked_add(v)?,
            Atom::Scale(m) => {
                total = total.checked_add(group.max(1).checked_mul(m)?)?;
                group = 0;
            }
        }
    }
    total.checked_add(group)
}

/// Vlera pranohet vetëm kur numërori i rigjeneruar (në njërën gjini) jep të njëjtat fjalë.
fn verified(value: u64, tokens: &[String]) -> bool {
    let text = tokens.join(" ");
    let same = |w: Vec<String>| {
        let w = w.join(" ");
        w == text || spelling_variants(&w).contains(&text)
    };
    value <= MAX && (same(words(value, "tre")) || same(words(value, "tri")))
}

/// Fjalët e hyrjes: shkronja të vogla, pa hapësira të tepërta, viza si ndarëse.
fn normalize(words: &str) -> Option<Vec<String>> {
    (words.len() <= MAX_INPUT).then_some(())?;
    let out: Vec<String> =
        words.to_lowercase().split(|c: char| c.is_whitespace() || c == '-').filter(|w| !w.is_empty()).map(str::to_string).collect();
    (!out.is_empty()).then_some(out)
}

/// Ndarja e shkrimit të bashkuar: «tridhjetenjë» → «tridhjetë», «e», «një»; «ë»-ja para «e»-së rikthehet.
/// Fjala më e gjatë se `MAX_JOINED` nuk ndahet.
fn segment(w: &str) -> Option<Vec<String>> {
    (w.len() <= MAX_JOINED).then_some(())?;
    match atom(w) {
        Some(_) => Some(vec![w.to_string()]),
        None => w.match_indices('e').find_map(|(i, _)| {
            let head = w.get(..i)?;
            let tail = w.get(i + 1..)?;
            let word =
                [head.to_string(), format!("{head}ë")].into_iter().find(|h| matches!(atom(h), Some(Atom::Small(_) | Atom::Scale(_))))?;
            segment(tail).map(|rest| [vec![word, "e".to_string()], rest].concat())
        }),
    }
}

fn read_standard(tokens: &[String]) -> Option<Reading> {
    let v = value_of(tokens)?;
    verified(v, tokens).then_some(Reading { value: v, rule: R_READ })
}

fn read_bare(tokens: &[String]) -> Option<Reading> {
    match tokens {
        [w] => BARE.iter().find(|(_, b, _)| *b == w.as_str()).map(|(v, _, _)| Reading { value: *v, rule: R_READ_BARE }),
        _ => None,
    }
}

fn read_joined(tokens: &[String]) -> Option<Reading> {
    let split: Vec<String> = tokens.iter().map(|t| segment(t)).collect::<Option<Vec<Vec<String>>>>()?.concat();
    (split.len() > tokens.len()).then_some(())?;
    let v = value_of(&split)?;
    verified(v, &split).then_some(Reading { value: v, rule: R_READ_JOINED })
}

/// Leximi i numërorit të shkruar me fjalë, me rregullin që e dha.
/// Rendi: shkrimi standard, trajtat pa «një», shkrimi i bashkuar i vjetër.
pub fn reading(words: &str) -> Option<Reading> {
    let tokens = normalize(words)?;
    read_standard(&tokens).or_else(|| read_bare(&tokens)).or_else(|| read_joined(&tokens))
}

/// Vlera e numërorit të shkruar me fjalë; e kundërta e `cardinal`.
pub fn parse(words: &str) -> Option<u64> {
    reading(words).map(|r| r.value)
}

/// «ë»-ja fundore e patheksuar bie; mbetet kur është bërthama e vetme e rrokjes (një).
fn drop_schwa(w: &str) -> &str {
    w.strip_suffix('ë').filter(|b| b.chars().any(|c| VOWELS.contains(c))).unwrap_or(w)
}

/// Prapashtesa rreshtore: «-t» pas «-r»; përndryshe «-të», ku «t + të» → «të».
/// Trajta me «-ët» (pesët) është jostandarde sipas leksikut të gramatikës dhe nuk prodhohet.
fn ord_suffix(w: &str) -> (String, &'static str) {
    let base = drop_schwa(w);
    match (w.ends_with('r'), base.ends_with('t')) {
        (true, _) => (format!("{w}t"), R_ORD_T),
        (false, true) => (format!("{base}ë"), R_ORD_TE),
        (false, false) => (format!("{base}të"), R_ORD_TE),
    }
}

/// «X e Y» → «X-e-Ytë», kur X është dhjetëshe 20–90 ose «njëqind» dhe Y njësi 1..9.
/// Leksiku dëshmon vetëm 21–99 dhe 101–109; qindëshet e tjera me njësi kthejnë None.
fn joined_ordinal(x: &str, link: &str, y: &str) -> Option<(String, &'static str)> {
    let head_ok = small_value(x).is_some_and(|v| (20..=100).contains(&v));
    let tail_ok = unit_value(y).is_some_and(|u| (1..=9).contains(&u));
    (head_ok && tail_ok && link == "e").then(|| (format!("{}e{}", drop_schwa(x), ord_suffix(y).0), R_ORD_JOINED))
}

/// «k mijë/milion/miliard» → «k + mijtë/miliontë/miliardtë», për k = 1..9.
fn large_ordinal(k: &str, scale: &str) -> Option<(String, &'static str)> {
    let one = match scale {
        "mijë" => Some("mijë"),
        "milion" | "milionë" => Some("milion"),
        "miliard" | "miliardë" => Some("miliard"),
        _ => None,
    }?;
    unit_value(k).filter(|u| (1..=9).contains(u))?;
    Some((format!("{k}{}", ord_suffix(one).0), R_ORD_LARGE))
}

/// Fjala rreshtore e fjalëve të një numërori themelor: një fjalë, njësi + shkallë, ose «X e Y».
fn ordinal_of(tokens: &[String]) -> Option<(String, &'static str)> {
    match tokens {
        [w] => Some(ord_suffix(w)),
        [k, scale] => large_ordinal(k, scale),
        [x, link, y] => joined_ordinal(x, link, y),
        _ => None,
    }
}

/// Fjala rreshtore pa nyjë, me rregullin e saj.
fn ordinal_word(n: u64) -> Option<(String, &'static str)> {
    match n {
        0 => None,
        1 => Some((FIRST.to_string(), R_ORD_FIRST)),
        _ => ordinal_of(&words(n, "tre")),
    }
}

/// Fjalët rreshtore të varianteve drejtshkrimore të themelorit: «tremijtë» krahas «trimijtë».
fn ordinal_variants(n: u64) -> Vec<String> {
    let text = (n >= 2).then(|| words(n, "tre").join(" ")).unwrap_or_default();
    spelling_variants(&text)
        .iter()
        .filter_map(|v| ordinal_of(&v.split(' ').map(str::to_string).collect::<Vec<String>>()).map(|(w, _)| w))
        .collect()
}

/// Numërori rreshtor me nyjën e përparme: «i» për mashkulloren, «e» për femëroren.
/// Rreshtorët me më shumë se dy përbërës dhe asnjanësja kthejnë None.
pub fn ordinal_form(n: u64, gender: Gender) -> Option<Form> {
    (n <= MAX).then_some(())?;
    let article = match gender {
        Gender::Masc => Some("i"),
        Gender::Fem => Some("e"),
        Gender::Neut => None,
    }?;
    let (word, rule) = ordinal_word(n)?;
    let bare = BARE_ORDINAL.iter().filter(|(v, _, _)| *v == n).map(|(_, w, _)| w.to_string());
    let variants = ordinal_variants(n).into_iter().chain(bare).map(|w| format!("{article} {w}")).collect();
    Some(Form { text: format!("{article} {word}"), variants, rule })
}

/// Numërori rreshtor si tekst.
pub fn ordinal(n: u64, gender: Gender) -> Option<String> {
    ordinal_form(n, gender).map(|f| f.text)
}

/// Temat themelore të mundshme të fjalës rreshtore: hiqet «-të», «-ë» ose «-t» pas «-r»
/// dhe «ë»-ja e rënë rikthehet.
fn ordinal_stems(w: &str) -> Vec<String> {
    let with_schwa = |b: &str| [b.to_string(), format!("{b}ë")];
    w.strip_suffix("të")
        .map(with_schwa)
        .into_iter()
        .flatten()
        .chain(w.strip_suffix('ë').map(with_schwa).into_iter().flatten())
        .chain(w.strip_suffix('t').filter(|b| b.ends_with('r')).map(str::to_string))
        .collect()
}

/// Vlera e temës: fjalë e vetme, shkrim i bashkuar me «-e-», ose njësi + shkallë (dymijë, njëmilion).
fn stem_value(s: &str) -> Option<u64> {
    let scaled = || {
        SCALE_WORDS
            .iter()
            .find_map(|&(w, m)| s.strip_suffix(w).and_then(unit_value).filter(|u| (1..=9).contains(u)).map(|u| u * m))
    };
    segment(s).and_then(|t| value_of(&t)).or_else(scaled)
}

/// Leximi i numërorit rreshtor («i njëzetenjëtë» → 21), me nyjë ose pa nyjë, me rregullin që e dha.
/// Prapashtesa hiqet në mënyrë analitike; vlera pranohet vetëm kur `ordinal_form` rigjeneron të njëjtën fjalë
/// me të njëjtën nyjë. Fjala që është edhe numëror themelor (gjashtë, dhjetë, tridhjetë) pa nyjë ose pas «e»
/// është e dykuptimtë, sepse «e» është edhe lidhëza e numërorit (njëzet e gjashtë): kthen None.
pub fn ordinal_reading(words: &str) -> Option<Reading> {
    let tokens = normalize(words)?;
    let (article, body): (Option<&str>, &[String]) = match tokens.split_first() {
        Some((a, rest)) => match (a.as_str(), rest.is_empty()) {
            ("i" | "e", false) => (Some(a.as_str()), rest),
            _ => (None, tokens.as_slice()),
        },
        None => (None, tokens.as_slice()),
    };
    let ambiguous = match article {
        Some("i") => false,
        _ => reading(&body.join(" ")).is_some(),
    };
    (!ambiguous).then_some(())?;
    let (word, split) = match body {
        [w] => Some((w.clone(), false)),
        [k, s] => (unit_value(k).is_some() && BARE_ORDINAL.iter().any(|(v, b, _)| *v >= 1_000 && *b == s.as_str()))
            .then(|| (format!("{k}{s}"), true)),
        _ => None,
    }?;
    let stems = ordinal_stems(&word);
    let first = (word == FIRST).then_some((1u64, R_ORD_FIRST));
    let bare = BARE_ORDINAL.iter().filter(|(_, b, _)| *b == word.as_str()).map(|(v, _, _)| (*v, R_ORD_BARE));
    let regular = stems.iter().filter_map(|s| stem_value(s)).map(|v| (v, R_ORD_READ));
    let regenerates = |v: u64| {
        [Gender::Masc, Gender::Fem].iter().filter_map(|g| ordinal_form(v, *g)).any(|f| {
            std::iter::once(&f.text)
                .chain(f.variants.iter())
                .filter_map(|t| t.split_once(' '))
                .any(|(a, w)| w == word && article.map_or(true, |x| x == a))
        })
    };
    first.into_iter().chain(bare).chain(regular).find(|(v, _)| regenerates(*v)).map(|(value, rule)| Reading {
        value,
        rule: match split {
            true => R_ORD_READ_SPLIT,
            false => rule,
        },
    })
}

/// Vlera e numërorit rreshtor, me nyjë ose pa nyjë; e kundërta e `ordinal`.
pub fn parse_ordinal(words: &str) -> Option<u64> {
    ordinal_reading(words).map(|r| r.value)
}

/// Rreshtori me dëshminë e leksikut: kur fjala gjendet aty si mbiemër, dëshmia i paraprin rregullit.
pub fn ordinal_in(lx: &Lexicon, n: u64, gender: Gender) -> Option<Form> {
    let form = ordinal_form(n, gender)?;
    let word = form.text.split_once(' ').map(|(_, w)| w.to_string()).unwrap_or_default();
    let attested = lx.entries(&word).iter().any(|l| l.pos == Pos::Adj);
    Some(match attested {
        true => Form { rule: R_ORD_WITNESS, ..form },
        false => form,
    })
}

/// Emëruesi rreshtor dhe nëse vjen nga tabela `BARE_ORDINAL` (e qindta, e mijta).
fn denominator(q: u64) -> Option<(String, bool)> {
    BARE_ORDINAL
        .iter()
        .find(|(v, _, _)| *v == q)
        .map(|(_, w, _)| (w.to_string(), true))
        .or_else(|| (q >= 2).then(|| ordinal_word(q)).flatten().map(|(w, _)| (w, false)))
}

/// Thyesa p/q: emëruesi merr trajtën femërore të shquar të rreshtorit (paradigma «adj-2-1»:
/// njëjës «<1>.a», shumës «<1>.at»): një e treta, dy të tretat. Emëruesi është femëror,
/// ndaj numëruesi merr «tri» (tri të katërtat); «tre të katërtat» pranohet si variant.
pub fn fraction_form(p: u64, q: u64) -> Option<Form> {
    ((1..=MAX).contains(&p) && (2..=MAX).contains(&q)).then_some(())?;
    let (word, bare) = denominator(q)?;
    let stem = drop_schwa(&word);
    let (text, variants, rule) = match p {
        1 => (format!("një e {stem}a"), (q == 2).then(|| "gjysmë".to_string()).into_iter().collect(), R_UNIT_FRACTION),
        _ => {
            let f = cardinal(p, Gender::Fem)?;
            let m = cardinal(p, Gender::Masc)?;
            (format!("{f} të {stem}at"), (f != m).then(|| format!("{m} të {stem}at")).into_iter().collect(), R_FRACTION)
        }
    };
    let rule = match bare {
        true => R_FRACTION_BARE,
        false => rule,
    };
    Some(Form { text, variants, rule })
}

/// Thyesa si tekst.
pub fn fraction(p: u64, q: u64) -> Option<String> {
    fraction_form(p, q).map(|f| f.text)
}

/// Numri dhjetor me presje: «3,14» → «tre presje katërmbëdhjetë». Pika nuk pranohet si ndarëse;
/// pjesa dhjetore me më shumë se tri shifra lexohet ndryshe në gjuhën e folur dhe kthen None.
pub fn decimal_form(text: &str) -> Option<Form> {
    let t = text.trim();
    let (sign, body) = t.strip_prefix('-').map_or((None, t), |b| (Some("minus".to_string()), b));
    let (int, frac) = body.split_once(',')?;
    let digits_ok = |s: &str, most: usize| (1..=most).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    (digits_ok(int, 12) && digits_ok(frac, MAX_FRACTION_DIGITS) && (int.len() == 1 || !int.starts_with('0'))).then_some(())?;
    let whole = cardinal(int.parse().ok()?, Gender::Masc)?;
    let zeros = frac.bytes().take_while(|b| *b == b'0').count();
    let rest = frac.get(zeros..).unwrap_or("");
    let rest_words = match rest.is_empty() {
        true => None,
        false => Some(cardinal(rest.parse().ok()?, Gender::Masc)?),
    };
    let all: Vec<String> = sign
        .into_iter()
        .chain([whole, "presje".to_string()])
        .chain(std::iter::repeat_n("zero".to_string(), zeros))
        .chain(rest_words)
        .collect();
    Some(Form::one(all.join(" "), R_DECIMAL))
}

/// Numri dhjetor si tekst.
pub fn decimal(text: &str) -> Option<String> {
    decimal_form(text).map(|f| f.text)
}

/// Viti si numëror themelor: 1941 → «një mijë e nëntëqind e dyzet e një».
pub fn year_form(y: u64) -> Option<Form> {
    (y >= 1).then_some(())?;
    cardinal_form(y, Gender::Masc).map(|f| Form { rule: R_YEAR, ..f })
}

/// Viti si tekst.
pub fn year(y: u64) -> Option<String> {
    year_form(y).map(|f| f.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(n: u64) -> String {
        cardinal(n, Gender::Masc).unwrap()
    }

    /// Numrat që kanë rreshtor të shkruar nga moduli: 1–109, qindëshet dhe k·10³, k·10⁶, k·10⁹ (k = 1..9).
    fn ordinal_domain() -> impl Iterator<Item = u64> {
        (1..=109u64).chain([100u64, 1_000, 1_000_000, 1_000_000_000].into_iter().flat_map(|s| (1..=9u64).map(move |k| k * s)))
    }

    #[test]
    fn decomposition_is_positional() {
        assert_eq!(digits(1941), vec![1, 4, 9, 1]);
        assert_eq!(digits(0), vec![0]);
        assert_eq!(compose(&digits(MAX)), Some(MAX));
        assert_eq!(groups(1_234_567_890), [890, 567, 234, 1]);
        assert_eq!(compose(&[9; 25]), None);
    }

    #[test]
    fn simple_teens_and_tens() {
        assert_eq!(c(0), "zero");
        assert_eq!(c(1), "një");
        assert_eq!(c(4), "katër");
        assert_eq!(c(10), "dhjetë");
        assert_eq!(c(11), "njëmbëdhjetë");
        assert_eq!(c(12), "dymbëdhjetë");
        assert_eq!(c(13), "trembëdhjetë");
        assert_eq!(c(19), "nëntëmbëdhjetë");
        assert_eq!(c(20), "njëzet");
        assert_eq!(c(30), "tridhjetë");
        assert_eq!(c(40), "dyzet");
        assert_eq!(c(50), "pesëdhjetë");
        assert_eq!(c(90), "nëntëdhjetë");
        assert_eq!(cardinal_form(40, Gender::Masc).unwrap().rule, R_TENS);
    }

    #[test]
    fn gender_of_three() {
        assert_eq!(cardinal(3, Gender::Masc).unwrap(), "tre");
        assert_eq!(cardinal(3, Gender::Fem).unwrap(), "tri");
        assert_eq!(cardinal(23, Gender::Fem).unwrap(), "njëzet e tri");
        assert_eq!(cardinal(13, Gender::Fem).unwrap(), "trembëdhjetë");
        assert_eq!(cardinal(300, Gender::Fem).unwrap(), "treqind");
        assert!(cardinal(3, Gender::Neut).is_none(), "tre/tri për asnjanësen: asnjë hamendje");
        assert_eq!(cardinal(5, Gender::Neut).unwrap(), "pesë");
        for g in [Gender::Masc, Gender::Fem, Gender::Neut] {
            let k = cardinal_form(3_000, g).unwrap();
            assert_eq!(k.text, "tri mijë", "«mijë» është femërore");
            assert!(k.accepts("tre mijë"));
        }
        assert_eq!(cardinal(23_003, Gender::Masc).unwrap(), "njëzet e tri mijë e tre");
        assert_eq!(cardinal(3_000_003, Gender::Fem).unwrap(), "tre milionë e tri");
        assert_eq!(cardinal(13_000, Gender::Fem).unwrap(), "trembëdhjetë mijë");
    }

    #[test]
    fn compounds_with_the_connector() {
        assert_eq!(c(21), "njëzet e një");
        assert_eq!(c(100), "njëqind");
        assert_eq!(c(105), "njëqind e pesë");
        assert_eq!(c(200), "dyqind");
        assert_eq!(c(999), "nëntëqind e nëntëdhjetë e nëntë");
        assert_eq!(c(1_000), "një mijë");
        assert_eq!(c(2_024), "dy mijë e njëzet e katër");
        assert_eq!(c(101_000), "njëqind e një mijë");
        assert_eq!(c(1_000_000), "një milion");
        assert_eq!(c(2_500_000), "dy milionë e pesëqind mijë");
        assert_eq!(c(1_000_000_000), "një miliard");
        assert_eq!(c(MAX), "nëntëqind e nëntëdhjetë e nëntë miliardë e nëntëqind e nëntëdhjetë e nëntë milionë e nëntëqind e nëntëdhjetë e nëntë mijë e nëntëqind e nëntëdhjetë e nëntë");
        assert!(cardinal(MAX + 1, Gender::Masc).is_none());
        assert!(cardinal_form(100, Gender::Masc).unwrap().accepts("qind"));
        assert_eq!(cardinal_form(2_024, Gender::Masc).unwrap().rule, R_COMPOUND);
    }

    #[test]
    fn parse_is_the_inverse() {
        for n in [0, 1, 3, 13, 21, 40, 99, 100, 105, 999, 1_000, 1_941, 2_024, 30_003, 1_000_001, 7_654_321, MAX] {
            assert_eq!(parse(&c(n)), Some(n), "{n}");
            assert_eq!(parse(&cardinal(n, Gender::Fem).unwrap()), Some(n), "{n}");
        }
        assert_eq!(parse("  Njëzet   E NJË "), Some(21));
        assert_eq!(parse("tri mijë"), Some(3_000));
        assert_eq!(parse("tre mijë"), Some(3_000));
        assert_eq!(parse("dy miliona"), Some(2_000_000));
        assert_eq!(reading("qind"), Some(Reading { value: 100, rule: R_READ_BARE }));
        assert_eq!(reading("mijë"), Some(Reading { value: 1_000, rule: R_READ_BARE }));
        assert_eq!(reading("njëqind").map(|r| r.rule), Some(R_READ));
    }

    #[test]
    fn parse_holds_on_malformed_input() {
        for w in ["", "  ", "e", "dy dy", "njëzet e njëmbëdhjetë", "tredhjetë", "zero e një", "një qind", "mijë mijë", "disa", "treve", "dhjetëra", "njëzet e", "mijë e pesë"] {
            assert_eq!(parse(w), None, "{w}");
        }
        assert_eq!(parse(&"një ".repeat(400)), None);
    }

    #[test]
    fn joined_readings() {
        assert_eq!(reading("njëzetenjë"), Some(Reading { value: 21, rule: R_READ_JOINED }));
        assert_eq!(parse("tridhjetedy"), Some(32));
        assert_eq!(parse("pesëdhjetedy"), Some(52));
        assert_eq!(parse("gjashtëqindegjashtëdhjetegjashtë"), Some(666));
        assert_eq!(reading("dyzet").map(|r| r.rule), Some(R_READ));
        assert_eq!(parse(&"tree".repeat(128)), None);
        assert_eq!(parse_ordinal(&format!("i {}xtë", "dye".repeat(169))), None);
    }

    /// Numërimi dialektor me bazë 20 nuk ka dëshmi në leksikun e gramatikës: asnjë lexim.
    #[test]
    fn vigesimal_dialect_forms_are_held() {
        let held = [
            "trezet", "trizet", "katërzet", "pesëzet", "gjashtëzet", "trezet e dhjetë", "katërzetedhjetë", "dyzet e dhjetë",
            "njëzet e dhjetë", "njëzetedhjetë", "dy zet", "një zet", "zet", "d y z e t", "dyz et", "dy-zet",
        ];
        for w in held {
            assert_eq!(parse(w), None, "{w}");
        }
    }

    #[test]
    fn ordinals() {
        let o = |n: u64| ordinal(n, Gender::Masc).unwrap();
        assert_eq!(o(1), "i parë");
        assert_eq!(ordinal(1, Gender::Fem).unwrap(), "e parë");
        assert_eq!(o(2), "i dytë");
        assert_eq!(o(3), "i tretë");
        assert_eq!(o(4), "i katërt");
        assert_eq!(o(5), "i pestë");
        assert_eq!(o(6), "i gjashtë");
        assert_eq!(o(10), "i dhjetë");
        assert_eq!(o(13), "i trembëdhjetë");
        assert_eq!(o(20), "i njëzetë");
        assert_eq!(o(21), "i njëzetenjëtë");
        assert_eq!(o(24), "i njëzetekatërt");
        assert_eq!(o(32), "i tridhjetedytë");
        assert_eq!(o(40), "i dyzetë");
        assert_eq!(o(100), "i njëqindtë");
        assert_eq!(o(102), "i njëqindedytë");
        assert_eq!(o(300), "i treqindtë");
        assert_eq!(o(2_000), "i dymijtë");
        assert_eq!(o(1_000_000), "i njëmiliontë");
        let k = ordinal_form(1_000, Gender::Masc).unwrap();
        assert_eq!(k.text, "i njëmijtë");
        assert!(k.accepts("i mijtë"));
        assert!(!k.accepts("i një mijtë"), "shkrimi ndaras vetëm për lexim");
        let t = ordinal_form(3_000, Gender::Masc).unwrap();
        assert_eq!(t.text, "i trimijtë");
        assert_eq!(t.variants, vec!["i tremijtë".to_string()]);
        assert!(ordinal_form(3_000_000, Gender::Masc).unwrap().variants.is_empty());
        assert_eq!(ordinal_form(4, Gender::Fem).unwrap().rule, R_ORD_T);
        assert!(ordinal(0, Gender::Masc).is_none());
        assert!(ordinal(121, Gender::Masc).is_none(), "tre përbërës: asnjë hamendje");
        assert!(ordinal(205, Gender::Masc).is_none(), "pa dëshmi në leksik: asnjë hamendje");
        assert!(ordinal(2, Gender::Neut).is_none());
    }

    #[test]
    fn ordinal_reading_is_the_inverse() {
        for n in ordinal_domain() {
            for g in [Gender::Masc, Gender::Fem] {
                let f = ordinal_form(n, g).unwrap();
                for text in std::iter::once(&f.text).chain(f.variants.iter()) {
                    let word = text.split_once(' ').map(|(_, w)| w).unwrap();
                    let ambiguous = g == Gender::Fem && parse(word).is_some();
                    assert_eq!(parse_ordinal(text), (!ambiguous).then_some(n), "{text}");
                }
            }
        }
        assert_eq!(parse_ordinal("E   Tretë"), Some(3));
        assert_eq!(parse_ordinal("katërt"), Some(4));
        assert_eq!(parse_ordinal("e dytë"), Some(2));
        assert_eq!(parse_ordinal("i gjashtë"), Some(6));
        assert_eq!(parse_ordinal("e njëzetë"), Some(20));
        assert_eq!(parse_ordinal("i trimijtë"), Some(3_000));
        assert_eq!(parse_ordinal("i tremijtë"), Some(3_000));
        assert_eq!(ordinal_reading("i tri mijtë"), Some(Reading { value: 3_000, rule: R_ORD_READ_SPLIT }));
        assert_eq!(ordinal_reading("i parë"), Some(Reading { value: 1, rule: R_ORD_FIRST }));
        assert_eq!(ordinal_reading("e qindtë"), Some(Reading { value: 100, rule: R_ORD_BARE }));
        assert_eq!(ordinal_reading("i një mijtë"), Some(Reading { value: 1_000, rule: R_ORD_READ_SPLIT }));
        assert_eq!(ordinal_reading("i njëzetenjëtë"), Some(Reading { value: 21, rule: R_ORD_READ }));
        for w in ["", "i", "e", "i pesët", "i njëzet e njëtë", "i tre", "parë parë", "i një qindtë", "të dytë", "i i dytë", "i mijë"] {
            assert_eq!(parse_ordinal(w), None, "{w}");
        }
        for w in ["gjashtë", "tetë", "dhjetë", "njëmbëdhjetë", "tridhjetë", "pesëdhjetë", "e gjashtë", "e dhjetë", "e tridhjetë"] {
            assert_eq!(parse_ordinal(w), None, "i dykuptimtë me themelorin: {w}");
        }
    }

    #[test]
    fn fractions_and_decimals() {
        let h = fraction_form(1, 2).unwrap();
        assert_eq!(h.text, "një e dyta");
        assert!(h.accepts("gjysmë"));
        assert_eq!(fraction(1, 3).unwrap(), "një e treta");
        assert_eq!(fraction(2, 3).unwrap(), "dy të tretat");
        let q = fraction_form(3, 4).unwrap();
        assert_eq!(q.text, "tri të katërtat");
        assert!(q.accepts("tre të katërtat"));
        assert_eq!(q.rule, R_FRACTION);
        let c = fraction_form(1, 100).unwrap();
        assert_eq!(c.text, "një e qindta");
        assert_eq!(c.rule, R_FRACTION_BARE);
        assert_eq!(fraction(1, 21).unwrap(), "një e njëzetenjëta");
        assert!(fraction(0, 5).is_none());
        assert!(fraction(5, 0).is_none());
        assert!(fraction(1, 1).is_none());
        assert_eq!(decimal("3,14").unwrap(), "tre presje katërmbëdhjetë");
        assert_eq!(decimal("0,05").unwrap(), "zero presje zero pesë");
        assert_eq!(decimal("-2,5").unwrap(), "minus dy presje pesë");
        assert_eq!(decimal("3,141").unwrap(), "tre presje njëqind e dyzet e një");
        for bad in ["3.14", "3,", ",5", "03,1", "abc", "1,2,3", "3,1x", "3,1415", "3,141592653589"] {
            assert!(decimal(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn years() {
        assert_eq!(year(1941).unwrap(), "një mijë e nëntëqind e dyzet e një");
        assert_eq!(year(2024).unwrap(), "dy mijë e njëzet e katër");
        assert_eq!(year_form(1912).unwrap().rule, R_YEAR);
        assert!(year(0).is_none());
    }

    /// Mostër e kontrolluar me dorë sipas gramatikës standarde, për pjesët pa ar të jashtëm:
    /// themelorët mbi 100, rreshtorët mbi 5, thyesat dhe vitet.
    #[test]
    fn textbook_sample() {
        let card = [
            (101, "njëqind e një"),
            (300, "treqind"),
            (555, "pesëqind e pesëdhjetë e pesë"),
            (2_000, "dy mijë"),
            (10_000, "dhjetë mijë"),
            (100_000, "njëqind mijë"),
            (2_000_000, "dy milionë"),
            (3_000_000_000, "tre miliardë"),
        ];
        for (n, w) in card {
            assert_eq!(c(n), w, "{n}");
        }
        let ord = [(7, "i shtatë"), (8, "i tetë"), (9, "i nëntë"), (11, "i njëmbëdhjetë"), (50, "i pesëdhjetë"), (60, "i gjashtëdhjetë"), (1_000, "i njëmijtë")];
        for (n, w) in ord {
            assert_eq!(ordinal(n, Gender::Masc).unwrap(), w, "{n}");
        }
        assert_eq!(ordinal(7, Gender::Fem).unwrap(), "e shtatë");
        let frac = [((1, 4), "një e katërta"), ((2, 5), "dy të pestat"), ((3, 10), "tri të dhjetat"), ((1, 1_000), "një e mijta")];
        for ((p, q), w) in frac {
            assert_eq!(fraction(p, q).unwrap(), w, "{p}/{q}");
        }
        assert_eq!(year(1912).unwrap(), "një mijë e nëntëqind e dymbëdhjetë");
        assert_eq!(year(2000).unwrap(), "dy mijë");
    }
}
