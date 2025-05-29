use std::sync::Arc;

use super::error::AppError;
use super::views::filters;
use super::AppState;
use crate::startgg::auth::AuthSession;
use crate::startgg::oauth::StartggUser;
use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "obs.html")]
pub struct ObsTemplate {
    maybe_user: Option<StartggUser>
}

#[axum::debug_handler]
pub async fn obs_page(
    State(_state): State<Arc<AppState>>,
    _auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {

    Ok(Html(ObsTemplate { maybe_user: None }.render()?))
}
