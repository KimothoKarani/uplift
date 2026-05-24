use apalis::prelude::{BoxDynError, Data};
use chrono::{NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uplift_connectors::google::{
    analytics::{Ga4Client, Ga4Metric},
    auth::GoogleOAuth,
};
use uplift_connectors::normalize;
use uplift_db::{AnalysisRepo, ConnectionRepo, PropertyRepo, TimeSeriesRepo};

use crate::{Error, JobContext, Result};

/// Run the causal analysis for a submitted analysis request.
/// This job is the core of the product — it ties together every prior phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAnalysisJob {
    pub analysis_id: Uuid,
    /// Included so we can use the existing repo methods which scope by org.
    pub org_id: Uuid,
}

pub async fn handle(
    job: RunAnalysisJob,
    ctx: Data<JobContext>,
) -> std::result::Result<(), BoxDynError> {
    let analysis_id = job.analysis_id;

    match execute(job, &ctx).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // Record the failure before returning — otherwise the analysis
            // row sits at 'running' with no indication of what went wrong.
            tracing::error!(analysis_id = %analysis_id, error = %e, "analysis failed");
            let _ =
                AnalysisRepo::set_status(&ctx.pool, analysis_id, "failed", Some(&e.to_string()))
                    .await;
            Err(e.into())
        }
    }
}

async fn execute(job: RunAnalysisJob, ctx: &JobContext) -> Result<()> {
    tracing::info!(analysis_id = %job.analysis_id, "starting analysis");

    // Step 1 — mark as running so the UI can show a spinner
    AnalysisRepo::set_status(&ctx.pool, job.analysis_id, "running", None).await?;

    // Step 2 — load the analysis configuration
    let analysis = AnalysisRepo::find_by_id(&ctx.pool, job.analysis_id, job.org_id)
        .await
        .map_err(|_| Error::NotFound)?;

    // Step 3 — load the GA4 property to get the real property ID string
    // and which Google connection owns it
    let property = PropertyRepo::find_by_id(&ctx.pool, analysis.property_id, job.org_id)
        .await
        .map_err(|_| Error::NotFound)?;

    // Step 4 — load and decrypt the Google connection
    let mut conn =
        ConnectionRepo::find_by_id(&ctx.pool, &ctx.cipher, property.google_connection_id)
            .await
            .map_err(|_| Error::NotFound)?;

    // Step 5 — refresh token if expiring within 5 minutes
    if conn.token_expires_at <= Utc::now() + chrono::Duration::minutes(5) {
        tracing::info!(connection_id = %conn.id, "token expiring — refreshing before analysis");

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

        conn.access_token = fresh.access_token;
    }

    // Step 6 — fetch fresh GA4 data for the full period (pre + post).
    // We always fetch fresh rather than relying on cache alone — an analysis
    // is a significant user action and stale data would undermine trust.
    let ga4_metric = metric_from_str(&analysis.metric)?;
    let ga4 = Ga4Client::new(ctx.http.clone());

    tracing::info!(
        property_id = %property.ga4_property_id,
        metric      = %analysis.metric,
        start       = %analysis.pre_period_start,
        end         = %analysis.post_period_end,
        "fetching GA4 data"
    );

    let raw = ga4
        .fetch_daily_metric(
            &conn.access_token,
            &property.ga4_property_id,
            ga4_metric,
            analysis.pre_period_start,
            analysis.post_period_end,
        )
        .await?;

    // Step 7 — normalize and cache locally
    let series = normalize::into_timeseries(raw, &analysis.metric)?;

    if series.points.len() < 30 {
        return Err(Error::NoData {
            property_id: analysis.property_id,
            metric: analysis.metric.clone(),
        });
    }

    TimeSeriesRepo::upsert_many(
        &ctx.pool,
        analysis.property_id,
        &analysis.metric,
        &series.points,
    )
    .await?;

    // Step 8 — run the causal model.
    // intervention_date is stored as NaiveDate — convert to DateTime<Utc>
    // at midnight so the partition in run_analysis splits on the right boundary.
    let intervention_dt = analysis
        .intervention_date
        .and_time(NaiveTime::MIN)
        .and_utc();

    tracing::info!(
        analysis_id       = %job.analysis_id,
        intervention_date = %analysis.intervention_date,
        series_len        = series.points.len(),
        "running causal model"
    );

    let report = uplift_core::impact::analysis::run_analysis(&series, intervention_dt, 0.05)?;

    // Step 9 — persist the result
    AnalysisRepo::save_result(&ctx.pool, job.analysis_id, &report).await?;

    // Step 10 — mark complete
    AnalysisRepo::set_status(&ctx.pool, job.analysis_id, "complete", None).await?;

    tracing::info!(
        analysis_id      = %job.analysis_id,
        relative_effect  = %report.summary.relative_effect,
        probability      = %report.summary.probability_of_effect,
        "analysis complete"
    );

    Ok(())
}

fn metric_from_str(s: &str) -> Result<Ga4Metric> {
    match s {
        "sessions" => Ok(Ga4Metric::Sessions),
        "activeUsers" => Ok(Ga4Metric::ActiveUsers),
        "conversions" => Ok(Ga4Metric::Conversions),
        "screenPageViews" => Ok(Ga4Metric::ScreenPageViews),
        other => Err(Error::Other(anyhow::anyhow!(
            "unknown GA4 metric: '{other}'"
        ))),
    }
}
