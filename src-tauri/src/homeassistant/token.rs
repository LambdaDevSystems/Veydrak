//! Access-token storage. The token never leaves the Rust side: it is read here for the WebSocket
//! handshake and is never returned to the webview or logged.

const SERVICE: &str = "veydrak";
const USER: &str = "homeassistant";
pub const ENV_VAR: &str = "VEYDRAK_HA_TOKEN";
const MAX_LEN: usize = 4096;

fn entry() -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, USER)
}

/// OS keyring first, then the `VEYDRAK_HA_TOKEN` environment variable.
pub fn load() -> Option<String> {
    entry()
        .and_then(|e| e.get_password())
        .ok()
        .filter(|t| !t.is_empty())
        .or_else(|| std::env::var(ENV_VAR).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()))
}

/// Validate a pasted token: non-empty, bounded, no whitespace inside.
pub fn validate(raw: &str) -> Result<String, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("token is empty".into());
    }
    if t.len() > MAX_LEN {
        return Err("token is too long".into());
    }
    if t.chars().any(char::is_whitespace) {
        return Err("token must not contain whitespace".into());
    }
    Ok(t.to_string())
}

pub fn store(token: &str) -> Result<(), String> {
    entry().and_then(|e| e.set_password(token)).map_err(|e| {
        format!("could not save to the OS keyring ({e}); set the {ENV_VAR} environment variable instead")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_tokens() {
        assert_eq!(validate("  abc.def  ").unwrap(), "abc.def");
        assert!(validate("").is_err());
        assert!(validate("a b").is_err());
        assert!(validate("a\nb").is_err());
        assert!(validate(&"x".repeat(5000)).is_err());
    }
}
