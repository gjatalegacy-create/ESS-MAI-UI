//! Rrokjezimi i fjalës së shkruar.
//!
//! Rendi i rregullave:
//! 1. çdo zanore është bërthamë rrokjeje; dy zanore ngjitur ndahen (Shqi-pë-ri-a, si-tu-a-të);
//! 2. çiftet ie, ye, ua, ue janë një bërthamë (diftong rënës) vetëm aty ku zanorja e parë
//!    theksohet në fjalët vendase: «-ua», «-ue» në fund (grua, për-rua); «-ie», «-ye» në fund
//!    pa zanore para (bie, krye); para një bashkëtingëlloreje të vetme fundore pa zanore para
//!    (diell, fier, zgjuath); «-ues-», «-yesh-», «-uaj-», «-iell-», «-yell-» kudo (më-sues,
//!    mua-ji, gër-fyell); «-uar», «-uam», «-uat», «-uan», «-yer» në fund (pu-nuar). Ndahen
//!    (dy bërthama) pas një zanoreje në fund ose para bashkëtingëlloreve fundore (qe-ni-e,
//!    ka-me-ri-er), para dy bashkëtingëlloreve fundore (kli-ent) dhe pas zanoreje me rrokje pas
//!    (si-tu-a-të, ar-sy-e-ja). Leximi është i pasigurt pa zanore para me rrokje pas (py-e-tje ~
//!    die-lli), te «-iej», «-yej» pa zanore para (ngjy-ej ~ krye-j), te «-yes» pas zanoreje
//!    (rrë-fyes ~ ar-sy-es) dhe te «-uar-» me rrokje pas (pu-nua-ra ~ ja-nu-a-ri);
//! 3. bashkëtingëllorja e vetme ndërmjet zanoresh shkon me rrokjen pasuese (bo-të);
//! 4. në grupin e bashkëtingëlloreve rrokja pasuese merr grupin nistor më të gjatë të
//!    ligjshëm (parimi i grupit nistor maksimal), ku i ligjshëm do të thotë sonoritet që
//!    rritet drejt zanores me të paktën dy shkallë (shkalla e Clements-it, largësia minimale
//!    e sonoritetit): pengore + gjysmëzanore (dhje-të), hundore + gjysmëzanore;
//! 5. grupet s/sh/z/zh + pengore dhe hundoret parapengore mb, nd, ng, ngj janë grupe nistore
//!    vetëm në fillim të fjalës (shtë-pi, mbret); në mes të fjalës ndahen (mos-tër, kën-gë);
//!    mbyllja e rrokjes nuk ngjitet në sonoritet (parimi i renditjes së sonoritetit);
//! 6. rrokjezimi kthen None kur burimet ndahen ose kufiri varet nga përbërja e fjalës:
//!    pengore + lëngëzore në mes të fjalës (a-krep ~ ak-rep), diftongjet e pikës 2 (grua ~ gru-a,
//!    zgjuath ~ zgju-ath), çiftet e pasigurta, bashkëtingëllore para mb, nd, ng, ngj, nx, nxh
//!    (për-mbaj, fat-ndjekës) dhe mbyllja që do të ngjitej në sonoritet.
//! Grupi nistor i fjalës duhet të jetë i ligjshëm sipas `legal_onset`, përndryshe None.

use super::alfabeti::{canonical, is_obstruent, is_vowel, letters, manner, sonority, Manner};

/// Çiftet e zanoreve që mund të jenë diftong i shqipes standarde (ie, ye, ua, ue).
pub const DIPHTHONGS: [(&str, &str); 4] = [("i", "e"), ("y", "e"), ("u", "a"), ("u", "e")];

/// Leximi i një çifti ie, ye, ua, ue sipas vendit të tij në fjalë.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pair {
    /// Një bërthamë: diftong rënës vendas (grua, diell, më-sues).
    Diphthong,
    /// Dy bërthama (kli-ent, qe-ni-e).
    Hiatus,
    /// Shkrimi nuk e tregon (py-e-tje ~ die-lli).
    Unsure,
}

/// Si lexohen çiftet e pasigurta kur numërohen bërthamat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reading {
    Join,
    Split,
}

/// Shkronjat kanonike të fjalës; None kur ka shenja jashtë alfabetit.
pub(crate) fn canonical_letters(word: &str) -> Option<Vec<&'static str>> {
    let units = letters(word);
    (!units.is_empty()).then_some(())?;
    units.iter().map(|u| canonical(u)).collect()
}

