use leptos::*;

/// Default Home Page
#[component]
pub fn Home() -> impl IntoView {
    view! {
        <section class="container flex-1">
            <div class="flex flex-col w-full gap-2 items-center text-center lg:text-start space-y-6">
                <div class="flex flex-col items-center gap-8 text-center px-4">
                    <div class="flex gap-2 lg:gap-4 items-center">
                        <div
                            class="w-[40px] md:w-[60px] bg-muted rounded-full"
                            inner_html=include_str!("../../../../logo.svg")
                        ></div>
                        <h1 class="inline-block font-black text-2xl md:text-4xl">
                            <span class="inline-block italic text-transparent bg-clip-text bg-gradient-to-r from-red-700 to-orange-400">
                                Leptos Query
                            </span>
                        </h1>
                    </div>
                    <h2 class="font-bold text-xl max-w-md md:text-2xl lg:max-w-2xl">
                        Robust asynchronous state management library for Leptos
                    </h2>
                </div>
            </div>
        </section>
    }
}
