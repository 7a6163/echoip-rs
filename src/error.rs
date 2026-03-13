use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    status: u16,
    error: String,
}

pub struct AppError {
    pub code: StatusCode,
    pub message: String,
    pub content_type: Option<String>,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST,
            message: message.into(),
            content_type: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::NOT_FOUND,
            message: message.into(),
            content_type: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
            content_type: None,
        }
    }

    pub fn into_json(mut self) -> Self {
        self.content_type = Some("application/json".to_string());
        self
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = if self.content_type.as_deref() == Some("application/json") {
            let err_body = ErrorBody {
                status: self.code.as_u16(),
                error: self.message,
            };
            serde_json::to_string_pretty(&err_body).unwrap_or_default()
        } else {
            self.message
        };

        let content_type = self
            .content_type
            .unwrap_or_else(|| "text/plain".to_string());

        (
            self.code,
            [(axum::http::header::CONTENT_TYPE, content_type)],
            body,
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_request() {
        let err = AppError::bad_request("test error");
        assert_eq!(err.code, StatusCode::BAD_REQUEST);
        assert_eq!(err.message, "test error");
        assert!(err.content_type.is_none());
    }

    #[test]
    fn test_not_found() {
        let err = AppError::not_found("not found");
        assert_eq!(err.code, StatusCode::NOT_FOUND);
        assert_eq!(err.message, "not found");
    }

    #[test]
    fn test_internal() {
        let err = AppError::internal("server error");
        assert_eq!(err.code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message, "server error");
    }

    #[test]
    fn test_into_json() {
        let err = AppError::bad_request("test").into_json();
        assert_eq!(err.content_type.as_deref(), Some("application/json"));
    }
}
