use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::StartGGClient;

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/startgg/graphql_schema.json",
    query_path = "src/startgg/tournament.graphql",
    response_derives = "Debug,Serialize"
)]
struct TournamentTeams;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/startgg/graphql_schema.json",
    query_path = "src/startgg/tournament.graphql",
    response_derives = "Debug,Serialize"
)]
struct UserTournaments;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/startgg/graphql_schema.json",
    query_path = "src/startgg/tournament.graphql",
    response_derives = "Debug,Serialize"
)]
struct Tournament;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGGImage {
    pub url: String,
    pub height: f64,
    pub width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGGTournament {
    pub name: String,
    pub images: Vec<StartGGImage>,
    pub slug: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGGTeamMember {
    pub gamer_tag: String,
    pub prefix: Option<String>,
    pub capitain: bool,
    pub alternate: bool,
    pub discord_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGGTeam {
    pub name: String,
    pub nickname: Option<String>,
    pub image: Option<StartGGImage>,
    pub id: String,
    pub team_members: Vec<StartGGTeamMember>,
}

impl StartGGClient<'_> {
    pub async fn fetch_tournaments_organized_by_user(
        &self,
    ) -> anyhow::Result<Vec<StartGGTournament>> {
        let var = user_tournaments::Variables {
            page: Some(1),
            per_page: Some(25),
        };
        let query = UserTournaments::build_query(var);

        let response_body: graphql_client::Response<user_tournaments::ResponseData> =
            self.graphql_request(&query).await?;

        if let Some(err) = response_body.errors {
            return Err(anyhow::anyhow!(
                "errors fetching user's tournaments: {err:?}"
            ));
        }

        let user: Option<Vec<StartGGTournament>> = (|| {
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
                    Some(StartGGTournament {
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
                        slug: t.slug?.trim_start_matches("tournament/").to_string(),
                        url: t.url?,
                    })
                })
                .collect::<Vec<_>>();

            Some(tournaments)
        })();

        user.ok_or(anyhow::anyhow!("failed to get startgg user information"))
    }

    pub async fn fetch_tournament(
        &self,
        tournament_slug: String,
    ) -> anyhow::Result<StartGGTournament> {
        let var = tournament::Variables {
            tournament: Some(tournament_slug.clone()),
        };
        let query = Tournament::build_query(var);

        let response_body: graphql_client::Response<tournament::ResponseData> =
            self.graphql_request(&query).await?;

        if let Some(err) = response_body.errors {
            return Err(anyhow::anyhow!("errors fetching tournament info: {err:?}"));
        }

        let tournament: Option<StartGGTournament> = (|| {
            let t = response_body.data?.tournament?;
            Some(StartGGTournament {
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
                slug: t.slug?.trim_start_matches("tournament/").to_string(),
                url: t.url?,
            })
        })();

        tournament.ok_or(anyhow::anyhow!(
            "failed to get startgg tournament information for tournament '{tournament_slug}'"
        ))
    }

    pub async fn fetch_tournament_teams(
        &self,
        tournament_slug: String,
    ) -> anyhow::Result<Vec<StartGGTeam>> {
        let var = tournament_teams::Variables {
            tournament: Some(tournament_slug),
        };
        let query = TournamentTeams::build_query(var);

        let response_body: graphql_client::Response<tournament_teams::ResponseData> =
            self.graphql_request(&query).await.inspect_err(|e| {
                e.backtrace();
            })?;

        if let Some(err) = response_body.errors {
            return Err(anyhow::anyhow!(
                "errors fetching tournament's teams: {err:?}"
            ));
        }

        let teams: Option<Vec<StartGGTeam>> = (|| {
            let tournament = response_body.data?.tournament?;
            let teams: Vec<_> = tournament
                .teams?
                .nodes?
                .into_iter()
                .filter_map(|team| {
                    let t = match team? {
                        tournament_teams::TournamentTeamsTournamentTeamsNodes::EventTeam(
                            tournament_teams_tournament_teams_nodes_on_event_team,
                        ) => tournament_teams_tournament_teams_nodes_on_event_team.global_team,
                        _ => None,
                    }?;
                    Some(StartGGTeam {
                        name: t.name?,
                        nickname: None,
                        image: t.images.unwrap_or_default().into_iter().find_map(|image| {
                            let img = image?;
                            Some(StartGGImage {
                                url: img.url?,
                                height: img.height?,
                                width: img.width?,
                            })
                        }),
                        id: t.discriminator?,
                        team_members: t
                            .members
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|member| {
                                let m = member?;
                                let player = m.player?;
                                Some(StartGGTeamMember {
                                    gamer_tag: player.gamer_tag?,
                                    prefix: player.prefix,
                                    capitain: m.is_captain?,
                                    alternate: m.is_alternate?,
                                    discord_id: m
                                        .participant?
                                        .required_connections?
                                        .into_iter()
                                        .find_map(|conn| {
                                            let c = conn?;
                                            match c.type_? {
                                                tournament_teams::AuthorizationType::DISCORD => {
                                                    c.external_id
                                                }
                                                _ => None,
                                            }
                                        })?,
                                })
                            })
                            .collect(),
                    })
                })
                .collect();
            Some(teams)
        })();
        let mut ts = teams.ok_or(anyhow::anyhow!("failed to get tournament team info"))?;

        let mut seen_ids = HashSet::new();
        ts.retain(|team| seen_ids.insert(team.id.clone()));

        Ok(ts)
    }
}
