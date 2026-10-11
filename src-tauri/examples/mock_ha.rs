//! Local fake Home Assistant for trying the panel without the real one:
//!
//!   cargo run --example mock_ha --features mock-ha
//!
//! then in ~/.config/veydrak/config.json set:
//!   "homeassistant": { "url": "http://127.0.0.1:8123", "entities": [
//!       {"id": "sensor.ups_runtime", "label": "UPS runtime"}, {"id": "sensor.rack_temp"} ] }
//! and start Veydrak with VEYDRAK_HA_TOKEN=mock-token (or enter "mock-token" in the panel).

use std::time::Duration;

use veydrak_lib::mock::{self, MockConfig, MockHandle};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8123").await.expect("port 8123 is busy");
    println!("mock Home Assistant on ws://127.0.0.1:8123/api/websocket (token: mock-token)");
    let cfg = MockConfig {
        token: "mock-token".into(),
        states: vec![
            mock::state("sensor.ups_runtime", "1800", Some("s"), Some("UPS Runtime")),
            mock::state("sensor.rack_temp", "24.5", Some("°C"), Some("Rack Temperature")),
            mock::state("sensor.office_power", "310", Some("W"), Some("Office Power")),
            mock::state("sensor.air_quality", "unavailable", None, Some("Air Quality")),
        ],
        tick: Some(Duration::from_secs(2)),
        drop_first_connection: false,
    };
    mock::serve(listener, cfg, MockHandle::default()).await;
}
