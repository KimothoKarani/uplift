use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_router::components::Redirect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::Shell;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub email: String,
    pub display_name: Option<String>,
    pub org_name: String,
    pub subscription: Option<SubInfo>,
    pub properties: Vec<PropInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubInfo {
    pub tier: String,
    pub status: String,
    pub renews: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropInfo {
    pub id: Uuid,
    pub display_name: String,
    pub ga4_property_id: String,
}

#[server(LoadSettings)]
pub async fn load_settings() -> Result<SettingsData, ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::{OrgRepo, PropertyRepo, SubscriptionRepo};

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let (org, sub_opt, props) = tokio::try_join!(
        OrgRepo::find_by_id(&pool, user.organization_id),
        SubscriptionRepo::find_by_org(&pool, user.organization_id),
        PropertyRepo::list_by_org(&pool, user.organization_id),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(SettingsData {
        email: user.email,
        display_name: user.display_name,
        org_name: org.name,
        subscription: sub_opt.map(|s| SubInfo {
            tier: s.tier,
            status: s.status,
            renews: s.current_period_end.format("%b %d, %Y").to_string(),
        }),
        properties: props
            .into_iter()
            .map(|p| PropInfo {
                id: p.id,
                display_name: p.display_name,
                ga4_property_id: p.ga4_property_id,
            })
            .collect(),
    })
}

#[server(DeleteProperty)]
pub async fn delete_property(property_id: String) -> Result<(), ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::PropertyRepo;

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let id: Uuid = property_id
        .parse()
        .map_err(|_| ServerFnError::new("invalid property id"))?;

    PropertyRepo::delete(&pool, id, user.organization_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| load_settings());

    view! {
        <Shell>
            <Suspense fallback=|| view! {
                <div class="px-8 py-7 max-w-2xl space-y-5">
                    <div class="h-7 bg-gray-200 rounded w-24 animate-pulse"/>
                    <div class="h-32 bg-white rounded-2xl border border-gray-100 animate-pulse"/>
                    <div class="h-24 bg-white rounded-2xl border border-gray-100 animate-pulse"/>
                </div>
            }>
                {move || {
                    data.get().map(|result| match result {
                        Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                        Ok(d) => view! { <SettingsContent data=d/> }.into_any(),
                    })
                }}
            </Suspense>
        </Shell>
    }
}

