use leptos::prelude::*;

#[component]
pub fn StatCard(label: String, value: String, #[prop(optional)] sub: Option<String>) -> impl IntoView {
    view! {
        <div class="bg-white border border-gray-200 rounded-xl p-5">
            <p class="text-xs font-medium text-gray-500 uppercase tracking-widest">{label}</p>
            <p class="mt-2 text-2xl font-bold font-mono text-gray-900">{value}</p>
            {sub.map(|s| view! {
                <p class="mt-1 text-xs text-gray-400">{s}</p>
            })}
        </div>
    }
}