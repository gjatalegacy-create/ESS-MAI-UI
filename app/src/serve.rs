//! Shërbim lokal minimal (vetëm 127.0.0.1) — e njëjta UI dhe i njëjti dispeçer
//! si dritarja Tauri, për verifikim në shfletues. Pa varësi të jashtme.
//!   GET  /, /app.js, /app.css, /assets/*   → skedarët e `dist`
//!   POST /api/<komanda>  (JSON)             → `App::dispatch`
//! Roja: çdo kërkesë me Host të huaj, me Origin të huaj, ose POST pa
//! `Content-Type: application/json` merr 403 (mbrojtje nga faqet e tjera
//! në shfletues dhe nga DNS rebinding). Pa Origin (curl, hapja e faqes) lejohet.

use crate::api::App;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

fn dist_dir() -> PathBuf {
    match std::env::var("UI_ESS_MAI_DIST") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist"),
    }
}

pub fn run(app: Arc<App>, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    // Porti i vërtetë (edhe kur jepet 0): roja krahason Host/Origin me këtë.
    let port = listener.local_addr()?.port();
    println!("[ui_ess_mai] shërbimi lokal: http://127.0.0.1:{port}/  (Ctrl+C për ndalim)");
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let a = app.clone();
                std::thread::spawn(move || {
                    let _ = handle(s, &a, port);
                });
            }
            Err(e) => eprintln!("[ui_ess_mai] lidhje e dështuar: {e}"),
        }
    }
    Ok(())
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn respond(s: &mut TcpStream, code: &str, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        s,
        "HTTP/1.1 {code}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    s.write_all(body)?;
    s.flush()
}

/// Autoritetet (host:port) e lejuara: vetëm adresa jonë lokale.
/// Për portin 80 shfletuesi e heq portin, ndaj lejohet edhe pa të.
fn autoritete(port: u16) -> Vec<String> {
    let baze = vec![format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    match port {
        80 => [baze, vec!["127.0.0.1".to_string(), "localhost".to_string()]].concat(),
        _ => baze,
    }
}

/// Emri i hostit nuk varet nga shkronjat e mëdha/vogla; pjesa tjetër duhet saktë.
fn autoritet_i_lejuar(a: &str, port: u16) -> bool {
    autoritete(port).iter().any(|x| x.eq_ignore_ascii_case(a))
}

/// Host: saktësisht një kokë dhe vetëm adresa jonë (mbrojtje nga DNS rebinding).
fn host_i_lejuar(host: &[&str], port: u16) -> bool {
    match host {
        [h] => autoritet_i_lejuar(h, port),
        _ => false,
    }
}

/// Origin: mungesa lejohet (curl, hapja e faqes); përndryshe vetëm faqja jonë
/// me http://, një herë. "null", https ose port tjetër refuzohen.
fn origin_i_lejuar(origin: &[&str], port: u16) -> bool {
    match origin {
        [] => true,
        [o] => match o.strip_prefix("http://") {
            Some(a) => autoritet_i_lejuar(a, port),
            None => false,
        },
        _ => false,
    }
}

/// POST pranon vetëm JSON (parametrat si `; charset=utf-8` lejohen). Kjo e
/// detyron shfletuesin të bëjë preflight për çdo faqe të huaj, e cila dështon.
fn lloji_i_lejuar(method: &str, ctype: &[&str]) -> bool {
    match (method, ctype) {
        ("POST", [c]) => c
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/json"),
        ("POST", _) => false,
        _ => true,
    }
}

/// Roja e kërkesës, funksion i pastër: Ok = shërbe, Err(arsye) = 403.
fn roja(
    method: &str,
    host: &[&str],
    origin: &[&str],
    ctype: &[&str],
    port: u16,
) -> Result<(), &'static str> {
    match (
        host_i_lejuar(host, port),
        origin_i_lejuar(origin, port),
        lloji_i_lejuar(method, ctype),
    ) {
        (true, true, true) => Ok(()),
        (false, _, _) => Err("e ndaluar: host i palejuar"),
        (true, false, _) => Err("e ndaluar: origjine e palejuar"),
        (true, true, false) => Err("e ndaluar: kerkohet Content-Type: application/json"),
    }
}

