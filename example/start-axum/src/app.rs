use crate::{
    error_template::{AppError, ErrorTemplate},
    todo::InteractiveTodo,
};
use leptos::*;
use leptos_meta::*;
use leptos_query::*;
use leptos_query_devtools::LeptosQueryDevtools;
use leptos_router::{Outlet, Route, Router, Routes};
use std::time::Duration;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    // Provides Query Client for entire app.
    provide_query_client_with_options_and_persister(
        Default::default(),
        local_storage_persister::LocalStoragePersister,
    );

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum.css"/>
        <Title text="Welcome to Leptos"/>
        <LeptosQueryDevtools/>
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main>
                <Routes>
                    <Route
                        path=""
                        view=|| {
                            view! {
                                <div id="simple" style:width="50rem" style:margin="auto">
                                    <Outlet/>
                                </div>
                            }
                        }
                    >

                        <Route
                            path="/"
                            view=|| {
                                view! { <HomePage/> }
                            }
                        />

                        <Route
                            path="single"
                            view=|| {
                                view! { <OnePost/> }
                            }
                        />

                        <Route
                            path="multi"
                            view=|| {
                                view! { <MultiPost/> }
                            }
                        />

                        <Route
                            path="reactive"
                            view=|| {
                                view! { <ReactivePost/> }
                            }
                        />

                        <Route
                            path="unique"
                            view=|| {
                                view! { <UniqueKeyExample/> }
                            }
                        />

                        <Route
                            path="refetch"
                            view=|| {
                                view! { <RefetchInterval/> }
                            }
                        />

                        <Route
                            path="todos"
                            view=|| {
                                view! { <InteractiveTodo/> }
                            }
                        />

                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let invalidate_one = move |_| {
        post_query().invalidate_query(PostKey(1));
    };

    let prefetch_two = move |_| {
        spawn_local(async move {
            post_query().prefetch_query(PostKey(2)).await;
        });
    };

    view! {
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
                <li>
                    <a href="/refetch">"Refetch Interval"</a>
                </li>
                <li>
                    <a href="/todos">"Todos"</a>
                </li>
            </ul>
            <br/>
            <div
                style:display="flex"
                style:flex-direction="column"
                style:gap="1rem"
                style:margin-top="1rem"
            >
                <p>"Cache Size " {move || use_query_client().size()}</p>
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

fn post_query() -> QueryScope<PostKey, Option<String>> {
    leptos_query::create_query(
        |id| async move { get_post(id).await.ok() },
        QueryOptions {
            default_value: None,
            refetch_interval: None,
            resource_option: Some(ResourceOption::NonBlocking),
            stale_time: Some(Duration::from_secs(5)),
            gc_time: Some(Duration::from_secs(60)),
        },
    )
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
struct PostKey(u32);

// Server function that fetches a post.
#[server(GetPost, "/api")]
async fn get_post(id: PostKey) -> Result<String, ServerFnError> {
    use leptos_query::Instant;

    logging::log!("Fetching post: {:?}", id);
    tokio::time::sleep(Duration::from_millis(2000)).await;
    let instant = Instant::now();
    let id = id.0;
    Ok(format!("Post {id}: Timestamp {instant}"))
}

#[component]
fn OnePost() -> impl IntoView {
    view! { <Post post_id=PostKey(1)/> }
}

#[component]
fn MultiPost() -> impl IntoView {
    view! {
        <h1>"Requests are de-duplicated across components"</h1>
        <br/>
        <Post post_id=PostKey(2)/>
        <hr/>
        <Post post_id=PostKey(2)/>
    }
}

#[component]
fn Post(#[prop(into)] post_id: MaybeSignal<PostKey>) -> impl IntoView {
    let QueryResult {
        data,
        state,
        refetch,
        ..
    } = post_query().use_query(post_id);

    create_effect(move |_| logging::log!("State: {:#?}", state.get()));

    view! {
        <div class="container">
            <a href="/">"Home"</a>
            <h2>"Post Key: " {move || post_id.get().0}</h2>
            <div class="post-body">
                <p>"Post Body"</p>
                <Transition fallback=move || {
                    view! { <h2>"Loading..."</h2> }
                }>
                    <h2>

                        {move || {
                            data.get()
                                .map(|post| {
                                    match post {
                                        Some(post) => post,
                                        None => "Not Found".into(),
                                    }
                                })
                        }}

                    </h2>
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
fn ReactivePost() -> impl IntoView {
    let (post_id, set_post_id) = create_signal(PostKey(1));

    view! {
        <Post post_id=post_id/>
        <div style:margin-top="1rem">
            <button
                class="button"
                on:click=move |_| {
                    if post_id.get() == PostKey(1) {
                        set_post_id(PostKey(2));
                    } else {
                        set_post_id(PostKey(1));
                    }
                }
            >

                "Switch Post"
            </button>
        </div>
    }
}

#[server(GetUnique, "/api")]
pub async fn get_unique() -> Result<String, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(2000)).await;
    Ok("Super duper unique value".into())
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct UniqueKey;

#[component]
fn UniqueKeyExample() -> impl IntoView {
    let QueryResult { data, .. } = use_query(
        || UniqueKey,
        |_| async { get_unique().await.expect("Failed to retrieve unique") },
        QueryOptions::once(),
    );

    view! {
        <div class="container">
            <a href="/">"Home"</a>
            <div class="post-body">
                <p>"Unique Key"</p>
                <Transition fallback=move || {
                    view! { <h2>"Loading..."</h2> }
                }>
                    {move || {
                        data.get()
                            .map(|response| {
                                view! { <h2>{response}</h2> }
                            })
                    }}

                </Transition>
            </div>
        </div>
    }
}

#[component]
fn RefetchInterval() -> impl IntoView {
    let QueryResult {
        data,
        state,
        refetch,
        ..
    } = post_query().use_query_with_options(
        || PostKey(1),
        QueryOptions {
            refetch_interval: Some(Duration::from_secs(5)),
            ..Default::default()
        },
    );

    create_effect(move |_| logging::log!("State: {:#?}", state.get()));

    view! {
        <div class="container">
            <a href="/">"Home"</a>
            <div class="post-body">
                <p>"Refetch Interval"</p>
                <Transition fallback=move || {
                    view! { <h2>"Loading..."</h2> }
                }>
                    {move || {
                        data.get()
                            .map(|response| {
                                view! { <h2>{response}</h2> }
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
