use leptos::prelude::*;
use leptos_router::components::Redirect;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::Shell;

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
    pub property_name: String,
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
    pub chart_actual: Vec<f64>,
    pub chart_counterfactual: Vec<f64>,
    pub intervention_index: usize,
}

#[server(LoadAnalysis)]
pub async fn load_analysis(id: String) -> Result<AnalysisView, ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::{AnalysisRepo, PropertyRepo};
    use uplift_core::impact::report::PointwiseEffect;

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let analysis_id: Uuid = id
        .parse()
        .map_err(|_| ServerFnError::new("invalid analysis id"))?;

    let analysis = AnalysisRepo::find_by_id(&pool, analysis_id, user.organization_id)
        .await
        .map_err(|_| ServerFnError::new("analysis not found"))?;

    let property_name = PropertyRepo::find_by_id(&pool, analysis.property_id, user.organization_id)
        .await
        .map(|p| p.display_name)
        .unwrap_or_else(|_| "Unknown".into());

    let result = AnalysisRepo::get_result(&pool, analysis_id).await.ok().map(|r| {
        let pointwise: Vec<PointwiseEffect> =
            serde_json::from_value(r.pointwise_effects).unwrap_or_default();

        let intervention = analysis.intervention_date;
        let intervention_index = pointwise
            .iter()
            .position(|p| p.timestamp.date_naive() >= intervention)
            .unwrap_or(pointwise.len() / 2);

        let chart_actual = pointwise.iter().map(|p| p.actual).collect();
        let chart_counterfactual = pointwise.iter().map(|p| p.counterfactual).collect();

        AnalysisResultView {
            cumulative_effect: r.cumulative_effect,
            cumulative_effect_lower: r.cumulative_effect_lower,
            cumulative_effect_upper: r.cumulative_effect_upper,
            relative_effect: r.relative_effect,
            probability_of_effect: r.probability_of_effect,
            narrative: r.narrative,
            chart_actual,
            chart_counterfactual,
            intervention_index,
        }
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
        created_at: analysis.created_at.format("%b %d, %Y %H:%M UTC").to_string(),
        property_name,
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
        <Shell>
            <Suspense fallback=AnalysisSkeleton>
                {move || {
                    data.get().map(|result| match result {
                        Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                        Ok(a) => view! { <AnalysisDetail analysis=a/> }.into_any(),
                    })
                }}
            </Suspense>
        </Shell>
    }
}

