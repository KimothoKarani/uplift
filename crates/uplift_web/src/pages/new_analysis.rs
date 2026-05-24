use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_router::components::Redirect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::Shell;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyOption {
    pub id: Uuid,
    pub display_name: String,
    pub ga4_property_id: String,
}

#[server(LoadProperties)]
pub async fn load_properties() -> Result<Vec<PropertyOption>, ServerFnError> {
    use crate::server_utils::require_user;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::PropertyRepo;

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let props = PropertyRepo::list_by_org(&pool, user.organization_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(props
        .into_iter()
        .map(|p| PropertyOption {
            id: p.id,
            display_name: p.display_name,
            ga4_property_id: p.ga4_property_id,
        })
        .collect())
}

#[server(CreateAnalysis)]
pub async fn create_analysis(
    property_id: String,
    metric: String,
    intervention_date: String,
    pre_period_start: String,
    pre_period_end: String,
    post_period_start: String,
    post_period_end: String,
    description: String,
) -> Result<Uuid, ServerFnError> {
    use crate::server_utils::require_user;
    use chrono::NaiveDate;
    use leptos::context::use_context;
    use sqlx::PgPool;
    use uplift_db::{AnalysisRepo, PropertyRepo};
    use uplift_jobs::run_analysis::RunAnalysisJob;

    let user = require_user().await?;
    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("no db pool in context"))?;

    let parse_date = |s: &str| -> Result<NaiveDate, ServerFnError> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| ServerFnError::new(format!("invalid date '{s}'")))
    };

    let property_id: Uuid = property_id
        .parse()
        .map_err(|_| ServerFnError::new("invalid property id"))?;
    let intervention = parse_date(&intervention_date)?;
    let pre_start = parse_date(&pre_period_start)?;
    let pre_end = parse_date(&pre_period_end)?;
    let post_start = parse_date(&post_period_start)?;
    let post_end = parse_date(&post_period_end)?;

    if pre_start >= pre_end {
        return Err(ServerFnError::new("Pre-period start must be before pre-period end"));
    }
    if pre_end >= intervention {
        return Err(ServerFnError::new("Pre-period must end before the intervention date"));
    }
    if post_start > post_end {
        return Err(ServerFnError::new(
            "Post-period start must be before or equal to post-period end",
        ));
    }
    let pre_days = (pre_end - pre_start).num_days();
    if pre_days < 30 {
        return Err(ServerFnError::new(format!(
            "Pre-period must be at least 30 days (got {pre_days})"
        )));
    }

    PropertyRepo::find_by_id(&pool, property_id, user.organization_id)
        .await
        .map_err(|_| ServerFnError::new("property not found"))?;

    let analysis = AnalysisRepo::create(
        &pool,
        user.organization_id,
        property_id,
        &metric,
        intervention,
        pre_start,
        pre_end,
        post_start,
        post_end,
        &description,
        user.id,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    uplift_jobs::enqueue_run_analysis(
        &pool,
        RunAnalysisJob {
            analysis_id: analysis.id,
            org_id: user.organization_id,
        },
    )
    .await
    .map_err(|e: anyhow::Error| ServerFnError::new(e.to_string()))?;

    Ok(analysis.id)
}

#[component]
pub fn NewAnalysisPage() -> impl IntoView {
    let props = Resource::new(|| (), |_| load_properties());
    let action = ServerAction::<CreateAnalysis>::new();
    let created_id: RwSignal<Option<Uuid>> = RwSignal::new(None);
    let action_value = action.value();

    Effect::new(move |_| {
        if let Some(Ok(id)) = action_value.get() {
            created_id.set(Some(id));
        }
    });

    view! {
        <Shell>
            {move || created_id.get().map(|id| view! { <Redirect path=format!("/analyses/{id}") /> })}

            <div class="px-8 py-7 max-w-2xl">
                <div class="mb-7">
                    <a href="/dashboard" class="inline-flex items-center gap-1.5 text-[12px] text-gray-400 hover:text-gray-600 mb-4 transition-colors">
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                            <polyline points="15 18 9 12 15 6"/>
                        </svg>
                        "Back to dashboard"
                    </a>
                    <h1 class="text-2xl font-bold text-gray-900">"New Analysis"</h1>
                    <p class="text-sm text-gray-400 mt-1">
                        "The model fits on the pre-period and projects a counterfactual into the post-period to measure causal impact."
                    </p>
                </div>

                <Suspense fallback=|| view! {
                    <div class="bg-white rounded-2xl border border-gray-100 p-6">
                        <div class="space-y-4">
                            <div class="h-5 bg-gray-200 rounded w-20 animate-pulse"/>
                            <div class="h-10 bg-gray-100 rounded-xl animate-pulse"/>
                            <div class="h-5 bg-gray-200 rounded w-20 animate-pulse"/>
                            <div class="h-10 bg-gray-100 rounded-xl animate-pulse"/>
                        </div>
                    </div>
                }>
                    {move || {
                        props.get().map(|result| match result {
                            Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                            Ok(opts) if opts.is_empty() => view! { <NoPropertiesWarning/> }.into_any(),
                            Ok(opts) => view! { <AnalysisForm action=action props=opts/> }.into_any(),
                        })
                    }}
                </Suspense>
            </div>
        </Shell>
    }
}

