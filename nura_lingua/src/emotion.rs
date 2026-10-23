//! Funksioni emocional i ndërfaqes — makinë gjendjesh dhe tranzicionesh.
//!
//! ÇFARË ËSHTË: një model i sjelljes shprehëse të UI-së. Gjendja ndryshon
//! vetëm nga ngjarje reale të ndërveprimit (shtypje, dërgim, rezultat kërkimi,
//! eliminim kuptimesh, verifikim, mungesë, paqartësi, zgjedhje e përdoruesit,
//! eksplorim, gabim teknik, kalim kohe). Çdo tranzicion ka shkak dhe rregull
//! të emërtuar (T01…T14) dhe regjistrohet.
//!
//! ÇFARË NUK ËSHTË: pretendim për ndjenja subjektive. Gjendja nuk prek kuptimin
//! faktik: ajo ndikon vetëm mënyrën e paraqitjes (lëvizje, ritëm, theks, ngjyrë,
//! dritë, organizim hapësinor, regjistër gjuhësor).
//!
//! TRASHËGIMIA: emrat LIGHT_COORDINATION, QUANTUM_ELIMINATION,
//! SHADOW_VERIFICATION, NURA_SPEAKING, ngjyrat NURA_GOLD/LIGHT_AMBER/
//! QUANTUM_BLUE/SHADOW_VIOLET dhe animacionet COORDINATION_PULSE,
//! ELIMINATION_ORBIT, VERIFY_RING, SPEAK_PULSE vijnë nga kontrata origjinale
//! `ui_contracts/emotional_command.rs` e ESS-MAI. Atje faza nxirrej nga markerë
//! teksti në output-in e Light-it; këtu faza nxirret nga ngjarje të tipizuara.
//! TË REJA (projektuar tani): HOLD_IZ_QUESTION, NEGATIVE_WITNESS, TECHNICAL_FAULT,
//! regjistrat gjuhësorë, ritmi, organizimi hapësinor dhe mënyra me lëvizje të reduktuar.
//! Kuptimi i HOLD (pritje e autorizimit, jo dije negative) dhe i iZ (output +
//! pyetje) ndjek semantikën e arkitektit të ESS-MAI.

use crate::MASS_SCALE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    LightCoordination,
    QuantumElimination,
    ShadowVerification,
    NuraSpeaking,
    HoldIzQuestion,
    NegativeWitness,
    TechnicalFault,
}

impl Phase {
    pub fn code(self) -> &'static str {
        match self {
            Phase::LightCoordination => "LIGHT_COORDINATION",
            Phase::QuantumElimination => "QUANTUM_ELIMINATION",
            Phase::ShadowVerification => "SHADOW_VERIFICATION",
            Phase::NuraSpeaking => "NURA_SPEAKING",
            Phase::HoldIzQuestion => "HOLD_IZ_QUESTION",
            Phase::NegativeWitness => "NEGATIVE_WITNESS",
            Phase::TechnicalFault => "TECHNICAL_FAULT",
        }
    }
    /// A vjen emri nga kontrata origjinale EmotionalCommand e ESS-MAI?
    pub fn inherited(self) -> bool {
        matches!(
            self,
            Phase::LightCoordination
                | Phase::QuantumElimination
                | Phase::ShadowVerification
                | Phase::NuraSpeaking
        )
    }
    pub fn label_sq(self) -> &'static str {
        match self {
            Phase::LightCoordination => "koordinim — në pritje, i vëmendshëm",
            Phase::QuantumElimination => "eliminim — kandidatët po ndahen dhe po bien",
            Phase::ShadowVerification => "verifikim — përputhja me burimin po kontrollohet",
            Phase::NuraSpeaking => "Nura flet — shprehje e mbështetur nga burimi",
            Phase::HoldIzQuestion => "HOLD + iZ — output i pjesshëm dhe pyetje te përdoruesi",
            Phase::NegativeWitness => "dëshmi mungese — burimi nuk e përmban; nuk është mohim",
            Phase::TechnicalFault => "gabim teknik — jo emocion, jo pasiguri gjuhësore",
        }
    }
    pub const ALL: [Phase; 7] = [
        Phase::LightCoordination,
        Phase::QuantumElimination,
        Phase::ShadowVerification,
        Phase::NuraSpeaking,
        Phase::HoldIzQuestion,
        Phase::NegativeWitness,
        Phase::TechnicalFault,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Përdoruesi po shtyp në fushën e hyrjes.
    Typing,
    /// Hyrja u dërgua: i₀ u mor.
    Submitted { terms: u32 },
    /// Ndarja: u gjetën kandidatë në burime (kuptime Yi, dëshmi forme Xi).
    CandidatesFound { senses: u32, sources_hit: u32 },
    /// Konteksti eliminoi kandidatë brenda hapësirës së tyre.
    Eliminated { removed: u32, remaining: u32 },
    /// Një kuptim/marrëdhënie u mbështet nga burimi.
    Verified { evidence_mass: u32 },
    /// Mbetën disa kuptime të barabarta — nevojitet zgjedhja e përdoruesit.
    Ambiguous { remaining: u32 },
    /// Fjala ose lidhja mungon në burimet e kontrolluara.
    Absent { sources_checked: u32 },
    /// Pohimi i përdoruesit bie ndesh me burimin (polariteti i kundërt).
    Contradiction { evidence_mass: u32 },
    /// Shprehja gjuhësore u ndërtua dhe u shfaq.
    Realized,
    /// Përdoruesi zgjodhi një kuptim (përgjigje ndaj pyetjes iZ).
    UserChoseSense,
    /// Përdoruesi hapi një nyje për eksplorim.
    Explored,
    /// Gabim teknik (I/O, indeks, transport).
    Fault,
    /// Kalim kohe (ms) — për ritmin dhe kthimin në qetësi.
    Tick { ms: u32 },
}

