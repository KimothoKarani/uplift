pub mod analyses;
pub mod auth;
pub mod health;
pub mod properties;
pub mod stripe_webhooks;

use axum::{Router, middleware, routing::get};

use crate::{middleware as mw, state::AppState};

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .nest("/properties", properties::router())
        .nest("/connections", properties::connections_router())
        .nest("/analyses", analyses::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            mw::auth::authenticate,
        ));

    Router::new()
        .route("/health", get(health::handle))
        .nest("/auth", auth::router())
        .nest("/api", api)
        .nest("/stripe", stripe_webhooks::router())
        .layer(middleware::from_fn(mw::logging::log_request))
        .with_state(state)
}
