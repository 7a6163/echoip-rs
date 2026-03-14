use std::net::IpAddr;
use std::time::Duration;

use num_bigint::BigUint;
use tokio::net::TcpStream;
use tokio::time::timeout;

const DNS_TIMEOUT: Duration = Duration::from_secs(3);

pub fn to_decimal(ip: IpAddr) -> BigUint {
    match ip {
        IpAddr::V4(v4) => BigUint::from_bytes_be(&v4.octets()),
        IpAddr::V6(v6) => BigUint::from_bytes_be(&v6.octets()),
    }
}

pub async fn lookup_addr(ip: IpAddr) -> Option<String> {
    let result = timeout(
        DNS_TIMEOUT,
        tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&ip)),
    )
    .await;

    match result {
        Ok(Ok(Ok(hostname))) => {
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
    if s.starts_with('[') {
        if let Some((addr, _)) = s.strip_prefix('[').and_then(|s| s.split_once("]:")) {
            if let Ok(ip) = addr.parse::<IpAddr>() {
                return Ok(ip);
            }
        }
    }

    // Try stripping port from IPv4 addr:port (only if exactly one colon)
    if s.matches(':').count() == 1 {
        if let Some((host, _)) = s.rsplit_once(':') {
            if let Ok(ip) = host.parse::<IpAddr>() {
                return Ok(ip);
            }
        }
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
    if let Some(ip_str) = query_ip {
        if !ip_str.is_empty() {
            return parse_ip(ip_str);
        }
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
        assert_eq!(parse_ip("::1").unwrap(), "::1".parse::<IpAddr>().unwrap());
        assert!(parse_ip("").is_err());
    }

    #[test]
    fn test_ip_from_forwarded_for() {
        assert_eq!(ip_from_forwarded_for("1.3.3.7"), "1.3.3.7");
        assert_eq!(ip_from_forwarded_for("1.3.3.7,4.2.4.2"), "1.3.3.7");
        assert_eq!(ip_from_forwarded_for("1.3.3.7, 4.2.4.2"), "1.3.3.7");
    }

    #[test]
    fn test_extract_ip_from_query() {
        let headers = axum::http::HeaderMap::new();
        let result = extract_ip(&[], &headers, Some("8.8.8.8"), None);
        assert_eq!(result.unwrap(), "8.8.8.8".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_ip_from_remote() {
        let headers = axum::http::HeaderMap::new();
        let remote: IpAddr = "192.168.1.1".parse().unwrap();
        let result = extract_ip(&[], &headers, None, Some(remote));
        assert_eq!(result.unwrap(), remote);
    }

    #[test]
    fn test_extract_ip_from_trusted_header() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "10.0.0.1".parse().unwrap());
        let trusted = vec!["x-real-ip".to_string()];
        let result = extract_ip(&trusted, &headers, None, None);
        assert_eq!(result.unwrap(), "10.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_ip_forwarded_for() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        let trusted = vec!["x-forwarded-for".to_string()];
        let result = extract_ip(&trusted, &headers, None, None);
        assert_eq!(result.unwrap(), "1.2.3.4".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_ip_query_priority() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "10.0.0.1".parse().unwrap());
        let trusted = vec!["x-real-ip".to_string()];
        let remote: IpAddr = "192.168.1.1".parse().unwrap();
        // Query IP takes priority over header and remote
        let result = extract_ip(&trusted, &headers, Some("8.8.8.8"), Some(remote));
        assert_eq!(result.unwrap(), "8.8.8.8".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_parse_ip_whitespace() {
        assert_eq!(
            parse_ip("  127.0.0.1  ").unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_parse_ip_invalid() {
        assert!(parse_ip("not-an-ip").is_err());
        assert!(parse_ip("999.999.999.999").is_err());
    }

    #[test]
    fn test_parse_ip_ipv6_no_port() {
        assert_eq!(
            parse_ip("2001:db8::1").unwrap(),
            "2001:db8::1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_parse_ip_ipv6_bracket_only() {
        // Bracket but no port separator — should fail since [addr] isn't valid by itself
        assert!(parse_ip("[::1]").is_err());
    }

    #[test]
    fn test_extract_ip_empty_query() {
        let headers = axum::http::HeaderMap::new();
        let remote: IpAddr = "192.168.1.1".parse().unwrap();
        // Empty query string should fall through to remote
        let result = extract_ip(&[], &headers, Some(""), Some(remote));
        assert_eq!(result.unwrap(), remote);
    }

    #[test]
    fn test_extract_ip_empty_header_value() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "".parse().unwrap());
        let trusted = vec!["x-real-ip".to_string()];
        let remote: IpAddr = "192.168.1.1".parse().unwrap();
        // Empty header should be skipped, fall through to remote
        let result = extract_ip(&trusted, &headers, None, Some(remote));
        assert_eq!(result.unwrap(), remote);
    }

    #[test]
    fn test_extract_ip_no_source() {
        let headers = axum::http::HeaderMap::new();
        let result = extract_ip(&[], &headers, None, None);
        assert!(result.is_err());
    }
}
