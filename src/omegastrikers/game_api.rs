use axum::http::HeaderName;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::Deserialize;

use super::*;

const OMEGASTRIKERS_INNER_API_URL: &str = "https://prometheus-proxy.odysseyinteractive.gg/api/v1";

pub struct OmegaApiClient {
    // auth: OmegaStrikersAccessTokens,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OmegaStrikersAccessTokens {
    jwt: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OmegaStrikersIdentity {
    access_tokens: OmegaStrikersAccessTokens,
    // original_auth_provider: String,
    // platform_id: String,
}

impl OmegaApiClient {
    pub fn new_from_file(identity_file_name: &str) -> anyhow::Result<Self> {
        let mut identity_file = std::fs::read_to_string(identity_file_name)?;
        identity_file.retain(|c| c.is_ascii());
        let identity: OmegaStrikersIdentity = serde_json::from_str(&identity_file)?;

        let mut headers = HeaderMap::new();
        let x_authorization = HeaderName::from_static("x-authorization");
        headers.insert(
            x_authorization,
            format!("Bearer {}", identity.access_tokens.jwt).parse()?,
        );
        let x_refresh_token = HeaderName::from_static("x-refresh-token");
        headers.insert(
            x_refresh_token,
            identity.access_tokens.refresh_token.parse()?,
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;

        Ok(Self { client })
    }

    pub async fn search_user_name(&self, query: &str) -> anyhow::Result<Vec<OmegaStrikersUser>> {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct OmegaStrikersUserSearchResponse {
            matches: Vec<OmegaStrikersUser>,
        }

        let response = self
            .client
            .get(format!("{}/players", OMEGASTRIKERS_INNER_API_URL))
            .query(&[("page", "1"), ("pageSize", "5"), ("usernameQuery", query)])
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!(
                "failed to search name due to an api error {}: {:?}",
                response.status(),
                response.text().await
            ));
        }

        let user_search_response: OmegaStrikersUserSearchResponse = response.json().await?;

        Ok(user_search_response.matches)
    }
}
