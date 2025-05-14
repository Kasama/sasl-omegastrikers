use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse};
use axum_htmx::HxRequest;
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::routes::error::AppError;
use crate::routes::sse::SSEventType;
use crate::routes::AppState;

use super::waiting::{TodaysMatchesTemplate, WaitInfoTemplate};

#[derive(Debug, Clone, Deserialize)]
pub struct PartialQuery {
    name: SSEventType,
}

#[axum::debug_handler]
pub async fn partial(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PartialQuery>,
    HxRequest(hx_request): HxRequest,
    Path(overlay_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if hx_request {
        match query.name {
            SSEventType::IngameOverlayUpdate => todo!(),
            SSEventType::CasterOverlayUpdate => todo!(),
            SSEventType::TodaysMatchesUpdate => {
                return Ok(Html(
                    TodaysMatchesTemplate {
                        todays_matches: state.db.get_overlay_matches(overlay_id).await?,
                    }
                    .render()?,
                ));
            }
            SSEventType::WaitInfoUpdate => {
                let wait_timer = super::waiting::get_wait_timer(state, &overlay_id).await;
                Ok(Html(
                    WaitInfoTemplate {
                        overlay_id,
                        wait_timer,
                    }
                    .render()?,
                ))
            }
            SSEventType::WebsocketEvent => todo!(),
            _ => Err(
                AppError::from(format!("Invalid SSEventType: {}", query.name))
                    .with_status(StatusCode::NOT_FOUND),
            ),
        }
    } else {
        Ok(Html(
            Partial {
                overlay_id,
                partial_name: query.name.to_string(),
            }
            .render()?,
        ))
    }
}

#[derive(Template)]
#[template(path = "stream_overlays/partial.html")]
pub struct Partial {
    pub overlay_id: Uuid,
    pub partial_name: String,
}
