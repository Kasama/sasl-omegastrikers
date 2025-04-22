use axum::http::header;
use axum::routing::{delete, post, put};
use axum::{body::Body, http::Request, routing::get, Extension, Router};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::sensitive_headers::SetSensitiveHeadersLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::database::DB;
use crate::startgg;
use crate::startgg::oauth::OAuthConfig;

mod app;
pub mod auth;
mod index;
mod sse;
mod stream_overlay;
mod tournament;
pub mod views;

#[derive(Debug)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub oauth_config: OAuthConfig,
    pub db: Arc<DB>,
    pub events_receiver: broadcast::Receiver<sse::SSEvent>,
    pub events_sender: broadcast::Sender<sse::SSEvent>,
}

#[derive(Debug)]
pub struct AppStateBuilder {
    oauth_config: OAuthConfig,
    db: Arc<DB>,
    http_client: Option<reqwest::Client>,
}

impl AppState {
    pub fn builder(oauth_config: OAuthConfig, db: Arc<DB>) -> AppStateBuilder {
        AppStateBuilder {
            oauth_config,
            http_client: None,
            db,
        }
    }
}

impl AppStateBuilder {
    pub fn http_client(self, client: reqwest::Client) -> AppStateBuilder {
        AppStateBuilder {
            http_client: Some(client),
            ..self
        }
    }

    pub fn build(self) -> AppState {
        let (sender, receiver) = broadcast::channel(32);

        AppState {
            http_client: self.http_client.unwrap_or_default(),
            oauth_config: self.oauth_config,
            db: self.db,
            events_sender: sender,
            events_receiver: receiver,
        }
    }
}

pub fn init_router(state: AppState) -> Router {
    let s = Arc::new(state);
    Router::new()
        .route("/", get(index::index_handler))
        .nest_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets", ServeDir::new("assets"))
        .route("/login", get(auth::login_handler))
        .route("/oauth/startgg_callback", get(auth::oauth_callback_handler))
        .route("/logout", get(auth::logout_handler))
        .route("/sse", get(sse::handle_sse))
        .route("/send-sse", get(sse::send_event))
        .nest("/app",
            Router::new()
                .route("/", get(tournament::tournaments_handler))
                .route("/tournament", get(tournament::tournaments_handler))
                .nest("/tournament/{tournament_slug}", Router::new()
                    .route("/", get(tournament::tournament_setup))
                    .route("/overlay", put(tournament::create_overlay))
                    .nest("/overlay/{overlay_id}", Router::new()
                        .route("/", delete(tournament::delete_overlay).patch(tournament::update_overlay))
                        .route("/teams", get(tournament::team_setup_handler))
                        .route("/ingame", put(stream_overlay::update_ingame_scoreboard))
                        .route("/teams/nickname", post(tournament::update_team_nickname))
                        .route("/casters", get(tournament::casters_handler).put(stream_overlay::casters::update_casters))
                        .route("/waiting", get(stream_overlay::waiting::waiting_setup))
                        .route("/waiting/matches", post(stream_overlay::waiting::todays_matches_update))
                        .route("/waiting/timer", post(stream_overlay::waiting::timer_update))
                    )
                    .layer(axum::middleware::from_fn_with_state(s.clone(), tournament::tournament_access_middleware))
                )
        )
        .nest("/stream_overlay/{overlay_id}", Router::new()
            .route("/ingame", get(stream_overlay::ingame_overlay))
            .route("/waiting", get(stream_overlay::waiting::waiting_overlay))
            .route("/waiting/timer", get(stream_overlay::waiting::timer_overlay))
            .route("/waiting/todays_matches", get(stream_overlay::waiting::todays_matches_overlay))
            .route("/casters", get(stream_overlay::casters::casters_overlay))
        )
        .layer(axum::middleware::from_fn_with_state(s.clone(), startgg::auth::auth_middleware))
        .layer(
            ServiceBuilder::new()
                .layer(SetSensitiveHeadersLayer::new([ header::AUTHORIZATION, header::COOKIE ]))
                .layer(NormalizePathLayer::trim_trailing_slash())
                .layer(TraceLayer::new_for_http().make_span_with(|req: &Request<Body>| {
                    let req_id = Uuid::new_v4();
                    let method = req.method().as_str();
                    let uri = req.uri();
                    let span =
                        tracing::info_span!("handling request", %req_id, method = %method, uri = %uri, version = ?req.version(), headers = ?req.headers());
                    span
                }))
                .layer(Extension(s.clone())),
        )
        .with_state(s)
}
