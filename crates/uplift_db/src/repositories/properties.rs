use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Property {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub google_connection_id: Uuid,
    pub ga4_property_id: String,
    pub display_name: String,
    pub website_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct PropertyRepo;

impl PropertyRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: Uuid,
        connection_id: Uuid,
        ga4_property_id: &str,
        display_name: &str,
        website_url: Option<&str>,
    ) -> Result<Property> {
        let p = sqlx::query_as!(
            Property,
            r#"
            INSERT INTO ga4_properties
                (organization_id, google_connection_id, ga4_property_id, display_name, website_url)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
            org_id,
            connection_id,
            ga4_property_id,
            display_name,
            website_url,
        )
        .fetch_one(pool)
        .await?;
        Ok(p)
    }

    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<Property>> {
        Ok(sqlx::query_as!(
            Property,
            "SELECT * FROM ga4_properties WHERE organization_id = $1 ORDER BY created_at",
            org_id,
        )
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid, org_id: Uuid) -> Result<Property> {
        sqlx::query_as!(
            Property,
            "SELECT * FROM ga4_properties WHERE id = $1 AND organization_id = $2",
            id,
            org_id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }

    pub async fn delete(pool: &PgPool, id: Uuid, org_id: Uuid) -> Result<()> {
        sqlx::query!(
            "DELETE FROM ga4_properties WHERE id = $1 AND organization_id = $2",
            id,
            org_id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
