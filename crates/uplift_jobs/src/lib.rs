pub mod error;
pub mod fetch_timeseries;
pub mod refresh_tokens;
pub mod run_analysis;
pub mod send_report;

pub use error::{Error, Result};

use apalis::prelude::{Monitor, Storage, WorkerBuilder, WorkerFactoryFn};
use apalis_sql::postgres::PostgresStorage;
use reqwest::Client;
use sqlx::PgPool;
use uplift_db::crypto::Cipher;

/// Everything a job handler needs. Injected via Apalis Data<T> extractor.
/// All fields are cheap to clone — PgPool and Client are Arc-backed internally.
#[derive(Clone)]
pub struct JobContext {
    pub pool: PgPool,
    pub http: Client,
    pub cipher: Cipher,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub smtp: Option<SmtpConfig>,
}

/// Email configuration. Optional — if None, send_report jobs skip silently.
/// Set from environment variables at startup.
#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub from_address: String,
}

/// Create the Apalis internal tables. One call — setup is not per job type,
/// it creates a single shared jobs table. PostgresStorage::<()> is the
/// untyped handle used only for this setup call.
pub async fn setup_job_storage(pool: &PgPool) -> anyhow::Result<()> {
    match PostgresStorage::<()>::setup(pool).await {
        Ok(()) => Ok(()),
        Err(e) if e.to_string().contains("previously applied but is missing") => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}
/// Spin up all background workers. Blocks until the monitor shuts down.
/// In main.rs, spawn this as a separate tokio task alongside the Axum server.
pub async fn start_workers(ctx: JobContext) -> anyhow::Result<()> {
    let analysis_storage: PostgresStorage<run_analysis::RunAnalysisJob> =
        PostgresStorage::new(ctx.pool.clone());

    let fetch_ts_storage: PostgresStorage<fetch_timeseries::FetchTimeSeriesJob> =
        PostgresStorage::new(ctx.pool.clone());

    let refresh_tokens_storage: PostgresStorage<refresh_tokens::RefreshTokensJob> =
        PostgresStorage::new(ctx.pool.clone());

    // .data() must come before .backend() — the WorkerBuilder uses a typestate
    // pattern and .data() is only available before the backend is set.
    Monitor::new()
        .register(
            WorkerBuilder::new("uplift-run-analysis")
                .data(ctx.clone())
                .backend(analysis_storage)
                .build_fn(run_analysis::handle),
        )
        .register(
            WorkerBuilder::new("uplift-fetch-timeseries")
                .data(ctx.clone())
                .backend(fetch_ts_storage)
                .build_fn(fetch_timeseries::handle),
        )
        .register(
            WorkerBuilder::new("uplift-refresh-tokens")
                .data(ctx.clone())
                .backend(refresh_tokens_storage)
                .build_fn(refresh_tokens::handle),
        )
        .run()
        .await?;

    Ok(())
}

/// Enqueue a RunAnalysisJob from the API layer.
/// Kept here sp uplift_api doesn't need to import apalis-sql directly.
pub async fn enqueue_run_analysis(
    pool: &PgPool,
    job: run_analysis::RunAnalysisJob,
) -> anyhow::Result<()> {
    // use apalis::prelude::Backend;

    let mut storage: PostgresStorage<run_analysis::RunAnalysisJob> =
        PostgresStorage::new(pool.clone());

    storage
        .push(job)
        .await
        .map_err(|e| anyhow::anyhow!("failed to enqueue job: {e}"))?;

    Ok(())
}