#[component]
fn AnalysisDetail(analysis: AnalysisView) -> impl IntoView {
    let (status_dot, status_badge) = match analysis.status.as_str() {
        "complete" => ("bg-green-500", "text-green-700 bg-green-50"),
        "failed" => ("bg-red-400", "text-red-700 bg-red-50"),
        "running" => ("bg-blue-500", "text-blue-700 bg-blue-50"),
        _ => ("bg-yellow-400", "text-yellow-700 bg-yellow-50"),
    };

    view! {
        <div class="px-8 py-7">
            // ── Breadcrumb ────────────────────────────────────────
            <a href="/dashboard" class="inline-flex items-center gap-1.5 text-[12px] text-gray-400 hover:text-gray-600 mb-5 transition-colors">
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="15 18 9 12 15 6"/>
                </svg>
                "All analyses"
            </a>

            // ── Header ────────────────────────────────────────────
            <div class="flex items-start justify-between gap-4 mb-6">
                <div>
                    <h1 class="text-xl font-bold text-gray-900">{analysis.description.clone()}</h1>
                    <div class="flex items-center gap-3 mt-1.5">
                        <span class="text-[11px] font-medium text-gray-400 bg-gray-100 px-2 py-0.5 rounded-md">
                            {analysis.property_name}
                        </span>
                        <span class="text-[12px] font-mono text-gray-400">{analysis.metric.clone()}</span>
                        <span class="text-[12px] text-gray-400">{analysis.created_at}</span>
                    </div>
                </div>
                <span class=format!("flex-shrink-0 flex items-center gap-1.5 text-[12px] font-semibold px-3 py-1.5 rounded-full {}", status_badge)>
                    <span class=format!("w-2 h-2 rounded-full {}", status_dot)/>
                    {analysis.status.clone()}
                </span>
            </div>

            // ── Meta grid ─────────────────────────────────────────
            <div class="grid grid-cols-4 gap-4 mb-6">
                <MetaCard label="Intervention" value=analysis.intervention_date.clone()/>
                <MetaCard
                    label="Pre-period"
                    value=format!("{} – {}", analysis.pre_period_start, analysis.pre_period_end)
                />
                <MetaCard
                    label="Post-period"
                    value=format!("{} – {}", analysis.post_period_start, analysis.post_period_end)
                />
                <MetaCard label="Metric" value=analysis.metric.clone()/>
            </div>

            // ── Body based on status ──────────────────────────────
            {match analysis.status.as_str() {
                "pending" | "running" => view! {
                    <div class="bg-white rounded-2xl border border-gray-100 p-12 text-center">
                        <div class="w-10 h-10 border-4 border-indigo-100 border-t-indigo-600 rounded-full animate-spin mx-auto mb-4"/>
                        <p class="text-[13px] font-semibold text-gray-700">"Analysis is running"</p>
                        <p class="text-[12px] text-gray-400 mt-1">
                            "The model is fetching your GA4 data and fitting the Bayesian ITS model."
                        </p>
                        <p class="text-[11px] text-gray-300 mt-3">"Refresh this page in a few minutes."</p>
                    </div>
                }.into_any(),
                "failed" => view! {
                    <div class="bg-red-50 rounded-2xl border border-red-100 p-6">
                        <div class="flex items-start gap-3">
                            <div class="w-8 h-8 bg-red-100 rounded-xl flex items-center justify-center flex-shrink-0">
                                <svg class="w-4 h-4 text-red-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <circle cx="12" cy="12" r="10"/>
                                    <line x1="15" y1="9" x2="9" y2="15"/>
                                    <line x1="9" y1="9" x2="15" y2="15"/>
                                </svg>
                            </div>
                            <div>
                                <p class="text-sm font-semibold text-red-800">"Analysis failed"</p>
                                <p class="text-[12px] text-red-600 mt-1">
                                    {analysis.error_message.unwrap_or_else(|| "An unknown error occurred.".into())}
                                </p>
                            </div>
                        </div>
                    </div>
                }.into_any(),
                "complete" => {
                    if let Some(r) = analysis.result {
                        view! { <ResultView result=r/> }.into_any()
                    } else {
                        view! { <p class="text-sm text-gray-400">"Result data missing."</p> }.into_any()
                    }
                }
                _ => view! { <></> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ResultView(result: AnalysisResultView) -> impl IntoView {
    let pct = result.relative_effect * 100.0;
    let prob = result.probability_of_effect * 100.0;
    let sign = if result.cumulative_effect >= 0.0 { "+" } else { "" };

    let confidence_label = if prob >= 95.0 {
        ("High confidence", "text-green-700 bg-green-50")
    } else if prob >= 80.0 {
        ("Medium confidence", "text-yellow-700 bg-yellow-50")
    } else {
        ("Low confidence", "text-gray-600 bg-gray-100")
    };

    view! {
        <div class="space-y-5">
            // ── KPI row ───────────────────────────────────────────
            <div class="grid grid-cols-3 gap-4">
                <KpiCard
                    label="Cumulative effect"
                    value=format!(
                        "{}{:.0}",
                        sign,
                        result.cumulative_effect,
                    )
                    sub=format!(
                        "95% CI: {}{:.0} – {}{:.0}",
                        sign, result.cumulative_effect_lower,
                        sign, result.cumulative_effect_upper,
                    )
                    primary=true
                />
                <KpiCard
                    label="Relative effect"
                    value=format!("{}{:.1}%", if pct >= 0.0 { "+" } else { "" }, pct)
                    sub="vs. counterfactual baseline".to_string()
                    primary=false
                />
                <div class="bg-white rounded-2xl border border-gray-100 p-5">
                    <p class="text-[11px] font-semibold text-gray-400 uppercase tracking-wide">
                        "Probability of effect"
                    </p>
                    <p class="text-3xl font-bold text-gray-900 mt-2 font-mono">
                        {format!("{:.1}%", prob)}
                    </p>
                    <span class=format!(
                        "inline-flex items-center mt-2 text-[11px] font-semibold px-2 py-0.5 rounded-full {} {}",
                        confidence_label.1, "",
                    )>
                        {confidence_label.0}
                    </span>
                </div>
            </div>

            // ── Chart ─────────────────────────────────────────────
            {if !result.chart_actual.is_empty() {
                view! {
                    <EffectChart
                        actual=result.chart_actual
                        counterfactual=result.chart_counterfactual
                        intervention_index=result.intervention_index
                    />
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // ── Narrative ─────────────────────────────────────────
            <div class="bg-white rounded-2xl border border-gray-100 p-6">
                <p class="text-[11px] font-bold text-gray-400 uppercase tracking-widest mb-3">
                    "Interpretation"
                </p>
                <p class="text-[13px] text-gray-700 leading-relaxed">{result.narrative}</p>
            </div>
        </div>
    }
}

#[component]
fn EffectChart(
    actual: Vec<f64>,
    counterfactual: Vec<f64>,
    intervention_index: usize,
) -> impl IntoView {
    let n = actual.len();
    if n < 2 {
        return view! { <></> }.into_any();
    }

    let all: Vec<f64> = actual.iter().chain(counterfactual.iter()).copied().collect();
    let min_v = all.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = all.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let vw = 560.0_f64;
    let vh = 140.0_f64;
    let pad_x = 10.0_f64;
    let pad_y = 10.0_f64;
    let range = (max_v - min_v).max(1.0);

    let xi = |i: usize| pad_x + (i as f64) * (vw / (n - 1) as f64);
    let yi = |v: f64| pad_y + vh * (1.0 - (v - min_v) / range);

    let path = |series: &[f64]| -> String {
        series
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                if i == 0 {
                    format!("M {:.1} {:.1}", xi(i), yi(v))
                } else {
                    format!("L {:.1} {:.1}", xi(i), yi(v))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let actual_path = path(&actual);
    let cf_path = path(&counterfactual);
    let int_x = format!("{:.1}", xi(intervention_index));
    let vb = format!("0 0 {} {}", vw + pad_x * 2.0, vh + pad_y * 2.0);

    view! {
        <div class="bg-white rounded-2xl border border-gray-100 p-6">
            <p class="text-[11px] font-bold text-gray-400 uppercase tracking-widest mb-1">
                "Observed vs. Counterfactual"
            </p>
            <p class="text-[11px] text-gray-300 mb-5">
                "What happened (indigo) vs. what the model predicted without the intervention (orange dashed)"
            </p>

            <svg viewBox=vb class="w-full h-auto" style="max-height: 180px">
                // Intervention vertical line
                <line
                    x1=int_x.clone()
                    y1=format!("{:.1}", pad_y - 4.0)
                    x2=int_x
                    y2=format!("{:.1}", vh + pad_y + 4.0)
                    stroke="#e5e7eb"
                    stroke-width="1.5"
                    stroke-dasharray="4 3"
                />
                // Counterfactual line (orange, dashed)
                <path d=cf_path fill="none" stroke="#f97316" stroke-width="1.5" stroke-dasharray="5 3"/>
                // Actual line (indigo, solid)
                <path d=actual_path fill="none" stroke="#4f46e5" stroke-width="2"/>
            </svg>

            <div class="flex items-center gap-5 mt-3">
                <div class="flex items-center gap-2">
                    <div class="w-5 h-0.5 bg-indigo-600 rounded-full"/>
                    <span class="text-[11px] text-gray-400">"Observed"</span>
                </div>
                <div class="flex items-center gap-2">
                    <svg width="20" height="3" viewBox="0 0 20 3">
                        <line x1="0" y1="1.5" x2="20" y2="1.5" stroke="#f97316" stroke-width="1.5" stroke-dasharray="5 3"/>
                    </svg>
                    <span class="text-[11px] text-gray-400">"Counterfactual"</span>
                </div>
                <div class="flex items-center gap-2">
                    <svg width="20" height="3" viewBox="0 0 20 3">
                        <line x1="0" y1="1.5" x2="20" y2="1.5" stroke="#e5e7eb" stroke-width="1.5" stroke-dasharray="4 3"/>
                    </svg>
                    <span class="text-[11px] text-gray-400">"Intervention"</span>
                </div>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn KpiCard(
    label: &'static str,
    value: String,
    sub: String,
    primary: bool,
) -> impl IntoView {
    let outer = if primary {
        "bg-indigo-600 rounded-2xl p-5"
    } else {
        "bg-white rounded-2xl border border-gray-100 p-5"
    };
    let label_cls = if primary { "text-indigo-200" } else { "text-gray-400" };
    let value_cls = if primary { "text-white" } else { "text-gray-900" };
    let sub_cls = if primary { "text-indigo-300" } else { "text-gray-400" };
    view! {
        <div class=outer>
            <p class=format!("text-[11px] font-semibold uppercase tracking-wide {}", label_cls)>
                {label}
            </p>
            <p class=format!("text-3xl font-bold font-mono mt-2 {}", value_cls)>{value}</p>
            <p class=format!("text-[11px] mt-1 {}", sub_cls)>{sub}</p>
        </div>
    }
}

#[component]
fn MetaCard(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="bg-white rounded-2xl border border-gray-100 px-4 py-3.5">
            <p class="text-[10px] font-semibold text-gray-400 uppercase tracking-widest">{label}</p>
            <p class="text-[13px] font-semibold text-gray-900 mt-1 font-mono">{value}</p>
        </div>
    }
}

#[component]
fn AnalysisSkeleton() -> impl IntoView {
    view! {
        <Shell>
            <div class="px-8 py-7 space-y-5">
                <div class="h-5 bg-gray-200 rounded w-24 animate-pulse"/>
                <div class="h-8 bg-gray-200 rounded-xl w-80 animate-pulse"/>
                <div class="grid grid-cols-4 gap-4">
                    {(0..4)
                        .map(|_| view! { <div class="h-16 bg-gray-200 rounded-2xl animate-pulse"/> })
                        .collect_view()}
                </div>
                <div class="h-40 bg-gray-200 rounded-2xl animate-pulse"/>
                <div class="h-56 bg-gray-200 rounded-2xl animate-pulse"/>
            </div>
        </Shell>
    }
}