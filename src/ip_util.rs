use std::net::IpAddr;
use std::time::Duration;

use num_bigint::BigUint;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub fn to_decimal(ip: IpAddr) -> BigUint {
    match ip {
        IpAddr::V4(v4) => BigUint::from_bytes_be(&v4.octets()),
        IpAddr::V6(v6) => BigUint::from_bytes_be(&v6.octets()),
    }
}

pub async fn lookup_addr(ip: IpAddr) -> Option<String> {
    let ip_str = ip.to_string();
    let result = tokio::task::spawn_blocking(move || {
        dns_lookup::lookup_addr(&ip_str.parse().unwrap())
    })
    .await;

    match result {
        Ok(Ok(hostname)) => {
            let trimmed = hostname.trim_end_matches('.');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        _ => None,
    }
}

pub async fn lookup_port(ip: IpAddr, port: u16) -> bool {
    let addr = std::net::SocketAddr::new(ip, port);
    timeout(Duration::from_secs(2), TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

pub fn ip_from_forwarded_for(value: &str) -> &str {
    value.split_once(',').map_or(value, |(first, _)| first)
}

pub fn parse_ip(s: &str) -> Result<IpAddr, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty IP string".into());
    }

    // Try direct parse first
    if let Ok(ip) = s.parse::<IpAddr>() {
        return Ok(ip);
    }

    // Try stripping port from IPv6 [addr]:port
    if s.starts_with('[')
        && let Some((addr, _)) = s.strip_prefix('[').and_then(|s| s.split_once("]:"))
            && let Ok(ip) = addr.parse::<IpAddr>() {
                return Ok(ip);
            }

    // Try stripping port from IPv4 addr:port (only if exactly one colon)
    if s.matches(':').count() == 1
        && let Some((host, _)) = s.rsplit_once(':')
            && let Ok(ip) = host.parse::<IpAddr>() {
                return Ok(ip);
            }

    Err(format!("could not parse IP: {s}"))
}

pub fn extract_ip(
    headers: &[String],
    header_map: &axum::http::HeaderMap,
    query_ip: Option<&str>,
    remote_addr: Option<IpAddr>,
) -> Result<IpAddr, String> {
    // Query parameter takes priority
    if let Some(ip_str) = query_ip
        && !ip_str.is_empty() {
            return parse_ip(ip_str);
        }

    // Trusted headers
    for header in headers {
        if let Some(value) = header_map.get(header.as_str()) {
            let value_str = value.to_str().unwrap_or("");
            if value_str.is_empty() {
                continue;
            }
            let ip_str = if header.eq_ignore_ascii_case("x-forwarded-for") {
                ip_from_forwarded_for(value_str)
            } else {
                value_str
            };
            return parse_ip(ip_str);
        }
    }

    // Remote address
    remote_addr.ok_or_else(|| "no IP address found".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_decimal() {
        let tests = vec![
            ("127.0.0.1", "2130706433"),
            ("::1", "1"),
            ("8000::", "170141183460469231731687303715884105728"),
        ];
        for (input, expected) in tests {
            let ip: IpAddr = input.parse().unwrap();
            let decimal = to_decimal(ip);
            assert_eq!(decimal.to_string(), expected, "Failed for {input}");
        }
    }

    #[test]
    fn test_parse_ip() {
        assert_eq!(
            parse_ip("127.0.0.1").unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            parse_ip("1.3.3.7:1337").unwrap(),
            "1.3.3.7".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            parse_ip("[::ffff:103:307]:1337").unwrap(),
            "::ffff:103:307".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            parse_ip("::1").unwrap(),
            "::1".parse::<IpAddr>().unwrap()
        );
        assert!(parse_ip("").is_err());
    }

    #[test]
    fn test_ip_from_forwarded_for() {
        assert_eq!(ip_from_forwarded_for("1.3.3.7"), "1.3.3.7");
        assert_eq!(ip_from_forwarded_for("1.3.3.7,4.2.4.2"), "1.3.3.7");
        assert_eq!(ip_from_forwarded_for("1.3.3.7, 4.2.4.2"), "1.3.3.7");
    }
}
