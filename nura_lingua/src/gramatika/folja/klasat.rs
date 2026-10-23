//! Klasat e zgjedhimit sipas uniparser-grammar-albanian 2.3.0 (MIT):
//! për çdo klasë, cila temë (indeksi në listën e temave të leksemës),
//! me cilën ndajshtesë, merr cilën nënparadigmë mbaresash.
//! Si trajtë kryesore jepen vetëm mbaresat standarde. Mbaresa «-shit»/«-çit» e
//! vetës II shumës së dëshirores, që gramatika e shënon jostandarde, mbahet vetëm
//! si variant i pranuar për njohje, kurrë si trajtë kryesore; trajtat dialektore,
//! gegërishte dhe të vjetruara lihen jashtë, sepse këtu prodhohet, s'analizohet.

/// Veta dhe numri: 0 = I nj., 1 = II nj., 2 = III nj., 3 = I sh., 4 = II sh., 5 = III sh.
pub type Pn = u8;

/// Qeliza sintetike e paradigmës.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    Pres(Pn),
    /// Lidhorja e tashme, vetëm aty ku ndryshon nga dëftorja (II e III njëjës).
    Subj(Pn),
    Ipf(Pn),
    Aor(Pn),
    /// Urdhërorja: 0 = njëjës, 1 = shumës.
    Imp(u8),
    AdmPres(Pn),
    AdmIpf(Pn),
    Opt(Pn),
    Ptcp,
    MidPres(Pn),
    MidIpf(Pn),
    /// Aoristi jovepror i vetës III njëjës (pa mbaresë, pas «u»).
    MidAor3,
    MidImp(u8),
}

/// Kushti mbi fundin e bazës (tema + ndajshtesa) kur dy mbaresa garojnë për një qelizë.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phon {
    Any,
    Vowel,
    Cons,
    /// Diftong në fund («ua», «ue», «ye», «ie»): pjesorja merr «-r».
    Diph,
    /// Zanore e thjeshtë në fund: pjesorja merr «-rë».
    NoDiph,
    /// Urdhërorja ruan «-j» pas «a», «ë», «e», «ua» (laj, bëj, gjej, shkruaj).
    KeepJ,
    /// Urdhërorja e humb «-j» pas «i», «y», «ye», «ie» (fshi, lëpi, krye).
    DropJ,
    /// Dëshirorja «-sh-» bëhet «-ç-» pas hundores «n», «nj» (ndenjça, dhënça).
    Nasal,
    NoNasal,
    /// Aoristi III njëjës merr «-u» pas fytores «k», «g», «h» (iku, preku, zbehu), «-i» pas të tjerave.
    Velar,
    NoVelar,
}

/// Një mbaresë e nënparadigmës.
#[derive(Debug, Clone, Copy)]
pub struct End {
    pub slot: Slot,
    pub text: &'static str,
    pub phon: Phon,
    /// Mbaresa e posaçme mund mbi atë të përgjithshmen për të njëjtën qelizë.
    pub strong: bool,
    /// Variant jostandard i pranuar i së njëjtës qelizë («-shit» krahas «-shi»); bosh kur s'ka.
    pub alt: &'static str,
}

const fn e(slot: Slot, text: &'static str) -> End {
    End { slot, text, phon: Phon::Any, strong: false, alt: "" }
}

const fn ep(slot: Slot, text: &'static str, phon: Phon) -> End {
    End { slot, text, phon, strong: false, alt: "" }
}

const fn es(slot: Slot, text: &'static str) -> End {
    End { slot, text, phon: Phon::Any, strong: true, alt: "" }
}

const fn ea(slot: Slot, text: &'static str, alt: &'static str) -> End {
    End { slot, text, phon: Phon::Any, strong: false, alt }
}

const fn epa(slot: Slot, text: &'static str, phon: Phon, alt: &'static str) -> End {
    End { slot, text, phon, strong: false, alt }
}

use Phon::{Cons, Diph, DropJ, KeepJ, Nasal, NoDiph, NoNasal, NoVelar, Velar, Vowel};
use Slot::*;