impl Event {
    pub fn code(&self) -> String {
        match self {
            Event::Typing => "Typing".into(),
            Event::Submitted { terms } => format!("Submitted(terms={terms})"),
            Event::CandidatesFound { senses, sources_hit } => {
                format!("CandidatesFound(senses={senses}, sources={sources_hit})")
            }
            Event::Eliminated { removed, remaining } => {
                format!("Eliminated(removed={removed}, remaining={remaining})")
            }
            Event::Verified { evidence_mass } => format!("Verified(mass={evidence_mass})"),
            Event::Ambiguous { remaining } => format!("Ambiguous(remaining={remaining})"),
            Event::Absent { sources_checked } => format!("Absent(checked={sources_checked})"),
            Event::Contradiction { evidence_mass } => format!("Contradiction(mass={evidence_mass})"),
            Event::Realized => "Realized".into(),
            Event::UserChoseSense => "UserChoseSense".into(),
            Event::Explored => "Explored".into(),
            Event::Fault => "Fault".into(),
            Event::Tick { ms } => format!("Tick({ms}ms)"),
        }
    }
}

/// Regjistri gjuhësor: ndikon vetëm kornizën e fjalisë, kurrë thelbin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    Neutral,
    Assertive,
    Measured,
    Questioning,
    Witnessing,
    Corrective,
    Technical,
}

impl Register {
    pub fn code(self) -> &'static str {
        match self {
            Register::Neutral => "NEUTRAL",
            Register::Assertive => "ASSERTIVE",
            Register::Measured => "MEASURED",
            Register::Questioning => "QUESTIONING",
            Register::Witnessing => "WITNESSING",
            Register::Corrective => "CORRECTIVE",
            Register::Technical => "TECHNICAL",
        }
    }
    pub const ALL: [Register; 7] = [
        Register::Neutral,
        Register::Assertive,
        Register::Measured,
        Register::Questioning,
        Register::Witnessing,
        Register::Corrective,
        Register::Technical,
    ];
    pub fn parse(s: &str) -> Option<Register> {
        Register::ALL.iter().copied().find(|r| r.code() == s)
    }
}

/// Cila anë e ndarjes i₀ → Xi / Yi theksohet (logoja: ari majtas, blu djathtas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    Balanced,
    /// Yi (kuptimet, blu) — eliminimi ndodh këtu.
    MeaningSide,
    /// Qendra i₀ — përputhja me burimin.
    Center,
    /// Y u verifikua dhe X trashëgon besimin: të dyja anët ndizen.
    XInheritsY,
    /// Kandidatët e barabartë mbahen pezull.
    Suspended,
    /// Ana që mungon shënohet, nuk mbushet.
    AbsenceMark,
    None,
}

