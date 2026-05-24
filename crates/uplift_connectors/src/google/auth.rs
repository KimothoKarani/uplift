use chrono::{DateTime, Utc};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl, basic::BasicClient,
};
use reqwest::Client;
use serde::Deserialize;

use crate::error::{Error, Result};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub struct GoogleOAuth {
    oauth_client: BasicClient,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub account_email: Option<String>,
}

#[derive(Deserialize)]
struct RawTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

impl GoogleOAuth {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Result<Self> {
        let oauth_client = BasicClient::new(
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret.clone())),
            AuthUrl::new(AUTH_URL.to_string()).map_err(|e| Error::Auth(e.to_string()))?,
            Some(TokenUrl::new(TOKEN_URL.to_string()).map_err(|e| Error::Auth(e.to_string()))?),
        )
        .set_redirect_uri(
            RedirectUrl::new(redirect_uri.clone()).map_err(|e| Error::Auth(e.to_string()))?,
        );

        Ok(Self {
            oauth_client,
            client_id,
            client_secret,
            redirect_uri,
        })
    }

    /// Build the Google consent screen URL.
    /// Returns (url, csrf_state) - store csrf_state in the session and verify
    /// it matches the 'state' parameter Google sends back in the callback.
    pub fn authorization_url(&self) -> (String, String) {
        let (url, csrf_token) = self
            .oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/analytics.readonly".into(),
            ))
            .add_scope(Scope::new("openid".into()))
            .add_scope(Scope::new("email".into()))
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent")
            .url();

        (url.to_string(), csrf_token.secret().to_string())
    }

    /// Exchange the authorization code from Google's callback for tokens.
    pub async fn exchange_code(&self, code: String, http: &Client) -> Result<TokenSet> {
        let params = [
            ("code", code.as_str()),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ];

        let raw = self.post_token(http, &params).await?;

        Ok(TokenSet {
            access_token: raw.access_token,
            refresh_token: raw.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(raw.expires_in),
            account_email: None,
        })
    }

    /// Get a new access token using the stored refresh token.
    /// Call this when expires_at is within 5 minutes of now.
    pub async fn refresh(&self, refresh_token: &str, http: &Client) -> Result<TokenSet> {
        let params = [
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let raw = self.post_token(http, &params).await?;

        Ok(TokenSet {
            access_token: raw.access_token,
            refresh_token: Some(refresh_token.to_string()),
            expires_at: Utc::now() + chrono::Duration::seconds(raw.expires_in),
            account_email: None,
        })
    }

    async fn post_token(&self, http: &Client, params: &[(&str, &str)]) -> Result<RawTokenResponse> {
        let resp = http.post(TOKEN_URL).form(params).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(Error::Api { status, message });
        }

        resp.json::<RawTokenResponse>()
            .await
            .map_err(|e| Error::Parse(e.to_string()))
    }
}
