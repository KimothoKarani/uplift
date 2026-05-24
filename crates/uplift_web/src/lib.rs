use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

pub mod components;
pub mod pages;
pub mod server_utils;

pub fn shell(options: leptos::config::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link
                    rel="preconnect"
                    href="https://fonts.gstatic.com"
                    crossorigin=""
                />
                <link
                    rel="stylesheet"
                    href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap"
                />
                <script src="https://cdn.tailwindcss.com"></script>
                <script>
                    "tailwind.config = {
                        theme: {
                            extend: {
                                fontFamily: {
                                    sans: ['Inter', 'ui-sans-serif', 'system-ui'],
                                },
                                colors: {
                                    brand: {
                                        DEFAULT: '#1B3A3A',
                                        hover:   '#2E5E5E',
                                    },
                                    sidebar: '#F7F7F5',
                                    page:    '#EDECEA',
                                    card:    '#FFFFFF',
                                    border:  '#E5E2DC',
                                    ink: {
                                        DEFAULT: '#1A1917',
                                        muted:   '#6B6760',
                                        hint:    '#9B9790',
                                    },
                                    ok:     '#1B6B4E',
                                    warn:   '#B87316',
                                    danger: '#B83232',
                                    'ok-bg':     '#EEF4F1',
                                    'warn-bg':   '#FDF6E8',
                                    'danger-bg': '#FCEEED',
                                    accent: '#4C9BE8',
                                },
                            },
                        },
                    }"
                </script>
            </head>
            <body class="bg-page font-sans antialiased text-ink">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Title text="Uplift"/>
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route
                    path=path!("/")
                    view=|| view! { <Redirect path="/dashboard"/> }
                />
                <Route path=path!("/login")         view=pages::login::LoginPage/>
                <Route path=path!("/dashboard")     view=pages::dashboard::DashboardPage/>
                <Route path=path!("/analyses/new")  view=pages::new_analysis::NewAnalysisPage/>
                <Route path=path!("/analyses/:id")  view=pages::analysis_result::AnalysisResultPage/>
                <Route path=path!("/settings")      view=pages::settings::SettingsPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-sidebar flex flex-col items-center justify-center px-4">
            <p class="text-xs font-medium tracking-widest text-brand">
                "404"
            </p>
            <h1 class="mt-3 text-3xl font-medium tracking-tight text-ink">
                "Page not found"
            </h1>
            <p class="mt-2 text-base text-ink-hint">
                "This page does not exist."
            </p>
            <a
                href="/dashboard"
                class="mt-8 text-sm font-medium text-brand hover:text-brand-hover"
            >
                "Back to dashboard"
            </a>
        </div>
    }
}