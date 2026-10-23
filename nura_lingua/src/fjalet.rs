//! Trajtat e fjalëve për bisedën.
//!
//! Rendi i dëshmisë për çdo trajtë:
//! 1. trajta e atestuar (UD, tabelat e Wiktionary-t, forma e validuar e gramatikës);
//! 2. rregulli i gramatikës;
//! 3. asgjë: fjalia që do ta kishte nevojë nuk thuhet.

use crate::biseda::{open_pool, portrait_sense, Forms, Reading};
use crate::gramatika::tipet::{Case, Definite, Gender, Mood, NonFinite, NounCell, Number, Person, Tense, VerbCell, Voice};
use crate::gramatika::tipet::Pos as GPos;
use crate::gramatika::{emri, folja, mbiemri};
use crate::graph::{sense_key, word_key, Lang, Rel, Source};
use crate::morph::{feat, FormOrigin};
use crate::query::Engine;
use crate::graph::{NodeKind, Pos};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

/// Synset-i «person» i WordNet 3.0: paraardhësi i çdo emri që tregon njeri.
const PERSON: &str = "00007846-n";
/// Synset-et «worker» dhe «professional»: kush ushtron një punë a profesion.
const WORKER: [&str; 2] = ["09632518-n", "10480253-n"];
/// Synset-i «animal» i WordNet 3.0.
const ANIMAL: &str = "00015388-n";

pub struct Fjalet<'a, 'e> {
    pub eng: &'e Engine<'a>,
    senses: RefCell<HashMap<String, Kuptimet>>,
}

/// Kuptimet e një fjale shqipe siç i sheh biseda.
#[derive(Debug, Clone, Default)]
struct Kuptimet {
    /// Kuptimi për të cilin flitet: nyja në graf, synset-i dhe klasa leksikografike.
    primary: Option<(u32, String, Option<u8>)>,
    /// Kuptimet e hapura: synset-i, klasa, rendi i kuptimit për secilën lemë angleze.
    open: Vec<(String, Option<u8>, BTreeMap<String, u32>)>,
    /// Etiketat shqipe të kuptimeve të hapura (BabelNet shënon kuptimin në kllapa: «Vera (pije alkoolike)»).
    labels: BTreeMap<String, Vec<String>>,
}

/// Treguesi «lemë angleze → synset-et e saj si emër, me rendin e kuptimit», i ndërtuar një herë
/// për grafin (nga vetë-laqet `SENSE_FREQ`).
type EnIndex = HashMap<String, Vec<(u32, String)>>;

fn en_index(g: &crate::graph::Graph) -> Arc<EnIndex> {
    static CACHE: OnceLock<Mutex<Option<(usize, usize, Arc<EnIndex>)>>> = OnceLock::new();
    let key = (g as *const crate::graph::Graph as usize, g.nodes.len());
    let cell = CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = match cell.lock() {
        Ok(x) => x,
        Err(poisoned) => poisoned.into_inner(),
    };
    let hit = guard.as_ref().filter(|(p, n, _)| (*p, *n) == key).map(|(_, _, ix)| ix.clone());
    match hit {
        Some(ix) => ix,
        None => {
            let mut ix: EnIndex = HashMap::new();
            for i in 0..g.nodes.len() as u32 {
                let synset = match &g.node(i).kind {
                    NodeKind::Sense { source: Source::Pwn30, synset, pos: Pos::Noun } => synset.clone(),
                    _ => continue,
                };
                let en = g.out_edges(i).find(|(_, e)| e.rel == Rel::Sem("SENSE_FREQ".into())).and_then(|(_, e)| e.attr("en"));
                for (l, r) in en.unwrap_or("").split(';').filter_map(|p| p.split_once('=')) {
                    match r.parse::<u32>() {
                        Ok(r) => ix.entry(l.to_string()).or_default().push((r, synset.clone())),
                        Err(_) => {}
                    }
                }
            }
            ix.values_mut().for_each(|v| v.sort());
            let ix = Arc::new(ix);
            *guard = Some((key.0, key.1, ix.clone()));
            ix
        }
    }
}

/// Trajtat njëjës të një fjale angleze të burimit («cars» → «car», «knives» → «knife»).
fn en_singulars(w: &str) -> Vec<String> {
    const IRREGULAR: [(&str, &str); 12] = [
        ("men", "man"),
        ("women", "woman"),
        ("children", "child"),
        ("people", "person"),
        ("mice", "mouse"),
        ("teeth", "tooth"),
        ("feet", "foot"),
        ("geese", "goose"),
        ("knives", "knife"),
        ("leaves", "leaf"),
        ("wolves", "wolf"),
        ("lives", "life"),
    ];
    let mut out = vec![w.to_string()];
    IRREGULAR.iter().filter(|(p, _)| *p == w).for_each(|(_, s)| out.push(s.to_string()));
    w.strip_suffix("ies").iter().for_each(|b| out.push(format!("{b}y")));
    w.strip_suffix("es").iter().for_each(|b| out.push(b.to_string()));
    w.strip_suffix('s').iter().for_each(|b| out.push(b.to_string()));
    out
}

