use axum::http::{self, HeaderMap, StatusCode};

pub mod filters;
pub mod index;
pub mod tournaments;

pub struct ViewError {
    pub status_code: Option<StatusCode>,
    pub content: String,
}

type ViewResult = Result<String, ViewError>;

struct HtmlPage(ViewResult);

impl axum::response::IntoResponse for HtmlPage {
    fn into_response(self) -> axum::response::Response {
        let status = if let Err(ref e) = self.0 {
            e.status_code
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR)
        } else {
            http::StatusCode::OK
        };

        let str = self.0.unwrap_or_else(|e| e.content);

        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/html; charset=utf-8"),
        );

        (status, headers, str).into_response()
    }
}

impl axum::response::IntoResponse for ViewError {
    fn into_response(self) -> axum::response::Response {
        let html_page: HtmlPage = self.into();
        html_page.into_response()
    }
}

impl From<String> for HtmlPage {
    fn from(value: String) -> Self {
        HtmlPage(Ok(value))
    }
}

impl From<ViewError> for HtmlPage {
    fn from(value: ViewError) -> Self {
        HtmlPage(Err(value))
    }
}

impl From<askama::Error> for HtmlPage {
    fn from(value: askama::Error) -> Self {
        HtmlPage(Err(ViewError {
            status_code: Some(http::StatusCode::INTERNAL_SERVER_ERROR),
            content: value.to_string(),
        }))
    }
}
