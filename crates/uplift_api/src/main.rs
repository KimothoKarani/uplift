mod error;
mod state;
mod middleware;
mod routes;

use anyhow::Context;
use state::{AppConfig, AppState};
use uplift_db::crypto::Cipher;
use uplift_jobs::JobContext;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env - ok() means we don't crash in production where env vars
    // come from the platform (Fly.io secrets) rather than a file
    dotenvy::dotenv().ok();

    // Structured logging - RUST_LOG env var controls the filter
    // e.g. RUST_LOG=uplift_api=debug, uplift_jobs=info, sqlx=warn
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("uplift starting");

    let cfg = AppConfig::from_env();

    // Connect to PostgreSQL and run our SQLx migrations
    let pool = uplift_db::connect(&cfg.database_url)
        .await
        .context("failed to connect to database")?;

    uplift_db::run_migrations(&pool)
        .await
        .context("failed to run database migrations")?;

    tracing::info!("database ready");

    // Create Apalis's internal job tables - separate from our migrations.
    // Must run after our migrations, before starting workers.
    uplift_jobs::setup_job_storage(&pool)
        .await
        .context("failed to set up job storage")?;

    tracing::info!("job storage ready");

    // Build the cipher from the base64-encoded encryption key.
    // All OAuth tokens are encryped at rest using this.
    let cipher = Cipher::from_base64_key(&cfg.encryption_key)
        .context("invalid ENCRYPTION_KEY")?;

    let http = reqwest::Client::new();

    // Extract SMTP config before cfg is consumed by AppState::new
    let smtp = cfg.smtp_config();

    // Build JobContext for the Apalis workers
    let job_ctx = JobContext {
        pool: pool.clone(),
        http: http.clone(),
        cipher: cipher.clone(),
        google_client_id: cfg.google_client_id.clone(),
        google_client_secret: cfg.google_client_secret.clone(),
        google_redirect_uri: cfg.google_redirect_uri.clone(),
        smtp,
    };

    // Build AppState for Axum - cfg is consumed here
    let state = AppState::new(
        pool.clone(),
        cipher,
        http,
        cfg);
    
    // Build the Axum router with all routes attached
    let router = routes::router(state);

    // Fly.io sets PORT - defaukt to 3000 fro local dev
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .context("failed to bind TCP listener")?;

    tracing::info!(port, "server listening");

    // Run the Axum server and Apalis workers concurrently.
    // tokio::select! means: if either crashes, the whole process exits.
    // This is correct - a crashed worker is not a degraded state we
    // want to limp along in.
    tokio::select! {
        res = axum::serve(listener, router) => {
            res.context("axum server error")?;
        }
        res = uplift_jobs::start_workers(job_ctx) => {
            res.context("job worker error")?;
        }
    }

    Ok(())

}
