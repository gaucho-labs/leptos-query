use std::time::Duration;

use leptos::*;
use leptos_query::{create_query, QueryOptions, QueryScope};
use serde::*;

#[component]
pub fn SingleQuery() -> impl IntoView {
    let post_id = create_rw_signal(1_u32);

    let query = post_query().use_query(move || PostQueryKey(post_id.get()));

    let data = query.data;
    let fetching = query.is_fetching;

    view! {
        <div class="flex flex-col w-full gap-4">
            <div class="space-y-2">
                <h1 class="scroll-m-20 text-4xl font-bold tracking-tight">Post with Query</h1>
                <p class="text-lg text-muted-foreground">
                    <span class="inline-block align-top max-w-sm">
                        Add dependencies to your project manually.
                    </span>
                </p>
            </div>

            <div class="flex items-center gap-4">
                <div class="w-32">
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
                <Show when=fetching>
                    <div class="pt-6">
                        <svg
                            class="h-5 w-5 stroke-foreground text-foreground animate-spin"
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <line x1="12" x2="12" y1="2" y2="6"></line>
                            <line x1="12" x2="12" y1="18" y2="22"></line>
                            <line x1="4.93" x2="7.76" y1="4.93" y2="7.76"></line>
                            <line x1="16.24" x2="19.07" y1="16.24" y2="19.07"></line>
                            <line x1="2" x2="6" y1="12" y2="12"></line>
                            <line x1="18" x2="22" y1="12" y2="12"></line>
                            <line x1="4.93" x2="7.76" y1="19.07" y2="16.24"></line>
                            <line x1="16.24" x2="19.07" y1="7.76" y2="4.93"></line>
                        </svg>
                    </div>
                </Show>
            </div>
            <Transition fallback=|| {
                view! {
                    <div class="flex flex-col items-start gap-2 bg-card border rounded-md p-4 max-w-xl mx-auto w-full">
                        <Skeleton class="h-8 w-full"/>
                        <Skeleton class="h-20 w-full"/>
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
        <div class="flex flex-col items-start gap-2 bg-card border rounded-md p-4 max-w-xl mx-auto">
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
            gloo_timers::future::sleep(Duration::from_millis(500)).await;

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
