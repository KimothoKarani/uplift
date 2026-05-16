use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub stripe_customer_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct OrgRepo;

impl OrgRepo {
    pub async fn create(pool: &PgPool, name: &str, slug: &str) -> Result<Organization> {
        let org = sqlx::query_as!(
            Organization,
            "INSERT INTO organizations (name, slug) VALUES ($1, $2) RETURNING *",
            name,
            slug,
        )
        .fetch_one(pool)
        .await?;
        
        Ok(org)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Organization> {
        sqlx::query_as!(
            Organization,
            "SELECT * FROM organizations WHERE id = $1",
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }

    pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Organization> {
        sqlx::query_as!(
            Organization,
            "SELECT * FROM organizations WHERE slug = $1",
            slug,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }

    pub async fn set_stripe_customer(
        pool: &PgPool,
        id: Uuid,
        customer_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE organizations SET stripe_customer_id = $1 WHERE id = $2",
            customer_id,
            id,
        )
        .execute(pool)
        .await?;
        
        Ok(())
    }


}
