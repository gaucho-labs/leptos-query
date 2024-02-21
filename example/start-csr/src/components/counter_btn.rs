use leptos::*;

/// A parameterized incrementing button
#[component]
pub fn Button(#[prop(default = 1)] increment: i32) -> impl IntoView {
    let (count, set_count) = create_signal(0);
    view! {
        <button
            on:click= move |_| {
                set_count(count() + increment)
            }
        >
            "Click me: " {count}
        </button>
    }
}
