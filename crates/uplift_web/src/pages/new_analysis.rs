use leptos::prelude::*;
use leptos_meta::Title;
use leptos::form::ActionForm;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::nav::AppLayout;
use crate::server_utils::extract_session_id;

// ── Data types ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormData {
    pub org_name: String,
    pub user_name: String,
    pub user_email: String,
    pub properties: Vec<FormProperty>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormProperty {
    pub id: String,
    pub display_name: String,
}

// ── Server functions ───────────────────────────────────────────────────────

#[server]
pub async fn load_form_data() -> Result<FormData, ServerFnError> {
    use axum::http::HeaderMap;
    use leptos_axum::extract;
    use uplift_db::{OrgRepo, PgPool, PropertyRepo, SessionRepo, UserRepo};

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

    let properties = PropertyRepo::list_by_org(&pool, org.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let user_name = user
        .display_name
        .unwrap_or_else(|| user.email.split('@').next().unwrap_or("User").to_string());

    Ok(FormData {
        org_name: org.name,
        user_name,
        user_email: user.email,
        properties: properties
            .into_iter()
            .map(|p| FormProperty {
                id: p.id.to_string(),
                display_name: p.display_name,
            })
            .collect(),
    })
}

#[server]
pub async fn create_analysis(
    property_id: String,
    metric: String,
    description: String,
    intervention_date: String,
    pre_period_start: String,
    pre_period_end: String,
    post_period_start: String,
    post_period_end: String,
) -> Result<Uuid, ServerFnError> {
    use axum::http::HeaderMap;
    use chrono::NaiveDate;
    use leptos_axum::extract;
    use uplift_db::{AnalysisRepo, OrgRepo, PgPool, PropertyRepo, SessionRepo, UserRepo};
    use uplift_jobs::enqueue_run_analysis;

    let pool = expect_context::<PgPool>();
    let headers: HeaderMap = extract().await?;

    let session_id = extract_session_id(&headers)
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let session = SessionRepo::find_valid(&pool, session_id)
        .await
        .map_err(|_| ServerFnError::new("session expired"))?;

    let user = UserRepo::find_by_id(&pool, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let org = OrgRepo::find_by_id(&pool, user.organization_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let property_uuid = property_id
        .parse::<Uuid>()
        .map_err(|_| ServerFnError::new("invalid property selected"))?;

    // Ownership check — ensures users can't submit jobs for other orgs' properties
    PropertyRepo::find_by_id(&pool, property_uuid, org.id)
        .await
        .map_err(|_| ServerFnError::new("property not found"))?;

    let parse = |s: &str, field: &'static str| {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| ServerFnError::new(format!("{field}: invalid date")))
    };

    let pre_start  = parse(&pre_period_start,  "pre-period start")?;
    let pre_end    = parse(&pre_period_end,    "pre-period end")?;
    let int_date   = parse(&intervention_date, "intervention date")?;
    let post_start = parse(&post_period_start, "post-period start")?;
    let post_end   = parse(&post_period_end,   "post-period end")?;

    if pre_end >= int_date {
        return Err(ServerFnError::new(
            "Pre-period must end before the intervention date.",
        ));
    }
    if post_start > int_date {
        return Err(ServerFnError::new(
            "Post-period must start on or after the intervention date.",
        ));
    }
    if post_start > post_end {
        return Err(ServerFnError::new("Post-period start must be before its end."));
    }
    let pre_days = (pre_end - pre_start).num_days();
    if pre_days < 30 {
        return Err(ServerFnError::new(
            "Pre-period must cover at least 30 days for reliable results.",
        ));
    }

    let analysis = AnalysisRepo::create(
        &pool,
        org.id,
        property_uuid,
        &metric,
        int_date,
        pre_start,
        pre_end,
        post_start,
        post_end,
        &description,
        user.id,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    enqueue_run_analysis(&pool, uplift_jobs::run_analysis::RunAnalysisJob {
    analysis_id: analysis.id,
    org_id: org.id,
        })
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(analysis.id)
}

// ── Page component ─────────────────────────────────────────────────────────

#[component]
pub fn NewAnalysisPage() -> impl IntoView {
    let form_data = Resource::new(|| (), |_| load_form_data());

    view! {
        <Title text="New Analysis — Uplift"/>
        <Suspense fallback=FormSkeleton>
            {move || form_data.get().map(|result| match result {
                Err(_) => view! {
                    <div class="min-h-screen flex items-center justify-center">
                        <p class="text-sm text-gray-400">"Redirecting…"</p>
                    </div>
                }.into_any(),
                Ok(d) => view! { <AnalysisForm data=d/> }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn AnalysisForm(data: FormData) -> impl IntoView {
    let navigate      = use_navigate();
    let create_action = ServerAction::<CreateAnalysis>::new();

    Effect::new(move |_| {
        if let Some(Ok(id)) = create_action.value().get() {
            navigate(&format!("/analyses/{}", id), Default::default());
        }
    });

    let pending    = create_action.pending();
    let action_err = create_action.value();
    let properties = data.properties.clone();

    view! {
        <AppLayout
            org_name=data.org_name
            user_name=data.user_name
            user_email=data.user_email
        >
            // Breadcrumb
            <div class="mb-6">
                <a
                    href="/dashboard"
                    class="text-sm text-gray-500 hover:text-gray-700 transition-colors"
                >
                    "← Dashboard"
                </a>
            </div>

            <div class="mb-8">
                <h1 class="text-2xl font-bold tracking-tight text-gray-900">
                    "New Analysis"
                </h1>
                <p class="mt-1 text-sm text-gray-500">
                    "Measure the causal impact of a campaign, content change, or any intervention."
                </p>
            </div>

            // Server-side error banner
            {move || action_err.get().and_then(|r| r.err()).map(|e| view! {
                <div class="mb-6 rounded-lg bg-red-50 border border-red-200 px-4 py-3">
                    <p class="text-sm font-medium text-red-700">{e.to_string()}</p>
                </div>
            })}

            <ActionForm action=create_action>
                <div class="space-y-5">

                    // ── Section 1: What ────────────────────────────────────
                    <FormSection title="What are you measuring?">
                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                            <Field label="GA4 Property">
                                <select name="property_id" required class=SELECT_CLASS>
                                    <option value="">"Select a property…"</option>
                                    {properties.into_iter().map(|p| {
                                        let id = p.id.clone();
                                        view! {
                                            <option value=id>{p.display_name}</option>
                                        }
                                    }).collect_view()}
                                </select>
                            </Field>

                            <Field label="Metric">
                                <select name="metric" required class=SELECT_CLASS>
                                    <option value="sessions">"Sessions"</option>
                                    <option value="activeUsers">"Active Users"</option>
                                    <option value="newUsers">"New Users"</option>
                                    <option value="screenPageViews">"Page Views"</option>
                                    <option value="conversions">"Conversions"</option>
                                    <option value="totalRevenue">"Revenue"</option>
                                    <option value="engagementRate">"Engagement Rate"</option>
                                </select>
                            </Field>
                        </div>

                        <Field label="Description" optional=true>
                            <input
                                type="text"
                                name="description"
                                placeholder="e.g. Launched Google Ads campaign targeting UK users"
                                class=INPUT_CLASS
                            />
                        </Field>
                    </FormSection>

                    // ── Section 2: When ────────────────────────────────────
                    <FormSection title="When did the intervention start?">
                        <div class="max-w-xs">
                            <Field label="Intervention date">
                                <input
                                    type="date"
                                    name="intervention_date"
                                    required
                                    class=INPUT_CLASS
                                />
                            </Field>
                        </div>
                        <p class="text-xs text-gray-400 mt-1">
                            "The date your campaign launched, content went live, or change was made."
                        </p>
                    </FormSection>

                    // ── Section 3: Analysis window ─────────────────────────
                    <FormSection title="Define your analysis window">
                        <div class="space-y-4">
                            <div>
                                <p class="text-sm font-medium text-gray-700 mb-2">
                                    "Pre-period"
                                    <span class="ml-1.5 text-xs font-normal text-gray-400">
                                        "(baseline — before intervention)"
                                    </span>
                                </p>
                                <div class="flex items-center gap-3">
                                    <input
                                        type="date"
                                        name="pre_period_start"
                                        required
                                        class=DATE_CLASS
                                    />
                                    <span class="text-gray-400 text-sm shrink-0">"→"</span>
                                    <input
                                        type="date"
                                        name="pre_period_end"
                                        required
                                        class=DATE_CLASS
                                    />
                                </div>
                            </div>

                            <div>
                                <p class="text-sm font-medium text-gray-700 mb-2">
                                    "Post-period"
                                    <span class="ml-1.5 text-xs font-normal text-gray-400">
                                        "(evaluation — after intervention)"
                                    </span>
                                </p>
                                <div class="flex items-center gap-3">
                                    <input
                                        type="date"
                                        name="post_period_start"
                                        required
                                        class=DATE_CLASS
                                    />
                                    <span class="text-gray-400 text-sm shrink-0">"→"</span>
                                    <input
                                        type="date"
                                        name="post_period_end"
                                        required
                                        class=DATE_CLASS
                                    />
                                </div>
                            </div>
                        </div>

                        <div class="mt-4 rounded-lg bg-amber-50 border border-amber-200
                                    px-4 py-3">
                            <p class="text-xs text-amber-800 leading-relaxed">
                                <span class="font-semibold">"Tip: "</span>
                                "The pre-period must be at least 30 days. A 90-day baseline \
                                 gives more reliable causal estimates. The pre-period end must \
                                 come before the intervention date."
                            </p>
                        </div>
                    </FormSection>

                </div>

                // ── Submit row ─────────────────────────────────────────────
                <div class="mt-8 flex items-center justify-end gap-3">
                    <a
                        href="/dashboard"
                        class="px-4 py-2 text-sm font-medium text-gray-600
                               hover:text-gray-900 transition-colors"
                    >
                        "Cancel"
                    </a>
                    <button
                        type="submit"
                        prop:disabled=pending
                        class="inline-flex items-center gap-2 px-5 py-2 bg-brand-600
                               text-white text-sm font-medium rounded-lg shadow-sm
                               hover:bg-brand-700 disabled:opacity-50
                               disabled:cursor-not-allowed transition-colors"
                    >
                        {move || if pending.get() { "Queuing analysis…" } else { "Run Analysis →" }}
                    </button>
                </div>
            </ActionForm>
        </AppLayout>
    }
}

// ── Reusable form primitives ───────────────────────────────────────────────

const INPUT_CLASS: &str =
    "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm \
     text-gray-900 placeholder-gray-400 focus:border-brand-500 \
     focus:outline-none focus:ring-1 focus:ring-brand-500 transition-colors";

const SELECT_CLASS: &str =
    "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm \
     text-gray-900 focus:border-brand-500 focus:outline-none \
     focus:ring-1 focus:ring-brand-500 transition-colors";

const DATE_CLASS: &str =
    "rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm \
     text-gray-900 focus:border-brand-500 focus:outline-none \
     focus:ring-1 focus:ring-brand-500 transition-colors";

#[component]
fn FormSection(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="bg-white border border-gray-200 rounded-xl px-6 py-5">
            <h2 class="text-sm font-semibold text-gray-900 mb-5">{title}</h2>
            <div class="space-y-4">{children()}</div>
        </div>
    }
}

#[component]
fn Field(
    label: &'static str,
    #[prop(optional)] optional: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1.5">
                {label}
                {optional.then(|| view! {
                    <span class="ml-1 text-xs font-normal text-gray-400">"(optional)"</span>
                })}
            </label>
            {children()}
        </div>
    }
}

// ── Loading skeleton ───────────────────────────────────────────────────────

#[component]
fn FormSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="bg-white border-b border-gray-200 h-14"/>
            <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
                <div class="h-4 w-24 bg-gray-200 rounded animate-pulse mb-6"/>
                <div class="h-8 w-48 bg-gray-200 rounded animate-pulse mb-2"/>
                <div class="h-4 w-80 bg-gray-100 rounded animate-pulse mb-8"/>
                <div class="space-y-5">
                    {(0..3).map(|_| view! {
                        <div class="bg-white border border-gray-200 rounded-xl h-36
                                    animate-pulse"/>
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}