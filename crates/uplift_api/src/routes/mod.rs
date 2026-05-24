pub mod analyses;
pub mod auth;
pub mod health;
pub mod properties;
pub mod stripe_webhooks;

use axum::{middleware, routing::get, Router};
use leptos::prelude::*;
use leptos_axum::{file_and_error_handler, generate_route_list, LeptosRoutes};
use leptos_config::LeptosOptions;
use uplift_web::{shell, App};

use crate::{middleware as mw, state::AppState};

// Returns Router<()> — both sub-routers have been finalized with their
// respective states and merged into a single stateless service.
pub fn router(state: AppState, leptos_options: LeptosOptions) -> Router {
    let pool = state.pool.clone();
    let routes = generate_route_list(App);

    let api = Router::new()
        .nest("/properties", properties::router())
        .nest("/connections", properties::connections_router())
        .nest("/analyses", analyses::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            mw::auth::authenticate,
        ));

    // API router: state is AppState
    let api_router: Router = Router::new()
        .route("/health", get(health::handle))
        .nest("/auth", auth::router())
        .nest("/api", api)
        .nest("/stripe", stripe_webhooks::router())
        .with_state(state);

    // Leptos SSR router: state is LeptosOptions.
    // LeptosOptions: FromRef<LeptosOptions> is satisfied by Axum's blanket
    // impl<S: Clone> FromRef<S> for S — no custom impl needed.
    let leptos_router: Router = Router::<LeptosOptions>::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || { provide_context(pool.clone()); },
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