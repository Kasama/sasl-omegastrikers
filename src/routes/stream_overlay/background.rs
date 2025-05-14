use askama::Template;
use axum::extract::Path;
use axum::response::{Html, IntoResponse};
use uuid::Uuid;

use crate::routes::error::AppError;

#[axum::debug_handler]
pub async fn background(Path(overlay_id): Path<Uuid>) -> Result<impl IntoResponse, AppError> {
    Ok(Html(Background { overlay_id }.render()?))
}

#[derive(Template)]
#[template(path = "stream_overlays/background.html")]
pub struct Background {
    pub overlay_id: Uuid,
}