#[component]
fn NoPropertiesWarning() -> impl IntoView {
    view! {
        <div class="bg-amber-50 border border-amber-200 rounded-2xl p-6 text-center">
            <p class="text-sm font-semibold text-amber-900">"No GA4 properties connected"</p>
            <p class="text-xs text-amber-700 mt-1">
                "Connect a Google Analytics 4 property before creating an analysis."
            </p>
            <a
                href="/auth/google"
                class="mt-4 inline-flex items-center gap-1.5 px-4 py-2 bg-amber-600 text-white text-xs font-semibold rounded-lg hover:bg-amber-700 transition-colors"
            >
                "Connect GA4 →"
            </a>
        </div>
    }
}

#[component]
fn AnalysisForm(
    action: ServerAction<CreateAnalysis>,
    props: Vec<PropertyOption>,
) -> impl IntoView {
    let pending = action.pending();
    let value = action.value();

    view! {
        <ActionForm action=action attr:class="bg-white rounded-2xl border border-gray-100 divide-y divide-gray-50">

            // ── Section: What to measure ───────────────────────────
            <div class="p-6 space-y-4">
                <p class="text-[11px] font-bold text-gray-400 uppercase tracking-widest">"What to measure"</p>

                <FormField label="GA4 Property">
                    <select
                        name="property_id"
                        required
                        class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 bg-white text-gray-900"
                    >
                        <option value="">"Select a property…"</option>
                        {props
                            .into_iter()
                            .map(|p| {
                                let id = p.id.to_string();
                                let label = format!("{} ({})", p.display_name, p.ga4_property_id);
                                view! { <option value=id>{label}</option> }
                            })
                            .collect_view()}
                    </select>
                </FormField>

                <FormField label="Metric">
                    <select
                        name="metric"
                        required
                        class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 bg-white text-gray-900"
                    >
                        <option value="sessions">"Sessions"</option>
                        <option value="activeUsers">"Active users"</option>
                        <option value="newUsers">"New users"</option>
                        <option value="screenPageViews">"Page views"</option>
                        <option value="conversions">"Conversions"</option>
                    </select>
                </FormField>

                <FormField label="Description">
                    <textarea
                        name="description"
                        required
                        rows="2"
                        placeholder="e.g. Launched redesigned checkout page — impact on sessions"
                        class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 resize-none text-gray-900 placeholder-gray-300"
                    />
                </FormField>
            </div>

            // ── Section: Dates ─────────────────────────────────────
            <div class="p-6 space-y-4">
                <p class="text-[11px] font-bold text-gray-400 uppercase tracking-widest">"Time windows"</p>

                <FormField label="Intervention date">
                    <input
                        type="date"
                        name="intervention_date"
                        required
                        class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 text-gray-900"
                    />
                    <p class="text-[11px] text-gray-400 mt-1">"The day the change or campaign started."</p>
                </FormField>

                <div class="grid grid-cols-2 gap-4">
                    <FormField label="Pre-period start">
                        <input
                            type="date"
                            name="pre_period_start"
                            required
                            class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 text-gray-900"
                        />
                    </FormField>
                    <FormField label="Pre-period end">
                        <input
                            type="date"
                            name="pre_period_end"
                            required
                            class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 text-gray-900"
                        />
                    </FormField>
                </div>
                <p class="text-[11px] text-gray-400 -mt-2">"Training window — at least 30 days recommended."</p>

                <div class="grid grid-cols-2 gap-4">
                    <FormField label="Post-period start">
                        <input
                            type="date"
                            name="post_period_start"
                            required
                            class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 text-gray-900"
                        />
                    </FormField>
                    <FormField label="Post-period end">
                        <input
                            type="date"
                            name="post_period_end"
                            required
                            class="w-full border border-gray-200 rounded-xl px-3.5 py-2.5 text-[13px] focus:outline-none focus:ring-2 focus:ring-indigo-500 text-gray-900"
                        />
                    </FormField>
                </div>
                <p class="text-[11px] text-gray-400 -mt-2">"Measurement window — the effect is summed over this period."</p>
            </div>

            // ── Submit ─────────────────────────────────────────────
            <div class="p-6 space-y-3">
                {move || {
                    value
                        .get()
                        .and_then(|r| r.err())
                        .map(|e| {
                            view! {
                                <div class="rounded-xl bg-red-50 border border-red-100 px-4 py-3 text-[12px] text-red-700">
                                    {e.to_string()}
                                </div>
                            }
                        })
                }}
                <button
                    type="submit"
                    disabled=pending
                    class="w-full py-3 px-4 bg-indigo-600 text-white text-[13px] font-semibold rounded-xl hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                    {move || {
                        if pending.get() {
                            "Fetching data and running model…"
                        } else {
                            "Run analysis"
                        }
                    }}
                </button>
            </div>
        </ActionForm>
    }
}

#[component]
fn FormField(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div>
            <label class="block text-[11px] font-semibold text-gray-500 uppercase tracking-wide mb-1.5">
                {label}
            </label>
            {children()}
        </div>
    }
}