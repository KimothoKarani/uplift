use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub stripe_subscription_id: String,
    pub tier: String,
    pub status: String,
    pub current_period_end: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SubscriptionRepo;

impl SubscriptionRepo {
    /// Creates or updates the subscription. Called from the Stripe webhook handler.
    pub async fn upsert(
        pool: &PgPool,
        org_id: Uuid,
        stripe_subscription_id: &str,
        tier: &str,
        status: &str,
        current_period_end: DateTime<Utc>,
    ) -> Result<Subscription> {
        let sub = sqlx::query_as!(
            Subscription,
            r#"
            INSERT INTO subscriptions
                (organization_id, stripe_subscription_id, tier, status, current_period_end)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (organization_id) DO UPDATE SET
                stripe_subscription_id = EXCLUDED.stripe_subscription_id,
                tier                   = EXCLUDED.tier,
                status                 = EXCLUDED.status,
                current_period_end     = EXCLUDED.current_period_end,
                updated_at             = NOW()
            RETURNING *
            "#,
            org_id,
            stripe_subscription_id as &str,
            tier as &str,
            status as &str,
            current_period_end,
        )
        .fetch_one(pool)
        .await?;
        Ok(sub)
    }

    /// Check if an org has an active subscription before allowing an analysis.
    pub async fn find_by_org(
        pool: &PgPool,
        org_id: Uuid,
    ) -> Result<Option<Subscription>> {
        let sub = sqlx::query_as!(
            Subscription,
            "SELECT * FROM subscriptions WHERE organization_id = $1",
            org_id,
        )
        .fetch_optional(pool)
        .await?;
        Ok(sub)
    }

    /// Locate which org a Stripe webhook event belongs to.
    pub async fn find_by_stripe_id(
        pool: &PgPool,
        stripe_subscription_id: &str,
    ) -> Result<Subscription> {
        sqlx::query_as!(
            Subscription,
            "SELECT * FROM subscriptions WHERE stripe_subscription_id = $1",
            stripe_subscription_id as &str,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }
}
