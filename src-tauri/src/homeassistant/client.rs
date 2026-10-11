//! Connection management: one WebSocket session (`run_session`) and the reconnect loop (`run_loop`).
//! Decoupled from Tauri: results go out over an mpsc channel, so it is testable against `mock`.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::{mpsc, Notify};
use tokio_tungstenite::tungstenite::Message;

use super::protocol::{self, EntityState, Incoming};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const PING_EVERY: Duration = Duration::from_secs(30);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    NoConfig,
    NoToken,
    Connecting,
    Online,
    Offline,
    AuthFailed,
}

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct StatusInfo {
    pub status: Status,
    pub message: Option<String>,
}

impl StatusInfo {
    pub fn new(status: Status, message: Option<String>) -> Self {
        Self { status, message }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HaEvent {
    Status(StatusInfo),
    /// Newly received states for configured entities (initial snapshot or incremental updates).
    Entities(Vec<EntityState>),
}

#[derive(Debug)]
pub enum SessionEnd {
    AuthInvalid(String),
    Closed { reason: String, was_online: bool },
}

pub type TokenProvider = Arc<dyn Fn() -> Option<String> + Send + Sync>;

/// One connection: handshake, initial snapshot, subscription, event loop. Returns why it ended.
pub async fn run_session(
    url: &str,
    token: &str,
    ids: &HashSet<String>,
    tx: &mpsc::Sender<HaEvent>,
    ping_every: Duration,
) -> SessionEnd {
    let closed = |reason: String, was_online: bool| SessionEnd::Closed { reason, was_online };

    let connect = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url)).await;
    let mut ws = match connect {
        Err(_) => return closed("connection timed out".into(), false),
        Ok(Err(e)) => return closed(format!("connection failed: {e}"), false),
        Ok(Ok((ws, _))) => ws,
    };

    let mut online = false;
    let mut ping_id: u64 = 100;
    let mut ticker = tokio::time::interval(ping_every);
    ticker.tick().await; // first tick completes immediately

    loop {
        tokio::select! {
            msg = ws.next() => {
                let text = match msg {
                    None => return closed("connection closed".into(), online),
                    Some(Err(e)) => return closed(format!("connection error: {e}"), online),
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) => return closed("server closed the connection".into(), online),
                    Some(Ok(_)) => continue, // binary / ping / pong frames
                };
                let incoming = match protocol::parse_incoming(text.as_str()) {
                    Ok(i) => i,
                    Err(e) => return closed(e, online),
                };
                match incoming {
                    Incoming::AuthRequired => {
                        if ws.send(Message::Text(protocol::auth_message(token).into())).await.is_err() {
                            return closed("could not send credentials".into(), online);
                        }
                    }
                    Incoming::AuthOk => {
                        online = true;
                        let status = HaEvent::Status(StatusInfo::new(Status::Online, None));
                        if tx.send(status).await.is_err() { return closed("receiver gone".into(), online); }
                        for m in [protocol::get_states_message(), protocol::subscribe_message()] {
                            if ws.send(Message::Text(m.into())).await.is_err() {
                                return closed("could not send request".into(), online);
                            }
                        }
                    }
                    Incoming::AuthInvalid(m) => return SessionEnd::AuthInvalid(m),
                    Incoming::CommandResult { id, success, error, states } => {
                        if !success && id == protocol::SUBSCRIBE_ID {
                            return closed(format!("subscribe failed: {}", error.unwrap_or_default()), online);
                        }
                        if success && id == protocol::GET_STATES_ID {
                            let wanted: Vec<EntityState> = states.into_iter().filter(|s| ids.contains(&s.entity_id)).collect();
                            if tx.send(HaEvent::Entities(wanted)).await.is_err() { return closed("receiver gone".into(), online); }
                        }
                    }
                    Incoming::StateChanged(s) => {
                        if ids.contains(&s.entity_id) && tx.send(HaEvent::Entities(vec![s])).await.is_err() {
                            return closed("receiver gone".into(), online);
                        }
                    }
                    Incoming::Removed(id) => {
                        if ids.contains(&id) {
                            let gone = EntityState { entity_id: id, state: "unavailable".into(), unit: None, friendly_name: None, last_updated: None };
                            if tx.send(HaEvent::Entities(vec![gone])).await.is_err() { return closed("receiver gone".into(), online); }
                        }
                    }
                    Incoming::Pong | Incoming::Other => {}
                }
            }
            _ = ticker.tick(), if online => {
                ping_id += 1;
                if ws.send(Message::Text(protocol::ping_message(ping_id).into())).await.is_err() {
                    return closed("ping failed".into(), online);
                }
            }
        }
    }
}

