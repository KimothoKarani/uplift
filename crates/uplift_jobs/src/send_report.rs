use apalis::prelude::{BoxDynError, Data};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uplift_db::{AnalysisRepo, UserRepo};

use crate::{Error, JobContext, Result};

/// Send an email to the user who requested the analysis when it completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendReportJob {
    pub analysis_id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
}

pub async fn handle(
    job: SendReportJob,
    ctx: Data<JobContext>,
) -> std::result::Result<(), BoxDynError> {
    send(job, &ctx).await.map_err(Into::into)
}

async fn send(job: SendReportJob, ctx: &JobContext) -> Result<()> {
    // If SMTP is not configured, skip silently — this is expected in development.
    let smtp = match &ctx.smtp {
        Some(s) => s,
        None => {
            tracing::info!(analysis_id = %job.analysis_id, "SMTP not configured — skipping email");
            return Ok(());
        }
    };

    // Load the user to get their email address
    let user = UserRepo::find_by_id(&ctx.pool, job.user_id)
        .await
        .map_err(|_| Error::NotFound)?;

    // Load the analysis for the description and metric
    let analysis = AnalysisRepo::find_by_id(&ctx.pool, job.analysis_id, job.org_id)
        .await
        .map_err(|_| Error::NotFound)?;

    // Load the computed result for the numbers and narrative
    let result = AnalysisRepo::get_result(&ctx.pool, job.analysis_id)
        .await
        .map_err(|_| Error::NotFound)?;

    let subject = format!(
        "Analysis ready: {}",
        analysis.description
    );

    let relative_pct = result.relative_effect * 100.0;
    let direction = if relative_pct >= 0.0 { "+" } else { "" };
    let probability_pct = (result.probability_of_effect * 100.0).round() as u32;

    let body = format!(
        "Your analysis is ready.\n\n\
        Event: {description}\n\
        Metric: {metric}\n\
        Intervention date: {date}\n\n\
        Result: {direction}{relative_pct:.1}% change in {metric}\n\
        Confidence: {probability_pct}% probability this effect is real\n\n\
        {narrative}\n\n\
        Log in to view the full chart and export your client report.",
        description    = analysis.description,
        metric         = analysis.metric,
        date           = analysis.intervention_date,
        direction      = direction,
        relative_pct   = relative_pct,
        probability_pct = probability_pct,
        narrative      = result.narrative,
    );

    let email = Message::builder()
        .from(
            smtp.from_address
                .parse()
                .map_err(|e: lettre::address::AddressError| {
                    Error::Email(e.to_string())
                })?,
        )
        .to(user
            .email
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                Error::Email(e.to_string())
            })?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| Error::Email(e.to_string()))?;

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
        .map_err(|e| Error::Email(e.to_string()))?
        .credentials(Credentials::new(smtp.username.clone(), smtp.password.clone()))
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| Error::Email(e.to_string()))?;

    tracing::info!(
        analysis_id = %job.analysis_id,
        to          = %user.email,
        "analysis report email sent"
    );

    Ok(())
}