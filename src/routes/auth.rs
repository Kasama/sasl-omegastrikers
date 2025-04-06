use std::{error::Error, sync::Arc};

use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::Engine;
use chrono::{Duration, Utc};
use rand::{rngs::OsRng, TryRngCore};
use reqwest::StatusCode;
use url::Url;

use crate::{
    startgg::{
        auth::AuthSession,
        oauth::{self, OAuthCallbackParams, OAuthConfig, TokenResponse, REQUIRED_SCOPES},
        StartGGClient,
    },
    views::index::IndexTemplate,
};

pub struct AppError(pub String);

impl<E: Error> From<E> for AppError {
    fn from(value: E) -> Self {
        AppError(value.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}

use super::AppState;

#[axum::debug_handler]
pub async fn index_handler(
    State(state): State<Arc<AppState>>,
    auth_session: Option<AuthSession>, // Use optional extractor
) -> Result<impl IntoResponse, AppError> {
    // If AuthSession exists, try to get user data for display
    let user = if let Some(session) = auth_session {
        StartGGClient::new(&state.http_client, &session.access_token)
            .fetch_startgg_user()
            .await
            .ok()
    } else {
        None
    };

    Ok(Html(IndexTemplate { maybe_user: user }.render()?))
}

#[axum::debug_handler]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    let config = &state.oauth_config;
    // 1. Generate secure random state
    let mut state_bytes = [0u8; 32];
    OsRng.try_fill_bytes(&mut state_bytes)?;
    let state_str = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(state_bytes);

    // 2. Store state in HttpOnly cookie
    let state_cookie = Cookie::build((oauth::OAUTH_STATE_COOKIE, state_str.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax) // Lax is usually sufficient for OAuth redirects
        // .secure(true) // Enable this when using HTTPS
        .max_age(time::Duration::minutes(5)) // Short expiry
        .build();

    let destination_cookie = Cookie::build((oauth::OAUTH_REDIRECT_GOTO, "/app"))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax) // Lax is usually sufficient for OAuth redirects
        // .secure(true) // Enable this when using HTTPS
        .max_age(time::Duration::minutes(5)) // Short expiry
        .build();

    let jar = jar.add(state_cookie).add(destination_cookie);

    // 3. Construct authorization URL
    let mut auth_url = Url::parse(oauth::STARTGG_AUTHORIZE_URL)?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.startgg_client_id)
        .append_pair("scope", oauth::REQUIRED_SCOPES)
        .append_pair("redirect_uri", &config.startgg_redirect_uri)
        .append_pair("state", &state_str);

    // 4. Redirect user
    tracing::debug!("Redirecting to start.gg auth: {}", auth_url);
    Ok((jar, Redirect::temporary(auth_url.as_str())))
}

