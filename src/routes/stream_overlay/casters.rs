use crate::database::casters::Caster;
use crate::routes::error::AppError;
use crate::routes::sse::{SSEDestination, SSEvent, SSEventType};
use crate::startgg::StartGGClient;
use askama::Template;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::Form;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::routes::AppState;
use crate::startgg::auth::AuthSession;

#[derive(Debug, Deserialize)]
pub struct UpdateCastersForm {
    narrator: String,
    narrator_video: String,
    commenter: String,
    commenter_video: String,
}
#[axum::debug_handler]
pub async fn update_casters(
    state: State<Arc<AppState>>,
    auth_session: AuthSession,
    Path((_tournament_slug, overlay_id)): Path<(String, Uuid)>,
    Form(form): Form<UpdateCastersForm>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Updating casters for overlay {}: {:?}", overlay_id, form);

    let startgg_client = StartGGClient::new(&state.http_client, &auth_session.access_token);
    let tournaments = startgg_client.fetch_tournaments_organized_by_user().await?;

    let overlay = state.db.get_overlay(overlay_id).await?;

    if !tournaments
        .iter()
        .any(|t| t.slug == overlay.tournament_slug)
    {
        return Err("You are not allowed to update this overlay".into());
    };

    let caster_narrator = Caster {
        overlay_id,
        name: form.narrator,
        kind: crate::database::casters::CasterKind::Narrator,
        stream_video: form.narrator_video,
    };

    state.db.upsert_caster(&caster_narrator).await?;

    let caster_commenter = Caster {
        overlay_id,
        name: form.commenter,
        kind: crate::database::casters::CasterKind::Commenter,
        stream_video: form.commenter_video,
    };

    state.db.upsert_caster(&caster_commenter).await?;

    state.events_sender.send(SSEvent {
        destination: SSEDestination::Channel(format!("overlay_{}", overlay.id)),
        event: SSEventType::CasterOverlayUpdate,
        data: CastersContentTemplate {
            casters: Some((caster_narrator, caster_commenter)),
        }
        .render()?,
    })?;

    Ok("Casters atualizados!")
}

#[derive(Template)]
#[template(path = "stream_overlays/casters.html", block = "casters_content")]
pub struct CastersContentTemplate {
    pub casters: Option<(Caster, Caster)>,
}

#[derive(Template)]
#[template(path = "stream_overlays/casters.html")]
pub struct CastersOverlayTemplate {
    pub overlay_id: Uuid,
    pub casters: Option<(Caster, Caster)>,
}

#[axum::debug_handler]
pub async fn casters_overlay(
    s: State<Arc<AppState>>,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if let Ok(casters) = s.db.get_casters(&overlay_id).await {
        if casters.len() == 2 {
            return Ok(Html(
                CastersOverlayTemplate {
                    overlay_id,
                    casters: Some((casters[0].clone(), casters[1].clone())),
                }
                .render()?,
            ));
        }
    }
    Ok(Html(
        CastersOverlayTemplate {
            overlay_id,
            casters: None,
        }
        .render()?,
    ))
}
