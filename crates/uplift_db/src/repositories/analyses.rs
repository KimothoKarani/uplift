use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use uplift_core::impact::report::ImpactReport;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Analysis {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub property_id: Uuid,
    pub metric: String,
    pub intervention_date: NaiveDate,
    pub pre_period_start: NaiveDate,
    pub pre_period_end: NaiveDate,
    pub post_period_start: NaiveDate,
    pub post_period_end: NaiveDate,
    pub description: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalysisResult {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub model_version: String,
    pub cumulative_effect: f64,
    pub cumulative_effect_lower: f64,
    pub cumulative_effect_upper: f64,
    pub relative_effect: f64,
    pub relative_effect_lower: f64,
    pub relative_effect_upper: f64,
    pub probability_of_effect: f64,
    pub pointwise_effects: serde_json::Value,
    pub narrative: String,
    pub computed_at: DateTime<Utc>,
}

pub struct AnalysisRepo;

impl AnalysisRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: Uuid,
        property_id: Uuid,
        metric: &str,
        intervention_date: NaiveDate,
        pre_period_start: NaiveDate,
        pre_period_end: NaiveDate,
        post_period_start: NaiveDate,
        post_period_end: NaiveDate,
        description: &str,
        created_by: Uuid,
    ) -> Result<Analysis> {
        let a = sqlx::query_as!(
            Analysis,
            r#"
            INSERT INTO analyses
                (organization_id, property_id, metric, intervention_date,
                 pre_period_start, pre_period_end, post_period_start, post_period_end,
                 description, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
            org_id,
            property_id,
            metric,
            intervention_date,
            pre_period_start,
            pre_period_end,
            post_period_start,
            post_period_end,
            description,
            created_by,
        )
        .fetch_one(pool)
        .await?;
        Ok(a)
    }

    /// Move the analysis through its lifecycle: pending → running → complete/failed.
    pub async fn set_status(
        pool: &PgPool,
        id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE analyses
            SET status = $1, error_message = $2
            WHERE id = $3
            "#,
            status,
            error_message,
            id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<Analysis>> {
        Ok(sqlx::query_as!(
            Analysis,
            "SELECT * FROM analyses WHERE organization_id = $1 ORDER BY created_at DESC",
            org_id,
        )
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid, org_id: Uuid) -> Result<Analysis> {
        sqlx::query_as!(
            Analysis,
            "SELECT * FROM analyses WHERE id = $1 AND organization_id = $2",
            id,
            org_id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }

    /// Store the ImpactReport from uplift_core after the job completes.
    pub async fn save_result(
        pool: &PgPool,
        analysis_id: Uuid,
        report: &ImpactReport,
    ) -> Result<AnalysisResult> {
        let s = &report.summary;
        let r = sqlx::query_as!(
            AnalysisResult,
            r#"
            INSERT INTO analysis_results
                (analysis_id, model_version,
                 cumulative_effect,       cumulative_effect_lower, cumulative_effect_upper,
                 relative_effect,         relative_effect_lower,   relative_effect_upper,
                 probability_of_effect,   pointwise_effects,       narrative)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
            analysis_id,
            report.model_version.as_str(),
            s.cumulative_effect,
            s.cumulative_effect_lower,
            s.cumulative_effect_upper,
            s.relative_effect,
            s.relative_effect_lower,
            s.relative_effect_upper,
            s.probability_of_effect,
            serde_json::to_value(&report.pointwise)?,
            report.narrative,
        )
        .fetch_one(pool)
        .await?;
        Ok(r)
    }

    pub async fn get_result(pool: &PgPool, analysis_id: Uuid) -> Result<AnalysisResult> {
        sqlx::query_as!(
            AnalysisResult,
            "SELECT * FROM analysis_results WHERE analysis_id = $1",
            analysis_id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)
    }
}
