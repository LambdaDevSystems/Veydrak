//! Home Assistant integration: a Rust-side WebSocket client feeding the `homeassistant` panel.
//! The access token and all network I/O stay in Rust; the webview only sees entity states.

pub mod client;
#[cfg(any(test, feature = "mock-ha"))]
pub mod mock;
pub mod protocol;
mod token;

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;

use crate::config;
use crate::state::AppState;
use client::{HaEvent, Status, StatusInfo};
use protocol::EntityState;

pub struct HaSnapshot {
    pub status: StatusInfo,
    pub entities: BTreeMap<String, EntityState>,
}

impl Default for HaSnapshot {
    fn default() -> Self {
        Self { status: StatusInfo::new(Status::NoConfig, None), entities: BTreeMap::new() }
    }
}

#[derive(Serialize)]
pub struct HaSnapshotView {
    status: StatusInfo,
    entities: Vec<EntityState>,
}

fn set_status(app: &AppHandle, info: StatusInfo) {
    if let Ok(mut snap) = app.state::<AppState>().ha.lock() {
        snap.status = info.clone();
    }
    let _ = app.emit("ha-status", info);
}

/// Start the client if `homeassistant.url` and at least one entity are configured.
pub fn start(app: &AppHandle) {
    let ha = config::get_config().config.homeassistant;
    let Some(raw) = ha.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else { return };

    let url = match protocol::ws_url(raw) {
        Ok(u) => u,
        Err(e) => {
            set_status(app, StatusInfo::new(Status::NoConfig, Some(format!("invalid homeassistant.url: {e}"))));
            return;
        }
    };
    let ids: HashSet<String> = ha.entities.iter().map(|e| e.id.clone()).collect();
    if ids.is_empty() {
        set_status(app, StatusInfo::new(Status::NoConfig, Some("homeassistant.entities is empty".into())));
        return;
    }

    let (tx, mut rx) = mpsc::channel::<HaEvent>(64);
    let notify = app.state::<AppState>().ha_notify.clone();
    let provider: client::TokenProvider = Arc::new(token::load);

    let forward = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                HaEvent::Status(info) => set_status(&forward, info),
                HaEvent::Entities(list) => {
                    if let Ok(mut snap) = forward.state::<AppState>().ha.lock() {
                        for e in &list {
                            snap.entities.insert(e.entity_id.clone(), e.clone());
                        }
                    }
                    let _ = forward.emit("ha-entities", list);
                }
            }
        }
    });
    tauri::async_runtime::spawn(client::run_loop(url, ids, tx, provider, notify, Duration::from_secs(1)));
}

#[tauri::command]
pub fn ha_snapshot(state: State<'_, AppState>) -> Result<HaSnapshotView, String> {
    let snap = state.ha.lock().map_err(|e| e.to_string())?;
    Ok(HaSnapshotView { status: snap.status.clone(), entities: snap.entities.values().cloned().collect() })
}

/// Save the access token (OS keyring) and wake the client. The token is never echoed back.
#[tauri::command]
pub async fn set_ha_token(state: State<'_, AppState>, token: String) -> Result<(), String> {
    let t = token::validate(&token)?;
    tokio::task::spawn_blocking(move || token::store(&t)).await.map_err(|e| e.to_string())??;
    state.ha_notify.notify_one();
    Ok(())
}
