use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// Full-page layout: fixed sidebar on the left, scrollable content on the right.
/// Every authenticated page uses this.
#[component]
pub fn Shell(children: Children) -> impl IntoView {
    view! {
        <div class="flex h-screen bg-gray-50 overflow-hidden">
            <Sidebar/>
            <div class="flex-1 overflow-y-auto min-w-0">
                {children()}
            </div>
        </div>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let location = use_location();
    let path = move || location.pathname.get();

    let is = move |prefix: &'static str| move || path().starts_with(prefix);

    view! {
        <aside class="w-56 flex-shrink-0 bg-white border-r border-gray-100 flex flex-col h-full">
            // ── Logo ──────────────────────────────────────────────
            <div class="px-5 py-5 flex items-center gap-2.5">
                <div class="w-7 h-7 bg-indigo-600 rounded-lg flex items-center justify-center flex-shrink-0">
                    <svg
                        class="w-4 h-4 text-white"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/>
                        <polyline points="17 6 23 6 23 12"/>
                    </svg>
                </div>
                <span class="text-[15px] font-bold text-gray-900">"Uplift"</span>
            </div>

            // ── Primary nav ───────────────────────────────────────
            <nav class="flex-1 px-3 pt-1 space-y-0.5 overflow-y-auto">
                <NavItem
                    href="/dashboard"
                    label="Dashboard"
                    active=Signal::derive(is("/dashboard"))
                >
                    <IconDashboard/>
                </NavItem>
                <NavItem
                    href="/analyses/new"
                    label="Analyses"
                    active=Signal::derive(is("/analyses"))
                >
                    <IconChart/>
                </NavItem>
                <NavItem
                    href="/settings"
                    label="Settings"
                    active=Signal::derive(is("/settings"))
                >
                    <IconSettings/>
                </NavItem>
            </nav>

            // ── Upgrade card ──────────────────────────────────────
            <div class="mx-3 mb-3 bg-indigo-50 rounded-2xl p-4">
                <p class="text-[13px] font-semibold text-indigo-900">"Upgrade to Pro"</p>
                <p class="text-[11px] text-indigo-500 mt-0.5 leading-relaxed">
                    "Get 1 month free and unlock unlimited analyses"
                </p>
                <button class="mt-3 w-full py-1.5 bg-indigo-600 text-white text-xs font-semibold rounded-lg hover:bg-indigo-700 transition-colors">
                    "Upgrade"
                </button>
            </div>

            // ── Bottom links ──────────────────────────────────────
            <div class="border-t border-gray-100 px-3 py-3 space-y-0.5">
                <a
                    href="#"
                    class="flex items-center gap-2.5 px-3 py-2 rounded-xl text-[13px] text-gray-400 hover:bg-gray-50 hover:text-gray-600 transition-colors"
                >
                    <IconHelp/>
                    "Help & information"
                </a>
                <form method="post" action="/auth/logout">
                    <button
                        type="submit"
                        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-[13px] text-gray-400 hover:bg-gray-50 hover:text-gray-600 transition-colors"
                    >
                        <IconLogout/>
                        "Log out"
                    </button>
                </form>
            </div>
        </aside>
    }
}

#[component]
fn NavItem(
    href: &'static str,
    label: &'static str,
    active: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let cls = move || {
        if active.get() {
            "flex items-center gap-2.5 px-3 py-2 rounded-xl text-[13px] font-semibold bg-gray-100 text-gray-900"
        } else {
            "flex items-center gap-2.5 px-3 py-2 rounded-xl text-[13px] text-gray-500 hover:bg-gray-50 hover:text-gray-800 transition-colors"
        }
    };
    view! {
        <a href=href class=cls>
            {children()}
            {label}
        </a>
    }
}

// ── Icons (20×20 heroicons style) ─────────────────────────────────────────────

#[component]
fn IconDashboard() -> impl IntoView {
    view! {
        <svg
            class="w-[18px] h-[18px] flex-shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <rect x="3" y="3" width="7" height="7" rx="1.5"/>
            <rect x="14" y="3" width="7" height="7" rx="1.5"/>
            <rect x="3" y="14" width="7" height="7" rx="1.5"/>
            <rect x="14" y="14" width="7" height="7" rx="1.5"/>
        </svg>
    }
}

#[component]
fn IconChart() -> impl IntoView {
    view! {
        <svg
            class="w-[18px] h-[18px] flex-shrink-0"
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
    }
}

#[component]
fn IconSettings() -> impl IntoView {
    view! {
        <svg
            class="w-[18px] h-[18px] flex-shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
    }
}

#[component]
fn IconHelp() -> impl IntoView {
    view! {
        <svg
            class="w-[16px] h-[16px] flex-shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <circle cx="12" cy="12" r="10"/>
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
            <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
    }
}

#[component]
fn IconLogout() -> impl IntoView {
    view! {
        <svg
            class="w-[16px] h-[16px] flex-shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>
            <polyline points="16 17 21 12 16 7"/>
            <line x1="21" y1="12" x2="9" y2="12"/>
        </svg>
    }
}
