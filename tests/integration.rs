use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::http::StatusCode;
use echoip::config::Config;
use tokio::sync::RwLock;

mod helpers {
    use super::*;
    use echoip::cache::Cache;
    use echoip::config::Config;
    use echoip::geo::{Asn, City, Country, GeoProvider};
    use echoip::server::{AppState, build_router};

    pub struct TestDb;

    impl GeoProvider for TestDb {
        fn country(&self, _ip: IpAddr) -> Option<Country> {
            Some(Country {
                name: "Elbonia".into(),
                iso: "EB".into(),
                is_eu: false,
            })
        }

        fn city(&self, _ip: IpAddr) -> Option<City> {
            Some(City {
                name: "Bornyasherk".into(),
                region_name: "North Elbonia".into(),
                region_code: "1234".into(),
                metro_code: 1234,
                postal_code: "1234".into(),
                latitude: 63.416667,
                longitude: 10.416667,
                timezone: "Europe/Bornyasherk".into(),
            })
        }

        fn asn(&self, _ip: IpAddr) -> Option<Asn> {
            Some(Asn {
                number: 59795,
                organization: "Hosting4Real".into(),
            })
        }

        fn is_empty(&self) -> bool {
            false
        }
    }

    pub struct EmptyDb;

    impl GeoProvider for EmptyDb {
        fn country(&self, _ip: IpAddr) -> Option<Country> {
            None
        }
        fn city(&self, _ip: IpAddr) -> Option<City> {
            None
        }
        fn asn(&self, _ip: IpAddr) -> Option<Asn> {
            None
        }
        fn is_empty(&self) -> bool {
            true
        }
    }

    pub fn test_config() -> Config {
        Config {
            country_db: String::new(),
            city_db: String::new(),
            asn_db: String::new(),
            listen: ":8080".into(),
            reverse_lookup: true,
            port_lookup: true,
            template: String::new(),
            cache_size: 100,
            profile: false,
            sponsor: false,
            trusted_headers: vec![],
            ip66_db: None,
            data_dir: "data".into(),
            update_interval: 0,
            no_auto_download: true,
        }
    }

    pub async fn start_server(geo: Arc<dyn GeoProvider>, config: Config) -> String {
        start_server_with_tera(geo, config, None).await
    }

    pub async fn start_server_with_tera(
        geo: Arc<dyn GeoProvider>,
        config: Config,
        tera: Option<Arc<tera::Tera>>,
    ) -> String {
        let cache_size = config.cache_size;
        let state = AppState {
            config: Arc::new(config),
            geo,
            cache: Arc::new(RwLock::new(Cache::new(cache_size))),
            tera,
        };

        let app = build_router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });

        format!("http://{addr}")
    }

    pub async fn http_get(url: &str, accept: &str, user_agent: &str) -> (String, StatusCode) {
        let client = reqwest::Client::new();
        let mut req = client.get(url);
        if !accept.is_empty() {
            req = req.header("Accept", accept);
        }
        if !user_agent.is_empty() {
            req = req.header("User-Agent", user_agent);
        } else {
            req = req.header("User-Agent", "");
        }
        let resp = req.send().await.unwrap();
        let status = resp.status();
        let body = resp.text().await.unwrap();
        (body, StatusCode::from_u16(status.as_u16()).unwrap())
    }
}

use helpers::*;

#[tokio::test]
async fn test_cli_handlers() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let tests = vec![
        // (path, expected_body, expected_status, user_agent, accept)
        ("", "127.0.0.1\n", 200, "curl/7.43.0", ""),
        ("", "127.0.0.1\n", 200, "foo/bar", "text/plain"),
        ("/ip", "127.0.0.1\n", 200, "", ""),
        ("/country", "Elbonia\n", 200, "", ""),
        ("/country-iso", "EB\n", 200, "", ""),
        ("/coordinates", "63.416667,10.416667\n", 200, "", ""),
        ("/city", "Bornyasherk\n", 200, "", ""),
        ("/foo", "404 page not found", 404, "", ""),
        ("/asn", "AS59795\n", 200, "", ""),
        ("/asn-org", "Hosting4Real\n", 200, "", ""),
    ];

    for (path, expected_body, expected_status, ua, accept) in &tests {
        let url = format!("{base}{path}");
        let (body, status) = http_get(&url, accept, ua).await;
        assert_eq!(
            status.as_u16(),
            *expected_status as u16,
            "Status mismatch for {path}: got {status}"
        );
        assert_eq!(body, *expected_body, "Body mismatch for {path}");
    }
}

