//! Dispeçeri i komandave të UI-së. I njëjti kod shërben dritaren Tauri dhe
//! shërbimin lokal. Çdo ndërveprim kalon në motorin emocional si ngjarje e tipizuar.

use crate::emotional_command::{EmotionalCommand, EMOTIONAL_CONTRACT_VERSION};
use nura_lingua::emotion::{EmotionMachine, Event, Expressive, Phase, Register, Transition};
use nura_lingua::expression::{realize, signature, Claim, ExprPlan, Polarity, Realization};
use nura_lingua::graph::{Lang, NodeKind, Rel};
use nura_lingua::layout::{analysis_view, neighborhood_view, View};
use nura_lingua::gramatika::leksiku::Lexicon;
use nura_lingua::query::{Analysis, CandStatus, Engine};
use nura_lingua::sentence::{self, Composed};
use crate::product::{self, Product, Receipt};
use std::path::PathBuf;
use nura_lingua::store::{conceptnet_partial, default_data_dir, index_path, load_index, read_lock, sq_lexicon, Built, LockRow};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct Core {
    pub built: Built,
    pub lex: HashMap<String, u32>,
    pub lock: Vec<LockRow>,
    /// Leksiku gramatikor (lakimi, zgjedhimi, përshtatja), i ngarkuar një herë në nisje.
    pub gram: Lexicon,
    pub load_ms: u128,
}

pub struct Session {
    pub em: EmotionMachine,
    pub last: Option<Analysis>,
    pub trace: u64,
    /// Rasti i fundit i hapur te produkti (skedari `.lgc`), për vazhdimin e bisedës atje.
    pub product_last: Option<PathBuf>,
}

pub struct App {
    pub core: Result<Core, String>,
    pub session: Mutex<Session>,
}

impl App {
    pub fn load() -> Self {
        let data = default_data_dir();
        let t0 = std::time::Instant::now();
        let core = load_index(&index_path(&data)).and_then(|built| {
            let lex = sq_lexicon(&built.graph);
            let lock = read_lock(&data)?;
            let gram = Lexicon::load(&data).unwrap_or_default();
            Ok(Core { built, lex, lock, gram, load_ms: t0.elapsed().as_millis() })
        });
        let mut em = EmotionMachine::new();
        match &core {
            Ok(_) => {}
            Err(_) => {
                em.apply(Event::Fault);
            }
        }
        Self { core, session: Mutex::new(Session { em, last: None, trace: 0, product_last: None }) }
    }

    pub fn dispatch(&self, cmd: &str, args: &Value) -> Result<Value, String> {
        match cmd.strip_prefix("product.") {
            Some(sub) => return self.product(sub, args),
            None => {}
        }
        let mut s = self.session.lock().map_err(|_| "gjendja e sesionit u helmua".to_string())?;
        match cmd {
            "status" => Ok(self.status(&s)),
            "typing" => {
                let t = s.em.apply(Event::Typing);
                Ok(json!({ "transition": t.map(|t| tr_json(&t)), "emotion": emotion_json(&s.em) }))
            }
            "tick" => {
                let ms = args.get("ms").and_then(|v| v.as_u64()).unwrap_or(1000).min(60_000) as u32;
                let t = s.em.apply(Event::Tick { ms });
                Ok(json!({ "transition": t.map(|t| tr_json(&t)), "emotion": emotion_json(&s.em) }))
            }
            "motion" => {
                let on = args.get("reduced").and_then(|v| v.as_bool()).unwrap_or(false);
                s.em.set_reduced_motion(on);
                Ok(json!({ "emotion": emotion_json(&s.em) }))
            }
            "ask" => {
                let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                match text.chars().count() {
                    0 => Err("hyrja është bosh".into()),
                    n => match n > 400 {
                        true => Err("hyrja kalon 400 shenja".into()),
                        false => self.ask(&mut s, &text),
                    },
                }
            }
            "choose" => {
                let lemma = args.get("lemma").and_then(|v| v.as_str()).unwrap_or("");
                let key = args.get("sense").and_then(|v| v.as_str()).unwrap_or("");
                self.choose(&mut s, lemma, key)
            }
            "explore" => {
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                self.explore(&mut s, key)
            }
            "retone" => {
                let reg = args
                    .get("register")
                    .and_then(|v| v.as_str())
                    .and_then(Register::parse)
                    .ok_or("regjistër i panjohur")?;
                let core = self.core.as_ref().map_err(|e| e.clone())?;
                match &s.last {
                    Some(a) => {
                        let r = realize(&a.plan, &core.built.morph, reg);
                        let base = realize(&a.plan, &core.built.morph, Register::Neutral);
                        Ok(json!({ "realization": real_json(&r), "core_identical": r.core_text == base.core_text }))
                    }
                    None => Err("nuk ka ende shprehje për t'u rishprehur".into()),
                }
            }
            other => Err(format!("komandë e panjohur: {other}")),
        }
    }

