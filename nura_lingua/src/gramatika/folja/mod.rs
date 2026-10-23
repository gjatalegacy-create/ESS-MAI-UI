//! Zgjedhimi i foljes shqipe: mënyra, koha, veta, numri, zëri.
//!
//! Dëshmitari leksikor: klasa e paradigmës dhe temat e leksemës nga
//! uniparser-grammar-albanian 2.3.0, MIT (Morozova, Rusakov, Arkhangelskiy).
//! Rendi i dëshmisë, i fiksuar:
//! 1. «jam» dhe «kam»: tabelë e mbyllur, edhe si ndihmëse të kohëve të përbëra;
//! 2. trajta sintetike: tema e klasës + mbaresa e nënparadigmës; kur garojnë dy
//!    mbaresa, vendos fundi i bazës (zanore, bashkëtingëllore, diftong), pastaj
//!    mbaresa e posaçme, pastaj rendi i grupeve të klasës;
//! 3. trajta analitike: ndihmëse + pjesore, «do të», «të», «u»;
//! 4. kontrolli i lemës: lema duhet të dalë nga vetë klasa dhe temat (veta I ose
//!    III e së tashmes, veprore ose joveprore), ndryshe asnjë trajtë;
//! 5. joveprorja jepet vetëm me dëshmi: leksema joveprore «-1» e gramatikës ose
//!    vetë lema joveprore («lahem»).
//!
//! Kur një hap nuk është i sigurt, kthehet `None`: asnjë trajtë me hamendje.

mod klasat;
mod ndihmese;

use crate::gramatika::leksiku::{Lexeme, Lexicon};
use crate::gramatika::tipet::{Form, Mood, NonFinite, Number, Person, Pos, Tense, VerbCell, Voice};
use klasat::{Class, Group, Kind, Slot};
use ndihmese::{Aux, JAM, KAM};
use std::collections::{BTreeSet, HashMap};

pub const R_TABLE: &str = "F0 folje ndihmëse e parregullt: tabelë e mbyllur";
pub const R_SYN: &str = "F1 trajtë sintetike: tema e klasës + mbaresa";
pub const R_IMP: &str = "F2 urdhërore: tema e klasës + mbaresa e urdhërores";
pub const R_PERF: &str = "F3 e kryer: kam + pjesore";
pub const R_PLUPERF: &str = "F4 më se e kryer: kisha + pjesore";
pub const R_ANTERIOR: &str = "F5 e kryer e tejshkuar: pata + pjesore";
pub const R_FUT: &str = "F6 e ardhme: do të + e tashme e lidhores";
pub const R_FUT_PERF: &str = "F7 e ardhme e përparme: do të + kem/jem + pjesore";
pub const R_COND: &str = "F8 kushtore / e ardhme e së shkuarës: do të + e pakryer";
pub const R_COND_PERF: &str = "F9 kushtore e kryer: do të + kisha/isha + pjesore";
pub const R_SUBJ: &str = "F10 lidhore: të + trajtë e thjeshtë";
pub const R_SUBJ_PERF: &str = "F11 lidhore e kryer: të + kem/kisha + pjesore";
pub const R_ADM_PERF: &str = "F12 habitore e kryer: paskam/paskësha + pjesore";
pub const R_OPT_PERF: &str = "F13 dëshirore e kryer: paça + pjesore";
pub const R_MID: &str = "F14 joveprore sintetike: tema + -em/-esha/-u";
pub const R_MID_U: &str = "F15 joveprore me «u»: u + aorist, habitore ose dëshirore";
pub const R_MID_AUX: &str = "F16 joveprore analitike: jam/isha/qeshë/qenkam/qofsha + pjesore";
pub const R_PTCP: &str = "F17 pjesore: tema e klasës + -rë/-r/-ur/-ë";
pub const R_GERUND: &str = "F18 përcjellore: duke + pjesore";
pub const R_INF: &str = "F19 paskajore: për të + pjesore (gegërisht: me + pjesore)";
pub const R_PRIV: &str = "F20 mohore: pa + pjesore";
pub const R_ABS: &str = "F21 trajtë kohore: me të + pjesore";

/// Trajtë e ndërmjetme: teksti dhe variantet e pranuara.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Syn {
    text: String,
    alts: Vec<String>,
}

impl Syn {
    fn form(self, rule: &'static str) -> Form {
        Form { text: self.text, variants: self.alts, rule }
    }

    /// Fjalë e pandryshueshme përpara: «do të», «të», «u», «duke».
    fn after(self, words: &str) -> Syn {
        Syn { text: format!("{words} {}", self.text), alts: self.alts.iter().map(|a| format!("{words} {a}")).collect() }
    }
}

/// Ndihmëse + pjesore; variantet e secilës pjesë ruhen.
fn join(aux: Syn, ptcp: &Syn) -> Syn {
    let text = format!("{} {}", aux.text, ptcp.text);
    let alts = aux.alts.iter().map(|a| format!("{a} {}", ptcp.text)).chain(ptcp.alts.iter().map(|p| format!("{} {p}", aux.text))).collect();
    Syn { text, alts }
}

fn table_syn(a: &Aux, slot: Slot) -> Option<Syn> {
    ndihmese::cell(a, slot).map(|(t, alt)| Syn { text: t.to_string(), alts: alt.into_iter().map(str::to_string).collect() })
}

/// Leksema e një klase: temat, lema dhe dy fakte që dalin nga tema e parë.
struct Stems<'a> {
    class: &'static Class,
    stems: Vec<Vec<String>>,
    lemma: &'a str,
    /// Skeleti bashkëtingëllor i kokës së temës së parë.
    head: String,
    /// Lema ka temën në «-s» me alternant në «-t» (pyes/pyet, blludis/blludit).
    s_lemma: bool,
}

enum Base<'a> {
    Table(&'static Aux),
    Class(Stems<'a>),
}

/// Folja e gatshme për zgjedhim.
struct Verb<'a> {
    base: Base<'a>,
    /// Lema është trajtë joveprore («aftosem», «ajkoset»): s'ka trajta veprore.
    /// Klasa «-1» (leksema joveprore e një leme veprore) jep edhe veprore, sepse
    /// vetë lema dëshmon foljen veprore me të njëjtat tema.
    mid_only: bool,
    /// Leksema dëshmon joveproren: klasa «-1» e gramatikës ose lema joveprore.
    mid_witness: bool,
}

struct Cand {
    text: String,
    alt: Option<String>,
    fits: bool,
    strong: bool,
}

/// Variantet e temës `n`. Leksema me një temë të vetme e jep atë për çdo indeks
/// vetëm në klasat pa ndërrim teme (hap, laj); te të tjerat tema që mungon s'merret me hamendje.
fn stem_vars<'s>(class: &Class, stems: &'s [Vec<String>], n: u8) -> Option<&'s Vec<String>> {
    stems.get(n as usize).or_else(|| match (class.one_stem, stems.len()) {
        (true, 1) => stems.first(),
        _ => None,
    })
}

