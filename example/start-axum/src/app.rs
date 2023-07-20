use crate::error_template::{AppError, ErrorTemplate};
use leptos::{leptos_dom::helpers::IntervalHandle, *};
use leptos_meta::*;
use leptos_query::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use std::{cell::Cell, rc::Rc, time::Duration};

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
                        <a href="/single">"Post 1"</a>
                    </li>
                    <li>
                        <a href="/multi">"Post 2"</a>
                    </li>
                    <li>
                        <a href="/reactive">"Reactive"</a>
                    </li>
                </ul>
                <br/>
            </div>
            <div id="complex"></div>
        </div>
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PostId(String);

fn use_post_query(cx: Scope, key: impl Fn() -> PostId + 'static) -> QueryResult<String> {
    leptos_query::use_query(
        cx,
        key,
        get_post_unwrapped,
        QueryOptions {
            default_value: None,
            refetch_interval: Some(Duration::from_secs(5)),
            resource_option: ResourceOption::NonBlocking,
            stale_time: Some(Duration::from_secs(10)),
            cache_time: Some(Duration::from_secs(20)),
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
    view! { cx, <Post post_id=PostId("one".into())/> }
}

#[component]
fn MultiPost(cx: Scope) -> impl IntoView {
    view! { cx,
        <h1>"Requests are de-duplicated across components"</h1>
        <br/>
        <Post post_id=PostId("two".into())/>
        <hr/>
        <Post post_id=PostId("two".into())/>
    }
}

#[component]
fn Post(cx: Scope, #[prop(into)] post_id: MaybeSignal<PostId>) -> impl IntoView {
    let query = use_post_query(cx, post_id.clone());

    let QueryResult {
        data,
        is_loading,
        is_refetching,
        is_stale,
        ..
    } = query;

    view! { cx,
        <div class="post">
            <a href="/">"Home"</a>
            <h2>"Post Key: " {move || post_id.get().0}</h2>
            <div>
                <span>"Loading Status: "</span>
                <span>{move || { if is_loading.get() { "Loading..." } else { "Loaded" } }}</span>
            </div>
            <div>
                <span>"Fetching Status: "</span>
                <span>
                    {move || { if is_refetching.get() { "Fetching..." } else { "Idle" } }}
                </span>
            </div>
            <div>
                <span>"Stale Status: "</span>
                <span>
                    {move || { if is_stale.get() { "Stale" } else { "Fresh" } }}
                </span>
            </div>
            <div class="post-body">
                <p>"Post Body"</p>
                <Transition fallback=move || {
                    view! { cx, <h2>"Loading..."</h2> }
                }>
                    {move || {
                        data()
                            .map(|post| {
                                view! { cx, <h2>{post}</h2> }
                            })
                    }}
                </Transition>
            </div>
            <div>
                <button on:click=move |_| query.refetch()>"Refetch query"</button>
            </div>
        </div>
    }
}

#[component]
fn ReactivePost(cx: Scope) -> impl IntoView {
    let (post_id, set_post_id) = create_signal(cx, PostId("one".into()));

    let last_interval = Rc::new(Cell::new(None as Option<IntervalHandle>));

    on_cleanup(cx, {
        let last_interval = last_interval.clone();
        move || {
            if let Some(interval) = last_interval.get() {
                interval.clear();
            }
        }
    });

    create_effect(cx, move |interval: Option<Option<IntervalHandle>>| {
        if let Some(interval) = interval.flatten() {
            interval.clear();
        }
        let interval = set_interval_with_handle(
            move || {
                log!("changing post !!!");
                if post_id.get().0 == "one" {
                    set_post_id(PostId("two".into()));
                } else {
                    set_post_id(PostId("one".into()));
                }
            },
            Duration::from_secs(5),
        )
        .ok();
        last_interval.set(interval);
        interval
    });

    view! { cx, <Post post_id=post_id/> }
}