fn ud_case(c: Case) -> &'static str {
    match c {
        Case::Nom => "Nom",
        Case::Gen => "Gen",
        Case::Dat => "Dat",
        Case::Acc => "Acc",
        Case::Abl => "Abl",
        Case::Voc => "Voc",
    }
}

fn ud_definite(d: Definite) -> &'static str {
    match d {
        Definite::Ind => "Ind",
        Definite::Def => "Def",
    }
}

fn ud_number(n: Number) -> &'static str {
    match n {
        Number::Sing => "Sing",
        Number::Plur => "Plur",
    }
}

impl<'a, 'e> Fjalet<'a, 'e> {
    pub fn new(eng: &'e Engine<'a>) -> Self {
        Fjalet { eng, senses: RefCell::new(HashMap::new()) }
    }

    /// Klasa dhe renditja e lemave angleze të një synset-i të WordNet-it (vetë-laku `SENSE_FREQ`).
    fn wordnet(&self, synset: &str) -> (Option<u8>, BTreeMap<String, u32>) {
        let edge = self
            .eng
            .g
            .get(&sense_key(Source::Pwn30, synset))
            .and_then(|n| self.eng.g.out_edges(n).find(|(_, e)| e.rel == Rel::Sem("SENSE_FREQ".into())).map(|(_, e)| e));
        let class = edge.and_then(|e| e.attr("lex")).and_then(|l| l.parse::<u8>().ok());
        let lemmas = edge
            .and_then(|e| e.attr("en"))
            .map(|en| en.split(';').filter_map(|p| p.split_once('=')).filter_map(|(l, r)| r.parse::<u32>().ok().map(|r| (l.to_string(), r))).collect())
            .unwrap_or_default();
        (class, lemmas)
    }

    /// Kuptimet e fjalës (të ruajtura pas llogaritjes së parë).
    fn kuptimet(&self, lemma: &str) -> Kuptimet {
        let cached = self.senses.borrow().get(lemma).cloned();
        match cached {
            Some(k) => k,
            None => {
                let mut f = self.eng.focus(lemma);
                self.eng.eliminate(&mut f, None, &[]);
                let open: Vec<(String, Option<u8>, BTreeMap<String, u32>)> = open_pool(&f)
                    .into_iter()
                    .map(|s| {
                        let (class, lemmas) = self.wordnet(&s.synset);
                        (s.synset.clone(), class, lemmas)
                    })
                    .collect();
                let primary = portrait_sense(self.eng, &f).map(|s| (s.sense, s.synset.clone(), self.wordnet(&s.synset).0));
                let labels: BTreeMap<String, Vec<String>> = open_pool(&f).into_iter().map(|s| (s.synset.clone(), s.labels_sq.clone())).collect();
                let k = Kuptimet { primary, open, labels };
                self.senses.borrow_mut().insert(lemma.to_string(), k.clone());
                k
            }
        }
    }

    /// Synset-i `upper` është hiperonim i `lower` në një ose dy hapa të WordNet-it.
    fn within_two(&self, lower: &str, upper: &str) -> bool {
        let g = self.eng.g;
        let (from, to) = match (g.get(&sense_key(Source::Pwn30, lower)), g.get(&sense_key(Source::Pwn30, upper))) {
            (Some(a), Some(b)) => (a, b),
            _ => return false,
        };
        let up = |n: u32| -> Vec<u32> {
            g.out_edges(n)
                .filter(|(_, e)| e.rel == Rel::Wn("@".into()) || e.rel == Rel::Wn("@i".into()))
                .map(|(_, e)| e.to)
                .collect()
        };
        let one = up(from);
        one.contains(&to) || one.iter().any(|&m| up(m).contains(&to))
    }

    fn attested(&self, lemma: &str, upos: &[&str], want: &[(&str, &str)]) -> Option<String> {
        self.eng
            .morph
            .entries_of(lemma)
            .into_iter()
            .filter(|e| upos.contains(&e.upos.as_str()))
            .find(|e| want.iter().all(|(k, v)| feat(&e.feats, k) == Some(*v)) && feat(&e.feats, "Voice") != Some("Pass"))
            .map(|e| e.form.to_lowercase())
            .filter(|f| !f.contains(' ') || lemma.contains(' '))
    }

