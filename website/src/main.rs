mod app;
mod data;
mod pages;

fn main() {
    leptos::mount::mount_to_body(app::App);
}
