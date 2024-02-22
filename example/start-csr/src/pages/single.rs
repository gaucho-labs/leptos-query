use std::time::Duration;

use leptos::*;
use leptos_query::{create_query, QueryOptions, QueryScope};
use serde::*;

use crate::components::{header::Header, skeleton::Skeleton, spinner::Spinner, Loud};

#[component]
pub fn QueryVsResource() -> impl IntoView {
    view! {
        <div class="container mx-auto p-8">
            <div class="flex flex-col gap-8">
                <SingleQuery/>
                <div class="h-2 w-full bg-border"></div>
                <SingleResource/>
            </div>
        </div>
    }
}

#[component]
fn SingleQuery() -> impl IntoView {
    let post_id = create_rw_signal(1_u32);

    let query = post_query().use_query(move || PostQueryKey(post_id.get()));

    let data = query.data;
    let fetching = query.is_fetching;

    view! {
        <div class="flex flex-col w-full gap-4">
            <Header title="Post with Query">
                <p>
                    Fetching with <Loud>Leptos Query</Loud>
                </p>
            </Header>

            <div class="flex items-center gap-4">
                <div class="w-32">
                    <label class=LABEL_CLASS for="post-id-query">
                        Post ID
                    </label>
                    <input
                        type="number"
                        id="post-id-query"
                        on:input=move |ev| {
                            let new_post = event_target_value(&ev).parse().unwrap_or(1).max(1);
                            post_id.set(new_post);
                        }

                        prop:value=post_id
                        class=INPUT_CLASS
                    />
                </div>
                <div class="pt-6">
                    <Spinner fetching/>
                </div>
            </div>
            <Transition fallback=|| {
                view! { <SkeletonCard/> }
            }>
                {move || {
                    data.get()
                        .map(|post| {
                            match post {
                                Some(post) => view! { <Post post/> }.into_view(),
                                None => view! { <div>No Post Found</div> }.into_view(),
                            }
                        })
                }}

            </Transition>
        </div>
    }
}

#[component]
fn Post(post: PostValue) -> impl IntoView {
    view! {
        <div class=CARD_CLASS>
            <div class="space-y-0.5">
                <h2 class="text-2xl font-bold tracking-tight">{post.title}</h2>
                <p class="text-muted-foreground">{post.body}</p>
            </div>
        </div>
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PostQueryKey(u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostValue {
    user_id: u32,
    id: u32,
    title: String,
    body: String,
}

fn post_query() -> QueryScope<PostQueryKey, Option<PostValue>> {
    create_query(
        |post_id: PostQueryKey| async move { get_post(post_id.0).await },
        QueryOptions::default(),
    )
}

async fn get_post(post_id: u32) -> Option<PostValue> {
    gloo_timers::future::sleep(Duration::from_millis(500)).await;
    let response = reqwest::get(&format!(
        "https://jsonplaceholder.typicode.com/posts/{}",
        post_id
    ))
    .await;

    if let Ok(result) = response {
        let result = result.json::<PostValue>().await;
        result.ok()
    } else {
        None
    }
}

#[component]
fn SkeletonCard() -> impl IntoView {
    view! {
        <div class=CARD_CLASS>
            <Skeleton class="h-8 w-full"/>
            <Skeleton class="h-20 w-full"/>
        </div>
    }
}

const CARD_CLASS: &str =
    "flex flex-col items-start gap-2 bg-card border rounded-md p-4 max-w-xl w-full h-40";
const LABEL_CLASS: &str =
    "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70";
const INPUT_CLASS: &str= "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50";

#[component]
fn SingleResource() -> impl IntoView {
    let post_id = create_rw_signal(1_u32);

    let resource = create_local_resource(post_id, get_post);

    view! {
        <div class="flex flex-col w-full gap-4">
            <Header title="Post with Resource">
                <p>
                    Fetching with
                    <a
                        href="https://book.leptos.dev/async/10_resources.html"
                    >
                        <Loud>Leptos Resource</Loud>
                    </a>
                </p>
            </Header>

            <div class="flex items-center gap-4">
                <div class="w-32">
                    <label class=LABEL_CLASS for="post-id-resource">
                        Post ID
                    </label>
                    <input
                        type="number"
                        id="post-id-resource"
                        on:input=move |ev| {
                            let new_post = event_target_value(&ev).parse().unwrap_or(1).max(1);
                            post_id.set(new_post);
                        }

                        prop:value=post_id
                        class=INPUT_CLASS
                    />
                </div>
                <div class="pt-6">
                    <Spinner fetching=move || resource.loading().get()/>
                </div>
            </div>
            <Transition fallback=|| {
                view! { <SkeletonCard/> }
            }>
                {move || {
                    resource
                        .get()
                        .map(|post| {
                            match post {
                                Some(post) => view! { <Post post/> }.into_view(),
                                None => view! { <div>No Post Found</div> }.into_view(),
                            }
                        })
                }}

            </Transition>
        </div>
    }
}
