use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub google_id: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

pub struct UserRepo;

impl UserRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: Uuid,
        email: &str,
        display_name: Option<&str>,
        google_id: Option<&str>,
        role: &str,
    ) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (organization_id, email, display_name, google_id, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
            org_id,
            email,
            display_name,
            google_id,
            role,
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_optional(pool)
            .await?
            .ok_or(Error::NotFound)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email)
            .fetch_optional(pool)
            .await?
            .ok_or(Error::NotFound)
    }

    pub async fn find_by_google_id(pool: &PgPool, google_id: &str) -> Result<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE google_id = $1", google_id,)
            .fetch_optional(pool)
            .await?
            .ok_or(Error::NotFound)
    }

    /// Handles first login (creates) and return login (updates) in one atomic query
    pub async fn upsert_by_google_id(
        pool: &PgPool,
        org_id: Uuid,
        email: &str,
        display_name: Option<&str>,
        google_id: &str,
        role: &str,
    ) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (organization_id, email, display_name, google_id, role)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (google_id) DO UPDATE
                SET display_name = COALESCE(EXCLUDED.display_name, users.display_name),
                    email = EXCLUDED.email
            RETURNING *
            "#,
            org_id,
            email,
            display_name,
            google_id,
            role,
        )
        .fetch_one(pool)
        .await?;
        Ok(user)
    }
}
