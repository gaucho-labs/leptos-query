use leptos::*;

#[component]
pub fn Header(#[prop(optional, into)] title: String, children: ChildrenFn) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <h1 class="scroll-m-20 text-4xl font-bold tracking-tight">{title}</h1>
            <div class="text-lg text-muted-foreground">
                <div class="inline-block align-top max-w-xl">{children()}</div>
            </div>
        </div>
    }
}
