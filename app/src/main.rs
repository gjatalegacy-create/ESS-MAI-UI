//! ui_ess_mai — Nura Legacy, UI emocionale me grafe fjalësh.
//!
//! Dy mënyra nisjeje, i njëjti dispeçer dhe e njëjta gjendje:
//! - parazgjedhje: dritare Tauri 2 (si UI-të origjinale të ESS-MAI);
//! - `--serve [port]`: shërbim lokal vetëm në 127.0.0.1 për verifikim në shfletues.
//!
//! Kufiri: kjo UI nuk thërret Light/Quantum/Shadow dhe nuk krijon verdict,
//! token ose GCL. Ajo punon vetëm mbi të dhënat gjuhësore lokale të importuara.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;

#[allow(dead_code)]
#[path = "../../inherited/emotional_command.rs"]
mod emotional_command;
mod product;

mod serve;

use std::sync::Arc;

#[tauri::command]
fn nura(state: tauri::State<'_, Arc<api::App>>, cmd: String, args: serde_json::Value) -> Result<serde_json::Value, String> {
    state.dispatch(&cmd, &args)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let app = Arc::new(api::App::load());
    match args.iter().position(|a| a == "--serve") {
        Some(i) => {
            let port: u16 = args.get(i + 1).and_then(|p| p.parse().ok()).unwrap_or(7878);
            match serve::run(app, port) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("[ui_ess_mai] shërbimi lokal dështoi: {e}");
                    std::process::exit(2);
                }
            }
        }
        None => {
            let ticker = app.clone();
            match tauri::Builder::default()
                .manage(app)
                .invoke_handler(tauri::generate_handler![nura])
                .setup(move |_| {
                    let _ = &ticker;
                    Ok(())
                })
                .run(tauri::generate_context!())
            {
                Ok(()) => {}
                Err(e) => eprintln!("[ui_ess_mai] Tauri 2 dështoi: {e}"),
            }
        }
    }
}
