use leptos::*;
use leptos_meta::*;
use leptos_query::{provide_query_client_with_options, DefaultQueryOptions};
use leptos_query_devtools::LeptosQueryDevtools;
use leptos_router::*;

// Modules
mod components;
mod layout;
mod pages;

// Top-Level pages
use crate::layout::Layout;
use crate::pages::home::Home;
use crate::pages::not_found::NotFound;

/// An app router which renders the homepage and handles 404's
#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_query_client_with_options(DefaultQueryOptions {
        resource_option: leptos_query::ResourceOption::Local,
        ..DefaultQueryOptions::default()
    });

    view! {
        <Html lang="en" dir="ltr" attr:data-theme="light" class=""/>

        // sets the document title
        <Title text="Welcome to Leptos CSR"/>

        // injects metadata in the <head> of the page
        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <LeptosQueryDevtools/>

        <ErrorBoundary fallback=|errors| {
            view! {
                <h1>"Uh oh! Something went wrong!"</h1>

                <p>"Errors: "</p>
                // Render a list of errors as strings - good for development purposes
                <ul>
                    {move || {
                        errors
                            .get()
                            .into_iter()
                            .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                            .collect_view()
                    }}

                </ul>
            }
        }>

            <Layout>
                <Router>
                    <Routes>
                        <Route path="/" view=Home/>
                        <Route path="/single" view=pages::single::SingleQuery/>
                        <Route path="/*" view=NotFound/>
                    </Routes>
                </Router>
            </Layout>

        </ErrorBoundary>
    }
}
