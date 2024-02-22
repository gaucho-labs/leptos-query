use leptos::*;

pub mod header;
pub mod skeleton;
pub mod spinner;

#[component]
pub fn Loud(children: Children) -> impl IntoView {
    view! { <span class="font-semibold text-amber-600 dark:text-amber-400">{children()}</span> }
}
