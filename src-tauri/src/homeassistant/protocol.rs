//! Home Assistant WebSocket API messages (https://developers.home-assistant.io/docs/api/websocket).
//! Pure functions only: no I/O, so everything here is unit-testable.

use serde::Serialize;
use serde_json::{json, Value};
use url::Url;

pub const GET_STATES_ID: u64 = 1;
pub const SUBSCRIBE_ID: u64 = 2;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct EntityState {
    pub entity_id: String,
    pub state: String,
    pub unit: Option<String>,
    pub friendly_name: Option<String>,
    pub last_updated: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Incoming {
    AuthRequired,
    AuthOk,
    AuthInvalid(String),
    /// Response to a command we sent.
    CommandResult { id: u64, success: bool, error: Option<String>, states: Vec<EntityState> },
    StateChanged(EntityState),
    /// Entity removed from HA (`new_state` is null).
    Removed(String),
    Pong,
    Other,
}

pub fn auth_message(token: &str) -> String {
    json!({ "type": "auth", "access_token": token }).to_string()
}

pub fn get_states_message() -> String {
    json!({ "id": GET_STATES_ID, "type": "get_states" }).to_string()
}

pub fn subscribe_message() -> String {
    json!({ "id": SUBSCRIBE_ID, "type": "subscribe_events", "event_type": "state_changed" }).to_string()
}

pub fn ping_message(id: u64) -> String {
    json!({ "id": id, "type": "ping" }).to_string()
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Parse one HA state object: `{entity_id, state, attributes{unit_of_measurement, friendly_name}, last_updated}`.
pub fn parse_state(v: &Value) -> Option<EntityState> {
    let entity_id = str_field(v, "entity_id")?;
    let state = str_field(v, "state")?;
    let attrs = v.get("attributes");
    Some(EntityState {
        entity_id,
        state,
        unit: attrs.and_then(|a| str_field(a, "unit_of_measurement")),
        friendly_name: attrs.and_then(|a| str_field(a, "friendly_name")),
        last_updated: str_field(v, "last_updated"),
    })
}

pub fn parse_incoming(text: &str) -> Result<Incoming, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("invalid JSON from server: {e}"))?;
    let kind = v.get("type").and_then(Value::as_str).unwrap_or_default();
    Ok(match kind {
        "auth_required" => Incoming::AuthRequired,
        "auth_ok" => Incoming::AuthOk,
        "auth_invalid" => Incoming::AuthInvalid(str_field(&v, "message").unwrap_or_else(|| "invalid access token".into())),
        "pong" => Incoming::Pong,
        "result" => {
            let states = v
                .get("result")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(parse_state).collect())
                .unwrap_or_default();
            Incoming::CommandResult {
                id: v.get("id").and_then(Value::as_u64).unwrap_or(0),
                success: v.get("success").and_then(Value::as_bool).unwrap_or(false),
                error: v.pointer("/error/message").and_then(Value::as_str).map(str::to_owned),
                states,
            }
        }
        "event" => {
            let data = v.pointer("/event/data");
            let is_state_changed = v.pointer("/event/event_type").and_then(Value::as_str) == Some("state_changed");
            match (is_state_changed, data) {
                (true, Some(d)) => match d.get("new_state") {
                    Some(ns) if !ns.is_null() => parse_state(ns).map(Incoming::StateChanged).unwrap_or(Incoming::Other),
                    _ => str_field(d, "entity_id").map(Incoming::Removed).unwrap_or(Incoming::Other),
                },
                _ => Incoming::Other,
            }
        }
        _ => Incoming::Other,
    })
}