impl Emphasis {
    pub fn code(self) -> &'static str {
        match self {
            Emphasis::Balanced => "BALANCED",
            Emphasis::MeaningSide => "MEANING_SIDE",
            Emphasis::Center => "CENTER",
            Emphasis::XInheritsY => "X_INHERITS_Y",
            Emphasis::Suspended => "SUSPENDED",
            Emphasis::AbsenceMark => "ABSENCE_MARK",
            Emphasis::None => "NONE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub seq: u64,
    pub at_ms: u64,
    pub from: Phase,
    pub to: Phase,
    pub cause: String,
    pub rule: &'static str,
}

/// Efektet vizuale dhe gjuhësore të gjendjes aktuale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expressive {
    pub phase: Phase,
    pub trust_mass: u32,
    pub intensity_mass: u32,
    pub attention_mass: u32,
    /// Emra ngjyrash të trashëguar (NURA_GOLD …) ose të rinj (HOLD_SILVER …).
    pub color: &'static str,
    pub motion: &'static str,
    pub animation: &'static str,
    /// Ritmi: periudha e pulsit në ms (0 = pa lëvizje).
    pub tempo_ms: u32,
    pub emphasis: Emphasis,
    /// Shpërndarja hapësinore e grafit (0..10000): sa larg hapen kandidatët.
    pub spread_mass: u32,
    /// Drita / shkëlqimi (0..10000).
    pub glow_mass: u32,
    pub register: Register,
    pub reduced_motion: bool,
}

#[derive(Debug, Clone)]
pub struct EmotionMachine {
    pub phase: Phase,
    pub trust_mass: u32,
    pub attention_mass: u32,
    pub contrast: bool,
    pub reduced_motion: bool,
    pub idle_ms: u64,
    pub clock_ms: u64,
    seq: u64,
    pub log: Vec<Transition>,
}

/// Pas sa ms qetësie kthehet NURA_SPEAKING / NEGATIVE_WITNESS në koordinim.
pub const SETTLE_MS: u64 = 12_000;

impl Default for EmotionMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl EmotionMachine {
    pub fn new() -> Self {
        Self {
            phase: Phase::LightCoordination,
            trust_mass: 2_500,
            attention_mass: 0,
            contrast: false,
            reduced_motion: false,
            idle_ms: 0,
            clock_ms: 0,
            seq: 0,
            log: Vec::new(),
        }
    }

    pub fn set_reduced_motion(&mut self, on: bool) {
        self.reduced_motion = on;
    }

