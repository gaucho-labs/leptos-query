pub mod home;
pub mod todo;

use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_query::provide_query_client;
use leptos_query_devtools::LeptosQueryDevtools;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_query_client();

    view! {
        <Stylesheet id="leptos" href="/pkg/query-demo.css"/>

        // add inter font
        // <Link href="InterVariable.ttf" rel="preload" as_="font" crossorigin="anonymous" />

        // sets the document title
        <Title text="Welcome to Leptos"/>

        <LeptosQueryDevtools/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main class="bg-background text-foreground font-sans h-screen">
                <Header/>
                <Routes>
                    <Route path="" view=home::HomePage/>
                    <Route path="todo" view=todo::TodoPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <nav class="flex items-center justify-between p-6 lg:px-8" aria-label="Global">
            <div class="flex lg:flex-1">
                <a href="/" class="-m-1.5 p-1.5 w-">
                    <span class="sr-only">Leptos Query Demo</span>
                    <div inner_html=include_str!("../../../../logo.svg") class="w-8 h-8"></div>
                </a>
            </div>
            <div class="flex gap-x-12">
                <a href="#" class="text-sm font-semibold leading-6 text-gray-900">
                    Product
                </a>
                <a href="#" class="text-sm font-semibold leading-6 text-gray-900">
                    Features
                </a>
                <a href="#" class="text-sm font-semibold leading-6 text-gray-900">
                    Marketplace
                </a>
                <a href="#" class="text-sm font-semibold leading-6 text-gray-900">
                    Company
                </a>
            </div>
        // <div class="hidden lg:flex lg:flex-1 lg:justify-end">
        // <a href="#" class="text-sm font-semibold leading-6 text-gray-900">Log in</a>
        // </div>
        </nav>
    }
}