/// Turn a user-supplied base URL (`http://host:8123`, `https://ha.example.com`, `ws://...`) into the
/// WebSocket endpoint. Credentials in the URL are rejected; the port must be given explicitly if non-default.
pub fn ws_url(raw: &str) -> Result<String, String> {
    let mut u = Url::parse(raw.trim()).map_err(|e| format!("not a valid URL: {e}"))?;
    let scheme = match u.scheme() {
        "http" | "ws" => "ws",
        "https" | "wss" => "wss",
        other => return Err(format!("unsupported scheme '{other}' (use http, https, ws or wss)")),
    };
    if u.host_str().is_none() {
        return Err("URL has no host".into());
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err("credentials in the URL are not supported; use an access token".into());
    }
    u.set_scheme(scheme).map_err(|_| "could not set scheme".to_string())?;
    let base = u.path().trim_end_matches('/').trim_end_matches("/api/websocket").trim_end_matches('/').to_string();
    u.set_path(&format!("{base}/api/websocket"));
    u.set_query(None);
    u.set_fragment(None);
    Ok(u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outgoing_messages_match_documented_shapes() {
        let auth: Value = serde_json::from_str(&auth_message("abc")).unwrap();
        assert_eq!(auth, json!({"type": "auth", "access_token": "abc"}));
        let gs: Value = serde_json::from_str(&get_states_message()).unwrap();
        assert_eq!(gs, json!({"id": 1, "type": "get_states"}));
        let sub: Value = serde_json::from_str(&subscribe_message()).unwrap();
        assert_eq!(sub, json!({"id": 2, "type": "subscribe_events", "event_type": "state_changed"}));
        let ping: Value = serde_json::from_str(&ping_message(7)).unwrap();
        assert_eq!(ping, json!({"id": 7, "type": "ping"}));
    }

    #[test]
    fn auth_flow() {
        assert_eq!(parse_incoming(r#"{"type":"auth_required","ha_version":"2024.10.0"}"#).unwrap(), Incoming::AuthRequired);
        assert_eq!(parse_incoming(r#"{"type":"auth_ok","ha_version":"2024.10.0"}"#).unwrap(), Incoming::AuthOk);
        assert_eq!(
            parse_incoming(r#"{"type":"auth_invalid","message":"Invalid access token or password"}"#).unwrap(),
            Incoming::AuthInvalid("Invalid access token or password".into())
        );
    }

    #[test]
    fn get_states_result() {
        let msg = r#"{"id":1,"type":"result","success":true,"result":[
            {"entity_id":"sensor.ups_runtime","state":"1830","attributes":{"unit_of_measurement":"s","friendly_name":"UPS Runtime"},"last_updated":"2024-10-01T10:00:00+00:00"},
            {"entity_id":"light.hall","state":"on","attributes":{}},
            {"bad":"entry"}
        ]}"#;
        match parse_incoming(msg).unwrap() {
            Incoming::CommandResult { id, success, states, .. } => {
                assert_eq!((id, success), (1, true));
                assert_eq!(states.len(), 2); // malformed entry skipped
                assert_eq!(states[0].unit.as_deref(), Some("s"));
                assert_eq!(states[0].friendly_name.as_deref(), Some("UPS Runtime"));
                assert_eq!(states[1].unit, None);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn failed_result_carries_error() {
        let msg = r#"{"id":2,"type":"result","success":false,"error":{"code":"unauthorized","message":"nope"}}"#;
        match parse_incoming(msg).unwrap() {
            Incoming::CommandResult { success, error, .. } => {
                assert!(!success);
                assert_eq!(error.as_deref(), Some("nope"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn state_changed_event() {
        let msg = r#"{"id":2,"type":"event","event":{"event_type":"state_changed","data":{
            "entity_id":"sensor.rack_temp","old_state":null,
            "new_state":{"entity_id":"sensor.rack_temp","state":"unavailable","attributes":{"unit_of_measurement":"°C"}}}}}"#;
        match parse_incoming(msg).unwrap() {
            Incoming::StateChanged(s) => {
                assert_eq!(s.entity_id, "sensor.rack_temp");
                assert_eq!(s.state, "unavailable");
                assert_eq!(s.unit.as_deref(), Some("°C"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn removed_entity_and_other_events() {
        let removed = r#"{"id":2,"type":"event","event":{"event_type":"state_changed","data":{"entity_id":"sensor.gone","new_state":null}}}"#;
        assert_eq!(parse_incoming(removed).unwrap(), Incoming::Removed("sensor.gone".into()));
        let other = r#"{"id":2,"type":"event","event":{"event_type":"call_service","data":{}}}"#;
        assert_eq!(parse_incoming(other).unwrap(), Incoming::Other);
        assert_eq!(parse_incoming(r#"{"id":9,"type":"pong"}"#).unwrap(), Incoming::Pong);
        assert!(parse_incoming("not json").is_err());
    }

    #[test]
    fn url_normalization() {
        assert_eq!(ws_url("http://homeassistant.local:8123").unwrap(), "ws://homeassistant.local:8123/api/websocket");
        assert_eq!(ws_url("https://ha.example.com/").unwrap(), "wss://ha.example.com/api/websocket");
        assert_eq!(ws_url("ws://10.0.0.5:8123/api/websocket").unwrap(), "ws://10.0.0.5:8123/api/websocket");
        assert_eq!(ws_url("https://example.com/ha?x=1#y").unwrap(), "wss://example.com/ha/api/websocket");
    }

    #[test]
    fn url_rejections() {
        assert!(ws_url("ftp://host").is_err());
        assert!(ws_url("not a url").is_err());
        assert!(ws_url("http://user:pw@host:8123").is_err());
        assert!(ws_url("file:///etc/passwd").is_err());
    }
}
