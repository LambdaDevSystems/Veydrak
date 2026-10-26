use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use mdns_sd::{ServiceDaemon, ServiceEvent};

pub type NameMap = Arc<Mutex<HashMap<IpAddr, String>>>;

const SERVICES: &[&str] = &[
    "_workstation._tcp.local.",
    "_ssh._tcp.local.",
    "_http._tcp.local.",
    "_airplay._tcp.local.",
    "_ipp._tcp.local.",
];

/// Start background mDNS browsing; discovered hostnames land in the returned map.
/// Names are untrusted network input: the frontend sanitizes before display.
pub fn start() -> NameMap {
    let names: NameMap = Arc::new(Mutex::new(HashMap::new()));
    let Ok(daemon) = ServiceDaemon::new() else { return names };

    for svc in SERVICES {
        let Ok(rx) = daemon.browse(svc) else { continue };
        let names = names.clone();
        std::thread::spawn(move || {
            while let Ok(ev) = rx.recv() {
                if let ServiceEvent::ServiceResolved(info) = ev {
                    let host = info.get_hostname().trim_end_matches('.').trim_end_matches(".local").to_string();
                    if let Ok(mut m) = names.lock() {
                        for ip in info.get_addresses() {
                            m.insert(ip.to_ip_addr(), host.clone());
                        }
                    }
                }
            }
        });
    }
    names
}