/// Variantet e përdorshme të temës `n`: pa hiat me asnjë mbaresë të grupeve të saj
/// dhe me skeletin e kokës së temës së parë të paprekur (gabimi «duarktoket» bie jashtë).
fn usable(st: &Stems, vars: &[String], n: u8) -> Vec<String> {
    let joins = |v: &str| {
        let open = st.class.kind == Kind::Vowel || !klasat::ends_in_vowel(v);
        open || st.class.groups.iter().filter(|g| g.stem == n).all(|g| g.ends.iter().flat_map(|set| set.iter()).all(|e| !klasat::hiatus(v, g.infix, e.text)))
    };
    vars.iter().filter(|v| joins(v) && klasat::keeps_head(v, &st.head)).cloned().collect()
}

/// Varianti i temës para një mbarese. Tema e parë merr variantin që gjendet në
/// vetë lemën («mbështes//mbështet» → mbështet). Te temat e tjera, ndërrimi
/// «-t»/«-s» (prit-//pris-, flit-//flis-) është leksikor: para mbaresës që nis me
/// «t» vendos disimilimi (priste, fliste, vriste), para të tjerave asnjë variant
/// (prisni/vritni s'dalin nga rregulli). Variantet e tjera: i pari i gramatikës.
fn pick<'s>(vars: &'s [String], n: u8, lemma: &str, ending: &str) -> Option<&'s String> {
    let by_lemma = match n {
        0 => vars.iter().find(|v| lemma.starts_with(v.as_str())),
        _ => None,
    };
    let first = by_lemma.or_else(|| vars.first())?;
    let s_alternant = first
        .strip_suffix('t')
        .filter(|_| n > 0)
        .and_then(|r| vars.iter().find(|v| v.strip_suffix('s') == Some(r)));
    match (s_alternant, ending.starts_with('t')) {
        (None, _) => Some(first),
        (Some(s), true) => Some(s),
        (Some(_), false) => None,
    }
}

/// Vetat që ruajnë temën e lemës te foljet «-s»/«-t»: I njëjës, I e III shumës,
/// lidhorja (pyes, pyesim, pyesin, të pyesë). Të tjerat (pyetni, pyetja) s'dalin nga lema.
fn lemma_slot(slot: Slot) -> bool {
    matches!(slot, Slot::Pres(0) | Slot::Pres(3) | Slot::Pres(5) | Slot::Subj(_))
}

/// Trajta sintetike e klasës: kandidatët e të gjitha grupeve për qelizën,
/// pastaj kushti fonologjik, pastaj mbaresa e posaçme, pastaj rendi.
fn class_syn(st: &Stems, slot: Slot) -> Option<Syn> {
    let groups: Vec<&Group> = st.class.groups.iter().filter(|g| g.ends.iter().flat_map(|set| set.iter()).any(|e| e.slot == slot)).collect();
    let blocked = st.s_lemma && !lemma_slot(slot) && groups.iter().any(|g| g.stem == 0);
    (!blocked).then_some(())?;
    let mut cands: Vec<Cand> = Vec::new();
    for grp in groups {
        let vars = stem_vars(st.class, &st.stems, grp.stem).map(|v| usable(st, v, grp.stem)).unwrap_or_default();
        for end in grp.ends.iter().flat_map(|set| set.iter()).filter(|end| end.slot == slot) {
            let ending = format!("{}{}", grp.infix, end.text);
            let base = pick(&vars, grp.stem, st.lemma, &ending).map(|s| format!("{s}{}", grp.infix));
            cands.extend(base.map(|b| Cand {
                text: format!("{b}{}", end.text),
                alt: (!end.alt.is_empty()).then(|| format!("{b}{}", end.alt)),
                fits: klasat::fits(&b, end.phon),
                strong: end.strong,
            }));
        }
    }
    let any_fits = cands.iter().any(|c| c.fits);
    let pool: Vec<&Cand> = cands.iter().filter(|c| c.fits || !any_fits).collect();
    let strong = pool.iter().any(|c| c.strong);
    pool.into_iter().find(|c| c.strong == strong).map(|c| Syn { text: c.text.clone(), alts: c.alt.iter().cloned().collect() })
}

