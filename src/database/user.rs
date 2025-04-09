use super::DB;

#[derive(Debug)]
pub struct User {
    pub username: String,
    pub discord: String,
    pub omegastrikers_id: Option<String>,
    pub startgg_id: Option<String>,
    // created_at: chrono::DateTime<chrono::Utc>,
    // updated_at: chrono::NaiveDateTime,
    // "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    // "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
}

impl DB {
    pub async fn get_user(&self, username: &str) -> Result<User, sqlx::Error> {
        sqlx::query!("SELECT * from users WHERE username = $1", username)
            .fetch_one(&self.pool)
            .await
            .map(|row| User {
                username: row.username,
                discord: row.discord,
                omegastrikers_id: row.omegastrikers_id,
                startgg_id: row.startgg_id,
            })
    }

    pub async fn upsert_user(
        &self,
        user: &User,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        let query = sqlx::query!(
            r#"INSERT INTO users (username, discord, omegastrikers_id, startgg_id, created_at, updated_at)
               VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
               ON CONFLICT (username) DO
                   UPDATE SET discord = $2, omegastrikers_id = $3, startgg_id = $4, updated_at = CURRENT_TIMESTAMP
            "#,
            user.username,
            user.discord,
            user.omegastrikers_id,
            user.startgg_id,
        );

        if let Err(sqlx::Error::RowNotFound) = self.get_user(&user.username).await {
            let result = query.execute(&self.pool).await?;

            Ok(result)
        } else {
            query.execute(&self.pool).await
        }
    }
}
