use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::lan::roster::Roster;
use crate::pty::manager::PtyInstance;

pub struct AppState {
    /// Update found by the last `check_update`, consumed by `install_update`.
    pub pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
    pub ptys: Mutex<HashMap<String, PtyInstance>>,
    pub roster: Arc<Mutex<Roster>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pending_update: Mutex::new(None),
            ptys: Mutex::new(HashMap::new()),
            roster: Arc::new(Mutex::new(Roster::default())),
        }
    }
}
