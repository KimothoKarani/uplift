pub mod analyses;
pub mod auth;
pub mod health;
pub mod properties;
pub mod stripe_webhooks;

use axum::{routing::get, Router};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        // Health check - no auth needed. Fly.io polls this
        .route("/health", get(health::handle))
        // Google OAuth - public, no auth middleware
        .nest("/auth", auth::router())
        // REST API - will have auth middleware added when we rite middleware/auth.rs
        .nest("/api", api_router())
        // Stripe webhooks - public but signature-verified inside the handler
        .nest("/stripe", stripe_webhooks::router())
        .with_state(state)
}

/// Protected API routes, nested under /api
fn api_router() -> Router<AppState> {
    Router::new()
        .nest("/properties", properties::router())
        .nest("/analyses", analyses::router())
}