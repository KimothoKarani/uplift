use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::Redirect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        <Title text="Settings — Uplift"/>
        <Suspense fallback=SettingsSkeleton>
            {move || {
                data.get().map(|result| match result {
                    Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                    Ok(d) => view! { <SettingsContent data=d/> }.into_any(),
                })
            }}
        </Suspense>
    }
}

#[component]
fn SettingsContent(data: SettingsData) -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <nav class="bg-white border-b border-gray-200">
                <div class="max-w-3xl mx-auto px-4 sm:px-6 flex items-center justify-between h-14">
                    <span class="text-lg font-bold text-indigo-600">"Uplift"</span>
                    <a href="/dashboard" class="text-sm text-gray-500 hover:text-gray-900">
                        "← Dashboard"
                    </a>
                </div>
            </nav>

            <main class="max-w-3xl mx-auto px-4 sm:px-6 py-8 space-y-6">
                <h1 class="text-2xl font-bold text-gray-900">"Settings"</h1>

                <SectionCard title="Account">
                    <InfoField label="Email" value=data.email/>
                    {data
                        .display_name
                        .map(|name| view! { <InfoField label="Name" value=name/> })}
                    <InfoField label="Organisation" value=data.org_name/>
                </SectionCard>

                <SectionCard title="Billing">
                    {match data.subscription {
                        None => view! {
                            <p class="text-sm text-gray-500">
                                "No active subscription. "
                                <a href="/billing" class="text-indigo-600 hover:underline">
                                    "Upgrade →"
                                </a>
                            </p>
                        }.into_any(),
                        Some(sub) => view! {
                            <div class="grid grid-cols-3 gap-4">
                                <InfoField label="Plan" value=sub.tier/>
                                <InfoField label="Status" value=sub.status/>
                                <InfoField label="Renews" value=sub.renews/>
                            </div>
                        }.into_any(),
                    }}
                </SectionCard>

                <SectionCard title="GA4 Properties">
                    {if data.properties.is_empty() {
                        view! {
                            <p class="text-sm text-gray-500">
                                "No properties connected. "
                                <a href="/auth/google" class="text-indigo-600 hover:underline">
                                    "Connect one →"
                                </a>
                            </p>
                        }.into_any()
                    } else {
                        let rows = data
                            .properties
                            .into_iter()
                            .map(|p| view! { <PropertyRow prop=p/> })
                            .collect_view();
                        view! { <div class="divide-y divide-gray-100">{rows}</div> }.into_any()
                    }}
                </SectionCard>

                <SectionCard title="Session">
                    <form method="post" action="/auth/logout">
                        <button
                            type="submit"
                            class="text-sm font-medium text-red-600 hover:text-red-700"
                        >
                            "Sign out"
                        </button>
                    </form>
                </SectionCard>
            </main>
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
            <div>
                <p class="text-sm font-medium text-gray-900">{name}</p>
                <p class="text-xs text-gray-400 font-mono">{ga4_id}</p>
            </div>
            <ActionForm action=action>
                <input type="hidden" name="property_id" value=prop_id/>
                <button
                    type="submit"
                    disabled=pending
                    class="text-xs font-medium text-red-500 hover:text-red-700 disabled:opacity-50"
                >
                    "Remove"
                </button>
            </ActionForm>
        </div>
    }
}

#[component]
fn SectionCard(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="bg-white rounded-xl border border-gray-200 p-6">
            <h2 class="text-sm font-semibold text-gray-900 mb-4">{title}</h2>
            {children()}
        </div>
    }
}

#[component]
fn InfoField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="mb-3 last:mb-0">
            <p class="text-xs font-medium text-gray-400 uppercase tracking-wide">{label}</p>
            <p class="text-sm text-gray-800 mt-0.5">{value}</p>
        </div>
    }
}

#[component]
fn SettingsSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="bg-white border-b border-gray-200 h-14"/>
            <div class="max-w-3xl mx-auto px-4 sm:px-6 py-8 space-y-5">
                <div class="h-7 bg-gray-200 rounded w-24 animate-pulse"/>
                <div class="h-32 bg-gray-200 rounded-xl animate-pulse"/>
                <div class="h-24 bg-gray-200 rounded-xl animate-pulse"/>
            </div>
        </div>
    }
}
