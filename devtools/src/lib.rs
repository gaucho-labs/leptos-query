use leptos::*;

#[component]
pub fn LeptosQueryDevtools() -> impl IntoView {
    #[cfg(all(target_arch = "wasm32"))]
    {
        use dev_tools::InnerDevtools;
        view! { <InnerDevtools/> }
    }
}

// #[cfg(all(target_arch = "wasm32"))]
mod dev_tools {
    use leptos::*;
    use leptos_query::*;
    use std::collections::HashMap;

    #[component]
    pub(crate) fn InnerDevtools() -> impl IntoView {
        let client = leptos_query::use_query_client();
        let state = DevtoolsContext::new();
        client.register_cache_observer(state.clone());
        provide_context(state);

        view! {
            <Portal>
                <style>{include_str!("./styles.css")}</style>
                <div class="leptos-query-devtools">
                    <Devtools/>
                </div>
            </Portal>
        }
    }

    #[derive(Clone)]
    struct DevtoolsContext {
        query_state: RwSignal<HashMap<QueryCacheKey, QueryCacheEntry>>,
        open: RwSignal<bool>,
        filter: RwSignal<String>,
        selected_query: RwSignal<Option<QueryCacheEntry>>,
    }

    #[derive(Clone)]
    struct QueryCacheEntry {
        key: QueryCacheKey,
        state: Signal<QueryState<String>>,
        observer_count: Signal<usize>,
    }

    fn use_devtools_context() -> DevtoolsContext {
        use_context::<DevtoolsContext>().expect("Devtools Context to be present.")
    }

    impl DevtoolsContext {
        fn new() -> Self {
            DevtoolsContext {
                query_state: create_rw_signal(HashMap::new()),
                open: create_rw_signal(false),
                filter: create_rw_signal("".to_string()),
                selected_query: create_rw_signal(None),
            }
        }
    }

    impl CacheObserver for DevtoolsContext {
        fn process_cache_event(&self, event: CacheEvent) {
            match event {
                CacheEvent::Created(QueryCachePayload {
                    key,
                    state,
                    observer_count,
                }) => self.query_state.update(|map| {
                    let entry = QueryCacheEntry {
                        key: key.clone(),
                        state,
                        observer_count,
                    };
                    map.insert(key, entry);
                }),
                CacheEvent::Removed(key) => self.query_state.update(|map| {
                    map.remove(&key);
                }),
            }
        }
    }

