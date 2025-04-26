use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Redirect};

use crate::startgg::auth::AuthSession;

use super::error::AppError;
use super::AppState;

#[axum::debug_handler]
pub async fn index_handler(
    _s: State<Arc<AppState>>,
    _auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    Ok(Redirect::temporary("/app/tournament"))
}
