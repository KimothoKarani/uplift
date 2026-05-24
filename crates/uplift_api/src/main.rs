#![recursion_limit = "512"]

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
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("uplift starting");

    let cfg = AppConfig::from_env();

    let pool = uplift_db::connect(&cfg.database_url)
        .await
        .context("failed to connect to database")?;

    uplift_jobs::setup_job_storage(&pool)
        .await
        .context("failed to set up job storage")?;

    tracing::info!("job storage ready");

    uplift_db::run_migrations(&pool)
        .await
        .context("failed to run database migrations")?;

    tracing::info!("database ready");

    let cipher = Cipher::from_base64_key(&cfg.encryption_key)
        .context("invalid ENCRYPTION_KEY")?;

    let http = reqwest::Client::new();

    let smtp = cfg.smtp_config();

    let job_ctx = JobContext {
        pool: pool.clone(),
        http: http.clone(),
        cipher: cipher.clone(),
        google_client_id: cfg.google_client_id.clone(),
        google_client_secret: cfg.google_client_secret.clone(),
        google_redirect_uri: cfg.google_redirect_uri.clone(),
        smtp,
    };

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    // LeptosOptions tells the SSR runtime where compiled WASM/JS assets
    // live (site_root/site_pkg_dir) and the address to bind for
    // hot-reload in dev. site_addr mirrors the actual server port.
    let leptos_options = leptos_config::LeptosOptions {
        output_name: "uplift_web".into(),
        site_addr: std::net::SocketAddr::from(([0, 0, 0, 0], port)),
        ..Default::default()
    };

       // Build AppState — cfg is consumed here
    let state = AppState::new(pool.clone(), cipher, http, cfg);

    let router = routes::router(state, leptos_options);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .context("failed to bind TCP listener")?;

    tracing::info!(port, "server listening");

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