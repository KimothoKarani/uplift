use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uplift_connectors::google::auth;
use uuid::Uuid;

use uplift_db::{Analysis, AnalysisRepo, AnalysisResult, PropertyRepo};
use uplift_jobs::run_analysis::RunAnalysisJob;

use crate::{error::AppError, middleware::auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_analyses).post(create_analysis))
        .route("/:id", get(get_analysis))

}

// ── Create ────────────────────────────────────────────────────
#[derive(Deserialize)]
struct CreateAnalysisRequest {
    property_id: Uuid,
    /// e.g. "sessions", "activeUsers"
    metric: String,
    /// The date the campaign/change happened
    intervention_date: NaiveDate,
    /// Training window — must be at least 30 days
    pre_period_start: NaiveDate,
    pre_period_end: NaiveDate,
    /// Measurement window — what we're attributing the effect over
    post_period_start: NaiveDate,
    post_period_end: NaiveDate,
    /// Measurement window — what we're attributing the effect over
    description: String,
}   

async fn create_analysis(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateAnalysisRequest>,
) -> Result<(StatusCode, Json<Analysis>), AppError> {
    let user = &auth_user.0;
    let org_id = user.organization_id;

    // Validate the date windows before touching the DB
    validate_dates(&req)?;

    // Verify the property exists and belongs to this org
    PropertyRepo::find_by_id(&state.pool, req.property_id, org_id)
        .await
        .map_err(|_| AppError::NotFound)?;

    // Create the analysis row — starts at status = 'pending'
    let analysis = AnalysisRepo::create(
        &state.pool,
        org_id,
        req.property_id,
        &req.metric,
        req.intervention_date,
        req.pre_period_start,
        req.pre_period_end,
        req.post_period_start,
        req.post_period_end,
        &req.description,
        user.id,
    ).await?;

    // Enqueue the background job — the handler returns immediately
    uplift_jobs::enqueue_run_analysis(&state.pool, 
        RunAnalysisJob { analysis_id: analysis.id, org_id, },).await.map_err(|e| AppError::Internal(e))?;

    tracing::info!(
        analysis_id = %analysis.id,
        org_id      = %org_id,
        metric      = %req.metric,
        "analysis enqueued"
    );

    // 202 Accepted - the job is running, not complete yet
    Ok((StatusCode::ACCEPTED, Json(analysis)))

}

// ------ List -----------
async fn list_analyses(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<Analysis>>, AppError> {
    let analyses = 
        AnalysisRepo::list_by_org(&state.pool, auth_user.0.organization_id).await?;

    Ok(Json(analyses))
}

// ── Get ─────────────────────────

/// Combined response — analysis metadata plus result if the job has completed.
/// The frontend polls this until status == 'complete' and result is populated.
#[derive(Serialize)]
struct AnalysisDetail {
    #[serde(flatten)]
    analysis: Analysis,
    result: Option<AnalysisResult>,
}

async fn get_analysis(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<AnalysisDetail>, AppError> {
    let org_id = auth_user.0.organization_id;
    let analysis = AnalysisRepo::find_by_id(&state.pool, id, org_id).await?;

    // Result is None until the job completes — that's expected, not an error
    let result = AnalysisRepo::get_result(&state.pool, id).await.ok();

    Ok(Json(AnalysisDetail { analysis, result }))
}



fn validate_dates(req: &CreateAnalysisRequest) -> Result<(), AppError> {
    if req.pre_period_start >= req.pre_period_end {
        return Err(AppError::BadRequest("pre_period_start must be before pre_period_end".into()));
    }

    if req.pre_period_end >= req.intervention_date {
        return Err(AppError::BadRequest("pre_period_end must be before intervention_date".into()));

    }

    if req.post_period_start > req.post_period_end {
        return Err(AppError::BadRequest("post_period_start must be before or equal to post_period_end".into()));
    }

     // The causal model needs at least 30 pre-period data points to fit reliably
     let pre_days = (req.pre_period_end - req.pre_period_start).num_days();
     if pre_days < 30 {
        return Err(AppError::BadRequest(format!("pre_period must be at least 30 days - got {pre_days}")));
     }

     Ok(())
}