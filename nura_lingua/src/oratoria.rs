//! Oratoria: mjeshtëria e të folurit të Nurës, vetëm me formula të deklaruara.
//!
//! Çdo matje është aritmetikë me numra të plotë, në mijësha (0..=1000): pa probabilitet, pa
//! rastësi, e njëjta hyrje jep gjithmonë të njëjtin numër. Rrokjet vijnë nga
//! `fonologjia::rrokjet`, theksi nga `fonologjia::theksi`.
//!
//! Shënimet: për një fjali, `L` = numri i rrokjeve; `P₁ < P₂ < … < Pₘ` = pozitat e rrokjeve të
//! theksuara (nga fillimi, duke filluar nga 0); `Δₖ = Pₖ₊₁ − Pₖ`, `n = m − 1`.
//!
//! 1. Ritmi (rregullsia e intervaleve mes thekseve):
//!    `R = 1000 · n² / (n² + n·ΣΔ² − (ΣΔ)²)`, që është `1000 / (1 + Var(Δ))`; me më pak se tre
//!    thekse, `R = 1000` (s'ka çfarë të matet).
//! 2. Kadenca (mbyllja e fjalisë sipas *cursus*-it të retorikës klasike): `a` = largësia e theksit
//!    të fundit nga rrokja e fundit, `b` = largësia e theksit të parafundit. Format e mbylljes:
//!    *planus* `/ x x / x` (a=1, b=4), *tardus* `/ x x / x x` (a=2, b=5), *velox* `/ x x x x / x`
//!    (a=1, b=6), *trispondaicus* `/ x x x / x` (a=1, b=5).
//!    `K = max(0, 1000 − 150·d)`, ku `d` = largësia më e vogël `|a−a′| + |b−b′|` nga këto forma.
//! 3. Baraspesha e kolonave (*isokolon*): fjalia ndahet te «,», «;», «:» dhe te lidhëzat «dhe»,
//!    «ndërsa», «por», «ose»; për çdo dy kolona fqinje me `Lᵢ, Lⱼ` rrokje,
//!    `Bᵢⱼ = 1000 − 1000·|Lᵢ − Lⱼ| / (Lᵢ + Lⱼ)`; `B` = mesatarja (me një kolonë, 1000).
//! 4. Rritja e pjesëve (ligji i Behaghel-it, 1909, «Gesetz der wachsenden Glieder»): ndër të
//!    gjitha çiftet e kolonave `i < j`, `G = 1000 · #{Lᵢ ≤ Lⱼ} / #çifteve` (me një kolonë, 1000).
//! 5. Fryma (grupi i frymëmarrjes): `F = 1000` për `8 ≤ L ≤ 24`; `F = 1000·L/8` për `L < 8`;
//!    `F = max(0, 1000 − 50·(L − 24))` për `L > 24`.
//!
//! Vlerësimi i përgjithshëm: `O = (2R + 3K + 2B + 2G + F) / 10`.
//!
//! Rregullat e theksit: fjalët e pandara pa theks të vetin (nyjat, parafjalët, pjesëzat, trajtat
//! e shkurtra të përemrave) nuk kanë theks; kur rregulli i theksit nuk e di theksin e një fjale
//! me shumë rrokje, theksi vihet te rrokja e parafundit (vendi i theksit të shqipes standarde),
//! dhe numërohet si «theks i paracaktuar». Kur rregulli i rrokjeve nuk e di numrin e rrokjeve,
//! numërohen grupet e zanoreve.
//!
//! Stil: pa if/else — vetëm match.

use crate::gramatika::fonologjia::{stressed_syllable, syllable_count, Hint};

/// Fjalët që s'kanë theks të vetin në ligjërim (proklitika dhe enklitika).
const CLITICS: &[&str] = &[
    "i", "e", "të", "së", "u", "t'u", "ta", "ia", "iu", "ma", "m'i", "t'i", "më", "ju", "na", "ia", "në", "me", "për", "nga", "te",
    "tek", "mbi", "nën", "pa", "që", "do", "dhe", "e", "a", "o", "se", "si", "ku",
];

/// Lidhëzat ku ndahen kolonat.
const COLON_JOINERS: &[&str] = &["dhe", "ndërsa", "por", "ose", "apo"];

/// Format e mbylljes sipas *cursus*-it: (a, b) = largësitë e dy thekseve të fundit nga fundi.
const CURSUS: [(&str, usize, usize); 4] = [("planus", 1, 4), ("tardus", 2, 5), ("velox", 1, 6), ("trispondaicus", 1, 5)];

/// Matjet e një teksti.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Matje {
    /// Numri i rrokjeve.
    pub rrokje: usize,
    /// Pozitat e rrokjeve të theksuara, nga fillimi.
    pub theks: Vec<usize>,
    /// Sa thekse u vunë me rregullin e paracaktuar (rrokja e parafundit).
    pub paracaktuar: usize,
}

