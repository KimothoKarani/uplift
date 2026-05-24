use leptos::prelude::*;
use leptos_router::components::Redirect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::Shell;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub display_name: String,
    pub properties: Vec<PropSummary>,
    pub analyses: Vec<AnalysisSummary>,
    pub complete_count: usize,
    pub running_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropSummary {
    pub id: Uuid,
    pub display_name: String,
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub id: Uuid,
    pub description: String,
    pub metric: String,
    pub status: String,
    pub property_name: String,
    pub created_at: String,
}

#[server(LoadDashboard)]
pub async fn load_dashboard() -> Result<DashboardData, ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use std::collections::HashMap;
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

    let prop_map: HashMap<Uuid, String> = properties
        .iter()
        .map(|p| (p.id, p.display_name.clone()))
        .collect();

    let complete_count = analyses.iter().filter(|a| a.status == "complete").count();
    let running_count = analyses
        .iter()
        .filter(|a| a.status == "pending" || a.status == "running")
        .count();

    let display_name = user
        .display_name
        .unwrap_or_else(|| user.email.split('@').next().unwrap_or("there").to_string());

    Ok(DashboardData {
        display_name,
        properties: properties
            .into_iter()
            .map(|p| PropSummary {
                id: p.id,
                website_url: p.website_url,
                display_name: p.display_name,
            })
            .collect(),
        analyses: analyses
            .into_iter()
            .take(10)
            .map(|a| AnalysisSummary {
                property_name: prop_map
                    .get(&a.property_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".into()),
                id: a.id,
                description: a.description,
                metric: a.metric,
                status: a.status,
                created_at: a.created_at.format("%b %d").to_string(),
            })
            .collect(),
        complete_count,
        running_count,
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| load_dashboard());

    view! {
        <Shell>
            <Suspense fallback=DashboardSkeleton>
                {move || {
                    data.get().map(|result| match result {
                        Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                        Ok(d) => view! { <DashboardContent data=d/> }.into_any(),
                    })
                }}
            </Suspense>
        </Shell>
    }
}

#[component]
fn DashboardContent(data: DashboardData) -> impl IntoView {
    let today = chrono_today();
    let name = data.display_name.clone();
    let ga4_connected = !data.properties.is_empty();
    let props_count = data.properties.len();
    let analyses_count = data.analyses.len();

    view! {
        <div class="px-8 py-7">
            // ── Header ────────────────────────────────────────────
            <div class="flex items-start justify-between mb-8">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">
                        "Hello, " {name} "!"
                    </h1>
                    <p class="text-sm text-gray-400 mt-1">
                        "Run an analysis to measure the real impact of your changes."
                    </p>
                </div>
                <div class="flex items-center gap-1.5 text-sm text-gray-400 bg-white border border-gray-100 rounded-xl px-3 py-1.5">
                    <svg
                        class="w-4 h-4"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <rect x="3" y="4" width="18" height="18" rx="2"/>
                        <line x1="16" y1="2" x2="16" y2="6"/>
                        <line x1="8" y1="2" x2="8" y2="6"/>
                        <line x1="3" y1="10" x2="21" y2="10"/>
                    </svg>
                    {today}
                </div>
            </div>

            // ── Stats row ─────────────────────────────────────────
            <div class="grid grid-cols-4 gap-4 mb-8">
                <StatCard
                    icon_bg="bg-blue-50"
                    icon_color="text-blue-500"
                    label="Analyses done"
                    value=data.complete_count.to_string()
                    sub=format!("{} total", analyses_count)
                >
                    <svg
                        class="w-5 h-5"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points="20 6 9 17 4 12"/>
                    </svg>
                </StatCard>
                <StatCard
                    icon_bg="bg-orange-50"
                    icon_color="text-orange-400"
                    label="Running now"
                    value=data.running_count.to_string()
                    sub="in progress".to_string()
                >
                    <svg
                        class="w-5 h-5"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <circle cx="12" cy="12" r="10"/>
                        <polyline points="12 6 12 12 16 14"/>
                    </svg>
                </StatCard>
                <StatCard
                    icon_bg="bg-indigo-50"
                    icon_color="text-indigo-500"
                    label="GA4 Properties"
                    value=props_count.to_string()
                    sub=if ga4_connected { "connected".to_string() } else { "not connected".to_string() }
                >
                    <svg
                        class="w-5 h-5"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <ellipse cx="12" cy="5" rx="9" ry="3"/>
                        <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/>
                        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
                    </svg>
                </StatCard>
                <StatCard
                    icon_bg="bg-green-50"
                    icon_color="text-green-500"
                    label="Model"
                    value="ITS v1".to_string()
                    sub="Bayesian structural".to_string()
                >
                    <svg
                        class="w-5 h-5"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M12 2L2 7l10 5 10-5-10-5z"/>
                        <path d="M2 17l10 5 10-5"/>
                        <path d="M2 12l10 5 10-5"/>
                    </svg>
                </StatCard>
            </div>

            // ── Data sources ──────────────────────────────────────
            <h2 class="text-[13px] font-semibold text-gray-400 uppercase tracking-widest mb-3">
                "Data Sources"
            </h2>
            <div class="grid grid-cols-2 gap-4 mb-8">
                <DataSourceCard
                    name="Google Analytics 4"
                    description="Session, user, conversion and engagement metrics"
                    connected=ga4_connected
                    connect_href="/auth/google"
                    logo_color="bg-orange-500"
                    logo_letter="G"
                    properties=data.properties
                />
                <DataSourceCard
                    name="Google Search Console"
                    description="Organic impressions, clicks, CTR and position"
                    connected=false
                    connect_href="#"
                    logo_color="bg-green-500"
                    logo_letter="S"
                    properties=vec![]
                />
            </div>

            // ── Recent analyses ───────────────────────────────────
            <div class="flex items-center justify-between mb-3">
                <div>
                    <h2 class="text-[13px] font-semibold text-gray-400 uppercase tracking-widest">
                        "Recent Analyses"
                    </h2>
                </div>
                <a
                    href="/analyses/new"
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 bg-indigo-600 text-white text-[12px] font-semibold rounded-lg hover:bg-indigo-700 transition-colors"
                >
                    <svg
                        class="w-3.5 h-3.5"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <line x1="12" y1="5" x2="12" y2="19"/>
                        <line x1="5" y1="12" x2="19" y2="12"/>
                    </svg>
                    "New analysis"
                </a>
            </div>
            <AnalysesTable analyses=data.analyses/>
        </div>
    }
}

#[component]
fn StatCard(
    icon_bg: &'static str,
    icon_color: &'static str,
    label: &'static str,
    value: String,
    sub: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-2xl border border-gray-100 p-5">
            <div class=format!(
                "w-9 h-9 {} {} rounded-xl flex items-center justify-center mb-3",
                icon_bg,
                icon_color,
            )>{children()}</div>
            <p class="text-[12px] font-medium text-gray-400 uppercase tracking-wide">{label}</p>
            <div class="flex items-end gap-2 mt-1">
                <p class="text-2xl font-bold text-gray-900">{value}</p>
                <p class="text-xs text-gray-400 mb-0.5">{sub}</p>
            </div>
        </div>
    }
}

