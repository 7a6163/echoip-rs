use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;

use crate::cache::Cache;
use crate::config::Config;
use crate::error::AppError;
use crate::geo::GeoProvider;
use crate::ip_util;
use crate::response;
use crate::user_agent;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub geo: Arc<dyn GeoProvider>,
    pub cache: Arc<RwLock<Cache>>,
    pub tera: Option<Arc<tera::Tera>>,
}

pub async fn build_response(
    state: &AppState,
    headers: &HeaderMap,
    query_ip: Option<&str>,
    remote_addr: Option<IpAddr>,
) -> Result<response::Response, AppError> {
    let ip = ip_util::extract_ip(&state.config.trusted_headers, headers, query_ip, remote_addr)
        .map_err(|e| AppError::bad_request(&e).as_json())?;

    // Check cache
    {
        let mut cache = state.cache.write().await;
        if let Some(mut cached) = cache.get(ip) {
            // Do not cache user agent
            let ua_str = headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            cached.user_agent = if ua_str.is_empty() {
                None
            } else {
                Some(user_agent::parse(ua_str))
            };
            return Ok(cached);
        }
    }

    let ip_decimal = ip_util::to_decimal(ip);
    let country = state.geo.country(ip).await.unwrap_or_default();
    let city = state.geo.city(ip).await.unwrap_or_default();
    let asn = state.geo.asn(ip).await.unwrap_or_default();

    let hostname = if state.config.reverse_lookup {
        ip_util::lookup_addr(ip).await.unwrap_or_default()
    } else {
        String::new()
    };

    let asn_str = if asn.number > 0 {
        format!("AS{}", asn.number)
    } else {
        String::new()
    };

    let resp = response::Response {
        ip,
        ip_decimal,
        country: country.name,
        country_iso: country.iso,
        country_eu: country.is_eu,
        region_name: city.region_name,
        region_code: city.region_code,
        metro_code: city.metro_code,
        zip_code: city.postal_code,
        city: city.name,
        latitude: city.latitude,
        longitude: city.longitude,
        time_zone: city.timezone,
        asn: asn_str,
        asn_org: asn.organization,
        hostname,
        user_agent: None,
    };

    // Store in cache (without user_agent)
    {
        let mut cache = state.cache.write().await;
        cache.set(ip, resp.clone());
    }

    // Add user agent (not cached)
    let ua_str = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut resp = resp;
    resp.user_agent = if ua_str.is_empty() {
        None
    } else {
        Some(user_agent::parse(ua_str))
    };

    Ok(resp)
}

/// Root handler that does content negotiation based on Accept header and User-Agent
async fn root_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Response {
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("application/json") {
        return match crate::handlers::json::json_handler(
            State(state),
            headers,
            query,
            remote,
        )
        .await
        {
            Ok(resp) => resp.into_response(),
            Err(e) => e.into_response(),
        };
    }

    if user_agent::is_cli(ua) || accept.contains("text/plain") {
        return match crate::handlers::cli::ip_handler(
            State(state),
            headers,
            query,
            remote,
        )
        .await
        {
            Ok(resp) => resp.into_response(),
            Err(e) => e.into_response(),
        };
    }

    // Default: if template is configured, serve HTML; otherwise show IP
    if state.tera.is_some() {
        return match crate::handlers::html::html_handler(
            State(state),
            headers,
            query,
            remote,
        )
        .await
        {
            Ok(resp) => resp.into_response(),
            Err(e) => e.into_response(),
        };
    }

    // No template: just return IP as plain text
    match crate::handlers::cli::ip_handler(State(state), headers, query, remote).await {
        Ok(resp) => resp.into_response(),
        Err(e) => e.into_response(),
    }
}

pub fn build_router(state: AppState) -> Router {
    let mut app = Router::new();

    // Health
    app = app.route("/health", get(health_handler));

    // Root: GET with content negotiation + HEAD
    app = app.route("/", get(root_handler).head(head_handler));

    // JSON
    app = app.route("/json", get(crate::handlers::json::json_handler));

    // CLI endpoints
    app = app.route("/ip", get(crate::handlers::cli::ip_handler));

    // Always register geo routes — databases may be loaded later via auto-download
    if !state.geo.is_empty() || !state.config.no_auto_download {
        app = app
            .route("/country", get(crate::handlers::cli::country_handler))
            .route(
                "/country-iso",
                get(crate::handlers::cli::country_iso_handler),
            )
            .route("/city", get(crate::handlers::cli::city_handler))
            .route(
                "/coordinates",
                get(crate::handlers::cli::coordinates_handler),
            )
            .route("/asn", get(crate::handlers::cli::asn_handler))
            .route("/asn-org", get(crate::handlers::cli::asn_org_handler));
    }

    // Port testing
    if state.config.port_lookup {
        app = app.route("/port/{port}", get(crate::handlers::port::port_handler));
    }

    // Debug/profiling
    if state.config.profile {
        app = app
            .route("/debug/cache/", get(crate::handlers::debug::cache_handler))
            .route(
                "/debug/cache/resize",
                post(crate::handlers::debug::cache_resize_handler),
            );
    }

    // Fallback
    app = app.fallback(not_found_handler);

    app.with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        r#"{"status":"OK"}"#,
    )
}

async fn head_handler() -> impl IntoResponse {
    axum::http::StatusCode::NO_CONTENT
}

async fn not_found_handler(headers: HeaderMap) -> impl IntoResponse {
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("application/json") {
        AppError::not_found("404 page not found")
            .as_json()
            .into_response()
    } else {
        AppError::not_found("404 page not found").into_response()
    }
}
