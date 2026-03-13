use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UserAgent {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub product: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub comment: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub raw_value: String,
}

pub fn parse(s: &str) -> UserAgent {
    if s.is_empty() {
        return UserAgent::default();
    }

    let (product, version, comment) = if let Some((before, after)) = s.split_once('/') {
        if after.starts_with(|c: char| c.is_ascii_digit()) {
            if let Some((ver, cmt)) = after.split_once(' ') {
                (before.to_string(), ver.to_string(), cmt.to_string())
            } else {
                (before.to_string(), after.to_string(), String::new())
            }
        } else {
            (before.to_string(), String::new(), after.to_string())
        }
    } else if let Some((before, after)) = s.split_once(' ') {
        (before.to_string(), String::new(), after.to_string())
    } else {
        (s.to_string(), String::new(), String::new())
    };

    UserAgent {
        product,
        version,
        comment,
        raw_value: s.to_string(),
    }
}

const CLI_PRODUCTS: &[&str] = &[
    "curl",
    "HTTPie",
    "httpie-go",
    "Wget",
    "fetch libfetch",
    "Go",
    "Go-http-client",
    "ddclient",
    "Mikrotik",
    "xh",
];

pub fn is_cli(ua_str: &str) -> bool {
    let ua = parse(ua_str);
    CLI_PRODUCTS.contains(&ua.product.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let tests = vec![
            ("", UserAgent::default()),
            (
                "curl/",
                UserAgent {
                    product: "curl".into(),
                    ..Default::default()
                },
            ),
            (
                "curl/foo",
                UserAgent {
                    product: "curl".into(),
                    comment: "foo".into(),
                    ..Default::default()
                },
            ),
            (
                "curl/7.26.0",
                UserAgent {
                    product: "curl".into(),
                    version: "7.26.0".into(),
                    ..Default::default()
                },
            ),
            (
                "Wget/1.13.4 (linux-gnu)",
                UserAgent {
                    product: "Wget".into(),
                    version: "1.13.4".into(),
                    comment: "(linux-gnu)".into(),
                    ..Default::default()
                },
            ),
            (
                "Wget",
                UserAgent {
                    product: "Wget".into(),
                    ..Default::default()
                },
            ),
            (
                "fetch libfetch/2.0",
                UserAgent {
                    product: "fetch libfetch".into(),
                    version: "2.0".into(),
                    ..Default::default()
                },
            ),
            (
                "Go 1.1 package http",
                UserAgent {
                    product: "Go".into(),
                    comment: "1.1 package http".into(),
                    ..Default::default()
                },
            ),
            (
                "Mikrotik/6.x Fetch",
                UserAgent {
                    product: "Mikrotik".into(),
                    version: "6.x".into(),
                    comment: "Fetch".into(),
                    ..Default::default()
                },
            ),
            (
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_8_4) \
                 AppleWebKit/537.36 (KHTML, like Gecko) Chrome/30.0.1599.28 \
                 Safari/537.36",
                UserAgent {
                    product: "Mozilla".into(),
                    version: "5.0".into(),
                    comment: "(Macintosh; Intel Mac OS X 10_8_4) \
                              AppleWebKit/537.36 (KHTML, like Gecko) Chrome/30.0.1599.28 \
                              Safari/537.36"
                        .into(),
                    ..Default::default()
                },
            ),
        ];

        for (input, expected) in &tests {
            let ua = parse(input);
            assert_eq!(ua.product, expected.product, "Product mismatch for {input:?}");
            assert_eq!(ua.version, expected.version, "Version mismatch for {input:?}");
            assert_eq!(ua.comment, expected.comment, "Comment mismatch for {input:?}");
        }
    }

    #[test]
    fn test_is_cli() {
        assert!(is_cli("curl/7.26.0"));
        assert!(is_cli("Wget/1.13.4 (linux-gnu)"));
        assert!(is_cli("Wget"));
        assert!(is_cli("fetch libfetch/2.0"));
        assert!(is_cli("HTTPie/0.9.3"));
        assert!(is_cli("httpie-go/0.6.0"));
        assert!(is_cli("Go 1.1 package http"));
        assert!(is_cli("Go-http-client/1.1"));
        assert!(is_cli("Go-http-client/2.0"));
        assert!(is_cli("ddclient/3.8.3"));
        assert!(is_cli("Mikrotik/6.x Fetch"));
        assert!(is_cli("xh/0.1.0"));
        assert!(!is_cli(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_8_4) \
             AppleWebKit/537.36 (KHTML, like Gecko) Chrome/30.0.1599.28 Safari/537.36"
        ));
    }
}
