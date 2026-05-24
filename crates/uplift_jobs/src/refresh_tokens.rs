use apalis::prelude::{BoxDynError, Data};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uplift_connectors::google::auth::GoogleOAuth;
use uplift_db::ConnectionRepo;

use crate::{Error, JobContext, Result};

/// Proactively refresh an expiring Google OAuth access token.
/// Enqueued on a schedule for every active connection - not user-triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokensJob {
    pub connection_id: Uuid,
}

pub async fn handle(
    job: RefreshTokensJob,
    ctx: Data<JobContext>,
) -> std::result::Result<(), BoxDynError> {
    refresh(job, &ctx).await.map_err(Into::into)
}

async fn refresh(job: RefreshTokensJob, ctx: &JobContext) -> Result<()> {
    tracing::info!(
        connection_id = %job.connection_id, "checking token expiry"
    );

    let conn = ConnectionRepo::find_by_id(&ctx.pool, &ctx.cipher, job.connection_id)
        .await
        .map_err(|_| Error::NotFound)?;

    // 10-minute buffer - wider than the 5-minute check in FetchTimeSeriesJob.
    // This job fires on a schedule which can slio, so we want to refresh well
    // before other jobs hot their own inline check.
    let expires_soon = conn.token_expires_at <= Utc::now() + chrono::Duration::minutes(10);

    if !expires_soon {
        tracing::info!(
            connection_id = %conn.id,
            expires_at = %conn.token_expires_at,
            "token still valid - skipping"
        );
        return Ok(());
    }

    tracing::info!(
        connection_id = %conn.id,
        expires_at    = %conn.token_expires_at,
        "token expiring — refreshing"
    );

    let oauth = GoogleOAuth::new(
        ctx.google_client_id.clone(),
        ctx.google_client_secret.clone(),
        ctx.google_redirect_uri.clone(),
    )
    .map_err(|e| Error::TokenRefresh {
        connection_id: conn.id,
        reason: e.to_string(),
    })?;

    let fresh = oauth
        .refresh(&conn.refresh_token, &ctx.http)
        .await
        .map_err(|e| Error::TokenRefresh {
            connection_id: conn.id,
            reason: e.to_string(),
        })?;

    ConnectionRepo::update_access_token(
        &ctx.pool,
        &ctx.cipher,
        conn.id,
        &fresh.access_token,
        fresh.expires_at,
    )
    .await?;

    tracing::info!(
        connection_id = %conn.id,
        new_expires_at = %fresh.expires_at,
        "token refreshed successfully"
    );

    Ok(())
}
