pub mod casters;
pub mod waiting;

use super::auth::AppError;
use super::sse::{SSEDestination, SSEvent, SSEventType};
use super::AppState;
use crate::routes::views::filters;
use crate::startgg::auth::AuthSession;
use crate::startgg::tournaments::StartGGTeam;
use crate::startgg::StartGGClient;
use askama::Template;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::Form;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[axum::debug_handler]
pub async fn ingame_overlay(
    _s: State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Html(IngameOverlayTemplate { overlay_id }.render()?))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamForm {
    team_a: String,
    team_b: String,
    // score_a: String,
    // score_b: String,
}
#[axum::debug_handler]
pub async fn update_ingame_scoreboard(
    state: State<Arc<AppState>>,
    auth_session: AuthSession,
    Path((_tournament_slug, overlay_id)): Path<(String, Uuid)>,
    Form(form): Form<UpdateTeamForm>,
) -> Result<impl IntoResponse, AppError> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let tournaments = startgg_client
        .fetch_tournaments_organized_by_user()
        .await
        .map_err(|e| AppError(e.to_string()))?;

    let overlay = state.db.get_overlay(overlay_id).await?;

    if !tournaments
        .iter()
        .any(|t| t.slug == overlay.tournament_slug)
    {
        return Err(AppError(
            "You are not allowed to update this overlay".to_string(),
        ));
    };

    let team_a = state
        .db
        .get_team(&form.team_a)
        .await
        .map_err(|e| AppError(e.to_string()))?;
    let team_b = state
        .db
        .get_team(&form.team_b)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    let description = format!("{} vs {}", team_a.name, team_b.name);

    state.events_sender.send(SSEvent {
        destination: SSEDestination::Channel(format!("overlay_{}", overlay.id)),
        event: SSEventType::IngameOverlayUpdate,
        data: ScoreboardTemplate {
            team_a,
            team_b,
            team_a_score: 1,
            team_a_standing: "1 - 0".to_string(),
            team_b_score: 1,
            team_b_standing: "2 - 3".to_string(),
        }
        .render()?,
    })?;

    Ok(description)
}

#[derive(Template)]
#[template(path = "stream_overlays/ingame.html")]
pub struct IngameOverlayTemplate {
    pub overlay_id: Uuid,
}

#[derive(Template)]
#[template(path = "stream_overlays/scoreboard.html")]
pub struct ScoreboardTemplate {
    pub team_a: StartGGTeam,
    pub team_a_score: i32,
    pub team_a_standing: String,
    pub team_b: StartGGTeam,
    pub team_b_score: i32,
    pub team_b_standing: String,
}
