//! Startup guard: Veydrak is designed to run unprivileged.

const OVERRIDE_VAR: &str = "VEYDRAK_ALLOW_ROOT";

#[cfg(unix)]
fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    false
}

fn decide(root: bool, override_value: Option<&str>) -> Result<(), String> {
    if root && override_value != Some("1") {
        return Err(format!(
            "veydrak: refusing to run as root (it needs no privileges). \
             Run as a normal user, or set {OVERRIDE_VAR}=1 to override."
        ));
    }
    Ok(())
}

pub fn refuse_root() -> Result<(), String> {
    decide(is_root(), std::env::var(OVERRIDE_VAR).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_refused_unless_overridden() {
        assert!(decide(true, None).is_err());
        assert!(decide(true, Some("0")).is_err());
        assert!(decide(true, Some("1")).is_ok());
    }

    #[test]
    fn non_root_always_ok() {
        assert!(decide(false, None).is_ok());
    }
}