    #[component]
    fn Devtools() -> impl IntoView {
        let DevtoolsContext {
            open,
            query_state,
            selected_query,
            filter,
        } = use_devtools_context();

        let query_state = Signal::derive(move || {
            let filter = filter.get().to_ascii_lowercase();
            query_state
                .get()
                .into_iter()
                .filter(|(key, _)| key.0.to_ascii_lowercase().contains(&filter))
                .collect::<HashMap<_, _>>()
        });

        view! {
            <Show
                when=move || open.get()
                fallback=move || {
                    view! {
                        <button
                            on:click=move |_| open.set(true)
                            class="bg-background text-foreground absolute bottom-5 right-5 border-2 rounded-full w-10 h-10 p-1 transition-colors hover:bg-accent"
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

                <div class="bg-background text-foreground px-0 fixed bottom-0 left-0 right-0 h-[500px] z-[1000]">
                    <div class="h-full flex flex-col relative">
                        <div class="flex-1 overflow-hidden flex">
                            <div class="flex flex-col flex-1 overflow-y-auto">
                                <Header/>
                                <div class="py-1 px-2 border-b border-zinc-800">
                                    <SearchInput/>
                                </div>

                                <ul class="flex flex-col gap-1 px-1 m-0 list-none">
                                    <For
                                        each=move || query_state.get()
                                        key=|(key, _)| key.clone()
                                        let:entry
                                    >
                                        <QueryRow entry=entry.1/>
                                    </For>

                                </ul>
                            </div>
                            <Show when=move || selected_query.get().is_some()>
                                {move || {
                                    selected_query.get().map(|q| view!{
                                        <SelectedQuery query=q/>
                                    })
                                }}
                            </Show>
                        </div>
                        <div class="absolute -top-5 right-2">
                            <CloseButton/>
                        </div>
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
                class="bg-background text-foreground rounded-sm w-6 h-6 p-1 transition-colors hover:bg-accent"
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
                .map(|q| q.state)
                .filter(|s| s.with(|s| matches!(s, QueryState::Loaded(_))))
                .count()
        });

        let num_fetching = Signal::derive(move || {
            query_state
                .get()
                .values()
                .map(|q| q.state)
                .filter(|s| s.with(|s| matches!(s, QueryState::Fetching(_) | QueryState::Loading)))
                .count()
        });

        let invalid = Signal::derive(move || {
            query_state
                .get()
                .values()
                .map(|q| q.state)
                .filter(|s| s.with(|s| matches!(s, QueryState::Invalid(_))))
                .count()
        });

        let total = Signal::derive(move || query_state.get().len());

        let label_class = "hidden lg:inline-block";
        view! {
            <div class="flex-none flex justify-between w-full overflow-y-hidden items-center border-b border-zinc-800 py-2 px-1">
                <div class="text-transparent bg-clip-text font-bold bg-gradient-to-r from-red-700 to-orange-300 text-base">
                    Leptos Query
                </div>

                <div class="flex gap-2 px-2">
                    <DotBadge option=BadgeOption::Blue>
                        <span class=label_class>Fetching</span>
                        <span>{num_fetching}</span>
                    </DotBadge>

                    <DotBadge option=BadgeOption::Green>
                        <span class=label_class>Loaded</span>
                        <span>{num_loaded}</span>
                    </DotBadge>

                    <DotBadge option=BadgeOption::Red>
                        <span class=label_class>Invalid</span>
                        <span>{invalid}</span>
                    </DotBadge>

                    <DotBadge option=BadgeOption::Gray>
                        <span class=label_class>Total</span>
                        <span>{total}</span>
                    </DotBadge>
                </div>
            </div>
        }
    }

    #[component]
    fn SearchInput() -> impl IntoView {
        let DevtoolsContext { filter, .. } = use_devtools_context();

        view! {
            <div class="relative text-zinc-400 w-72">
                <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
                    <svg class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                        <path
                            fill-rule="evenodd"
                            d="M9 3.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2 9a7 7 0 1112.452 4.391l3.328 3.329a.75.75 0 11-1.06 1.06l-3.329-3.328A7 7 0 012 9z"
                            clip-rule="evenodd"
                        ></path>
                    </svg>
                </div>
                <input
                    id="search"
                    class="block w-full rounded-md border-0 bg-zinc-700 py-1 pl-10 pr-3 text-zinc-100 focus:ring-2 focus:ring-blue-800 sm:text-sm sm:leading-6 placeholder-zinc-400"
                    placeholder="Search"
                    name="search"
                    type="text"
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        filter.set(value);
                    }

                    prop:value=filter
                />
            </div>
        }
    }

