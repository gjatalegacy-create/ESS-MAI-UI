//! Ura me produktin ESS-MAI (codex_zvn): një kërkesë = një ekzekutim i veçantë i `light-platform`,
//! me gjendjen (handoff, vault, tmp) në dosjen e këtij aplikacioni, kurrë në atë të produktit.
//! Hyrja është një rresht ≤ 512 bajte, i shkruar saktë (pa rresht të ri) në një skedar; dalja
//! është rreshti JSON i Light-it dhe inspektimi i plotë i skedarit `.lgc`.
//!
//! Rrënja e produktit: `ESSMAI_PRODUCT_ROOT`, ndryshe `data/product_root.txt`, ndryshe shtegu i
//! njohur i codex_zvn. Asnjë skedar i produktit nuk ndryshohet.

use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_ROOT: &str = r"C:\Users\AG\Desktop\poc 007\obligation_promise_law_2_success\codex_zvn";
pub const INPUT_LIMIT: usize = 512;
const RUN_LIMIT: Duration = Duration::from_secs(180);

static SEQ: AtomicU64 = AtomicU64::new(0);

pub struct Product {
    root: PathBuf,
    state: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Receipt {
    pub operation: String,
    pub status: String,
    pub question: String,
    pub record: Value,
    pub light: Value,
    pub lgc: PathBuf,
    pub elapsed_ms: u128,
    pub exit: i32,
}

impl Receipt {
    pub fn json(&self) -> Value {
        json!({
            "operation": self.operation,
            "status": self.status,
            "question": self.question,
            "record": self.record,
            "light": self.light,
            "lgc": self.lgc.to_string_lossy(),
            "elapsed_ms": self.elapsed_ms as u64,
            "exit": self.exit,
        })
    }
}

fn exists(p: &Path) -> Result<(), String> {
    match p.is_file() {
        true => Ok(()),
        false => Err(format!("mungon skedari i produktit: {}", p.display())),
    }
}

impl Product {
    pub fn discover(data: &Path) -> Result<Product, String> {
        let from_file = std::fs::read_to_string(data.join("product_root.txt")).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let root = std::env::var("ESSMAI_PRODUCT_ROOT").ok().filter(|s| !s.trim().is_empty()).or(from_file).unwrap_or_else(|| DEFAULT_ROOT.to_string());
        let root = PathBuf::from(root);
        let p = Product { root, state: data.join("product_state") };
        for b in [p.light(), p.quantum(), p.shadow(), p.inspector()] {
            exists(&b)?;
        }
        for d in [p.state.join("handoff"), p.state.join("tmp"), p.state.join("cases")] {
            std::fs::create_dir_all(&d).map_err(|e| format!("dosja e gjendjes {}: {e}", d.display()))?;
        }
        Ok(p)
    }

    fn light(&self) -> PathBuf {
        self.root.join("bin").join("light-platform.exe")
    }
    fn quantum(&self) -> PathBuf {
        self.root.join("bin").join("quantum-platform.exe")
    }
    fn shadow(&self) -> PathBuf {
        self.root.join("bin").join("shadow_platform.exe")
    }
    fn inspector(&self) -> PathBuf {
        self.root.join("bin").join("examples").join("pd_iz_input.exe")
    }

