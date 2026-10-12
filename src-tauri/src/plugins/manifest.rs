//! `manifest.json` parsing and validation. Anything unexpected rejects the whole plugin.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use serde::Deserialize;

use crate::pty::manager::valid_id;

pub const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const MAX_NAME_CHARS: usize = 40;
const MAX_VERSION_CHARS: usize = 32;
const MAX_ENTRY_CHARS: usize = 128;
const MAX_PERMISSIONS: usize = 32;

/// A capability a plugin can request. Serialized as its manifest string (`net:<host>` for network).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    SysmonRead,
    RosterRead,
    HaRead,
    Storage,
    /// Exact hostname the host may fetch on the plugin's behalf; no wildcards.
    Net(String),
}

impl Permission {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "sysmon.read" => Ok(Self::SysmonRead),
            "roster.read" => Ok(Self::RosterRead),
            "ha.read" => Ok(Self::HaRead),
            "storage" => Ok(Self::Storage),
            _ => match s.strip_prefix("net:") {
                Some(host) if valid_host(host) => Ok(Self::Net(host.to_string())),
                Some(_) => Err(format!("invalid net host in permission {s:?}")),
                None => Err(format!("unknown permission {s:?}")),
            },
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::SysmonRead => "sysmon.read".into(),
            Self::RosterRead => "roster.read".into(),
            Self::HaRead => "ha.read".into(),
            Self::Storage => "storage".into(),
            Self::Net(h) => format!("net:{h}"),
        }
    }
}

/// Plain lowercase DNS names only: no schemes, ports, paths, wildcards or IP literals. A numeric last
/// label rules out dotted-quad and decimal/hex-style IPv4 forms (`127.0.0.1`, `2130706433`); IPv6 fails on `:`.
fn valid_host(h: &str) -> bool {
    let numeric_tld = h.rsplit('.').next().is_some_and(|l| l.chars().all(|c| c.is_ascii_digit()));
    !h.is_empty()
        && !numeric_tld
        && h.len() <= 253
        && !h.starts_with(['.', '-'])
        && !h.ends_with(['.', '-'])
        && !h.contains("..")
        && h.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    id: String,
    name: String,
    version: String,
    #[serde(default = "default_entry")]
    entry: String,
    #[serde(default)]
    permissions: Vec<String>,
}

fn default_entry() -> String {
    "index.html".into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: String,
    pub permissions: BTreeSet<Permission>,
}

/// The entry must be a plain relative `.html` path inside the plugin folder.
fn valid_entry(entry: &str) -> bool {
    if entry.is_empty() || entry.len() > MAX_ENTRY_CHARS || entry.contains(['\\', '\0', ':']) || !entry.ends_with(".html") {
        return false;
    }
    Path::new(entry).components().all(|c| matches!(c, Component::Normal(_)))
}

/// Control characters plus zero-width and bidi format characters, which could spoof the name shown in
/// the permission prompt.
fn has_control(s: &str) -> bool {
    s.chars().any(|c| {
        c.is_control()
            || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
    })
}

pub fn parse(text: &str) -> Result<Manifest, String> {
    let raw: RawManifest = serde_json::from_str(text).map_err(|e| format!("invalid manifest: {e}"))?;
    if !valid_id(&raw.id) {
        return Err("invalid plugin id".into());
    }
    let name = raw.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || has_control(name) {
        return Err("invalid plugin name".into());
    }
    if raw.version.is_empty()
        || raw.version.len() > MAX_VERSION_CHARS
        || !raw.version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
    {
        return Err("invalid plugin version".into());
    }
    if !valid_entry(&raw.entry) {
        return Err("invalid entry path".into());
    }
    if raw.permissions.len() > MAX_PERMISSIONS {
        return Err("too many permissions".into());
    }
    let permissions = raw.permissions.iter().map(|p| Permission::parse(p)).collect::<Result<BTreeSet<_>, _>>()?;
    Ok(Manifest { id: raw.id, name: name.to_string(), version: raw.version, entry: raw.entry, permissions })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(field: &str, value: &str) -> String {
        let mut m = serde_json::json!({"id": "demo", "name": "Demo", "version": "1.0.0"});
        m[field] = serde_json::from_str(value).unwrap();
        m.to_string()
    }

    #[test]
    fn minimal_manifest_gets_defaults() {
        let m = parse(r#"{"id":"demo","name":"Demo","version":"1.0.0"}"#).unwrap();
        assert_eq!(m.entry, "index.html");
        assert!(m.permissions.is_empty());
    }

    #[test]
    fn parses_all_permissions_and_roundtrips() {
        let m = parse(&with("permissions", r#"["sysmon.read","roster.read","ha.read","storage","net:api.example.com"]"#)).unwrap();
        assert_eq!(m.permissions.len(), 5);
        assert!(parse(&with("permissions", r#"["net:localhost","net:my-nas.local"]"#)).is_ok());
        for p in &m.permissions {
            assert_eq!(&Permission::parse(&p.as_string()).unwrap(), p);
        }
    }

    #[test]
    fn rejects_bad_ids() {
        for id in ["", "../x", "a/b", "a b", "ünï", &"a".repeat(65)] {
            assert!(parse(&with("id", &serde_json::to_string(id).unwrap())).is_err(), "{id:?}");
        }
    }

    #[test]
    fn rejects_traversal_and_odd_entries() {
        for e in ["../index.html", "/etc/x.html", "a/../b.html", "a\\b.html", "index.js", "", "./", "C:/x.html"] {
            assert!(parse(&with("entry", &serde_json::to_string(e).unwrap())).is_err(), "{e:?}");
        }
        assert!(parse(&with("entry", r#""ui/main.html""#)).is_ok());
    }

    #[test]
    fn rejects_unknown_and_malformed_permissions() {
        for p in ["pty", "net:", "net:*.example.com", "net:https://x.com", "net:host:80", "net:Example.com", "net:-x.com", "net:a..b", "net:127.0.0.1", "net:192.168.1.5", "net:2130706433", "net:localhost.5"] {
            assert!(parse(&with("permissions", &serde_json::to_string(&[p]).unwrap())).is_err(), "{p:?}");
        }
    }

    #[test]
    fn rejects_unknown_fields_and_too_many_permissions() {
        assert!(parse(&with("extra", "1")).is_err());
        let many: Vec<String> = (0..33).map(|i| format!("net:h{i}.com")).collect();
        assert!(parse(&with("permissions", &serde_json::to_string(&many).unwrap())).is_err());
    }

    #[test]
    fn rejects_bad_name_and_version() {
        assert!(parse(&with("name", r#""  ""#)).is_err());
        assert!(parse(&with("name", r#""a\u0007b""#)).is_err());
        for c in ["\u{202e}", "\u{200b}", "\u{2066}", "\u{feff}"] {
            assert!(parse(&with("name", &serde_json::to_string(&format!("ok{c}name")).unwrap())).is_err(), "{c:?}");
        }
        assert!(parse(&with("name", &serde_json::to_string(&"x".repeat(41)).unwrap())).is_err());
        assert!(parse(&with("version", r#""1 0""#)).is_err());
    }
}