#[component]
fn DataSourceCard(
    name: &'static str,
    description: &'static str,
    connected: bool,
    connect_href: &'static str,
    logo_color: &'static str,
    logo_letter: &'static str,
    properties: Vec<PropSummary>,
) -> impl IntoView {
    let props_list = properties.clone();
    view! {
        <div class="bg-white rounded-2xl border border-gray-100 p-5">
            <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-3">
                    <div class=format!(
                        "w-9 h-9 {} rounded-xl flex items-center justify-center text-white text-sm font-bold flex-shrink-0",
                        logo_color,
                    )>{logo_letter}</div>
                    <div>
                        <p class="text-sm font-semibold text-gray-900">{name}</p>
                        <p class="text-[11px] text-gray-400">{description}</p>
                    </div>
                </div>
                {if connected {
                    view! {
                        <span class="flex items-center gap-1 text-[11px] font-semibold text-green-600 bg-green-50 px-2 py-0.5 rounded-full">
                            <span class="w-1.5 h-1.5 bg-green-500 rounded-full"/>
                            "Connected"
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="flex items-center gap-1 text-[11px] font-medium text-gray-400 bg-gray-50 px-2 py-0.5 rounded-full">
                            "Not connected"
                        </span>
                    }.into_any()
                }}
            </div>

            {if connected && !props_list.is_empty() {
                let tags = props_list
                    .into_iter()
                    .map(|p| {
                        view! {
                            <span class="inline-flex items-center px-2 py-0.5 bg-indigo-50 text-indigo-700 text-[11px] font-medium rounded-lg">
                                {p.display_name}
                            </span>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="flex flex-wrap gap-1.5 mt-1">
                        {tags}
                        <a
                            href=connect_href
                            class="inline-flex items-center px-2 py-0.5 border border-dashed border-indigo-300 text-indigo-500 text-[11px] rounded-lg hover:bg-indigo-50 transition-colors"
                        >
                            "+ Add property"
                        </a>
                    </div>
                }.into_any()
            } else if connected {
                view! {
                    <a
                        href=connect_href
                        class="inline-flex items-center gap-1 text-[12px] font-medium text-indigo-600 hover:underline"
                    >
                        "Connect a property →"
                    </a>
                }.into_any()
            } else {
                view! {
                    <a
                        href=connect_href
                        class="inline-flex items-center gap-1 text-[12px] font-medium text-indigo-600 hover:underline"
                    >
                        "Connect →"
                    </a>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn AnalysesTable(analyses: Vec<AnalysisSummary>) -> impl IntoView {
    if analyses.is_empty() {
        return view! {
            <div class="bg-white rounded-2xl border border-dashed border-gray-200 py-14 text-center">
                <div class="w-10 h-10 bg-gray-100 rounded-full flex items-center justify-center mx-auto mb-3">
                    <svg
                        class="w-5 h-5 text-gray-400"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <line x1="18" y1="20" x2="18" y2="10"/>
                        <line x1="12" y1="20" x2="12" y2="4"/>
                        <line x1="6" y1="20" x2="6" y2="14"/>
                    </svg>
                </div>
                <p class="text-sm font-medium text-gray-500">"No analyses yet"</p>
                <p class="text-xs text-gray-400 mt-1">
                    "Connect a GA4 property and run your first analysis"
                </p>
                <a
                    href="/analyses/new"
                    class="mt-4 inline-block text-xs font-semibold text-indigo-600 hover:underline"
                >
                    "Create your first analysis →"
                </a>
            </div>
        }.into_any();
    }

    let rows = analyses
        .into_iter()
        .map(|a| {
            let (dot, badge) = status_styles(&a.status);
            let href = format!("/analyses/{}", a.id);
            view! {
                <tr class="group hover:bg-gray-50 transition-colors">
                    <td class="px-5 py-3.5">
                        <div class="flex items-center gap-2">
                            <span class=format!("w-2 h-2 rounded-full flex-shrink-0 {}", dot)/>
                            <a href=href class="text-[13px] font-medium text-gray-900 group-hover:text-indigo-600 transition-colors truncate max-w-[220px]">
                                {a.description}
                            </a>
                        </div>
                    </td>
                    <td class="px-5 py-3.5">
                        <span class="text-[11px] font-medium text-gray-500 bg-gray-100 px-2 py-0.5 rounded-md">
                            {a.property_name}
                        </span>
                    </td>
                    <td class="px-5 py-3.5 font-mono text-[12px] text-gray-500">{a.metric}</td>
                    <td class="px-5 py-3.5">
                        <span class=format!("text-[11px] font-semibold px-2.5 py-1 rounded-full {}", badge)>
                            {a.status}
                        </span>
                    </td>
                    <td class="px-5 py-3.5 text-[12px] text-gray-400">{a.created_at}</td>
                    <td class="px-5 py-3.5 text-right">
                        <a href=format!("/analyses/{}", a.id) class="text-gray-300 hover:text-indigo-500 transition-colors">
                            <svg class="w-4 h-4 inline" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <polyline points="9 18 15 12 9 6"/>
                            </svg>
                        </a>
                    </td>
                </tr>
            }
        })
        .collect_view();

    view! {
        <div class="bg-white rounded-2xl border border-gray-100 overflow-hidden">
            <table class="w-full">
                <thead>
                    <tr class="border-b border-gray-50">
                        <th class="px-5 py-3 text-left text-[11px] font-semibold text-gray-400 uppercase tracking-wider">
                            "Description"
                        </th>
                        <th class="px-5 py-3 text-left text-[11px] font-semibold text-gray-400 uppercase tracking-wider">
                            "Property"
                        </th>
                        <th class="px-5 py-3 text-left text-[11px] font-semibold text-gray-400 uppercase tracking-wider">
                            "Metric"
                        </th>
                        <th class="px-5 py-3 text-left text-[11px] font-semibold text-gray-400 uppercase tracking-wider">
                            "Status"
                        </th>
                        <th class="px-5 py-3 text-left text-[11px] font-semibold text-gray-400 uppercase tracking-wider">
                            "Created"
                        </th>
                        <th class="px-5 py-3"/>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-50">{rows}</tbody>
            </table>
        </div>
    }
    .into_any()
}

fn status_styles(status: &str) -> (&'static str, &'static str) {
    match status {
        "complete" => ("bg-green-500", "text-green-700 bg-green-50"),
        "failed" => ("bg-red-400", "text-red-700 bg-red-50"),
        "running" => ("bg-blue-500 animate-pulse", "text-blue-700 bg-blue-50"),
        _ => ("bg-yellow-400", "text-yellow-700 bg-yellow-50"),
    }
}

fn chrono_today() -> String {
    // Called at render time — chrono is available server-side
    #[cfg(not(target_arch = "wasm32"))]
    {
        chrono::Utc::now().format("%d %B, %Y").to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        String::new()
    }
}

#[component]
fn DashboardSkeleton() -> impl IntoView {
    view! {
        <Shell>
            <div class="px-8 py-7 space-y-6">
                <div class="h-8 bg-gray-200 rounded-xl w-48 animate-pulse"/>
                <div class="grid grid-cols-4 gap-4">
                    {(0..4)
                        .map(|_| view! { <div class="h-28 bg-gray-200 rounded-2xl animate-pulse"/> })
                        .collect_view()}
                </div>
                <div class="grid grid-cols-2 gap-4">
                    <div class="h-28 bg-gray-200 rounded-2xl animate-pulse"/>
                    <div class="h-28 bg-gray-200 rounded-2xl animate-pulse"/>
                </div>
                <div class="h-48 bg-gray-200 rounded-2xl animate-pulse"/>
            </div>
        </Shell>
    }
}