    pub fn status(&self) -> Value {
        let cases = std::fs::read_dir(self.state.join("cases")).map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().extension().map(|x| x == "lgc").unwrap_or(false)).count()).unwrap_or(0);
        json!({
            "root": self.root.to_string_lossy(),
            "state": self.state.to_string_lossy(),
            "light": self.light().to_string_lossy(),
            "cases": cases,
            "input_limit": INPUT_LIMIT,
        })
    }

    /// Identifikues operacioni i veçantë për çdo ekzekutim (koha + numëruesi i procesit).
    pub fn operation(prefix: &str) -> String {
        let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}-{t}-{n}-{}", std::process::id())
    }

    fn env(&self, c: &mut Command) {
        let tmp = self.state.join("tmp");
        c.env("ESSMAI_VAULT", self.state.join("shadow.svlt"))
            .env("ESSMAI_HANDOFF_DIR", self.state.join("handoff"))
            .env("ESSMAI_QUANTUM_BIN", self.quantum())
            .env("ESSMAI_SHADOW_BIN", self.shadow())
            .env("TEMP", &tmp)
            .env("TMP", &tmp)
            .env_remove("ESSMAI_SHADOW_WORKER")
            .current_dir(&self.state);
        let path = std::env::var("PATH").unwrap_or_default();
        c.env("PATH", format!("{};{path}", self.root.join("bin").display()));
    }

    fn run(&self, exe: &Path, args: &[&std::ffi::OsStr]) -> Result<(String, String, i32), String> {
        let mut cmd = Command::new(exe);
        cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        self.env(&mut cmd);
        let mut child = cmd.spawn().map_err(|e| format!("nisja e {}: {e}", exe.display()))?;
        let mut out = child.stdout.take().ok_or("pa dalje")?;
        let mut err = child.stderr.take().ok_or("pa dalje gabimesh")?;
        let t_out = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        });
        let t_err = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        });
        let start = Instant::now();
        let code = loop {
            match child.try_wait() {
                Ok(Some(st)) => break st.code().unwrap_or(-1),
                Ok(None) => match start.elapsed() > RUN_LIMIT {
                    true => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("produkti kaloi kufirin e kohës (180 s); rezultati është i pasigurt".into());
                    }
                    false => std::thread::sleep(Duration::from_millis(40)),
                },
                Err(e) => return Err(format!("pritja e procesit: {e}")),
            }
        };
        let stdout = t_out.join().unwrap_or_default();
        let stderr = t_err.join().unwrap_or_default();
        Ok((stdout, stderr, code))
    }

    fn write_input(&self, op: &str, text: &str) -> Result<PathBuf, String> {
        let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        match (one.is_empty(), one.len() > INPUT_LIMIT) {
            (true, _) => return Err("hyrja është bosh".into()),
            (_, true) => return Err(format!("hyrja kalon {INPUT_LIMIT} bajte ({} bajte)", one.len())),
            _ => {}
        }
        let p = self.state.join("cases").join(format!("{op}.txt"));
        std::fs::write(&p, one.as_bytes()).map_err(|e| format!("shkrimi i hyrjes: {e}"))?;
        Ok(p)
    }

    fn last_json(stdout: &str) -> Value {
        stdout.lines().rev().find(|l| l.trim_start().starts_with('{')).and_then(|l| serde_json::from_str(l.trim()).ok()).unwrap_or(Value::Null)
    }

    fn inspect(&self, lgc: &Path) -> Result<Value, String> {
        let (out, err, code) = self.run(&self.inspector(), &["inspect-lgc-result".as_ref(), lgc.as_os_str()])?;
        match code {
            0 => serde_json::from_str(out.trim()).map_err(|e| format!("inspektimi: JSON i palexueshëm: {e}")),
            c => Err(format!("inspektimi dështoi (kodi {c}): {}", err.lines().last().unwrap_or("")))
        }
    }

    fn finish(&self, op: String, lgc: PathBuf, light_out: (String, String, i32), start: Instant) -> Result<Receipt, String> {
        let (stdout, stderr, exit) = light_out;
        let light = Self::last_json(&stdout);
        match exit {
            0 => {}
            c => return Err(explain(c, &stderr)),
        }
        let inspected = self.inspect(&lgc)?;
        let status = inspected.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let record = inspected.get("record").cloned().unwrap_or(Value::Null);
        let question = record.get("question").and_then(|v| v.as_str()).or_else(|| light.get("question").and_then(|v| v.as_str())).unwrap_or("").to_string();
        Ok(Receipt { operation: op, status, question, record, light, lgc, elapsed_ms: start.elapsed().as_millis(), exit })
    }

    /// Rast i ri: fjalia hyn si origjina i₀ e rastit.
    pub fn new_case(&self, prefix: &str, text: &str) -> Result<Receipt, String> {
        let op = Self::operation(prefix);
        let input = self.write_input(&op, text)?;
        let lgc = self.state.join("cases").join(format!("{op}.lgc"));
        let start = Instant::now();
        let out = self.run(&self.light(), &["--iz-lgc-new-once".as_ref(), op.as_ref(), input.as_os_str(), lgc.as_os_str()])?;
        self.finish(op, lgc, out, start)
    }

    /// Përgjigje brenda rastit ekzistues: referenca nxirret nga `.lgc` i mëparshëm.
    pub fn reply(&self, prefix: &str, previous_lgc: &Path, text: &str) -> Result<Receipt, String> {
        let op = Self::operation(prefix);
        let reference = previous_lgc.with_extension("ref");
        match reference.is_file() {
            true => {}
            false => {
                let (_, err, code) = self.run(&self.inspector(), &["export-lgc-result".as_ref(), previous_lgc.as_os_str(), reference.as_os_str()])?;
                match code {
                    0 => {}
                    c => return Err(format!("eksporti i referencës dështoi (kodi {c}): {}", err.lines().last().unwrap_or(""))),
                }
            }
        }
        let input = self.write_input(&op, text)?;
        let lgc = self.state.join("cases").join(format!("{op}.lgc"));
        let start = Instant::now();
        let out = self.run(&self.light(), &["--iz-lgc-reply-once".as_ref(), op.as_ref(), reference.as_os_str(), input.as_os_str(), lgc.as_os_str()])?;
        self.finish(op, lgc, out, start)
    }
}

