// src/oauth.rs
use serde::Deserialize;

pub const STARTGG_AUTHORIZE_URL: &str = "https://start.gg/oauth/authorize";
pub const STARTGG_TOKEN_URL: &str = "https://api.start.gg/oauth/access_token";
pub const REQUIRED_SCOPES: &str = "user.identity user.email tournament.manager tournament.reporter";

pub const ACCESS_TOKEN_COOKIE: &str = "sg_access_token";
pub const REFRESH_TOKEN_COOKIE: &str = "sg_refresh_token";
pub const EXPIRES_AT_COOKIE: &str = "sg_expires_at";
pub const OAUTH_STATE_COOKIE: &str = "sg_oauth_state";
pub const OAUTH_REDIRECT_GOTO: &str = "sg_oauth_redirect_goto";

// Header to notify client of new token after refresh
pub const HEADER_SET_ACCESS_TOKEN: &str = "X-Set-Access-Token";

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64, // Duration in seconds
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartggUser {
    pub slug: String,
    pub gamer_tag: Option<String>,
}

// Define a structure for the refresh token request body
#[derive(serde::Serialize)]
pub struct RefreshTokenRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub grant_type: &'static str,
    pub refresh_token: &'a str,
}

// Define a structure for the authorization code request body
#[derive(serde::Serialize)]
pub struct AuthCodeTokenRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub grant_type: &'static str,
    pub code: &'a str,
    pub redirect_uri: &'a str,
    pub scope: &'static str,
}

#[derive(Debug)]
pub struct OAuthConfig {
    pub startgg_client_id: String,
    pub startgg_client_secret: String,
    pub startgg_redirect_uri: String,
}