/// Leximi i çiftit që nis te shkronja `i` (shkronjat kanonike).
fn pair_kind(low: &[&str], i: usize) -> Pair {
    let first = low.get(i).copied().unwrap_or("");
    let second = low.get(i + 1).copied().unwrap_or("");
    let vowel_before = low.get(..i).map(|s| s.iter().any(|l| is_vowel(l))).unwrap_or(false);
    let after = low.get(i + 2..).unwrap_or(&[]);
    let cons = after.iter().take_while(|l| !is_vowel(l)).count();
    let coda = after.get(..cons).unwrap_or(&[]);
    let closed = cons == after.len();
    let front = matches!(first, "i" | "y");
    let pair_ua = first == "u" && second == "a";
    let lead = coda.first().copied().unwrap_or("");
    let steps = [
        (after.is_empty() && first == "u", Pair::Diphthong),
        (after.is_empty(), choose(vowel_before, Pair::Hiatus, Pair::Diphthong)),
        (first == "u" && second == "e" && matches!(lead, "s" | "sh"), Pair::Diphthong),
        (first == "y" && lead == "sh", Pair::Diphthong),
        (pair_ua && lead == "j", Pair::Diphthong),
        (front && lead == "ll", Pair::Diphthong),
        (first == "y" && coda == ["s"] && vowel_before, Pair::Unsure),
        (front && lead == "j" && !vowel_before, Pair::Unsure),
        (closed && !vowel_before && cons == 1, Pair::Diphthong),
        (closed && pair_ua && matches!(coda, ["r"] | ["m"] | ["t"] | ["n"]), Pair::Diphthong),
        (closed && first == "y" && coda == ["r"], Pair::Diphthong),
        (closed, Pair::Hiatus),
        (!vowel_before, Pair::Unsure),
        (pair_ua && coda == ["r"], Pair::Unsure),
    ];
    steps.iter().find(|(c, _)| *c).map(|(_, p)| *p).unwrap_or(Pair::Hiatus)
}

fn choose(c: bool, yes: Pair, no: Pair) -> Pair {
    match c {
        true => yes,
        false => no,
    }
}

/// Bërthamat e rrokjeve si vargje [fillim, fund) mbi shkronjat kanonike; çiftet e pasigurta
/// lexohen sipas `reading`.
pub(crate) fn nuclei(low: &[&str], reading: Reading) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < low.len() {
        let here = low.get(i).copied().unwrap_or("");
        let next = low.get(i + 1).copied().unwrap_or("");
        let pair = DIPHTHONGS.contains(&(here, next));
        let join = pair
            && match pair_kind(low, i) {
                Pair::Diphthong => true,
                Pair::Hiatus => false,
                Pair::Unsure => reading == Reading::Join,
            };
        let step = match (is_vowel(here), join) {
            (true, true) => {
                out.push((i, i + 2));
                2
            }
            (true, false) => {
                out.push((i, i + 1));
                1
            }
            (false, _) => 1,
        };
        i += step;
    }
    out
}

/// Leximet e çifteve ie, ye, ua, ue të fjalës, nga e majta.
pub(crate) fn pairs(low: &[&str]) -> Vec<Pair> {
    (0..low.len())
        .filter(|i| {
            let here = low.get(*i).copied().unwrap_or("");
            let next = low.get(i + 1).copied().unwrap_or("");
            let previous_pair = i.checked_sub(1).and_then(|p| low.get(p)).map(|p| DIPHTHONGS.contains(&(*p, here))).unwrap_or(false);
            DIPHTHONGS.contains(&(here, next)) && !previous_pair
        })
        .map(|i| pair_kind(low, i))
        .collect()
}

/// Numri i rrokjeve sipas normës (diftongu vendas = një rrokje); None kur fjala ka një çift
/// zanoresh me lexim të pasigurt. Nuk varet nga vendi i kufirit brenda grupeve.
pub fn syllable_count(word: &str) -> Option<usize> {
    let low = canonical_letters(word)?;
    (!pairs(&low).contains(&Pair::Unsure)).then_some(())?;
    let n = nuclei(&low, Reading::Join).len();
    (n > 0).then_some(n)
}

fn is_liquid(l: &str) -> bool {
    manner(l) == Some(Manner::Liquid)
}

/// Grup nistor në mes të fjalës: zero ose një bashkëtingëllore, ose dy me sonoritet që
/// rritet të paktën dy shkallë.
fn medial_onset(cl: &[&str]) -> bool {
    match cl {
        [] | [_] => true,
        [a, b] => sonority(b).zip(sonority(a)).map(|(sb, sa)| sb >= sa + 2 && sb < 5).unwrap_or(false),
        _ => false,
    }
}

