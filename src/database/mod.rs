pub mod casters;
pub mod group;
pub mod matches;
pub mod overlay;
pub mod scoreboard;
pub mod team;
pub mod user;
pub mod wait_timer;

use sqlx::PgPool;

#[derive(Debug)]
pub struct DB {
    pub pool: PgPool,
}

impl DB {
    pub async fn from_pool(pool: PgPool) -> anyhow::Result<Self> {
        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(url).await?;

        Self::from_pool(pool).await
    }
}