    #[component]
    fn QueryRow(entry: QueryCacheEntry) -> impl IntoView {
        let selected_query = use_devtools_context().selected_query;
        let state = entry.state.clone();
        let key = entry.key.clone();
        let observer_count = entry.observer_count.clone();
        let observer = move || {
            let count = observer_count.get();
            if count == 0 {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-gray-100 px-2 py-1 text-xs font-medium text-gray-700">
                        {count}
                    </span>
                }
            } else {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-green-100 px-2 py-1 text-xs font-medium text-green-700">
                        {count}
                    </span>
                }
            }
        };
        view! {
            <li
                class="hover:bg-accent transition-colors flex w-full gap-4 items-center border-b py-1"
                on:click={
                    let key = key.clone();
                    move |_| {
                        if selected_query.get_untracked().map(|q| q.key) == Some(key.clone()) {
                            selected_query.set(None);
                        } else {
                            selected_query.set(Some(entry.clone()))
                        }
                    }
                }
            >
                {observer}
                <RowStateLabel state/>
                <span>{key.0}</span>
            </li>
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

        let badge = Signal::derive(move || match state.get() {
            QueryState::Created | QueryState::Loading | QueryState::Fetching(_) => {
                BadgeOption::Blue
            }
            QueryState::Loaded(_) => BadgeOption::Green,
            QueryState::Invalid(_) => BadgeOption::Red,
        });

        move || {
            view! { <DotBadge option=badge.get()>{state_label.get()}</DotBadge> }
        }
    }

    #[component]
    fn SelectedQuery(query: QueryCacheEntry) -> impl IntoView {
        let query_state = query.state;
        let query_key = query.key.0;

        #[cfg(all(target_arch = "wasm32"))]
        let last_update = Signal::derive(move || {
            use wasm_bindgen::JsValue;
            query_state.get().updated_at().map(|i| {
                let time = JsValue::from_f64(i.0.as_millis() as f64);
                let date = js_sys::Date::new(&time);
                let hours = date.get_hours();
                let minutes = date.get_minutes();
                let seconds = date.get_seconds();
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            })
        });

        #[cfg(not(all(target_arch = "wasm32")))]
        let last_update =
            Signal::derive(move || query_state.get().updated_at().map(|i| i.to_string()));

        // Pretty print the JSON
        #[cfg(all(target_arch = "wasm32"))]
        let value: Signal<Option<String>> = Signal::derive(move || {
            use wasm_bindgen::JsValue;
            let value = query_state.get().data().cloned()?;
            let json = js_sys::JSON::parse(value.as_str()).ok()?;
            let result = js_sys::JSON::stringify_with_replacer_and_space(
                &json,
                &JsValue::NULL,
                &JsValue::from_f64(2.0),
            )
            .ok()?;

            result.as_string()
        });

        #[cfg(not(all(target_arch = "wasm32")))]
        let value: Signal<Option<String>> =
            Signal::derive(move || query_state.get().data().cloned());

        view! {
            <div class="w-1/2 border-l overflow-y-scroll max-h-full">
                <div class="flex flex-col w-full h-full items-center gap-2">
                    <div class="w-full">
                        <div class="text-base text-foreground p-1 bg-accent">Query Details</div>
                        <dl class="px-2 py-1 flex flex-col items-center gap-1 w-full">
                            <div class="flex items-center justify-between text-sm font-medium w-full">
                                <dt class="text-zinc-100">Status</dt>
                                <dd class="text-zinc-200">
                                    <RowStateLabel state=query_state/>
                                </dd>
                            </div>
                            <div class="flex items-center justify-between text-sm font-medium w-full">
                                <dt class="text-zinc-100">Key</dt>
                                <dd class="text-zinc-200">{query_key}</dd>
                            </div>
                            <div class="flex items-center justify-between text-sm font-medium w-full">
                                <dt class="text-zinc-100">Last Update</dt>
                                <dd class="text-zinc-200">{last_update}</dd>
                            </div>
                            <div class="flex items-center justify-between text-sm font-medium w-full">
                                <dt class="text-zinc-100">Active Observers</dt>
                                <dd class="text-zinc-200">{query.observer_count}</dd>
                            </div>
                        </dl>
                    </div>
                    <div class="text-base text-foreground p-1 bg-accent w-full">Query Data</div>
                    <div class="flex-1 p-4 rounded-md bg-zinc-800 shadow-md w-11/12 text-sm ">
                        <pre>{move || value.get().unwrap_or_default()}</pre>
                    </div>
                </div>
            </div>
        }
    }

    #[derive(Clone)]
    enum BadgeOption {
        Blue,
        Green,
        Red,
        Gray,
    }

    #[component]
    fn DotBadge(children: ChildrenFn, option: BadgeOption) -> impl IntoView {
        match option {
            BadgeOption::Blue => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-blue-100 px-2 py-1 text-xs font-medium text-blue-700">
                        <svg class="h-1.5 w-1.5 fill-blue-500" viewBox="0 0 6 6" aria-hidden="true">
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
            BadgeOption::Green => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-green-100 px-2 py-1 text-xs font-medium text-green-700">
                        <svg
                            class="h-1.5 w-1.5 fill-green-500"
                            viewBox="0 0 6 6"
                            aria-hidden="true"
                        >
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
            BadgeOption::Red => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-red-100 px-2 py-1 text-xs font-medium text-red-700">
                        <svg class="h-1.5 w-1.5 fill-red-500" viewBox="0 0 6 6" aria-hidden="true">
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
            BadgeOption::Gray => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-gray-100 px-2 py-1 text-xs font-medium text-gray-700">
                        <svg class="h-1.5 w-1.5 fill-gray-500" viewBox="0 0 6 6" aria-hidden="true">
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
        }
    }
}
