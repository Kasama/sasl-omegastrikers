pub mod group;
pub mod team;
pub mod user;

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{migrate, SqlitePool};

#[derive(Debug)]
pub struct DB {
    pub pool: SqlitePool,
}

impl DB {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await?;

        let migrator = migrate!("./migrations");

        migrator.run(&pool).await?;

        Ok(Self { pool })
    }
}
