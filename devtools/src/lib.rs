use std::collections::HashMap;

use leptos::*;
use leptos_query::*;

#[component]
pub fn LeptosQueryDevtools() -> impl IntoView {
    let client = use_query_client();
    let state = DevtoolsContext::new();

    client.register_cache_observer(state.clone());
    provide_context(state);

    view! {
        <Portal>
            <Devtools/>
        </Portal>
    }
}

#[derive(Clone)]
struct DevtoolsContext {
    query_state: RwSignal<HashMap<QueryCacheKey, Signal<QueryState<String>>>>,
    open: RwSignal<bool>,
}

fn use_devtools_context() -> DevtoolsContext {
    use_context::<DevtoolsContext>().expect("Devtools Context to be present.")
}

impl DevtoolsContext {
    fn new() -> Self {
        DevtoolsContext {
            query_state: create_rw_signal(HashMap::new()),
            open: create_rw_signal(false),
        }
    }
}

impl CacheObserver for DevtoolsContext {
    fn process_cache_event(&self, event: CacheEvent) {
        match event {
            CacheEvent::Created(QueryCachePayload { key, state }) => {
                self.query_state.update(|map| {
                    map.insert(key, state);
                })
            }
            CacheEvent::Removed(key) => self.query_state.update(|map| {
                map.remove(&key);
            }),
        }
    }
}