#[component]
fn SettingsContent(data: SettingsData) -> impl IntoView {
    view! {
        <div class="px-8 py-7 max-w-2xl space-y-5">
            <div class="mb-2">
                <h1 class="text-2xl font-bold text-gray-900">"Settings"</h1>
                <p class="text-sm text-gray-400 mt-1">"Manage your account, billing, and connected data sources."</p>
            </div>

            // ── Account ───────────────────────────────────────────
            <SettingsSection title="Account">
                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                    <InfoField label="Email" value=data.email/>
                    <InfoField label="Organisation" value=data.org_name/>
                    {data.display_name.map(|name| view! { <InfoField label="Name" value=name/> })}
                </div>
            </SettingsSection>

            // ── Billing ───────────────────────────────────────────
            <SettingsSection title="Billing">
                {match data.subscription {
                    None => view! {
                        <div class="flex items-center justify-between">
                            <div>
                                <p class="text-sm font-semibold text-gray-700">"Free plan"</p>
                                <p class="text-xs text-gray-400 mt-0.5">"Limited to 5 analyses per month."</p>
                            </div>
                            <a
                                href="/billing"
                                class="px-4 py-2 bg-indigo-600 text-white text-xs font-semibold rounded-lg hover:bg-indigo-700 transition-colors"
                            >
                                "Upgrade to Pro"
                            </a>
                        </div>
                    }.into_any(),
                    Some(sub) => view! {
                        <div class="grid grid-cols-3 gap-4">
                            <InfoField label="Plan" value=sub.tier/>
                            <InfoField label="Status" value=sub.status/>
                            <InfoField label="Renews" value=sub.renews/>
                        </div>
                    }.into_any(),
                }}
            </SettingsSection>

            // ── GA4 Properties ────────────────────────────────────
            <SettingsSection title="GA4 Properties">
                {if data.properties.is_empty() {
                    view! {
                        <div class="flex items-center justify-between">
                            <div>
                                <p class="text-sm font-semibold text-gray-700">"No properties connected"</p>
                                <p class="text-xs text-gray-400 mt-0.5">"Connect Google Analytics 4 to start running analyses."</p>
                            </div>
                            <a
                                href="/auth/google"
                                class="px-4 py-2 bg-indigo-600 text-white text-xs font-semibold rounded-lg hover:bg-indigo-700 transition-colors"
                            >
                                "Connect GA4"
                            </a>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <div class="divide-y divide-gray-50">
                                {data.properties.into_iter().map(|p| view! { <PropertyRow prop=p/> }).collect_view()}
                            </div>
                            <div class="mt-4 pt-4 border-t border-gray-50">
                                <a
                                    href="/auth/google"
                                    class="inline-flex items-center gap-1.5 text-xs font-semibold text-indigo-600 hover:text-indigo-700"
                                >
                                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                        <line x1="12" y1="5" x2="12" y2="19"/>
                                        <line x1="5" y1="12" x2="19" y2="12"/>
                                    </svg>
                                    "Connect another property"
                                </a>
                            </div>
                        </div>
                    }.into_any()
                }}
            </SettingsSection>

            // ── Search Console ────────────────────────────────────
            <SettingsSection title="Google Search Console">
                <div class="flex items-center justify-between">
                    <div>
                        <div class="flex items-center gap-2">
                            <p class="text-sm font-semibold text-gray-700">"Not connected"</p>
                            <span class="px-2 py-0.5 bg-amber-50 text-amber-700 text-[10px] font-semibold rounded-full border border-amber-100">"Coming soon"</span>
                        </div>
                        <p class="text-xs text-gray-400 mt-0.5">"Analyse the impact of SEO changes using impressions and clicks data."</p>
                    </div>
                    <button
                        disabled
                        class="px-4 py-2 bg-gray-100 text-gray-400 text-xs font-semibold rounded-lg cursor-not-allowed"
                    >
                        "Connect"
                    </button>
                </div>
            </SettingsSection>

            // ── Session ───────────────────────────────────────────
            <SettingsSection title="Session">
                <div class="flex items-center justify-between">
                    <p class="text-sm text-gray-500">"Sign out of your account on this device."</p>
                    <form method="post" action="/auth/logout">
                        <button
                            type="submit"
                            class="px-4 py-2 bg-red-50 text-red-600 text-xs font-semibold rounded-lg hover:bg-red-100 border border-red-100 transition-colors"
                        >
                            "Sign out"
                        </button>
                    </form>
                </div>
            </SettingsSection>
        </div>
    }
}

#[component]
fn PropertyRow(prop: PropInfo) -> impl IntoView {
    let action = ServerAction::<DeleteProperty>::new();
    let prop_id = prop.id.to_string();
    let name = prop.display_name.clone();
    let ga4_id = prop.ga4_property_id.clone();
    let pending = action.pending();

    view! {
        <div class="flex items-center justify-between py-3">
            <div class="flex items-center gap-3">
                <div class="w-8 h-8 bg-indigo-50 rounded-lg flex items-center justify-center flex-shrink-0">
                    <svg class="w-4 h-4 text-indigo-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/>
                        <polyline points="17 6 23 6 23 12"/>
                    </svg>
                </div>
                <div>
                    <p class="text-sm font-semibold text-gray-800">{name}</p>
                    <p class="text-[11px] text-gray-400 font-mono mt-0.5">{ga4_id}</p>
                </div>
            </div>
            <ActionForm action=action>
                <input type="hidden" name="property_id" value=prop_id/>
                <button
                    type="submit"
                    disabled=pending
                    class="text-xs font-medium text-red-400 hover:text-red-600 disabled:opacity-40 transition-colors"
                >
                    "Remove"
                </button>
            </ActionForm>
        </div>
    }
}

#[component]
fn SettingsSection(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="bg-white rounded-2xl border border-gray-100 p-6">
            <p class="text-[11px] font-bold text-gray-400 uppercase tracking-widest mb-4">{title}</p>
            {children()}
        </div>
    }
}

#[component]
fn InfoField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div>
            <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-widest">{label}</p>
            <p class="text-sm text-gray-800 mt-1">{value}</p>
        </div>
    }
}
