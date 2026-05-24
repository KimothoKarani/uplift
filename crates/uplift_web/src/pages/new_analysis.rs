use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::Redirect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyOption {
    pub id: Uuid,
    pub display_name: String,
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
        return Err(ServerFnError::new("Post-period start must be before or equal to post-period end"));
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
        <Title text="New Analysis — Uplift"/>
        {move || created_id.get().map(|id| view! { <Redirect path=format!("/analyses/{id}") /> })}
        <div class="min-h-screen bg-gray-50">
            <nav class="bg-white border-b border-gray-200">
                <div class="max-w-3xl mx-auto px-4 sm:px-6 flex items-center justify-between h-14">
                    <span class="text-lg font-bold text-indigo-600">"Uplift"</span>
                    <a href="/dashboard" class="text-sm text-gray-500 hover:text-gray-900">
                        "← Dashboard"
                    </a>
                </div>
            </nav>

            <main class="max-w-3xl mx-auto px-4 sm:px-6 py-8">
                <h1 class="text-2xl font-bold text-gray-900 mb-1">"New Analysis"</h1>
                <p class="text-sm text-gray-500 mb-6">
                    "Measure the causal impact of an intervention on a GA4 metric using Bayesian structural time series."
                </p>

                <Suspense fallback=|| view! { <div class="h-8 bg-gray-200 rounded w-40 animate-pulse"/> }>
                    {move || {
                        props.get().map(|result| match result {
                            Err(_) => view! { <Redirect path="/login"/> }.into_any(),
                            Ok(opts) => view! { <AnalysisForm action=action props=opts/> }.into_any(),
                        })
                    }}
                </Suspense>
            </main>
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
        <ActionForm action=action attr:class="bg-white rounded-xl border border-gray-200 p-6 space-y-5">
            <FormField label="Property">
                <select
                    name="property_id"
                    required
                    class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                >
                    <option value="">"Select a property…"</option>
                    {props
                        .into_iter()
                        .map(|p| {
                            let id = p.id.to_string();
                            view! { <option value=id>{p.display_name}</option> }
                        })
                        .collect_view()}
                </select>
            </FormField>

            <FormField label="Metric">
                <select
                    name="metric"
                    required
                    class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                >
                    <option value="sessions">"Sessions"</option>
                    <option value="activeUsers">"Active users"</option>
                    <option value="newUsers">"New users"</option>
                    <option value="screenPageViews">"Page views"</option>
                    <option value="conversions">"Conversions"</option>
                </select>
            </FormField>

            <FormField label="Intervention date">
                <input
                    type="date"
                    name="intervention_date"
                    required
                    class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
            </FormField>

            <div class="grid grid-cols-2 gap-4">
                <FormField label="Pre-period start">
                    <input
                        type="date"
                        name="pre_period_start"
                        required
                        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                    />
                </FormField>
                <FormField label="Pre-period end">
                    <input
                        type="date"
                        name="pre_period_end"
                        required
                        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                    />
                </FormField>
            </div>

            <div class="grid grid-cols-2 gap-4">
                <FormField label="Post-period start">
                    <input
                        type="date"
                        name="post_period_start"
                        required
                        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                    />
                </FormField>
                <FormField label="Post-period end">
                    <input
                        type="date"
                        name="post_period_end"
                        required
                        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                    />
                </FormField>
            </div>

            <FormField label="Description">
                <textarea
                    name="description"
                    required
                    rows="3"
                    placeholder="e.g. Launched redesigned checkout — measuring impact on sessions"
                    class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500 resize-none"
                />
            </FormField>

            {move || {
                value
                    .get()
                    .and_then(|r| r.err())
                    .map(|e| {
                        view! {
                            <div class="rounded-lg bg-red-50 border border-red-200 px-4 py-3 text-sm text-red-700">
                                {e.to_string()}
                            </div>
                        }
                    })
            }}

            <button
                type="submit"
                disabled=pending
                class="w-full py-2.5 px-4 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
                {move || if pending.get() { "Running…" } else { "Run analysis" }}
            </button>
        </ActionForm>
    }
}

#[component]
fn FormField(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div>
            <label class="block text-xs font-medium text-gray-600 uppercase tracking-wide mb-1">
                {label}
            </label>
            {children()}
        </div>
    }
}
