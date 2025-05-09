use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, Query, Request, State};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use axum_htmx::HxRequest;
use futures_util::future::join_all;
use serde::Deserialize;
use uuid::Uuid;

use crate::database::casters::Caster;
use crate::database::overlay::Overlay;
use crate::database::scoreboard::Scoreboard;
use crate::startgg::auth::AuthSession;
use crate::startgg::StartGGClient;

use super::error::AppError;
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
    let user = startgg_client.fetch_startgg_user().await?;
    let tournaments = startgg_client.fetch_tournaments_organized_by_user().await?;

    Ok(Html(
        TournamentsTemplate {
            maybe_user: Some(user),
            tournaments,
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "tournament_setup.html")]
pub struct TournamentSetup {
    pub maybe_user: Option<StartggUser>,
    pub tournament: StartGGTournament,
    pub overlays: Vec<Overlay>,
    pub selected_overlay: Option<Overlay>,
}

#[derive(Template)]
#[template(path = "tournament_setup.html", block = "manageoverlay")]
pub struct TournamentSetupManageOverlay {
    pub selected_overlay: Option<Overlay>,
    pub tournament: StartGGTournament,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TournamentSetupQuery {
    pub overlay: Option<Uuid>,
}
#[axum::debug_handler]
pub async fn tournament_setup(
    State(state): State<Arc<AppState>>,
    Path(tournament_slug): Path<String>,
    Query(query): Query<TournamentSetupQuery>,
    HxRequest(is_hx_request): HxRequest,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let user = startgg_client.fetch_startgg_user().await?;
    let tournament = startgg_client
        .fetch_tournament(tournament_slug.to_string())
        .await?;

    let overlays = state.db.get_tournament_overlays(&tournament.slug).await?;

    let selected_overlay = match query.overlay {
        Some(id) => Some(state.db.get_overlay(id).await?),
        None => None,
    };

    Ok(Html(if is_hx_request {
        format!(
            "{}\n{}",
            TournamentSetupManageOverlay {
                tournament: tournament.clone(),
                selected_overlay: selected_overlay.clone()
            }
            .render()?,
            TournamentStreamOverlayList {
                tournament,
                overlays,
                selected_overlay
            }
            .render()?
        )
    } else {
        TournamentSetup {
            maybe_user: Some(user),
            tournament,
            overlays,
            selected_overlay,
        }
        .render()?
    }))
}

#[derive(Template)]
#[template(path = "tournament_setup.html", block = "overlaylist")]
pub struct TournamentStreamOverlayList {
    tournament: StartGGTournament,
    overlays: Vec<Overlay>,
    selected_overlay: Option<Overlay>,
}

#[axum::debug_handler]
pub async fn create_overlay(
    State(state): State<Arc<AppState>>,
    Path(tournament_slug): Path<String>,
    Query(query): Query<TournamentSetupQuery>,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Handling create_overlay");

    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let tournament = startgg_client
        .fetch_tournament(tournament_slug.to_string())
        .await?;

    let _ = state.db.create_overlay(&tournament.slug).await?;

    let overlays = state.db.get_tournament_overlays(&tournament.slug).await?;

    let selected_overlay = match query.overlay {
        Some(id) => Some(state.db.get_overlay(id).await?),
        None => None,
    };

    Ok(Html(
        TournamentStreamOverlayList {
            tournament,
            overlays,
            selected_overlay,
        }
        .render()?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpdateOverlayForm {
    pub name: String,
}

#[axum::debug_handler]
pub async fn update_overlay(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    auth_session: AuthSession,
    Form(data): Form<UpdateOverlayForm>,
) -> Result<impl IntoResponse, AppError> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let tournament = startgg_client
        .fetch_tournament(tournament_slug.to_string())
        .await?;

    state.db.update_overlay(overlay_id, &data.name).await?;

    let overlays = state.db.get_tournament_overlays(&tournament.slug).await?;

    let selected_overlay = state.db.get_overlay(overlay_id).await?;

    Ok(format!(
        "{}\n{}",
        TournamentSetupManageOverlay {
            tournament: tournament.clone(),
            selected_overlay: Some(selected_overlay.clone()),
        }
        .render()?,
        TournamentStreamOverlayList {
            tournament,
            overlays,
            selected_overlay: Some(selected_overlay)
        }
        .render()?
    ))
}

#[axum::debug_handler]
pub async fn delete_overlay(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let tournament = startgg_client
        .fetch_tournament(tournament_slug.to_string())
        .await?;

    state.db.delete_overlay(overlay_id).await?;

    let overlays = state.db.get_tournament_overlays(&tournament.slug).await?;

    Ok(Html(
        TournamentStreamOverlayList {
            tournament,
            overlays,
            selected_overlay: None,
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "teams_setup.html")]
pub struct TeamsSetup {
    pub tournament_slug: String,
    pub overlay_id: Uuid,
    pub teams: Vec<StartGGTeam>,
    pub selected_teams: Option<(StartGGTeam, StartGGTeam, Scoreboard)>,
}

#[axum::debug_handler]
pub async fn team_setup_handler(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    let selected_teams = async {
        let scoreboard = state.db.get_scoreboard(overlay_id).await.ok()?;

        let team_a = state.db.get_team(&scoreboard.team_a).await.ok()?;
        let team_b = state.db.get_team(&scoreboard.team_b).await.ok()?;

        Some((team_a, team_b, scoreboard))
    }
    .await;

    let teams = get_tournament_teams(state, &auth_session, &tournament_slug).await?;

    Ok(Html(
        TeamsSetup {
            teams,
            overlay_id,
            tournament_slug,
            selected_teams,
        }
        .render()?,
    ))
}

pub async fn get_tournament_teams(
    state: Arc<AppState>,
    auth_session: &AuthSession,
    tournament_slug: &str,
) -> anyhow::Result<Vec<StartGGTeam>> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);

    let teams = startgg_client
        .fetch_tournament_teams(tournament_slug.to_string())
        .await?;

    let _ = join_all(
        teams
            .iter()
            .map(|team| state.db.upsert_team(tournament_slug, team)),
    )
    .await;

    let teams = state.db.get_tournament_teams(tournament_slug).await?;

    Ok(teams)
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamNicknameForm {
    team: String,
    team_nickname: String,
}
#[axum::debug_handler]
pub async fn update_team_nickname(
    State(state): State<Arc<AppState>>,
    auth_session: AuthSession,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    Form(form): Form<UpdateTeamNicknameForm>,
) -> Result<impl IntoResponse, AppError> {
    let team = state.db.get_team(&form.team).await?;

    let t = StartGGTeam {
        nickname: Some(form.team_nickname),
        ..team
    };

    state.db.upsert_team(&tournament_slug, &t).await?;

    let selected_teams = async {
        let scoreboard = state.db.get_scoreboard(overlay_id).await.ok()?;

        let team_a = state.db.get_team(&scoreboard.team_a).await.ok()?;
        let team_b = state.db.get_team(&scoreboard.team_b).await.ok()?;

        Some((team_a, team_b, scoreboard))
    }
    .await;

    let teams = get_tournament_teams(state, &auth_session, &tournament_slug).await?;

    Ok(Html(
        TeamsSetup {
            tournament_slug,
            overlay_id,
            teams,
            selected_teams,
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "casters_setup.html")]
pub struct CastersSetup {
    pub tournament_slug: String,
    pub overlay: Overlay,
    pub casters: Option<(Caster, Caster)>,
}

#[axum::debug_handler]
pub async fn casters_handler(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    _auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    let overlay = state.db.get_overlay(overlay_id).await?;

    let db_casters = state.db.get_casters(&overlay_id).await?;
    let casters = if db_casters.len() == 2 {
        Some((db_casters[0].clone(), db_casters[1].clone()))
    } else {
        None
    };

    Ok(Html(
        CastersSetup {
            overlay,
            casters,
            tournament_slug,
        }
        .render()?,
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub struct TournamentSlugPathExtractor {
    tournament_slug: String,
}

pub async fn tournament_access_middleware(
    State(state): State<Arc<AppState>>,
    Path(path_extractor): Path<TournamentSlugPathExtractor>,
    auth_session: AuthSession,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);

    let user_tournaments = startgg_client
        .fetch_tournaments_organized_by_user()
        .await?
        .into_iter()
        .find(|t| t.slug == path_extractor.tournament_slug);

    user_tournaments.ok_or(
        AppError::from(format!(
            "user is not authorized to manage tournament {}",
            path_extractor.tournament_slug
        ))
        .with_unauthorized(),
    )?;

    let res = next.run(req).await;
    Ok(res)
}
