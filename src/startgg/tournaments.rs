use serde::{Deserialize, Serialize};

use super::StartGGClient;

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/startgg/graphql_schema.json",
    query_path = "src/startgg/usersOmegaTournaments.graphql",
    response_derives = "Debug,Serialize"
)]
struct UserTournaments;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGGImage {
    pub url: String,
    pub height: f64,
    pub width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub name: String,
    pub images: Vec<StartGGImage>,
    pub slug: String,
    pub url: String,
}

impl StartGGClient<'_> {
    pub async fn fetch_tournaments_organized_by_user(&self) -> anyhow::Result<Vec<Tournament>> {
        let var = user_tournaments::Variables {
            page: Some(1),
            per_page: Some(25),
        };
        let query = UserTournaments::build_query(var);

        let response_body: graphql_client::Response<user_tournaments::ResponseData> =
            self.graphql_request(&query).await?;

        let user: Option<Vec<Tournament>> = (|| {
            let cuser = response_body.data?.current_user?.tournaments?.nodes?;
            let tournaments = cuser
                .into_iter()
                .filter(|t| {
                    if let Some(tournaments) = t {
                        tournaments.admins.is_some()
                    } else {
                        false
                    }
                })
                .filter_map(|tournament| {
                    let t = tournament?;
                    Some(Tournament {
                        name: t.name?,
                        images: t
                            .images
                            .unwrap_or_default()
                            .into_iter()
                            .flat_map(|image| {
                                let i = image?;
                                Some(StartGGImage {
                                    url: i.url?,
                                    height: i.height?,
                                    width: i.width?,
                                })
                            })
                            .collect(),
                        slug: t.slug?,
                        url: t.url?,
                    })
                })
                .collect::<Vec<_>>();

            Some(tournaments)
        })();

        user.ok_or(anyhow::anyhow!("failed to get startgg user information"))
    }
}