#[tokio::test]
async fn test_disabled_handlers() {
    let config = Config {
        port_lookup: false,
        reverse_lookup: false,
        ..test_config()
    };
    let base = start_server(Arc::new(EmptyDb), config).await;

    let tests = vec![
        ("/port/1337", "404 page not found", 404),
        ("/country", "404 page not found", 404),
        ("/country-iso", "404 page not found", 404),
        ("/city", "404 page not found", 404),
        (
            "/json",
            "{\n  \"ip\": \"127.0.0.1\",\n  \"ip_decimal\": 2130706433\n}",
            200,
        ),
    ];

    for (path, expected_body, expected_status) in &tests {
        let url = format!("{base}{path}");
        let (body, status) = http_get(&url, "", "").await;
        assert_eq!(
            status.as_u16(),
            *expected_status as u16,
            "Status mismatch for {path}: got {status}"
        );
        assert_eq!(body, *expected_body, "Body mismatch for {path}");
    }
}

#[tokio::test]
async fn test_json_handlers() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let tests = vec![
        (
            "",
            concat!(
                "{\n",
                "  \"ip\": \"127.0.0.1\",\n",
                "  \"ip_decimal\": 2130706433,\n",
                "  \"country\": \"Elbonia\",\n",
                "  \"country_iso\": \"EB\",\n",
                "  \"region_name\": \"North Elbonia\",\n",
                "  \"region_code\": \"1234\",\n",
                "  \"metro_code\": 1234,\n",
                "  \"zip_code\": \"1234\",\n",
                "  \"city\": \"Bornyasherk\",\n",
                "  \"latitude\": 63.416667,\n",
                "  \"longitude\": 10.416667,\n",
                "  \"time_zone\": \"Europe/Bornyasherk\",\n",
                "  \"asn\": \"AS59795\",\n",
                "  \"asn_org\": \"Hosting4Real\",\n",
                "  \"hostname\": \"localhost\",\n",
                "  \"user_agent\": {\n",
                "    \"product\": \"curl\",\n",
                "    \"version\": \"7.2.6.0\",\n",
                "    \"raw_value\": \"curl/7.2.6.0\"\n",
                "  }\n",
                "}"
            ),
            200,
        ),
        (
            "/port/foo",
            "{\n  \"status\": 400,\n  \"error\": \"invalid port: foo\"\n}",
            400,
        ),
        (
            "/port/0",
            "{\n  \"status\": 400,\n  \"error\": \"invalid port: 0\"\n}",
            400,
        ),
        (
            "/port/65537",
            "{\n  \"status\": 400,\n  \"error\": \"invalid port: 65537\"\n}",
            400,
        ),
        (
            "/foo",
            "{\n  \"status\": 404,\n  \"error\": \"404 page not found\"\n}",
            404,
        ),
        ("/health", "{\"status\":\"OK\"}", 200),
    ];

    for (path, expected_body, expected_status) in &tests {
        let url = format!("{base}{path}");
        let (body, status) = http_get(&url, "application/json", "curl/7.2.6.0").await;
        assert_eq!(
            status.as_u16(),
            *expected_status as u16,
            "Status mismatch for {path}: got {status}"
        );
        assert_eq!(body, *expected_body, "Body mismatch for {path}");
    }
}