// E tashmja dëftore veprore.
const P1A: &[End] = &[ep(Pres(0), "j", Vowel)];
const P1B: &[End] = &[ep(Pres(0), "", Cons)];
const P23A: &[End] = &[ep(Pres(1), "n", Vowel), ep(Pres(2), "n", Vowel)];
const P23B: &[End] = &[ep(Pres(1), "", Cons), ep(Pres(2), "", Cons)];
const P2PL: &[End] = &[e(Pres(4), "ni")];
const P13A: &[End] = &[e(Pres(3), "jmë"), e(Pres(5), "jnë")];
const P13B: &[End] = &[e(Pres(3), "më"), e(Pres(5), "në")];
const P13C: &[End] = &[e(Pres(3), "m"), e(Pres(5), "n")];
const P13D: &[End] = &[e(Pres(3), "im"), e(Pres(5), "in")];
// Lidhorja.
const SB2: &[End] = &[e(Subj(1), "sh")];
const SB3A: &[End] = &[ep(Subj(2), "jë", Vowel)];
const SB3B: &[End] = &[ep(Subj(2), "ë", Cons)];
// E pakryera.
const IPF_A: &[End] = &[e(Ipf(0), "ja"), e(Ipf(1), "je"), e(Ipf(2), "nte"), e(Ipf(3), "nim"), e(Ipf(4), "nit"), e(Ipf(5), "nin")];
const IPF_B: &[End] = &[e(Ipf(0), "ja"), e(Ipf(1), "je"), e(Ipf(2), "te"), e(Ipf(3), "nim"), e(Ipf(4), "nit"), e(Ipf(5), "nin")];
const IPF_C: &[End] = &[e(Ipf(0), "a"), e(Ipf(1), "e"), e(Ipf(2), "te"), e(Ipf(3), "im"), e(Ipf(4), "it"), e(Ipf(5), "in")];
// Aoristi.
const AOR12: &[End] = &[e(Aor(0), "a"), e(Aor(1), "e")];
/// «pashë, pa»: veta III njëjës është tema e zhveshur.
const AOR12B: &[End] = &[e(Aor(0), "shë"), e(Aor(1), ""), e(Aor(2), "")];
const AOR13: &[End] = &[e(Aor(0), "shë"), e(Aor(2), "")];
const AOR2: &[End] = &[es(Aor(1), "")];
const AOR3: &[End] = &[es(Aor(2), "")];
const AOR3A: &[End] = &[e(Aor(2), "i")];
const AOR3B: &[End] = &[e(Aor(2), "u")];
const AOR_PL_A: &[End] = &[e(Aor(3), "më"), e(Aor(4), "të"), e(Aor(5), "në")];
const AOR_PL_B: &[End] = &[e(Aor(3), "m"), e(Aor(4), "t"), e(Aor(5), "n")];
const AOR_PASS: &[End] = &[e(MidAor3, "")];
// Pjesorja.
const PT1: &[End] = &[ep(Ptcp, "rë", NoDiph)];
const PT2: &[End] = &[ep(Ptcp, "r", Diph)];
const PT3: &[End] = &[e(Ptcp, "ur")];
const PT4: &[End] = &[e(Ptcp, "ë")];
// Habitorja.
const ADM: &[End] = &[e(AdmPres(0), "kam"), e(AdmPres(1), "ke"), e(AdmPres(2), "ka"), e(AdmPres(3), "kemi"), e(AdmPres(4), "keni"), e(AdmPres(5), "kan")];
const ADM_IPF: &[End] = &[e(AdmIpf(0), "kësha"), e(AdmIpf(1), "këshe"), e(AdmIpf(2), "kësh"), e(AdmIpf(3), "këshim"), e(AdmIpf(4), "këshit"), e(AdmIpf(5), "këshin")];
// Dëshirorja.
const OPT_A: &[End] = &[ep(Opt(0), "sha", NoNasal), ep(Opt(1), "sh", NoNasal), ep(Opt(2), "të", NoNasal), ep(Opt(3), "shim", NoNasal), epa(Opt(4), "shi", NoNasal, "shit"), ep(Opt(5), "shin", NoNasal)];
const OPT_AN: &[End] = &[ep(Opt(0), "ça", Nasal), ep(Opt(1), "ç", Nasal), ep(Opt(2), "të", Nasal), ep(Opt(3), "çim", Nasal), epa(Opt(4), "çi", Nasal, "çit"), ep(Opt(5), "çin", Nasal)];
const OPT_B: &[End] = &[e(Opt(0), "ça"), e(Opt(1), "ç"), e(Opt(2), "të"), e(Opt(3), "çim"), ea(Opt(4), "çi", "çit"), e(Opt(5), "çin")];
const OPT_C: &[End] = &[e(Opt(0), "fsha"), e(Opt(1), "fsh"), e(Opt(2), "ftë"), e(Opt(3), "fshim"), ea(Opt(4), "fshi", "fshit"), e(Opt(5), "fshin")];
// Urdhërorja.
const IMP_A: &[End] = &[ep(Imp(0), "j", KeepJ)];
const IMP_BJ: &[End] = &[ep(Imp(0), "", DropJ)];
const IMP_B: &[End] = &[e(Imp(0), "")];
const IMP_PL: &[End] = &[e(Imp(1), "ni")];
// Joveprorja sintetike: mbaresat ngjiten te bazë me bashkëtingëllore (lexo-h-em, hap-em).
const MID: &[End] = &[ep(MidPres(0), "em", Cons), ep(MidPres(1), "esh", Cons), ep(MidPres(2), "et", Cons), ep(MidPres(3), "emi", Cons), ep(MidPres(4), "eni", Cons), ep(MidPres(5), "en", Cons)];
const MID_IPF: &[End] = &[ep(MidIpf(0), "esha", Cons), ep(MidIpf(1), "eshe", Cons), ep(MidIpf(2), "ej", Cons), ep(MidIpf(3), "eshim", Cons), ep(MidIpf(4), "eshit", Cons), ep(MidIpf(5), "eshin", Cons)];
const MID_IMP: &[End] = &[ep(MidImp(0), "u", Cons), ep(MidImp(1), "uni", Cons)];
// Trajtat e veçanta të disa foljeve.
const ECI_PRES: &[End] = &[e(Pres(0), "i"), e(Pres(1), "ën"), e(Pres(2), "ën")];
/// «eci», «hipi», «iki»: veta III e aoristit sipas fundit të temës (eci, hipi / iku).
const ECI_AOR3: &[End] = &[ep(Aor(2), "i", NoVelar), ep(Aor(2), "u", Velar)];
const THEM_0: &[End] = &[e(Pres(0), "m"), e(Pres(3), "mi"), e(Aor(1), "")];
const THEM_1: &[End] = &[e(Pres(1), "")];
const THEM_2: &[End] = &[e(Pres(2), "të"), e(Subj(2), "të"), e(Pres(4), "ni"), e(Pres(5), "në")];
const VETE_1: &[End] = &[e(Imp(0), "sh"), e(Pres(3), "mi"), e(Pres(4), "ni"), e(Pres(5), "në")];
/// «dua»: dëshirorja nga tema «da-» (daça, daç, daçim), veta III nga «dash-» (dashtë).
const OPT_DA: &[End] = &[e(Opt(0), "ça"), e(Opt(1), "ç"), e(Opt(3), "çim"), ea(Opt(4), "çi", "çit"), e(Opt(5), "çin")];
const OPT_DASH: &[End] = &[e(Opt(2), "të")];

