use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};

use crate::startgg::auth::AuthSession;
use crate::startgg::StartGGClient;
use crate::views::tournaments::TournamentsTemplate;

use super::auth::AppError;
use super::AppState;

#[axum::debug_handler]
pub async fn tournaments_handler(
    State(state): State<Arc<AppState>>,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    // If AuthSession exists, try to get user data for display
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let user = startgg_client
        .fetch_startgg_user()
        .await
        .map_err(|e| AppError(e.to_string()))?;
    let tournaments = startgg_client
        .fetch_tournaments_organized_by_user()
        .await
        .map_err(|e| AppError(e.to_string()))?;

    Ok(Html(
        TournamentsTemplate {
            maybe_user: Some(user),
            tournaments,
        }
        .render()?,
    ))
}

#[axum::debug_handler]
pub async fn manage_handler(
    State(_state): State<Arc<AppState>>,
    Path(tournament_slug): Path<String>,
    _auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    // let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);

    Ok(Html(
        format!("Chaning tournament: {tournament_slug}")
    ))
}
