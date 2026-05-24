#![recursion_limit = "512"]

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

pub mod components;
pub mod pages;
pub mod server_utils;

use pages::{
    analyses::AnalysisPage,
    dashboard::DashboardPage,
    login::LoginPage,
    new_analysis::NewAnalysisPage,
    settings::SettingsPage,
};

pub fn shell(options: leptos_config::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous"/>
                <link
                    href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap"
                    rel="stylesheet"
                />
                <script src="https://cdn.tailwindcss.com"/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body class="bg-gray-50 text-gray-900" style="font-family: 'Inter', sans-serif">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFoundPage/> }>
                <Route path=path!("/") view=|| view! { <Redirect path="/dashboard"/> }/>
                <Route path=path!("/login") view=LoginPage/>
                <Route path=path!("/dashboard") view=DashboardPage/>
                <Route path=path!("/analyses/new") view=NewAnalysisPage/>
                <Route path=path!("/analyses/:id") view=AnalysisPage/>
                <Route path=path!("/settings") view=SettingsPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50 flex items-center justify-center">
            <div class="text-center">
                <p class="text-4xl font-bold text-gray-300">"404"</p>
                <p class="text-gray-500 mt-2 text-sm">"Page not found"</p>
                <a href="/dashboard" class="mt-4 inline-block text-sm text-indigo-600 hover:underline">
                    "Go to dashboard →"
                </a>
            </div>
        </div>
    }
}
