use uuid::Uuid;

use super::DB;

#[derive(Debug, Clone)]
pub struct Scoreboard {
    pub overlay_id: Uuid,
    pub team_a: String,
    pub team_b: String,
    pub team_a_score: i32,
    pub team_b_score: i32,
    pub team_a_standing: String,
    pub team_b_standing: String,
}

impl DB {
    pub async fn get_scoreboard(&self, overlay_id: Uuid) -> Result<Scoreboard, anyhow::Error> {
        sqlx::query!(
            r#"SELECT * from scoreboard
               WHERE overlay_id = $1
            "#,
            overlay_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get team: {}", e))
        .inspect_err(|e| tracing::error!("{}", e))
        .map(|row| Scoreboard {
            overlay_id,
            team_a: row.team_a,
            team_b: row.team_b,
            team_a_score: row.team_a_score,
            team_b_score: row.team_b_score,
            team_a_standing: row.team_a_standing.unwrap_or_default(),
            team_b_standing: row.team_b_standing.unwrap_or_default(),
        })
    }

    pub async fn upsert_scoreboard(
        &self,
        scoreboard: Scoreboard,
    ) -> Result<Scoreboard, anyhow::Error> {
        let query = sqlx::query!(
            r#"INSERT INTO scoreboard
                (overlay_id, team_a, team_b, team_a_score, team_b_score, team_a_standing, team_b_standing)
                VALUES
                ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (overlay_id) DO
                UPDATE SET
                    team_a = $2,
                    team_b = $3,
                    team_a_score = $4,
                    team_b_score = $5,
                    team_a_standing = $6,
                    team_b_standing = $7
            "#,
            scoreboard.overlay_id,
            scoreboard.team_a,
            scoreboard.team_b,
            scoreboard.team_a_score,
            scoreboard.team_b_score,
            scoreboard.team_a_standing,
            scoreboard.team_b_standing,
        );
        query
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("failed to upsert scoreboard: {}", e.to_string()))
            .inspect_err(|e| tracing::error!("{}", e))?;
        Ok(scoreboard)
    }
}
