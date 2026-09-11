use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub name: String,
    pub address: String,
    pub online: bool,
}

/// Parse `tailscale status` text into peers. Empty/garbage → empty list, never panics.
pub fn parse_tailscale_status(raw: &str) -> Vec<Peer> {
    let mut peers = Vec::new();
    if raw.trim().is_empty() {
        return peers;
    }
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("logged in") || lower.starts_with("health") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let address = parts[0];
        if !looks_like_addr(address) {
            continue;
        }
        let name = parts[1].to_string();
        let online = !lower.contains("offline");
        peers.push(Peer {
            name,
            address: address.to_string(),
            online,
        });
    }
    peers
}

/// Parse avahi-browse / dns-sd style lines: `hostname address`
pub fn parse_mdns(raw: &str) -> Vec<Peer> {
    let mut peers = Vec::new();
    if raw.trim().is_empty() {
        return peers;
    }
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let (name, address) = if looks_like_addr(parts[0]) {
            (parts[1].to_string(), parts[0].to_string())
        } else if looks_like_addr(parts[1]) {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            continue;
        };
        peers.push(Peer {
            name,
            address,
            online: true,
        });
    }
    peers
}

pub fn merge_peers(mut a: Vec<Peer>, b: Vec<Peer>) -> Vec<Peer> {
    for p in b {
        if !a.iter().any(|x| x.address == p.address || x.name == p.name) {
            a.push(p);
        }
    }
    a
}

fn looks_like_addr(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return false;
    }
    s.contains('.') || s.contains(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
# Health check:
#     - derp
100.64.0.1    desk-linux   hideo@  linux   -
100.64.0.2    pixel-9      hideo@  android offline
fd7a:115c::3  tablet       hideo@  android active
not-a-peer
Logged in as hideo
"#;

    #[test]
    fn parses_online_and_offline_rows() {
        let peers = parse_tailscale_status(FIXTURE);
        assert!(peers.iter().any(|p| p.name == "desk-linux" && p.online));
        assert!(peers.iter().any(|p| p.name == "pixel-9" && !p.online));
        assert!(peers.iter().any(|p| p.name == "tablet" && p.online));
        assert_eq!(peers.len(), 3);
    }

    #[test]
    fn empty_and_error_text_is_empty_list() {
        assert!(parse_tailscale_status("").is_empty());
        assert!(parse_tailscale_status("   \n# only comments").is_empty());
        assert!(parse_tailscale_status("failed to connect").is_empty());
        assert!(parse_mdns("").is_empty());
    }

    #[test]
    fn mdns_records_merge() {
        let mdns = parse_mdns("kitchen-pi 10.0.0.9\n10.0.0.10 lounge");
        assert_eq!(mdns.len(), 2);
        let ts = parse_tailscale_status("100.1.1.1 desk linux -");
        let all = merge_peers(ts, mdns);
        assert!(all.len() >= 3);
    }
}
