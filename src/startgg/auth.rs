use std::sync::Arc;

use crate::{
    routes::{views::ViewError, AppState},
    startgg::{
        oauth::{self, TokenResponse},
        StartGGClient,
    },
};
use anyhow::Context;
use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts, State},
    http::{header, request::Parts, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Duration;
use chrono::{DateTime, Utc};
use tracing;

use super::oauth::OAuthConfig;

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

// --- Middleware ---
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ViewError> {
    tracing::debug!("Running auth middleware");

    // Use the AuthSession extractor logic directly or re-implement parts here
    let cookies = CookieJar::from_headers(req.headers());

    let access_token: Option<String> = cookies
        .get(oauth::ACCESS_TOKEN_COOKIE)
        .map(|c| c.value().to_string());
    let refresh_token: Option<String> = cookies
        .get(oauth::REFRESH_TOKEN_COOKIE)
        .map(|c| c.value().to_string());
    let expires_at_str: Option<String> = cookies
        .get(oauth::EXPIRES_AT_COOKIE)
        .map(|c| c.value().to_string());

    let mut auth_data = match (access_token, expires_at_str) {
        (Some(token), Some(exp_str)) => {
            match DateTime::parse_from_rfc3339(&exp_str) {
                Ok(exp_dt) => Some((token, exp_dt.with_timezone(&Utc))),
                Err(_) => {
                    tracing::warn!("Failed to parse expiry cookie: {}", exp_str);
                    None // Treat unparseable expiry as missing/invalid
                }
            }
        }
        _ => None,
    };

    let mut new_cookies: Option<CookieJar> = None; // To hold potentially refreshed cookies

    // Check if token is present and expired
    if let Some((ref _token, expiry)) = auth_data {
        let now = Utc::now();
        // Add a small buffer (e.g., 60 seconds) to refresh slightly before expiry
        if now >= expiry - Duration::seconds(60) {
            tracing::info!("Access token expired or nearing expiry, attempting refresh.");
            // Token expired, attempt refresh if refresh token exists
            if let Some(rt) = refresh_token {
                match refresh_access_token(&state.http_client, &state.oauth_config, &rt).await {
                    Ok((refreshed_token_resp, new_jar)) => {
                        tracing::info!("Successfully refreshed access token.");
                        // Update auth_data with new token details
                        let new_expiry =
                            Utc::now() + Duration::seconds(refreshed_token_resp.expires_in);
                        auth_data = Some((refreshed_token_resp.access_token.clone(), new_expiry));
                        // Store new cookies to be added to the response
                        new_cookies = Some(new_jar);
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh token: {}", e);
                        // Refresh failed, clear auth data, user needs to re-login
                        auth_data = None;
                        // Also clear cookies in the response later
                        // Create a jar that removes the cookies
                        let mut removal_jar = CookieJar::new();
                        removal_jar = removal_jar.remove(Cookie::from(oauth::ACCESS_TOKEN_COOKIE));
                        removal_jar = removal_jar.remove(Cookie::from(oauth::REFRESH_TOKEN_COOKIE));
                        removal_jar = removal_jar.remove(Cookie::from(oauth::EXPIRES_AT_COOKIE));
                        new_cookies = Some(removal_jar); // Signal to clear cookies
                    }
                }
            } else {
                // Token expired, no refresh token available
                tracing::warn!("Access token expired, but no refresh token found.");
                auth_data = None;
                // Clear potentially lingering expired cookies
                let mut removal_jar = CookieJar::new();
                removal_jar = removal_jar.remove(Cookie::from(oauth::ACCESS_TOKEN_COOKIE));
                removal_jar = removal_jar.remove(Cookie::from(oauth::EXPIRES_AT_COOKIE)); // Keep refresh token? debatable
                new_cookies = Some(removal_jar);
            }
        } else {
            // Token is valid, proceed
            tracing::debug!("Access token is valid.");
        }
    }

    // If after checks/refresh, we have valid auth data, fetch user and insert session
    if let Some((token, expiry)) = auth_data {
        match StartGGClient::new(&state.http_client, &token)
            .fetch_startgg_user()
            .await
        {
            Ok(_user) => {
                let session = AuthSession {
                    access_token: token.clone(), // Clone token for the session
                    expires_at: expiry,
                };
                // Add session to request extensions for handlers to use
                req.extensions_mut().insert(session);

                // Proceed to the next handler
                let mut res = next.run(req).await;

                // If tokens were refreshed, add Set-Cookie headers and the custom header
                if let Some(jar) = new_cookies {
                    let headers = res.headers_mut();
                    for cookie in jar.iter() {
                        // Use delta to get only changed cookies
                        headers.append(header::SET_COOKIE, cookie.to_string().parse().unwrap());
                    }
                    // Add the custom header with the NEW access token
                    headers.insert(oauth::HEADER_SET_ACCESS_TOKEN, token.parse().unwrap());
                }

                Ok(res)
            }
            Err(e) => {
                tracing::error!("Failed to fetch user data with valid token: {}", e);
                // Treat as unauthorized if we can't verify the user with the token
                // Respond early, potentially clearing cookies
                let mut res = next.run(req).await;
                if let Some(jar) = new_cookies.or_else(|| {
                    // Clear cookies if refresh didn't happen or failed
                    let mut removal_jar = CookieJar::new();
                    removal_jar = removal_jar.remove(Cookie::from(oauth::ACCESS_TOKEN_COOKIE));
                    removal_jar = removal_jar.remove(Cookie::from(oauth::REFRESH_TOKEN_COOKIE));
                    removal_jar = removal_jar.remove(Cookie::from(oauth::EXPIRES_AT_COOKIE));
                    Some(removal_jar)
                }) {
                    let headers = res.headers_mut();
                    for cookie in jar.iter() {
                        headers.append(header::SET_COOKIE, cookie.to_string().parse().unwrap());
                    }
                }
                Ok(res)
            }
        }
    } else {
        // No valid authentication data found after checks
        tracing::debug!("No valid auth session found.");
        let mut res = next.run(req).await;
        if let Some(jar) = new_cookies {
            // Only add cookies if refresh attempt resulted in changes (like clearing)
            let headers = res.headers_mut();
            for cookie in jar.iter() {
                headers.append(header::SET_COOKIE, cookie.to_string().parse().unwrap());
            }
        }
        Ok(res)
    }
}

// --- Extractors ---

impl<S> FromRequestParts<S> for AuthSession
where
    S: Send + Sync,
{
    type Rejection = ViewError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // This extractor *relies* on the auth_middleware having run successfully first
        // and inserted the AuthSession into request extensions.
        if let Some(session) = parts.extensions.get::<AuthSession>().cloned() {
            let now = Utc::now();
            if now >= session.expires_at {
                return Err(ViewError {
                    status_code: Some(StatusCode::UNAUTHORIZED),
                    content: "Authentication Expired, please login again".to_string(),
                });
            }
            Ok(session)
        } else {
            tracing::warn!("AuthSession extractor could not find AuthSession in request extensions. Ensure auth_middleware is applied.");
            Err(ViewError {
                status_code: Some(StatusCode::UNAUTHORIZED),
                content: "Required authentication headers, please login again".to_string(),
            })
        }
    }
}