/// Grupet e mbaresave të shkruara për një folje të vetme dhe të prejardhurat e saj.
const OWN_SETS: &[&[End]] = &[AOR12B, AOR13, ECI_PRES, ECI_AOR3, THEM_0, THEM_1, THEM_2, VETE_1, OPT_DA, OPT_DASH];
const OWN_CLASSES: &[&str] = &["shoh", "jap", "lë", "them", "eci", "vete", "dua"];

/// Klasat me mbaresa të posaçme të një foljeje dhe numri i këtyre mbaresave.
pub fn own() -> (Vec<&'static str>, usize) {
    (OWN_CLASSES.to_vec(), OWN_SETS.iter().map(|set| set.len()).sum())
}

/// Një grup i klasës: tema `stem` + ndajshtesa `infix` + mbaresat e nënparadigmave.
#[derive(Debug)]
pub struct Group {
    pub stem: u8,
    pub infix: &'static str,
    pub ends: &'static [&'static [End]],
}

const fn g(stem: u8, infix: &'static str, ends: &'static [&'static [End]]) -> Group {
    Group { stem, infix, ends }
}

/// Fundi i temës së parë që klasa kërkon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Vowel,
    Cons,
}

#[derive(Debug)]
pub struct Class {
    pub name: &'static str,
    pub kind: Kind,
    /// Klasë pa ndërrim teme: leksema me një temë të vetme e jep atë për çdo indeks.
    pub one_stem: bool,
    /// Urdhërorja e shumësit nga temë më vete («ejani»), jo e njëjtë me të tashmen.
    pub own_imp_pl: bool,
    pub groups: &'static [Group],
}

