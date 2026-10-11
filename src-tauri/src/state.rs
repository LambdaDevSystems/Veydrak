use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::lan::roster::Roster;
use crate::pty::manager::PtyInstance;

pub struct AppState {
    pub ptys: Mutex<HashMap<String, PtyInstance>>,
    pub roster: Arc<Mutex<Roster>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            ptys: Mutex::new(HashMap::new()),
            roster: Arc::new(Mutex::new(Roster::default())),
        }
    }
}
