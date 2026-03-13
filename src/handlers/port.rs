use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::ip_util;
use crate::response::PortResponse;
use crate::server::AppState;

pub async fn port_handler(
    State(state): State<AppState>,
    Path(port_str): Path<String>,
    headers: HeaderMap,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let port: u16 = port_str
        .parse()
        .map_err(|_| AppError::bad_request(format!("invalid port: {port_str}")).as_json())?;

    if port == 0 {
        return Err(AppError::bad_request(format!("invalid port: {port_str}")).as_json());
    }

    // Port lookup does NOT honor ?ip= parameter (security: no remote port scanning)
    let ip = ip_util::extract_ip(
        &state.config.trusted_headers,
        &headers,
        None,
        Some(remote.0.ip()),
    )
    .map_err(|e| AppError::bad_request(e).as_json())?;

    let reachable = ip_util::lookup_port(ip, port).await;

    let resp = PortResponse {
        ip,
        port,
        reachable,
    };

    let body = serde_json::to_string_pretty(&resp)
        .map_err(|e| AppError::internal(e.to_string()).as_json())?;

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json".to_string(),
        )],
        body,
    ))
}
