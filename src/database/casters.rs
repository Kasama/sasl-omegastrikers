use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DB;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CasterKind {
    Narrator,
    Commenter,
}

impl Display for CasterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CasterKind::Narrator => write!(f, "narrator"),
            CasterKind::Commenter => write!(f, "commenter"),
        }
    }
}

impl FromStr for CasterKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "narrator" => Ok(CasterKind::Narrator),
            "commenter" => Ok(CasterKind::Commenter),
            _ => Err(anyhow::anyhow!("Unknown caster kind")),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Caster {
    pub overlay_id: Uuid,
    pub name: String,
    pub kind: CasterKind,
    pub stream_video: String,
}

impl DB {
    pub async fn get_casters(&self, overlay_id: &Uuid) -> Result<Vec<Caster>, anyhow::Error> {
        sqlx::query!(
            "SELECT * from casters WHERE overlay_id = $1",
            overlay_id.to_string()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get team: {}", e))
        .inspect_err(|e| tracing::error!("{}", e))
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    let error_msg = format!("got invalid kind for caster {}", &row.name);
                    Caster {
                        overlay_id: Uuid::parse_str(&row.overlay_id).unwrap(),
                        name: row.name,
                        kind: row.kind.parse().expect(&error_msg),
                        stream_video: row.stream_video,
                    }
                })
                .collect::<Vec<_>>()
        })
    }

    pub async fn upsert_caster(&self, caster: &Caster) -> Result<(), anyhow::Error> {
        let query = sqlx::query!(
            "INSERT INTO casters (overlay_id, kind, name, stream_video) VALUES ($1, $2, $3, $4)
            ON CONFLICT (overlay_id, kind) DO UPDATE SET name = $3, stream_video = $4",
            caster.overlay_id.to_string(),
            caster.kind.to_string(),
            caster.name,
            caster.stream_video,
        );
        query
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("failed to upsert team: {}", e.to_string()))
            .inspect_err(|e| tracing::error!("{}", e))?;
        Ok(())
    }
}
