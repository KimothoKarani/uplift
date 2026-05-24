use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

pub mod components;
pub mod pages;
pub mod server_utils;

// Called by uplift_api to render the full HTML document shell.
// LeptosOptions comes from the server's Leptos config.
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
                    href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap"
                />
                <script src="https://cdn.tailwindcss.com"></script>
                <script>
                    "tailwind.config = {
                        theme: {
                            extend: {
                                fontFamily: {
                                    sans: ['Inter', 'ui-sans-serif', 'system-ui'],
                                    mono: ['JetBrains Mono', 'ui-monospace'],
                                },
                                colors: {
                                    brand: {
                                        50:  '#eef2ff',
                                        100: '#e0e7ff',
                                        500: '#6366f1',
                                        600: '#4f46e5',
                                        700: '#4338ca',
                                    },
                                },
                            },
                        },
                    }"
                </script>
            </head>
            <body class="bg-gray-50 font-sans antialiased text-gray-900">
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
        <div class="min-h-screen bg-gray-50 flex flex-col items-center justify-center px-4">
            <p class="text-xs font-semibold tracking-widest text-brand-600 uppercase">
                "404"
            </p>
            <h1 class="mt-3 text-3xl font-bold tracking-tight text-gray-900">
                "Page not found"
            </h1>
            <p class="mt-2 text-base text-gray-500">
                "The page you're looking for doesn't exist."
            </p>
            <a
                href="/dashboard"
                class="mt-8 text-sm font-medium text-brand-600 hover:text-brand-700"
            >
                "← Back to dashboard"
            </a>
        </div>
    }
}