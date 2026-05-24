use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::nav::AppLayout;
use crate::server_utils::extract_session_id;

// ── Data types ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisPageData {
    pub org_name: String,
    pub user_name: String,
    pub user_email: String,
    pub info: AnalysisInfo,
    pub result: Option<ResultData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisInfo {
    pub property_name: String,
    pub metric: String,
    pub status: String,
    pub intervention_date: String,
    pub pre_period: String,
    pub post_period: String,
    pub description: String,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultData {
    pub relative_effect: String,
    pub relative_effect_ci: String,
    pub probability: String,
    pub cumulative_effect: String,
    pub cumulative_effect_ci: String,
    pub narrative: String,
    // [[timestamp_ms, value], ...] — pre-period actual from DB
    pub pre_points: Vec<[f64; 2]>,
    // [[timestamp_ms, actual, counterfactual, cf_lower, cf_upper], ...] — post-period
    pub post_points: Vec<[f64; 5]>,
    pub intervention_ts: f64,
}

// ── Server function ────────────────────────────────────────────────────────

#[server]
pub async fn load_analysis(id: String) -> Result<AnalysisPageData, ServerFnError> {
    use axum::http::HeaderMap;
    use leptos_axum::extract;
    use uplift_db::{
        AnalysisRepo, OrgRepo, PgPool, PropertyRepo, SessionRepo, TimeSeriesRepo, UserRepo,
    };

    let pool = expect_context::<PgPool>();
    let headers: HeaderMap = extract().await?;

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

    let user = UserRepo::find_by_id(&pool, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let org = OrgRepo::find_by_id(&pool, user.organization_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let analysis_id = id
        .parse::<Uuid>()
        .map_err(|_| ServerFnError::new("invalid analysis id"))?;

    let analysis = AnalysisRepo::find_by_id(&pool, analysis_id, org.id)
        .await
        .map_err(|_| ServerFnError::new("analysis not found"))?;

    let property = PropertyRepo::find_by_id(&pool, analysis.property_id, org.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let user_name = user
        .display_name
        .unwrap_or_else(|| user.email.split('@').next().unwrap_or("User").to_string());

    let info = AnalysisInfo {
        property_name: property.display_name,
        metric: fmt_metric(&analysis.metric),
        status: analysis.status.clone(),
        intervention_date: analysis.intervention_date.format("%b %d, %Y").to_string(),
        pre_period: format!(
            "{} – {}",
            analysis.pre_period_start.format("%b %d, %Y"),
            analysis.pre_period_end.format("%b %d, %Y"),
        ),
        post_period: format!(
            "{} – {}",
            analysis.post_period_start.format("%b %d, %Y"),
            analysis.post_period_end.format("%b %d, %Y"),
        ),
        description: analysis.description.clone(),
        error_message: analysis.error_message.clone(),
    };

    let result = if analysis.status == "complete" {
        match AnalysisRepo::get_result(&pool, analysis.id).await.ok() {
            None => None,
            Some(r) => {
                // Deserialise the stored pointwise JSON back to the core type
                let pointwise: Vec<uplift_core::impact::report::PointwiseEffect> =
                    serde_json::from_value(r.pointwise_effects)
                        .map_err(|e| ServerFnError::new(e.to_string()))?;

                // Pre-period actual values from the time-series cache
                let pre_ts = TimeSeriesRepo::get_range(
                    &pool,
                    analysis.property_id,
                    &analysis.metric,
                    analysis.pre_period_start,
                    analysis.pre_period_end,
                )
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

                let pre_points: Vec<[f64; 2]> = pre_ts
                    .points
                    .iter()
                    .map(|p| [p.timestamp.timestamp_millis() as f64, p.value])
                    .collect();

                // Post-period: actual + counterfactual + uncertainty band.
                // From the analysis code:
                //   effect_lower = effect - r_hi  →  cf_upper = actual - effect_lower
                //   effect_upper = effect - r_lo  →  cf_lower = actual - effect_upper
                let post_points: Vec<[f64; 5]> = pointwise
                    .iter()
                    .map(|p| {
                        let cf_upper = p.actual - p.effect_lower;
                        let cf_lower = p.actual - p.effect_upper;
                        [
                            p.timestamp.timestamp_millis() as f64,
                            p.actual,
                            p.counterfactual,
                            cf_lower,
                            cf_upper,
                        ]
                    })
                    .collect();

                let intervention_ts = analysis
                    .intervention_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp_millis() as f64;

                let pct = |v: f64| {
                    if v >= 0.0 {
                        format!("+{:.1}%", v * 100.0)
                    } else {
                        format!("{:.1}%", v * 100.0)
                    }
                };
                let num = |v: f64| {
                    if v >= 0.0 {
                        format!("+{:.0}", v)
                    } else {
                        format!("{:.0}", v)
                    }
                };

                Some(ResultData {
                    relative_effect: pct(r.relative_effect),
                    relative_effect_ci: format!(
                        "{} to {}",
                        pct(r.relative_effect_lower),
                        pct(r.relative_effect_upper),
                    ),
                    probability: format!("{:.0}%", r.probability_of_effect * 100.0),
                    cumulative_effect: num(r.cumulative_effect),
                    cumulative_effect_ci: format!(
                        "{} to {}",
                        num(r.cumulative_effect_lower),
                        num(r.cumulative_effect_upper),
                    ),
                    narrative: r.narrative,
                    pre_points,
                    post_points,
                    intervention_ts,
                })
            }
        }
    } else {
        None
    };

    Ok(AnalysisPageData {
        org_name: org.name,
        user_name,
        user_email: user.email,
        info,
        result,
    })
}

fn fmt_metric(raw: &str) -> String {
    match raw {
        "sessions"        => "Sessions",
        "activeUsers"     => "Active Users",
        "newUsers"        => "New Users",
        "screenPageViews" => "Page Views",
        "conversions"     => "Conversions",
        "totalRevenue"    => "Revenue",
        "engagementRate"  => "Engagement Rate",
        other             => other,
    }
    .to_string()
}

// ── Page component ─────────────────────────────────────────────────────────

#[component]
pub fn AnalysisResultPage() -> impl IntoView {
    let params = use_params_map();
    let analysis_id =
        move || params.with(|p| p.get("id").unwrap_or_default());

    let data = Resource::new(analysis_id, |id| load_analysis(id));

    view! {
        <Title text="Analysis — Uplift"/>
        <Suspense fallback=ResultSkeleton>
            {move || data.get().map(|r| match r {
                Err(_) => view! {
                    <div class="min-h-screen flex items-center justify-center">
                        <p class="text-sm text-gray-400">"Redirecting…"</p>
                    </div>
                }.into_any(),
                Ok(d) => view! { <ResultPage data=d/> }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn ResultPage(data: AnalysisPageData) -> impl IntoView {
    let status      = data.info.status.clone();
    let processing  = status == "pending" || status == "running";

    view! {
        // Auto-refresh while the job is running — no JS framework needed
        {processing.then(|| view! {
            <script>"setTimeout(()=>location.reload(),8000);"</script>
        })}

        <AppLayout
            org_name=data.org_name
            user_name=data.user_name
            user_email=data.user_email
        >
            // Breadcrumb
            <div class="mb-6">
                <a href="/dashboard"
                   class="text-sm text-gray-500 hover:text-gray-700 transition-colors">
                    "← Dashboard"
                </a>
            </div>

            // Header
            <div class="flex items-start justify-between mb-8">
                <div>
                    <div class="flex items-center gap-2 mb-1">
                        <span class="text-sm text-gray-500">{data.info.property_name.clone()}</span>
                        <span class="text-gray-300">"/"</span>
                        <span class="text-sm font-medium text-gray-700">{data.info.metric.clone()}</span>
                    </div>
                    <h1 class="text-2xl font-bold tracking-tight text-gray-900">
                        "Impact Analysis"
                    </h1>
                    {(!data.info.description.is_empty()).then(|| view! {
                        <p class="mt-1 text-sm text-gray-500">{data.info.description.clone()}</p>
                    })}
                </div>
                <StatusBadge status=status.clone()/>
            </div>

            {match status.as_str() {
                "pending" | "running" => view! {
                    <ProcessingCard status=status.clone()/>
                }.into_any(),
                "failed" => view! {
                    <FailedCard info=data.info.clone()/>
                }.into_any(),
                _ => match data.result {
                    Some(result) => view! {
                        <ResultContent info=data.info result/>
                    }.into_any(),
                    None => view! {
                        <ProcessingCard status="pending".into()/>
                    }.into_any(),
                },
            }}
        </AppLayout>
    }
}

// ── Status-specific cards ──────────────────────────────────────────────────

#[component]
fn ProcessingCard(status: String) -> impl IntoView {
    let (title, body) = if status == "running" {
        (
            "Running analysis…",
            "The Bayesian ITS model is fitting to your data. This usually takes 10–30 seconds.",
        )
    } else {
        (
            "Analysis queued",
            "Your analysis is waiting to be picked up by the worker. It will start shortly.",
        )
    };

    view! {
        <div class="bg-white border border-gray-200 rounded-xl p-12 text-center">
            <div class="flex justify-center mb-5">
                <div class="w-10 h-10 rounded-full border-[3px] border-brand-100
                            border-t-brand-600 animate-spin"/>
            </div>
            <p class="text-sm font-semibold text-gray-900">{title}</p>
            <p class="mt-1.5 text-sm text-gray-500 max-w-sm mx-auto">{body}</p>
            <p class="mt-5 text-xs text-gray-400">"Page refreshes automatically every 8 seconds."</p>
        </div>
    }
}

#[component]
fn FailedCard(info: AnalysisInfo) -> impl IntoView {
    view! {
        <div class="bg-red-50 border border-red-200 rounded-xl p-8">
            <p class="text-sm font-semibold text-red-800 mb-2">"Analysis failed"</p>
            {info.error_message.map(|msg| view! {
                <p class="text-xs text-red-700 font-mono bg-red-100 rounded px-3 py-2 mb-3">
                    {msg}
                </p>
            })}
            <p class="text-sm text-gray-600">
                "This is usually caused by insufficient data in the selected period. \
                 Try a longer pre-period (90+ days gives the most reliable results)."
            </p>
            <a href="/analyses/new"
               class="mt-5 inline-flex text-sm font-medium text-brand-600 hover:text-brand-700">
                "Run a new analysis →"
            </a>
        </div>
    }
}

// ── Complete result view ───────────────────────────────────────────────────

#[component]
fn ResultContent(info: AnalysisInfo, result: ResultData) -> impl IntoView {
    let positive = !result.relative_effect.starts_with('-');

    view! {
        // Chart
        <div class="bg-white border border-gray-200 rounded-xl p-6 mb-6">
            <div class="flex items-center justify-between mb-5">
                <h2 class="text-sm font-semibold text-gray-900">"Time Series"</h2>
                <div class="flex items-center gap-5 text-xs text-gray-500">
                    <span class="flex items-center gap-1.5">
                        <svg width="20" height="2"><line x1="0" y1="1" x2="20" y2="1"
                            stroke="#4f46e5" stroke-width="2"/></svg>
                        "Observed"
                    </span>
                    <span class="flex items-center gap-1.5">
                        <svg width="20" height="2"><line x1="0" y1="1" x2="20" y2="1"
                            stroke="#a5b4fc" stroke-width="1.5" stroke-dasharray="4,2"/></svg>
                        "Counterfactual"
                    </span>
                    <span class="flex items-center gap-1.5">
                        <span class="inline-block w-5 h-3 bg-indigo-200 opacity-50 rounded-sm"/>
                        "95% CI"
                    </span>
                </div>
            </div>
            <ImpactChart
                pre_points=result.pre_points
                post_points=result.post_points
                intervention_ts=result.intervention_ts
            />
        </div>

        // Stat cards
        <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
            <StatCard
                label="Relative Effect"
                value=result.relative_effect.clone()
                sub=result.relative_effect_ci.clone()
                highlight=positive
            />
            <StatCard
                label="Probability"
                value=result.probability.clone()
                sub="causal effect exists".into()
                highlight=true
            />
            <StatCard
                label="Cumulative Effect"
                value=result.cumulative_effect.clone()
                sub=result.cumulative_effect_ci.clone()
                highlight=positive
            />
            <StatCard
                label="Intervention"
                value=info.intervention_date.clone()
                sub=info.post_period.clone()
                highlight=false
            />
        </div>

        // Narrative
        <div class="bg-white border border-gray-200 rounded-xl p-6 mb-6">
            <h2 class="text-sm font-semibold text-gray-900 mb-3">"Summary"</h2>
            <p class="text-sm text-gray-600 leading-7">{result.narrative}</p>
        </div>

        // Details
        <div class="bg-white border border-gray-200 rounded-xl p-6">
            <h2 class="text-sm font-semibold text-gray-900 mb-4">"Analysis Details"</h2>
            <dl class="grid grid-cols-2 sm:grid-cols-3 gap-y-4 gap-x-6">
                <Detail label="Property"     value=info.property_name/>
                <Detail label="Metric"       value=info.metric/>
                <Detail label="Pre-period"   value=info.pre_period/>
                <Detail label="Post-period"  value=info.post_period/>
                <Detail label="Intervention" value=info.intervention_date/>
                <Detail label="Model"        value="ITS v1 — Bayesian Bootstrap".into()/>
            </dl>
        </div>
    }
}

// ── SVG Impact Chart ───────────────────────────────────────────────────────

#[component]
fn ImpactChart(
    pre_points: Vec<[f64; 2]>,
    post_points: Vec<[f64; 5]>,
    intervention_ts: f64,
) -> impl IntoView {
    const W: f64 = 800.0;
    const H: f64 = 260.0;
    const PL: f64 = 54.0; // left padding (Y labels)
    const PR: f64 = 16.0;
    const PT: f64 = 14.0;
    const PB: f64 = 28.0;
    const PW: f64 = W - PL - PR;
    const PH: f64 = H - PT - PB;

    // X range
    let t_min = pre_points
        .first()
        .map(|p| p[0])
        .or_else(|| post_points.first().map(|p| p[0]))
        .unwrap_or(0.0);
    let t_max = post_points
        .last()
        .map(|p| p[0])
        .or_else(|| pre_points.last().map(|p| p[0]))
        .unwrap_or(t_min + 1.0);

    // Y range — include actual + band extremes
    let all_y: Vec<f64> = pre_points
        .iter()
        .map(|p| p[1])
        .chain(post_points.iter().flat_map(|p| [p[1], p[2], p[3], p[4]]))
        .collect();
    let v_min = all_y.iter().cloned().fold(f64::INFINITY, f64::min);
    let v_max = all_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span  = (v_max - v_min).max(1.0);
    let v_lo  = v_min - span * 0.06;
    let v_hi  = v_max + span * 0.06;

    let tx = |t: f64| PL + (t - t_min) / (t_max - t_min) * PW;
    let ty = |v: f64| PT + (1.0 - (v - v_lo) / (v_hi - v_lo)) * PH;

    let int_x = tx(intervention_ts);

    // Pre-period actual
    let pre_d = line_path(pre_points.iter().map(|p| (tx(p[0]), ty(p[1]))));

    // Last pre-period point — used to connect lines at the intervention boundary
    let last_pre = pre_points.last().map(|p| (tx(p[0]), ty(p[1])));

    // Post-period actual — starts from last pre point for visual continuity
    let post_actual_d = line_path(
        last_pre.into_iter()
            .chain(post_points.iter().map(|p| (tx(p[0]), ty(p[1])))),
    );

    // Counterfactual — also starts from last pre point
    let cf_d = line_path(
        last_pre.into_iter()
            .chain(post_points.iter().map(|p| (tx(p[0]), ty(p[2])))),
    );

    // Confidence band polygon
    let band_d = if post_points.is_empty() {
        String::new()
    } else {
        let upper: Vec<_> = post_points.iter().map(|p| (tx(p[0]), ty(p[4]))).collect();
        let lower: Vec<_> = post_points.iter().map(|p| (tx(p[0]), ty(p[3]))).collect();
        let mut d = format!("M {:.1},{:.1}", upper[0].0, upper[0].1);
        for (x, y) in &upper[1..] { d.push_str(&format!(" L {x:.1},{y:.1}")); }
        for (x, y) in lower.iter().rev() { d.push_str(&format!(" L {x:.1},{y:.1}")); }
        d + " Z"
    };

    // Y axis — 5 evenly spaced ticks
    let y_ticks: Vec<(f64, String)> = (0..=4)
        .map(|i| {
            let v = v_lo + (v_hi - v_lo) * i as f64 / 4.0;
            (ty(v), abbrev(v))
        })
        .collect();

    view! {
        <svg
            viewBox=format!("0 0 {W} {H}")
            class="w-full"
            aria-label="Causal impact time series"
        >
            // Grid lines + Y labels
            {y_ticks.iter().map(|(yp, label)| {
                let yp = *yp;
                let label = label.clone();
                view! {
                    <line x1=PL y1=yp x2={W - PR} y2=yp
                          stroke="#f3f4f6" stroke-width="1"/>
                    <text x={PL - 5.0} y={yp + 3.5}
                          text-anchor="end" font-size="10" fill="#9ca3af">
                        {label}
                    </text>
                }
            }).collect_view()}

            // Tinted pre-period area
            <rect x=PL y=PT width={int_x - PL} height=PH fill="#f9fafb"/>

            // Confidence band
            {(!band_d.is_empty()).then(|| view! {
                <path d=band_d fill="#c7d2fe" opacity="0.40"/>
            })}

            // Pre-period actual (gray)
            {(!pre_d.is_empty()).then(|| view! {
                <path d=pre_d fill="none" stroke="#9ca3af" stroke-width="1.5"/>
            })}

            // Counterfactual dashed (indigo-300)
            {(!cf_d.is_empty()).then(|| view! {
                <path d=cf_d fill="none" stroke="#a5b4fc" stroke-width="1.5"
                      stroke-dasharray="5,3"/>
            })}

            // Post-period actual (indigo-600) — on top
            {(!post_actual_d.is_empty()).then(|| view! {
                <path d=post_actual_d fill="none" stroke="#4f46e5" stroke-width="2"/>
            })}

            // Intervention marker
            <line x1=int_x y1=PT x2=int_x y2={PT + PH}
                  stroke="#374151" stroke-width="1" stroke-dasharray="4,3"/>
            <text x={int_x + 4.0} y={PT + 11.0} font-size="9" fill="#6b7280">
                "Intervention"
            </text>

            // X baseline
            <line x1=PL y1={PT + PH} x2={W - PR} y2={PT + PH}
                  stroke="#e5e7eb" stroke-width="1"/>
        </svg>
    }
}

fn line_path(mut pts: impl Iterator<Item = (f64, f64)>) -> String {
    let Some((x0, y0)) = pts.next() else {
        return String::new();
    };
    let mut d = format!("M {x0:.1},{y0:.1}");
    for (x, y) in pts {
        d.push_str(&format!(" L {x:.1},{y:.1}"));
    }
    d
}

fn abbrev(v: f64) -> String {
    let a = v.abs();
    if a >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if a >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        format!("{:.0}", v)
    }
}

// ── Shared sub-components ──────────────────────────────────────────────────

#[component]
fn StatCard(
    label: &'static str,
    value: String,
    sub: String,
    highlight: bool,
) -> impl IntoView {
    let value_class = if highlight {
        "text-green-700"
    } else {
        "text-gray-900"
    };
    view! {
        <div class="bg-white border border-gray-200 rounded-xl p-5">
            <p class="text-xs font-medium text-gray-500 uppercase tracking-widest mb-2">
                {label}
            </p>
            <p class=format!("text-2xl font-bold font-mono {value_class}")>
                {value}
            </p>
            <p class="mt-1 text-xs text-gray-400">{sub}</p>
        </div>
    }
}

#[component]
fn Detail(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div>
            <dt class="text-xs text-gray-500 mb-0.5">{label}</dt>
            <dd class="text-sm font-medium text-gray-900">{value}</dd>
        </div>
    }
}

#[component]
fn StatusBadge(status: String) -> impl IntoView {
    let (ring, text, dot) = match status.as_str() {
        "complete" => ("bg-green-50 border-green-200", "text-green-700", "bg-green-500"),
        "running"  => ("bg-blue-50 border-blue-200",   "text-blue-700",  "bg-blue-400"),
        "failed"   => ("bg-red-50 border-red-200",     "text-red-700",   "bg-red-500"),
        _          => ("bg-gray-100 border-gray-200",  "text-gray-600",  "bg-gray-400"),
    };
    view! {
        <span class=format!(
            "inline-flex items-center gap-1.5 px-3 py-1 rounded-full \
             text-xs font-medium border {ring} {text}"
        )>
            <span class=format!("w-1.5 h-1.5 rounded-full {dot}")/>
            {status}
        </span>
    }
}

// ── Loading skeleton ───────────────────────────────────────────────────────

#[component]
fn ResultSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="bg-white border-b border-gray-200 h-14"/>
            <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
                <div class="h-4 w-24 bg-gray-200 rounded animate-pulse mb-6"/>
                <div class="h-8 w-56 bg-gray-200 rounded animate-pulse mb-8"/>
                <div class="bg-white border border-gray-200 rounded-xl h-72
                            animate-pulse mb-6"/>
                <div class="grid grid-cols-4 gap-4 mb-6">
                    {(0..4).map(|_| view! {
                        <div class="bg-white border border-gray-200 rounded-xl
                                    h-24 animate-pulse"/>
                    }).collect_view()}
                </div>
                <div class="bg-white border border-gray-200 rounded-xl h-40
                            animate-pulse"/>
            </div>
        </div>
    }
}