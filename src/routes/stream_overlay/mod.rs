pub mod casters;
pub mod waiting;

use super::error::AppError;
use super::sse::{SSEDestination, SSEvent, SSEventType};
use super::tournament::{get_tournament_teams, TeamsSetup};
use super::AppState;
use crate::database::scoreboard::Scoreboard;
use crate::routes::views::filters;
use crate::startgg::auth::AuthSession;
use crate::startgg::tournaments::StartGGTeam;
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

pub async fn get_scoreboard(
    state: Arc<AppState>,
    scoreboard: Scoreboard,
) -> anyhow::Result<ScoreboardTemplate> {
    let team_a = state.db.get_team(&scoreboard.team_a).await?;
    let team_b = state.db.get_team(&scoreboard.team_b).await?;

    Ok(ScoreboardTemplate {
        team_a,
        team_b,
        team_a_score: scoreboard.team_a_score,
        team_a_standing: scoreboard.team_a_standing,
        team_b_score: scoreboard.team_b_score,
        team_b_standing: scoreboard.team_b_standing,
    })
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamForm {
    team_a: String,
    team_b: String,
    #[serde(default)]
    team_a_score: i32,
    #[serde(default)]
    team_b_score: i32,
}
#[axum::debug_handler]
pub async fn update_ingame_scoreboard(
    State(state): State<Arc<AppState>>,
    auth_session: AuthSession,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    Form(form): Form<UpdateTeamForm>,
) -> Result<impl IntoResponse, AppError> {
    let overlay = state.db.get_overlay(overlay_id).await?;

    let scoreboard = Scoreboard {
        overlay_id,
        team_a: form.team_a,
        team_b: form.team_b,
        team_a_score: form.team_a_score,
        team_b_score: form.team_b_score,
        team_a_standing: "0-0".to_string(),
        team_b_standing: "0-0".to_string(),
    };

    let scoreboard = state.db.upsert_scoreboard(scoreboard).await?;

    let team_a = state.db.get_team(&scoreboard.team_a).await?;
    let team_b = state.db.get_team(&scoreboard.team_b).await?;

    state.events_sender.send(SSEvent {
        destination: SSEDestination::Channel(format!("overlay_{}", overlay.id)),
        event: SSEventType::IngameOverlayUpdate,
        data: ScoreboardTemplate {
            team_a: team_a.clone(),
            team_b: team_b.clone(),
            team_a_score: scoreboard.team_a_score,
            team_a_standing: scoreboard.team_a_standing.clone(),
            team_b_score: scoreboard.team_b_score,
            team_b_standing: scoreboard.team_b_standing.clone(),
        }
        .render()?,
    })?;

    let teams = get_tournament_teams(state, &auth_session, &tournament_slug).await?;

    Ok(TeamsSetup {
        teams,
        overlay_id,
        tournament_slug,
        selected_teams: Some((team_a, team_b, scoreboard)),
    }
    .render()?)
}

#[axum::debug_handler]
pub async fn ingame_scoreboard(
    State(state): State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let scoreboard = state.db.get_scoreboard(overlay_id).await?;

    Ok(get_scoreboard(state.clone(), scoreboard).await?.render()?)
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
