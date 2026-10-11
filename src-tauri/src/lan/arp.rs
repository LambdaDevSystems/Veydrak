use std::net::Ipv4Addr;
use std::process::Command;

/// Parse IPv4 neighbours out of `/proc/net/arp` text (Linux). Skips incomplete entries (flags 0x0).
pub fn parse_proc_arp(text: &str) -> Vec<Ipv4Addr> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() >= 4 && f[2] != "0x0" && f[3] != "00:00:00:00:00:00" {
                f[0].parse().ok()
            } else {
                None
            }
        })
        .collect()
}

/// Fallback for macOS/Windows: pull dotted-quad tokens out of `arp -a`.
fn parse_arp_a(text: &str) -> Vec<Ipv4Addr> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter_map(|t| t.parse::<Ipv4Addr>().ok())
        .filter(|ip| !ip.is_multicast() && !ip.is_broadcast() && !ip.is_unspecified())
        .collect()
}

pub fn neighbours() -> Vec<Ipv4Addr> {
    let mut found = match std::fs::read_to_string("/proc/net/arp") {
        Ok(t) => parse_proc_arp(&t),
        Err(_) => Command::new("arp")
            .arg("-a")
            .output()
            .map(|o| parse_arp_a(&String::from_utf8_lossy(&o.stdout)))
            .unwrap_or_default(),
    };
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_arp() {
        let t = "IP address       HW type     Flags       HW address            Mask     Device\n\
                 192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:ff     *        eth0\n\
                 192.168.1.9      0x1         0x0         00:00:00:00:00:00     *        eth0\n";
        assert_eq!(parse_proc_arp(t), vec![Ipv4Addr::new(192, 168, 1, 1)]);
    }
}
