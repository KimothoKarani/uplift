use sqlx::PgPool;
use uplift_db::crypto::Cipher;
use uplift_jobs::SmtpConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cipher: Cipher,
    pub http: reqwest::Client,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub app_base_url: String,
}

pub struct AppConfig {
    pub database_url: String,
    pub encryption_key: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub app_base_url: String,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        AppConfig {
            database_url: require_env("DATABASE_URL"),
            encryption_key: require_env("ENCRYPTION_KEY"),
            google_client_id: require_env("GOOGLE_CLIENT_ID"),
            google_client_secret: require_env("GOOGLE_CLIENT_SECRET"),
            google_redirect_uri: require_env("GOOGLE_REDIRECT_URI"),
            stripe_secret_key: require_env("STRIPE_SECRET_KEY"),
            stripe_webhook_secret: require_env("STRIPE_WEBHOOK_SECRET"),
            app_base_url: require_env("APP_BASE_URL"),
            smtp_host: std::env::var("SMTP_HOST").ok(),
            smtp_port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            smtp_username: std::env::var("SMTP_USERNAME").ok(),
            smtp_password: std::env::var("SMTP_PASSWORD").ok(),
            smtp_from: std::env::var("SMTP_FROM").ok(),
        }
    }

    pub fn smtp_config(&self) -> Option<SmtpConfig> {
        match (&self.smtp_host, &self.smtp_username, &self.smtp_password, &self.smtp_from) {
            (Some(host), Some(user), Some(pass), Some(from)) => Some(SmtpConfig {
                host: host.clone(),
                port: self.smtp_port.unwrap_or(587),
                username: user.clone(),
                password: pass.clone(),
                from_address: from.clone(),
            }),
            _ => None,
        }
    }
}

fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("required env var {key} is not set"))
}