    /// Komandat e produktit: procesi i jashtëm ekzekutohet pa mbajtur bllokimin e sesionit.
    fn product(&self, sub: &str, args: &Value) -> Result<Value, String> {
        let prod = Product::discover(&default_data_dir());
        match sub {
            "status" => Ok(match &prod {
                Ok(p) => {
                    let mut v = p.status();
                    v["ok"] = json!(true);
                    v
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }),
            "ask" | "reply" => {
                let p = prod?;
                let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                let words: Vec<String> = args
                    .get("words")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str()).map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect())
                    .unwrap_or_default();
                let auto_reply = args.get("auto_reply").and_then(|v| v.as_bool()).unwrap_or(true);
                let continue_case = sub == "reply" || args.get("continue").and_then(|v| v.as_bool()).unwrap_or(false);
                // Fjalia: e ndërtuar nga Nura (fjalët) ose e dhënë nga përdoruesi (teksti).
                let composed: Option<Composed> = match words.is_empty() {
                    true => None,
                    false => {
                        let core = self.core.as_ref().map_err(|e| e.clone())?;
                        let eng = Engine::new(&core.built.graph, &core.built.morph, &core.lex).with_conceptnet_partial(conceptnet_partial(&core.lock)).with_grammar(&core.gram);
                        Some(sentence::compose(&eng, &words)?)
                    }
                };
                let sentence_text = composed.as_ref().map(|c| c.text.clone()).unwrap_or(text);
                match sentence_text.is_empty() {
                    true => return Err("s'ka fjali për t'u dërguar".into()),
                    false => {}
                }
                let previous = self.session.lock().map_err(|_| "gjendja e sesionit u helmua".to_string())?.product_last.clone();
                let first = match (continue_case, previous) {
                    (true, Some(prev)) => p.reply("ui", &prev, &sentence_text)?,
                    _ => p.new_case("ui", &sentence_text)?,
                };
                // Kthesa e dytë: materiali pozitiv/negativ i Nurës, kur fjalinë e ndërtoi ajo.
                // Refuzimi i saj nuk e humb kthesën e parë: thuhet si i tillë.
                let second: Option<Result<Receipt, String>> = match (&composed, auto_reply) {
                    (Some(c), true) => Some(p.reply("ui", &first.lgc, &c.material.line())),
                    _ => None,
                };
                let (second, second_err): (Option<Receipt>, Option<String>) = match second {
                    Some(Ok(r)) => (Some(r), None),
                    Some(Err(e)) => (None, Some(e)),
                    None => (None, None),
                };
                let mut s = self.session.lock().map_err(|_| "gjendja e sesionit u helmua".to_string())?;
                s.product_last = Some(second.as_ref().map(|r| r.lgc.clone()).unwrap_or_else(|| first.lgc.clone()));
                let t1 = s.em.apply(Event::Submitted { terms: sentence_text.split_whitespace().count() as u32 });
                let t2 = s.em.apply(Event::Verified { evidence_mass: 6_000 });
                let t3 = s.em.apply(Event::Realized);
                s.trace += 1;
                let steps: Vec<Value> = [t1, t2, t3].iter().flatten().map(tr_json).collect();
                let receipts: Vec<Value> = std::iter::once(&first).chain(second.iter()).map(Receipt::json).collect();
                let mut plain: Vec<String> = std::iter::once(&first).chain(second.iter()).map(product::plain).collect();
                plain.extend(second_err.map(|e| format!("Kthesa e dytë (materiali i Nurës) nuk u pranua: {e}")));
                Ok(json!({
                    "trace": s.trace,
                    "sentence": sentence_text,
                    "composed": composed.as_ref().map(composed_json),
                    "receipts": receipts,
                    "plain": plain,
                    "steps": steps,
                    "emotion": emotion_json(&s.em),
                }))
            }
            other => Err(format!("komandë e panjohur e produktit: {other}")),
        }
    }

    fn status(&self, s: &Session) -> Value {
        let phases: Vec<Value> = Phase::ALL
            .iter()
            .map(|p| json!({ "code": p.code(), "label": p.label_sq(), "inherited": p.inherited() }))
            .collect();
        match &self.core {
            Ok(c) => json!({
                "ok": true,
                "nodes": c.built.graph.nodes.len(),
                "edges": c.built.graph.edges.len(),
                "morph": c.built.morph.entries.len(),
                "sq_words": c.lex.len(),
                "load_ms": c.load_ms as u64,
                "report": c.built.report.lines,
                "sources": c.lock.iter().map(|r| json!({
                    "source": r.source, "file": r.file, "url": r.url, "version": r.version,
                    "license": r.license, "attribution": r.attribution, "bytes": r.bytes,
                    "sha256": r.sha256, "publisher_check": r.publisher_check, "status": r.status
                })).collect::<Vec<_>>(),
                "phases": phases,
                "emotion": emotion_json(&s.em),
            }),
            Err(e) => json!({
                "ok": false,
                "error": format!("indeksi nuk u ngarkua: {e}. Ekzekuto: cargo run -p nura_fetch dhe cargo run -p nura_lingua --bin nura_import"),
                "phases": phases,
                "emotion": emotion_json(&s.em),
            }),
        }
    }

    fn run_steps(&self, s: &mut Session, a: &Analysis) -> Vec<Value> {
        a.steps
            .iter()
            .map(|st| {
                let t = st.event.and_then(|e| s.em.apply(e));
                json!({
                    "code": st.code,
                    "label": st.label,
                    "nodes": st.nodes,
                    "edges": st.edges,
                    "event": st.event.map(|e| e.code()),
                    "transition": t.map(|t| tr_json(&t)),
                    "expressive": expressive_json(&s.em.expressive()),
                })
            })
            .collect()
    }

    fn finish(&self, s: &mut Session, core: &Core, a: Analysis, view: View) -> Value {
        let steps = self.run_steps(s, &a);
        let x = s.em.expressive();
        let r = realize(&a.plan, &core.built.morph, x.register);
        let all: Vec<Value> = Register::ALL
            .iter()
            .map(|reg| real_json(&realize(&a.plan, &core.built.morph, *reg)))
            .collect();
        let invariant = Register::ALL
            .iter()
            .all(|reg| realize(&a.plan, &core.built.morph, *reg).core_text == r.core_text);
        s.trace += 1;
        let out = json!({
            "trace": s.trace,
            "input": a.input,
            "form": a.form.code(),
            "analysis": analysis_json(core, &a),
            "plan": plan_json(&a.plan),
            "signature": format!("{:?}", signature(&a.plan)),
            "realization": real_json(&r),
            "registers": all,
            "core_invariant": invariant,
            "steps": steps,
            "view": view_json(&view),
            "emotion": emotion_json(&s.em),
            "emotional_command": emotional_command(s.trace, &x),
        });
        s.last = Some(a);
        out
    }

    fn ask(&self, s: &mut Session, text: &str) -> Result<Value, String> {
        let core = match &self.core {
            Ok(c) => c,
            Err(e) => {
                s.em.apply(Event::Fault);
                return Err(format!("gabim teknik: {e}"));
            }
        };
        let eng = Engine::new(&core.built.graph, &core.built.morph, &core.lex).with_conceptnet_partial(conceptnet_partial(&core.lock)).with_grammar(&core.gram);
        match eng.analyze(text) {
            Ok(a) => {
                // Fjalët pa kuptim shënohen në heshtje për shkarkimin e radhës nga BabelNet.
                for f in a.focus.iter().chain(a.other.iter()) {
                    match f.senses.is_empty() {
                        true => queue_for_babelnet(&f.resolved.lemma),
                        false => {}
                    }
                }
                let view = analysis_view(&core.built.graph, &a);
                Ok(self.finish(s, core, a, view))
            }
            Err(reason) => {
                // Hyrje jashtë gramatikës së kufizuar: dëshmi mungese rregulli, jo gabim teknik.
                let t1 = s.em.apply(Event::Submitted { terms: 0 });
                let t2 = s.em.apply(Event::Absent { sources_checked: 0 });
                let t3 = s.em.apply(Event::Realized);
                let steps: Vec<Value> = [t1, t2, t3].iter().flatten().map(tr_json).collect();
                Ok(json!({
                    "unsupported": true,
                    "input": text,
                    "message": "Këtë pyetje nuk e kuptoj ende. Provo: «qen», «zjarr armë», «a është qeni kafshë?», «e kundërta e bardhë», «nga vjen fjala mësues», «si quhet qen në anglisht».",
                    "reason": reason,
                    "steps": steps,
                    "emotion": emotion_json(&s.em),
                }))
            }
        }
    }

    fn choose(&self, s: &mut Session, lemma: &str, key: &str) -> Result<Value, String> {
        let core = self.core.as_ref().map_err(|e| e.clone())?;
        let eng = Engine::new(&core.built.graph, &core.built.morph, &core.lex).with_conceptnet_partial(conceptnet_partial(&core.lock)).with_grammar(&core.gram);
        let a = eng.choose_sense(lemma, key)?;
        let view = analysis_view(&core.built.graph, &a);
        Ok(self.finish(s, core, a, view))
    }

    fn explore(&self, s: &mut Session, key: &str) -> Result<Value, String> {
        let core = self.core.as_ref().map_err(|e| e.clone())?;
        let g = &core.built.graph;
        let id = g.get(key).ok_or_else(|| format!("nyja «{key}» nuk ekziston në graf"))?;
        let t = s.em.apply(Event::Explored);
        let n = g.node(id);
        let (label, lang) = g.display_label(id);
        let texts = |v: &Vec<nura_lingua::graph::Text>| -> Vec<Value> {
            v.iter().map(|t| json!({ "lang": t.lang.code(), "source": t.source.label(), "text": t.text })).collect()
        };
        let kind = match &n.kind {
            NodeKind::Word { .. } => "word",
            NodeKind::Sense { .. } => "sense",
            NodeKind::Concept { .. } => "concept",
            NodeKind::Sentence { .. } => "sentence",
        };
        let view = neighborhood_view(g, id, 14);
        Ok(json!({
            "node": {
                "key": n.key, "kind": kind, "label": label, "lang": node_lang(&n.kind, lang),
                "labels": texts(&n.labels), "glosses": texts(&n.glosses), "examples": texts(&n.examples),
                "out_degree": g.out[id as usize].len(), "in_degree": g.inc[id as usize].len(),
            },
            "view": view_json(&view),
            "transition": t.map(|t| tr_json(&t)),
            "emotion": emotion_json(&s.em),
        }))
    }
}

