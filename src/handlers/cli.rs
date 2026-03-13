use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::server::AppState;

fn format_coordinate(c: f64) -> String {
    format!("{c:.6}")
}

async fn build_response(
    state: &AppState,
    headers: &HeaderMap,
    query_ip: Option<&str>,
    remote_addr: Option<std::net::IpAddr>,
) -> Result<crate::response::Response, AppError> {
    crate::server::build_response(state, headers, query_ip, remote_addr).await
}

pub async fn ip_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let query_ip = query.get("ip").map(|s| s.as_str());
    let ip = crate::ip_util::extract_ip(
        &state.config.trusted_headers,
        &headers,
        query_ip,
        Some(remote.0.ip()),
    )
    .map_err(|e| AppError::bad_request(&e).into_json())?;
    Ok(format!("{ip}\n"))
}

pub async fn country_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;
    Ok(format!("{}\n", resp.country))
}

pub async fn country_iso_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;
    Ok(format!("{}\n", resp.country_iso))
}

pub async fn city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;
    Ok(format!("{}\n", resp.city))
}

pub async fn coordinates_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;
    Ok(format!(
        "{},{}\n",
        format_coordinate(resp.latitude),
        format_coordinate(resp.longitude)
    ))
}

pub async fn asn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;
    Ok(format!("{}\n", resp.asn))
}

pub async fn asn_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;
    Ok(format!("{}\n", resp.asn_org))
}
