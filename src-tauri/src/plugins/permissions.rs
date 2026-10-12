//! Persisted permission grants. A plugin runs only if every permission its manifest requests has been
//! granted; if an update asks for more, it needs approval again.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use super::manifest::{Manifest, Permission};

const STORE_FILE: &str = "plugins.json";
const GRANTS_KEY: &str = "grants";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Granted,
    NeedsApproval,
}

/// Granted permission strings per plugin id.
#[derive(Default, Debug, PartialEq)]
pub struct Grants(BTreeMap<String, BTreeSet<String>>);

impl Grants {
    pub fn status(&self, m: &Manifest) -> Status {
        let granted = self.0.get(&m.id);
        let all = m.permissions.iter().all(|p| granted.is_some_and(|g| g.contains(&p.as_string())));
        if all { Status::Granted } else { Status::NeedsApproval }
    }

    /// Whether `perm` may be used right now; hosts check this before every plugin call.
    pub fn allows(&self, m: &Manifest, perm: &Permission) -> bool {
        m.permissions.contains(perm) && self.0.get(&m.id).is_some_and(|g| g.contains(&perm.as_string()))
    }

    /// Record approval of exactly what the manifest currently requests, dropping stale grants.
    pub fn grant(&mut self, m: &Manifest) {
        self.0.insert(m.id.clone(), m.permissions.iter().map(Permission::as_string).collect());
    }

    pub fn revoke(&mut self, id: &str) {
        self.0.remove(id);
    }
}

pub fn load(app: &AppHandle) -> Grants {
    let Ok(store) = app.store(STORE_FILE) else { return Grants::default() };
    let map = store.get(GRANTS_KEY).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    Grants(map)
}

pub fn save(app: &AppHandle, grants: &Grants) {
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(GRANTS_KEY, json!(grants.0));
        let _ = store.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::parse;

    fn manifest(perms: &str) -> Manifest {
        parse(&format!(r#"{{"id":"demo","name":"Demo","version":"1","permissions":{perms}}}"#)).unwrap()
    }

    #[test]
    fn no_permissions_needs_no_approval() {
        assert_eq!(Grants::default().status(&manifest("[]")), Status::Granted);
    }

    #[test]
    fn ungranted_needs_approval_then_granted() {
        let m = manifest(r#"["storage","ha.read"]"#);
        let mut g = Grants::default();
        assert_eq!(g.status(&m), Status::NeedsApproval);
        assert!(!g.allows(&m, &Permission::Storage));
        g.grant(&m);
        assert_eq!(g.status(&m), Status::Granted);
        assert!(g.allows(&m, &Permission::Storage));
    }

    #[test]
    fn extra_permission_in_update_reprompts() {
        let mut g = Grants::default();
        g.grant(&manifest(r#"["storage"]"#));
        let updated = manifest(r#"["storage","net:api.example.com"]"#);
        assert_eq!(g.status(&updated), Status::NeedsApproval);
        assert!(g.allows(&updated, &Permission::Storage));
        assert!(!g.allows(&updated, &Permission::Net("api.example.com".into())));
    }

    #[test]
    fn dropped_permission_stays_granted_but_unusable() {
        let mut g = Grants::default();
        g.grant(&manifest(r#"["storage","ha.read"]"#));
        let updated = manifest(r#"["storage"]"#);
        assert_eq!(g.status(&updated), Status::Granted);
        assert!(!g.allows(&updated, &Permission::HaRead));
    }

    #[test]
    fn revoke_clears_grants() {
        let m = manifest(r#"["storage"]"#);
        let mut g = Grants::default();
        g.grant(&m);
        g.revoke("demo");
        assert_eq!(g.status(&m), Status::NeedsApproval);
    }
}
