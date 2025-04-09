use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

use crate::startgg::auth::AuthSession;
use crate::startgg::oauth::StartggUser;
use crate::startgg::StartGGClient;

use super::auth::AppError;
use super::views::filters;
use super::AppState;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub maybe_user: Option<StartggUser>,
}

#[axum::debug_handler]
pub async fn index_handler(
    State(state): State<Arc<AppState>>,
    auth_session: Option<AuthSession>, // Use optional extractor
) -> Result<impl IntoResponse, AppError> {
    // If AuthSession exists, try to get user data for display
    let user = if let Some(session) = auth_session {
        StartGGClient::new(&state.http_client, &session.access_token)
            .fetch_startgg_user()
            .await
            .ok()
    } else {
        None
    };

    Ok(Html(IndexTemplate { maybe_user: user }.render()?))
}
