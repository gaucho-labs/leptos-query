use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_query::*;
use leptos_router::{Outlet, Route, Router, Routes};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);
    // Provides Query Client for entire app.
    provide_query_client(cx);

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
                            view! { cx,
                                <div id="simple" style:width="50rem" style:margin="auto">
                                    <Outlet/>
                                </div>
                            }
                        }
                    >
                        <Route
                            path="/"
                            view=|cx| {
                                view! { cx, <HomePage/> }
                            }
                        />
                        <Route
                            path="single"
                            view=|cx| {
                                view! { cx, <OnePost/> }
                            }
                        />
                        <Route
                            path="multi"
                            view=|cx| {
                                view! { cx, <MultiPost/> }
                            }
                        />
                        <Route
                            path="reactive"
                            view=|cx| {
                                view! { cx, <ReactivePost/> }
                            }
                        />
                        <Route
                            path="unique"
                            view=|cx| {
                                view! { cx, <UniqueKey/> }
                            }
                        />
                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    let invalidate_one = move |_| {
        use_query_client(cx).invalidate_query::<PostId, String>(&PostId(1));
    };

    let prefetch_two = move |_| {
        use_query_client(cx).prefetch_query(cx, || PostId(2), get_post_unwrapped, true);
    };

    view! { cx,
        <div class="container">
            <h1>"Welcome to Leptos Query!"</h1>
            <p>"This is a simple example of using a query cache."</p>
            <p>"Each post has a stale_time of 5 seconds."</p>
            <h2>"Examples"</h2>
            <ul>
                <li>
                    <a href="/single">"Post 1"</a>
                </li>
                <li>
                    <a href="/multi">"Double use of Post 2"</a>
                </li>
                <li>
                    <a href="/reactive">"Reactive"</a>
                </li>
                <li>
                    <a href="/unique">"Non-Dynamic Key"</a>
                </li>
            </ul>
            <br/>
            <div
                style:display="flex"
                style:flex-direction="column"
                style:gap="1rem"
                style:margin-top="1rem"
            >
                <p>"Cache Size " {move || use_query_client(cx).size()}</p>
                <p>"If you invalidate a post, it will automatically fetch on it's next usage."</p>
                <button class="button" on:click=invalidate_one>
                    "Invalidate Post One"
                </button>
                <p>"If you prefetch a post, it will load the data ahead of visiting the page."</p>
                <button class="button" on:click=prefetch_two>
                    "Prefetch Post Two"
                </button>
            </div>
        </div>
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PostId(u32);

fn use_post_query(
    cx: Scope,
    key: impl Fn() -> PostId + 'static,
) -> QueryResult<String, impl RefetchFn> {
    use_query(
        cx,
        key,
        get_post_unwrapped,
        QueryOptions {
            default_value: None,
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
            stale_time: Some(Duration::from_secs(5)),
            cache_time: Some(Duration::from_secs(60)),
        },
    )
}

async fn get_post_unwrapped(id: PostId) -> String {
    get_post(id).await.unwrap()
}

// Server function that fetches a post.
#[server(GetPost, "/api")]
pub async fn get_post(id: PostId) -> Result<String, ServerFnError> {
    use std::time::Instant;

    log!("Fetching post: {:?}", id.0);
    tokio::time::sleep(Duration::from_millis(2000)).await;
    let instant = Instant::now();
    Ok(format!("Post {} : timestamp {:?}", id.0, instant))
}

#[component]
fn OnePost(cx: Scope) -> impl IntoView {
    view! { cx, <Post post_id=PostId(1)/> }
}

#[component]
fn MultiPost(cx: Scope) -> impl IntoView {
    view! { cx,
        <h1>"Requests are de-duplicated across components"</h1>
        <br/>
        <Post post_id=PostId(2)/>
        <hr/>
        <Post post_id=PostId(2)/>
    }
}

#[component]
fn Post(cx: Scope, #[prop(into)] post_id: MaybeSignal<PostId>) -> impl IntoView {
    let QueryResult {
        data,
        state,
        is_loading,
        is_fetching,
        is_stale,
        is_invalid,
        refetch,
    } = use_post_query(cx, post_id.clone());

    create_effect(cx, move |_| log!("State: {:?}", state.get()));

    view! { cx,
        <div class="container">
            <a href="/">"Home"</a>
            <h2>"Post Key: " {move || post_id.get().0}</h2>
            <div>
                <span>"Loading Status: "</span>
                <span>{move || { if is_loading.get() { "Loading..." } else { "Loaded" } }}</span>
            </div>
            <div>
                <span>"Fetching Status: "</span>
                <span>{move || { if is_fetching.get() { "Fetching..." } else { "Idle" } }}</span>
            </div>
            <div>
                <span>"Stale Status: "</span>
                <span>{move || { if is_stale.get() { "Stale" } else { "Fresh" } }}</span>
            </div>
            <div>
                <span>"Invalidated: "</span>
                <span>{move || { if is_invalid.get() { "Invalid" } else { "Valid" } }}</span>
            </div>
            <div class="post-body">
                <p>"Post Body"</p>
                <Transition fallback=move || {
                    view! { cx, <h2>"Loading..."</h2> }
                }>
                    {move || {
                        data.get()
                            .map(|post| {
                                view! { cx, <h2>{post}</h2> }
                            })
                    }}
                </Transition>
            </div>
            <div>
                <button class="button" on:click=move |_| refetch()>
                    "Refetch query"
                </button>
            </div>
        </div>
    }
}

#[component]
fn ReactivePost(cx: Scope) -> impl IntoView {
    let (post_id, set_post_id) = create_signal(cx, PostId(1));

    view! { cx,
        <Post post_id=post_id/>
        <div style:margin-top="1rem">
            <button
                class="button"
                on:click=move |_| {
                    if post_id.get().0 == 1 {
                        set_post_id(PostId(2));
                    } else {
                        set_post_id(PostId(1));
                    }
                }
            >
                "Switch Post"
            </button>
        </div>
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Unique();

#[server(GetUnique, "/api")]
pub async fn get_unique() -> Result<String, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(2000)).await;
    Ok("Super duper unique value".into())
}

#[component]
fn UniqueKey(cx: Scope) -> impl IntoView {
    let query = use_query(
        cx,
        || Unique(),
        |_| async { get_unique().await.expect("Failed to retrieve unique") },
        QueryOptions::empty(),
    );

    view! { cx,
        <div class="container">
            <a href="/">"Home"</a>
            <div class="post-body">
                <p>"Unique Key"</p>
                <Transition fallback=move || {
                    view! { cx, <h2>"Loading..."</h2> }
                }>
                    {move || {
                        query
                            .data
                            .get()
                            .map(|response| {
                                view! { cx, <h2>{response}</h2> }
                            })
                    }}
                </Transition>
            </div>
        </div>
    }
}
