use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_query::{QueryCache, QueryOptions, QueryState};
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);
    provide_post_cache(cx);

    view! { cx,
        <Stylesheet id="leptos" href="/pkg/start-axum.css"/>
        <Title text="Welcome to Leptos"/>
        <Router fallback=|cx| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { cx, <ErrorTemplate outside_errors/> }
                .into_view(cx)
        }>
            <main>
                <Routes>
                    <Route
                        path=""
                        view=|cx| {
                            view! { cx, <HomePage/> }
                        }
                    />
                    <Route
                        path="one"
                        view=|cx| {
                            view! { cx, <PostOne/> }
                        }
                    />
                    <Route
                        path="two"
                        view=|cx| {
                            view! { cx, <PostTwo/> }
                        }
                    />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    view! { cx,
        <div>
            <h1>"Welcome to Leptos Query!"</h1>
            <div id="simple" style:width="20rem" style:margin="auto">
                <p>"This is a simple example of using a query cache."</p>
                <p>"Each post has a stale_time of 5 seconds."</p>

                <h2>"Posts"</h2>
                <ul>
                    <li>
                        <a href="/one">"Post 1"</a>
                    </li>
                    <li>
                        <a href="/two">"Post 2"</a>
                    </li>
                </ul>
                <br/>
            </div>
            <div id="complex"></div>
        </div>
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PostId(String);

pub fn provide_post_cache(cx: Scope) {
    QueryCache::<PostId, String>::provide_resource_cache_with_options(
        cx,
        |id| async move { get_post(id).await.unwrap() },
        QueryOptions {
            default_value: None,
            stale_time: Some(std::time::Duration::from_millis(5000)),
        },
    );
}

#[server(GetPost, "/api")]
pub async fn get_post(id: PostId) -> Result<String, ServerFnError> {
    log!("fetching post: {:?}", id);
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
    Ok(format!("Post Number {:?}", id.0))
}

pub fn use_post_cache(cx: Scope) -> QueryCache<PostId, String> {
    use_context::<QueryCache<PostId, String>>(cx).expect("No Post Cache")
}

#[component]
fn PostOne(cx: Scope) -> impl IntoView {
    let cache = use_post_cache(cx);
    let query = cache.get(PostId("one".into()));

    view! { cx, <Post query/> }
}

#[component]
fn PostTwo(cx: Scope) -> impl IntoView {
    let cache = use_post_cache(cx);
    let query = cache.get(PostId("two".into()));

    view! { cx, <Post query/> }
}

#[component]
fn Post(cx: Scope, query: QueryState<PostId, String>) -> impl IntoView {
    let signal = query.read(cx);
    let loading = query.loading();
    let key = query.key().0;
    view! { cx,
        <div class="post">
            <h2>"Post Key: " {key}</h2>
            <div>
                <span>"Fetching Status: "</span>
                <span>{move || { if loading.get() { "Fetching..." } else { "Idle..." } }}</span>
            </div>
            <div class="post-body">
                <p>"Post Body"</p>
                <Transition fallback=move || {
                    view! { cx, <h2>"Loading..."</h2> }
                }>
                    {move || {
                        signal()
                            .map(|post| {
                                view! { cx, <h1>{post}</h1> }
                            })
                    }}
                </Transition>
            </div>
        </div>
    }
}