#[component]
fn Devtools() -> impl IntoView {
    let DevtoolsContext { open, query_state } = use_devtools_context();

    view! {
        <Show
            when=move || open.get()
            fallback=move || {
                view! {
                    <button
                        on:click=move |_| open.set(true)
                        style:position="absolute"
                        style:bottom="20px"
                        style:right="20px"
                        style:border-radius="9999px"
                        style:color="rgb(241 245 249)"
                        style:background-color="rgb(17 24 39)"
                        style:width="2.5rem"
                        style:height="2.5rem"
                        // slate-300
                        style:border-color="rgb(203 213 225)"
                        style:border-width="2px"
                    >
                        <svg
                            width="100%"
                            height="100%"
                            viewBox="0 0 15 15"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path
                                d="M7.07095 0.650238C6.67391 0.650238 6.32977 0.925096 6.24198 1.31231L6.0039 2.36247C5.6249 2.47269 5.26335 2.62363 4.92436 2.81013L4.01335 2.23585C3.67748 2.02413 3.23978 2.07312 2.95903 2.35386L2.35294 2.95996C2.0722 3.2407 2.0232 3.6784 2.23493 4.01427L2.80942 4.92561C2.62307 5.2645 2.47227 5.62594 2.36216 6.00481L1.31209 6.24287C0.924883 6.33065 0.650024 6.6748 0.650024 7.07183V7.92897C0.650024 8.32601 0.924883 8.67015 1.31209 8.75794L2.36228 8.99603C2.47246 9.375 2.62335 9.73652 2.80979 10.0755L2.2354 10.9867C2.02367 11.3225 2.07267 11.7602 2.35341 12.041L2.95951 12.6471C3.24025 12.9278 3.67795 12.9768 4.01382 12.7651L4.92506 12.1907C5.26384 12.377 5.62516 12.5278 6.0039 12.6379L6.24198 13.6881C6.32977 14.0753 6.67391 14.3502 7.07095 14.3502H7.92809C8.32512 14.3502 8.66927 14.0753 8.75705 13.6881L8.99505 12.6383C9.37411 12.5282 9.73573 12.3773 10.0748 12.1909L10.986 12.7653C11.3218 12.977 11.7595 12.928 12.0403 12.6473L12.6464 12.0412C12.9271 11.7604 12.9761 11.3227 12.7644 10.9869L12.1902 10.076C12.3768 9.73688 12.5278 9.37515 12.638 8.99596L13.6879 8.75794C14.0751 8.67015 14.35 8.32601 14.35 7.92897V7.07183C14.35 6.6748 14.0751 6.33065 13.6879 6.24287L12.6381 6.00488C12.528 5.62578 12.3771 5.26414 12.1906 4.92507L12.7648 4.01407C12.9766 3.6782 12.9276 3.2405 12.6468 2.95975L12.0407 2.35366C11.76 2.07292 11.3223 2.02392 10.9864 2.23565L10.0755 2.80989C9.73622 2.62328 9.37437 2.47229 8.99505 2.36209L8.75705 1.31231C8.66927 0.925096 8.32512 0.650238 7.92809 0.650238H7.07095ZM4.92053 3.81251C5.44724 3.44339 6.05665 3.18424 6.71543 3.06839L7.07095 1.50024H7.92809L8.28355 3.06816C8.94267 3.18387 9.5524 3.44302 10.0794 3.81224L11.4397 2.9547L12.0458 3.56079L11.1882 4.92117C11.5573 5.44798 11.8164 6.0575 11.9321 6.71638L13.5 7.07183V7.92897L11.932 8.28444C11.8162 8.94342 11.557 9.55301 11.1878 10.0798L12.0453 11.4402L11.4392 12.0462L10.0787 11.1886C9.55192 11.5576 8.94241 11.8166 8.28355 11.9323L7.92809 13.5002H7.07095L6.71543 11.932C6.0569 11.8162 5.44772 11.5572 4.92116 11.1883L3.56055 12.046L2.95445 11.4399L3.81213 10.0794C3.4431 9.55266 3.18403 8.94326 3.06825 8.2845L1.50002 7.92897V7.07183L3.06818 6.71632C3.18388 6.05765 3.44283 5.44833 3.81171 4.92165L2.95398 3.561L3.56008 2.95491L4.92053 3.81251ZM9.02496 7.50008C9.02496 8.34226 8.34223 9.02499 7.50005 9.02499C6.65786 9.02499 5.97513 8.34226 5.97513 7.50008C5.97513 6.65789 6.65786 5.97516 7.50005 5.97516C8.34223 5.97516 9.02496 6.65789 9.02496 7.50008ZM9.92496 7.50008C9.92496 8.83932 8.83929 9.92499 7.50005 9.92499C6.1608 9.92499 5.07513 8.83932 5.07513 7.50008C5.07513 6.16084 6.1608 5.07516 7.50005 5.07516C8.83929 5.07516 9.92496 6.16084 9.92496 7.50008Z"
                                fill="currentColor"
                                fill-rule="evenodd"
                                clip-rule="evenodd"
                            ></path>
                        </svg>
                    </button>
                }
            }
        >

            <div
                // slate-50
                style:color="rgb(241 245 249)"
                // slate-900
                style:background-color="rgb(17 24 39)"
                style:padding-left="0px"
                style:padding-right="0px"
                style="position: fixed; bottom: 0; left: 0; right: 0; height: 500px; overflow-y: auto; z-index: 1000; resize: vertical;"
            >
                <div
                    style:width="100%"
                    style:height="100%"
                    style:display="flex"
                    style:flex-direction="column"
                    style:position="relative"
                >
                    <CloseButton/>
                    <Header/>
                    <ul
                        style:width="100%"
                        style:height="100%"
                        style:display="flex"
                        style:flex-direction="column"
                        style:gap="0.25rem"
                        style:margin="0"
                        style:padding="0"
                        style:list-style-type="none"
                    >
                        <For each=move || query_state.get() key=|(key, _)| key.clone() let:entry>
                            <li>
                                <QueryRow key=entry.0 state=entry.1/>
                            </li>
                        </For>

                    </ul>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn CloseButton() -> impl IntoView {
    let DevtoolsContext { open, .. } = use_devtools_context();

    view! {
        <button
            on:click=move |_| open.set(false)
            style:position="absolute"
            style:top="20px"
            style:right="20px"
            style:border-radius="9999px"
            style:color="rgb(241 245 249)"
            style:background-color="rgb(17 24 39)"
            style:width="2rem"
            style:height="2rem"
            // slate-300
            style:border-color="rgb(203 213 225)"
            style:border-width="2px"
        >
            <svg
                width="100%"
                height="100%"
                viewBox="0 0 15 15"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <path
                    d="M12.8536 2.85355C13.0488 2.65829 13.0488 2.34171 12.8536 2.14645C12.6583 1.95118 12.3417 1.95118 12.1464 2.14645L7.5 6.79289L2.85355 2.14645C2.65829 1.95118 2.34171 1.95118 2.14645 2.14645C1.95118 2.34171 1.95118 2.65829 2.14645 2.85355L6.79289 7.5L2.14645 12.1464C1.95118 12.3417 1.95118 12.6583 2.14645 12.8536C2.34171 13.0488 2.65829 13.0488 2.85355 12.8536L7.5 8.20711L12.1464 12.8536C12.3417 13.0488 12.6583 13.0488 12.8536 12.8536C13.0488 12.6583 13.0488 12.3417 12.8536 12.1464L8.20711 7.5L12.8536 2.85355Z"
                    fill="currentColor"
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                ></path>
            </svg>
        </button>
    }
}

#[component]
fn Header() -> impl IntoView {
    let DevtoolsContext { query_state, .. } = use_devtools_context();

    let num_loaded = Signal::derive(move || {
        query_state
            .get()
            .values()
            .map(|v| v.get())
            .filter(|s| matches!(s, QueryState::Loaded(_)))
            .count()
    });

    let num_fetching = Signal::derive(move || {
        query_state
            .get()
            .values()
            .map(|v| v.get())
            .filter(|s| matches!(s, QueryState::Fetching(_) | QueryState::Loading))
            .count()
    });

    let total = Signal::derive(move || query_state.get().len());

    view! {
        <div style:display="flex" style:gap="1rem" style:padding="1rem" style:width="100%">
            <div>{move || format!("Fresh: {}", num_loaded.get())}</div>
            <div>{move || format!("Fetching: {}", num_fetching.get())}</div>
            <div>{move || format!("Total: {}", total.get())}</div>
        </div>
    }
}

#[component]
fn QueryRow(key: QueryCacheKey, state: Signal<QueryState<String>>) -> impl IntoView {
    view! {
        <div
            style:display="flex"
            style:gap="1rem"
            style:align-items="center"
            style:border-top-width="1px"
            style:border-bottom-width="0px"
            style:border-color="rgb(203 213 225)"
        >
            <RowStateLabel state/>
            <span>{key.0}</span>
        </div>
    }
}

#[component]
fn RowStateLabel(state: Signal<QueryState<String>>) -> impl IntoView {
    let state_label = Signal::derive(move || match state.get() {
        QueryState::Created => "Created",
        QueryState::Loading => "Loading",
        QueryState::Fetching(_) => "Fetching",
        QueryState::Loaded(_) => "Loaded",
        QueryState::Invalid(_) => "Invalid",
    });

    // tailwind 900 colors
    let bg_color = Signal::derive(move || match state.get() {
        // blue
        QueryState::Created | QueryState::Loading | QueryState::Fetching(_) => "rgb(30, 58, 138)",
        // green
        QueryState::Loaded(_) => "rgb(20, 83, 45)",
        // red
        QueryState::Invalid(_) => "rgb(127, 29, 29)",
    });

    let text_color = Signal::derive(move || match state.get() {
        // blue
        QueryState::Created | QueryState::Loading | QueryState::Fetching(_) => "rgb(191, 219, 254)",
        // green
        QueryState::Loaded(_) => "rgb(187, 247, 208)",
        // red
        QueryState::Invalid(_) => "rgb(254, 202, 202)",
    });

    view! {
        <span
            style:background-color=bg_color
            style:color=text_color
            style="font-size: 1rem; font-weight: 500; padding: 0.125rem 0.625rem; border-radius: 0.25rem; width: 4rem; text-align: center;"
        >
            {state_label}
        </span>
    }
}