const fn c(name: &'static str, kind: Kind, groups: &'static [Group]) -> Class {
    Class { name, kind, one_stem: false, own_imp_pl: false, groups }
}

const fn c1(name: &'static str, kind: Kind, groups: &'static [Group]) -> Class {
    Class { name, kind, one_stem: true, own_imp_pl: false, groups }
}

use Kind::{Cons as C, Vowel as V};

/// Klasat. Rendi i grupeve është rendi i dëshmisë kur dy tema garojnë për një qelizë.
pub const CLASSES: &[Class] = &[
    c1("laj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, SB2, SB3A, IPF_A, IMP_A, IMP_BJ, IMP_PL, AOR3B, AOR_PL_A, AOR_PASS, PT1, ADM, ADM_IPF, OPT_C]),
        g(0, "h", &[MID_IMP, MID, MID_IPF]),
        g(0, "v", &[AOR12]),
    ]),
    c("blej", V, &[
        g(0, "", &[P1A, P23A, P13A, SB2, SB3A, AOR3B, AOR_PL_A, AOR_PASS, PT1, ADM, ADM_IPF, OPT_C]),
        g(0, "v", &[AOR12]),
        g(1, "", &[P2PL, IPF_A]),
        g(1, "h", &[MID, MID_IPF]),
        g(2, "", &[IMP_B, IMP_PL]),
        g(2, "h", &[MID_IMP]),
    ]),
    c("lexoj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, SB2, SB3A, IPF_A, AOR3A, OPT_C, IMP_B, IMP_PL]),
        g(0, "v", &[AOR12]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[AOR_PL_B, AOR_PASS, PT2, ADM, ADM_IPF]),
    ]),
    c("kthej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, AOR3B, SB2, SB3A, OPT_C, IMP_B, IMP_PL]),
        g(0, "v", &[AOR12]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[AOR_PL_B, AOR_PASS, PT1, PT2, ADM, ADM_IPF]),
    ]),
    c("shkruaj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, AOR_PL_B, AOR_PASS, PT1, PT2, SB2, SB3A, ADM, ADM_IPF, IMP_A, IMP_BJ, IMP_PL]),
        g(1, "h", &[MID, MID_IPF, MID_IMP]),
        g(2, "", &[OPT_C, AOR3A]),
        g(2, "v", &[AOR12]),
    ]),
    c("shkruej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, ADM, ADM_IPF, AOR_PL_B, AOR_PASS, IMP_A, IMP_BJ, IMP_PL]),
    ]),
    c("ndiej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, AOR_PL_B, AOR_PASS, PT1, PT2, SB2, SB3A, ADM, ADM_IPF, IMP_A, IMP_BJ, IMP_PL]),
        g(1, "h", &[MID, MID_IPF, MID_IMP]),
        g(2, "", &[OPT_C, AOR3B]),
        g(2, "v", &[AOR12]),
    ]),
    c("arrij", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_B, IMP_PL]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("dërsij", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_B, IMP_PL]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[PT3, ADM, ADM_IPF, AOR12, AOR3A, AOR_PASS, OPT_B]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("rroj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, AOR3A, OPT_C, IMP_B, IMP_PL]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(0, "v", &[AOR12]),
        g(1, "", &[AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("gjej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_A, IMP_BJ, IMP_PL]),
        g(0, "h", &[MID_IMP]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(2, "ë", &[AOR_PL_B]),
        g(1, "", &[MID, MID_IPF]),
    ]),
    c("brej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_A, IMP_BJ, IMP_PL]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("mej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_A, IMP_BJ, IMP_PL]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[AOR12, AOR3B, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("ruaj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_A, IMP_BJ, IMP_PL]),
        g(1, "h", &[MID, MID_IPF, MID_IMP]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
    c("ruej", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, IMP_A, IMP_BJ, IMP_PL]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS, PT4, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
    c("hyj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, SB2, SB3A, PT1, ADM, ADM_IPF, AOR_PL_A, AOR_PASS, OPT_C, IMP_PL]),
        g(0, "h", &[MID, MID_IPF]),
        g(1, "", &[AOR12, AOR3A, IMP_B]),
    ]),
    c("bëj", V, &[
        g(0, "", &[P1A, P23A, P2PL, P13A, IPF_A, AOR_PL_A, AOR_PASS, SB2, SB3A, PT1, ADM, ADM_IPF, OPT_C, IMP_A, IMP_BJ, IMP_PL]),
        g(0, "h", &[MID, MID_IPF, MID_IMP]),
        g(1, "", &[AOR12, AOR3A]),
    ]),
    Class {
        name: "vij",
        kind: V,
        one_stem: false,
        own_imp_pl: true,
        groups: &[
            g(0, "", &[P1A, P2PL, P13A, IPF_A, SB2, SB3A]),
            g(0, "h", &[MID, MID_IPF, MID_IMP]),
            g(1, "", &[P23A]),
            g(2, "", &[PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
            g(3, "", &[AOR12, AOR3A, AOR_PASS]),
            g(3, "ë", &[AOR_PL_B]),
            g(4, "", &[IMP_B, IMP_PL]),
        ],
    },
    c1("hap", C, &[
        g(0, "", &[P1B, P23B, P2PL, P13D, IPF_B, MID, MID_IPF, AOR12, AOR3A, AOR_PASS, SB3B, PT3, ADM, ADM_IPF, OPT_A, OPT_AN, MID_IMP, IMP_B, IMP_PL]),
        g(0, "ë", &[SB2]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c1("beh", C, &[
        g(0, "", &[P1B, P23B, P2PL, P13D, IPF_B, MID, MID_IPF, SB3B, PT3, ADM, ADM_IPF, AOR12, AOR3B, AOR_PASS, OPT_A, OPT_AN, IMP_B, IMP_PL, MID_IMP]),
        g(0, "ë", &[SB2]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("heq", C, &[
        g(0, "", &[P1B, P23B, P13D, SB3B, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL, MID_IMP]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
    c("vjel", C, &[
        g(0, "", &[P1B, P23B, P13D, SB3B, PT4, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(1, "", &[P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL, MID_IMP]),
        g(0, "ë", &[SB2]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
    c("dal", C, &[
        g(0, "", &[P1B, P13D, SB3B, PT4, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B, IMP_B, IMP_PL, MID, MID_IPF]),
        g(3, "", &[AOR12, AOR3A, AOR_PASS]),
        g(3, "ë", &[AOR_PL_B]),
    ]),
    c("prier", C, &[
        g(0, "", &[P1B, P23B, P2PL, P13D, IPF_B, SB3B, IMP_B, IMP_PL, MID_IMP]),
        g(0, "ë", &[SB2]),
        g(1, "", &[MID, MID_IPF, PT4, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
    c("fsheh", C, &[
        g(0, "", &[P1B, P23B, P13D, AOR12, AOR3B, AOR_PASS, SB3B, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(0, "ë", &[AOR_PL_B, SB2]),
        g(1, "", &[P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL, MID_IMP]),
    ]),
    c("zhvesh", C, &[
        g(0, "", &[P1B, P23B, P13D, AOR12, AOR3A, AOR_PASS, SB3B, PT3, ADM, ADM_IPF, OPT_B]),
        g(0, "ë", &[AOR_PL_B, SB2]),
        g(1, "", &[P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL, MID_IMP]),
    ]),
    c("njoh", C, &[
        g(0, "", &[P1B, P13D, AOR12, AOR3B, AOR_PASS, SB3B, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(0, "ë", &[AOR_PL_B, SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL, MID_IMP]),
    ]),
    c("mat", C, &[
        g(0, "", &[P1B, P13D, P2PL, IPF_B, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B, MID, MID_IPF, AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN, IMP_B, IMP_PL, MID_IMP]),
        g(1, "ë", &[AOR_PL_B]),
    ]),
    c("pres-wait", C, &[
        g(0, "", &[P1B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B, IMP_PL]),
        g(3, "", &[MID, MID_IPF, AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN, IMP_B, MID_IMP]),
        g(3, "ë", &[AOR_PL_B]),
    ]),
    c("vdes", C, &[
        g(0, "", &[P1B, P23B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P2PL, IPF_B, IMP_B, IMP_PL, MID, MID_IPF]),
        g(2, "", &[PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(3, "", &[AOR12, AOR3, AOR_PASS]),
        g(3, "ë", &[AOR_PL_B]),
    ]),
    c("marr", C, &[
        g(0, "", &[P1B, P13D, SB3B, PT4, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B, P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL, MID_IMP]),
        g(3, "", &[AOR12, AOR3A, AOR_PASS]),
        g(3, "ë", &[AOR_PL_B]),
    ]),
    c("humbas", C, &[
        g(0, "", &[P1B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B, IMP_PL]),
        g(3, "", &[MID, MID_IPF, AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN, MID_IMP]),
        g(3, "ë", &[AOR_PL_B]),
    ]),
    c("flas", C, &[
        g(0, "", &[P1B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B]),
        g(3, "", &[MID, MID_IPF, IMP_PL]),
        g(4, "", &[AOR12, AOR3A, AOR_PASS, PT3, OPT_A, OPT_AN, ADM, ADM_IPF, IMP_B]),
        g(4, "ë", &[AOR_PL_B]),
    ]),
    c("vras", C, &[
        g(0, "", &[P1B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B]),
        g(3, "", &[AOR3B, AOR_PL_A, AOR_PASS, PT1, ADM, ADM_IPF, OPT_C, OPT_B]),
        g(3, "v", &[AOR12]),
        g(4, "", &[MID, MID_IPF, IMP_B, IMP_PL]),
    ]),
    c("dhjes", C, &[
        g(0, "", &[P1B, P23B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P2PL, IPF_B, IMP_PL]),
        g(2, "", &[AOR3B, AOR_PL_A, AOR_PASS, PT1, ADM, ADM_IPF, OPT_C]),
        g(2, "v", &[AOR12]),
        g(3, "", &[MID, MID_IPF, IMP_B]),
    ]),
    c("shoh", C, &[
        g(0, "", &[P1B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B]),
        g(2, "", &[P2PL, IPF_B, MID, MID_IPF, IMP_B, IMP_PL]),
        g(3, "", &[AOR12B, AOR_PL_A, AOR_PASS, PT1, ADM, ADM_IPF, OPT_C]),
        g(4, "", &[AOR2]),
    ]),
    c("jap", C, &[
        g(0, "", &[P1B, P13D, SB3B]),
        g(0, "ë", &[SB2]),
        g(1, "", &[P23B, P2PL, MID, IPF_B, MID_IPF, IMP_B, IMP_PL]),
        g(3, "", &[PT4, ADM, ADM_IPF, OPT_B]),
        g(4, "", &[AOR12B, AOR_PL_A, AOR_PASS, OPT_C]),
        g(5, "", &[AOR2]),
    ]),
    c1("pi", V, &[
        g(0, "", &[P1B, P23B, P2PL, P13B, IPF_A, AOR3B, AOR_PL_A, AOR_PASS, SB2, SB3A, PT1, ADM, ADM_IPF, OPT_C, IMP_B, IMP_PL]),
        g(0, "v", &[AOR12]),
        g(0, "h", &[MID, MID_IPF]),
    ]),
    c("di", V, &[
        g(0, "", &[P1B, P23B, P13B, SB2, SB3A]),
        g(1, "", &[P2PL, IPF_A, IMP_B, IMP_PL]),
        g(1, "h", &[MID, MID_IPF, MID_IMP]),
        g(2, "", &[AOR12, AOR3A, AOR_PASS, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
    c("shtie", V, &[
        g(0, "", &[P1B, P23B, P13B, SB2]),
        g(1, "", &[P2PL, IPF_A, AOR_PASS, AOR3B, AOR_PL_A, IMP_PL]),
        g(1, "h", &[MID, MID_IPF]),
        g(1, "v", &[AOR12]),
        g(2, "", &[PT4, ADM, ADM_IPF, OPT_B]),
        g(3, "", &[SB3B, IMP_B]),
        g(3, "h", &[MID_IMP]),
    ]),
    c("zë", V, &[
        g(0, "", &[P1B, P23B, P13B, SB2]),
        g(1, "", &[P2PL, IPF_A, IMP_PL]),
        g(1, "h", &[MID, MID_IPF]),
        g(2, "", &[PT4, ADM, ADM_IPF, OPT_B]),
        g(3, "", &[AOR12, AOR3A]),
        g(4, "", &[AOR_PL_A, AOR_PASS]),
        g(5, "", &[SB3B, IMP_B]),
    ]),
    c("lë", V, &[
        g(0, "", &[P1B, P23B, P13B, SB2]),
        g(1, "", &[P2PL, IPF_A, IMP_PL]),
        g(1, "h", &[MID, MID_IPF, MID_IMP]),
        g(2, "", &[PT4, ADM, ADM_IPF, OPT_B]),
        g(3, "", &[AOR12B, AOR_PL_A, AOR_PASS, PT1]),
        g(4, "", &[AOR2]),
        g(5, "", &[SB3B, IMP_B]),
    ]),
    c("ha", V, &[
        g(0, "", &[P1B, P23B, P2PL, P13B, IPF_A, SB2, SB3A, IMP_B, IMP_PL]),
        g(0, "h", &[MID, MID_IPF]),
        g(1, "", &[PT4, ADM, ADM_IPF, OPT_B]),
        g(2, "", &[AOR12, AOR3A]),
        g(2, "ë", &[AOR_PL_B]),
        g(3, "", &[AOR_PASS, OPT_A, OPT_AN]),
    ]),
    c("dua", V, &[
        g(0, "", &[P1B, P13C, SB2, IMP_A, IMP_BJ]),
        g(1, "", &[P23B, P2PL, IPF_A, SB3A, IMP_PL]),
        g(2, "h", &[MID, MID_IPF, MID_IMP]),
        g(3, "", &[PT3, ADM, ADM_IPF, OPT_DASH]),
        g(4, "sh", &[AOR12, AOR3A, AOR_PASS]),
        g(4, "shë", &[AOR_PL_B]),
        g(7, "", &[OPT_DA]),
    ]),
    c("due", V, &[
        g(0, "", &[P1B, P13C, SB2, IMP_A, IMP_BJ, IMP_PL]),
    ]),
    c("bie", V, &[
        g(0, "", &[P1B, P23B, P13C, SB2]),
        g(1, "", &[P2PL, IPF_A, IMP_PL]),
        g(1, "h", &[MID, MID_IPF]),
        g(2, "", &[AOR_PL_A, AOR_PASS, PT1, ADM, ADM_IPF, OPT_C]),
        g(3, "", &[AOR12, AOR3A]),
        g(4, "", &[SB3B, IMP_B]),
    ]),
    c1("eci", C, &[
        g(0, "", &[ECI_PRES, P2PL, P13D, IPF_B, AOR12, ECI_AOR3, AOR_PASS, SB3B, PT3, ADM, ADM_IPF, OPT_A, OPT_AN, IMP_B, IMP_PL]),
        g(0, "ë", &[AOR_PL_B, SB2]),
    ]),
    c("them", V, &[
        g(0, "", &[THEM_0]),
        g(1, "", &[THEM_1, SB2, IMP_A, IMP_BJ]),
        g(2, "", &[THEM_2, IMP_PL]),
        g(3, "", &[PT4, ADM, ADM_IPF, OPT_B]),
        g(4, "", &[IPF_C]),
        g(5, "", &[AOR13, AOR_PL_A, AOR_PASS]),
        g(6, "h", &[MID, MID_IPF]),
    ]),
    c("vete", V, &[
        g(0, "", &[P1B, P23B]),
        g(1, "", &[VETE_1, IPF_A, SB2, IMP_PL]),
        g(2, "", &[AOR12, AOR3A, PT3, ADM, ADM_IPF, OPT_A, OPT_AN]),
        g(2, "ë", &[AOR_PL_B]),
    ]),
];

/// Klasa sipas emrit të paradigmës; «-1» (leksema joveprore) ndan klasën bazë.
pub fn find(name: &str) -> Option<&'static Class> {
    let base = name.strip_suffix("-1").unwrap_or(name);
    CLASSES.iter().find(|c| c.name == base)
}

const VOWELS: &[char] = &['a', 'e', 'ë', 'i', 'o', 'u', 'y'];

pub fn is_vowel(ch: char) -> bool {
    VOWELS.contains(&ch)
}

pub fn ends_in_vowel(s: &str) -> bool {
    s.chars().last().is_some_and(is_vowel)
}

/// Hiati: në klasat me bashkëtingëllore tema me zanore në fund s'merr mbaresë që nis
/// me zanore (*gërga-a, *gërga-ur); përjashtim «-u» e aoristit, që lidh diftong (vra-u, pre-u).
pub fn hiatus(stem: &str, infix: &str, ending: &str) -> bool {
    let opens = infix.chars().chain(ending.chars()).next().is_some_and(is_vowel);
    let aorist_u = infix.is_empty() && ending == "u";
    ends_in_vowel(stem) && opens && !aorist_u
}

/// Skeleti bashkëtingëllor: shkronjat jo-zanore, me radhë.
pub fn skeleton(s: &str) -> String {
    s.chars().filter(|ch| !is_vowel(*ch)).collect()
}

/// A e ruan tema skeletin bashkëtingëllor `head` në fillim.
pub fn keeps_head(stem: &str, head: &str) -> bool {
    let mut cons = stem.chars().filter(|ch| !is_vowel(*ch));
    head.chars().all(|h| cons.next() == Some(h))
}

/// Koka e temës: deri në fund të grupit zanor të parafundit («thë-» te «thërras»,
/// «pu-» te «puno»). Ndërrimet e temës prekin zanoret dhe rrokjen e fundit, kurrë
/// bashkëtingëlloret e kokës (thërras/thirr, pjek/piq/poq).
pub fn head(s: &str) -> &str {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let inner = chars.windows(2).filter_map(|w| match w {
        [(_, a), (i, b)] => (is_vowel(*a) && !is_vowel(*b)).then_some(*i),
        _ => None,
    });
    let ends: Vec<usize> = inner.chain(ends_in_vowel(s).then_some(s.len())).collect();
    let cut = ends.len().checked_sub(2).and_then(|k| ends.get(k)).copied().unwrap_or(0);
    s.get(..cut).unwrap_or("")
}

pub fn ends_in_diphthong(s: &str) -> bool {
    ["ua", "ue", "ye", "ie"].iter().any(|d| s.ends_with(d))
}

/// A e plotëson baza kushtin fonologjik të mbaresës.
pub fn fits(base: &str, phon: Phon) -> bool {
    match phon {
        Phon::Any => true,
        Phon::Vowel => ends_in_vowel(base),
        Phon::Cons => !base.is_empty() && !ends_in_vowel(base),
        Phon::Diph => ends_in_diphthong(base),
        Phon::NoDiph => !ends_in_diphthong(base),
        Phon::KeepJ => ends_in_vowel(base) && !drops_j(base),
        Phon::DropJ => drops_j(base),
        Phon::Nasal => base.ends_with('n') || base.ends_with("nj"),
        Phon::NoNasal => !(base.ends_with('n') || base.ends_with("nj")),
        Phon::Velar => velar(base),
        Phon::NoVelar => !velar(base),
    }
}

/// Fytorja në fund: «k», «g», ose «h» që s'është pjesë e «sh», «zh», «th», «dh», «xh».
fn velar(base: &str) -> bool {
    let h = base.ends_with('h') && !["sh", "zh", "th", "dh", "xh"].iter().any(|d| base.ends_with(d));
    base.ends_with('k') || base.ends_with('g') || h
}

fn drops_j(base: &str) -> bool {
    base.ends_with(['i', 'y']) || base.ends_with("ye") || base.ends_with("ie")
}
