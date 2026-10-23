//! Foljet ndihmëse «jam» dhe «kam»: klasë e mbyllur, tabelë e plotë.
//! Prej tyre ndërtohen të gjitha kohët e përbëra; temat e tyre (je-, ish-,
//! qe-, qen-, qof-; ke-, kish-, pat-, pas-, paç-) nuk dalin nga asnjë rregull.

use super::klasat::Slot;

#[derive(Debug)]
pub struct Aux {
    pub lemma: &'static str,
    pub pres: [&'static str; 6],
    pub subj: [&'static str; 6],
    pub ipf: [&'static str; 6],
    pub aor: [&'static str; 6],
    pub imp: [&'static str; 2],
    pub adm: [&'static str; 6],
    pub adm_ipf: [&'static str; 6],
    pub opt: [&'static str; 6],
    /// Varianti jostandard i vetës II shumës së dëshirores («qofshit», «paçit»):
    /// pranohet për njohje, kurrë si trajtë kryesore.
    pub opt_2pl_alt: &'static str,
    pub ptcp: &'static str,
}

pub const JAM: Aux = Aux {
    lemma: "jam",
    pres: ["jam", "je", "është", "jemi", "jeni", "janë"],
    subj: ["jem", "jesh", "jetë", "jemi", "jeni", "jenë"],
    ipf: ["isha", "ishe", "ishte", "ishim", "ishit", "ishin"],
    aor: ["qeshë", "qe", "qe", "qemë", "qetë", "qenë"],
    imp: ["ji", "jini"],
    adm: ["qenkam", "qenke", "qenka", "qenkemi", "qenkeni", "qenkan"],
    adm_ipf: ["qenkësha", "qenkëshe", "qenkësh", "qenkëshim", "qenkëshit", "qenkëshin"],
    opt: ["qofsha", "qofsh", "qoftë", "qofshim", "qofshi", "qofshin"],
    opt_2pl_alt: "qofshit",
    ptcp: "qenë",
};

pub const KAM: Aux = Aux {
    lemma: "kam",
    pres: ["kam", "ke", "ka", "kemi", "keni", "kanë"],
    subj: ["kem", "kesh", "ketë", "kemi", "keni", "kenë"],
    ipf: ["kisha", "kishe", "kishte", "kishim", "kishit", "kishin"],
    aor: ["pata", "pate", "pati", "patëm", "patët", "patën"],
    imp: ["ki", "kini"],
    adm: ["paskam", "paske", "paska", "paskemi", "paskeni", "paskan"],
    adm_ipf: ["paskësha", "paskëshe", "paskësh", "paskëshim", "paskëshit", "paskëshin"],
    opt: ["paça", "paç", "pastë", "paçim", "paçi", "paçin"],
    opt_2pl_alt: "paçit",
    ptcp: "pasur",
};

const ALL: [&Aux; 2] = [&JAM, &KAM];

/// Lemat e tabelës së parregullt.
pub fn lemmas() -> Vec<&'static str> {
    ALL.iter().map(|a| a.lemma).collect()
}

/// Numri i trajtave sintetike në tabelë, me variantet dhe pjesoren.
pub fn size() -> usize {
    ALL.iter()
        .map(|a| a.pres.len() + a.subj.len() + a.ipf.len() + a.aor.len() + a.imp.len() + a.adm.len() + a.adm_ipf.len() + a.opt.len() + 2)
        .sum()
}

pub fn table(lemma: &str) -> Option<&'static Aux> {
    ALL.iter().find(|a| a.lemma == lemma).copied()
}

/// Trajta e tabelës për një qelizë sintetike: (trajta, varianti).
pub fn cell(a: &Aux, slot: Slot) -> Option<(&'static str, Option<&'static str>)> {
    let at = |row: &[&'static str; 6], p: u8| row.get(p as usize).copied();
    match slot {
        Slot::Pres(p) => at(&a.pres, p).map(|t| (t, None)),
        Slot::Subj(p) => at(&a.subj, p).map(|t| (t, None)),
        Slot::Ipf(p) => at(&a.ipf, p).map(|t| (t, None)),
        Slot::Aor(p) => at(&a.aor, p).map(|t| (t, None)),
        Slot::Imp(n) => a.imp.get(n as usize).copied().map(|t| (t, None)),
        Slot::AdmPres(p) => at(&a.adm, p).map(|t| (t, None)),
        Slot::AdmIpf(p) => at(&a.adm_ipf, p).map(|t| (t, None)),
        Slot::Opt(p) => at(&a.opt, p).map(|t| (t, (p == 4).then_some(a.opt_2pl_alt))),
        Slot::Ptcp => Some((a.ptcp, None)),
        Slot::MidPres(_) | Slot::MidIpf(_) | Slot::MidAor3 | Slot::MidImp(_) => None,
    }
}
