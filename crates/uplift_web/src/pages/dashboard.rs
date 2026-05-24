use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::Redirect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub email: String,
    pub properties: Vec<PropSummary>,
    pub analyses: Vec<AnalysisSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropSummary {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub id: Uuid,
    pub description: String,
    pub metric: String,
    pub status: String,
    pub created_at: String,
}

#[server(LoadDashboard)]
pub async fn load_dashboard() -> Result<DashboardData, ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::{AnalysisRepo, PropertyRepo};

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let (properties, analyses) = tokio::try_join!(
        PropertyRepo::list_by_org(&pool, user.organization_id),
        AnalysisRepo::list_by_org(&pool, user.organization_id),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(DashboardData {
        email: user.email,
        properties: properties
            .into_iter()
            .map(|p| PropSummary {
                id: p.id,
                display_name: p.display_name,
            })
            .collect(),
        analyses: analyses
            .into_iter()
            .map(|a| AnalysisSummary {
                id: a.id,
                description: a.description,
                metric: a.metric,
                status: a.status,
                created_at: a.created_at.format("%b %d, %Y").to_string(),
            })
            .collect(),
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| load_dashboard());

    view! {
        <Title text="Dashboard — Uplift"/>
        <Suspense fallback=DashboardSkeleton>
            {move || {
                data.get().map(|result| match result {
                    Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                    Ok(d) => view! { <DashboardContent data=d/> }.into_any(),
                })
            }}
        </Suspense>
    }
}

#[component]
fn DashboardContent(data: DashboardData) -> impl IntoView {
    let email = data.email.clone();
    let short_email = if email.len() > 24 {
        format!("{}…", &email[..24])
    } else {
        email
    };

    view! {
        <div class="min-h-screen bg-gray-50">
            <nav class="bg-white border-b border-gray-200">
                <div class="max-w-6xl mx-auto px-4 sm:px-6 flex items-center justify-between h-14">
                    <span class="text-lg font-bold text-indigo-600">"Uplift"</span>
                    <div class="flex items-center gap-4 text-sm">
                        <a href="/settings" class="text-gray-600 hover:text-gray-900">
                            {short_email}
                        </a>
                        <form method="post" action="/auth/logout">
                            <button
                                type="submit"
                                class="text-gray-400 hover:text-gray-700 transition-colors"
                            >
                                "Sign out"
                            </button>
                        </form>
                    </div>
                </div>
            </nav>

            <main class="max-w-6xl mx-auto px-4 sm:px-6 py-8 space-y-8">
                <PropertiesSection properties=data.properties/>
                <AnalysesSection analyses=data.analyses/>
            </main>
        </div>
    }
}

#[component]
fn PropertiesSection(properties: Vec<PropSummary>) -> impl IntoView {
    let is_empty = properties.is_empty();
    view! {
        <section>
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-base font-semibold text-gray-900">"GA4 Properties"</h2>
                <a
                    href="/auth/google"
                    class="text-sm font-medium text-indigo-600 hover:text-indigo-700"
                >
                    "+ Connect property"
                </a>
            </div>
            {if is_empty {
                view! {
                    <div class="bg-white rounded-xl border border-dashed border-gray-300 p-8 text-center">
                        <p class="text-sm text-gray-500">"No GA4 properties connected yet."</p>
                        <a
                            href="/auth/google"
                            class="mt-2 inline-block text-sm font-medium text-indigo-600 hover:underline"
                        >
                            "Connect your first property →"
                        </a>
                    </div>
                }.into_any()
            } else {
                let cards = properties
                    .into_iter()
                    .map(|p| {
                        view! {
                            <div class="bg-white rounded-xl border border-gray-200 p-4">
                                <p class="text-sm font-medium text-gray-900">{p.display_name}</p>
                            </div>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">{cards}</div>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn AnalysesSection(analyses: Vec<AnalysisSummary>) -> impl IntoView {
    let is_empty = analyses.is_empty();
    view! {
        <section>
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-base font-semibold text-gray-900">"Analyses"</h2>
                <a
                    href="/analyses/new"
                    class="inline-flex items-center px-3 py-1.5 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition-colors"
                >
                    "New analysis"
                </a>
            </div>
            {if is_empty {
                view! {
                    <div class="bg-white rounded-xl border border-dashed border-gray-300 p-8 text-center">
                        <p class="text-sm text-gray-500">"No analyses yet."</p>
                        <a
                            href="/analyses/new"
                            class="mt-2 inline-block text-sm font-medium text-indigo-600 hover:underline"
                        >
                            "Create your first analysis →"
                        </a>
                    </div>
                }.into_any()
            } else {
                let rows = analyses
                    .into_iter()
                    .map(|a| {
                        let status_cls = match a.status.as_str() {
                            "complete" => "text-green-700 bg-green-50",
                            "failed" => "text-red-700 bg-red-50",
                            "running" => "text-blue-700 bg-blue-50",
                            _ => "text-gray-700 bg-gray-100",
                        }
                        .to_string();
                        let href = format!("/analyses/{}", a.id);
                        view! {
                            <tr class="hover:bg-gray-50">
                                <td class="px-4 py-3">
                                    <a
                                        href=href
                                        class="font-medium text-indigo-600 hover:underline text-sm"
                                    >
                                        {a.description}
                                    </a>
                                </td>
                                <td class="px-4 py-3 text-sm font-mono text-gray-600">
                                    {a.metric}
                                </td>
                                <td class="px-4 py-3">
                                    <span class=format!(
                                        "inline-flex px-2 py-0.5 rounded-full text-xs font-medium {}",
                                        status_cls,
                                    )>{a.status}</span>
                                </td>
                                <td class="px-4 py-3 text-sm text-gray-400">{a.created_at}</td>
                            </tr>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
                        <table class="w-full text-sm">
                            <thead class="border-b border-gray-100 bg-gray-50 text-left">
                                <tr>
                                    <th class="px-4 py-3 font-medium text-gray-500 text-xs uppercase tracking-wide">
                                        "Description"
                                    </th>
                                    <th class="px-4 py-3 font-medium text-gray-500 text-xs uppercase tracking-wide">
                                        "Metric"
                                    </th>
                                    <th class="px-4 py-3 font-medium text-gray-500 text-xs uppercase tracking-wide">
                                        "Status"
                                    </th>
                                    <th class="px-4 py-3 font-medium text-gray-500 text-xs uppercase tracking-wide">
                                        "Created"
                                    </th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-gray-50">{rows}</tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn DashboardSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="bg-white border-b border-gray-200 h-14"/>
            <div class="max-w-6xl mx-auto px-4 sm:px-6 py-8 space-y-6">
                <div class="h-5 bg-gray-200 rounded w-32 animate-pulse"/>
                <div class="h-20 bg-gray-200 rounded-xl animate-pulse"/>
                <div class="h-5 bg-gray-200 rounded w-32 animate-pulse"/>
                <div class="h-48 bg-gray-200 rounded-xl animate-pulse"/>
            </div>
        </div>
    }
}
