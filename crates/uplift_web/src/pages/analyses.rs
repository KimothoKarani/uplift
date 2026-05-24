use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::Redirect;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisView {
    pub id: Uuid,
    pub description: String,
    pub metric: String,
    pub status: String,
    pub intervention_date: String,
    pub pre_period_start: String,
    pub pre_period_end: String,
    pub post_period_start: String,
    pub post_period_end: String,
    pub created_at: String,
    pub error_message: Option<String>,
    pub result: Option<AnalysisResultView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResultView {
    pub cumulative_effect: f64,
    pub cumulative_effect_lower: f64,
    pub cumulative_effect_upper: f64,
    pub relative_effect: f64,
    pub probability_of_effect: f64,
    pub narrative: String,
}

#[server(LoadAnalysis)]
pub async fn load_analysis(id: String) -> Result<AnalysisView, ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::AnalysisRepo;

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let analysis_id: Uuid = id
        .parse()
        .map_err(|_| ServerFnError::new("invalid analysis id"))?;

    let analysis = AnalysisRepo::find_by_id(&pool, analysis_id, user.organization_id)
        .await
        .map_err(|_| ServerFnError::new("analysis not found"))?;

    let result = AnalysisRepo::get_result(&pool, analysis_id)
        .await
        .ok()
        .map(|r| AnalysisResultView {
            cumulative_effect: r.cumulative_effect,
            cumulative_effect_lower: r.cumulative_effect_lower,
            cumulative_effect_upper: r.cumulative_effect_upper,
            relative_effect: r.relative_effect,
            probability_of_effect: r.probability_of_effect,
            narrative: r.narrative,
        });

    Ok(AnalysisView {
        id: analysis.id,
        description: analysis.description,
        metric: analysis.metric,
        status: analysis.status,
        intervention_date: analysis.intervention_date.to_string(),
        pre_period_start: analysis.pre_period_start.to_string(),
        pre_period_end: analysis.pre_period_end.to_string(),
        post_period_start: analysis.post_period_start.to_string(),
        post_period_end: analysis.post_period_end.to_string(),
        created_at: analysis.created_at.format("%b %d, %Y %H:%M").to_string(),
        error_message: analysis.error_message,
        result,
    })
}

#[component]
pub fn AnalysisPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.get().get("id").unwrap_or_default();
    let data = Resource::new(id, |id| load_analysis(id));

    view! {
        <Title text="Analysis — Uplift"/>
        <Suspense fallback=AnalysisSkeleton>
            {move || {
                data.get().map(|result| match result {
                    Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                    Ok(a) => view! { <AnalysisDetail analysis=a/> }.into_any(),
                })
            }}
        </Suspense>
    }
}

#[component]
fn AnalysisDetail(analysis: AnalysisView) -> impl IntoView {
    let status_cls = match analysis.status.as_str() {
        "complete" => "text-green-700 bg-green-50 border-green-200",
        "failed" => "text-red-700 bg-red-50 border-red-200",
        "running" => "text-blue-700 bg-blue-50 border-blue-200",
        _ => "text-gray-700 bg-gray-100 border-gray-200",
    }
    .to_string();

    view! {
        <div class="min-h-screen bg-gray-50">
            <nav class="bg-white border-b border-gray-200">
                <div class="max-w-4xl mx-auto px-4 sm:px-6 flex items-center justify-between h-14">
                    <span class="text-lg font-bold text-indigo-600">"Uplift"</span>
                    <a href="/dashboard" class="text-sm text-gray-500 hover:text-gray-900">
                        "← Dashboard"
                    </a>
                </div>
            </nav>

            <main class="max-w-4xl mx-auto px-4 sm:px-6 py-8 space-y-6">
                <div class="flex items-start justify-between gap-4">
                    <div>
                        <h1 class="text-xl font-bold text-gray-900">{analysis.description.clone()}</h1>
                        <p class="text-sm text-gray-500 mt-1">"Created " {analysis.created_at}</p>
                    </div>
                    <span class=format!(
                        "flex-shrink-0 inline-flex items-center px-3 py-1 rounded-full text-xs font-semibold border {}",
                        status_cls,
                    )>{analysis.status.clone()}</span>
                </div>

                <div class="bg-white rounded-xl border border-gray-200 p-5 grid grid-cols-2 sm:grid-cols-4 gap-4 text-sm">
                    <MetaField label="Metric" value=analysis.metric.clone()/>
                    <MetaField label="Intervention" value=analysis.intervention_date.clone()/>
                    <MetaField
                        label="Pre-period"
                        value=format!("{} – {}", analysis.pre_period_start, analysis.pre_period_end)
                    />
                    <MetaField
                        label="Post-period"
                        value=format!("{} – {}", analysis.post_period_start, analysis.post_period_end)
                    />
                </div>

                {match analysis.status.as_str() {
                    "pending" | "running" => {
                        view! {
                            <div class="bg-white rounded-xl border border-gray-200 p-8 text-center">
                                <div class="inline-block w-8 h-8 border-4 border-indigo-200 border-t-indigo-600 rounded-full animate-spin mb-3"/>
                                <p class="text-sm text-gray-600">
                                    "Analysis is running. Refresh this page to check for results."
                                </p>
                            </div>
                        }.into_any()
                    }
                    "failed" => {
                        view! {
                            <div class="bg-red-50 rounded-xl border border-red-200 p-5">
                                <h2 class="text-sm font-semibold text-red-800 mb-1">"Analysis failed"</h2>
                                <p class="text-sm text-red-700">
                                    {analysis.error_message.unwrap_or_else(|| "Unknown error".into())}
                                </p>
                            </div>
                        }.into_any()
                    }
                    "complete" => {
                        if let Some(r) = analysis.result {
                            view! { <ResultCard result=r/> }.into_any()
                        } else {
                            view! { <div class="text-sm text-gray-500">"No result data found."</div> }
                                .into_any()
                        }
                    }
                    _ => view! { <></> }.into_any(),
                }}
            </main>
        </div>
    }
}

