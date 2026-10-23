//! nura_probe — provë e rrjedhës së plotë në terminal me indeksin real.
//! Përdorimi: nura_probe "A është qeni kafshë?" ["hyrje tjetër" …]

use nura_lingua::emotion::{EmotionMachine, Register};
use nura_lingua::expression::{realize, signature};
use nura_lingua::query::Engine;
use nura_lingua::store::{conceptnet_partial, default_data_dir, index_path, load_index, read_lock, sq_lexicon};

fn main() {
    let t0 = std::time::Instant::now();
    let b = match load_index(&index_path(&default_data_dir())) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("indeksi: {e}");
            std::process::exit(2);
        }
    };
    let lex = sq_lexicon(&b.graph);
    println!("[indeksi u ngarkua në {} ms]", t0.elapsed().as_millis());
    let partial = read_lock(&default_data_dir()).map(|l| conceptnet_partial(&l)).unwrap_or(true);
    let eng = Engine::new(&b.graph, &b.morph, &lex).with_conceptnet_partial(partial);
    let mut em = EmotionMachine::new();
    for input in std::env::args().skip(1) {
        println!("\n══ {input}");
        match eng.analyze(&input) {
            Ok(a) => {
                println!("forma: {}", a.form.code());
                for s in &a.steps {
                    let t = s.event.and_then(|e| em.apply(e));
                    println!(
                        "  [{}] {}{}",
                        s.code,
                        s.label,
                        t.map(|t| format!("   ⇒ {} → {} ({})", t.from.code(), t.to.code(), t.rule)).unwrap_or_default()
                    );
                }
                match &a.focus {
                    Some(f) => {
                        for s in &f.senses {
                            println!("    Yi {} {:?} sq={:?} gl={:?} {:?}", s.synset, s.pos, s.labels_sq, s.gloss_sq.as_deref().map(|g| g.chars().take(60).collect::<String>()), s.status);
                        }
                        println!("    Xi usages={} form_rels={}", f.usages.len(), f.form_rels.len());
                    }
                    None => {}
                }
                let x = em.expressive();
                let r = realize(&a.plan, &b.morph, x.register);
                println!("  faza={} regjistri={} ngjyra={} animacioni={} ritmi={}ms", x.phase.code(), x.register.code(), x.color, x.animation, x.tempo_ms);
                println!("  NURA › {}", r.full);
                for f in &r.forms {
                    println!("    forma «{}»: {}", f.surface, f.origin);
                }
                let base = realize(&a.plan, &b.morph, Register::Neutral).core_text;
                let all_same = Register::ALL.iter().all(|reg| realize(&a.plan, &b.morph, *reg).core_text == base);
                println!("  thelbi i njëjtë në 7 regjistra: {all_same}; nënshkrimi: {:?}", signature(&a.plan).claims);
            }
            Err(e) => println!("  hyrje e pambështetur: {e}"),
        }
    }
}
