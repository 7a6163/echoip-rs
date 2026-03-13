use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::server::AppState;

pub async fn json_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let resp = crate::server::build_response(
        &state,
        &headers,
        query.get("ip").map(|s| s.as_str()),
        Some(remote.0.ip()),
    )
    .await?;

    let body = serde_json::to_string_pretty(&resp)
        .map_err(|e| AppError::internal(e.to_string()).into_json())?;

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json".to_string(),
        )],
        body,
    ))
}
