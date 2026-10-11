//! Battery state from `/sys/class/power_supply` (Linux, unprivileged). Absent elsewhere -> `None`.

use std::path::Path;

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

fn read_trim(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

/// First `type == Battery` supply under `root` with a valid capacity.
pub fn read_from(root: &Path) -> Option<Battery> {
    let mut entries: Vec<_> = std::fs::read_dir(root).ok()?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let dir = e.path();
        if read_trim(&dir.join("type")).as_deref() != Some("Battery") {
            continue;
        }
        let Some(percent) = read_trim(&dir.join("capacity")).and_then(|c| c.parse::<u8>().ok()) else {
            continue;
        };
        let status = read_trim(&dir.join("status")).unwrap_or_default();
        return Some(Battery { percent: percent.min(100), charging: status == "Charging" || status == "Full" });
    }
    None
}

pub fn read() -> Option<Battery> {
    read_from(Path::new("/sys/class/power_supply"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn supply(root: &Path, name: &str, files: &[(&str, &str)]) {
        let d = root.join(name);
        fs::create_dir_all(&d).unwrap();
        for (k, v) in files {
            fs::write(d.join(k), v).unwrap();
        }
    }

    #[test]
    fn reads_first_battery_and_skips_mains() {
        let root = std::env::temp_dir().join(format!("veydrak-bat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        supply(&root, "AC", &[("type", "Mains\n"), ("online", "1\n")]);
        supply(&root, "BAT0", &[("type", "Battery\n"), ("capacity", "73\n"), ("status", "Charging\n")]);
        assert_eq!(read_from(&root), Some(Battery { percent: 73, charging: true }));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn none_without_battery() {
        let root = std::env::temp_dir().join(format!("veydrak-nobat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        supply(&root, "AC", &[("type", "Mains\n")]);
        assert_eq!(read_from(&root), None);
        assert_eq!(read_from(&root.join("missing")), None);
        let _ = fs::remove_dir_all(&root);
    }
}
