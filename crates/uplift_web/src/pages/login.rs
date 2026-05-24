use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <Title text="Sign in — Uplift"/>
        <div class="min-h-screen bg-gray-50 flex flex-col justify-center py-12 px-4 sm:px-6 lg:px-8">

            <div class="sm:mx-auto sm:w-full sm:max-w-md">
                // Wordmark
                <div class="flex items-center justify-center gap-2 mb-2">
                    <span class="text-2xl font-bold tracking-tight text-gray-900">
                        "Uplift"
                    </span>
                </div>
                <p class="text-center text-sm text-gray-500 mb-8">
                    "Prove your marketing actually worked."
                </p>

                // Card
                <div class="bg-white border border-gray-200 rounded-xl px-8 py-10 shadow-sm">

                    <h1 class="text-xl font-semibold text-gray-900 mb-1">
                        "Sign in to your workspace"
                    </h1>
                    <p class="text-sm text-gray-500 mb-8">
                        "Connect your Google Analytics account to get started."
                    </p>

                    // Google OAuth button — links to our API route, not a JS click
                    <a
                        href="/api/auth/google"
                        class="flex items-center justify-center gap-3 w-full px-4 py-2.5
                               bg-white border border-gray-300 rounded-lg text-sm font-medium
                               text-gray-700 hover:bg-gray-50 hover:border-gray-400
                               transition-colors duration-150 shadow-sm"
                    >
                        // Google "G" logo inline SVG — exact brand colours
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 18 18"
                            aria-hidden="true"
                        >
                            <path
                                fill="#4285F4"
                                d="M16.51 8H8.98v3h4.3c-.18 1-.74 1.48-1.6 2.04v2.01h2.6a7.8 7.8 0 0 0 2.38-5.88c0-.57-.05-.66-.15-1.18z"
                            />
                            <path
                                fill="#34A853"
                                d="M8.98 17c2.16 0 3.97-.72 5.3-1.94l-2.6-2.01c-.72.48-1.63.76-2.7.76-2.07 0-3.83-1.4-4.46-3.28H1.88v2.07A8 8 0 0 0 8.98 17z"
                            />
                            <path
                                fill="#FBBC05"
                                d="M4.52 10.53c-.16-.48-.25-.99-.25-1.53s.09-1.05.25-1.53V5.4H1.88A8 8 0 0 0 1 9c0 1.29.31 2.51.88 3.6l2.64-2.07z"
                            />
                            <path
                                fill="#EA4335"
                                d="M8.98 3.58c1.16 0 2.2.4 3.02 1.18l2.26-2.26A8 8 0 0 0 8.98 1 8 8 0 0 0 1.88 5.4L4.52 7.47c.63-1.88 2.39-3.9 4.46-3.9z"
                            />
                        </svg>
                        "Continue with Google"
                    </a>

                    <p class="mt-6 text-xs text-center text-gray-400">
                        "By signing in you agree to our "
                        <a href="/terms" class="underline hover:text-gray-600">"Terms"</a>
                        " and "
                        <a href="/privacy" class="underline hover:text-gray-600">"Privacy Policy"</a>
                        "."
                    </p>
                </div>

                <p class="mt-6 text-center text-xs text-gray-400">
                    "Powered by Bayesian causal inference"
                </p>
            </div>

        </div>
    }
}