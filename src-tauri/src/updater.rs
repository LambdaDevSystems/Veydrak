//! Signed updates from GitHub Releases. Checking is automatic (if enabled), installing never is:
//! the frontend must call `install_update` after the user clicks.

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::state::AppState;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Serialize, Clone)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// Release notes come from the network; cap their size before they reach the UI.
fn trim_notes(notes: Option<String>) -> Option<String> {
    notes.map(|n| n.chars().take(2000).collect::<String>()).filter(|n| !n.trim().is_empty())
}

#[tauri::command]
pub async fn check_update(app: AppHandle, state: State<'_, AppState>) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    let info = update.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: trim_notes(u.body.clone()) });
    *state.pending_update.lock().map_err(|e| e.to_string())? = update;
    Ok(info)
}

#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let update = state
        .pending_update
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .ok_or("no update has been checked")?;

    let mut downloaded: u64 = 0;
    let progress_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress_app.emit("update-progress", Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_capped_and_blank_dropped() {
        assert_eq!(trim_notes(None), None);
        assert_eq!(trim_notes(Some("  \n".into())), None);
        assert_eq!(trim_notes(Some("x".repeat(5000))).unwrap().len(), 2000);
        assert_eq!(trim_notes(Some("fixes".into())), Some("fixes".into()));
    }
}