impl<'a> Verb<'a> {
    fn of(lex: &'a Lexeme) -> Option<Verb<'a>> {
        (lex.pos == Pos::Verb).then_some(())?;
        let name = lex.paradigms.first()?;
        match ndihmese::table(&lex.lemma).filter(|a| a.lemma == name.as_str()) {
            Some(a) => Some(Verb { base: Base::Table(a), mid_only: false, mid_witness: false }),
            None => Verb::of_class(lex, name),
        }
    }

    fn of_class(lex: &'a Lexeme, name: &str) -> Option<Verb<'a>> {
        let class = klasat::find(name)?;
        let stems = lex.clean_stems();
        let vars0 = stems.first()?;
        let first = vars0.iter().find(|v| lex.lemma.starts_with(v.as_str())).or_else(|| vars0.first())?;
        let kind_ok = match class.kind {
            Kind::Vowel => klasat::ends_in_vowel(first),
            Kind::Cons => !klasat::ends_in_vowel(first),
        };
        kind_ok.then_some(())?;
        let head = klasat::skeleton(klasat::head(first));
        let s_lemma = first.strip_suffix('s').is_some_and(|r| vars0.iter().chain(stems.get(1).into_iter().flatten()).any(|v| v.strip_suffix('t') == Some(r)));
        let st = Stems { class, stems: stems.clone(), lemma: &lex.lemma, head, s_lemma };
        let probe = Verb { base: Base::Class(st), mid_only: false, mid_witness: true };
        let gives = |slots: [Slot; 2]| slots.iter().any(|s| probe.syn(*s).is_some_and(|f| f.text == lex.lemma));
        let active_lemma = gives([Slot::Pres(0), Slot::Pres(2)]);
        let middle_lemma = gives([Slot::MidPres(0), Slot::MidPres(2)]);
        (active_lemma || middle_lemma).then_some(())?;
        Some(Verb { base: probe.base, mid_only: !active_lemma, mid_witness: name.ends_with("-1") || !active_lemma })
    }

    fn syn(&self, slot: Slot) -> Option<Syn> {
        match &self.base {
            Base::Table(a) => table_syn(a, slot),
            Base::Class(st) => class_syn(st, slot),
        }
    }

    fn rule(&self, class_rule: &'static str) -> &'static str {
        match self.base {
            Base::Table(_) => R_TABLE,
            Base::Class(_) => class_rule,
        }
    }

    fn has_mid(&self) -> bool {
        self.syn(Slot::MidPres(2)).is_some()
    }

    fn own_imp_pl(&self) -> bool {
        match &self.base {
            Base::Table(_) => true,
            Base::Class(st) => st.class.own_imp_pl,
        }
    }
}

fn pn(person: Person, number: Number) -> u8 {
    let p = match person {
        Person::P1 => 0,
        Person::P2 => 1,
        Person::P3 => 2,
    };
    match number {
        Number::Sing => p,
        Number::Plur => p + 3,
    }
}

/// Lidhorja e tashme: veta II e III njëjës kanë trajtë të vetën, të tjerat = dëftorja.
fn subjunctive(v: &Verb, p: u8) -> Option<Syn> {
    v.syn(Slot::Subj(p)).or_else(|| match p {
        1 | 2 => None,
        _ => v.syn(Slot::Pres(p)),
    })
}

/// Urdhërorja ka vetëm vetën II.
fn imperative_slot(p: u8, mid: bool) -> Option<Slot> {
    match (p, mid) {
        (1, false) => Some(Slot::Imp(0)),
        (4, false) => Some(Slot::Imp(1)),
        (1, true) => Some(Slot::MidImp(0)),
        (4, true) => Some(Slot::MidImp(1)),
        _ => None,
    }
}

/// Urdhërorja veprore e shumësit = e tashmja e vetës II shumës («punoni», «hapni»);
/// kur temat e klasës japin dy trajta të ndryshme, asnjë. Përjashtim urdhërorja
/// supletive («ejani», «jini», «kini»).
fn imperative(v: &Verb, p: u8) -> Option<Syn> {
    let imp = imperative_slot(p, false).and_then(|s| v.syn(s))?;
    let same = v.syn(Slot::Pres(p)).is_some_and(|pres| pres.text == imp.text);
    (p != 4 || same || v.own_imp_pl()).then_some(imp)
}

fn active(v: &Verb, c: VerbCell, p: u8) -> Option<Form> {
    let syn = |s: Slot| v.syn(s).map(|f| f.form(v.rule(R_SYN)));
    let with = |a: &Aux, s: Slot| Some(join(table_syn(a, s)?, &v.syn(Slot::Ptcp)?));
    match (c.mood, c.tense) {
        (Mood::Ind, Tense::Pres) => syn(Slot::Pres(p)),
        (Mood::Ind, Tense::Imperf) => syn(Slot::Ipf(p)),
        (Mood::Ind, Tense::Aor) => syn(Slot::Aor(p)),
        (Mood::Ind, Tense::Perf) => with(&KAM, Slot::Pres(p)).map(|s| s.form(R_PERF)),
        (Mood::Ind, Tense::Pluperf) => with(&KAM, Slot::Ipf(p)).map(|s| s.form(R_PLUPERF)),
        (Mood::Ind, Tense::PastAnterior) => with(&KAM, Slot::Aor(p)).map(|s| s.form(R_ANTERIOR)),
        (Mood::Ind, Tense::Fut) => subjunctive(v, p).map(|s| s.after("do të").form(R_FUT)),
        (Mood::Ind, Tense::FutPerf) => with(&KAM, Slot::Subj(p)).map(|s| s.after("do të").form(R_FUT_PERF)),
        (Mood::Ind, Tense::FutPast) | (Mood::Cnd, Tense::Pres) | (Mood::Cnd, Tense::Imperf) => v.syn(Slot::Ipf(p)).map(|s| s.after("do të").form(R_COND)),
        (Mood::Ind, Tense::FutPastPerf) | (Mood::Cnd, Tense::Perf) | (Mood::Cnd, Tense::Pluperf) => with(&KAM, Slot::Ipf(p)).map(|s| s.after("do të").form(R_COND_PERF)),
        (Mood::Sub, Tense::Pres) => subjunctive(v, p).map(|s| s.after("të").form(R_SUBJ)),
        (Mood::Sub, Tense::Imperf) => v.syn(Slot::Ipf(p)).map(|s| s.after("të").form(R_SUBJ)),
        (Mood::Sub, Tense::Perf) => with(&KAM, Slot::Subj(p)).map(|s| s.after("të").form(R_SUBJ_PERF)),
        (Mood::Sub, Tense::Pluperf) => with(&KAM, Slot::Ipf(p)).map(|s| s.after("të").form(R_SUBJ_PERF)),
        (Mood::Adm, Tense::Pres) => syn(Slot::AdmPres(p)),
        (Mood::Adm, Tense::Imperf) => syn(Slot::AdmIpf(p)),
        (Mood::Adm, Tense::Perf) => with(&KAM, Slot::AdmPres(p)).map(|s| s.form(R_ADM_PERF)),
        (Mood::Adm, Tense::Pluperf) => with(&KAM, Slot::AdmIpf(p)).map(|s| s.form(R_ADM_PERF)),
        (Mood::Opt, Tense::Pres) => syn(Slot::Opt(p)),
        (Mood::Opt, Tense::Perf) => with(&KAM, Slot::Opt(p)).map(|s| s.form(R_OPT_PERF)),
        (Mood::Imp, Tense::Pres) => imperative(v, p).map(|f| f.form(v.rule(R_IMP))),
        _ => None,
    }
}

fn middle(v: &Verb, c: VerbCell, p: u8) -> Option<Form> {
    v.has_mid().then_some(())?;
    let syn = |s: Slot| v.syn(s);
    let with = |a: &Aux, s: Slot| Some(join(table_syn(a, s)?, &v.syn(Slot::Ptcp)?));
    match (c.mood, c.tense) {
        (Mood::Ind, Tense::Pres) => syn(Slot::MidPres(p)).map(|s| s.form(R_MID)),
        (Mood::Ind, Tense::Imperf) => syn(Slot::MidIpf(p)).map(|s| s.form(R_MID)),
        (Mood::Ind, Tense::Aor) => match p {
            2 => syn(Slot::MidAor3),
            _ => syn(Slot::Aor(p)),
        }
        .map(|s| s.after("u").form(R_MID_U)),
        (Mood::Ind, Tense::Perf) => with(&JAM, Slot::Pres(p)).map(|s| s.form(R_MID_AUX)),
        (Mood::Ind, Tense::Pluperf) => with(&JAM, Slot::Ipf(p)).map(|s| s.form(R_MID_AUX)),
        (Mood::Ind, Tense::PastAnterior) => with(&JAM, Slot::Aor(p)).map(|s| s.form(R_MID_AUX)),
        (Mood::Ind, Tense::Fut) => syn(Slot::MidPres(p)).map(|s| s.after("do të").form(R_FUT)),
        (Mood::Ind, Tense::FutPerf) => with(&JAM, Slot::Subj(p)).map(|s| s.after("do të").form(R_FUT_PERF)),
        (Mood::Ind, Tense::FutPast) | (Mood::Cnd, Tense::Pres) | (Mood::Cnd, Tense::Imperf) => syn(Slot::MidIpf(p)).map(|s| s.after("do të").form(R_COND)),
        (Mood::Ind, Tense::FutPastPerf) | (Mood::Cnd, Tense::Perf) | (Mood::Cnd, Tense::Pluperf) => with(&JAM, Slot::Ipf(p)).map(|s| s.after("do të").form(R_COND_PERF)),
        (Mood::Sub, Tense::Pres) => syn(Slot::MidPres(p)).map(|s| s.after("të").form(R_SUBJ)),
        (Mood::Sub, Tense::Imperf) => syn(Slot::MidIpf(p)).map(|s| s.after("të").form(R_SUBJ)),
        (Mood::Sub, Tense::Perf) => with(&JAM, Slot::Subj(p)).map(|s| s.after("të").form(R_SUBJ_PERF)),
        (Mood::Sub, Tense::Pluperf) => with(&JAM, Slot::Ipf(p)).map(|s| s.after("të").form(R_SUBJ_PERF)),
        (Mood::Adm, Tense::Pres) => syn(Slot::AdmPres(p)).map(|s| s.after("u").form(R_MID_U)),
        (Mood::Adm, Tense::Imperf) => syn(Slot::AdmIpf(p)).map(|s| s.after("u").form(R_MID_U)),
        (Mood::Adm, Tense::Perf) => with(&JAM, Slot::AdmPres(p)).map(|s| s.form(R_MID_AUX)),
        (Mood::Adm, Tense::Pluperf) => with(&JAM, Slot::AdmIpf(p)).map(|s| s.form(R_MID_AUX)),
        (Mood::Opt, Tense::Pres) => syn(Slot::Opt(p)).map(|s| s.after("u").form(R_MID_U)),
        (Mood::Opt, Tense::Perf) => with(&JAM, Slot::Opt(p)).map(|s| s.form(R_MID_AUX)),
        (Mood::Imp, Tense::Pres) => imperative_slot(p, true).and_then(syn).map(|s| s.form(R_MID)),
        _ => None,
    }
}

/// Qeliza e foljes; `mid_ok` = joveprorja ka dëshmi (leksemë «-1» ose lemë joveprore).
fn conjugate_verb(v: &Verb, cell: VerbCell, mid_ok: bool) -> Option<Form> {
    let p = pn(cell.person, cell.number);
    match cell.voice {
        Voice::Act => (!v.mid_only).then(|| active(v, cell, p)).flatten(),
        Voice::Mid => mid_ok.then(|| middle(v, cell, p)).flatten(),
    }
}

/// Trajta e një qelize të zgjedhimit, me rregullin që e dha; `None` kur s'ka dëshmi të sigurt.
/// Joveprorja jepet vetëm nga leksema joveprore («laj-1», «aftosem»), jo nga veprorja e thjeshtë.
pub fn conjugate(lex: &Lexeme, cell: VerbCell) -> Option<Form> {
    let v = Verb::of(lex)?;
    conjugate_verb(&v, cell, v.mid_witness)
}

/// Pjesorja: baza e të gjitha kohëve të përbëra dhe e trajtave të pashtjelluara.
pub fn participle(lex: &Lexeme) -> Option<Form> {
    Verb::of(lex)?.syn(Slot::Ptcp).map(|s| s.form(R_PTCP))
}

/// Trajta e pashtjelluar veprore.
pub fn nonfinite(lex: &Lexeme, nf: NonFinite) -> Option<Form> {
    nonfinite_voice(lex, nf, Voice::Act)
}

/// Trajta e pashtjelluar në zërin e dhënë; joveprorja fut «u» para pjesores
/// («duke u larë», «për t'u larë», «pa u larë») dhe kërkon leksemën joveprore.
pub fn nonfinite_voice(lex: &Lexeme, nf: NonFinite, voice: Voice) -> Option<Form> {
    let v = Verb::of(lex)?;
    nonfinite_verb(&v, nf, voice, v.mid_witness)
}

fn nonfinite_verb(v: &Verb, nf: NonFinite, voice: Voice, mid_ok: bool) -> Option<Form> {
    let allowed = match voice {
        Voice::Act => !v.mid_only,
        Voice::Mid => mid_ok && v.has_mid(),
    };
    allowed.then_some(())?;
    let ptcp = v.syn(Slot::Ptcp)?;
    let mid = voice == Voice::Mid;
    let lead = |act: &str, mid_words: &str| ptcp.clone().after(match mid {
        true => mid_words,
        false => act,
    });
    Some(match nf {
        NonFinite::Participle => ptcp.clone().form(R_PTCP),
        NonFinite::Gerund => lead("duke", "duke u").form(R_GERUND),
        NonFinite::Infinitive => {
            let gheg = lead("me", "me u");
            let mut inf = lead("për të", "për t'u");
            inf.alts.push(gheg.text);
            inf.alts.extend(gheg.alts);
            inf.form(R_INF)
        }
        NonFinite::Privative => lead("pa", "pa u").form(R_PRIV),
        NonFinite::Absolutive => lead("me të", "me t'u").form(R_ABS),
    })
}

/// Pajtimi i dëshmive: çdo leksemë dëshmitare e lemës duhet të japë të njëjtën
/// trajtë; kur njëra hesht ose dy homonime ndryshojnë («pres» pret / «pres»
/// këput, «shihem» nga «shoh» / nga «shij»), nuk zgjidhet asnjëra.
fn agree(forms: impl Iterator<Item = Option<Form>>) -> Option<Form> {
    let all: Vec<Form> = forms.collect::<Option<Vec<Form>>>()?;
    let first = all.first()?;
    all.iter().all(|f| f.text == first.text).then(|| first.clone())
}

/// Leksemat foljore të lemës: standardet, ose jostandardet kur s'ka standarde.
fn verb_entries<'l>(lx: &'l Lexicon, lemma: &str) -> Vec<&'l Lexeme> {
    let verbs: Vec<&Lexeme> = lx.entries(lemma).iter().filter(|l| l.pos == Pos::Verb).collect();
    let standard: Vec<&Lexeme> = verbs.iter().copied().filter(|l| l.standard).collect();
    match standard.is_empty() {
        true => verbs,
        false => standard,
    }
}

/// Veta I e së tashmes joveprore e leksemës, kur klasa e jep.
fn mid_first(lex: &Lexeme) -> Option<String> {
    let probe = VerbCell { mood: Mood::Ind, tense: Tense::Pres, person: Person::P1, number: Number::Sing, voice: Voice::Mid };
    Verb::of(lex).and_then(|v| conjugate_verb(&v, probe, true)).map(|f| f.text)
}

/// Lemat veprore që mund ta japin lemën joveprore: trajta joveprore = një temë e
/// leksemës + («h») + «-em», ndaj kërkohet në gjithë leksikun çdo leksemë me atë
/// temë; asnjë homonim («shihem»: shoh / shij; «ngrihem»: ngre / ngrij) s'mbetet jashtë.
fn mid_names<'l>(lx: &'l Lexicon, lemma: &str) -> BTreeSet<&'l str> {
    let bases: Vec<&str> = [lemma.strip_suffix("hem"), lemma.strip_suffix("em")].into_iter().flatten().filter(|b| !b.is_empty()).collect();
    let carries = |l: &Lexeme| l.pos == Pos::Verb && l.stems.iter().flat_map(|s| s.split("//")).any(|v| bases.contains(&v.trim_matches('.')));
    match bases.is_empty() {
        true => BTreeSet::new(),
        false => lx.iter().filter(|l| carries(l)).map(|l| l.lemma.as_str()).collect(),
    }
}

