#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod player;
mod updates;

use roadmap_app::{request_body, App, Error as ApiError};
use serde_json::Value;
use std::sync::Arc;
use tauri::{ipc::Channel, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn api(
    app: State<'_, Arc<App>>,
    method: String,
    path: String,
    body: Value,
) -> Result<Value, ApiError> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let body = request_body(body)?;
        app.route(&method, &path, &body)
    })
    .await
    .map_err(worker_error)?
}

#[tauri::command]
async fn chat_stream(
    app: State<'_, Arc<App>>,
    body: Value,
    on_piece: Channel<String>,
    on_status: Channel<Value>,
) -> Result<String, ApiError> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let body = request_body(body)?;
        app.chat_stream_with(
            &body,
            &mut |piece| {
                on_piece.send(piece.to_owned()).map_err(|e| ApiError {
                    status: 500,
                    message: e.to_string(),
                })
            },
            &mut |text, done| {
                let _ = on_status.send(serde_json::json!({"text":text,"done":done}));
            },
        )
    })
    .await
    .map_err(worker_error)?
}

fn worker_error(error: tauri::Error) -> ApiError {
    ApiError {
        status: 500,
        message: format!("Request worker failed: {error}"),
    }
}

fn main() {
    let mut context = tauri::generate_context!();
    // The lesson page frames this origin; without it the CSP placeholder allows no frames and the page says why.
    let player = match player::start() {
        Ok(origin) => {
            let security = &mut context.config_mut().app.security;
            let csp = match &security.csp {
                Some(tauri::utils::config::Csp::Policy(csp)) => csp.clone(),
                _ => String::new(),
            };
            let narrowed = player::app_csp(&csp, &origin)
                .unwrap_or_else(|| panic!("tauri.conf.json CSP lacks {}", player::CSP_PLACEHOLDER));
            security.csp = Some(tauri::utils::config::Csp::Policy(narrowed));
            origin
        }
        Err(error) => {
            eprintln!("video player server did not start: {error}");
            String::new()
        }
    };
    tauri::Builder::default()
        .append_invoke_initialization_script(format!(
            "Object.defineProperty(window, '__ROADMAP_PLAYER__', {{ value: {} }});",
            serde_json::Value::String(player)
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage(updates::Updates::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let roadmap = App::new(move |kind| {
                let dialog = handle.dialog().file();
                let picked = if kind == "folder" {
                    dialog
                        .set_title("Choose the project folder")
                        .blocking_pick_folder()
                } else {
                    dialog
                        .set_title("Choose a roadmap file")
                        .blocking_pick_file()
                };
                picked
                    .and_then(|file| file.into_path().ok())
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })?;
            let _ = player::VIDEOS.set(roadmap.data.join("videos"));
            // Points Claude Code, Codex, OpenCode and the other AI tools here at this copy's MCP server.
            let data = roadmap.data.clone();
            std::thread::spawn(move || roadmap_app::harness::keep_connected(&data));
            app.manage(roadmap);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api, chat_stream, updates::update_status, updates::update_check, updates::update_install])
        .run(context)
        .expect("run Roadmap");
}
