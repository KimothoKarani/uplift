use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::crypto::Cipher;
use crate::error::{Error, Result};

/// What callers work with - tokens are plaintext
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub google_account_email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expires_at: DateTime<Utc>,
}

/// The raw DB row - tokens still encrypted. Never leaves this module
#[derive(sqlx::FromRow)]
struct ConnectionRow {
    id: Uuid,
    organization_id: Uuid,
    google_account_email: String,
    access_token: String,
    refresh_token: String,
    token_expires_at: DateTime<Utc>,
}

impl ConnectionRow {
    fn decrypt(self, cipher: &Cipher) -> Result<Connection> {
        Ok(Connection { 
            id: self.id, 
            organization_id: self.organization_id, 
            google_account_email: self.google_account_email, 
            access_token: cipher.decrypt(&self.access_token)?, 
            refresh_token: cipher.decrypt(&self.refresh_token)?, 
            token_expires_at: self.token_expires_at,
        })
    }
}

pub struct ConnectionRepo;

impl ConnectionRepo {
    /// Called after the OAuth callback — stores a fresh token set.
    pub async fn upsert(
        pool: &PgPool,
        cipher: &Cipher,
        org_id: Uuid,
        google_account_email: &str,
        access_token: &str,
        refresh_token: &str,
        token_expires_at: DateTime<Utc>,
    ) -> Result<Connection> {
        let access_enc = cipher.encrypt(access_token)?;
        let refresh_enc = cipher.encrypt(refresh_token)?;

        let row = sqlx::query_as!(
            ConnectionRow,
            r#"
            INSERT INTO google_connections
                (organization_id, google_account_email, access_token, refresh_token, token_expires_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (organization_id, google_account_email) DO UPDATE SET
                access_token     = EXCLUDED.access_token,
                refresh_token    = EXCLUDED.refresh_token,
                token_expires_at = EXCLUDED.token_expires_at
            RETURNING
                id, organization_id, google_account_email,
                access_token, refresh_token, token_expires_at
            "#,
            org_id,
            google_account_email,
            access_enc,
            refresh_enc,
            token_expires_at,
        )
        .fetch_one(pool)
        .await?;

        row.decrypt(cipher)
    }

    /// Called by the refresh-tokens job — only the access token changes.
    pub async fn update_access_token(
        pool: &PgPool,
        cipher: &Cipher,
        id: Uuid,
        access_token: &str,
        token_expires_at: DateTime<Utc>,
    ) -> Result<()> {
        let access_enc = cipher.encrypt(access_token)?;
        sqlx::query!(
            r#"
            UPDATE google_connections
            SET access_token = $1, token_expires_at = $2
            WHERE id = $3
            "#,
            access_enc,
            token_expires_at,
            id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(
        pool: &PgPool,
        cipher: &Cipher,
        id: Uuid,
    ) -> Result<Connection> {
        let row = sqlx::query_as!(
            ConnectionRow,
            r#"
            SELECT id, organization_id, google_account_email,
                   access_token, refresh_token, token_expires_at
            FROM google_connections WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)?;

        row.decrypt(cipher)
    }

    pub async fn list_by_org(
        pool: &PgPool,
        cipher: &Cipher,
        org_id: Uuid,
    ) -> Result<Vec<Connection>> {
        let rows = sqlx::query_as!(
            ConnectionRow,
            r#"
            SELECT id, organization_id, google_account_email,
                   access_token, refresh_token, token_expires_at
            FROM google_connections WHERE organization_id = $1
            "#,
            org_id,
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(|r| r.decrypt(cipher)).collect()
    }

    pub async fn delete(pool: &PgPool, id: Uuid, org_id: Uuid) -> Result<()> {
        sqlx::query!(
            "DELETE FROM google_connections WHERE id = $1 AND organization_id = $2",
            id,
            org_id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}