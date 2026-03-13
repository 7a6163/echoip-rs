use axum::extract::State;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::server::AppState;

pub async fn cache_handler(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let cache = state.cache.read().await;
    let stats = cache.stats();

    let body = serde_json::to_string_pretty(&stats)
        .map_err(|e| AppError::internal(e.to_string()).into_json())?;

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json".to_string(),
        )],
        body,
    ))
}

pub async fn cache_resize_handler(
    State(state): State<AppState>,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    let capacity: usize = body.trim().parse().map_err(|_| {
        AppError::bad_request(format!("invalid capacity: {}", body.trim())).into_json()
    })?;

    {
        let mut cache = state.cache.write().await;
        cache.resize(capacity);
    }

    let resp = serde_json::json!({
        "message": format!("Changed cache capacity to {capacity}.")
    });

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
