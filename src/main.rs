#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
) -> Result<String, ApiError> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let body = request_body(body)?;
        app.chat_stream(&body, &mut |piece| {
            on_piece.send(piece.to_owned()).map_err(|e| ApiError {
                status: 500,
                message: e.to_string(),
            })
        })
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(App::new(move |kind| {
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
            })?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api, chat_stream])
        .run(tauri::generate_context!())
        .expect("run Roadmap");
}
