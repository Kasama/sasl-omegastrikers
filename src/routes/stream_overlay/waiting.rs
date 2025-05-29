use std::sync::Arc;

use crate::database::matches::Match;
use crate::database::wait_timer::{WaitTimer, WaitType};
use crate::routes::error::AppError;
use crate::routes::sse::{SSEDestination, SSEvent, SSEventType};
use crate::routes::tournament::get_tournament_teams;
use crate::routes::views::filters;
use crate::routes::AppState;
use crate::startgg::auth::AuthSession;
use crate::startgg::tournaments::StartGGTeam;
use askama::Template;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum_extra::extract::Form;
use axum_htmx::HxRequest;
use futures_util::future::join_all;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "stream_overlays/waiting/page.html")]
pub struct WaitingOverlayTemplate {
    pub overlay_id: Uuid,
    pub wait_timer: Option<WaitTimer>,
}

#[derive(Template)]
#[template(path = "stream_overlays/waiting/page.html", block = "wait_info")]
pub struct WaitInfoTemplate {
    pub overlay_id: Uuid,
    pub wait_timer: Option<WaitTimer>,
}

#[axum::debug_handler]
pub async fn waiting_overlay(
    State(state): State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Html(
        WaitingOverlayTemplate {
            overlay_id,
            wait_timer: get_wait_timer(state, &overlay_id).await,
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "stream_overlays/waiting/timer.html")]
pub struct TimerTemplate {
    pub duration: chrono::Duration,
}

#[axum::debug_handler]
pub async fn timer_overlay(
    State(state): State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let wait_timer = get_wait_timer(state, &overlay_id).await;

    let d = if let Some(t) = wait_timer {
        let now = chrono::Utc::now();
        t.wait_until.signed_duration_since(now)
    } else {
        chrono::Duration::zero()
    };

    Ok(Html(TimerTemplate { duration: d }.render()?))
}

#[derive(Template)]
#[template(path = "stream_overlays/waiting/standalone_timer.html", blocks = ["wait_info"])]
pub struct StandaloneTimerTemplate {
    pub overlay_id: Uuid,
    pub wait_timer: Option<WaitTimer>,
}

#[axum::debug_handler]
pub async fn standalone_timer_overlay(
    State(state): State<Arc<AppState>>,
    HxRequest(hx_request): HxRequest,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let wait_timer = get_wait_timer(state, &overlay_id).await;

    Ok(Html(if hx_request {
        StandaloneTimerTemplate {
            overlay_id,
            wait_timer,
        }
        .as_wait_info()
        .render()?
    } else {
        StandaloneTimerTemplate {
            overlay_id,
            wait_timer,
        }
        .render()?
    }))
}

#[derive(Template)]
#[template(path = "waiting/setup.html", block = "wait_section")]
pub struct WaitTimerSetupTemplate {
    pub overlay_id: Uuid,
    pub tournament_slug: String,
    pub wait_timer: Option<WaitTimer>,
}

#[derive(Debug, Deserialize)]
pub struct TimerUpdateForm {
    wait_type: WaitType,
    waiting_until: String,
    timezone_offset: i32,
}
#[axum::debug_handler]
pub async fn timer_update(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    Form(form): Form<TimerUpdateForm>,
) -> Result<impl IntoResponse, AppError> {
    let naive_time = chrono::NaiveDateTime::parse_from_str(&form.waiting_until, "%Y-%m-%dT%H:%M")?;
    let tz_offset = chrono::FixedOffset::west_opt(form.timezone_offset * 60)
        .unwrap_or_else(|| chrono::FixedOffset::west_opt(3 * 60 * 60).unwrap()); // Use America/Sao_Paulo as default
    let time = match naive_time.and_local_timezone(tz_offset) {
        chrono::offset::LocalResult::Single(s) => s,
        chrono::offset::LocalResult::Ambiguous(e, _) => e,
        chrono::offset::LocalResult::None => return Err("Invalid time".into()),
    };

    let wait_timer = WaitTimer {
        overlay_id,
        wait_until: time,
        wait_type: form.wait_type,
    };

    state.db.upsert_wait_timer(&wait_timer).await?;

    let wait_timer = get_wait_timer(state.clone(), &overlay_id).await;

    let _ = state
        .events_sender
        .send(SSEvent {
            destination: SSEDestination::Channel(format!("overlay_{}", overlay_id)),
            event: SSEventType::WaitInfoUpdate,
            data: WaitInfoTemplate {
                overlay_id,
                wait_timer: wait_timer.clone(),
            }
            .render()?,
        })
        .inspect_err(|e| {
            tracing::error!("Failed to send wait timer update: {}", e);
        });

    let _ = state
        .events_sender
        .send(SSEvent {
            destination: SSEDestination::Channel(format!("overlay_{}", overlay_id)),
            event: SSEventType::WaitInfoStandaloneUpdate,
            data: StandaloneTimerTemplate {
                overlay_id,
                wait_timer: wait_timer.clone(),
            }
            .render()?,
        })
        .inspect_err(|e| {
            tracing::error!("Failed to send wait timer update: {}", e);
        });

    Ok(Html(
        WaitTimerSetupTemplate {
            overlay_id,
            tournament_slug,
            wait_timer,
        }
        .render()?,
    ))
}

pub async fn get_wait_timer(state: Arc<AppState>, overlay_id: &Uuid) -> Option<WaitTimer> {
    state
        .db
        .get_wait_timer(overlay_id)
        .await
        .inspect_err(|e| {
            tracing::warn!("Failed to get wait timer: {}", e);
        })
        .ok()
}

#[derive(Template)]
#[template(path = "stream_overlays/waiting/todays_matches.html")]
pub struct TodaysMatchesTemplate {
    pub todays_matches: Vec<Match>,
}

#[axum::debug_handler]
pub async fn todays_matches_overlay(
    state: State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let matches = state.db.get_overlay_matches(overlay_id).await?;

    Ok(Html(
        TodaysMatchesTemplate {
            todays_matches: matches,
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "stream_overlays/waiting/next_up_match.html", blocks = ["next_match_info"])]
pub struct NextUpMatchTemplate {
    pub overlay_id: Uuid,
    pub todays_matches: Vec<Match>,
}

#[axum::debug_handler]
pub async fn todays_matches_single_overlay(
    state: State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let matches = state.db.get_overlay_matches(overlay_id).await?;

    Ok(Html(
        NextUpMatchTemplate {
            overlay_id,
            todays_matches: matches,
        }
        .render()?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct TodaysMatchesUpdateForm {
    #[serde(default)]
    #[serde(rename = "existing_match_id")]
    existing_match_ids: Vec<Uuid>,
    #[serde(default)]
    #[serde(rename = "match_id")]
    match_ids: Vec<Uuid>,
    #[serde(default)]
    team_a: Vec<String>,
    #[serde(default)]
    team_a_score: Vec<i32>,
    #[serde(default)]
    team_b: Vec<String>,
    #[serde(default)]
    team_b_score: Vec<i32>,
    #[serde(default)]
    completed: Vec<Uuid>,
    #[serde(default)]
    in_progress: Vec<Uuid>,
    #[serde(default)]
    featured: Vec<Uuid>,
}

#[axum::debug_handler]
pub async fn todays_matches_update(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    auth_session: AuthSession,
    Form(todays_matches_form): Form<TodaysMatchesUpdateForm>,
) -> Result<impl IntoResponse, AppError> {
    let delete_matches = todays_matches_form
        .existing_match_ids
        .into_iter()
        .filter_map(|id| {
            if todays_matches_form.match_ids.contains(&id) {
                return None;
            }
            tracing::trace!("deleting match {}", id);
            let st = state.clone();
            Some(async move {
                let match_ = st.db.get_match(id).await?;
                let match_ = st
                    .db
                    .upsert_match(Match {
                        overlay_id: None,
                        ..match_
                    })
                    .await?;
                Ok(match_) as anyhow::Result<_>
            })
        });

    join_all(delete_matches)
        .await
        .into_iter()
        .filter_map(|m| if let Err(e) = m { Some(e) } else { None })
        .for_each(|e| {
            tracing::error!("Failed to delete match: {}", e);
        });

    let update_matches = todays_matches_form
        .team_a
        .into_iter()
        .zip(todays_matches_form.team_a_score)
        .zip(
            todays_matches_form
                .team_b
                .into_iter()
                .zip(todays_matches_form.team_b_score),
        )
        .zip(todays_matches_form.match_ids)
        .map(|(((team_a, score_a), (team_b, score_b)), match_id)| {
            let st = state.clone();
            let tournament_slug = tournament_slug.clone();
            let completed = todays_matches_form.completed.contains(&match_id);
            let in_progress = todays_matches_form.in_progress.contains(&match_id);
            let featured = todays_matches_form.featured.contains(&match_id);
            async move {
                let m = Match {
                    id: match_id,
                    overlay_id: Some(overlay_id),
                    tournament_slug,
                    team_a: st.db.get_team(&team_a).await?,
                    team_b: st.db.get_team(&team_b).await?,
                    team_a_score: score_a,
                    team_b_score: score_b,
                    completed,
                    in_progress,
                    featured,
                };

                st.db.upsert_match(m).await
            }
        });

    // Need to guarantee that the order that the matches are created is the same as in the UI.
    // Because of the creation timestamp that is used for ordering.
    // So shouldn't use join_all here, as it doesn' guarantee execution order.
    // awaiting one by one is probably slower, though.
    //
    // Previous code lookes like this:
    //
    // join_all(update_matches)
    //     .await
    //     .into_iter()
    //     .filter_map(|m| if let Err(e) = m { Some(e) } else { None })
    //     .for_each(|e| {
    //         tracing::error!("Failed to update match: {}", e);
    //     });
    for fut in update_matches {
        let _ = fut.await.inspect_err(|e| {
            tracing::error!("Failed to update match: {}", e);
        });
    }

    let teams = get_tournament_teams(state.clone(), &auth_session, &tournament_slug).await?;

    let matches = state.db.get_overlay_matches(overlay_id).await?;

    let _ = state
        .events_sender
        .send(SSEvent {
            destination: SSEDestination::Channel(format!("overlay_{}", overlay_id)),
            event: SSEventType::TodaysMatchesUpdate,
            data: TodaysMatchesTemplate {
                todays_matches: matches.clone(),
            }
            .render()?,
        })
        .inspect_err(|e| {
            tracing::error!("Failed to send todays matches update: {}", e);
        });

    let _ = state
        .events_sender
        .send(SSEvent {
            destination: SSEDestination::Channel(format!("overlay_{}", overlay_id)),
            event: SSEventType::NextMatchInfoUpdate,
            data: NextUpMatchTemplate {
                overlay_id,
                todays_matches: matches.clone(),
            }
            .as_next_match_info()
            .render()?,
        })
        .inspect_err(|e| {
            tracing::error!("Failed to send todays matches update: {}", e);
        });

    Ok(Html(
        WaitingSetupTemplate {
            tournament_slug,
            overlay_id,
            upcoming_matches: matches,
            teams,
            wait_timer: get_wait_timer(state, &overlay_id).await,
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "waiting/setup.html")]
pub struct WaitingSetupTemplate {
    tournament_slug: String,
    overlay_id: Uuid,
    upcoming_matches: Vec<Match>,
    teams: Vec<StartGGTeam>,
    wait_timer: Option<WaitTimer>,
}

#[axum::debug_handler]
pub async fn waiting_setup(
    State(state): State<Arc<AppState>>,
    Path((tournament_slug, overlay_id)): Path<(String, Uuid)>,
    auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    let teams = get_tournament_teams(state.clone(), &auth_session, &tournament_slug).await?;

    let upcoming_matches = state.db.get_overlay_matches(overlay_id).await?;

    Ok(Html(
        WaitingSetupTemplate {
            upcoming_matches,
            teams,
            tournament_slug,
            overlay_id,
            wait_timer: get_wait_timer(state, &overlay_id).await,
        }
        .render()?,
    ))
}