impl<S> OptionalFromRequestParts<S> for AuthSession
where
    S: Send + Sync,
{
    type Rejection = ViewError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        tracing::warn!("Checking if request parts contain auth session");
        if let Some(session) = parts.extensions.get::<AuthSession>() {
            tracing::warn!("   It did contain session");
            Ok(Some(session.clone()))
        } else {
            tracing::warn!("   It did not contain session");
            Ok(None)
        }
    }
}

// --- Token Refresh Logic ---
async fn refresh_access_token(
    client: &reqwest::Client,
    config: &OAuthConfig,
    refresh_token: &str,
) -> anyhow::Result<(TokenResponse, CookieJar)> {
    // Return new cookies too
    let response = client
        .post(oauth::STARTGG_TOKEN_URL)
        .json(&serde_json::json!( {
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "scope": [],
            "client_id": &config.startgg_client_id,
            "client_secret": &config.startgg_client_secret,
            "redirect_uri": &config.startgg_redirect_uri,
        }))
        .send()
        .await
        .context("Failed to send refresh token request to start.gg")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body".to_string());
        tracing::error!("start.gg token refresh failed: {} - {}", status, body);
        return Err(anyhow::anyhow!(
            "Failed to refresh token. Status: {}. Body: {}",
            status,
            body
        ));
    }

    let token_response: TokenResponse = response
        .json()
        .await
        .context("Failed to parse refresh token response JSON from start.gg")?;

    // Create new cookies based on the response
    let mut new_jar = CookieJar::new();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::seconds(token_response.expires_in);
    let cookie_max_age = Duration::seconds(token_response.expires_in - 60);

    let access_token_cookie = Cookie::build((
        oauth::ACCESS_TOKEN_COOKIE,
        token_response.access_token.clone(),
    )) // Clone token here
    .path("/")
    .http_only(true)
    .same_site(SameSite::Lax)
    .secure(true)
    .max_age(time::Duration::seconds(cookie_max_age.num_seconds()))
    .build();

    // Note: Refresh tokens might be rotated by the provider. The response might contain a *new* refresh token.
    // Check the `token_response` fields. start.gg docs don't explicitly mention refresh token rotation,
    // but it's good practice to handle it. Let's assume the returned `refresh_token` is the one to use now.
    let refresh_token_cookie = Cookie::build((
        oauth::REFRESH_TOKEN_COOKIE,
        token_response.refresh_token.clone(),
    ))
    .path("/")
    .http_only(true)
    .same_site(SameSite::Lax)
    // .secure(true)
    .max_age(time::Duration::days(30)) // Keep original long duration
    .build();

    let expires_at_cookie = Cookie::build((oauth::EXPIRES_AT_COOKIE, expires_at.to_rfc3339()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        // .secure(true)
        .max_age(time::Duration::seconds(cookie_max_age.num_seconds()))
        .build();

    new_jar = new_jar
        .add(access_token_cookie)
        .add(refresh_token_cookie)
        .add(expires_at_cookie);

    Ok((token_response, new_jar))
}
