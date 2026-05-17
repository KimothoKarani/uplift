use apalis::prelude::{BoxDynError, Data};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uplift_connectors::google::{
    analytics::{Ga4Client, Ga4Metric},
    auth::GoogleOAuth,
};
use uplift_connectors::normalize;
use uplift_db::{ConnectionRepo, TimeSeriesRepo};

use crate::{Error, JobContext, Result};

/// Pull one metric for one property from GA4 and cache it in time_series_data.
/// Triggered by RunAnalysisJob when local cache is missing or stale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchTimeSeriesJob {
    /// Our internal DB uuid for the ga4_properties row - the cache key.
    pub property_id: Uuid,
    /// Our internal DB uuid for the google_connections row — where tokens live.
    pub connection_id: Uuid,
    /// The GA4 numeric property ID string e.g. "123456789" — what Google's API expects.
    pub ga4_property_id: String,
    /// Metric name as we store it e.g. "sessions", "activeUsers".
    pub metric: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

pub async fn handle(
    job: FetchTimeSeriesJob,
    ctx: Data<JobContext>,
) -> std::result::Result<(), BoxDynError> {
    fetch(job, &ctx).await.map_err(Into::into)
}

async fn fetch(job: FetchTimeSeriesJob, ctx: &JobContext) -> Result<()> {
    tracing::info!(
        connection_id = %job.connection_id,
        property_id = %job.property_id,
        metric = %job.metric,
        start= %job.start,
        end = %job.end,
        "fetching time series from GA4"
    );

    // Load the connection - tokens come back already decrypted
    let mut conn = ConnectionRepo::find_by_id(&ctx.pool, &ctx.cipher, job.connection_id)
        .await
        .map_err(|_| Error::NotFound)?;

    // Refresh the access token if it expires within 5 minutes.
    // Belt-and-braces: RefreshTokensJob runs on a schedule but schedules slip.
    if conn.token_expires_at <= Utc::now() + chrono::Duration::minutes(5) {
        tracing::info!(connection_id = %conn.id, "token expiring - refreshing inline");

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
                reason: e.to_string()
            })?;
        
        ConnectionRepo::update_access_token(
            &ctx.pool, 
            &ctx.cipher, 
            conn.id, 
            &fresh.access_token, 
            fresh.expires_at
        )
        .await?;
        
        conn.access_token = fresh.access_token;
    }

    let ga4_metric = metric_from_str(&job.metric)?;
    let ga4 = Ga4Client::new(ctx.http.clone());

    let raw = ga4
        .fetch_daily_metric(
            &conn.access_token, 
            &job.ga4_property_id, 
            ga4_metric, 
            job.start, 
            job.end,
        )
        .await?;

    // normalize fills date gaps with 0.0 - causal model needs a contigoud series
    let series = normalize::into_timeseries(raw, &job.metric)?;

    TimeSeriesRepo::upsert_many(
        &ctx.pool, 
        job.property_id, 
        &job.metric, 
        &series.points).await?;
    
    tracing::info!(
        property_id = %job.property_id,
        metric = %job.metric,
        points = series.points.len(),
        "time series cached"
    );

    Ok(())

}

fn metric_from_str(s: &str) -> Result<Ga4Metric> {
    match s {
        "sessions" => Ok(Ga4Metric::Sessions),
        "activeUsers" => Ok(Ga4Metric::ActiveUsers),
        "conversions" => Ok(Ga4Metric::Conversions),
        "screenPageViews" => Ok(Ga4Metric::ScreenPageViews),
        other => Err(Error::Other(anyhow::anyhow!("unknown GA4 metric: '{other}'"))),
    }
}