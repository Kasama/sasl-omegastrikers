use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};

use crate::startgg::auth::AuthSession;
use crate::startgg::StartGGClient;

use super::auth::AppError;
use super::AppState;

use crate::startgg::oauth::StartggUser;
use crate::startgg::tournaments::{StartGGTeam, StartGGTournament};

use super::views::filters;

#[derive(Template)]
#[template(path = "tournaments.html")]
pub struct TournamentsTemplate {
    pub maybe_user: Option<StartggUser>,
    pub tournaments: Vec<StartGGTournament>,
}

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

#[derive(Template)]
#[template(path = "match_setup.html")]
pub struct MatchSetup {
    pub maybe_user: Option<StartggUser>,
    pub tournament: StartGGTournament,
    pub teams: Vec<StartGGTeam>,
}

#[axum::debug_handler]
pub async fn manage_handler(
    State(state): State<Arc<AppState>>,
    Path(tournament_slug): Path<String>,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let user = startgg_client
        .fetch_startgg_user()
        .await
        .map_err(|e| AppError(e.to_string()))?;

    let tournament = startgg_client
        .fetch_tournament(tournament_slug.to_string())
        .await
        .map_err(|e| AppError(e.to_string()))?;

    let teams = startgg_client
        .fetch_tournament_teams(tournament_slug)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    Ok(Html(
        MatchSetup {
            maybe_user: Some(user),
            tournament,
            teams,
        }
        .render()?,
    ))
}
