use leptos::prelude::*;
use leptos_router::{
    components::{Router, Routes, Route, A},
    path,
    hooks::use_location,
};
use soma_ui::ThemeToggle;
use crate::pages::{landing::LandingPage, explorer::ExplorerPage};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="flex flex-col min-h-screen page-atmosphere">
                <Nav />
                <main class="flex-1">
                    <Routes fallback=|| view! { <p class="text-muted-foreground p-8">"Not found"</p> }>
                        <Route path=path!("/") view=LandingPage />
                        <Route path=path!("/explorer") view=ExplorerPage />
                    </Routes>
                </main>
                <SiteFooter />
            </div>
        </Router>
    }
}

#[component]
fn SiteFooter() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <div class="landing-container">
                <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6">
                    <div>
                        <div class="flex items-center gap-2 mb-1">
                            <span class="nav-logo-mark">"ss"</span>
                            <span class="footer-logo">"soma-schema"</span>
                        </div>
                        <p class="footer-tagline">
                            "Plain-SQL Postgres migrations with manifest ordering and drift detection."
                        </p>
                    </div>
                    <div class="flex items-center gap-5">
                        <a
                            href="https://github.com/chaitugsk07/soma-schema"
                            target="_blank"
                            rel="noopener noreferrer"
                            class="footer-link"
                        >
                            "GitHub"
                        </a>
                        <a href="/docs/" class="footer-link">"Docs"</a>
                        <a
                            href="https://crates.io/crates/soma-schema"
                            target="_blank"
                            rel="noopener noreferrer"
                            class="footer-link"
                        >
                            "crates.io"
                        </a>
                    </div>
                </div>
                <div class="mt-6 pt-4 border-t border-border/40">
                    <span class="footer-copy">"© 2026 soma-schema · Apache-2.0"</span>
                </div>
            </div>
        </footer>
    }
}

#[component]
fn NavLink(href: &'static str, label: &'static str) -> impl IntoView {
    let location = use_location();
    view! {
        <A
            href=href
            attr:class=move || {
                let path = location.pathname.get();
                let is_active = if href == "/" {
                    path == "/"
                } else {
                    path.starts_with(href)
                };
                if is_active { "nav-link active" } else { "nav-link" }
            }
        >
            {label}
        </A>
    }
}

#[component]
fn Nav() -> impl IntoView {
    let menu_open = RwSignal::new(false);
    view! {
        <header class="site-nav">
            <div class="landing-container flex items-center justify-between py-3">
                <A href="/" attr:class="nav-logo">
                    <span class="nav-logo-mark" aria-hidden="true">"ss"</span>
                    "soma-schema"
                </A>
                // Desktop nav
                <nav class="hidden sm:flex items-center gap-5" aria-label="Main navigation">
                    <NavLink href="/" label="Home" />
                    <NavLink href="/explorer" label="Explorer" />
                    <a
                        href="https://github.com/chaitugsk07/soma-schema"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="nav-link"
                        aria-label="GitHub repository"
                    >
                        "GitHub"
                    </a>
                    <ThemeToggle />
                </nav>
                // Mobile hamburger
                <button
                    class="sm:hidden nav-hamburger"
                    aria-label="Toggle navigation menu"
                    attr:aria-expanded=move || if menu_open.get() { "true" } else { "false" }
                    on:click=move |_| menu_open.update(|v| *v = !*v)
                >
                    {move || if menu_open.get() {
                        view! { <span aria-hidden="true">"✕"</span> }.into_any()
                    } else {
                        view! { <span aria-hidden="true">"☰"</span> }.into_any()
                    }}
                </button>
            </div>
            // Mobile dropdown
            {move || menu_open.get().then(|| view! {
                <nav class="nav-mobile-drawer sm:hidden" aria-label="Mobile navigation">
                    <NavLink href="/" label="Home" />
                    <NavLink href="/explorer" label="Explorer" />
                    <a
                        href="https://github.com/chaitugsk07/soma-schema"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="nav-link"
                        aria-label="GitHub repository"
                    >
                        "GitHub"
                    </a>
                    <div class="mt-2"><ThemeToggle /></div>
                </nav>
            })}
        </header>
    }
}
