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
        <Link href="InterVariable.ttf" rel="preload" as_="font" crossorigin="anonymous" />

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
                <Routes>
                    <Route path="" view={home::HomePage}/>
                    <Route path="todo" view={todo::TodoPage}/>
                </Routes>
            </main>
        </Router>
    }
}
