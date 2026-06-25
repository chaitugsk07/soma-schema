use leptos::prelude::*;
use leptos_router::{
    components::{Router, Routes, Route, A},
    path,
    hooks::use_location,
};
use soma_ui::{ThemeToggle, STYLES};
use crate::pages::{landing::LandingPage, explorer::ExplorerPage};

/// Read the pathname from the document's `<base href="...">` element, normalized
/// for use as a leptos_router base (no trailing slash; empty string for root `/`).
/// Falls back to `""` if there is no base element or it can't be read.
///
/// Trunk sets `<base href="/soma-schema/">` for GitHub Pages builds and
/// `<base href="/">` for local dev; the router base strips the trailing slash
/// so routes defined as `/` and `/explorer` match correctly under both.
pub(crate) fn router_base() -> std::borrow::Cow<'static, str> {
    let base = (|| -> Option<String> {
        let doc = document();
        let el = doc.query_selector("base").ok()??;
        // HtmlBaseElement.href() returns the resolved absolute URL; fall back to
        // the raw attribute (a root-relative path) if the cast isn't needed.
        let href = {
            use web_sys::wasm_bindgen::JsCast;
            if let Ok(base_el) = el.clone().dyn_into::<web_sys::HtmlBaseElement>() {
                base_el.href()
            } else {
                el.get_attribute("href").unwrap_or_default()
            }
        };
        // If Trunk injects an absolute URL (e.g. https://host/soma-schema/), extract
        // just the pathname; if it's already root-relative, use it directly.
        let path = if let Some(after) = href.strip_prefix("http://").or_else(|| href.strip_prefix("https://")) {
            // skip host, keep from the first '/' onward
            after.find('/').map(|i| after[i..].to_string()).unwrap_or_default()
        } else {
            href
        };
        // Strip trailing slash; a lone "/" becomes "".
        Some(path.trim_end_matches('/').to_string())
    })();
    std::borrow::Cow::Owned(base.unwrap_or_default())
}

/// Build a URL to a page in the separate static docs site.
///
/// `rel` is a path relative to the docs root, e.g. `""` or `"use-with-ai/"`.
/// On GitHub Pages this returns `/soma-schema/docs/<rel>`;
/// on local dev it returns `/docs/<rel>`.
pub(crate) fn docs_url(rel: &str) -> String {
    format!("{}/docs/{}", router_base(), rel)
}

#[component]
pub fn App() -> impl IntoView {
    let base = router_base();
    view! {
        <style>{STYLES}</style>
        <Router base=base>
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
                        <a href=docs_url("") class="footer-link">"Docs"</a>
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
                    <a href=docs_url("") class="nav-link">"Docs"</a>
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
                    <a href=docs_url("") class="nav-link">"Docs"</a>
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
