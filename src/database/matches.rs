use futures_util::future::join_all;
use uuid::Uuid;

use crate::startgg::tournaments::StartGGTeam;

use super::DB;

#[derive(Debug, Clone)]
pub struct Match {
    pub id: Uuid,
    pub overlay_id: Option<Uuid>,
    pub tournament_slug: String,
    pub team_a: StartGGTeam,
    pub team_b: StartGGTeam,
    pub team_a_score: i32,
    pub team_b_score: i32,
    pub completed: bool,
    pub in_progress: bool,
    pub featured: bool,
}

impl DB {
    pub async fn get_match(&self, id: Uuid) -> Result<Match, anyhow::Error> {
        sqlx::query!(
            r#"SELECT * from matches as matches
               WHERE matches.id = $1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get team: {}", e))
        .inspect_err(|e| tracing::error!("{}", e))
        .map(|row| async move {
            Ok(Match {
                id: row.id,
                overlay_id: row.overlay_id,
                tournament_slug: row.tournament_slug,
                team_a: self.get_team(&row.team_a).await?,
                team_b: self.get_team(&row.team_b).await?,
                team_a_score: row.team_a_score,
                team_b_score: row.team_b_score,
                completed: row.completed,
                in_progress: row.in_progress,
                featured: row.featured,
            }) as anyhow::Result<_>
        })?
        .await
    }

    pub async fn get_overlay_matches(&self, overlay_id: Uuid) -> Result<Vec<Match>, anyhow::Error> {
        let matches_fut = sqlx::query!(
            "SELECT * from matches WHERE overlay_id = $1 ORDER BY created_at ASC",
            overlay_id
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| async move {
                    Ok(Match {
                        id: row.id,
                        overlay_id: row.overlay_id,
                        tournament_slug: row.tournament_slug,
                        team_a: self.get_team(&row.team_a).await?,
                        team_b: self.get_team(&row.team_b).await?,
                        team_a_score: row.team_a_score,
                        team_b_score: row.team_b_score,
                        completed: row.completed,
                        in_progress: row.in_progress,
                        featured: row.featured,
                    }) as anyhow::Result<_>
                })
                .collect::<Vec<_>>()
        })?;

        join_all(matches_fut)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn upsert_match(&self, match_: Match) -> Result<Match, anyhow::Error> {
        let match_ = if match_.id.is_nil() {
            Match {
                id: Uuid::new_v4(),
                ..match_
            }
        } else {
            match_
        };
        let query = sqlx::query!(
            r#"INSERT INTO matches
                (id, overlay_id, tournament_slug, team_a, team_b, team_a_score, team_b_score, completed, in_progress, featured)
                VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (id) DO
                UPDATE SET
                    overlay_id = $2,
                    tournament_slug = $3,
                    team_a = $4,
                    team_b = $5,
                    team_a_score = $6,
                    team_b_score = $7,
                    completed = $8,
                    in_progress = $9,
                    featured = $10,
                    updated_at = now()
            "#,
            match_.id,
            match_.overlay_id,
            match_.tournament_slug,
            match_.team_a.id,
            match_.team_b.id,
            match_.team_a_score,
            match_.team_b_score,
            match_.completed,
            match_.in_progress,
            match_.featured,
        );
        query
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("failed to upsert match: {}", e.to_string()))
            .inspect_err(|e| tracing::error!("{}", e))?;
        Ok(match_)
    }
}