/// Kodi i daljes së Light-it, me fjalë të thjeshta; shenja e tij (LIGHT_…/PD_…) ruhet për gjurmim.
fn explain(code: i32, stderr: &str) -> String {
    let tag = stderr
        .lines()
        .rev()
        .find_map(|l| l.split_whitespace().map(|t| t.trim_matches(['[', ']', ':', ','])).find(|t| t.starts_with("LIGHT_") || t.starts_with("PD_")))
        .unwrap_or("");
    match tag {
        "LIGHT_IZ_TERRITORY_MISMATCH" => "Light e vendosi këtë tekst në territor tjetër nga rasti i hapur (fjalët e tij e ndryshojnë rrugën), prandaj nuk e pranoi si vazhdim (LIGHT_IZ_TERRITORY_MISMATCH).".into(),
        "LIGHT_LGC_INPUT_NOT_EXACT" => "Light kërkon tekstin saktë, një rresht pa hapësira në fund (LIGHT_LGC_INPUT_NOT_EXACT).".into(),
        "LIGHT_LGC_OUTPUT_EXISTS" => "Ky operacion është ekzekutuar tashmë (LIGHT_LGC_OUTPUT_EXISTS).".into(),
        "" => format!("Light doli me kodin {code}"),
        t => format!("Light doli me kodin {code} ({t})"),
    }
}

/// Si ia thotë Nura përdoruesit përgjigjen e produktit, me fjalë të thjeshta.
pub fn plain(r: &Receipt) -> String {
    let turn = r.record.get("session_turn_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let gaps = r.record.get("dbab_gap_count").and_then(|v| v.as_u64());
    match r.status.as_str() {
        "OperationalNoVerdict" => {
            let missing = match gaps {
                Some(0) => "nuk i mungon material".to_string(),
                Some(n) => format!("i mungojnë ende {n} anë të provës"),
                None => "nuk e tregon sa i mungon".to_string(),
            };
            format!("Sistemi e mori (kthesa {turn}), e mban pezull pa verdikt dhe {missing}. Kërkon: material pozitiv, material negativ, dallimin Xi/Yi, burimin dhe kushtin që do ta rrëzonte.")
        }
        "NativeTerminal" => format!("Sistemi e mbylli rastin me verdikt (kthesa {turn})."),
        other => format!("Sistemi u përgjigj me gjendjen «{other}» (kthesa {turn})."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_ids_are_unique_and_prefixed() {
        let a = Product::operation("ui");
        let b = Product::operation("ui");
        assert!(a.starts_with("ui-") && a != b);
    }

    #[test]
    fn last_json_line_is_taken() {
        let v = Product::last_json("noise\n{\"kind\":\"OperationalNoVerdict\",\"question\":\"q\"}\nlog");
        assert_eq!(v["kind"], "OperationalNoVerdict");
        assert_eq!(Product::last_json("nothing"), Value::Null);
    }
}