/// Vlerësimi retorik, në mijësha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Vleresim {
    pub ritmi: u32,
    pub kadenca: u32,
    pub baraspesha: u32,
    pub rritja: u32,
    pub fryma: u32,
    pub gjithsej: u32,
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphabetic() || c == '\''))
        .map(|w| w.trim_matches('\'').to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Numri i grupeve të zanoreve (kur rregulli i rrokjeve nuk e di).
fn vowel_groups(w: &str) -> usize {
    let vowel = |c: char| "aeëiouy".contains(c);
    let mut n = 0;
    let mut prev = false;
    for c in w.chars() {
        let v = vowel(c);
        n += usize::from(v && !prev);
        prev = v;
    }
    n.max(1)
}

/// Rrokjet dhe theksi i një fjale: (rrokjet, rrokja e theksuar nga fundi, e paracaktuar?).
fn word_measure(w: &str) -> (usize, Option<usize>, bool) {
    let n = syllable_count(w).unwrap_or_else(|| vowel_groups(w));
    match CLITICS.contains(&w) {
        true => (n, None, false),
        false => match stressed_syllable(w, Hint::Unknown) {
            Some(s) => (n, Some(s.min(n.saturating_sub(1))), false),
            None => (n, Some(usize::from(n >= 2)), true),
        },
    }
}

/// Rrokjet dhe pozitat e thekseve të një teksti.
pub fn mat(text: &str) -> Matje {
    let mut m = Matje::default();
    for w in words(text) {
        let (n, from_end, default) = word_measure(&w);
        from_end.iter().for_each(|s| m.theks.push(m.rrokje + n - 1 - s));
        m.paracaktuar += usize::from(default && from_end.is_some());
        m.rrokje += n;
    }
    m
}

/// Ritmi: `1000 · n² / (n² + n·ΣΔ² − (ΣΔ)²)`.
pub fn ritmi(m: &Matje) -> u32 {
    let d: Vec<i64> = m.theks.windows(2).map(|w| w[1] as i64 - w[0] as i64).collect();
    let n = d.len() as i64;
    let s: i64 = d.iter().sum();
    let s2: i64 = d.iter().map(|x| x * x).sum();
    match n >= 2 {
        true => (1000 * n * n / (n * n + n * s2 - s * s)) as u32,
        false => 1000,
    }
}

/// Kadenca sipas *cursus*-it.
pub fn kadenca(m: &Matje) -> u32 {
    let from_end: Vec<usize> = m.theks.iter().rev().take(2).map(|p| m.rrokje - 1 - p).collect();
    match from_end.as_slice() {
        [a, b] => {
            let d = CURSUS.iter().map(|(_, x, y)| a.abs_diff(*x) + b.abs_diff(*y)).min().unwrap_or(0);
            1000u32.saturating_sub(150 * d as u32)
        }
        _ => 0,
    }
}

/// Emri i formës së mbylljes, kur fjalia mbyllet saktë me njërën.
pub fn cursus(m: &Matje) -> Option<&'static str> {
    let from_end: Vec<usize> = m.theks.iter().rev().take(2).map(|p| m.rrokje - 1 - p).collect();
    match from_end.as_slice() {
        [a, b] => CURSUS.iter().find(|(_, x, y)| a == x && b == y).map(|(n, _, _)| *n),
        _ => None,
    }
}

/// Kolonat e një fjalie (rrokjet e secilës).
pub fn kolonat(text: &str) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for part in text.split([',', ';', ':']) {
        let mut cur = 0usize;
        for w in words(part) {
            match COLON_JOINERS.contains(&w.as_str()) && cur > 0 {
                true => {
                    out.push(cur);
                    cur = 0;
                }
                false => {}
            }
            cur += word_measure(&w).0;
        }
        match cur > 0 {
            true => out.push(cur),
            false => {}
        }
    }
    out
}

/// Baraspesha e kolonave fqinje.
pub fn baraspesha(cola: &[usize]) -> u32 {
    let pairs: Vec<u32> = cola
        .windows(2)
        .map(|w| (1000 - 1000 * w[0].abs_diff(w[1]) / (w[0] + w[1]).max(1)) as u32)
        .collect();
    match pairs.is_empty() {
        true => 1000,
        false => pairs.iter().sum::<u32>() / pairs.len() as u32,
    }
}

/// Rritja e pjesëve sipas ligjit të Behaghel-it.
pub fn rritja(cola: &[usize]) -> u32 {
    let n = cola.len();
    let total = n * n.saturating_sub(1) / 2;
    let rising = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).filter(|(i, j)| cola[*i] <= cola[*j]).count();
    match total {
        0 => 1000,
        t => (1000 * rising / t) as u32,
    }
}

/// Fryma: gjatësia e fjalisë kundrejt grupit të frymëmarrjes (8..=24 rrokje).
pub fn fryma(l: usize) -> u32 {
    match (l < 8, l > 24) {
        (true, _) => (1000 * l / 8) as u32,
        (false, true) => 1000u32.saturating_sub(50 * (l - 24) as u32),
        (false, false) => 1000,
    }
}

