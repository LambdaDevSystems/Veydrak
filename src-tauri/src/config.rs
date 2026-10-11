//! User configuration at `~/.config/veydrak/config.json` (XDG on Linux, `%APPDATA%` on Windows).
//! Anything invalid falls back to defaults, with a warning surfaced to the UI.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const KNOWN_PANELS: &[&str] = &["terminal", "sysmon", "lan", "homeassistant"];
const MAX_HA_ENTITIES: usize = 24;
const MAX_LABEL_CHARS: usize = 40;
pub const KNOWN_THEMES: &[&str] = &["green", "amber"];
const MAX_SIDE_PANELS: usize = 4;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Layout {
    pub main: String,
    pub side: Vec<String>,
}

impl Default for Layout {
    fn default() -> Self {
        Self { main: "terminal".into(), side: vec!["sysmon".into(), "lan".into()] }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct HaEntity {
    pub id: String,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct HomeAssistant {
    /// Base URL, e.g. `http://homeassistant.local:8123`. Validated when the client starts.
    pub url: Option<String>,
    pub entities: Vec<HaEntity>,
}

/// `domain.object_id` with lowercase letters, digits and underscores, as HA entity ids are.
fn valid_entity_id(id: &str) -> bool {
    let ok = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    id.split_once('.').is_some_and(|(d, o)| ok(d) && ok(o))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    /// Check GitHub Releases for a signed update at startup (installing always needs a click).
    pub check_updates: bool,
    pub layout: Layout,
    pub homeassistant: HomeAssistant,
}

impl Default for Config {
    fn default() -> Self {
        Self { theme: "green".into(), check_updates: true, layout: Layout::default(), homeassistant: HomeAssistant::default() }
    }
}

/// Mark `id` as used if it is a known panel not yet placed; returns whether it was newly claimed.
fn claim(seen: &mut Vec<String>, id: &str) -> bool {
    if KNOWN_PANELS.contains(&id) && !seen.iter().any(|s| s == id) {
        seen.push(id.to_string());
        true
    } else {
        false
    }
}

impl Config {
    /// Clamp to known values: unknown themes/panels are dropped, panels are deduped across the layout.
    pub fn sanitized(mut self) -> Self {
        if !KNOWN_THEMES.contains(&self.theme.as_str()) {
            self.theme = Config::default().theme;
        }
        let mut seen: Vec<String> = Vec::new();
        let main = if claim(&mut seen, &self.layout.main) {
            self.layout.main.clone()
        } else {
            // Invalid main panel: fall back to the terminal.
            claim(&mut seen, "terminal");
            "terminal".to_string()
        };
        let mut side: Vec<String> = Vec::new();
        for id in &self.layout.side {
            if side.len() < MAX_SIDE_PANELS && claim(&mut seen, id) {
                side.push(id.clone());
            }
        }
        self.layout = Layout { main, side };

        let mut ids: Vec<String> = Vec::new();
        self.homeassistant.entities.retain_mut(|e| {
            e.id = e.id.trim().to_string();
            e.label = e.label.take().map(|l| l.trim().chars().take(MAX_LABEL_CHARS).collect::<String>()).filter(|l| !l.is_empty());
            let keep = valid_entity_id(&e.id) && !ids.contains(&e.id) && ids.len() < MAX_HA_ENTITIES;
            if keep {
                ids.push(e.id.clone());
            }
            keep
        });
        self
    }
}

#[derive(Serialize)]
pub struct LoadedConfig {
    pub config: Config,
    pub warning: Option<String>,
}

pub fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
    Some(base.join("veydrak").join("config.json"))
}

fn parse(text: &str) -> LoadedConfig {
    match serde_json::from_str::<Config>(text) {
        Ok(c) => LoadedConfig { config: c.sanitized(), warning: None },
        Err(e) => LoadedConfig {
            config: Config::default(),
            warning: Some(format!("config.json is invalid, using defaults: {e}")),
        },
    }
}

fn load_from(path: &std::path::Path) -> LoadedConfig {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // First run: write defaults so the file is discoverable. Failure is non-fatal.
            let cfg = Config::default();
            let _ = save_to(path, &cfg);
            LoadedConfig { config: cfg, warning: None }
        }
        Err(e) => LoadedConfig {
            config: Config::default(),
            warning: Some(format!("could not read config.json, using defaults: {e}")),
        },
    }
}