#[tokio::test]
async fn test_cache_handler() {
    let config = Config {
        profile: true,
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    let (body, status) = http_get(&format!("{base}/debug/cache/"), "application/json", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        "{\n  \"size\": 0,\n  \"capacity\": 100,\n  \"evictions\": 0\n}"
    );
}

#[tokio::test]
async fn test_cache_resize_handler() {
    let config = Config {
        profile: true,
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/debug/cache/resize"))
        .body("10")
        .send()
        .await
        .unwrap();
    let body = resp.text().await.unwrap();
    assert_eq!(
        body,
        "{\n  \"message\": \"Changed cache capacity to 10.\"\n}"
    );
}

#[tokio::test]
async fn test_cli_matcher() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    // CLI user agents should get plain text IP
    let cli_agents = vec![
        "curl/7.26.0",
        "Wget/1.13.4 (linux-gnu)",
        "Wget",
        "fetch libfetch/2.0",
        "HTTPie/0.9.3",
        "httpie-go/0.6.0",
        "Go 1.1 package http",
        "Go-http-client/1.1",
        "Go-http-client/2.0",
        "ddclient/3.8.3",
        "Mikrotik/6.x Fetch",
    ];

    for ua in &cli_agents {
        let (body, status) = http_get(&base, "", ua).await;
        assert_eq!(status, StatusCode::OK, "Failed for UA: {ua}");
        assert_eq!(body, "127.0.0.1\n", "Failed for UA: {ua}");
    }
}

#[tokio::test]
async fn test_head_request() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let client = reqwest::Client::new();
    let resp = client.head(&base).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 204);
}

#[tokio::test]
async fn test_query_ip_override() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let (body, status) = http_get(&format!("{base}/ip?ip=8.8.8.8"), "", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "8.8.8.8\n");
}

#[tokio::test]
async fn test_json_query_ip() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let (body, status) = http_get(
        &format!("{base}/json?ip=8.8.8.8"),
        "application/json",
        "curl/7.0",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "8.8.8.8");
}

#[tokio::test]
async fn test_ipv6_handling() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let (body, status) = http_get(&format!("{base}/ip?ip=::1"), "", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "::1\n");
}

#[tokio::test]
async fn test_cache_hit_miss() {
    let config = Config {
        profile: true,
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    // First request: cache miss
    let (_, status) = http_get(&format!("{base}/json"), "application/json", "curl/7.0").await;
    assert_eq!(status, StatusCode::OK);

    // Check cache now has 1 entry
    let (body, _) = http_get(&format!("{base}/debug/cache/"), "application/json", "").await;
    let stats: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(stats["size"], 1);

    // Second request: cache hit (same IP)
    let (_, status) = http_get(&format!("{base}/json"), "application/json", "curl/7.0").await;
    assert_eq!(status, StatusCode::OK);

    // Cache should still have 1 entry (same IP)
    let (body, _) = http_get(&format!("{base}/debug/cache/"), "application/json", "").await;
    let stats: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(stats["size"], 1);
}

#[tokio::test]
async fn test_cache_disabled() {
    let config = Config {
        cache_size: 0,
        profile: true,
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    let (_, status) = http_get(&format!("{base}/json"), "application/json", "curl/7.0").await;
    assert_eq!(status, StatusCode::OK);

    let (body, _) = http_get(&format!("{base}/debug/cache/"), "application/json", "").await;
    let stats: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(stats["size"], 0);
    assert_eq!(stats["capacity"], 0);
}

#[tokio::test]
async fn test_trusted_header() {
    let config = Config {
        trusted_headers: vec!["X-Real-IP".to_string()],
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/ip"))
        .header("X-Real-IP", "10.0.0.1")
        .send()
        .await
        .unwrap();
    let body = resp.text().await.unwrap();
    assert_eq!(body, "10.0.0.1\n");
}

#[tokio::test]
async fn test_forwarded_for_first_ip() {
    let config = Config {
        trusted_headers: vec!["X-Forwarded-For".to_string()],
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/ip"))
        .header("X-Forwarded-For", "1.2.3.4, 5.6.7.8")
        .send()
        .await
        .unwrap();
    let body = resp.text().await.unwrap();
    assert_eq!(body, "1.2.3.4\n");
}

#[tokio::test]
async fn test_html_handler() {
    let template_dir = format!("{}/html", env!("CARGO_MANIFEST_DIR"));
    let glob = format!("{template_dir}/*");
    let tera = tera::Tera::new(&glob).expect("failed to load templates");

    let config = Config {
        template: template_dir,
        ..test_config()
    };
    let base = start_server_with_tera(Arc::new(TestDb), config, Some(Arc::new(tera))).await;

    // Browser-like request (no CLI UA, no explicit Accept)
    let client = reqwest::Client::new();
    let resp = client
        .get(&base)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .header("Accept", "text/html")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let body = resp.text().await.unwrap();
    assert!(content_type.contains("text/html"));
    assert!(body.contains("127.0.0.1"));
    assert!(body.contains("Elbonia"));
}

#[tokio::test]
async fn test_html_handler_with_query_ip() {
    let template_dir = format!("{}/html", env!("CARGO_MANIFEST_DIR"));
    let glob = format!("{template_dir}/*");
    let tera = tera::Tera::new(&glob).expect("failed to load templates");

    let config = Config {
        template: template_dir,
        ..test_config()
    };
    let base = start_server_with_tera(Arc::new(TestDb), config, Some(Arc::new(tera))).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/?ip=8.8.8.8"))
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept", "text/html")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("8.8.8.8"));
}

#[tokio::test]
async fn test_root_content_negotiation_json() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    // Root with Accept: application/json should return JSON
    let (body, status) = http_get(&base, "application/json", "Mozilla/5.0").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "127.0.0.1");
}

#[tokio::test]
async fn test_root_no_template_non_cli() {
    // No template configured + non-CLI user agent should return plain text IP
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(&base)
        .header("User-Agent", "Mozilla/5.0 (Macintosh)")
        .header("Accept", "*/*")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "127.0.0.1\n");
}

#[tokio::test]
async fn test_not_found_json() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let (body, status) = http_get(&format!("{base}/nonexistent"), "application/json", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["status"], 404);
    assert_eq!(json["error"], "404 page not found");
}

