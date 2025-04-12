use crate::database::user::User;
use crate::startgg::tournaments::{StartGGImage, StartGGTeam};

use super::DB;

#[derive(Debug)]
pub struct Group {
    pub extras: Vec<User>,
    pub forwards: (User, User),
    pub goalie: User,
    pub coach: User,
}

impl DB {
    pub async fn get_team(&self, team_id: &str) -> Result<StartGGTeam, anyhow::Error> {
        sqlx::query!("SELECT * from team WHERE id = $1", team_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("failed to get team: {}", e))
            .inspect_err(|e| tracing::error!("{}", e))
            .map(|row| StartGGTeam {
                name: row.name,
                nickname: row.nickname,
                image: row.image.map(|img| StartGGImage {
                    url: img,
                    height: 0f64,
                    width: 0f64,
                }),
                id: row.id,
                team_members: vec![],
            })
    }

    pub async fn upsert_team(
        &self,
        tournament_slug: String,
        team: &StartGGTeam,
    ) -> Result<(), anyhow::Error> {
        let query = if let Some(nickname) = &team.nickname {
            sqlx::query!(
                "INSERT INTO team (tournament_slug, id, name, nickname, image) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET name = $3, nickname = $4, image = $5",
                tournament_slug,
                team.id,
                team.name,
                nickname,
                team.image.clone().map(|img| img.url)
            )
        } else {
            sqlx::query!(
                "INSERT INTO team (tournament_slug, id, name, image) VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET name = $3, image = $4",
                tournament_slug,
                team.id,
                team.name,
                team.image.clone().map(|img| img.url)
            )
        };
        query
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("failed to upsert team: {}", e.to_string()))
            .inspect_err(|e| tracing::error!("{}", e))?;
        Ok(())
    }
}
