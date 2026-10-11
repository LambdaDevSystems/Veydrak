use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tauri::{AppHandle, Emitter, Manager};

use super::roster::{Device, Status};
use super::{arp, mdns};
use crate::state::AppState;

/// Consecutive failures required to flip online -> offline (avoids flapping alerts).
pub const FAILS_TO_OFFLINE: u32 = 3;
/// Consecutive successes required to flip offline -> online.
pub const OKS_TO_ONLINE: u32 = 2;

const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
const SCAN_INTERVAL: Duration = Duration::from_secs(10);

/// Hysteresis state machine for a single device.
#[derive(Debug, Clone, Copy)]
pub struct Hysteresis {
    pub status: Status,
    fails: u32,
    oks: u32,
}

impl Default for Hysteresis {
    fn default() -> Self {
        Self { status: Status::Unknown, fails: 0, oks: 0 }
    }
}

impl Hysteresis {
    /// Feed one probe result; returns the (possibly updated) status.
    pub fn observe(&mut self, reachable: bool) -> Status {
        if reachable {
            self.fails = 0;
            self.oks += 1;
            match self.status {
                Status::Unknown => self.status = Status::Online,
                Status::Offline if self.oks >= OKS_TO_ONLINE => self.status = Status::Online,
                _ => {}
            }
        } else {
            self.oks = 0;
            self.fails += 1;
            match self.status {
                // A device never seen alive is not an alert until it has been confirmed down.
                Status::Online if self.fails >= FAILS_TO_OFFLINE => self.status = Status::Offline,
                _ => {}
            }
        }
        self.status
    }
}

async fn probe(client: &Client, ip: IpAddr) -> bool {
    let mut pinger = client.pinger(ip, PingIdentifier(rand::random())).await;
    pinger.timeout(PROBE_TIMEOUT);
    pinger.ping(PingSequence(0), &[0u8; 16]).await.is_ok()
}

pub async fn run_roster_loop(app: AppHandle) {
    let names = mdns::start();
    // If ICMP sockets are unavailable (no privilege), devices stay "unknown" instead of falsely offline.
    let client = Client::new(&Config::default()).ok();
    let mut machines: HashMap<IpAddr, Hysteresis> = HashMap::new();
    let mut tick = tokio::time::interval(SCAN_INTERVAL);

    loop {
        tick.tick().await;

        // Union of ARP neighbours and everything already tracked (so offline devices keep being probed).
        let mut targets: Vec<IpAddr> = arp::neighbours().into_iter().map(IpAddr::V4).collect();
        for ip in machines.keys() {
            if !targets.contains(ip) {
                targets.push(*ip);
            }
        }

        for ip in targets {
            let status = match &client {
                Some(c) => {
                    let ok = probe(c, ip).await;
                    machines.entry(ip).or_default().observe(ok)
                }
                None => {
                    machines.entry(ip).or_default();
                    Status::Unknown
                }
            };

            let name = names.lock().ok().and_then(|m| m.get(&ip).cloned()).unwrap_or_else(|| ip.to_string());
            let state = app.state::<AppState>();
            let Ok(mut roster) = state.roster.lock() else { continue };
            let pinned = roster.pins.contains(&ip.to_string());
            roster.devices.insert(ip, Device { ip: ip.to_string(), name, status, pinned });
        }

        let snapshot = app.state::<AppState>().roster.lock().map(|r| r.snapshot()).unwrap_or_default();
        let _ = app.emit("lan-roster", snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_success_goes_online() {
        let mut h = Hysteresis::default();
        assert_eq!(h.observe(true), Status::Online);
    }

    #[test]
    fn needs_consecutive_failures_to_go_offline() {
        let mut h = Hysteresis::default();
        h.observe(true);
        h.observe(false);
        h.observe(false);
        assert_eq!(h.observe(true), Status::Online); // blip resets the counter
        for _ in 0..FAILS_TO_OFFLINE - 1 {
            assert_eq!(h.observe(false), Status::Online);
        }
        assert_eq!(h.observe(false), Status::Offline);
    }

    #[test]
    fn needs_consecutive_successes_to_recover() {
        let mut h = Hysteresis::default();
        h.observe(true);
        for _ in 0..FAILS_TO_OFFLINE {
            h.observe(false);
        }
        assert_eq!(h.status, Status::Offline);
        assert_eq!(h.observe(true), Status::Offline);
        assert_eq!(h.observe(true), Status::Online);
    }

    #[test]
    fn never_seen_device_stays_unknown() {
        let mut h = Hysteresis::default();
        for _ in 0..10 {
            assert_eq!(h.observe(false), Status::Unknown);
        }
    }
}
