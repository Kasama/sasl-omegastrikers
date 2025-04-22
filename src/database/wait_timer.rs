use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use uuid::Uuid;

use super::DB;

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitType {
    Nothing,
    Starting,
    Break,
    Ending,
}

impl WaitType {
    pub fn from_str(s: &str) -> Result<Self, WaitTimerError> {
        match s {
            "nothing" => Ok(WaitType::Nothing),
            "starting" => Ok(WaitType::Starting),
            "break" => Ok(WaitType::Break),
            "ending" => Ok(WaitType::Ending),
            _ => Err(WaitTimerError::InvalidWaitType(s.to_string())),
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            WaitType::Nothing => "nothing",
            WaitType::Starting => "starting",
            WaitType::Break => "break",
            WaitType::Ending => "ending",
        }
    }
}

#[derive(Debug, Clone)]
enum WaitTimerError {
    InvalidWaitType(String),
}

impl std::fmt::Display for WaitTimerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitTimerError::InvalidWaitType(ref s) => write!(f, "Invalid wait type: {}", s),
        }
    }
}

impl std::error::Error for WaitTimerError {}

#[derive(Debug, Clone)]
pub struct WaitTimer {
    pub overlay_id: Uuid,
    pub wait_until: DateTime<FixedOffset>,
    pub wait_type: WaitType,
}

impl DB {
    pub async fn get_wait_timer(&self, overlay_id: &Uuid) -> Result<WaitTimer, sqlx::Error> {
        sqlx::query!("SELECT * from wait_timer WHERE overlay_id = $1", overlay_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| sqlx::Error::RowNotFound)
            .and_then(|row| {
                Ok(WaitTimer {
                    overlay_id: row.overlay_id,
                    wait_until: chrono::DateTime::parse_from_rfc2822(&row.wait_until)
                        .map_err(|e| sqlx::Error::Encode(Box::new(e)))?,
                    wait_type: WaitType::from_str(&row.wait_type)
                        .map_err(|e| sqlx::Error::Encode(Box::new(e)))?,
                }) as sqlx::Result<_>
            })
    }

    pub async fn upsert_wait_timer(&self, wait_timer: &WaitTimer) -> Result<(), anyhow::Error> {
        let query = sqlx::query!(
            "INSERT INTO wait_timer (overlay_id, wait_until, wait_type) VALUES ($1, $2, $3) ON CONFLICT (overlay_id) DO UPDATE SET wait_until = $2, wait_type = $3",
            wait_timer.overlay_id,
            wait_timer.wait_until.to_rfc2822(),
            wait_timer.wait_type.to_str(),
        );

        let response = query.execute(&self.pool).await?;
        if response.rows_affected() > 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to create new WaitTimer"))
        }
    }
}
