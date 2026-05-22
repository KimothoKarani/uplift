use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use leptos::server_fn::request::Req;
use uuid::Uuid;

use uplift_db::{SessionRepo, User, UserRepo};
use crate::{error::AppError, state::AppState};

/// The authenticated user injected into request extensions by this middleware.
/// Extract in route handler with: Extension(auth_user):  Extension<AuthUser>

#[derive(Clone)]
pub struct AuthUser(pub User);

pub async fn authenticate(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let session_id = extract_session_id(request.headers())
        .ok_or(AppError::Unauthorized)?;

    // find_valid checks both existence and expiry in one query
    let session = SessionRepo::find_valid(&state.pool, session_id)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let user = UserRepo::find_by_id(&state.pool, session.user_id)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    request.extensions_mut().insert(AuthUser(user));
    
    Ok(next.run(request).await)
}


    fn extract_session_id(headers: &axum::http::HeaderMap) -> Option<Uuid> {
        headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                s.split(';')
                .map(|part| part.trim())
                .find(|part| part.starts_with("uplift_session="))
                .map(|part| part["uplift_session=".len()..].to_string())
            })
            .and_then(|id| id.parse::<Uuid>().ok())
}
