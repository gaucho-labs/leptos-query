use leptos::*;

/// Default Home Page
#[component]
pub fn Home() -> impl IntoView {
    view! {
        <section class="container flex-1 flex flex-col gap-10">
            <div class="flex flex-col w-full gap-2 items-center text-center lg:text-start space-y-6">
                <div class="flex flex-col items-center gap-8 text-center px-4">
                    <div class="flex gap-2 lg:gap-4 items-center">
                        <div
                            class="w-[40px] md:w-[60px] bg-muted rounded-full"
                            inner_html=include_str!("../../../../logo.svg")
                        ></div>
                        <h1 class="inline-block font-black text-2xl md:text-4xl lg:text-6xl">
                            <span class="inline-block italic text-transparent bg-clip-text bg-gradient-to-r from-red-700 to-orange-400">
                                Leptos Query
                            </span>
                        </h1>
                    </div>
                    <h2 class="font-bold text-xl max-w-md md:text-2xl lg:max-w-2xl">
                        Robust asynchronous state management for Leptos
                    </h2>
                </div>
            </div>

            <div class="flex flex-col lg:flex-row items-stretch gap-8 p-8 max-w-[1200px] mx-auto lg:min-h-72">
                <InfoCard title="Simple & Expressive">
                    <p>
                        <Umphf>Expressive, powerful, and simple</Umphf>
                        client-side caching, handling background updates and stale data out of the box with zero-configuration.
                    </p>
                    <p>
                        Unlocks features like optimistic updates, cancellation, persistance, and more.
                    </p>
                </InfoCard>
                <InfoCard title="Deep Leptos Integration">
                    Flawless support for CSR and SSR rendering strategies, as well as {"Leptos's"} fine-grained reactivity, ensuring your data fetching is performant and up to date.
                </InfoCard>
                <InfoCard title="Developer First">
                    <div class="h-full flex flex-col justify-between items-start">
                        <p>
                            Designed with developers in mind, Leptos Query includes comprehensive devtools, making it easier to debug your queries, ensuring a seamless development experience.
                        </p>
                        <p>p.s. Take a peek at the bottom right corner!</p>
                    </div>
                </InfoCard>
            </div>
        </section>
    }
}
#[component]
fn Umphf(children: Children) -> impl IntoView {
    view! { <span class="font-semibold text-red-700 dark:text-red-400">{children()}</span> }
}

#[component]
fn InfoCard(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class="flex-1 flex flex-col gap-8 items-center p-4 rounded-md border">
            <div class="flex flex-col gap-4 flex-1">
                <h3 class="uppercase text-center text-lg text-foreground font-bold">{title}</h3>
                <div class="flex-1 text-sm text-muted-foreground leading-6">{children()}</div>
            </div>
        </div>
    }
}