/// Run forever: wait for a token, connect, and reconnect with exponential backoff. An invalid token
/// stops retries until `notify` fires (a new token was saved).
pub async fn run_loop(
    url: String,
    ids: HashSet<String>,
    tx: mpsc::Sender<HaEvent>,
    token: TokenProvider,
    notify: Arc<Notify>,
    base_backoff: Duration,
) {
    let mut backoff = base_backoff;
    loop {
        let provider = token.clone();
        let tok = tokio::task::spawn_blocking(move || provider()).await.ok().flatten();
        let Some(tok) = tok else {
            let _ = tx.send(HaEvent::Status(StatusInfo::new(Status::NoToken, None))).await;
            notify.notified().await;
            continue;
        };

        let _ = tx.send(HaEvent::Status(StatusInfo::new(Status::Connecting, None))).await;
        match run_session(&url, &tok, &ids, &tx, PING_EVERY).await {
            SessionEnd::AuthInvalid(m) => {
                let _ = tx.send(HaEvent::Status(StatusInfo::new(Status::AuthFailed, Some(m)))).await;
                notify.notified().await;
                backoff = base_backoff;
            }
            SessionEnd::Closed { reason, was_online } => {
                if tx.is_closed() {
                    return;
                }
                if was_online {
                    backoff = base_backoff;
                }
                let _ = tx.send(HaEvent::Status(StatusInfo::new(Status::Offline, Some(reason)))).await;
                let jitter = Duration::from_millis(u64::from(rand::random::<u8>()) * backoff.as_millis().min(4000) as u64 / 1024);
                tokio::select! {
                    _ = tokio::time::sleep(backoff + jitter) => {}
                    _ = notify.notified() => { backoff = base_backoff; }
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::mock::{self, MockConfig, MockHandle};
    use super::*;
    use std::sync::Mutex;
    use tokio::net::TcpListener;

    const TOKEN: &str = "test-token";

    async fn start_mock(tick: Option<Duration>, drop_first: bool) -> (String, MockHandle) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/api/websocket", listener.local_addr().unwrap());
        let handle = MockHandle::default();
        let cfg = MockConfig {
            token: TOKEN.into(),
            states: vec![
                mock::state("sensor.ups_runtime", "1800", Some("s"), Some("UPS Runtime")),
                mock::state("sensor.rack_temp", "24.5", Some("°C"), Some("Rack Temp")),
                mock::state("light.hall", "on", None, None),
            ],
            tick,
            drop_first_connection: drop_first,
        };
        tokio::spawn(mock::serve(listener, cfg, handle.clone()));
        (url, handle)
    }

    fn ids(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    async fn recv(rx: &mut mpsc::Receiver<HaEvent>) -> HaEvent {
        tokio::time::timeout(Duration::from_secs(5), rx.recv()).await.expect("timed out").expect("channel closed")
    }

    #[tokio::test]
    async fn happy_path_filters_to_configured_entities() {
        let (url, handle) = start_mock(Some(Duration::from_millis(30)), false).await;
        let (tx, mut rx) = mpsc::channel(64);
        let wanted = ids(&["sensor.rack_temp", "sensor.ups_runtime", "sensor.missing"]);
        let task = tokio::spawn(async move { run_session(&url, TOKEN, &wanted, &tx, Duration::from_secs(30)).await });

        assert_eq!(recv(&mut rx).await, HaEvent::Status(StatusInfo::new(Status::Online, None)));
        match recv(&mut rx).await {
            HaEvent::Entities(list) => {
                let mut got: Vec<_> = list.iter().map(|s| s.entity_id.as_str()).collect();
                got.sort();
                assert_eq!(got, ["sensor.rack_temp", "sensor.ups_runtime"]); // light.hall and missing excluded
            }
            other => panic!("unexpected {other:?}"),
        }
        // Live updates arrive for configured entities only (the mock also sends an unrelated one).
        for _ in 0..4 {
            match recv(&mut rx).await {
                HaEvent::Entities(list) => assert!(list.iter().all(|s| s.entity_id != "sensor.unrelated_noise")),
                other => panic!("unexpected {other:?}"),
            }
        }
        task.abort();
        let order = handle.received.lock().unwrap().clone();
        assert_eq!(&order[..3], ["auth", "get_states", "subscribe_events"]);
    }

    #[tokio::test]
    async fn wrong_token_is_reported_as_auth_failure() {
        let (url, _h) = start_mock(None, false).await;
        let (tx, _rx) = mpsc::channel(8);
        match run_session(&url, "wrong", &ids(&["sensor.rack_temp"]), &tx, Duration::from_secs(30)).await {
            SessionEnd::AuthInvalid(m) => assert!(m.contains("Invalid")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn unreachable_server_reports_closed() {
        let (tx, _rx) = mpsc::channel(8);
        // Port 1 on localhost: connection refused.
        match run_session("ws://127.0.0.1:1/api/websocket", TOKEN, &ids(&["a.b"]), &tx, Duration::from_secs(30)).await {
            SessionEnd::Closed { was_online, .. } => assert!(!was_online),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn loop_reconnects_after_a_dropped_connection() {
        let (url, handle) = start_mock(None, true).await;
        let (tx, mut rx) = mpsc::channel(64);
        let notify = Arc::new(Notify::new());
        let provider: TokenProvider = Arc::new(|| Some(TOKEN.to_string()));
        let task = tokio::spawn(run_loop(url, ids(&["sensor.rack_temp"]), tx, provider, notify, Duration::from_millis(10)));

        let mut statuses = Vec::new();
        let mut onlines = 0;
        while onlines < 2 {
            if let HaEvent::Status(s) = recv(&mut rx).await {
                if s.status == Status::Online {
                    onlines += 1;
                }
                statuses.push(s.status);
            }
        }
        task.abort();
        assert!(statuses.contains(&Status::Offline), "expected an Offline between the two Online: {statuses:?}");
        assert!(handle.connections.load(std::sync::atomic::Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn loop_waits_for_a_token_then_connects() {
        let (url, _h) = start_mock(None, false).await;
        let (tx, mut rx) = mpsc::channel(64);
        let notify = Arc::new(Notify::new());
        let slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let provider: TokenProvider = {
            let slot = slot.clone();
            Arc::new(move || slot.lock().unwrap().clone())
        };
        let task = tokio::spawn(run_loop(url, ids(&["sensor.rack_temp"]), tx, provider, notify.clone(), Duration::from_millis(10)));

        assert_eq!(recv(&mut rx).await, HaEvent::Status(StatusInfo::new(Status::NoToken, None)));
        *slot.lock().unwrap() = Some(TOKEN.into());
        notify.notify_one();
        loop {
            if let HaEvent::Status(s) = recv(&mut rx).await {
                if s.status == Status::Online {
                    break;
                }
            }
        }
        task.abort();
    }

    #[tokio::test]
    async fn loop_stops_retrying_after_auth_failure_until_new_token() {
        let (url, handle) = start_mock(None, false).await;
        let (tx, mut rx) = mpsc::channel(64);
        let notify = Arc::new(Notify::new());
        let slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some("wrong".into())));
        let provider: TokenProvider = {
            let slot = slot.clone();
            Arc::new(move || slot.lock().unwrap().clone())
        };
        let task = tokio::spawn(run_loop(url, ids(&["sensor.rack_temp"]), tx, provider, notify.clone(), Duration::from_millis(10)));

        loop {
            if let HaEvent::Status(s) = recv(&mut rx).await {
                if s.status == Status::AuthFailed {
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(handle.connections.load(std::sync::atomic::Ordering::SeqCst), 1, "must not retry a bad token");

        *slot.lock().unwrap() = Some(TOKEN.into());
        notify.notify_one();
        loop {
            if let HaEvent::Status(s) = recv(&mut rx).await {
                if s.status == Status::Online {
                    break;
                }
            }
        }
        task.abort();
    }
}
