use std::{sync::Mutex, time::Duration};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Default, Serialize)]
pub struct Status { pub version: String, pub state: String, pub available: String }
#[derive(Default)]
pub struct Updates { status: Mutex<Status>, pending: Mutex<Option<(Update, Vec<u8>)>> }
fn supported(bundle: Option<tauri::utils::config::BundleType>, appimage: bool) -> bool {
    use tauri::utils::config::BundleType::*;
    // Windows builds ship in the Inno Setup installer, which Tauri doesn't tag with a bundle type.
    // The released AppImage isn't tagged either (1.0.0 showed "Download"); the updater still
    // replaces the file named by APPIMAGE, so a running AppImage counts as updatable.
    cfg!(windows) || appimage || matches!(bundle, Some(Nsis | AppImage | Deb | Rpm))
}
/// Running from a mounted AppImage, the same test Tauri uses before trusting APPIMAGE.
fn in_appimage() -> bool {
    std::env::var_os("APPIMAGE").is_some()
        && std::env::current_exe().is_ok_and(|p| p.starts_with(std::env::temp_dir()) && p.to_string_lossy().contains("/.mount_"))
}
#[tauri::command]
pub fn update_status(app: AppHandle, state: State<Updates>) -> Status {
    let mut status = state.status.lock().unwrap().clone();
    status.version = app.package_info().version.to_string(); status
}
#[tauri::command]
pub async fn update_check(app: AppHandle, state: State<'_, Updates>) -> Result<(), String> {
    // MSIX (Store) installs live under WindowsApps; the Store updates them, so never self-update there.
    if std::env::current_exe().is_ok_and(|p| p.to_string_lossy().contains("\\WindowsApps\\")) {
        state.status.lock().unwrap().state = "latest".into(); return Ok(());
    }
    { let mut s = state.status.lock().unwrap(); if s.state == "downloading" || s.state == "ready" { return Ok(()); } s.state = "downloading".into(); }
    let result = async {
        // Inno Setup ignores the NSIS flags Tauri adds; these make it update without prompts.
        let updater = app.updater_builder().timeout(Duration::from_secs(120))
            .installer_args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]).build().map_err(|e| e.to_string())?;
        let update = updater.check().await.map_err(|e| e.to_string())?;
        if let Some(update) = update {
            let version = update.version.clone();
            // Notify once per version; the 6-hour recheck must not repeat it.
            let new = std::mem::replace(&mut state.status.lock().unwrap().available, version.clone()) != version;
            let body = if !cfg!(debug_assertions) && supported(tauri::utils::platform::bundle_type(), in_appimage()) {
                let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
                *state.pending.lock().unwrap() = Some((update, bytes));
                state.status.lock().unwrap().state = "ready".into();
                "Restart Roadmap to update."
            } else { state.status.lock().unwrap().state = "download".into(); "Open Roadmap to download it." };
            if new {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().builder().title(format!("Roadmap {version} is available")).body(body).show();
            }
        } else { state.status.lock().unwrap().state = "latest".into(); }
        Ok::<(), String>(())
    }.await;
    if let Err(error) = &result {
        eprintln!("update check: {error}");
        let mut s = state.status.lock().unwrap(); s.state = "error".into(); s.available.clear();
    }
    result
}
#[tauri::command]
pub async fn update_install(app: AppHandle, state: State<'_, Updates>) -> Result<(), String> {
    let pending = state.pending.lock().unwrap().take().ok_or("No update is ready")?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        pending.0.install(&pending.1).map_err(|e| e.to_string())?; Ok::<(), String>(())
    }).await.map_err(|e| e.to_string())?;
    if let Err(error) = result { state.status.lock().unwrap().state = "error".into(); return Err(error); }
    app.restart();
}
#[cfg(test)]
mod tests {
    #[test] fn self_updatable_packages() {
        assert_eq!(super::supported(None, false), cfg!(windows));
        assert!(super::supported(None, true), "an untagged AppImage still updates itself");
        assert!(!super::in_appimage(), "tests do not run from an AppImage");
        use tauri::utils::config::BundleType::*;
        for bundle in [Nsis, AppImage, Deb, Rpm] { assert!(super::supported(Some(bundle), false)); }
        assert_eq!(super::supported(Some(Dmg), false), cfg!(windows));
    }
}