    /// Rregullat e tranzicionit. Kthen tranzicionin kur rregulli zbatohet
    /// (ndryshim faze ose përditësim masash brenda së njëjtës fazë).
    pub fn apply(&mut self, ev: Event) -> Option<Transition> {
        use Phase::*;
        let from = self.phase;
        let decision: Option<(Phase, &'static str)> = match (from, ev) {
            // T13 — gabimi teknik ka kanalin e vet nga çdo gjendje.
            (_, Event::Fault) => Some((TechnicalFault, "T13 gabim teknik → TECHNICAL_FAULT")),
            // T01 — shtypja rrit vëmendjen vetëm në koordinim; nuk ndërpret HOLD.
            (LightCoordination, Event::Typing) => {
                self.attention_mass = (self.attention_mass + 1_200).min(MASS_SCALE);
                Some((LightCoordination, "T01 shtypje → vëmendje ↑"))
            }
            (_, Event::Typing) => None,
            // T02 — i₀ i ri fillon ciklin nga çdo gjendje (edhe nga gabimi).
            (_, Event::Submitted { .. }) => {
                self.contrast = false;
                self.attention_mass = MASS_SCALE;
                self.trust_mass = 2_500;
                Some((QuantumElimination, "T02 i₀ u mor → QUANTUM_ELIMINATION"))
            }
            // T03 — kandidatët e gjetur rrisin besimin sipas numrit të burimeve.
            (QuantumElimination, Event::CandidatesFound { sources_hit, .. }) => {
                self.trust_mass = (2_500 + sources_hit * 1_250).min(MASS_SCALE);
                Some((QuantumElimination, "T03 ndarja Xi/Yi → kandidatë"))
            }
            // T04 — eliminimi brenda hapësirës.
            (QuantumElimination, Event::Eliminated { .. }) => {
                Some((QuantumElimination, "T04 eliminim brenda hapësirës (−Yi)"))
            }
            // T05 — mbështetja nga burimi kalon në verifikim.
            (QuantumElimination | HoldIzQuestion | ShadowVerification, Event::Verified { evidence_mass }) => {
                self.trust_mass = evidence_mass.min(MASS_SCALE);
                Some((ShadowVerification, "T05 mbështetje nga burimi → SHADOW_VERIFICATION"))
            }
            // T06 — paqartësia: output + pyetje; pritje e autorizimit.
            (QuantumElimination, Event::Ambiguous { .. }) => {
                Some((HoldIzQuestion, "T06 kuptime të barabarta → HOLD_IZ_QUESTION"))
            }
            // T07 — mungesa e dëshmuar.
            (QuantumElimination | ShadowVerification, Event::Absent { .. }) => {
                self.trust_mass = 0;
                Some((NegativeWitness, "T07 mungesë në burim → NEGATIVE_WITNESS"))
            }
            // T08 — kundërshtimi ndaj pohimit të përdoruesit.
            (QuantumElimination | ShadowVerification, Event::Contradiction { evidence_mass }) => {
                self.contrast = true;
                self.trust_mass = evidence_mass.min(MASS_SCALE);
                Some((ShadowVerification, "T08 pohim në kundërshtim me burimin → SHADOW_VERIFICATION (kontrast)"))
            }
            // T09 — shprehja e realizuar.
            (ShadowVerification, Event::Realized) => Some((NuraSpeaking, "T09 shprehje e realizuar → NURA_SPEAKING")),
            (HoldIzQuestion, Event::Realized) => Some((HoldIzQuestion, "T09b output + pyetje iZ (HOLD mbetet)")),
            (NegativeWitness, Event::Realized) => Some((NegativeWitness, "T09c dëshmia e mungesës u shpreh")),
            // T10 — përgjigjja e përdoruesit e çliron HOLD-in.
            (HoldIzQuestion, Event::UserChoseSense) => {
                Some((ShadowVerification, "T10 zgjedhja e përdoruesit → SHADOW_VERIFICATION"))
            }
            (LightCoordination | NuraSpeaking | NegativeWitness, Event::UserChoseSense) => {
                Some((ShadowVerification, "T10b zgjedhje kuptimi → SHADOW_VERIFICATION"))
            }
            // T11 — eksplorimi zhvendos fokusin; nuk e prish HOLD-in.
            (LightCoordination | NuraSpeaking | NegativeWitness, Event::Explored) => {
                self.attention_mass = (self.attention_mass + 2_000).min(MASS_SCALE);
                Some((LightCoordination, "T11 eksplorim nyjeje → LIGHT_COORDINATION (fokus i ri)"))
            }
            // T12 — qetësimi me kohë: vetëm pas shprehjes ose dëshmisë.
            // HOLD nuk zbehet me kohë (pret autorizimin); gabimi pret i₀ të ri.
            (NuraSpeaking | NegativeWitness, Event::Tick { ms }) => {
                self.idle_ms += ms as u64;
                match self.idle_ms >= SETTLE_MS {
                    true => Some((LightCoordination, "T12 qetësim pas 12 s → LIGHT_COORDINATION")),
                    false => None,
                }
            }
            // T14 — vëmendja zbehet me kohë në koordinim; regjistrohet vetëm kur shuhet.
            (LightCoordination, Event::Tick { ms }) => {
                let before = self.attention_mass;
                self.attention_mass = self.attention_mass.saturating_sub(ms / 4);
                match (before > 0, self.attention_mass == 0) {
                    (true, true) => Some((LightCoordination, "T14 vëmendja u shua")),
                    _ => None,
                }
            }
            _ => None,
        };
        match ev {
            Event::Tick { ms } => self.clock_ms += ms as u64,
            _ => self.idle_ms = 0,
        }
        match decision {
            Some((to, rule)) => {
                self.phase = to;
                self.seq += 1;
                let t = Transition { seq: self.seq, at_ms: self.clock_ms, from, to, cause: ev.code(), rule };
                self.log.push(t.clone());
                match self.log.len() > 400 {
                    true => {
                        self.log.remove(0);
                    }
                    false => {}
                }
                Some(t)
            }
            None => None,
        }
    }