#[axum::debug_handler]
pub async fn oauth_callback_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<(CookieJar, Redirect), AppError> {
    let config = &state.oauth_config;
    tracing::debug!("Received OAuth callback with code: {}", params.code);

    // 1. Verify state parameter
    let saved_state = jar
        .get(oauth::OAUTH_STATE_COOKIE)
        .map(|c| c.value().to_string());

    let saved_destination = jar
        .get(oauth::OAUTH_REDIRECT_GOTO)
        .map(|c| c.value().to_string())
        .unwrap_or("/".to_string());

    match saved_state {
        Some(state_cookie_val) if state_cookie_val == params.state => {
            // State matches, proceed
            tracing::debug!("OAuth state validated successfully.");
        }
        _ => {
            tracing::warn!("OAuth state mismatch or cookie missing.");
            // You might want a specific error page here
            return Err(AppError(
                "OAuth State Mismatch, possibly old CSRF token, try refreshing".to_string(),
            ));
        }
    }

    // 2. Exchange code for tokens
    let token_response = exchange_code_for_token(&state.http_client, config, &params.code).await?;

    tracing::info!(
        "Successfully exchanged code for tokens. Access token expires in {}s",
        token_response.expires_in
    );

    // 3. Store tokens in secure cookies
    let now = Utc::now();
    let expires_at = now + Duration::seconds(token_response.expires_in);
    // Make cookie expiry slightly shorter than actual token expiry
    let cookie_max_age = Duration::seconds(token_response.expires_in - 60);

    // Ensure max_age is positive
    let cookie_max_age_std = if cookie_max_age > Duration::zero() {
        cookie_max_age
            .to_std()
            .map_err(|e| AppError(e.to_string()))?
    } else {
        // Expire immediately or don't set max_age if already expired?
        // Setting Max-Age=0 or a past date typically deletes the cookie.
        // Let's just expire it quickly if it's already almost gone.
        std::time::Duration::from_secs(1)
    };

    let access_token_cookie =
        Cookie::build((oauth::ACCESS_TOKEN_COOKIE, token_response.access_token))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            // .secure(true) // Enable for HTTPS
            .max_age(time::Duration::seconds(cookie_max_age_std.as_secs() as i64))
            .build();

    let refresh_token_cookie =
        Cookie::build((oauth::REFRESH_TOKEN_COOKIE, token_response.refresh_token))
            .path("/") // Be careful with refresh token scope if needed
            .http_only(true)
            .same_site(SameSite::Lax) // Or Strict?
            // .secure(true) // Enable for HTTPS
            .max_age(time::Duration::days(30))
            .build();

    let expires_at_cookie = Cookie::build((oauth::EXPIRES_AT_COOKIE, expires_at.to_rfc3339()))
        .path("/")
        .http_only(true) // Technically readable by JS is ok, but safer as HttpOnly
        .same_site(SameSite::Lax)
        // .secure(true) // Enable for HTTPS
        .max_age(time::Duration::seconds(cookie_max_age_std.as_secs() as i64))
        .build();

    let state_cookie = Cookie::build((oauth::OAUTH_STATE_COOKIE, ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::nanoseconds(1))
        .build();
    let destination_cookie = Cookie::build((oauth::OAUTH_REDIRECT_GOTO, ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::nanoseconds(1))
        .build();

    let jar = jar
        .add(access_token_cookie)
        .add(refresh_token_cookie)
        .add(state_cookie)
        .add(destination_cookie)
        .add(expires_at_cookie);

    // 4. Redirect to the main page (or a success page)
    Ok((jar, Redirect::temporary(&saved_destination)))
}

#[axum::debug_handler]
pub async fn logout_handler(mut jar: CookieJar) -> Result<(CookieJar, Redirect), AppError> {
    // Remove all auth-related cookies by expiring them immediately
    jar = jar.remove(Cookie::from(oauth::ACCESS_TOKEN_COOKIE));
    jar = jar.remove(Cookie::from(oauth::REFRESH_TOKEN_COOKIE));
    jar = jar.remove(Cookie::from(oauth::EXPIRES_AT_COOKIE));

    Ok((jar, Redirect::temporary("/")))
}

// --- Helper Functions ---

async fn exchange_code_for_token(
    client: &reqwest::Client,
    config: &OAuthConfig,
    code: &str,
) -> Result<TokenResponse, AppError> {
    let params = oauth::AuthCodeTokenRequest {
        client_id: &config.startgg_client_id,
        client_secret: &config.startgg_client_secret,
        grant_type: "authorization_code",
        code,
        redirect_uri: &config.startgg_redirect_uri,
        scope: REQUIRED_SCOPES,
    };

    let response = client
        .post(oauth::STARTGG_TOKEN_URL)
        .json(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body".to_string());
        tracing::error!("start.gg token exchange failed: {} - {}", status, body);
        return Err(AppError(format!(
            "Failed to exchange code for token. Status: {}. Body: {}",
            status, body
        )));
    }

    let token_response: TokenResponse = response.json().await?;

    Ok(token_response)
}
