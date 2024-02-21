use std::time::Duration;

use leptos::*;
use leptos_query::{create_query, QueryOptions, QueryScope};
use serde::*;

#[component]
pub fn SingleQuery() -> impl IntoView {
    let post_id = create_rw_signal(1_u32);

    let query = post_query().use_query(move || PostQueryKey(post_id.get()));

    let data = query.data;

    view! {
        <div>
            <div class="w-24">
                <label
                    class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    for="post-id"
                >
                    Post ID
                </label>
                <input
                    type="number"
                    id="post-id"
                    on:input=move |ev| {
                        let new_post = event_target_value(&ev).parse().unwrap_or(0);
                        post_id.set(new_post);
                    }

                    prop:value=post_id
                    class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                />
            </div>
            <Transition fallback=|| {
                view!{
                    <div class="flex flex-col items-start gap-2 bg-card border rounded-md p-4">
                        <Skeleton class="h-8 w-full"></Skeleton>
                        <Skeleton class="h-20 w-full"></Skeleton>
                    </div>
                }
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
        <div class="flex flex-col items-start gap-2 bg-card border rounded-md p-4">
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
        |post_id: PostQueryKey| async move {
            gloo_timers::future::sleep(Duration::from_millis(1000)).await;
            let response = reqwest::get(&format!(
                "https://jsonplaceholder.typicode.com/posts/{}",
                post_id.0
            ))
            .await;

            if let Ok(result) = response {
                let result = result.json::<PostValue>().await;
                result.ok()
            } else {
                None
            }
        },
        QueryOptions::default(),
    )
}

#[component]
fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
    view! { <div class=format!("animate-pulse rounded-md bg-primary/10 {class}")></div> }
}
