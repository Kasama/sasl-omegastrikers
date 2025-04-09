use super::auth::AppError;
use super::AppState;
use crate::startgg::auth::AuthSession;
use crate::startgg::tournaments::StartGGTeam;
use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use std::sync::Arc;

#[axum::debug_handler]
pub async fn ingame_overlay(
    _s: State<Arc<AppState>>,
    _auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    Ok(Html(
        IngameOverlayTemplate {
            team_a: StartGGTeam {
                name: "Pix".to_string(),
                image: None,
                id: "test".to_string(),
                team_members: vec![],
            },
            team_b: StartGGTeam {
                name: "PBWM".to_string(),
                image: None,
                id: "test".to_string(),
                team_members: vec![],
            },
        }
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "stream_overlays/ingame.html")]
pub struct IngameOverlayTemplate {
    pub team_a: StartGGTeam,
    pub team_b: StartGGTeam,
}
