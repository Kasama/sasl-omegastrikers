use anyhow::Context;

use super::oauth::StartggUser;
use super::StartGGClient;

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/startgg/graphql_schema.json",
    query_path = "src/startgg/currentUser.graphql",
    response_derives = "Debug,Serialize"
)]
struct CurrentUser;

impl StartGGClient<'_> {
    pub async fn fetch_startgg_user(&self) -> anyhow::Result<StartggUser> {
        let var = current_user::Variables {};
        let query = CurrentUser::build_query(var);

        let response_body: graphql_client::Response<current_user::ResponseData> = self
            .graphql_request(&query)
            .await
            .context("Failed to parse user info JSON response from start.gg")?;

        let user: Option<StartggUser> = (|| {
            let cuser = response_body.data?.current_user?;
            let gamer_tag = cuser.player?.gamer_tag;
            let slug = cuser.slug?;

            Some(StartggUser { slug, gamer_tag })
        })();

        user.ok_or(anyhow::anyhow!("failed to get startgg user information"))
    }
}
