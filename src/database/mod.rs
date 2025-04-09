pub mod group;
pub mod team;
pub mod user;

use sqlx::{migrate, PgPool};

#[derive(Debug)]
pub struct DB {
    pub pool: PgPool,
}

impl DB {
    pub async fn from_pool(pool: PgPool) -> anyhow::Result<Self> {
        let migrator = migrate!("./migrations");
        migrator.run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(url).await?;

        Self::from_pool(pool).await
    }
}
