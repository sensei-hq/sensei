//! Which addresses are on the public internet (CodeQL #1; tracker spec §G5.3).
//!
//! Defined positively for IPv6 (only global unicast `2000::/3`) and by an explicit deny
//! list for IPv4. An IPv4-mapped IPv6 address is judged as its IPv4 address.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Whether `ip` is a public, globally routed address a client may connect to.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => is_public_v6(v6),
        },
    }
}

fn is_public_v4(v4: Ipv4Addr) -> bool {
    let [a, b, c, _] = v4.octets();
    !(a == 0                                   // 0.0.0.0/8, unspecified
        || v4.is_loopback()
        || v4.is_private()                     // RFC 1918
        || (a == 100 && (64..128).contains(&b)) // CGNAT 100.64.0.0/10
        || v4.is_link_local()                  // 169.254.0.0/16
        || (a == 192 && b == 0 && c == 0)      // 192.0.0.0/24
        || (a == 198 && (b == 18 || b == 19))  // 198.18.0.0/15
        || (a == 192 && b == 88 && c == 99)    // 192.88.99.0/24
        || a >= 240                            // 240.0.0.0/4, broadcast included
        || v4.is_multicast()
        || v4.is_documentation())
}

fn is_public_v6(v6: Ipv6Addr) -> bool {
    let s = v6.segments();
    let global_unicast = (s[0] & 0xe000) == 0x2000; // 2000::/3
    global_unicast
        && s[0] != 0x2002                              // 6to4 2002::/16
        && !(s[0] == 0x2001 && s[1] == 0)              // Teredo 2001::/32
        && !(s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0010) // ORCHID 2001:10::/28
        && !(s[0] == 0x2001 && s[1] == 0x0002 && s[2] == 0) // benchmarking 2001:2::/48
        && !(s[0] == 0x2001 && s[1] == 0x0db8) // documentation 2001:db8::/32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public(ip: &str) -> bool {
        is_public_ip(ip.parse().unwrap())
    }

    #[test]
    fn only_public_addresses_are_public() {
        for ip in [
            // The library fetcher's original cases (CodeQL #1).
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.5",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "::1",
            "::",
            "fe80::1",
            "fd00::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            // Tracker spec §G5.3 / D13.
            "10.0.0.5",
            "::ffff:169.254.169.254",
            "0.1.2.3",
            "64:ff9b::a00:1",
            "64:ff9b:1::1",
            "fd00:ec2::254",
            "fd12::1",
            "100.100.100.200",
            "2002:7f00:1::",
            "2001:0:4136:e378::1",
            "2001:10::1",
            "2001:2::1",
            "2001:db8::1",
            "fec0::1",
            "ff02::1",
            "192.0.0.1",
            "198.18.0.1",
            "198.19.255.255",
            "192.88.99.1",
            "240.0.0.1",
            "192.0.2.1",
            "198.51.100.1",
            "203.0.113.1",
            "::127.0.0.1",
        ] {
            assert!(!public(ip), "{ip} must not count as public");
        }
        for ip in [
            "93.184.216.34",
            "1.1.1.1",
            "2606:4700:4700::1111",
            "100.128.0.1",
            "198.20.0.1",
            "2001:4860:4860::8888",
            "2001:3::1",
            "::ffff:1.1.1.1",
        ] {
            assert!(public(ip), "{ip} is public");
        }
    }
}
