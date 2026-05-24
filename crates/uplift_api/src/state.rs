use reqwest::Client;
use sqlx::PgPool;
use uplift_db::crypto::Cipher;
use uplift_jobs::SmtpConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cipher: Cipher,
    pub http: Client,

    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,

    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,

    pub app_base_url: String,
}

impl AppState {
    pub fn new(pool: PgPool, cipher: Cipher, http: Client, cfg: AppConfig) -> Self {
        Self {
            pool,
            cipher,
            http,
            google_client_id: cfg.google_client_id,
            google_client_secret: cfg.google_client_secret,
            google_redirect_uri: cfg.google_redirect_uri,
            stripe_secret_key: cfg.stripe_secret_key,
            stripe_webhook_secret: cfg.stripe_webhook_secret,
            app_base_url: cfg.app_base_url,
        }
    }
}

pub struct AppConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub app_base_url: String,
    pub encryption_key: String,
    pub database_url: String,
    pub smtp_host: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: require_env("DATABASE_URL"),
            encryption_key: require_env("ENCRYPTION_KEY"),
            google_client_id: require_env("GOOGLE_CLIENT_ID"),
            google_client_secret: require_env("GOOGLE_CLIENT_SECRET"),
            google_redirect_uri: require_env("GOOGLE_REDIRECT_URI"),
            stripe_secret_key: require_env("STRIPE_SECRET_KEY"),
            stripe_webhook_secret: require_env("STRIPE_WEBHOOK_SECRET"),
            app_base_url: require_env("APP_BASE_URL"),
            smtp_host: std::env::var("SMTP_HOST").ok(),
            smtp_username: std::env::var("SMTP_USERNAME").ok(),
            smtp_password: std::env::var("SMTP_PASSWORD").ok(),
            smtp_from: std::env::var("SMTP_FROM").ok(),
        }
    }

    pub fn smtp_config(&self) -> Option<SmtpConfig> {
        Some(SmtpConfig {
            host: self.smtp_host.clone()?,
            username: self.smtp_username.clone()?,
            password: self.smtp_password.clone()?,
            from_address: self.smtp_from.clone()?,
        })
    }
}

fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set"))
}