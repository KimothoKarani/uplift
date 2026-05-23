use axum::{
    extract::{Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use uplift_connectors::google::auth::GoogleOAuth;
use uplift_db::{ConnectionRepo, OrgRepo, SessionRepo, UserRepo};

use crate::{error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/google", get(google_login))
        .route("/callback", get(google_callback))
        .route("/logout", post(logout))
}

// ----- Login --------------------------
async fn google_login(State(state): State<AppState>) -> Result<Response, AppError> {
    let oauth = build_oauth(&state)?;
    let (url, csrf_token) = oauth.authorization_url();

    // Store the CSRF token in a short-lived cookie.
    // We verify it matches the 'state' param Google sends back in the callback.
    let csrf_cookie = format!(
        "uplift_oauth_state={csrf_token}; HttpOnly; SameSite=Lax; Path=/auth/callback; Max-Age=600"
    );

    Ok((
        [(header::SET_COOKIE, csrf_cookie)],
        Redirect::to(&url),
    )
        .into_response())
}

// ---- Callback -------------
#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String, // CSRF token Google echoes back
}

async fn google_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // Step 1 - validate CSRF token
    let stored_csrf = get_cookie(&headers, "uplift_oauth_state").ok_or_else(||{
        AppError::BadRequest("missing oauth state cookie - start login again".into())
    })?;

    if stored_csrf != params.state {
        return Err(AppError::BadRequest("oauth state mismatch".into()));
    }

    // Step 2 - exchange the authorization code for tokens
    let oauth = build_oauth(&state)?;
    let tokens = oauth
        .exchange_code(params.code, &state.http)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    // Step 3 - fetch the user's Google profile
    let profile = fetch_google_profile(&state.http, &tokens.access_token).await?;

    // Step 4 - find or create the org and user.
    // Returning users are found by google_id - their org stays the same.
    // New users get a fresh org created first.
    let user = match UserRepo::find_by_google_id(&state.pool, &profile.id).await {
        Ok(existing) => {
            // Returning user - update name/email in case they changed
            UserRepo::upsert_by_google_id(
                &state.pool,
                existing.organization_id,
                &profile.email,
                profile.name.as_deref(),
                &profile.id,
                "owner",
            )
            .await?
        }
        Err(uplift_db::Error::NotFound) => {
            // New user - create their org first
            let slug = org_slug_from_email(&profile.email);
            let org = OrgRepo::create(&state.pool, &profile.email, &slug).await?;
            UserRepo::upsert_by_google_id(
                &state.pool,
                org.id,
                &profile.email,
                profile.name.as_deref(),
                &profile.id,
            "owner",).await?
        }
        Err(e) => return Err(AppError::Internal(anyhow::anyhow!(e))),
    };

    // Step 5 - store the encrypted Oauth tokens for this Google account.
    // These are what the job workers use to call GA4 on the user's behalf.
    let refresh_token = tokens.refresh_token.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Google did not return a refresh token - ensure prompt=consent is set"
        ))
    })?;

    ConnectionRepo::upsert(
        &state.pool,
        &state.cipher,
        user.organization_id,
        &profile.email,
        &tokens.access_token,
        &refresh_token,
    tokens.expires_at
    )
    .await?;
    
    //Step 6 - create a session valid for 30 days
    let expires_at = Utc::now() + chrono::Duration::days(30);
    let session = SessionRepo::create(&state.pool, user.id, expires_at).await?;

    // Step 7 - set the session cookie and redirect to the dashboard
    let session_cookie = format!(
        "uplift_session={session_id}; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000",
        session_id = session.id
    );

    Ok((
        [(header::SET_COOKIE, session_cookie)],
        Redirect::to("/dashboard"),
    )
        .into_response())
}

// ----- Logout -------------------------
async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(session_id_str) = get_cookie(&headers, "uplift_session") {
        if let Ok(session_id) = session_id_str.parse::<Uuid>() {
            // Best-effort delete - if session doesn't exist, that's fine
            let _ = SessionRepo::delete(&state.pool, session_id).await;
        }
    }

    // Clear the cookie by setting MAx-Age=0
    let clear_cookie = "uplift_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";

    Ok((
        [(header::SET_COOKIE, clear_cookie)],
        Redirect::to("/"),
    )
        .into_response())
}



// ----- Helpers -----------------------------
fn build_oauth(state: &AppState) -> Result<GoogleOAuth, AppError> {
    GoogleOAuth::new(
        state.google_client_id.clone(),
        state.google_client_secret.clone(),
        state.google_redirect_uri.clone(),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

#[derive(Deserialize)]
struct GoogleProfile {
    id: String,
    email: String,
    name: Option<String>,
}

async fn fetch_google_profile(
    http: &reqwest::Client,
    access_token: &str,
) -> Result<GoogleProfile, AppError> {
    http.get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .json::<GoogleProfile>()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}


// Parse a specific cookie value from the Cookie header
fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(|part| part.trim())
                .find(|part| part.starts_with(&format!("{name}=")))
                .map(|part| part[name.len() + 1..].to_string())
        })
}


/// Derive a unique org slug from the user's email.
/// Appends a UUID fragment to avoid collisions.
fn org_slug_from_email(email: &str) -> String {
    let base = email
        .split('@')
        .next()
        .unwrap_or("org")
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() {c} else {'-'})
        .collect::<String>();

    format!("{base}-{}", &Uuid::new_v4().to_string()[..8])
}