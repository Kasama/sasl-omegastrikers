pub mod background;
pub mod casters;
pub mod partial;
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
use axum::extract::ws::{self, WebSocket};
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::Form;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;
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

pub async fn get_championhip_phase(
    scoreboard: Scoreboard,
    overlay_id: Uuid,
) -> anyhow::Result<ChampionshipPhaseTemplate> {
    Ok(ChampionshipPhaseTemplate {
        overlay_id,
        championship_phase: scoreboard.championship_phase,
    })
}

#[axum::debug_handler]
pub async fn overlay_ws(
    State(state): State<Arc<AppState>>,
    ws: ws::WebSocketUpgrade,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, String> {
    let conn_id = Uuid::new_v4();
    tracing::debug!(
        "Incoming websocket connection for overlay {}: {}",
        overlay_id,
        conn_id
    );
    Ok(ws.on_upgrade(move |socket| handle_websocket(socket, state, overlay_id, conn_id)))
}

async fn handle_websocket(
    socket: WebSocket,
    state: Arc<AppState>,
    overlay_id: Uuid,
    conn_id: Uuid,
) {
    let (sender, _receiver) = socket.split();
    let sender_arc = Arc::new(Mutex::new(sender));

    let events = state.events_sender.subscribe();

    let channel_id = format!("overlay_{}", overlay_id);
    let event_stream = BroadcastStream::new(events)
        .filter_map(|res| async { res.ok() })
        .filter_map(|event: SSEvent| async {
            match &event.destination {
                SSEDestination::Everyone => Some(event),
                SSEDestination::Channel(c) => {
                    if c == &channel_id {
                        Some(event)
                    } else {
                        None
                    }
                }
            }
        })
        .filter_map(|ev| async move {
            match ev.event {
                SSEventType::WebsocketEvent => Some(ev.data),
                _ => None,
            }
        });

    event_stream
        .for_each_concurrent(None, move |data| {
            let msg = ws::Message::Text(data.clone().into());
            let s = sender_arc.clone();
            async move {
                let mut sender = s.lock().await;
                if let Err(e) = sender.send(msg).await {
                    tracing::error!("Failed to send websocket message: {}", e);
                } else {
                    tracing::debug!("Sent websocket message for {}: {}", conn_id, data);
                };
            }
        })
        .await;
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamForm {
    team_a: String,
    team_b: String,
    #[serde(default)]
    team_a_score: i32,
    #[serde(default)]
    team_b_score: i32,
    team_a_standing: Option<String>,
    team_b_standing: Option<String>,
    championship_phase: Option<String>,
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
        team_a_standing: form.team_a_standing.unwrap_or("0-0".to_string()),
        team_b_standing: form.team_b_standing.unwrap_or("0-0".to_string()),
        championship_phase: form.championship_phase,
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

    state.events_sender.send(SSEvent {
        destination: SSEDestination::Channel(format!("overlay_{}", overlay.id)),
        event: SSEventType::ChampionshipPhaseUpdate,
        data: ChampionshipPhaseTemplate {
            overlay_id,
            championship_phase: scoreboard.championship_phase.clone(),
        }
        .as_phase()
        .render()?,
    })?;

    state.events_sender.send(SSEvent {
        destination: SSEDestination::Channel(format!("overlay_{}", overlay.id)),
        event: SSEventType::WebsocketEvent,
        data: format!(
            r#"{{"overlay_id": "{}", "team_a": "{}", "team_b": "{}"}}"#,
            overlay_id, scoreboard.team_a, scoreboard.team_b
        ),
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

#[axum::debug_handler]
pub async fn ingame_championship_phase(
    State(state): State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let scoreboard = state.db.get_scoreboard(overlay_id).await?;

    Ok(Html(get_championhip_phase(scoreboard, overlay_id).await?.render()?))
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

#[derive(Template)]
#[template(path = "stream_overlays/championship_phase.html", blocks = ["phase"])]
pub struct ChampionshipPhaseTemplate {
    pub overlay_id: Uuid,
    pub championship_phase: Option<String>,
}
