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
    use echoip::server::{build_router, AppState};

    pub struct TestDb;

    #[async_trait::async_trait]
    impl GeoProvider for TestDb {
        async fn country(&self, _ip: IpAddr) -> Option<Country> {
            Some(Country {
                name: "Elbonia".into(),
                iso: "EB".into(),
                is_eu: false,
            })
        }

        async fn city(&self, _ip: IpAddr) -> Option<City> {
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

        async fn asn(&self, _ip: IpAddr) -> Option<Asn> {
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

    #[async_trait::async_trait]
    impl GeoProvider for EmptyDb {
        async fn country(&self, _ip: IpAddr) -> Option<Country> {
            None
        }
        async fn city(&self, _ip: IpAddr) -> Option<City> {
            None
        }
        async fn asn(&self, _ip: IpAddr) -> Option<Asn> {
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
            ip66: false,
            ip66_url: None,
        }
    }

    pub async fn start_server(geo: Arc<dyn GeoProvider>, config: Config) -> String {
        let state = AppState {
            config: Arc::new(config),
            geo,
            cache: Arc::new(RwLock::new(Cache::new(100))),
            tera: None,
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
        (
            "/coordinates",
            "63.416667,10.416667\n",
            200,
            "",
            "",
        ),
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
        ("/foo", "{\n  \"status\": 404,\n  \"error\": \"404 page not found\"\n}", 404),
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

    let (body, status) = http_get(
        &format!("{base}/debug/cache/"),
        "application/json",
        "",
    )
    .await;
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
