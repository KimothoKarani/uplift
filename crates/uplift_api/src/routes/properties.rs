use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use serde::Deserialize;
use uuid::Uuid;

use uplift_db::{Connection, ConnectionRepo, Property, PropertyRepo};

use crate::{error::AppError, middleware::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_properties).post(connect_property))
        .route("/:id", delete(remove_property))
}

/// Separate router for /api/connections - mounted by routes/mod.rs
pub fn connections_router() -> Router<AppState> {
    Router::new().route("/", get(list_connections))
}

// ------ Properties -------
async fn list_properties(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<Property>>, AppError> {
    let properties = PropertyRepo::list_by_org(&state.pool, auth_user.0.organization_id).await?;
    Ok(Json(properties))
}

#[derive(Deserialize)]
struct ConnectPropertyRequest {
    /// The google_connections UUID - which Google account owns this property
    connection_id: Uuid,
    /// The numeric GA4 property ID e.g. "123456789"
    ga4_property_id: String,
    pub display_name: String,
    pub website_url: Option<String>,
}

async fn connect_property(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ConnectPropertyRequest>,
) -> Result<(StatusCode, Json<Property>), AppError> {
    let org_id = auth_user.0.organization_id;

    // Verify the connection belongs to this org before using it
    ConnectionRepo::find_by_id(&state.pool, &state.cipher, req.connection_id)
        .await
        .map_err(|_| AppError::NotFound)?;

    let property = PropertyRepo::create(
        &state.pool,
        org_id,
        req.connection_id,
        &req.ga4_property_id,
        &req.display_name,
        req.website_url.as_deref(),
    )
    .await?;

    Ok((StatusCode::CREATED, Json(property)))
}

async fn remove_property(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    PropertyRepo::delete(&state.pool, id, auth_user.0.organization_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Connections ────────────────────────────────────────────────────────────

/// Returns Google connections with tokens stripped — only metadata the UI needs.
/// We never expose decrypted tokens to the frontend.
#[derive(serde::Serialize)]
pub struct ConnectionView {
    pub id: Uuid,
    pub google_account_email: String,
}

impl From<Connection> for ConnectionView {
    fn from(c: Connection) -> Self {
        Self {
            id: c.id,
            google_account_email: c.google_account_email,
        }
    }
}

async fn list_connections(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<ConnectionView>>, AppError> {
    let connections =
        ConnectionRepo::list_by_org(&state.pool, &state.cipher, auth_user.0.organization_id)
            .await?;

    Ok(Json(
        connections.into_iter().map(ConnectionView::from).collect(),
    ))
}