    pub fn expressive(&self) -> Expressive {
        use Phase::*;
        let (color, motion, animation, tempo, emphasis, spread, base) = match self.phase {
            LightCoordination => ("LIGHT_AMBER", "BREATHE", "COORDINATION_PULSE", 4_200, Emphasis::Balanced, 5_000, 500),
            QuantumElimination => ("QUANTUM_BLUE", "ROTATE", "ELIMINATION_ORBIT", 900, Emphasis::MeaningSide, 9_000, 1_500),
            ShadowVerification => ("SHADOW_VIOLET", "FOCUS", "VERIFY_RING", 1_400, Emphasis::Center, 3_500, 2_000),
            NuraSpeaking => ("NURA_GOLD", "FORWARD", "SPEAK_PULSE", 2_200, Emphasis::XInheritsY, 6_000, 1_000),
            HoldIzQuestion => ("HOLD_SILVER", "SUSPEND", "HOLD_SUSPEND", 3_000, Emphasis::Suspended, 7_500, 800),
            NegativeWitness => ("WITNESS_ASH", "STILL", "WITNESS_MARK", 6_000, Emphasis::AbsenceMark, 4_000, 300),
            TechnicalFault => ("FAULT_RED", "NONE", "FAULT_FLAT", 0, Emphasis::None, 5_000, 0),
        };
        let register = match (self.phase, self.contrast, self.trust_mass >= 7_500) {
            (TechnicalFault, _, _) => Register::Technical,
            (NegativeWitness, _, _) => Register::Witnessing,
            (HoldIzQuestion, _, _) => Register::Questioning,
            (NuraSpeaking | ShadowVerification, true, _) => Register::Corrective,
            (NuraSpeaking, false, true) => Register::Assertive,
            (NuraSpeaking | ShadowVerification, false, _) => Register::Measured,
            (LightCoordination | QuantumElimination, _, _) => Register::Neutral,
        };
        let intensity = match self.phase {
            TechnicalFault => 0,
            _ => (self.trust_mass / 2 + base + self.attention_mass / 5).min(MASS_SCALE),
        };
        let (motion, animation, tempo) = match self.reduced_motion {
            true => ("NONE", "NONE", 0),
            false => (motion, animation, tempo),
        };
        Expressive {
            phase: self.phase,
            trust_mass: self.trust_mass,
            intensity_mass: intensity,
            attention_mass: self.attention_mass,
            color,
            motion,
            animation,
            tempo_ms: tempo,
            emphasis,
            spread_mass: spread,
            glow_mass: intensity,
            register,
            reduced_motion: self.reduced_motion,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(evs: &[Event]) -> EmotionMachine {
        let mut m = EmotionMachine::new();
        for e in evs {
            m.apply(*e);
        }
        m
    }

    #[test]
    fn verified_cycle_reaches_nura_speaking_with_documented_rules() {
        let m = run(&[
            Event::Submitted { terms: 1 },
            Event::CandidatesFound { senses: 3, sources_hit: 2 },
            Event::Eliminated { removed: 2, remaining: 1 },
            Event::Verified { evidence_mass: 8_000 },
            Event::Realized,
        ]);
        assert_eq!(m.phase, Phase::NuraSpeaking);
        let rules: Vec<&str> = m.log.iter().map(|t| &t.rule[..3]).collect();
        assert_eq!(rules, ["T02", "T03", "T04", "T05", "T09"]);
        let x = m.expressive();
        assert_eq!(x.animation, "SPEAK_PULSE");
        assert_eq!(x.register, Register::Assertive);
        assert_eq!(x.emphasis, Emphasis::XInheritsY);
    }

    #[test]
    fn ambiguity_holds_and_does_not_decay_with_time() {
        let mut m = run(&[Event::Submitted { terms: 1 }, Event::Ambiguous { remaining: 3 }, Event::Realized]);
        assert_eq!(m.phase, Phase::HoldIzQuestion);
        m.apply(Event::Tick { ms: 60_000 });
        assert_eq!(m.phase, Phase::HoldIzQuestion, "HOLD pret autorizimin, nuk zbehet");
        assert_eq!(m.expressive().register, Register::Questioning);
        m.apply(Event::UserChoseSense);
        assert_eq!(m.phase, Phase::ShadowVerification);
    }

    #[test]
    fn absence_is_witness_not_fault_and_settles() {
        let mut m = run(&[Event::Submitted { terms: 1 }, Event::Absent { sources_checked: 5 }, Event::Realized]);
        assert_eq!(m.phase, Phase::NegativeWitness);
        assert_eq!(m.expressive().register, Register::Witnessing);
        assert_ne!(m.expressive().color, "FAULT_RED");
        m.apply(Event::Tick { ms: 6_000 });
        assert_eq!(m.phase, Phase::NegativeWitness);
        m.apply(Event::Tick { ms: 6_000 });
        assert_eq!(m.phase, Phase::LightCoordination);
        assert!(m.log.last().unwrap().rule.starts_with("T12"));
    }

    #[test]
    fn technical_fault_is_a_separate_channel() {
        let mut m = run(&[Event::Submitted { terms: 1 }, Event::Fault]);
        assert_eq!(m.phase, Phase::TechnicalFault);
        assert_eq!(m.expressive().register, Register::Technical);
        assert_eq!(m.expressive().intensity_mass, 0);
        m.apply(Event::Tick { ms: 100_000 });
        assert_eq!(m.phase, Phase::TechnicalFault, "gabimi nuk 'qetësohet' vetvetiu");
        m.apply(Event::Submitted { terms: 1 });
        assert_eq!(m.phase, Phase::QuantumElimination);
    }

    #[test]
    fn contradiction_selects_corrective_register() {
        let m = run(&[
            Event::Submitted { terms: 2 },
            Event::Contradiction { evidence_mass: 7_000 },
            Event::Realized,
        ]);
        assert_eq!(m.phase, Phase::NuraSpeaking);
        assert_eq!(m.expressive().register, Register::Corrective);
    }

    #[test]
    fn reduced_motion_keeps_phase_and_color_but_removes_motion() {
        let mut m = run(&[Event::Submitted { terms: 1 }]);
        m.set_reduced_motion(true);
        let x = m.expressive();
        assert_eq!(x.phase, Phase::QuantumElimination);
        assert_eq!(x.color, "QUANTUM_BLUE");
        assert_eq!((x.motion, x.animation, x.tempo_ms), ("NONE", "NONE", 0));
    }

    #[test]
    fn typing_does_not_interrupt_hold() {
        let mut m = run(&[Event::Submitted { terms: 1 }, Event::Ambiguous { remaining: 2 }]);
        assert!(m.apply(Event::Typing).is_none());
        assert_eq!(m.phase, Phase::HoldIzQuestion);
    }

    #[test]
    fn every_transition_has_cause_and_rule() {
        let m = run(&[
            Event::Typing,
            Event::Submitted { terms: 1 },
            Event::Absent { sources_checked: 5 },
            Event::Explored,
            Event::Fault,
        ]);
        assert_eq!(m.log.len(), 5);
        for t in &m.log {
            assert!(!t.cause.is_empty());
            assert!(t.rule.starts_with('T'));
        }
    }

    #[test]
    fn attention_decays_silently_and_logs_only_when_extinguished() {
        let mut m = run(&[Event::Typing]);
        assert_eq!(m.attention_mass, 1_200);
        assert!(m.apply(Event::Tick { ms: 1_000 }).is_none());
        assert_eq!(m.attention_mass, 950);
        let t = m.apply(Event::Tick { ms: 4_000 }).unwrap();
        assert!(t.rule.starts_with("T14"));
        assert_eq!(m.attention_mass, 0);
        assert!(m.apply(Event::Tick { ms: 1_000 }).is_none());
    }

    #[test]
    fn inherited_phase_names_are_marked() {
        assert!(Phase::NuraSpeaking.inherited());
        assert!(!Phase::HoldIzQuestion.inherited());
    }
}
