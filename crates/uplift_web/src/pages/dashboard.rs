use std::collections::HashMap;

use leptos::prelude::*;
use leptos_meta::Title;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::nav::AppLayout;
use crate::server_utils::extract_session_id;


// ── Data types that cross the server/client boundary ───────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyRow {
    pub id: Uuid,
    pub display_name: String,
    pub ga4_property_id: String,
    pub analysis_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisRow {
    pub id: Uuid,
    pub property_name: String,
    pub metric: String,
    pub status: String,
    pub intervention_date: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardData {
    pub org_name: String,
    pub user_name: String,
    pub user_email: String,
    pub properties: Vec<PropertyRow>,
    pub analyses: Vec<AnalysisRow>,
}

// ── Server function ────────────────────────────────────────────────────────

#[server]
pub async fn load_dashboard() -> Result<DashboardData, ServerFnError> {
    use axum::http::HeaderMap;
    use leptos_axum::extract;
    use uplift_db::{AnalysisRepo, OrgRepo, PgPool, PropertyRepo, SessionRepo, UserRepo};

    let pool = expect_context::<PgPool>();
    let headers: HeaderMap = extract().await?;

    // ── Auth: validate session cookie ──────────────────────────────────────
    let session_id = extract_session_id(&headers).ok_or_else(|| {
        leptos_axum::redirect("/login");
        ServerFnError::new("not authenticated")
    })?;

    let session = SessionRepo::find_valid(&pool, session_id)
        .await
        .map_err(|_| {
            leptos_axum::redirect("/login");
            ServerFnError::new("session expired")
        })?;

    // ── Load user + org ────────────────────────────────────────────────────
    let user = UserRepo::find_by_id(&pool, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let org = OrgRepo::find_by_id(&pool, user.organization_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // ── Load properties + analyses in parallel ─────────────────────────────
    let (properties, analyses) = tokio::try_join!(
        PropertyRepo::list_by_org(&pool, org.id),
        AnalysisRepo::list_by_org(&pool, org.id),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Count analyses per property
    let mut counts: HashMap<Uuid, usize> = HashMap::new();
    for a in &analyses {
        *counts.entry(a.property_id).or_insert(0) += 1;
    }

    // Map property id → display name for the analyses table
    let name_map: HashMap<Uuid, String> = properties
        .iter()
        .map(|p| (p.id, p.display_name.clone()))
        .collect();

    let property_rows = properties
        .into_iter()
        .map(|p| {
            let count = counts.get(&p.id).copied().unwrap_or(0);
            PropertyRow {
                id: p.id,
                display_name: p.display_name,
                ga4_property_id: p.ga4_property_id,
                analysis_count: count,
            }
        })
        .collect();

    let analysis_rows = analyses
        .into_iter()
        .take(10)
        .map(|a| AnalysisRow {
            id: a.id,
            property_name: name_map
                .get(&a.property_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".into()),
            metric: format_metric(&a.metric),
            status: a.status,
            intervention_date: a.intervention_date.format("%b %d, %Y").to_string(),
            created_at: a.created_at.format("%b %d, %Y").to_string(),
        })
        .collect();

    let user_name = user
        .display_name
        .unwrap_or_else(|| user.email.split('@').next().unwrap_or("User").to_string());

    Ok(DashboardData {
        org_name: org.name,
        user_name,
        user_email: user.email,
        properties: property_rows,
        analyses: analysis_rows,
    })
}

fn format_metric(raw: &str) -> String {
    match raw {
        "sessions"            => "Sessions".into(),
        "activeUsers"         => "Active Users".into(),
        "newUsers"            => "New Users".into(),
        "screenPageViews"     => "Page Views".into(),
        "conversions"         => "Conversions".into(),
        "totalRevenue"        => "Revenue".into(),
        "engagementRate"      => "Engagement Rate".into(),
        other                 => other.to_string(),
    }
}

// ── Page component ─────────────────────────────────────────────────────────

#[component]
pub fn DashboardPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| load_dashboard());

    view! {
        <Title text="Dashboard — Uplift"/>
        <Suspense fallback=PageSkeleton>
            {move || data.get().map(|result| match result {
                Err(_) => view! {
                    // Server already issued a redirect to /login — show nothing
                    <div class="min-h-screen flex items-center justify-center">
                        <p class="text-sm text-gray-400">"Redirecting…"</p>
                    </div>
                }.into_any(),
                Ok(d) => view! { <DashboardContent data=d/> }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn DashboardContent(data: DashboardData) -> impl IntoView {
    let properties = data.properties.clone();
    let analyses   = data.analyses.clone();

    view! {
        <AppLayout
            org_name=data.org_name
            user_name=data.user_name
            user_email=data.user_email
        >
            // ── Page header ────────────────────────────────────────────────
            <div class="flex items-start justify-between mb-8">
                <div>
                    <h1 class="text-2xl font-bold tracking-tight text-gray-900">
                        "Dashboard"
                    </h1>
                    <p class="mt-1 text-sm text-gray-500">
                        "Your connected properties and causal impact analyses."
                    </p>
                </div>
                <a
                    href="/analyses/new"
                    class="inline-flex items-center gap-1.5 px-4 py-2 bg-brand-600 text-white
                           text-sm font-medium rounded-lg hover:bg-brand-700 transition-colors
                           shadow-sm"
                >
                    <span class="text-base leading-none">"+"</span>
                    "New Analysis"
                </a>
            </div>

            // ── Properties ─────────────────────────────────────────────────
            <section class="mb-10">
                <h2 class="text-xs font-semibold text-gray-400 uppercase tracking-widest mb-4">
                    "Connected Properties"
                </h2>

                {if properties.is_empty() {
                    view! { <EmptyProperties/> }.into_any()
                } else {
                    view! {
                        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                            {properties.into_iter()
                                .map(|p| view! { <PropertyCard property=p/> })
                                .collect_view()}
                        </div>
                    }.into_any()
                }}
            </section>

            // ── Recent Analyses ────────────────────────────────────────────
            <section>
                <h2 class="text-xs font-semibold text-gray-400 uppercase tracking-widest mb-4">
                    "Recent Analyses"
                </h2>

                {if analyses.is_empty() {
                    view! { <EmptyAnalyses/> }.into_any()
                } else {
                    view! { <AnalysesTable analyses=analyses/> }.into_any()
                }}
            </section>
        </AppLayout>
    }
}

// ── Sub-components ─────────────────────────────────────────────────────────

#[component]
fn PropertyCard(property: PropertyRow) -> impl IntoView {
    let analysis_label = match property.analysis_count {
        0 => "No analyses yet".to_string(),
        1 => "1 analysis".to_string(),
        n => format!("{n} analyses"),
    };

    let href = format!("/analyses/new?property={}", property.id);

    view! {
        <div class="bg-white border border-gray-200 rounded-xl p-5 flex flex-col gap-4
                    hover:border-gray-300 transition-colors">
            <div class="flex-1">
                <p class="text-sm font-semibold text-gray-900 truncate">
                    {property.display_name}
                </p>
                <p class="mt-0.5 text-xs font-mono text-gray-400 truncate">
                    {property.ga4_property_id}
                </p>
            </div>

            <div class="flex items-center justify-between">
                <span class="text-xs text-gray-400">{analysis_label}</span>
                <a
                    href=href
                    class="text-xs font-medium text-brand-600 hover:text-brand-700
                           transition-colors"
                >
                    "Run analysis →"
                </a>
            </div>
        </div>
    }
}

#[component]
fn AnalysesTable(analyses: Vec<AnalysisRow>) -> impl IntoView {
    view! {
        <div class="bg-white border border-gray-200 rounded-xl overflow-hidden">
            <table class="w-full text-sm">
                <thead>
                    <tr class="border-b border-gray-100 bg-gray-50">
                        <th class="px-5 py-3 text-left text-xs font-medium text-gray-500
                                   uppercase tracking-wider">
                            "Property"
                        </th>
                        <th class="px-5 py-3 text-left text-xs font-medium text-gray-500
                                   uppercase tracking-wider">
                            "Metric"
                        </th>
                        <th class="px-5 py-3 text-left text-xs font-medium text-gray-500
                                   uppercase tracking-wider">
                            "Intervention"
                        </th>
                        <th class="px-5 py-3 text-left text-xs font-medium text-gray-500
                                   uppercase tracking-wider">
                            "Status"
                        </th>
                        <th class="px-5 py-3 text-left text-xs font-medium text-gray-500
                                   uppercase tracking-wider">
                            "Created"
                        </th>
                        <th class="px-5 py-3"></th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">
                    {analyses.into_iter()
                        .map(|a| view! { <AnalysisRow analysis=a/> })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn AnalysisRow(analysis: AnalysisRow) -> impl IntoView {
    let href = format!("/analyses/{}", analysis.id);
    let (badge_bg, badge_text) = status_badge_classes(&analysis.status);

    view! {
        <tr class="hover:bg-gray-50 transition-colors">
            <td class="px-5 py-3.5 font-medium text-gray-900 max-w-[160px] truncate">
                {analysis.property_name}
            </td>
            <td class="px-5 py-3.5 text-gray-600">
                {analysis.metric}
            </td>
            <td class="px-5 py-3.5 font-mono text-gray-500 text-xs">
                {analysis.intervention_date}
            </td>
            <td class="px-5 py-3.5">
                <span class=format!(
                    "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {} {}",
                    badge_bg, badge_text
                )>
                    {analysis.status.clone()}
                </span>
            </td>
            <td class="px-5 py-3.5 text-gray-400 text-xs font-mono">
                {analysis.created_at}
            </td>
            <td class="px-5 py-3.5 text-right">
                <a
                    href=href
                    class="text-xs font-medium text-brand-600 hover:text-brand-700
                           transition-colors"
                >
                    "View →"
                </a>
            </td>
        </tr>
    }
}

fn status_badge_classes(status: &str) -> (&'static str, &'static str) {
    match status {
        "complete" => ("bg-green-50",  "text-green-700"),
        "running"  => ("bg-blue-50",   "text-blue-700"),
        "failed"   => ("bg-red-50",    "text-red-700"),
        _          => ("bg-gray-100",  "text-gray-600"),  // pending
    }
}

// ── Empty states ───────────────────────────────────────────────────────────

#[component]
fn EmptyProperties() -> impl IntoView {
    view! {
        <div class="bg-white border border-dashed border-gray-300 rounded-xl p-10 text-center">
            <p class="text-sm font-medium text-gray-900">"No properties connected"</p>
            <p class="mt-1 text-sm text-gray-500">
                "Connect a Google Analytics 4 property to start measuring impact."
            </p>
            <a
                href="/settings"
                class="mt-4 inline-flex items-center text-sm font-medium
                       text-brand-600 hover:text-brand-700"
            >
                "Connect a property →"
            </a>
        </div>
    }
}

#[component]
fn EmptyAnalyses() -> impl IntoView {
    view! {
        <div class="bg-white border border-dashed border-gray-300 rounded-xl p-10 text-center">
            <p class="text-sm font-medium text-gray-900">"No analyses yet"</p>
            <p class="mt-1 text-sm text-gray-500">
                "Run your first causal impact analysis to see results here."
            </p>
            <a
                href="/analyses/new"
                class="mt-4 inline-flex items-center text-sm font-medium
                       text-brand-600 hover:text-brand-700"
            >
                "Run first analysis →"
            </a>
        </div>
    }
}

// ── Loading skeleton ───────────────────────────────────────────────────────

#[component]
fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            // Nav skeleton
            <div class="bg-white border-b border-gray-200 h-14"/>

            <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
                // Header skeleton
                <div class="flex justify-between mb-8">
                    <div class="space-y-2">
                        <div class="h-7 w-32 bg-gray-200 rounded animate-pulse"/>
                        <div class="h-4 w-64 bg-gray-100 rounded animate-pulse"/>
                    </div>
                    <div class="h-9 w-32 bg-gray-200 rounded-lg animate-pulse"/>
                </div>

                // Properties skeleton
                <div class="h-4 w-40 bg-gray-200 rounded animate-pulse mb-4"/>
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-10">
                    {(0..3).map(|_| view! {
                        <div class="bg-white border border-gray-200 rounded-xl p-5 h-24
                                    animate-pulse"/>
                    }).collect_view()}
                </div>

                // Table skeleton
                <div class="h-4 w-40 bg-gray-200 rounded animate-pulse mb-4"/>
                <div class="bg-white border border-gray-200 rounded-xl h-48 animate-pulse"/>
            </div>
        </div>
    }
}