// ───────────────────────────── JSON ─────────────────────────────

fn composed_json(c: &Composed) -> Value {
    json!({
        "text": c.text,
        "frame": c.frame,
        "polarity": match c.polarity { Polarity::Pos => "pos", Polarity::Neg => "neg" },
        "links": c.links.iter().map(|l| json!({ "rel": l.rel, "label": l.label, "sources": l.sources.iter().map(|s| s.label()).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "agreements": c.agreements.iter().map(|a| json!({ "rule": a.rule, "surface": a.surface })).collect::<Vec<_>>(),
        "counter": c.counter,
        "material": {
            "positive": c.material.positive, "negative": c.material.negative, "distinction": c.material.distinction,
            "source": c.material.source, "refutation": c.material.refutation, "line": c.material.line(),
        },
    })
}

fn tr_json(t: &Transition) -> Value {
    json!({
        "seq": t.seq, "at_ms": t.at_ms, "from": t.from.code(), "to": t.to.code(),
        "cause": t.cause, "rule": t.rule,
    })
}

fn expressive_json(x: &Expressive) -> Value {
    json!({
        "phase": x.phase.code(), "phase_label": x.phase.label_sq(), "inherited": x.phase.inherited(),
        "trust_mass": x.trust_mass, "intensity_mass": x.intensity_mass, "attention_mass": x.attention_mass,
        "color": x.color, "motion": x.motion, "animation": x.animation, "tempo_ms": x.tempo_ms,
        "emphasis": x.emphasis.code(), "spread_mass": x.spread_mass, "glow_mass": x.glow_mass,
        "register": x.register.code(), "reduced_motion": x.reduced_motion,
    })
}

fn emotion_json(m: &EmotionMachine) -> Value {
    let log: Vec<Value> = m.log.iter().rev().take(40).map(tr_json).collect();
    json!({ "expressive": expressive_json(&m.expressive()), "log": log, "clock_ms": m.clock_ms })
}

/// Projeksion në kontratën e trashëguar `EmotionalCommand` të ESS-MAI.
fn emotional_command(trace: u64, x: &Expressive) -> EmotionalCommand {
    EmotionalCommand {
        contract_version: EMOTIONAL_CONTRACT_VERSION,
        trace_id: trace,
        phase: x.phase.code().to_string(),
        trust_mass: x.trust_mass,
        intensity_mass: x.intensity_mass,
        color: x.color.to_string(),
        motion: x.motion.to_string(),
        animation: x.animation.to_string(),
        source: "UI_ESS_MAI_EMOTION_ENGINE".to_string(),
    }
}

/// Matjet e oratorisë për tekstin e përgjigjes (formula të deklaruara te `nura_lingua::oratoria`).
fn oratory_json(text: &str) -> Value {
    let sentences: Vec<String> = text.split_inclusive(['.', '?', '!']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let v = nura_lingua::oratoria::vleresim_teksti(&sentences);
    let last = sentences.last().map(|s| nura_lingua::oratoria::mat(s)).unwrap_or_default();
    json!({
        "ritmi": v.ritmi, "kadenca": v.kadenca, "baraspesha": v.baraspesha, "rritja": v.rritja, "fryma": v.fryma,
        "gjithsej": v.gjithsej, "cursus": nura_lingua::oratoria::cursus(&last),
    })
}

fn real_json(r: &Realization) -> Value {
    json!({
        "register": r.register.code(), "frame_open": r.frame_open, "core": r.core_text,
        "frame_close": r.frame_close, "question": r.question, "full": r.full, "sources": r.sources,
        "forms": r.forms.iter().map(|f| json!({ "surface": f.surface, "origin": f.origin })).collect::<Vec<_>>(),
        "oratoria": oratory_json(&r.core_text),
    })
}

fn pol(p: Polarity) -> &'static str {
    match p {
        Polarity::Pos => "POHIM",
        Polarity::Neg => "MOHIM",
    }
}

fn claim_json(c: &Claim) -> Value {
    match c {
        Claim::IsA { subj, obj } => json!({ "type": "IsA", "subj": subj.lemma, "obj": obj.lemma, "context": subj.sense_gloss }),
        Claim::Opposite { a, b } => json!({ "type": "Opposite", "a": a.lemma, "b": b.lemma }),
        Claim::HasMeaning { word, gloss, gloss_lang, index, of } => {
            json!({ "type": "HasMeaning", "word": word.lemma, "gloss": gloss, "gloss_lang": gloss_lang.code(), "index": index, "of": of })
        }
        Claim::SenseCount { word, open, total } => json!({ "type": "SenseCount", "word": word.lemma, "open": open, "total": total }),
        Claim::NotInSource { word, sources, partial } => json!({
            "type": "NotInSource", "word": word.lemma, "partial": partial,
            "sources": sources.iter().map(|s| s.label()).collect::<Vec<_>>()
        }),
        Claim::Related { a, b, b_lang, relation } => {
            json!({ "type": "Related", "a": a.lemma, "b": b.lemma, "b_lang": b_lang, "relation": relation })
        }
        Claim::Facet { word, facet, items } => json!({ "type": "Facet", "word": word.lemma, "facet": facet.code(), "items": items }),
        Claim::Chain { subj, chain, obj, .. } => json!({ "type": "Chain", "subj": subj.lemma, "chain": chain, "obj": obj.lemma }),
        Claim::Compare { a, b, common, a_cat, b_cat } => {
            json!({ "type": "Compare", "a": a.lemma, "b": b.lemma, "common": common, "a_cat": a_cat, "b_cat": b_cat })
        }
        Claim::NoLink { subj, obj, relation } => json!({ "type": "NoLink", "subj": subj.lemma, "obj": obj.lemma, "relation": relation }),
        Claim::Usage { word, form, sent_id, source, .. } => {
            json!({ "type": "Usage", "word": word.lemma, "form": form, "sent_id": sent_id, "source": source.label() })
        }
        Claim::Sentence { text, frame, support, counter } => {
            json!({ "type": "Sentence", "text": text, "frame": frame, "support": support, "counter": counter })
        }
        Claim::Discourse { text, intent, links } => json!({ "type": "Discourse", "text": text, "intent": intent, "links": links }),
    }
}

fn plan_json(p: &ExprPlan) -> Value {
    let clause = |c: &nura_lingua::expression::Clause| {
        json!({
            "claim": claim_json(&c.claim),
            "polarity": pol(c.polarity),
            "attribution": c.attribution.as_ref().map(|a| json!({
                "sources": a.sources.iter().map(|s| s.label()).collect::<Vec<_>>(), "note": a.note
            })),
        })
    };
    json!({
        "verdict": p.verdict.code(),
        "user_claim": p.user_claim.as_ref().map(clause),
        "user_condition": p.user_condition.as_ref().map(clause),
        "condition": p.condition.as_ref().map(clause),
        "core": p.core.iter().map(clause).collect::<Vec<_>>(),
        "question": p.question,
    })
}

fn analysis_json(core: &Core, a: &Analysis) -> Value {
    let g = &core.built.graph;
    let eng = Engine::new(&core.built.graph, &core.built.morph, &core.lex).with_grammar(&core.gram);
    let focus = a.focus.as_ref().map(|f| {
        json!({
            "input": f.resolved.input, "lemma": f.resolved.lemma, "how": f.resolved.how,
            "word_key": f.resolved.word.map(|w| g.node(w).key.clone()),
            "sources_hit": f.sources_hit.iter().map(|s| s.label()).collect::<Vec<_>>(),
            "senses": f.senses.iter().map(|s| json!({
                "key": g.node(s.sense).key, "synset": s.synset, "pos": s.pos.label_sq(),
                "via": s.via.label(), "labels_sq": s.labels_sq, "labels_en": s.labels_en,
                "gloss_sq": s.gloss_sq, "gloss_en": s.gloss_en, "score": s.score,
                "label": eng.short_label(s, &f.resolved.lemma),
                "status": match &s.status { CandStatus::Open => "open", CandStatus::Eliminated(_) => "eliminated", CandStatus::Selected(_) => "selected" },
                "reason": match &s.status { CandStatus::Open => String::new(), CandStatus::Eliminated(r) | CandStatus::Selected(r) => r.clone() },
            })).collect::<Vec<_>>(),
            "usages": f.usages.iter().take(8).map(|u| json!({
                "sent_id": u.sent_id, "source": u.source.label(), "form": u.form, "feats": u.feats,
                "deprel": u.deprel, "text": u.text
            })).collect::<Vec<_>>(),
            "usage_count": f.usages.len(),
            "form_rels": f.form_rels.iter().map(|r| json!({
                "rel": r.rel, "other": r.other_term, "other_lang": r.other_lang.map(|l| l.code()),
                "dataset": r.dataset, "license": r.license
            })).collect::<Vec<_>>(),
        })
    });
    let path = |p: &Vec<nura_lingua::query::PathStep>| -> Vec<Value> {
        p.iter()
            .map(|st| {
                let (l, lang) = g.display_label(st.node);
                json!({ "key": g.node(st.node).key, "label": l, "lang": lang.code(),
                        "sq": g.label_in(st.node, Lang::Sq), "edge": st.edge })
            })
            .collect()
    };
    json!({
        "focus": focus,
        "other": a.other.as_ref().map(|o| json!({ "lemma": o.resolved.lemma, "found": o.resolved.word.is_some(), "senses": o.senses.len() })),
        "path": path(&a.path),
        "cond_path": path(&a.cond_path),
        "bridge_path": path(&a.bridge_path),
    })
}

fn view_json(v: &View) -> Value {
    json!({
        "center": v.center,
        "nodes": v.nodes.iter().map(|n| json!({
            "id": n.id, "key": n.key, "label": n.label, "lang": n.lang, "kind": n.kind,
            "side": n.side.code(), "x": n.x, "y": n.y, "status": n.status, "note": n.note, "source": n.source
        })).collect::<Vec<_>>(),
        "edges": v.edges.iter().map(|e| json!({
            "id": e.id, "from": e.from, "to": e.to, "rel": e.rel, "rel_label": e.rel_label,
            "source": e.source, "source_label": e.source_label, "lang": e.lang, "method": e.method,
            "attrs": e.attrs.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(), "on_path": e.on_path
        })).collect::<Vec<_>>(),
    })
}

#[allow(dead_code)]
pub fn is_semantic(rel: &Rel) -> bool {
    matches!(rel, Rel::Wn(_) | Rel::HasSense)
}

/// Gjuha e nyjes për ekran: për konceptet ConceptNet merret nga URI-ja
/// (/c/<gjuha>/…), që asnjë gjuhë tjetër të mos paraqitet si shqip.
fn node_lang(kind: &NodeKind, lang: Lang) -> String {
    match kind {
        NodeKind::Concept { uri } => uri.split('/').nth(2).unwrap_or("und").to_string(),
        _ => lang.code().to_string(),
    }
}

/// Shton lemën te `data/babelnet_requests.txt` (pa dyfishim). Nuk bën asnjë thirrje rrjeti:
/// shkarkimi ndodh vetëm kur ekzekutohet `nura_fetch --babelnet`.
fn queue_for_babelnet(lemma: &str) {
    let lemma = lemma.trim().to_lowercase();
    match lemma.is_empty() || lemma.contains(char::is_whitespace) && lemma.split_whitespace().count() > 3 {
        true => return,
        false => {}
    }
    let path = default_data_dir().join("babelnet_requests.txt");
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    match old.lines().any(|l| l.trim() == lemma) {
        true => {}
        false => {
            let _ = std::fs::write(&path, format!("{old}{lemma}\n"));
        }
    }
}
