use crate::{
    error_template::{AppError, ErrorTemplate},
    todo::InteractiveTodo,
};
use leptos::*;
use leptos_meta::*;
use leptos_query::*;
use leptos_router::{Outlet, Route, Router, Routes};
use std::time::Duration;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    // Provides Query Client for entire app.
    provide_query_client();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum.css"/>
        <Title text="Welcome to Leptos"/>
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {  <ErrorTemplate outside_errors/> }
                .into_view()
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
                                view! {  <HomePage/> }
                            }
                        />
                        <Route
                            path="single"
                            view=|| {
                                view! {  <OnePost/> }
                            }
                        />
                        <Route
                            path="multi"
                            view=|| {
                                view! {  <MultiPost/> }
                            }
                        />
                        <Route
                            path="reactive"
                            view=|| {
                                view! {  <ReactivePost/> }
                            }
                        />
                        <Route
                            path="unique"
                            view=|| {
                                view! {  <UniqueKey/> }
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
        use_query_client().invalidate_query::<u32, String>(&1);
    };

    let prefetch_two = move |_| {
        use_query_client().prefetch_query(|| 2, get_post_unwrapped, true);
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

fn use_post_query(key: impl Fn() -> u32 + 'static) -> QueryResult<Option<String>, impl RefetchFn> {
    use_query(
        key,
        get_post_unwrapped,
        QueryOptions {
            default_value: None,
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
            stale_time: Some(Duration::from_secs(5)),
            cache_time: Some(Duration::from_secs(60)),
            on_settled: Some(Callback::new(|res| {
                logging::log!("Got post: {res:?}");
            }))
        },
    )
}

async fn get_post_unwrapped(id: u32) -> Option<String> {
    get_post(id).await.ok()
}

// Server function that fetches a post.
#[server(GetPost, "/api")]
pub async fn get_post(id: u32) -> Result<String, ServerFnError> {
    use leptos_query::Instant;

    logging::log!("Fetching post: {}", id);
    tokio::time::sleep(Duration::from_millis(2000)).await;
    let instant = Instant::now();
    Ok(format!("Post {}: Timestamp {}", id, instant))
}

#[component]
fn OnePost() -> impl IntoView {
    view! { <Post post_id=1/> }
}

#[component]
fn MultiPost() -> impl IntoView {
    view! {
        <h1>"Requests are de-duplicated across components"</h1>
        <br/>
        <Post post_id=2/>
        <hr/>
        <Post post_id=2/>
    }
}

#[component]
fn Post(#[prop(into)] post_id: MaybeSignal<u32>) -> impl IntoView {
    let QueryResult {
        data,
        state,
        is_loading,
        is_fetching,
        is_stale,
        is_invalid,
        refetch,
    } = use_post_query(post_id);

    create_effect(move |_| logging::log!("State: {:#?}", state.get()));

    view! {
        <div class="container">
            <a href="/">"Home"</a>
            <h2>"Post Key: " {move || post_id.get()}</h2>
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
                    view! {  <h2>"Loading..."</h2> }
                }>
                    <h2>
                        {
                           move || {
                            data
                            .get()
                            .map(|post| {
                                match post {
                                    Some(post) => post,
                                    None => "Not Found".into(),
                                }
                            })
                        }
                        }
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
    let (post_id, set_post_id) = create_signal(1);

    view! {
        <Post post_id=post_id/>
        <div style:margin-top="1rem">
            <button
                class="button"
                on:click=move |_| {
                    if post_id.get() == 1 {
                        set_post_id(2);
                    } else {
                        set_post_id(1);
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

#[component]
fn UniqueKey() -> impl IntoView {
    let QueryResult { data, .. } = use_query(
        || (),
        |_| async { get_unique().await.expect("Failed to retrieve unique") },
        QueryOptions::empty(),
    );

    view! {
        <div class="container">
            <a href="/">"Home"</a>
            <div class="post-body">
                <p>"Unique Key"</p>
                <Transition fallback=move || {
                    view! {  <h2>"Loading..."</h2> }
                }>
                    {move || {
                        data.get()
                            .map(|response| {
                                view! {  <h2>{response}</h2> }
                            })
                    }}
                </Transition>
            </div>
        </div>
    }
}
