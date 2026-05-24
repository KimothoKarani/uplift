use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub struct SessionRepo;

impl SessionRepo {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<Session> {
        let session = sqlx::query_as!(
            Session,
            "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
            user_id,
            expires_at,
        )
        .fetch_one(pool)
        .await?;
        Ok(session)
    }

    /// Returns the session only if it exists and has not expired.
    pub async fn find_valid(pool: &PgPool, id: Uuid) -> Result<Session> {
        sqlx::query_as!(
            Session,
            "SELECT * FROM sessions WHERE id = $1 AND expires_at > NOW()",
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Logs the user out of all devices.
    pub async fn delete_for_user(pool: &PgPool, user_id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