    /// Pjesa e ligjëratës: leksiku gramatikor kur është ngarkuar, ndryshe skajet gramatikore të grafit.
    fn has_pos(&self, lemma: &str, pos: GPos, code: &str) -> bool {
        let by_grammar = self.eng.gram.map(|lx| lx.entries(lemma).iter().any(|l| l.pos == pos));
        let by_graph = || {
            self.eng
                .g
                .get(&word_key(Lang::Sq, lemma))
                .map(|w| self.eng.g.out_edges(w).any(|(_, e)| e.rel == Rel::Lex && e.attr("pos") == Some(code)))
                .unwrap_or(false)
        };
        match by_grammar {
            Some(true) => true,
            _ => by_graph(),
        }
    }

    fn lex_attr(&self, lemma: &str, key: &str) -> Option<String> {
        let w = self.eng.g.get(&word_key(Lang::Sq, lemma))?;
        self.eng
            .g
            .out_edges(w)
            .filter(|(_, e)| e.rel == Rel::Lex && e.source == Source::Uniparser)
            .find_map(|(_, e)| e.attr(key).map(str::to_string))
    }

    fn conjugated(&self, lemma: &str, mood: Mood, number: Number) -> Option<String> {
        let lx = self.eng.gram?;
        let cell = VerbCell { mood, tense: Tense::Pres, person: Person::P3, number, voice: Voice::Act };
        folja::conjugate_lemma(lx, lemma, cell).map(|f| f.text)
    }


    /// A arrin kuptimi kryesor i emrit, lart nëpër hiperonimet, te një nga synset-et e dhëna?
    fn reaches(&self, noun: &str, synsets: &[&str]) -> bool {
        let targets: Vec<u32> = synsets.iter().filter_map(|s| self.eng.g.get(&sense_key(Source::Pwn30, s))).collect();
        let start = match self.primary_sense(noun) {
            Some(s) => s,
            None => return false,
        };
        let mut seen: HashSet<u32> = HashSet::from([start]);
        let mut frontier = vec![start];
        for _ in 0..14 {
            let mut next = Vec::new();
            for n in frontier {
                for (_, e) in self.eng.g.out_edges(n) {
                    let up = match &e.rel {
                        Rel::Wn(s) => s == "@" || s == "@i",
                        Rel::Bn(r) => r == "HYPERNYM",
                        _ => false,
                    };
                    match (up, seen.insert(e.to)) {
                        (true, true) => next.push(e.to),
                        _ => {}
                    }
                }
            }
            match next.iter().any(|n| targets.contains(n)) {
                true => return true,
                false => {}
            }
            frontier = next;
        }
        false
    }

    /// Kuptimi kryesor i emrit (i pari që mbetet hapur pas eliminimit pa kontekst).
    fn primary_sense(&self, lemma: &str) -> Option<u32> {
        self.kuptimet(lemma).primary.map(|(n, _, _)| n)
    }
}

impl<'a, 'e> Forms for Fjalet<'a, 'e> {
    fn is_noun(&self, lemma: &str) -> bool {
        self.has_pos(lemma, GPos::Noun, "n")
    }

    fn is_verb(&self, lemma: &str) -> bool {
        self.has_pos(lemma, GPos::Verb, "v")
    }

    fn noun(&self, lemma: &str, case: Case, definite: Definite, number: Number) -> Option<String> {
        match self.is_noun(lemma) {
            true => {}
            false => return None,
        }
        let want = [("Case", ud_case(case)), ("Definite", ud_definite(definite)), ("Number", ud_number(number))];
        let attested = self.attested(lemma, &["NOUN", "PROPN"], &want);
        let grammar = self
            .eng
            .gram
            .and_then(|lx| emri::decline_lemma(lx, lemma, NounCell { case, number, definite }))
            .map(|f| f.text);
        let rule = match (case, definite, number) {
            (Case::Nom | Case::Acc, Definite::Ind, Number::Sing) => Some(lemma.to_string()),
            (Case::Nom, Definite::Def, Number::Sing) => {
                let r = self.eng.morph.definite_nom_sg(lemma);
                match r.origin {
                    FormOrigin::Citation => None,
                    _ => Some(r.text),
                }
            }
            _ => None,
        };
        attested.or(grammar).or(rule)
    }