#[component]
fn ResultCard(result: AnalysisResultView) -> impl IntoView {
    let pct_effect = result.relative_effect * 100.0;
    let probability_pct = result.probability_of_effect * 100.0;
    let sign = if result.cumulative_effect >= 0.0 { "+" } else { "" };

    view! {
        <div class="space-y-5">
            <div class="grid grid-cols-3 gap-4">
                <StatCard
                    label="Cumulative effect"
                    value=format!(
                        "{}{:.0} ({:.0} – {:.0})",
                        sign,
                        result.cumulative_effect,
                        result.cumulative_effect_lower,
                        result.cumulative_effect_upper,
                    )
                    highlight=true
                />
                <StatCard
                    label="Relative effect"
                    value=format!(
                        "{}{:.1}%",
                        if pct_effect >= 0.0 { "+" } else { "" },
                        pct_effect,
                    )
                    highlight=false
                />
                <StatCard
                    label="Probability of effect"
                    value=format!("{:.1}%", probability_pct)
                    highlight=false
                />
            </div>
            <div class="bg-white rounded-xl border border-gray-200 p-5">
                <h3 class="text-sm font-semibold text-gray-700 mb-3">"Interpretation"</h3>
                <p class="text-sm text-gray-700 leading-relaxed">{result.narrative}</p>
            </div>
        </div>
    }
}

#[component]
fn StatCard(label: &'static str, value: String, highlight: bool) -> impl IntoView {
    let outer = if highlight { "bg-indigo-600 text-white" } else { "bg-white border border-gray-200" };
    let label_cls = if highlight { "text-indigo-200" } else { "text-gray-500" };
    let value_cls = if highlight { "text-white" } else { "text-gray-900" };
    view! {
        <div class=format!("rounded-xl p-4 {}", outer)>
            <p class=format!("text-xs font-medium uppercase tracking-wide {}", label_cls)>
                {label}
            </p>
            <p class=format!("mt-1 text-lg font-bold font-mono {}", value_cls)>{value}</p>
        </div>
    }
}

#[component]
fn MetaField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div>
            <p class="text-xs font-medium text-gray-400 uppercase tracking-wide">{label}</p>
            <p class="text-sm font-medium text-gray-900 mt-0.5">{value}</p>
        </div>
    }
}

#[component]
fn AnalysisSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="bg-white border-b border-gray-200 h-14"/>
            <div class="max-w-4xl mx-auto px-4 sm:px-6 py-8 space-y-5">
                <div class="h-7 bg-gray-200 rounded w-64 animate-pulse"/>
                <div class="h-24 bg-gray-200 rounded-xl animate-pulse"/>
                <div class="h-48 bg-gray-200 rounded-xl animate-pulse"/>
            </div>
        </div>
    }
}