/// Pamje `&str` e kokave të mbledhura, për rojën.
fn si_str(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

fn handle(mut s: TcpStream, app: &App, port: u16) -> std::io::Result<()> {
    let mut reader = BufReader::new(s.try_clone()?);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    let mut len = 0usize;
    // Kokat e rojës mbahen si lista, që një kokë e dyfishtë të refuzohet.
    let mut host: Vec<String> = Vec::new();
    let mut origin: Vec<String> = Vec::new();
    let mut ctype: Vec<String> = Vec::new();
    loop {
        let mut h = String::new();
        reader.read_line(&mut h)?;
        let t = h.trim_end();
        match t.is_empty() {
            true => break,
            false => match t.split_once(':') {
                Some((k, v)) => {
                    let v = v.trim();
                    match k.trim().to_ascii_lowercase().as_str() {
                        "content-length" => len = v.parse().unwrap_or(0),
                        "host" => host.push(v.to_string()),
                        "origin" => origin.push(v.to_string()),
                        "content-type" => ctype.push(v.to_string()),
                        _ => {}
                    }
                }
                None => {}
            },
        }
    }
    match roja(&method, &si_str(&host), &si_str(&origin), &si_str(&ctype), port) {
        Ok(()) => sherbe(&mut s, &mut reader, app, &method, &path, len),
        // Trupi nuk lexohet fare: kërkesa e huaj nuk prek dispeçerin.
        Err(arsye) => respond(&mut s, "403 Forbidden", "text/plain; charset=utf-8", arsye.as_bytes()),
    }
}

/// Shërbimi pas rojës: API ose skedarët e `dist`.
fn sherbe(
    s: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    app: &App,
    method: &str,
    path: &str,
    len: usize,
) -> std::io::Result<()> {
    match (method, path.strip_prefix("/api/")) {
        ("POST", Some(cmd)) => {
            let mut body = vec![0u8; len.min(1 << 20)];
            reader.read_exact(&mut body)?;
            let args: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));
            let out = match app.dispatch(cmd, &args) {
                Ok(v) => serde_json::json!({ "ok": true, "data": v }),
                Err(e) => serde_json::json!({ "ok": false, "error": e }),
            };
            respond(s, "200 OK", "application/json; charset=utf-8", out.to_string().as_bytes())
        }
        ("GET", _) => {
            let rel = match path {
                "/" => "index.html".to_string(),
                p => p.trim_start_matches('/').to_string(),
            };
            match rel.contains("..") || rel.contains('\\') {
                true => respond(s, "400 Bad Request", "text/plain", b"rruge e palejuar"),
                false => match std::fs::read(dist_dir().join(&rel)) {
                    Ok(b) => respond(s, "200 OK", content_type(&rel), &b),
                    Err(_) => respond(s, "404 Not Found", "text/plain", b"nuk u gjet"),
                },
            }
        }
        _ => respond(s, "405 Method Not Allowed", "text/plain", b""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: u16 = 7878;

    #[test]
    fn faqja_pa_origin_lejohet() {
        assert_eq!(roja("GET", &["127.0.0.1:7878"], &[], &[], P), Ok(()));
        assert_eq!(roja("GET", &["localhost:7878"], &[], &[], P), Ok(()));
        // Emri i hostit pa dallim shkronjash.
        assert_eq!(roja("GET", &["LocalHost:7878"], &[], &[], P), Ok(()));
    }

    #[test]
    fn post_json_nga_faqja_jone_lejohet() {
        let ct = ["application/json"];
        assert_eq!(roja("POST", &["127.0.0.1:7878"], &["http://127.0.0.1:7878"], &ct, P), Ok(()));
        assert_eq!(roja("POST", &["localhost:7878"], &["http://localhost:7878"], &ct, P), Ok(()));
        // curl pa Origin, me charset.
        assert_eq!(roja("POST", &["127.0.0.1:7878"], &[], &["application/json; charset=utf-8"], P), Ok(()));
    }

    #[test]
    fn host_i_huaj_ose_mungese_refuzohet() {
        // DNS rebinding: emri i sulmuesit drejtohet te 127.0.0.1.
        assert!(roja("GET", &["evil.example:7878"], &[], &[], P).is_err());
        assert!(roja("GET", &["127.0.0.1:9999"], &[], &[], P).is_err());
        assert!(roja("GET", &["127.0.0.1"], &[], &[], P).is_err());
        assert!(roja("GET", &[], &[], &[], P).is_err());
        assert!(roja("GET", &["127.0.0.1:7878", "evil.example"], &[], &[], P).is_err());
    }

    #[test]
    fn origin_i_huaj_refuzohet() {
        let h = ["127.0.0.1:7878"];
        let ct = ["application/json"];
        for o in [
            "http://evil.example",
            "null",
            "https://127.0.0.1:7878",
            "http://127.0.0.1:9999",
            "http://127.0.0.1:7878/",
            "http://user@127.0.0.1:7878",
            "",
        ] {
            assert!(roja("POST", &h, &[o], &ct, P).is_err(), "{o}");
            assert!(roja("GET", &h, &[o], &[], P).is_err(), "{o}");
        }
        // Origin i dyfishtë nuk pranohet.
        assert!(roja("POST", &h, &["http://127.0.0.1:7878", "http://evil.example"], &ct, P).is_err());
    }

    #[test]
    fn post_pa_json_refuzohet() {
        let h = ["127.0.0.1:7878"];
        let o = ["http://127.0.0.1:7878"];
        // Kërkesat "simple" të CORS (pa preflight).
        assert!(roja("POST", &h, &o, &["text/plain"], P).is_err());
        assert!(roja("POST", &h, &o, &["text/plain;charset=UTF-8"], P).is_err());
        assert!(roja("POST", &h, &o, &["application/x-www-form-urlencoded"], P).is_err());
        assert!(roja("POST", &h, &o, &["multipart/form-data; boundary=x"], P).is_err());
        assert!(roja("POST", &h, &o, &["application/jsonx"], P).is_err());
        assert!(roja("POST", &h, &o, &[], P).is_err());
        assert!(roja("POST", &h, &o, &["application/json", "text/plain"], P).is_err());
    }

    #[test]
    fn porti_80_lejohet_pa_port() {
        assert_eq!(roja("GET", &["localhost"], &["http://localhost"], &[], 80), Ok(()));
        assert_eq!(roja("GET", &["127.0.0.1:80"], &[], &[], 80), Ok(()));
        assert!(roja("GET", &["localhost"], &[], &[], P).is_err());
    }

    #[test]
    fn arsyeja_e_pare_eshte_host() {
        assert_eq!(
            roja("POST", &["evil.example"], &["http://evil.example"], &["text/plain"], P),
            Err("e ndaluar: host i palejuar")
        );
        assert_eq!(
            roja("POST", &["127.0.0.1:7878"], &["http://evil.example"], &["text/plain"], P),
            Err("e ndaluar: origjine e palejuar")
        );
    }

    #[test]
    fn kokat_kthehen_ne_str() {
        let v = vec!["a".to_string(), "b".to_string()];
        assert_eq!(si_str(&v), vec!["a", "b"]);
    }
}
