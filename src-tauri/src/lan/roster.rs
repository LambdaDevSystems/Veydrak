use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::state::AppState;

const STORE_FILE: &str = "roster.json";
const PINS_KEY: &str = "pins";

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Online,
    Offline,
    Unknown,
}

#[derive(Serialize, Clone, Debug)]
pub struct Device {
    pub ip: String,
    pub name: String,
    pub status: Status,
    pub pinned: bool,
}

#[derive(Default)]
pub struct Roster {
    pub devices: BTreeMap<IpAddr, Device>,
    pub pins: HashSet<String>,
}

impl Roster {
    pub fn snapshot(&self) -> Vec<Device> {
        self.devices.values().cloned().collect()
    }

    pub fn toggle_pin(&mut self, ip: &str) -> bool {
        let pinned = if self.pins.remove(ip) { false } else { self.pins.insert(ip.to_string()) };
        if let Ok(addr) = ip.parse::<IpAddr>() {
            if let Some(d) = self.devices.get_mut(&addr) {
                d.pinned = pinned;
            }
        }
        pinned
    }
}

pub fn load_pins(app: &AppHandle) {
    let Ok(store) = app.store(STORE_FILE) else { return };
    let pins: Vec<String> = store.get(PINS_KEY).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    if let Ok(mut r) = app.state::<AppState>().roster.lock() {
        r.pins = pins.into_iter().filter(|p| p.parse::<IpAddr>().is_ok()).collect();
    }
}

fn save_pins(app: &AppHandle, pins: &HashSet<String>) {
    if let Ok(store) = app.store(STORE_FILE) {
        let mut v: Vec<&String> = pins.iter().collect();
        v.sort();
        store.set(PINS_KEY, json!(v));
        let _ = store.save();
    }
}

#[tauri::command]
pub fn get_roster(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    Ok(state.roster.lock().map_err(|e| e.to_string())?.snapshot())
}

#[tauri::command]
pub fn toggle_pin(app: AppHandle, state: State<'_, AppState>, ip: String) -> Result<(), String> {
    if ip.parse::<IpAddr>().is_err() {
        return Err("invalid ip".into());
    }
    let (snapshot, pins) = {
        let mut r = state.roster.lock().map_err(|e| e.to_string())?;
        r.toggle_pin(&ip);
        (r.snapshot(), r.pins.clone())
    };
    save_pins(&app, &pins);
    let _ = app.emit("lan-roster", snapshot);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_pin_flips() {
        let mut r = Roster::default();
        assert!(r.toggle_pin("10.0.0.2"));
        assert!(!r.toggle_pin("10.0.0.2"));
    }
}