/// Leksemat veprore të këtyre lemave që e prodhojnë vetë lemën joveprore.
fn mid_bases<'l>(lx: &'l Lexicon, lemma: &str, names: BTreeSet<&'l str>) -> Vec<&'l Lexeme> {
    names.into_iter().flat_map(|name| verb_entries(lx, name)).filter(|lex| mid_first(lex).is_some_and(|t| t == lemma)).collect()
}

/// A e jep leksema vetë lemën si trajtë veprore (veta I ose III e së tashmes).
fn active_citation(lex: &Lexeme) -> bool {
    Verb::of(lex).is_some_and(|v| !v.mid_only)
}

/// Leksemat e lemës dhe a është lema joveprore. Për lemën joveprore dëshmojnë
/// edhe leksema e saj (kur ka) edhe leksemat veprore të verifikuara; kur ato
/// s'pajtohen (p.sh. «birrem» si klasë më vete dhe si joveprore e «bjerr»), asgjë.
fn lexemes<'l>(lx: &'l Lexicon, lemma: &str, names: impl FnOnce() -> BTreeSet<&'l str>) -> (Vec<&'l Lexeme>, bool) {
    let direct = verb_entries(lx, lemma);
    match direct.iter().any(|l| active_citation(l)) {
        true => (direct, false),
        false => {
            let mut all = direct;
            for b in mid_bases(lx, lemma, names()) {
                let fresh = !all.iter().any(|o| std::ptr::eq(*o, b));
                all.extend(fresh.then_some(b));
            }
            (all, true)
        }
    }
}

