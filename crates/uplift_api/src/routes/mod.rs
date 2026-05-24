pub mod analyses;
pub mod auth;
pub mod health;
pub mod properties;
pub mod stripe_webhooks;

use axum::{middleware, routing::get, Router};
use leptos::prelude::provide_context;
use leptos_axum::{file_and_error_handler, generate_route_list, LeptosRoutes};
use leptos_config::LeptosOptions;
use sqlx::PgPool;
use uplift_web::{shell, App};

use crate::{middleware as mw, state::AppState};

pub fn router(state: AppState, leptos_options: LeptosOptions) -> Router {
    let pool: PgPool = state.pool.clone();
    let routes = generate_route_list(App);

    let api = Router::new()
        .nest("/properties", properties::router())
        .nest("/connections", properties::connections_router())
        .nest("/analyses", analyses::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            mw::auth::authenticate,
        ));

    // API router uses AppState — finalized with .with_state(state) → Router<()>
    let api_router: Router = Router::new()
        .route("/health", get(health::handle))
        .nest("/auth", auth::router())
        .nest("/api", api)
        .nest("/stripe", stripe_webhooks::router())
        .with_state(state);

    // Leptos router uses LeptosOptions directly as state (satisfies FromRef bound
    // via the blanket impl for any Clone type).
    let leptos_router: Router = Router::<LeptosOptions>::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                provide_context(pool.clone());
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(file_and_error_handler(shell))
        .with_state(leptos_options);

    api_router
        .merge(leptos_router)
        .layer(middleware::from_fn(mw::logging::log_request))
}
