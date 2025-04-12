use uuid::Uuid;

use super::DB;

#[derive(Debug, Clone)]
pub struct Overlay {
    pub id: Uuid,
    pub tournament_slug: String,
    pub name: Option<String>,
}

impl DB {
    pub async fn get_overlay(&self, id: Uuid) -> Result<Overlay, sqlx::Error> {
        sqlx::query!("SELECT * from stream_overlay WHERE id = $1", id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| sqlx::Error::RowNotFound)
            .map(|row| Overlay {
                id: row.id,
                tournament_slug: row.tournament_slug,
                name: row.name,
            })
    }

    pub async fn get_tournament_overlays(
        &self,
        tournament_slug: &str,
    ) -> Result<Vec<Overlay>, sqlx::Error> {
        Ok(sqlx::query!(
            "SELECT * from stream_overlay WHERE tournament_slug = $1",
            tournament_slug
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| Overlay {
            id: row.id,
            tournament_slug: row.tournament_slug,
            name: row.name,
        })
        .collect::<Vec<_>>())
    }

    pub async fn create_overlay(&self, tournament_slug: &str) -> Result<Overlay, anyhow::Error> {
        let id = uuid::Uuid::new_v4();
        let query = sqlx::query!(
            "INSERT INTO stream_overlay (id, tournament_slug) VALUES ($1, $2)",
            id,
            tournament_slug
        );

        let response = query.execute(&self.pool).await?;
        if response.rows_affected() > 0 {
            Ok(Overlay {
                id,
                tournament_slug: tournament_slug.to_string(),
                name: None,
            })
        } else {
            Err(anyhow::anyhow!("failed to create new overlay"))
        }
    }

    pub async fn update_overlay(&self, id: Uuid, name: &str) -> Result<Overlay, anyhow::Error> {
        let query = sqlx::query!(
            "UPDATE stream_overlay SET name = $1 WHERE id = $2",
            name,
            id,
        );

        let response = query.execute(&self.pool).await?;
        if response.rows_affected() > 0 {
            self.get_overlay(id)
                .await
                .map_err(|_| anyhow::anyhow!("failed to get updated overlay"))
        } else {
            Err(anyhow::anyhow!("failed to update new overlay"))
        }
    }

    pub async fn assign_teams(
        &self,
        overlay_id: Uuid,
        team_a: &str,
        team_b: &str,
        score_a: &str,
        score_b: &str,
    ) -> Result<(), anyhow::Error> {
        let query = sqlx::query!(
            "UPDATE stream_overlay SET team_a = $1, team_b = $2, score_a = $3, score_b = $4 WHERE id = $5",
            team_a,
            team_b,
            score_a,
            score_b,
            overlay_id
        );
        let response = query.execute(&self.pool).await?;
        if response.rows_affected() > 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to assign teams to overlay"))
        }
    }

    pub async fn delete_overlay(&self, overlay_id: Uuid) -> Result<(), anyhow::Error> {
        let query = sqlx::query!("DELETE FROM stream_overlay WHERE id = $1", overlay_id);
        let response = query.execute(&self.pool).await?;
        if response.rows_affected() > 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to delete overlay"))
        }
    }
}
