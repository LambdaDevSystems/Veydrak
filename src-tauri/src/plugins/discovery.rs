//! Scan the plugins directory. Invalid plugins are skipped with a warning; one bad plugin never
//! stops the others (or the app) from loading.

use std::path::{Path, PathBuf};

use super::manifest::{self, Manifest, MAX_MANIFEST_BYTES};

const MAX_PLUGINS: usize = 32;

#[derive(Debug, PartialEq)]
pub struct Plugin {
    pub manifest: Manifest,
    pub dir: PathBuf,
}

#[derive(Default, Debug)]
pub struct Discovered {
    pub plugins: Vec<Plugin>,
    pub warnings: Vec<String>,
}

pub fn plugins_dir() -> Option<PathBuf> {
    crate::config::config_path()?.parent().map(|p| p.join("plugins"))
}

fn is_real_file(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())
}

fn load_one(dir: &Path, name: &str) -> Result<Manifest, String> {
    let path = dir.join("manifest.json");
    let meta = std::fs::symlink_metadata(&path).map_err(|e| format!("cannot read manifest.json: {e}"))?;
    if !meta.is_file() || meta.len() > MAX_MANIFEST_BYTES {
        return Err("manifest.json must be a regular file under 16 KiB".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read manifest.json: {e}"))?;
    let m = manifest::parse(&text)?;
    if m.id != name {
        return Err(format!("folder name must match plugin id {:?}", m.id));
    }
    if !is_real_file(&dir.join(&m.entry)) {
        return Err(format!("entry {:?} is missing or not a regular file", m.entry));
    }
    Ok(m)
}

/// A missing plugins directory just means no plugins.
pub fn discover(root: &Path) -> Discovered {
    let mut out = Discovered::default();
    let Ok(rd) = std::fs::read_dir(root) else { return out };
    let mut dirs: Vec<(String, PathBuf)> = rd
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir())) // symlinked plugin folders are not followed
        .filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
        .collect();
    dirs.sort();
    for (name, dir) in dirs {
        if out.plugins.len() >= MAX_PLUGINS {
            out.warnings.push(format!("plugin limit ({MAX_PLUGINS}) reached; ignoring the rest"));
            break;
        }
        match load_one(&dir, &name) {
            Ok(manifest) => out.plugins.push(Plugin { manifest, dir }),
            Err(e) => out.warnings.push(format!("plugin {name:?} skipped: {e}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("veydrak-plugins-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_plugin(root: &Path, dir: &str, manifest: &str, with_entry: bool) {
        let d = root.join(dir);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("manifest.json"), manifest).unwrap();
        if with_entry {
            fs::write(d.join("index.html"), "<html></html>").unwrap();
        }
    }

    const GOOD: &str = r#"{"id":"good","name":"Good","version":"1.0.0"}"#;

    #[test]
    fn missing_dir_yields_nothing() {
        let d = discover(Path::new("/nonexistent/veydrak/plugins"));
        assert!(d.plugins.is_empty() && d.warnings.is_empty());
    }

    #[test]
    fn loads_valid_and_skips_invalid_with_warnings() {
        let root = tmp("mix");
        write_plugin(&root, "good", GOOD, true);
        write_plugin(&root, "noentry", r#"{"id":"noentry","name":"N","version":"1"}"#, false);
        write_plugin(&root, "mismatch", GOOD, true);
        write_plugin(&root, "broken", "{not json", true);
        let d = discover(&root);
        assert_eq!(d.plugins.len(), 1);
        assert_eq!(d.plugins[0].manifest.id, "good");
        assert_eq!(d.warnings.len(), 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn oversized_manifest_is_rejected() {
        let root = tmp("big");
        let big = format!(r#"{{"id":"big","name":"B","version":"1","pad":"{}"}}"#, "x".repeat(20_000));
        write_plugin(&root, "big", &big, true);
        let d = discover(&root);
        assert!(d.plugins.is_empty());
        assert_eq!(d.warnings.len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_entry_and_folder_are_ignored() {
        let root = tmp("sym");
        let outside = tmp("sym-outside");
        fs::write(outside.join("secret.html"), "x").unwrap();
        write_plugin(&root, "linked", r#"{"id":"linked","name":"L","version":"1"}"#, false);
        std::os::unix::fs::symlink(outside.join("secret.html"), root.join("linked/index.html")).unwrap();
        write_plugin(&outside, "viaLink", r#"{"id":"viaLink","name":"V","version":"1"}"#, true);
        std::os::unix::fs::symlink(outside.join("viaLink"), root.join("viaLink")).unwrap();
        let d = discover(&root);
        assert!(d.plugins.is_empty());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }
}