/// Bashkëtingëllorja që mund të paraprijë një grup nistor vetëm në fillim të fjalës.
fn appendix(a: &str, next: &str) -> bool {
    match a {
        "s" | "sh" | "z" | "zh" | "ç" | "c" => !is_vowel(next),
        "m" => matches!(next, "b" | "p") || is_liquid(next),
        "n" => matches!(next, "d" | "g" | "gj" | "x" | "xh" | "k"),
        "f" => matches!(next, "t" | "sh" | "q" | "s"),
        "v" => next == "d",
        "k" => matches!(next, "th" | "s"),
        "g" => next == "dh",
        "t" => matches!(next, "k" | "m"),
        "p" => matches!(next, "s" | "sh"),
        _ => false,
    }
}

/// Grup nistor në fillim të fjalës: grupi i ligjshëm në mes, lëngëzore + j (rrjedh), ose një
/// shtesë fillestare para grupi të ligjshëm: s/sh/z/zh/ç + bashkëtingëllore (shkrep, zgjat,
/// çmoj), hundore parapengore (mbret, ndjek, ngjyrë, nxjerr), si edhe ft, fsh, fq, vd, kth,
/// gdh, tk, tm, ps.
fn initial_onset(cl: &[&str]) -> bool {
    let liquid_glide = match cl {
        [a, b] => is_liquid(a) && *b == "j",
        _ => false,
    };
    medial_onset(cl)
        || liquid_glide
        || cl.split_first().map(|(a, rest)| rest.first().map(|n| appendix(a, n)).unwrap_or(false) && initial_onset(rest)).unwrap_or(false)
}

/// A është i ligjshëm grupi i bashkëtingëlloreve si fillim rrokjeje (në fillim ose në mes të fjalës).
pub fn legal_onset(cluster: &[&str], word_initial: bool) -> bool {
    let low: Option<Vec<&str>> = cluster.iter().map(|u| canonical(u)).collect();
    low.filter(|l| l.iter().all(|x| !is_vowel(x)))
        .map(|l| match word_initial {
            true => initial_onset(&l),
            false => medial_onset(&l),
        })
        .unwrap_or(false)
}

/// Grupi pengore + lëngëzore: kufiri i tij nuk është i qëndrueshëm.
fn ambiguous_onset(onset: &[&str]) -> bool {
    match onset {
        [a, b] => is_obstruent(a) && is_liquid(b),
        _ => false,
    }
}

/// Hundore parapengore (mb, nd, ng, ngj, nx, nxh) pas një bashkëtingëlloreje: kufi parashtese
/// ose fjale të përbërë (për-mbaj, fat-ndjekës, katër-mbë-dhjetë).
fn prenasal_after_consonant(cl: &[&str]) -> bool {
    cl.windows(2).skip(1).any(|w| matches!(w, ["m", "b"] | ["n", "d" | "g" | "gj" | "x" | "xh"]))
}

/// Mbyllja e rrokjes ngjitet në sonoritet (t → n te «fatn-»).
fn rising_coda(coda: &[&str]) -> bool {
    coda.windows(2).any(|w| sonority(w[1]).zip(sonority(w[0])).map(|(b, a)| b > a).unwrap_or(true))
}

/// Kufijtë e rrokjeve si indekse shkronjash (pa 0 dhe pa fundin).
fn boundaries(low: &[&str]) -> Option<Vec<usize>> {
    let nuc = nuclei(low, Reading::Split);
    let first = nuc.first()?;
    let initial = low.get(..first.0)?;
    legal_onset(initial, true).then_some(())?;
    nuc.windows(2)
        .map(|w| {
            let (end, next) = (w[0].1, w[1].0);
            let cl = low.get(end..next)?;
            (!prenasal_after_consonant(cl)).then_some(())?;
            let keep = (0..=cl.len()).find(|k| medial_onset(cl.get(*k..).unwrap_or(&[])))?;
            let onset = cl.get(keep..)?;
            let coda = cl.get(..keep)?;
            (!ambiguous_onset(onset) && !rising_coda(coda)).then_some(end + keep)
        })
        .collect()
}