    fn gender(&self, noun: &str) -> Option<Gender> {
        let by_lexicon = self.lex_attr(noun, "gender").and_then(|g| match g.as_str() {
            "m" | "mf" => Some(Gender::Masc),
            "f" => Some(Gender::Fem),
            "n" => Some(Gender::Neut),
            _ => None,
        });
        by_lexicon.or_else(|| {
            self.eng.morph.gender(noun).map(|g| match g {
                crate::morph::Gender::Masc => Gender::Masc,
                crate::morph::Gender::Fem => Gender::Fem,
            })
        })
    }

    /// Gjallesë: kuptimi kryesor arrin te «animal» ose «person» në WordNet; etiketa e leksikut
    /// vetëm kur WordNet s'e njeh fjalën.
    fn animate(&self, noun: &str) -> Option<bool> {
        match self.primary_sense(noun) {
            Some(_) => Some(self.reaches(noun, &[ANIMAL, PERSON])),
            None => self.lex_attr(noun, "anim").and_then(|a| match a.as_str() {
                "anim" => Some(true),
                "inanim" => Some(false),
                _ => None,
            }),
        }
    }

    fn human(&self, noun: &str) -> bool {
        self.reaches(noun, &[PERSON])
    }

    fn worker(&self, noun: &str) -> bool {
        self.reaches(noun, &WORKER)
    }


    fn verb(&self, lemma: &str, number: Number) -> Option<String> {
        let attested = match number {
            Number::Sing => crate::verb::present_3sg(self.eng.g, self.eng.morph, lemma).map(|v| v.text),
            Number::Plur => self.attested(
                lemma,
                &["VERB", "AUX"],
                &[("Person", "3"), ("Number", "Plur"), ("Tense", "Pres"), ("Mood", "Ind")],
            ),
        };
        attested.or_else(|| self.conjugated(lemma, Mood::Ind, number))
    }

    fn subjunctive(&self, lemma: &str, number: Number) -> Option<String> {
        self.attested(
            lemma,
            &["VERB", "AUX"],
            &[("Person", "3"), ("Number", ud_number(number)), ("Tense", "Pres"), ("Mood", "Sub")],
        )
        .or_else(|| self.conjugated(lemma, Mood::Sub, number))
        .map(|f| f.trim_start_matches("të ").to_string())
    }

    fn participle(&self, lemma: &str) -> Option<String> {
        self.attested(lemma, &["VERB", "AUX"], &[("VerbForm", "Part")]).or_else(|| {
            self.eng
                .gram
                .and_then(|lx| folja::nonfinite_lemma(lx, lemma, NonFinite::Participle, Voice::Act))
                .map(|f| f.text)
        })
    }

    fn adjective_np(&self, lemma: &str, gender: Gender, number: Number, case: Case, definite: Definite) -> Option<String> {
        let lx = self.eng.gram?;
        mbiemri::np_agree(gender, number, case, definite, &mbiemri::AdjLex::resolve(lx, lemma))
    }

    fn adjective(&self, lemma: &str, gender: Gender, number: Number) -> Option<String> {
        let grammar = self.eng.gram.and_then(|lx| mbiemri::citation(&mbiemri::AdjLex::resolve(lx, lemma), gender, number));
        grammar.or_else(|| self.adjective_by_rule(lemma, gender, number))
    }

    fn class(&self, lemma: &str) -> Option<u8> {
        self.kuptimet(lemma).primary.and_then(|(_, _, c)| c)
    }

    /// Ndër kuptimet e hapura të fjalës shqipe që i ka edhe fjala angleze, merret ai që WordNet-i
    /// e rendit më lart për fjalën angleze (në barazim, synset-i më i vogël). Është i njëjti
    /// lexim kur ai kuptim është kuptimi për të cilin flitet, ose hiperonimi a hiponimi i tij deri
    /// në dy hapa («house» → «dwelling»); jo kur takohen vetëm lart («car» dhe «machine» takohen
    /// te «instrumentality», gjashtë hapa larg).
    fn reading(&self, lemma: &str, en: &str) -> Reading {
        let primary = match self.kuptimet(lemma).primary {
            Some((_, p, _)) => p,
            None => return Reading::Unknown,
        };
        match self.read_as(lemma, en) {
            None => Reading::Unknown,
            Some(syn) => match self.near(&syn, &primary) {
                true => Reading::Same,
                false => Reading::Other,
            },
        }
    }

    fn senses(&self, lemma: &str) -> Vec<String> {
        let k = self.kuptimet(lemma);
        let primary: Vec<String> = k.primary.iter().map(|(_, s, _)| s.clone()).collect();
        primary.iter().cloned().chain(k.open.iter().map(|(s, _, _)| s.clone()).filter(|s| !primary.contains(s))).collect()
    }

    fn sense_class(&self, synset: &str) -> Option<u8> {
        self.wordnet(synset).0
    }