/// Dëshmitarët e lemës për zërin e dhënë. Veprorja: të gjitha leksemat e vlefshme
/// të lemës veprore. Joveprorja: leksemat e lemës joveprore; te lema veprore,
/// vetëm kur ka leksemë «-1», dhe atëherë pajtohen të gjitha homonimet; pa të
/// (folje vetëm jokalimtare: «shkoj», «vdes») asnjë.
fn witnesses<'l>(lx: &'l Lexicon, lemma: &str, voice: Voice, names: impl FnOnce() -> BTreeSet<&'l str>) -> Vec<Verb<'l>> {
    let (entries, mid_lemma) = lexemes(lx, lemma, names);
    let verbs: Vec<Verb<'l>> = entries.into_iter().filter_map(Verb::of).collect();
    let evidence = verbs.iter().any(|v| v.mid_witness);
    match (mid_lemma, voice, evidence) {
        (true, Voice::Act, _) | (false, Voice::Mid, false) => Vec::new(),
        _ => verbs,
    }
}

/// Zgjedhimi sipas lemës në leksik: të gjitha leksemat dëshmitare duhet të pajtohen.
/// Lema joveprore («lahem») zgjidhet te leksema veprore e verifikuar dhe jep
/// vetëm trajtat joveprore. Për shumë thirrje mbi të njëjtin leksik: `Folja`.
pub fn conjugate_lemma(lx: &Lexicon, lemma: &str, cell: VerbCell) -> Option<Form> {
    agree(witnesses(lx, lemma, cell.voice, || mid_names(lx, lemma)).iter().map(|v| conjugate_verb(v, cell, true)))
}

/// Trajta e pashtjelluar sipas lemës në leksik.
pub fn nonfinite_lemma(lx: &Lexicon, lemma: &str, nf: NonFinite, voice: Voice) -> Option<Form> {
    agree(witnesses(lx, lemma, voice, || mid_names(lx, lemma)).iter().map(|v| nonfinite_verb(v, nf, voice, true)))
}

/// Zgjedhuesi mbi një leksik: treguesi «veta I joveprore → lemat veprore» ndërtohet
/// një herë; përgjigjet janë po ato të `conjugate_lemma` dhe `nonfinite_lemma`.
pub struct Folja<'l> {
    lx: &'l Lexicon,
    mid: HashMap<String, BTreeSet<&'l str>>,
}

impl<'l> Folja<'l> {
    pub fn new(lx: &'l Lexicon) -> Folja<'l> {
        let mut mid: HashMap<String, BTreeSet<&'l str>> = HashMap::new();
        for lex in lx.iter().filter(|l| l.pos == Pos::Verb) {
            mid_first(lex).into_iter().for_each(|t| {
                mid.entry(t).or_default().insert(lex.lemma.as_str());
            });
        }
        Folja { lx, mid }
    }

    fn names(&self, lemma: &str) -> BTreeSet<&'l str> {
        self.mid.get(lemma).cloned().unwrap_or_default()
    }

    pub fn conjugate(&self, lemma: &str, cell: VerbCell) -> Option<Form> {
        agree(witnesses(self.lx, lemma, cell.voice, || self.names(lemma)).iter().map(|v| conjugate_verb(v, cell, true)))
    }

    pub fn nonfinite(&self, lemma: &str, nf: NonFinite, voice: Voice) -> Option<Form> {
        agree(witnesses(self.lx, lemma, voice, || self.names(lemma)).iter().map(|v| nonfinite_verb(v, nf, voice, true)))
    }
}

/// Lemat e tabelës së parregullt dhe numri i trajtave të saj.
pub fn irregular_table() -> (Vec<&'static str>, usize) {
    (ndihmese::lemmas(), ndihmese::size())
}

