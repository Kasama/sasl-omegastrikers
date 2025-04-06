use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod auth;
pub mod oauth;
pub mod tournaments;
pub mod user;

pub const STARTGG_GRAPHQL_ENDPOINT: &str = "https://api.start.gg/gql/alpha";

#[derive(Debug)]
pub struct StartGGClient<'a> {
    client: &'a reqwest::Client,
    token: &'a str,
}

impl<'a> StartGGClient<'a> {
    pub fn new(client: &'a reqwest::Client, token: &'a str) -> Self {
        Self { client, token }
    }

    async fn graphql_request<Query, ResponseData>(
        &self,
        query: &Query,
    ) -> anyhow::Result<graphql_client::Response<ResponseData>>
    where
        Query: Serialize + ?Sized,
        ResponseData: DeserializeOwned,
    {
        let response = self
            .client
            .post(STARTGG_GRAPHQL_ENDPOINT)
            .bearer_auth(self.token)
            .json(query)
            .send()
            .await?;
        // .context("Failed to send tournament info request to start.gg GraphQL API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error body".to_string());
            tracing::error!("start.gg graphql fetch failed: {} - {}", status, body);
            return Err(anyhow::anyhow!(
                "Failed to fetch info. Status: {}. Body: {}",
                status,
                body
            ));
        }

        let response_body: graphql_client::Response<ResponseData> = response.json().await?;

        Ok(response_body)
    }
}
