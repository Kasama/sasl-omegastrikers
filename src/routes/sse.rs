use std::sync::Arc;
use std::{convert::Infallible, fmt::Display};

use axum::extract::Query;
use axum::{
    extract::State,
    response::{
        sse::{self, KeepAlive, Sse},
        IntoResponse,
    },
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;

use crate::startgg::auth::AuthSession;

use super::error::AppError;
use super::AppState;

#[derive(Debug, Clone, Serialize)]
pub enum SSEDestination {
    Everyone,
    Channel(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SSEventType {
    Test,
    IngameOverlayUpdate,
    CasterOverlayUpdate,
    TodaysMatchesUpdate,
    NextMatchInfoUpdate,
    ChampionshipPhaseUpdate,
    WaitInfoUpdate,
    WaitInfoStandaloneUpdate,
    WebsocketEvent,
}

impl Display for SSEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self)
                .unwrap_or_default()
                .trim_matches('"')
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SSEvent {
    pub destination: SSEDestination,
    pub event: SSEventType,
    pub data: String,
}

impl From<SSEvent> for sse::Event {
    fn from(val: SSEvent) -> Self {
        sse::Event::default()
            .event(val.event.to_string())
            .data(val.data)
    }
}

#[derive(Debug, Deserialize)]
pub struct EventFilter {
    event: Option<SSEventType>,
    channel: Option<String>,
}

#[axum::debug_handler]
pub async fn handle_sse(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<EventFilter>,
    _auth_session: Option<AuthSession>,
) -> Result<impl IntoResponse, AppError> {
    let rx = state.events_sender.subscribe();

    let stream = BroadcastStream::new(rx)
        .filter_map(|res| async { res.ok() })
        .map(move |event: SSEvent| match &event.destination {
            SSEDestination::Everyone => Ok(event.into()),
            SSEDestination::Channel(c) => match (&filter.channel, &filter.event) {
                (None, Some(evt)) if *evt == event.event => Ok(event.into()),
                (Some(channel), None) if *channel == *c => Ok(event.into()),
                (Some(channel), Some(evt)) if *evt == event.event && *channel == *c => {
                    Ok(event.into())
                }
                _ => Err(()),
            },
        })
        .filter_map(|res| async { res.ok() })
        .map(|e| Ok(e) as Result<sse::Event, Infallible>);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[axum::debug_handler]
pub async fn send_event(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<EventFilter>,
) -> Result<impl IntoResponse, AppError> {
    let destination = if let Some(f) = filter.channel {
        SSEDestination::Channel(f)
    } else {
        SSEDestination::Everyone
    };
    state.events_sender.send(SSEvent {
        destination,
        event: SSEventType::Test,
        data: "data".to_string(),
    })?;

    Ok("Event sent")
}