/// Klasat me mbaresa të shkruara për një folje të vetme (them, shoh, jap, lë, eci,
/// vete, dua) dhe numri i atyre mbaresave.
pub fn verb_specific_classes() -> (Vec<&'static str>, usize) {
    klasat::own()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verb(lemma: &str, par: &str, stems: &str) -> Lexeme {
        Lexeme {
            lemma: lemma.into(),
            pos: Pos::Verb,
            gender_sg: None,
            gender_pl: None,
            animate: None,
            paradigms: vec![par.into()],
            stems: stems.split('|').map(str::to_string).collect(),
            standard: true,
            flags: Vec::new(),
            lexref: None,
            def_nom_sg: None,
        }
    }

    fn cell(mood: Mood, tense: Tense, person: Person, number: Number, voice: Voice) -> VerbCell {
        VerbCell { mood, tense, person, number, voice }
    }

    fn act(lex: &Lexeme, mood: Mood, tense: Tense, person: Person, number: Number) -> String {
        conjugate(lex, cell(mood, tense, person, number, Voice::Act)).map(|f| f.text).unwrap_or_default()
    }

    fn mid(lex: &Lexeme, mood: Mood, tense: Tense, person: Person, number: Number) -> String {
        conjugate(lex, cell(mood, tense, person, number, Voice::Mid)).map(|f| f.text).unwrap_or_default()
    }

    const ALL6: [(Person, Number); 6] = [
        (Person::P1, Number::Sing),
        (Person::P2, Number::Sing),
        (Person::P3, Number::Sing),
        (Person::P1, Number::Plur),
        (Person::P2, Number::Plur),
        (Person::P3, Number::Plur),
    ];

    fn row(lex: &Lexeme, mood: Mood, tense: Tense, voice: Voice) -> Vec<String> {
        ALL6.iter().map(|(p, n)| conjugate(lex, cell(mood, tense, *p, *n, voice)).map(|f| f.text).unwrap_or_default()).collect()
    }

    #[test]
    fn first_class_in_oj_gives_the_textbook_paradigm() {
        let punoj = verb("punoj", "lexoj", "puno.|punua.|punu.|pun.");
        assert_eq!(row(&punoj, Mood::Ind, Tense::Pres, Voice::Act), ["punoj", "punon", "punon", "punojmë", "punoni", "punojnë"]);
        assert_eq!(row(&punoj, Mood::Ind, Tense::Imperf, Voice::Act), ["punoja", "punoje", "punonte", "punonim", "punonit", "punonin"]);
        assert_eq!(row(&punoj, Mood::Ind, Tense::Aor, Voice::Act), ["punova", "punove", "punoi", "punuam", "punuat", "punuan"]);
        assert_eq!(row(&punoj, Mood::Sub, Tense::Pres, Voice::Act), ["të punoj", "të punosh", "të punojë", "të punojmë", "të punoni", "të punojnë"]);
        assert_eq!(row(&punoj, Mood::Adm, Tense::Pres, Voice::Act), ["punuakam", "punuake", "punuaka", "punuakemi", "punuakeni", "punuakan"]);
        assert_eq!(row(&punoj, Mood::Opt, Tense::Pres, Voice::Act), ["punofsha", "punofsh", "punoftë", "punofshim", "punofshi", "punofshin"]);
        assert_eq!(participle(&punoj).unwrap().text, "punuar");
        assert_eq!(act(&punoj, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "puno");
        assert_eq!(act(&punoj, Mood::Imp, Tense::Pres, Person::P2, Number::Plur), "punoni");
        assert_eq!(act(&punoj, Mood::Imp, Tense::Pres, Person::P3, Number::Sing), "", "urdhërorja s'ka vetë të tretë");
    }

    #[test]
    fn compound_tenses_use_kam_and_the_participle() {
        let punoj = verb("punoj", "lexoj", "puno.|punua.|punu.|pun.");
        assert_eq!(act(&punoj, Mood::Ind, Tense::Perf, Person::P1, Number::Sing), "kam punuar");
        assert_eq!(act(&punoj, Mood::Ind, Tense::Pluperf, Person::P3, Number::Sing), "kishte punuar");
        assert_eq!(act(&punoj, Mood::Ind, Tense::PastAnterior, Person::P1, Number::Plur), "patëm punuar");
        assert_eq!(act(&punoj, Mood::Ind, Tense::Fut, Person::P3, Number::Sing), "do të punojë");
        assert_eq!(act(&punoj, Mood::Ind, Tense::FutPerf, Person::P2, Number::Sing), "do të kesh punuar");
        assert_eq!(act(&punoj, Mood::Cnd, Tense::Pres, Person::P1, Number::Sing), "do të punoja");
        assert_eq!(act(&punoj, Mood::Cnd, Tense::Perf, Person::P3, Number::Plur), "do të kishin punuar");
        assert_eq!(act(&punoj, Mood::Sub, Tense::Perf, Person::P3, Number::Sing), "të ketë punuar");
        assert_eq!(act(&punoj, Mood::Adm, Tense::Perf, Person::P3, Number::Sing), "paska punuar");
        assert_eq!(act(&punoj, Mood::Opt, Tense::Perf, Person::P3, Number::Sing), "pastë punuar");
        let opt = conjugate(&punoj, cell(Mood::Opt, Tense::Perf, Person::P2, Number::Plur, Voice::Act)).unwrap();
        assert_eq!(opt.text, "paçi punuar");
        assert!(opt.accepts("paçit punuar"));
    }

    #[test]
    fn consonant_classes_and_stem_alternation() {
        let hap = verb("hap", "hap", "hap.");
        assert_eq!(row(&hap, Mood::Ind, Tense::Pres, Voice::Act), ["hap", "hap", "hap", "hapim", "hapni", "hapin"]);
        assert_eq!(row(&hap, Mood::Ind, Tense::Aor, Voice::Act), ["hapa", "hape", "hapi", "hapëm", "hapët", "hapën"]);
        assert_eq!(act(&hap, Mood::Sub, Tense::Pres, Person::P2, Number::Sing), "të hapësh");
        assert_eq!(participle(&hap).unwrap().text, "hapur");
        let flas = verb("flas", "flas", "flas.|flet.|flit.//flis.|flit.|fol.|folt.");
        assert_eq!(act(&flas, Mood::Ind, Tense::Pres, Person::P1, Number::Sing), "flas");
        assert_eq!(act(&flas, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "flet");
        assert_eq!(act(&flas, Mood::Ind, Tense::Pres, Person::P1, Number::Plur), "flasim");
        assert_eq!(row(&flas, Mood::Ind, Tense::Aor, Voice::Act), ["fola", "fole", "foli", "folëm", "folët", "folën"]);
        let mbeshtet = verb("mbështet", "mat", "mbështes.//mbështet.|mbështet.");
        assert_eq!(act(&mbeshtet, Mood::Ind, Tense::Pres, Person::P1, Number::Sing), "mbështet", "varianti i temës sipas lemës");
        assert_eq!(act(&mbeshtet, Mood::Ind, Tense::Imperf, Person::P3, Number::Sing), "mbështette");
        assert_eq!(participle(&flas).unwrap().text, "folur");
        let marr = verb("marr", "marr", "marr.|merr.|mirr.|mor.|muar.");
        assert_eq!(act(&marr, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "merr");
        assert_eq!(act(&marr, Mood::Ind, Tense::Aor, Person::P1, Number::Plur), "morëm");
        assert_eq!(participle(&marr).unwrap().text, "marrë");
    }

    #[test]
    fn imperative_keeps_j_after_a_and_drops_it_after_i_and_ye() {
        let laj = verb("laj", "laj", "la.");
        assert_eq!(act(&laj, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "laj");
        let fshij = verb("fshij", "laj", "fshi.");
        assert_eq!(act(&fshij, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "fshi");
        let kryej = verb("kryej", "ndiej", "krye.|kry.|kre.");
        assert_eq!(act(&kryej, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "krye");
        let shkruaj = verb("shkruaj", "shkruaj", "shkrua.|shkru.|shkro.");
        assert_eq!(act(&shkruaj, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "shkruaj");
    }

    #[test]
    fn participle_in_r_after_diphthong_in_re_after_simple_vowel() {
        let kthej = verb("kthej", "kthej", "kthe.|kthye.");
        assert_eq!(participle(&kthej).unwrap().text, "kthyer");
        let laj = verb("laj", "laj", "la.");
        assert_eq!(participle(&laj).unwrap().text, "larë");
        assert_eq!(row(&laj, Mood::Ind, Tense::Aor, Voice::Act), ["lava", "lave", "lau", "lamë", "latë", "lanë"]);
    }

    #[test]
    fn mediopassive_synthetic_and_analytic() {
        let laj = verb("laj", "laj-1", "la.");
        assert_eq!(row(&laj, Mood::Ind, Tense::Pres, Voice::Mid), ["lahem", "lahesh", "lahet", "lahemi", "laheni", "lahen"]);
        let ipf3 = conjugate(&laj, cell(Mood::Ind, Tense::Imperf, Person::P3, Number::Sing, Voice::Mid)).unwrap();
        assert!(!ipf3.accepts("lahesh"), "«lahesh» është veta II e së tashmes, jo veta III e së pakryerës");
        assert_eq!(row(&laj, Mood::Ind, Tense::Imperf, Voice::Mid), ["lahesha", "laheshe", "lahej", "laheshim", "laheshit", "laheshin"]);
        assert_eq!(row(&laj, Mood::Ind, Tense::Aor, Voice::Mid), ["u lava", "u lave", "u la", "u lamë", "u latë", "u lanë"]);
        assert_eq!(mid(&laj, Mood::Ind, Tense::Perf, Person::P1, Number::Sing), "jam larë");
        assert_eq!(mid(&laj, Mood::Adm, Tense::Pres, Person::P3, Number::Sing), "u laka");
        assert_eq!(mid(&laj, Mood::Opt, Tense::Pres, Person::P1, Number::Sing), "u lafsha");
        assert_eq!(mid(&laj, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "lahu");
        let hap1 = verb("aftosem", "hap-1", "aftos.");
        assert_eq!(mid(&hap1, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "aftoset");
        assert_eq!(act(&hap1, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "", "folja vetëm joveprore s'ka trajtë veprore");
        let caj = verb("çaj", "laj-1", "ça.");
        assert_eq!(act(&caj, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "çan", "lema veprore e leksemës joveprore dëshmon veproren");
    }

    #[test]
    fn auxiliaries_from_the_closed_table() {
        let jam = verb("jam", "jam", "ja.|je.//jë.|ësh.//ish.|qen.//klen.|ish.|qe.//kle.|ji.|qo.//klo.|ash.//âsh.|qën.//klën.");
        assert_eq!(row(&jam, Mood::Ind, Tense::Pres, Voice::Act), ["jam", "je", "është", "jemi", "jeni", "janë"]);
        assert_eq!(act(&jam, Mood::Ind, Tense::Perf, Person::P1, Number::Sing), "kam qenë");
        assert_eq!(conjugate(&jam, cell(Mood::Ind, Tense::Pres, Person::P1, Number::Sing, Voice::Act)).unwrap().rule, R_TABLE);
        assert!(conjugate(&jam, cell(Mood::Ind, Tense::Pres, Person::P1, Number::Sing, Voice::Mid)).is_none());
        let kam = verb("kam", "kam", "ka.|ke.|pas.|kish.|pat.|ki.|pa.");
        assert_eq!(row(&kam, Mood::Ind, Tense::Aor, Voice::Act), ["pata", "pate", "pati", "patëm", "patët", "patën"]);
    }

    #[test]
    fn nonfinite_forms() {
        let punoj = verb("punoj", "lexoj", "puno.|punua.|punu.|pun.");
        assert_eq!(nonfinite(&punoj, NonFinite::Gerund).unwrap().text, "duke punuar");
        let inf = nonfinite(&punoj, NonFinite::Infinitive).unwrap();
        assert_eq!(inf.text, "për të punuar");
        assert!(inf.accepts("me punuar"));
        assert_eq!(nonfinite(&punoj, NonFinite::Privative).unwrap().text, "pa punuar");
        let laj = verb("laj", "laj-1", "la.");
        assert_eq!(nonfinite_voice(&laj, NonFinite::Infinitive, Voice::Mid).unwrap().text, "për t'u larë");
        assert_eq!(nonfinite_voice(&laj, NonFinite::Gerund, Voice::Mid).unwrap().text, "duke u larë");
    }

    #[test]
    fn irregular_verbs_from_their_own_classes() {
        let them = verb("them", "them", "the.|thua.|tho.|thën.|thosh.|tha.|thu.|thue.|than.");
        assert_eq!(row(&them, Mood::Ind, Tense::Pres, Voice::Act), ["them", "thua", "thotë", "themi", "thoni", "thonë"]);
        assert_eq!(row(&them, Mood::Ind, Tense::Aor, Voice::Act), ["thashë", "the", "tha", "thamë", "thatë", "thanë"]);
        let shoh = verb("shoh", "shoh", "shoh.|sheh.|shih.|pa.|pe.");
        assert_eq!(row(&shoh, Mood::Ind, Tense::Aor, Voice::Act), ["pashë", "pe", "pa", "pamë", "patë", "panë"]);
        let vij = verb("vij", "vij", "vi.|vje.|ardh.|erdh.//erth.|eja.");
        assert_eq!(row(&vij, Mood::Ind, Tense::Pres, Voice::Act), ["vij", "vjen", "vjen", "vijmë", "vini", "vijnë"]);
        assert_eq!(act(&vij, Mood::Ind, Tense::Aor, Person::P3, Number::Sing), "erdhi");
        assert_eq!(participle(&vij).unwrap().text, "ardhur");
        let iki = verb("iki", "eci", "ik.");
        assert_eq!(row(&iki, Mood::Ind, Tense::Aor, Voice::Act), ["ika", "ike", "iku", "ikëm", "ikët", "ikën"], "«-u» pas fytores");
        let eci = verb("eci", "eci", "ec.");
        assert_eq!(act(&eci, Mood::Ind, Tense::Aor, Person::P3, Number::Sing), "eci");
    }

    #[test]
    fn hold_when_the_lexeme_does_not_fit_its_class() {
        let wrong = verb("feksoj", "hap", "fekso.|feksua.|feksu.|feks.");
        assert!(conjugate(&wrong, cell(Mood::Ind, Tense::Aor, Person::P1, Number::Sing, Voice::Act)).is_none());
        let mismatch = verb("mbloj", "brej", "mbloj.|mblojt.");
        assert!(participle(&mismatch).is_none());
        let unknown = verb("xyz", "pa-klasë", "xyz.");
        assert!(conjugate(&unknown, cell(Mood::Ind, Tense::Pres, Person::P1, Number::Sing, Voice::Act)).is_none());
        let mut noun = verb("mal", "lexoj", "mal.");
        noun.pos = Pos::Noun;
        assert!(participle(&noun).is_none());
    }

    fn lexicon(rows: &[(&str, &str, &str)]) -> Lexicon {
        let text: String = rows.iter().map(|(l, p, s)| format!("x	{l}	V	v	-	-	-	-	-	alb	std	{p}	{s}	-	-	-	-	-
")).collect();
        Lexicon::parse(&text)
    }

    fn by_lemma(lx: &Lexicon, lemma: &str, mood: Mood, tense: Tense, person: Person, number: Number, voice: Voice) -> String {
        conjugate_lemma(lx, lemma, cell(mood, tense, person, number, voice)).map(|f| f.text).unwrap_or_default()
    }

    #[test]
    fn mediopassive_needs_evidence() {
        let shkoj = verb("shkoj", "lexoj", "shko.|shkua.|shku.|shk.");
        assert_eq!(act(&shkoj, Mood::Ind, Tense::Aor, Person::P1, Number::Sing), "shkova");
        assert_eq!(mid(&shkoj, Mood::Ind, Tense::Aor, Person::P1, Number::Sing), "", "folja jokalimtare pa leksemë «-1»: asnjë «u shkova»");
        assert!(nonfinite_voice(&shkoj, NonFinite::Gerund, Voice::Mid).is_none());
        let lx = lexicon(&[("shkoj", "lexoj", "shko.|shkua.|shku.|shk.")]);
        assert_eq!(by_lemma(&lx, "shkoj", Mood::Imp, Tense::Pres, Person::P2, Number::Sing, Voice::Mid), "");
        let lx = lexicon(&[("laj", "laj", "la."), ("laj", "laj-1", "la.")]);
        assert_eq!(by_lemma(&lx, "laj", Mood::Ind, Tense::Pres, Person::P3, Number::Sing, Voice::Mid), "lahet");
        assert_eq!(by_lemma(&lx, "laj", Mood::Ind, Tense::Pres, Person::P3, Number::Sing, Voice::Act), "lan");
    }

    #[test]
    fn mediopassive_lemma_sees_every_homonym() {
        let shoh = ("shoh", "shoh", "shoh.|sheh.|shih.|pa.|pe.");
        let lx = lexicon(&[shoh]);
        assert_eq!(by_lemma(&lx, "shihem", Mood::Ind, Tense::Aor, Person::P3, Number::Sing, Voice::Mid), "u pa", "«shihem» → shoh: ndërrimi o/i nuk humbet");
        assert_eq!(by_lemma(&lx, "shihem", Mood::Ind, Tense::Perf, Person::P1, Number::Sing, Voice::Mid), "jam parë");
        let lx = lexicon(&[shoh, ("shij", "laj", "shi.")]);
        assert_eq!(by_lemma(&lx, "shihem", Mood::Ind, Tense::Aor, Person::P3, Number::Sing, Voice::Mid), "", "dy homonime: «u pa» / «u shi»");
        let lx = lexicon(&[("ngre", "di", "ngre.|ngri.//ngre.|ngrit.//ngrejt.|ngrer."), ("ngrij", "laj", "ngri.")]);
        assert_eq!(by_lemma(&lx, "ngrihem", Mood::Ind, Tense::Aor, Person::P1, Number::Sing, Voice::Mid), "", "«u ngrita» / «u ngriva»");
        assert_eq!(by_lemma(&lx, "ngre", Mood::Ind, Tense::Aor, Person::P1, Number::Sing, Voice::Act), "ngrita");
    }

    #[test]
    fn stem_variant_without_hiatus_and_with_intact_head() {
        let gergas = verb("gërgas", "flas", "gërgas.|gërget.|gërgit.//gërgis.|gërgit.|gërga.//gërgit.|NOSTEM.");
        assert_eq!(row(&gergas, Mood::Ind, Tense::Aor, Voice::Act), ["gërgita", "gërgite", "gërgiti", "gërgitëm", "gërgitët", "gërgitën"]);
        assert_eq!(participle(&gergas).unwrap().text, "gërgitur");
        let typo = verb("duartrokas", "pres-wait", "duartrokas.|duarktoket.|duartrokit.//duartrokis.|duartrokit.");
        assert_eq!(act(&typo, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "", "tema me kokë të prishur s'përdoret");
        assert_eq!(act(&typo, Mood::Ind, Tense::Aor, Person::P1, Number::Sing), "duartrokita");
        let vras = verb("vras", "vras", "vras.|vret.|vrit.//vris.|vra.|vrit.");
        assert_eq!(act(&vras, Mood::Ind, Tense::Aor, Person::P3, Number::Sing), "vrau", "«-u» e aoristit pas zanores s'është hiat");
        assert_eq!(klasat::head("thërras"), "thë");
        assert_eq!(klasat::head("puno"), "pu");
        assert_eq!(klasat::head("la"), "");
    }

    #[test]
    fn single_stem_only_in_classes_without_alternation() {
        let lone = verb("thërres", "humbas", "thërres.");
        assert_eq!(act(&lone, Mood::Ind, Tense::Pres, Person::P1, Number::Sing), "thërres");
        assert_eq!(act(&lone, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "", "klasa me ndërrim teme s'merr temën e parë për të tjerat");
        assert_eq!(act(&lone, Mood::Ind, Tense::Aor, Person::P1, Number::Sing), "");
        let hap = verb("hap", "hap", "hap.");
        assert_eq!(act(&hap, Mood::Ind, Tense::Aor, Person::P1, Number::Plur), "hapëm");
    }

    #[test]
    fn imperative_plural_equals_present_plural() {
        let pyes = verb("pyes", "mat", "pyes.|pyet.");
        assert_eq!(act(&pyes, Mood::Ind, Tense::Pres, Person::P1, Number::Plur), "pyesim");
        assert_eq!(act(&pyes, Mood::Ind, Tense::Pres, Person::P3, Number::Sing), "pyet");
        assert_eq!(act(&pyes, Mood::Sub, Tense::Pres, Person::P3, Number::Sing), "të pyesë");
        assert_eq!(act(&pyes, Mood::Imp, Tense::Pres, Person::P2, Number::Sing), "pyet");
        assert_eq!(act(&pyes, Mood::Ind, Tense::Pres, Person::P2, Number::Plur), "", "lema «-s» s'e dëshmon temën «-t» të vetës II shumës");
        assert_eq!(act(&pyes, Mood::Imp, Tense::Pres, Person::P2, Number::Plur), "");
        assert_eq!(act(&pyes, Mood::Ind, Tense::Imperf, Person::P1, Number::Sing), "");
        let flas = verb("flas", "flas", "flas.|flet.|flit.//flis.|flit.|fol.|folt.");
        assert_eq!(act(&flas, Mood::Imp, Tense::Pres, Person::P2, Number::Plur), "", "«flitni» ≠ e tashmja");
        let vij = verb("vij", "vij", "vi.|vje.|ardh.|erdh.//erth.|eja.");
        assert_eq!(act(&vij, Mood::Imp, Tense::Pres, Person::P2, Number::Plur), "ejani", "urdhërorja supletive");
        let hap = verb("hap", "hap", "hap.");
        assert_eq!(act(&hap, Mood::Imp, Tense::Pres, Person::P2, Number::Plur), "hapni");
    }

    #[test]
    fn dua_optative_from_its_own_stem() {
        let dua = verb("dua", "dua", "dua.|do.|du.|dash.|de.|dasht.//desht.|due.|da.");
        assert_eq!(row(&dua, Mood::Opt, Tense::Pres, Voice::Act), ["daça", "daç", "dashtë", "daçim", "daçi", "daçin"]);
        assert_eq!(participle(&dua).unwrap().text, "dashur");
    }

    #[test]
    fn mediopassive_lemma_resolves_to_its_verified_active_lexeme() {
        let lx = Lexicon::parse("x\tlaj\tV\tv\t-\t-\tvt\t-\t-\talb\tstd\tlaj\tla.\t-\t-\t-\t-\t-\n");
        let c = cell(Mood::Ind, Tense::Pres, Person::P3, Number::Sing, Voice::Mid);
        assert_eq!(conjugate_lemma(&lx, "lahem", c).unwrap().text, "lahet");
        let a = cell(Mood::Ind, Tense::Pres, Person::P3, Number::Sing, Voice::Act);
        assert!(conjugate_lemma(&lx, "lahem", a).is_none(), "lema joveprore s'jep trajtë veprore");
        assert!(conjugate_lemma(&lx, "lazem", c).is_none());
    }
}
