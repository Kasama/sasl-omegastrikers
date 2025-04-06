use axum::http::header;
use axum::{body::Body, http::Request, routing::get, Extension, Router};
use axum_extra::routing::RouterExt;
use reqwest::StatusCode;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::sensitive_headers::SetSensitiveHeadersLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::startgg;
use crate::startgg::oauth::OAuthConfig;

pub mod auth;
mod index;
mod tournament;
mod app;

#[derive(Debug)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub oauth_config: OAuthConfig,
}

pub fn init_router(state: AppState) -> Router {
    let s = Arc::new(state);
    Router::new()
        // .route("/", get(views::index::index))
        .route("/", get(auth::index_handler))
        .route("/login", get(auth::login_handler))
        .route("/oauth/startgg_callback", get(auth::oauth_callback_handler))
        .route("/logout", get(auth::logout_handler))
        .nest("/app",
            Router::new()
                .route("/", get(app::index_handler))
                .route_with_tsr("/tournaments", get(tournament::tournaments_handler))
                .route_with_tsr("/tournament/{tournament_slug}/manage", get(tournament::manage_handler))
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
