//! A tiny in-process Home Assistant WebSocket server for tests and local trials
//! (`cargo run --example mock_ha --features mock-ha`). Speaks just enough of the real protocol.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use super::protocol::EntityState;

#[derive(Clone)]
pub struct MockConfig {
    pub token: String,
    pub states: Vec<EntityState>,
    /// If set, push a `state_changed` for every numeric entity (plus an unrelated one) at this interval.
    pub tick: Option<Duration>,
    /// Close the first connection right after the subscribe, to exercise reconnects.
    pub drop_first_connection: bool,
}

#[derive(Clone, Default)]
pub struct MockHandle {
    /// Message "types" received from clients, in order (e.g. `auth`, `get_states`, `subscribe_events`).
    pub received: Arc<Mutex<Vec<String>>>,
    pub connections: Arc<AtomicUsize>,
}

pub fn state(id: &str, value: &str, unit: Option<&str>, name: Option<&str>) -> EntityState {
    EntityState {
        entity_id: id.into(),
        state: value.into(),
        unit: unit.map(Into::into),
        friendly_name: name.map(Into::into),
        last_updated: Some("2024-10-01T10:00:00+00:00".into()),
    }
}

fn state_json(s: &EntityState) -> Value {
    json!({
        "entity_id": s.entity_id, "state": s.state, "last_updated": s.last_updated,
        "attributes": { "unit_of_measurement": s.unit, "friendly_name": s.friendly_name }
    })
}

pub async fn serve(listener: TcpListener, cfg: MockConfig, handle: MockHandle) {
    loop {
        let Ok((stream, _)) = listener.accept().await else { return };
        let n = handle.connections.fetch_add(1, Ordering::SeqCst);
        let cfg = cfg.clone();
        let handle = handle.clone();
        tokio::spawn(async move {
            let _ = session(stream, cfg, handle, n == 0).await;
        });
    }
}

async fn session(stream: TcpStream, cfg: MockConfig, handle: MockHandle, first: bool) -> Result<(), ()> {
    let mut ws = tokio_tungstenite::accept_async(stream).await.map_err(|_| ())?;
    ws.send(Message::Text(json!({"type": "auth_required", "ha_version": "2024.10.0"}).to_string().into()))
        .await
        .map_err(|_| ())?;

    let mut ticker = cfg.tick.map(tokio::time::interval);
    let mut counter = 0u32;
    let mut subscribed = false;
    loop {
        tokio::select! {
            msg = ws.next() => {
                let Some(Ok(Message::Text(text))) = msg else { return Ok(()) };
                let v: Value = serde_json::from_str(text.as_str()).map_err(|_| ())?;
                let kind = v["type"].as_str().unwrap_or_default().to_string();
                handle.received.lock().unwrap().push(kind.clone());
                let id = v["id"].as_u64().unwrap_or(0);
                match kind.as_str() {
                    "auth" => {
                        if v["access_token"].as_str() == Some(cfg.token.as_str()) {
                            let ok = json!({"type": "auth_ok", "ha_version": "2024.10.0"});
                            ws.send(Message::Text(ok.to_string().into())).await.map_err(|_| ())?;
                        } else {
                            let bad = json!({"type": "auth_invalid", "message": "Invalid access token or password"});
                            ws.send(Message::Text(bad.to_string().into())).await.map_err(|_| ())?;
                            return Ok(());
                        }
                    }
                    "get_states" => {
                        let all: Vec<Value> = cfg.states.iter().map(state_json).collect();
                        let r = json!({"id": id, "type": "result", "success": true, "result": all});
                        ws.send(Message::Text(r.to_string().into())).await.map_err(|_| ())?;
                    }
                    "subscribe_events" => {
                        let r = json!({"id": id, "type": "result", "success": true, "result": null});
                        ws.send(Message::Text(r.to_string().into())).await.map_err(|_| ())?;
                        subscribed = true;
                        if first && cfg.drop_first_connection {
                            return Ok(());
                        }
                    }
                    "ping" => {
                        ws.send(Message::Text(json!({"id": id, "type": "pong"}).to_string().into())).await.map_err(|_| ())?;
                    }
                    _ => {}
                }
            }
            _ = async { ticker.as_mut().unwrap().tick().await }, if ticker.is_some() && subscribed => {
                counter += 1;
                let mut updates: Vec<EntityState> = cfg
                    .states
                    .iter()
                    .filter_map(|s| s.state.parse::<f64>().ok().map(|base| {
                        let mut s = s.clone();
                        s.state = format!("{:.1}", base + f64::from(counter % 10));
                        s
                    }))
                    .collect();
                updates.push(state("sensor.unrelated_noise", "1", None, None));
                for u in updates {
                    let ev = json!({"id": 2, "type": "event", "event": {"event_type": "state_changed",
                        "data": {"entity_id": u.entity_id, "old_state": null, "new_state": state_json(&u)}}});
                    ws.send(Message::Text(ev.to_string().into())).await.map_err(|_| ())?;
                }
            }
        }
    }
}
