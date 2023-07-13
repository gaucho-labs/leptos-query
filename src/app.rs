use crate::{
    error_template::{AppError, ErrorTemplate},
    resource_cache::ResourceCache,
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);
    provide_post_cache(cx);

    view! {
        cx,

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/start-axum.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|cx| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { cx,
                <ErrorTemplate outside_errors/>
            }
            .into_view(cx)
        }>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! { cx, <HomePage/> }/>
                    <Route path="one" view=|cx| view! { cx, <PostOne/> }/>
                    <Route path="two" view=|cx| view! { cx, <PostTwo/> }/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(cx, 0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! { cx,
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
        <div>
            <a href="/one">"Post One"</a>
            <br/>
            <a href="/two">"Post two"</a>
        </div>

    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PostId(String);

pub fn provide_post_cache(cx: Scope) {
    let cache =
        ResourceCache::<PostId, String>::new(cx, |id| async move { get_post(id).await.unwrap() });

    provide_context(cx, cache);
}

#[server(GetPost, "/api")]
pub async fn get_post(id: PostId) -> Result<String, ServerFnError> {
    log!("fetching post: {:?}", id);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    Ok(format!("Post: {:?}", id))
}

pub fn use_post_cache(cx: Scope) -> ResourceCache<PostId, String> {
    use_context::<ResourceCache<PostId, String>>(cx).expect("No Post Cache")
}

#[component]
fn PostOne(cx: Scope) -> impl IntoView {
    let cache = use_post_cache(cx);
    let query = cache.get(PostId("one".into()));
    let signal = query.read(cx);

    view! { cx,
        <div>"ONE"</div>
            <Transition fallback=|| ()>
            {move || {
               signal().map(|post| view! { cx, <h1>{post}</h1> })
            }
        }
            </Transition>
    }
}

#[component]
fn PostTwo(cx: Scope) -> impl IntoView {
    let cache = use_post_cache(cx);
    let query = cache.get(PostId("two".into()));
    let signal = query.read(cx);

    view! { cx,
        <div>"TWO"</div>
            <Transition fallback=|| ()>
            {move || {
               signal().map(|post| view! { cx,
                <h1>{post}</h1>
            })
            }
        }
            </Transition>
    }
}
