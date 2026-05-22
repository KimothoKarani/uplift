use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use chrono::{DateTime, Utc};
use stripe::{EventObject, EventType, Webhook};
use uuid::Uuid;

use uplift_db::{OrgRepo, SubscriptionRepo};

use crate::{error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/webhooks", post(handle_webhook))
}

async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("missing stripe-signature header".into()))?;

    let payload_str = std::str::from_utf8(&body)
        .map_err(|_| AppError::BadRequest("invalid utf-8 payload".into()))?;

    let event = Webhook::construct_event(payload_str, sig, &state.stripe_webhook_secret)
        .map_err(|_| AppError::BadRequest("invalid stripe signature".into()))?;

    tracing::info!(event_type = %event.type_, event_id = %event.id, "stripe webhook received");

    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            if let EventObject::CheckoutSession(session) = event.data.object {
                handle_checkout_completed(&state, session).await?;
            }
        }
        EventType::CustomerSubscriptionUpdated => {
            if let EventObject::Subscription(sub) = event.data.object {
                handle_subscription_updated(&state, sub).await?;
            }
        }
        EventType::CustomerSubscriptionDeleted => {
            if let EventObject::Subscription(sub) = event.data.object {
                handle_subscription_deleted(&state, sub).await?;
            }
        }
        EventType::InvoicePaymentFailed => {
            if let EventObject::Invoice(invoice) = event.data.object {
                handle_payment_failed(&state, invoice).await?;
            }
        }
        _ => {
            tracing::debug!(event_type = %event.type_, "ignoring unhandled event type");
        }
    }

    // Always 200 — Stripe retries on non-2xx, we don't want infinite retries
    // for events we've already processed or chosen to ignore.
    Ok(StatusCode::OK)
}

async fn handle_checkout_completed(
    state: &AppState,
    session: Box<stripe::CheckoutSession>,
) -> Result<(), AppError> {
    let org_id_str = session
        .metadata
        .as_ref()
        .and_then(|m| m.get("org_id"))
        .ok_or_else(|| {
            tracing::error!("checkout.session.completed missing org_id metadata");
            AppError::Internal(anyhow::anyhow!("missing org_id in session metadata"))
        })?;

    let org_id = org_id_str
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid org_id in metadata")))?;

    if let Some(customer) = &session.customer {
        OrgRepo::set_stripe_customer(&state.pool, org_id, &customer.id().to_string()).await?;
    }

    let stripe_sub_id = match &session.subscription {
        Some(sub) => sub.id().to_string(),
        None => {
            tracing::warn!(org_id = %org_id, "checkout completed but no subscription attached");
            return Ok(());
        }
    };

    // Line items are not expanded on checkout.session.completed so we cannot
    // read the price ID here. Use a 30-day placeholder — the
    // customer.subscription.updated event fires right after and overwrites
    // with the real period end and correct tier.
    let period_end = Utc::now() + chrono::Duration::days(30);

    SubscriptionRepo::upsert(&state.pool, org_id, &stripe_sub_id, "starter", "active", period_end)
        .await?;

    tracing::info!(org_id = %org_id, "subscription activated via checkout");
    Ok(())
}

async fn handle_subscription_updated(
    state: &AppState,
    sub: Box<stripe::Subscription>,
) -> Result<(), AppError> {
    let stripe_sub_id = sub.id.to_string();

    let existing = SubscriptionRepo::find_by_stripe_id(&state.pool, &stripe_sub_id)
        .await
        .map_err(|_| {
            tracing::warn!(stripe_sub_id = %stripe_sub_id, "subscription.updated for unknown sub");
            AppError::NotFound
        })?;

    let status = subscription_status_str(&sub.status);
    let tier = tier_from_items(&sub);
    let period_end = DateTime::from_timestamp(sub.current_period_end, 0)
        .unwrap_or_else(Utc::now);

    SubscriptionRepo::upsert(
        &state.pool,
        existing.organization_id,
        &stripe_sub_id,
        tier,
        status,
        period_end,
    )
    .await?;

    tracing::info!(
        org_id = %existing.organization_id,
        status = %status,
        tier   = %tier,
        "subscription updated"
    );
    Ok(())
}

async fn handle_subscription_deleted(
    state: &AppState,
    sub: Box<stripe::Subscription>,
) -> Result<(), AppError> {
    let stripe_sub_id = sub.id.to_string();

    let existing = SubscriptionRepo::find_by_stripe_id(&state.pool, &stripe_sub_id)
        .await
        .map_err(|_| AppError::NotFound)?;

    let period_end = DateTime::from_timestamp(sub.current_period_end, 0)
        .unwrap_or_else(Utc::now);

    SubscriptionRepo::upsert(
        &state.pool,
        existing.organization_id,
        &stripe_sub_id,
        &existing.tier,
        "canceled",
        period_end,
    )
    .await?;

    tracing::info!(org_id = %existing.organization_id, "subscription canceled");
    Ok(())
}

async fn handle_payment_failed(
    state: &AppState,
    invoice: Box<stripe::Invoice>,
) -> Result<(), AppError> {
    let stripe_sub_id = match &invoice.subscription {
        Some(sub) => sub.id().to_string(),
        None => return Ok(()),
    };

    let existing = SubscriptionRepo::find_by_stripe_id(&state.pool, &stripe_sub_id)
        .await
        .map_err(|_| AppError::NotFound)?;

    SubscriptionRepo::upsert(
        &state.pool,
        existing.organization_id,
        &stripe_sub_id,
        &existing.tier,
        "past_due",
        existing.current_period_end,
    )
    .await?;

    tracing::info!(org_id = %existing.organization_id, "subscription payment failed — marked past_due");
    Ok(())
}

fn tier_from_price_id(price_id: &str) -> &'static str {
    let starter    = std::env::var("STRIPE_STARTER_PRICE_ID").unwrap_or_default();
    let agency     = std::env::var("STRIPE_AGENCY_PRICE_ID").unwrap_or_default();
    let agency_pro = std::env::var("STRIPE_AGENCY_PRO_PRICE_ID").unwrap_or_default();

    if !agency_pro.is_empty() && price_id == agency_pro {
        "agency_pro"
    } else if !agency.is_empty() && price_id == agency {
        "agency"
    } else if !starter.is_empty() && price_id == starter {
        "starter"
    } else {
        "starter"
    }
}

fn tier_from_items(sub: &stripe::Subscription) -> &'static str {
    sub.items
        .data
        .first()
        .and_then(|item| item.price.as_ref())
        .map(|price| price.id.as_str())
        .map(tier_from_price_id)
        .unwrap_or("starter")
}

fn subscription_status_str(status: &stripe::SubscriptionStatus) -> &'static str {
    match status {
        stripe::SubscriptionStatus::Active           => "active",
        stripe::SubscriptionStatus::PastDue          => "past_due",
        stripe::SubscriptionStatus::Canceled         => "canceled",
        stripe::SubscriptionStatus::Trialing         => "trialing",
        stripe::SubscriptionStatus::Incomplete       => "incomplete",
        stripe::SubscriptionStatus::IncompleteExpired => "incomplete_expired",
        stripe::SubscriptionStatus::Unpaid           => "unpaid",
        _                                            => "active",
    }
}