    fn near(&self, a: &str, b: &str) -> bool {
        a == b || self.within_two(a, b) || self.within_two(b, a)
    }

    /// Kuptimi ku lexohet fjala angleze e burimit: (1) ndër kuptimet e hapura që e kanë si lemë,
    /// ai që WordNet-i e rendit më lart për të; (2) përndryshe, kuptimi i parë i fjalës angleze në
    /// WordNet, kur ai është një nga kuptimet e hapura ose afër njërit prej tyre (deri në dy hapa:
    /// «grass» → «herb», kuptimi «bar» si bimë).
    fn read_as(&self, lemma: &str, en: &str) -> Option<String> {
        let k = self.kuptimet(lemma);
        let variants = en_singulars(en);
        let labelled = k
            .open
            .iter()
            .filter_map(|(syn, _, lemmas)| variants.iter().filter_map(|v| lemmas.get(v)).min().map(|r| (*r, syn)))
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)))
            .map(|(_, s)| s.clone());
        let by_index = || -> Option<String> {
            let ix = en_index(self.eng.g);
            let first = variants.iter().filter_map(|v| ix.get(v)).filter_map(|v| v.first()).min()?.1.clone();
            k.open.iter().map(|(s, _, _)| s).find(|s| self.near(s, &first)).cloned()
        };
        labelled.or_else(by_index)
    }

    /// Emri i shkurtër i kuptimit: shënimi i BabelNet-it në kllapa («pije alkoolike»), ose
    /// hiperonimi i parë me emër shqip.
    fn sense_label(&self, lemma: &str, synset: &str) -> Option<String> {
        let k = self.kuptimet(lemma);
        let marked = k.labels.get(synset).and_then(|ls| {
            ls.iter().find_map(|l| l.split_once('(').and_then(|(_, r)| r.strip_suffix(')')).map(|x| x.trim().to_lowercase()))
        });
        let g = self.eng.g;
        let hypernym = || -> Option<String> {
            let node = g.get(&sense_key(Source::Pwn30, synset))?;
            g.out_edges(node)
                .filter(|(_, e)| e.rel == Rel::Wn("@".into()) || e.rel == Rel::Wn("@i".into()))
                .flat_map(|(_, e)| g.words_of_sense(e.to, Lang::Sq))
                .find_map(|w| match &g.node(w).kind {
                    NodeKind::Word { lemma, .. } => Some(lemma.clone()),
                    _ => None,
                })
        };
        marked.or_else(hypernym)
    }

    fn transitive_only(&self, verb: &str) -> bool {
        let flags: BTreeSet<String> = self
            .eng
            .g
            .get(&word_key(Lang::Sq, verb))
            .map(|w| {
                self.eng
                    .g
                    .out_edges(w)
                    .filter(|(_, e)| e.rel == Rel::Lex && e.source == Source::Uniparser)
                    .filter_map(|(_, e)| e.attr("trans"))
                    .flat_map(|t| t.split(',').map(|x| x.trim().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        flags.contains("vt") && !flags.contains("vi")
    }

    fn mid_lemma(&self, verb: &str) -> bool {
        verb.ends_with("em") && self.verb_mid(verb, Number::Sing).is_some()
    }

    fn verb_mid(&self, lemma: &str, number: Number) -> Option<String> {
        let lx = self.eng.gram?;
        let cell = VerbCell { mood: Mood::Ind, tense: Tense::Pres, person: Person::P3, number, voice: Voice::Mid };
        folja::conjugate_lemma(lx, lemma, cell).map(|f| f.text)
    }
}

impl<'a, 'e> Fjalet<'a, 'e> {
    /// Mbiemri pa gramatikën e ngarkuar: nyja sipas leksikut të grafit, femërorja sipas rregullit.
    fn adjective_by_rule(&self, lemma: &str, gender: Gender, number: Number) -> Option<String> {
        let (article, bare) = match lemma.split_once(' ') {
            Some((a @ ("i" | "e" | "të"), rest)) => (Some(a), rest),
            _ => (None, lemma),
        };
        let articled = article.is_some() || self.eng.g.get(&word_key(Lang::Sq, &format!("i {lemma}"))).is_some();
        let fem = || crate::sentence::feminine(self.eng.morph, bare).0;
        match (articled, gender, number) {
            (true, Gender::Fem, Number::Sing) => Some(format!("e {}", fem())),
            (true, _, Number::Sing) => Some(format!("i {bare}")),
            (false, Gender::Fem, Number::Sing) => Some(fem()),
            (false, _, Number::Sing) => Some(bare.to_string()),
            (_, _, Number::Plur) => None,
        }
    }
}