#[tokio::test]
async fn test_not_found_plain() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let (body, status) = http_get(&format!("{base}/nonexistent"), "text/plain", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, "404 page not found");
}

#[tokio::test]
async fn test_port_handler_reachable() {
    let config = Config {
        port_lookup: true,
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    // Test a port that should not be reachable on loopback
    let (body, status) = http_get(&format!("{base}/port/19283"), "application/json", "").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["port"], 19283);
    assert_eq!(json["reachable"], false);
}

#[tokio::test]
async fn test_health_endpoint() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    let (body, status) = http_get(&format!("{base}/health"), "", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, r#"{"status":"OK"}"#);
}

#[tokio::test]
async fn test_empty_db_json() {
    let config = Config {
        no_auto_download: true,
        ..test_config()
    };
    let base = start_server(Arc::new(EmptyDb), config).await;

    let (body, status) = http_get(&format!("{base}/json"), "application/json", "curl/7.0").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "127.0.0.1");
    // Empty DB should not include geo fields
    assert!(json.get("country").is_none());
    assert!(json.get("city").is_none());
    assert!(json.get("asn").is_none());
}

#[tokio::test]
async fn test_cache_resize_invalid() {
    let config = Config {
        profile: true,
        ..test_config()
    };
    let base = start_server(Arc::new(TestDb), config).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/debug/cache/resize"))
        .header("Accept", "application/json")
        .body("not-a-number")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400);
}

#[tokio::test]
async fn test_root_error_json() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    // Invalid ?ip= with Accept: application/json should return JSON error
    let (body, status) = http_get(
        &format!("{base}/?ip=invalid"),
        "application/json",
        "Mozilla/5.0",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["status"], 400);
}

#[tokio::test]
async fn test_root_error_cli() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    // Invalid ?ip= with CLI user-agent
    let (_, status) = http_get(&format!("{base}/?ip=invalid"), "", "curl/7.0").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_root_error_html() {
    let template_dir = format!("{}/html", env!("CARGO_MANIFEST_DIR"));
    let glob = format!("{template_dir}/*");
    let tera = tera::Tera::new(&glob).expect("failed to load templates");

    let config = Config {
        template: template_dir,
        ..test_config()
    };
    let base = start_server_with_tera(Arc::new(TestDb), config, Some(Arc::new(tera))).await;

    // Invalid ?ip= with HTML template and browser UA
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/?ip=invalid"))
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept", "text/html")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400);
}

#[tokio::test]
async fn test_root_error_no_template() {
    let base = start_server(Arc::new(TestDb), test_config()).await;

    // Invalid ?ip= with non-CLI UA and no template
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/?ip=invalid"))
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept", "*/*")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400);
}