/// Rrokjet e fjalës, me shkronjat e saj origjinale; None kur kufiri nuk është i sigurt.
pub fn syllables(word: &str) -> Option<Vec<String>> {
    let units = letters(word);
    let low: Vec<&str> = units.iter().map(|u| canonical(u)).collect::<Option<Vec<_>>>()?;
    pairs(&low).iter().all(|p| *p == Pair::Hiatus).then_some(())?;
    let cuts = boundaries(&low)?;
    let edges: Vec<usize> = std::iter::once(0).chain(cuts).chain(std::iter::once(units.len())).collect();
    Some(edges.windows(2).map(|w| units.get(w[0]..w[1]).map(|s| s.concat()).unwrap_or_default()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syl(w: &str) -> Option<String> {
        syllables(w).map(|s| s.join("-"))
    }

    #[test]
    fn single_consonant_goes_to_the_next_syllable() {
        assert_eq!(syl("botë").as_deref(), Some("bo-të"));
        assert_eq!(syl("shtëpi").as_deref(), Some("shtë-pi"));
        assert_eq!(syl("qytetari").as_deref(), Some("qy-te-ta-ri"));
        assert_eq!(syl("Kosova").as_deref(), Some("Ko-so-va"));
        assert_eq!(syl("vallahi").as_deref(), Some("va-lla-hi"));
    }

    #[test]
    fn falling_or_flat_sonority_splits_the_cluster() {
        assert_eq!(syl("mostër").as_deref(), Some("mos-tër"));
        assert_eq!(syl("këngë").as_deref(), Some("kën-gë"));
        assert_eq!(syl("kërkoj").as_deref(), Some("kër-koj"));
        assert_eq!(syl("hajdut").as_deref(), Some("haj-dut"));
        assert_eq!(syl("pesha").as_deref(), Some("pe-sha"));
        assert_eq!(syl("ëmbël").as_deref(), Some("ëm-bël"));
    }

    #[test]
    fn rising_onsets_and_vowel_pairs() {
        assert_eq!(syl("dhjetë").as_deref(), Some("dhje-të"));
        assert_eq!(syl("kështjellë").as_deref(), Some("kësh-tje-llë"));
        assert_eq!(syl("Shqipëria").as_deref(), Some("Shqi-pë-ri-a"));
        assert_eq!(syl("hernie").as_deref(), Some("her-ni-e"));
        assert_eq!(syl("situatë").as_deref(), Some("si-tu-a-të"));
        assert_eq!(syl("klient").as_deref(), Some("kli-ent"));
        assert_eq!(syl("grua"), None);
        assert_eq!(syl("mësues"), None);
        assert_eq!(syl("pyetje"), None);
    }

    #[test]
    fn pair_readings() {
        let kind = |w: &str| pairs(&canonical_letters(w).unwrap_or_default());
        assert_eq!(kind("grua"), vec![Pair::Diphthong]);
        assert_eq!(kind("diell"), vec![Pair::Diphthong]);
        assert_eq!(kind("dielli"), vec![Pair::Diphthong]);
        assert_eq!(kind("mësuesi"), vec![Pair::Diphthong]);
        assert_eq!(kind("punuar"), vec![Pair::Diphthong]);
        assert_eq!(kind("gruaja"), vec![Pair::Diphthong]);
        assert_eq!(kind("qenie"), vec![Pair::Hiatus]);
        assert_eq!(kind("arsyeja"), vec![Pair::Hiatus]);
        assert_eq!(kind("kamerier"), vec![Pair::Hiatus]);
        assert_eq!(kind("pacient"), vec![Pair::Hiatus]);
        assert_eq!(kind("pyetje"), vec![Pair::Unsure]);
        assert_eq!(kind("mbiemër"), vec![Pair::Unsure]);
        assert_eq!(syllable_count("grua"), Some(1));
        assert_eq!(syllable_count("dielli"), Some(2));
        assert_eq!(syllable_count("klient"), Some(2));
        assert_eq!(syllable_count("qenie"), Some(3));
        assert_eq!(syllable_count("tryezë"), None);
    }

    #[test]
    fn unstable_boundaries_are_held() {
        assert_eq!(syl("akrep"), None);
        assert_eq!(syl("fabrikë"), None);
        assert_eq!(syl("psst"), None);
        assert_eq!(syl("e hënë"), None);
        assert_eq!(syl("përmbajtje"), None);
        assert_eq!(syl("fatndjekës"), None);
        assert_eq!(syl("katërmbëdhjetë"), None);
        assert_eq!(syllable_count("akrep"), Some(2));
    }

    #[test]
    fn word_initial_onsets() {
        let l = |w: &str| canonical_letters(w).unwrap_or_default();
        assert!(legal_onset(&l("shtr"), true));
        assert!(legal_onset(&l("mbr"), true));
        assert!(legal_onset(&l("ngj"), true));
        assert!(legal_onset(&l("zgj"), true));
        assert!(legal_onset(&l("kth"), true));
        assert!(legal_onset(&l("rrj"), true));
        assert!(!legal_onset(&l("rk"), true));
        assert!(!legal_onset(&l("st"), false));
        assert!(!legal_onset(&l("nd"), false));
        assert!(legal_onset(&l("dhj"), false));
        assert!(legal_onset(&["SH", "T", "R"], true));
        assert!(!legal_onset(&["a"], false));
    }
}
