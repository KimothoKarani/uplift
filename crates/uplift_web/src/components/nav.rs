use leptos::prelude::*;
use leptos_router::components::A;

/// Top navigation bar. Receives user/org info as props — the page is
/// responsible for fetching these via a server function and passing them in.
#[component]
pub fn Nav(
    org_name: String,
    user_name: String,
    user_email: String,
) -> impl IntoView {
    // First two chars of the name for the avatar circle
    let initials = user_name
        .split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();

    view! {
        <header class="sticky top-0 z-40 bg-white border-b border-gray-200">
            <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
                <div class="flex h-14 items-center justify-between">

                    // ── Left: wordmark + org ──────────────────────────────
                    <div class="flex items-center gap-3">
                        <A
                            href="/dashboard"
                            attr:class="text-base font-bold tracking-tight text-gray-900 \
                                        hover:text-brand-600 transition-colors"
                        >
                            "Uplift"
                        </A>
                        <span class="text-gray-300 select-none">"/"</span>
                        <span class="text-sm font-medium text-gray-600 max-w-[160px] truncate">
                            {org_name}
                        </span>
                    </div>

                    // ── Centre: primary navigation ─────────────────────────
                    <nav class="hidden md:flex items-center gap-1">
                        <NavLink href="/dashboard">"Dashboard"</NavLink>
                        <NavLink href="/analyses/new">"New Analysis"</NavLink>
                    </nav>

                    // ── Right: user identity + actions ─────────────────────
                    <div class="flex items-center gap-3">
                        // Avatar circle
                        <div
                            class="flex items-center justify-center w-7 h-7 rounded-full \
                                   bg-brand-600 text-white text-xs font-semibold select-none"
                            title=user_email.clone()
                        >
                            {initials}
                        </div>

                        // Name — hidden on small screens
                        <span class="hidden sm:block text-sm font-medium text-gray-700 \
                                     max-w-[140px] truncate">
                            {user_name}
                        </span>

                        <span class="text-gray-200 select-none">"|"</span>

                        <a
                            href="/settings"
                            class="text-sm text-gray-500 hover:text-gray-900 transition-colors"
                        >
                            "Settings"
                        </a>

                        // Sign out — plain link to the API logout endpoint
                        <a
                            href="/api/auth/logout"
                            class="text-sm text-gray-500 hover:text-gray-900 transition-colors"
                        >
                            "Sign out"
                        </a>
                    </div>

                </div>
            </div>
        </header>
    }
}

/// A single nav link. Renders as plain text with underline on hover.
/// Active state styling will be added when leptos_router exposes
/// the active class API stably in 0.7.
#[component]
fn NavLink(href: &'static str, children: Children) -> impl IntoView {
    view! {
        <A
            href=href
            attr:class="px-3 py-1.5 text-sm font-medium text-gray-600 rounded-md \
                        hover:text-gray-900 hover:bg-gray-100 transition-colors"
        >
            {children()}
        </A>
    }
}

/// Page layout wrapper used by every authenticated page.
/// Composes Nav + a centred content area.
#[component]
pub fn AppLayout(
    org_name: String,
    user_name: String,
    user_email: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <Nav org_name user_name user_email/>
            <main class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
                {children()}
            </main>
        </div>
    }
}