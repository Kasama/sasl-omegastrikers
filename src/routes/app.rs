use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Redirect};

use crate::startgg::auth::AuthSession;

use super::error::AppError;
use super::AppState;
