use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

// use crate::db_schema::User;
use super::user::User;

// Database repository
pub struct AuthRepository {
    pool: PgPool,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        full_name: Option<&str>,
    ) -> Result<(Uuid, String), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO users (email, password_hash, full_name)
            VALUES ($1, $2, $3)
            RETURNING id, email
            "#,
            email,
            password_hash,
            full_name
        )
        .fetch_one(&self.pool)
        .await
        .map(|row| (row.id, row.email))
    }

    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<(Uuid, String, String)>, sqlx::Error> {
        sqlx::query!(
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|row| (row.id, row.email, row.password_hash)))
    }

    pub async fn store_refresh_token(
        &self,
        user_id: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (user_id, token, expires_at)
            VALUES ($1, $2, $3)
            "#,
            user_id,
            token,
            sqlx::types::time::OffsetDateTime::from_unix_timestamp(expires_at.timestamp()).unwrap()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn verify_refresh_token(&self, token: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query!(
            r#"
            SELECT u.id, u.email, u.password_hash, u.full_name, u.created_at, u.updated_at
            FROM users u
            INNER JOIN refresh_tokens rt ON rt.user_id = u.id
            WHERE rt.token = $1 AND rt.expires_at > CURRENT_TIMESTAMP
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await
        .map(|user| {
            user.map(|real_user| User {
                id: real_user.id,
                email: real_user.email,
                password_hash: real_user.password_hash,
                full_name: real_user.full_name,
                balance: 0.into(),
                status: "active".to_string(),
                created_at: super::utils::convert_offsetdt_to_dt(real_user.created_at.unwrap()),
                updated_at: super::utils::convert_offsetdt_to_dt(real_user.updated_at.unwrap()),
            })
        })
    }
}
