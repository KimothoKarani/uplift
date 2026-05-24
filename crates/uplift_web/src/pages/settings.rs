use leptos::prelude::*;
use leptos_meta::Title;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::nav::AppLayout;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub org_name: String,
    pub org_slug: String,
    pub user_name: String,
    pub user_email: String,
    pub user_role: String,
    pub subscription: Option<SubInfo>,
    pub properties: Vec<PropInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubInfo {
    pub tier: String,
    pub status: String,
    pub period_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropInfo {
    pub id: String,
    pub display_name: String,
    pub ga4_property_id: String,
}

// ── Server functions ──────────────────────────────────────────────────────────

#[server]
pub async fn load_settings() -> Result<SettingsData, ServerFnError> {
    use axum::http::HeaderMap;
    use leptos_axum::extract;
    use sqlx::PgPool;
    use uplift_db::repositories::{
        organizations::OrgRepo,
        properties::PropertyRepo,
        sessions::SessionRepo,
        subscriptions::SubscriptionRepo,
        users::UserRepo,
    };

    let headers: HeaderMap = extract().await?;
    let pool = expect_context::<PgPool>();

    let session_id = crate::server_utils::extract_session_id(&headers)
        .ok_or_else(|| {
            leptos_axum::redirect("/login");
            ServerFnError::new("unauthenticated")
        })?;

    let session = SessionRepo::find_valid(&pool, session_id).await.map_err(|_| {
        leptos_axum::redirect("/login");
        ServerFnError::new("unauthenticated")
    })?;

    let user = UserRepo::find_by_id(&pool, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (org, subscription, properties) = tokio::try_join!(
        OrgRepo::find_by_id(&pool, user.organization_id),
        SubscriptionRepo::find_by_org(&pool, user.organization_id),
        PropertyRepo::list_by_org(&pool, user.organization_id),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let user_name = user
        .display_name
        .unwrap_or_else(|| user.email.split('@').next().unwrap_or("User").to_string());

    let sub_info = subscription.map(|s| SubInfo {
        tier: s.tier,
        status: s.status,
        period_end: s.current_period_end.format("%B %d, %Y").to_string(),
    });

    let props = properties
        .into_iter()
        .map(|p| PropInfo {
            id: p.id.to_string(),
            display_name: p.display_name,
            ga4_property_id: p.ga4_property_id,
        })
        .collect();

    Ok(SettingsData {
        org_name: org.name,
        org_slug: org.slug,
        user_name,
        user_email: user.email,
        user_role: user.role,
        subscription: sub_info,
        properties: props,
    })
}

#[server]
pub async fn delete_property(property_id: String) -> Result<(), ServerFnError> {
    use axum::http::HeaderMap;
    use leptos_axum::extract;
    use sqlx::PgPool;
    use uplift_db::repositories::{
        properties::PropertyRepo,
        sessions::SessionRepo,
        users::UserRepo,
    };

    let headers: HeaderMap = extract().await?;
    let pool = expect_context::<PgPool>();

    let session_id = crate::server_utils::extract_session_id(&headers)
        .ok_or_else(|| {
            leptos_axum::redirect("/login");
            ServerFnError::new("unauthenticated")
        })?;

    let session = SessionRepo::find_valid(&pool, session_id).await.map_err(|_| {
        leptos_axum::redirect("/login");
        ServerFnError::new("unauthenticated")
    })?;

    let user = UserRepo::find_by_id(&pool, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let prop_id = Uuid::parse_str(&property_id)
        .map_err(|_| ServerFnError::new("invalid property id"))?;

    PropertyRepo::delete(&pool, prop_id, user.organization_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

// ── Page component ────────────────────────────────────────────────────────────

#[component]
pub fn SettingsPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| load_settings());

    view! {
        <Title text="Settings — Uplift"/>
        <Suspense fallback=|| view! { <SettingsSkeleton/> }>
            {move || data.get().map(|res| match res {
                Ok(d) => view! { <SettingsContent data=d/> }.into_any(),
                Err(_) => view! {
                    <div class="min-h-screen bg-gray-50 flex items-center justify-center">
                        <p class="text-sm text-gray-500">"Failed to load settings."</p>
                    </div>
                }.into_any(),
            })}
        </Suspense>
    }
}

// ── Main content ──────────────────────────────────────────────────────────────

#[component]
fn SettingsContent(data: SettingsData) -> impl IntoView {
    view! {
        <AppLayout
            org_name=data.org_name.clone()
            user_name=data.user_name.clone()
            user_email=data.user_email.clone()
        >
            <div class="py-10 space-y-8">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900">"Settings"</h1>
                    <p class="mt-1 text-sm text-gray-500">
                        "Manage your account, billing, and connected properties."
                    </p>
                </div>

                <AccountSection
                    org_name=data.org_name.clone()
                    org_slug=data.org_slug.clone()
                    user_name=data.user_name.clone()
                    user_email=data.user_email.clone()
                    user_role=data.user_role.clone()
                />
                <BillingSection subscription=data.subscription/>
                <PropertiesSection properties=data.properties/>
                <SessionSection/>
            </div>
        </AppLayout>
    }
}

// ── Account section ───────────────────────────────────────────────────────────

#[component]
fn AccountSection(
    org_name: String,
    org_slug: String,
    user_name: String,
    user_email: String,
    user_role: String,
) -> impl IntoView {
    let initials = user_name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    let role_label = match user_role.as_str() {
        "owner" => "Owner",
        "admin" => "Admin",
        _ => "Member",
    }
    .to_string();

    view! {
        <SectionCard title="Account">
            <div class="flex items-start gap-6">
                <div class="flex-shrink-0 w-14 h-14 rounded-full bg-brand-100 flex items-center justify-center">
                    <span class="text-lg font-semibold text-brand-700">{initials}</span>
                </div>
                <dl class="flex-1 grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-5">
                    <Field label="Name" value=user_name/>
                    <Field label="Email" value=user_email/>
                    <Field label="Role" value=role_label/>
                    <Field label="Organization" value=org_name/>
                    <Field label="Workspace slug" value=org_slug/>
                </dl>
            </div>
        </SectionCard>
    }
}

// ── Billing section ───────────────────────────────────────────────────────────

#[component]
fn BillingSection(subscription: Option<SubInfo>) -> impl IntoView {
    view! {
        <SectionCard title="Billing">
            {match subscription {
                None => view! {
                    <div class="flex items-center justify-between gap-4">
                        <div>
                            <p class="text-sm font-medium text-gray-900">"No active subscription"</p>
                            <p class="text-sm text-gray-500 mt-0.5">
                                "Upgrade to run analyses and unlock all features."
                            </p>
                        </div>
                        <a
                            href="/api/billing/checkout"
                            class="flex-shrink-0 inline-flex items-center gap-1.5 px-4 py-2 bg-brand-600 text-white text-sm font-medium rounded-lg hover:bg-brand-700 transition-colors"
                        >
                            "Upgrade plan"
                            <svg class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor">
                                <path fill-rule="evenodd" d="M5 10a.75.75 0 01.75-.75h6.638L10.23 7.29a.75.75 0 111.04-1.08l3.5 3.25a.75.75 0 010 1.08l-3.5 3.25a.75.75 0 11-1.04-1.08l2.158-1.96H5.75A.75.75 0 015 10z" clip-rule="evenodd"/>
                            </svg>
                        </a>
                    </div>
                }.into_any(),
                Some(sub) => {
                    let (status_bg, status_text) = match sub.status.as_str() {
                        "active"   => ("bg-emerald-50 text-emerald-700 ring-emerald-600/20", "Active"),
                        "trialing" => ("bg-brand-50 text-brand-700 ring-brand-600/20",       "Trial"),
                        "past_due" => ("bg-amber-50 text-amber-700 ring-amber-600/20",       "Past due"),
                        "canceled" => ("bg-red-50 text-red-700 ring-red-600/20",             "Canceled"),
                        _          => ("bg-gray-50 text-gray-600 ring-gray-600/20",          "Unknown"),
                    };
                    let tier_label = match sub.tier.as_str() {
                        "pro"        => "Pro",
                        "growth"     => "Growth",
                        "enterprise" => "Enterprise",
                        _            => "Starter",
                    };
                    view! {
                        <div class="flex items-start justify-between gap-6">
                            <div class="space-y-3">
                                <div class="flex items-center gap-3">
                                    <span class="text-sm font-semibold text-gray-900">
                                        {tier_label}" plan"
                                    </span>
                                    <span class=format!(
                                        "inline-flex items-center rounded-md px-2 py-0.5 \
                                         text-xs font-medium ring-1 ring-inset {status_bg}"
                                    )>
                                        {status_text}
                                    </span>
                                </div>
                                <p class="text-sm text-gray-500">
                                    "Renews on "
                                    <span class="font-medium text-gray-700">{sub.period_end}</span>
                                </p>
                            </div>
                            <a
                                href="/api/billing/portal"
                                class="flex-shrink-0 inline-flex items-center gap-1.5 px-4 py-2 \
                                       text-sm font-medium text-gray-700 bg-white border border-gray-300 \
                                       rounded-lg hover:bg-gray-50 transition-colors"
                            >
                                "Manage billing"
                                <svg class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor">
                                    <path fill-rule="evenodd" d="M5 10a.75.75 0 01.75-.75h6.638L10.23 7.29a.75.75 0 111.04-1.08l3.5 3.25a.75.75 0 010 1.08l-3.5 3.25a.75.75 0 11-1.04-1.08l2.158-1.96H5.75A.75.75 0 015 10z" clip-rule="evenodd"/>
                                </svg>
                            </a>
                        </div>
                    }.into_any()
                }
            }}
        </SectionCard>
    }
}

// ── Properties section ────────────────────────────────────────────────────────

#[component]
fn PropertiesSection(properties: Vec<PropInfo>) -> impl IntoView {
    let delete_action = ServerAction::<DeleteProperty>::new();

    view! {
        <SectionCard title="GA4 Properties">
            {if properties.is_empty() {
                view! {
                    <div class="rounded-lg border border-dashed border-gray-300 p-10 text-center">
                        <svg class="mx-auto w-8 h-8 text-gray-300" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round"
                                d="M3.75 3v11.25A2.25 2.25 0 006 16.5h12M3.75 3h-1.5m1.5 0h16.5m0 0h1.5\
                                   m-1.5 0v11.25A2.25 2.25 0 0118 16.5h-12m12 0v3.75m-12-3.75v3.75m0 0H6m6 0h6"/>
                        </svg>
                        <p class="mt-3 text-sm font-medium text-gray-900">
                            "No properties connected"
                        </p>
                        <p class="mt-1 text-xs text-gray-500">
                            "Connect a Google Analytics 4 property to start analysing."
                        </p>
                        <a
                            href="/api/auth/google"
                            class="mt-5 inline-flex items-center gap-2 px-4 py-2 bg-brand-600 \
                                   text-white text-sm font-medium rounded-lg hover:bg-brand-700 transition-colors"
                        >
                            "Connect Google Analytics"
                        </a>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="space-y-4">
                        <div class="divide-y divide-gray-100 rounded-lg border border-gray-200 overflow-hidden">
                            <For
                                each=move || properties.clone()
                                key=|p| p.id.clone()
                                children=move |prop| view! {
                                    <PropertyRow prop=prop action=delete_action/>
                                }
                            />
                        </div>
                        <a
                            href="/api/auth/google"
                            class="inline-flex items-center gap-1.5 text-sm font-medium \
                                   text-brand-600 hover:text-brand-700 transition-colors"
                        >
                            <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor">
                                <path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z"/>
                            </svg>
                            "Add another property"
                        </a>
                    </div>
                }.into_any()
            }}
        </SectionCard>
    }
}

#[component]
fn PropertyRow(prop: PropInfo, action: ServerAction<DeleteProperty>) -> impl IntoView {
    let prop_id = prop.id.clone();
    view! {
        <div class="flex items-center justify-between px-4 py-3 bg-white hover:bg-gray-50 transition-colors">
            <div class="flex items-center gap-3 min-w-0">
                <div class="flex-shrink-0 w-8 h-8 rounded-lg bg-brand-50 flex items-center justify-center">
                    <svg class="w-4 h-4 text-brand-600" viewBox="0 0 20 20" fill="currentColor">
                        <path fill-rule="evenodd"
                            d="M3 3a1 1 0 000 2v8a2 2 0 002 2h2.586l-1.293 1.293a1 1 0 101.414 1.414L10 \
                               15.414l2.293 2.293a1 1 0 001.414-1.414L12.414 15H15a2 2 0 002-2V5a1 1 0 \
                               100-2H3zm11 4a1 1 0 10-2 0v4a1 1 0 102 0V7zm-3 1a1 1 0 10-2 0v3a1 1 0 \
                               102 0V8zM8 9a1 1 0 00-2 0v2a1 1 0 102 0V9z"
                            clip-rule="evenodd"/>
                    </svg>
                </div>
                <div class="min-w-0">
                    <p class="text-sm font-medium text-gray-900 truncate">{prop.display_name}</p>
                    <p class="text-xs text-gray-400 font-mono mt-0.5">{prop.ga4_property_id}</p>
                </div>
            </div>
            <ActionForm action=action>
                <input type="hidden" name="property_id" value=prop_id/>
                <button
                    type="submit"
                    class="ml-4 flex-shrink-0 text-xs font-medium text-red-600 hover:text-red-700 \
                           px-2.5 py-1.5 rounded-md hover:bg-red-50 transition-colors"
                >
                    "Remove"
                </button>
            </ActionForm>
        </div>
    }
}

// ── Session section ───────────────────────────────────────────────────────────

#[component]
fn SessionSection() -> impl IntoView {
    view! {
        <SectionCard title="Session">
            <div class="flex items-center justify-between gap-4">
                <div>
                    <p class="text-sm font-medium text-gray-900">"Sign out of all devices"</p>
                    <p class="text-sm text-gray-500 mt-0.5">
                        "Ends every active session across all browsers and devices."
                    </p>
                </div>
                <a
                    href="/api/auth/logout-all"
                    class="flex-shrink-0 inline-flex items-center px-4 py-2 text-sm font-medium \
                           text-red-600 bg-white border border-red-200 rounded-lg hover:bg-red-50 transition-colors"
                >
                    "Sign out everywhere"
                </a>
            </div>
        </SectionCard>
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

#[component]
fn SectionCard(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-100">
                <h2 class="text-base font-semibold text-gray-900">{title}</h2>
            </div>
            <div class="px-6 py-5">
                {children()}
            </div>
        </div>
    }
}

#[component]
fn Field(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div>
            <dt class="text-xs font-medium text-gray-400 uppercase tracking-wider">{label}</dt>
            <dd class="mt-1 text-sm text-gray-900">{value}</dd>
        </div>
    }
}

// ── Skeleton ──────────────────────────────────────────────────────────────────

#[component]
fn SettingsSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="h-16 bg-white border-b border-gray-200"/>
            <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-10 space-y-8">
                <div class="h-8 w-28 bg-gray-200 rounded-md animate-pulse"/>
                {(0..3).map(|_| view! {
                    <div class="bg-white rounded-xl border border-gray-200 shadow-sm h-36 animate-pulse"/>
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}