fn save_to(path: &std::path::Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(cfg).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[tauri::command]
pub fn get_config() -> LoadedConfig {
    match config_path() {
        Some(p) => load_from(&p),
        None => LoadedConfig {
            config: Config::default(),
            warning: Some("no config directory found, using defaults".into()),
        },
    }
}

#[tauri::command]
pub fn set_theme(theme: String) -> Result<(), String> {
    if !KNOWN_THEMES.contains(&theme.as_str()) {
        return Err("unknown theme".into());
    }
    let path = config_path().ok_or("no config directory")?;
    let mut loaded = load_from(&path);
    if loaded.warning.is_some() {
        // Don't clobber a hand-edited file that failed to parse.
        return Err("config.json is invalid; fix it before changing the theme".into());
    }
    loaded.config.theme = theme;
    save_to(&path, &loaded.config).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn defaults_are_stable_under_sanitize() {
        assert_eq!(Config::default().sanitized(), Config::default());
    }

    #[test]
    fn partial_config_fills_defaults() {
        let l = parse(r#"{"theme":"amber"}"#);
        assert!(l.warning.is_none());
        assert_eq!(l.config.theme, "amber");
        assert_eq!(l.config.layout, Layout::default());
    }

    #[test]
    fn check_updates_defaults_on_and_can_be_disabled() {
        assert!(parse(r#"{}"#).config.check_updates);
        assert!(!parse(r#"{"check_updates":false}"#).config.check_updates);
    }

    #[test]
    fn homeassistant_entities_are_validated() {
        let l = parse(
            r#"{"homeassistant":{"url":"http://ha.local:8123","entities":[
                {"id":"sensor.ups_runtime","label":"  UPS  "},
                {"id":"sensor.ups_runtime"},
                {"id":"Sensor.Bad"},
                {"id":"nodot"},
                {"id":"light.x","label":""},
                {"label":"no id"}]}}"#,
        );
        let ha = l.config.homeassistant;
        assert_eq!(ha.url.as_deref(), Some("http://ha.local:8123"));
        let ids: Vec<&str> = ha.entities.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["sensor.ups_runtime", "light.x"]);
        assert_eq!(ha.entities[0].label.as_deref(), Some("UPS"));
        assert_eq!(ha.entities[1].label, None);
    }

    #[test]
    fn homeassistant_entities_are_capped() {
        let many: Vec<String> = (0..40).map(|i| format!(r#"{{"id":"sensor.s{i}"}}"#)).collect();
        let l = parse(&format!(r#"{{"homeassistant":{{"entities":[{}]}}}}"#, many.join(",")));
        assert_eq!(l.config.homeassistant.entities.len(), 24);
    }

    #[test]
    fn invalid_json_falls_back_with_warning() {
        let l = parse("{ nope");
        assert!(l.warning.is_some());
        assert_eq!(l.config, Config::default());
    }

    #[test]
    fn unknown_values_are_dropped_and_deduped() {
        let c = Config {
            theme: "hotpink".into(),
            layout: Layout { main: "bogus".into(), side: s(&["lan", "lan", "x", "sysmon", "terminal"]) },
            ..Config::default()
        }
        .sanitized();
        assert_eq!(c.theme, "green");
        assert_eq!(c.layout.main, "terminal");
        assert_eq!(c.layout.side, s(&["lan", "sysmon"]));
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let dir = std::env::temp_dir().join(format!("veydrak-cfg-{}", std::process::id()));
        let path = dir.join("veydrak").join("config.json");
        let first = load_from(&path); // creates defaults
        assert_eq!(first.config, Config::default());
        let mut c = first.config;
        c.theme = "amber".into();
        save_to(&path, &c).unwrap();
        assert_eq!(load_from(&path).config.theme, "amber");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
