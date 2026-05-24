use axum::http::{header::COOKIE, HeaderMap};
use leptos::prelude::ServerFnError;
use sqlx::PgPool;
use uuid::Uuid;

/// Extract the authenticated user from the session cookie.
/// Calls `leptos_axum::redirect("/login")` before returning Err so that
/// SSR responses become 302 redirects automatically.
pub async fn require_user() -> Result<uplift_db::User, ServerFnError> {
    use leptos::context::use_context;
    use uplift_db::{SessionRepo, UserRepo};

    let headers: HeaderMap = leptos_axum::extract().await
        .map_err(|e| ServerFnError::new(format!("extract headers: {e}")))?;

    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let session_id = match extract_session_id(&headers) {
        Some(id) => id,
        None => {
            leptos_axum::redirect("/login");
            return Err(ServerFnError::new("not authenticated"));
        }
    };

    let session = match SessionRepo::find_valid(&pool, session_id).await {
        Ok(s) => s,
        Err(_) => {
            leptos_axum::redirect("/login");
            return Err(ServerFnError::new("session expired"));
        }
    };

    UserRepo::find_by_id(&pool, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(format!("user not found: {e}")))
}

fn extract_session_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get(COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(|p| p.trim())
                .find(|p| p.starts_with("uplift_session="))
                .and_then(|p| p["uplift_session=".len()..].parse::<Uuid>().ok())
        })
}
