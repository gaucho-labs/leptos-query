use leptos::*;

#[component]
pub fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
    view! { <div class=format!("animate-pulse rounded-md bg-primary/10 {class}")></div> }
}
