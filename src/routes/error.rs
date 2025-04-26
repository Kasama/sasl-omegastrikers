use std::fmt::Display;

use axum::response::IntoResponse;
use reqwest::StatusCode;

pub struct AppError {
    pub error: String,
    pub status_code: Option<StatusCode>,
}

impl AppError {
    pub fn with_status(self, status_code: StatusCode) -> Self {
        AppError {
            error: self.error,
            status_code: Some(status_code),
        }
    }

    pub fn with_unauthorized(self) -> Self {
        self.with_status(StatusCode::UNAUTHORIZED)
    }
}

impl<S: Display> From<S> for AppError {
    fn from(value: S) -> Self {
        AppError {
            error: value.to_string(),
            status_code: None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status_code
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            self.error,
        )
            .into_response()
    }
}