/// Vlerësimi i një fjalie.
pub fn vleresim(text: &str) -> Vleresim {
    let m = mat(text);
    let cola = kolonat(text);
    let (r, k, b, g, f) = (ritmi(&m), kadenca(&m), baraspesha(&cola), rritja(&cola), fryma(m.rrokje));
    Vleresim { ritmi: r, kadenca: k, baraspesha: b, rritja: g, fryma: f, gjithsej: (2 * r + 3 * k + 2 * b + 2 * g + f) / 10 }
}

/// Vlerësimi i një përgjigjeje me disa fjali: mesatarja e fjalive.
pub fn vleresim_teksti(sentences: &[String]) -> Vleresim {
    let all: Vec<Vleresim> = sentences.iter().map(|s| vleresim(s)).collect();
    let n = all.len().max(1) as u32;
    let sum = |f: fn(&Vleresim) -> u32| all.iter().map(f).sum::<u32>() / n;
    Vleresim {
        ritmi: sum(|v| v.ritmi),
        kadenca: sum(|v| v.kadenca),
        baraspesha: sum(|v| v.baraspesha),
        rritja: sum(|v| v.rritja),
        fryma: sum(|v| v.fryma),
        gjithsej: sum(|v| v.gjithsej),
    }
}

/// Rendi i pjesëve të një renditjeje sipas ligjit të Behaghel-it: nga më e shkurtra te më e
/// gjata (në rrokje); pjesët me gjatësi të njëjtë ruajnë rendin e tyre.
pub fn rendit(items: &[String]) -> Vec<String> {
    let mut v: Vec<(usize, usize, String)> = items.iter().enumerate().map(|(i, s)| (mat(s).rrokje, i, s.clone())).collect();
    v.sort();
    v.into_iter().map(|(_, _, s)| s).collect()
}

/// Ndër variantet me të njëjtin kuptim, ai me vlerësimin më të lartë; në barazim, i pari.
pub fn zgjidh(variants: &[String]) -> Option<&String> {
    variants
        .iter()
        .enumerate()
        .max_by(|(i, a), (j, b)| vleresim(a).gjithsej.cmp(&vleresim(b).gjithsej).then(j.cmp(i)))
        .map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllables_and_stress_are_counted() {
        let m = mat("Qeni leh");
        assert_eq!(m.rrokje, 3);
        assert_eq!(m.theks, vec![0, 2]);
    }

    #[test]
    fn clitics_carry_no_stress() {
        let m = mat("për të ngrënë");
        assert_eq!(m.theks.len(), 1, "{m:?}");
    }

    #[test]
    fn regular_rhythm_scores_full_and_irregular_less() {
        let even = Matje { rrokje: 9, theks: vec![0, 2, 4, 6, 8], paracaktuar: 0 };
        let uneven = Matje { rrokje: 9, theks: vec![0, 1, 5, 6, 8], paracaktuar: 0 };
        assert_eq!(ritmi(&even), 1000);
        assert!(ritmi(&uneven) < 1000);
    }

    #[test]
    fn cursus_endings_are_recognised() {
        // / x x / x
        let planus = Matje { rrokje: 5, theks: vec![0, 3], paracaktuar: 0 };
        assert_eq!(cursus(&planus), Some("planus"));
        assert_eq!(kadenca(&planus), 1000);
        let flat = Matje { rrokje: 5, theks: vec![3, 4], paracaktuar: 0 };
        assert!(kadenca(&flat) < 1000);
    }

    #[test]
    fn balance_and_growth_follow_the_formulas() {
        assert_eq!(baraspesha(&[6, 6]), 1000);
        assert_eq!(baraspesha(&[2, 6]), 500);
        assert_eq!(rritja(&[2, 4, 6]), 1000);
        assert_eq!(rritja(&[6, 4, 2]), 0);
        assert_eq!(fryma(16), 1000);
        assert_eq!(fryma(4), 500);
        assert_eq!(fryma(30), 700);
    }

    #[test]
    fn behaghel_puts_the_longer_part_last() {
        let items = vec!["për të ngrënë ushqim".to_string(), "për të hapur".to_string(), "për të ndarë".to_string()];
        let r = rendit(&items);
        assert_eq!(r.last().map(String::as_str), Some("për të ngrënë ushqim"));
        assert_eq!(rendit(&r), r, "i qëndrueshëm");
    }

    #[test]
    fn the_choice_is_deterministic() {
        let v = vec!["Qeni leh dhe vrapon shpejt nëpër fushë.".to_string(), "Qeni vrapon shpejt nëpër fushë dhe leh.".to_string()];
        assert_eq!(zgjidh(&v), zgjidh(&v));
        assert_eq!(vleresim(&v[0]), vleresim(&v[0]));
    }
}
