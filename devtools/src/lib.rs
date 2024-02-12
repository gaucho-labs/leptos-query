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

        // Ensure that selected query is closed if it is evicted.
        create_effect({
            move |_| {
                let context = use_devtools_context();

                if let Some(key) = context
                    .selected_query
                    .with(|maybe| maybe.as_ref().map(|q| q.key.clone()))
                {
                    let cache = context.query_state.get();

                    if cache.get(&key).is_none() {
                        context.selected_query.set(None);
                    }
                }
            }
        });

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
        is_stale: Signal<bool>,
        observer_count: Signal<usize>,
        mark_invalid: std::rc::Rc<dyn Fn() -> bool>,
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
                CacheEvent::Created(CreatedQuery {
                    key,
                    state,
                    observer_count,
                    is_stale,
                    mark_invalid,
                }) => self.query_state.update(|map| {
                    let entry = QueryCacheEntry {
                        key: key.clone(),
                        state,
                        observer_count,
                        is_stale,
                        mark_invalid,
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
            let mut query_state = query_state
                .get()
                .into_iter()
                .filter(|(key, _)| key.0.to_ascii_lowercase().contains(&filter))
                .map(|(_, q)| q)
                .collect::<Vec<_>>();
            query_state.sort_by(|a, b| {
                let a_updated = a.state.with(|s| s.updated_at()).unwrap_or(Instant::now());
                let b_updated = b.state.with(|s| s.updated_at()).unwrap_or(Instant::now());
                a_updated.cmp(&b_updated).reverse()
            });
            query_state
        });

        view! {
            <Show
                when=move || open.get()
                fallback=move || {
                    view! {
                        <button
                            on:click=move |_| open.set(true)
                            class="bg-zinc-200 text-foreground absolute bottom-3 right-3 rounded-full w-12 h-12 hover:-translate-y-1 hover:bg-zinc-300 transition-all duration-200"
                            inner_html=include_str!("../../logo.svg")
                        ></button>
                    }
                }
            >

                <div class="bg-background text-foreground px-0 fixed bottom-0 left-0 right-0 h-[500px] z-[1000] border-t-4">
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
                                        key=|q| q.key.clone()
                                        let:entry
                                    >
                                        <QueryRow entry=entry/>
                                    </For>

                                </ul>
                            </div>
                            <Show when=move || {
                                selected_query.get().is_some()
                            }>
                                {move || {
                                    selected_query.get().map(|q| view! { <SelectedQuery query=q/> })
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
                    <DotBadge color=ColorOption::Blue>
                        <span class=label_class>Fetching</span>
                        <span>{num_fetching}</span>
                    </DotBadge>

                    <DotBadge color=ColorOption::Green>
                        <span class=label_class>Loaded</span>
                        <span>{num_loaded}</span>
                    </DotBadge>

                    <DotBadge color=ColorOption::Red>
                        <span class=label_class>Invalid</span>
                        <span>{invalid}</span>
                    </DotBadge>

                    <DotBadge color=ColorOption::Gray>
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
                    autocomplete="off"
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
        let QueryCacheEntry {
            key,
            state,
            observer_count,
            is_stale,
            ..
        } = entry.clone();
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
                <RowStateLabel state is_stale/>
                <span>{key.0}</span>
            </li>
        }
    }

    #[component]
    fn RowStateLabel(state: Signal<QueryState<String>>, is_stale: Signal<bool>) -> impl IntoView {
        let state_label = Signal::derive(move || {
            let is_stale = is_stale.get();
            match state.get() {
                QueryState::Created => "Created",
                QueryState::Loading => "Loading",
                QueryState::Fetching(_) => "Fetching",
                QueryState::Loaded(_) if is_stale => "Stale",
                QueryState::Loaded(_) => "Loaded",
                QueryState::Invalid(_) => "Invalid",
            }
        });

        let badge = Signal::derive(move || {
            let is_stale = is_stale.get();
            match state.get() {
                QueryState::Created | QueryState::Loading | QueryState::Fetching(_) => {
                    ColorOption::Blue
                }
                QueryState::Loaded(_) if is_stale => ColorOption::Yellow,
                QueryState::Loaded(_) => ColorOption::Green,
                QueryState::Invalid(_) => ColorOption::Red,
            }
        });

        move || {
            view! {
                <DotBadge color=badge.get()>
                    {state_label}
                </DotBadge>
            }
        }
    }

    #[component]
    fn SelectedQuery(query: QueryCacheEntry) -> impl IntoView {
        let QueryCacheEntry {
            key: query_key,
            state: query_state,
            is_stale,
            observer_count,
            mark_invalid,
        } = query;

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
            .ok()
            .map(|r| r.as_string())
            // If value is not json, just present value.
            .unwrap_or(Some(value));

            result
        });

        #[cfg(not(all(target_arch = "wasm32")))]
        let value: Signal<Option<String>> =
            Signal::derive(move || query_state.get().data().cloned());

        let section_class = "px-2 py-1 flex flex-col items-center gap-1 w-full";
        let entry_class = "flex items-center justify-between text-sm font-medium w-full";

        view! {
            <div class="w-1/2 border-l overflow-y-scroll max-h-full">
                <div class="flex flex-col w-full h-full items-center">
                    <div class="w-full">
                        <div class="text-base text-foreground p-1 bg-accent">Query Details</div>
                        <dl class=section_class>
                            <div class=entry_class>
                                <dt class="text-zinc-100">Status</dt>
                                <dd class="text-zinc-200">
                                    <RowStateLabel state=query_state is_stale/>
                                </dd>
                            </div>
                            <div class=entry_class>
                                <dt class="text-zinc-100">Key</dt>
                                <dd class="text-zinc-200">{query_key.0}</dd>
                            </div>
                            <div class=entry_class>
                                <dt class="text-zinc-100">Last Update</dt>
                                <dd class="text-zinc-200">{last_update}</dd>
                            </div>
                            <div class=entry_class>
                                <dt class="text-zinc-100">Active Observers</dt>
                                <dd class="text-zinc-200">{observer_count}</dd>
                            </div>
                        </dl>
                    </div>
                    <div class="w-full">
                        <div class="text-base text-foreground p-1 bg-accent">Query Actions</div>
                        <div class="flex items-center gap-2 p-1">
                            <Button color={ColorOption::Red} on:click={move |_| {mark_invalid();}}>
                                Invalidate
                            </Button>
                        </div>
                    </div>
                    <div class="text-base text-foreground p-1 bg-accent w-full">Query Data</div>
                    <div class="flex-1 flex p-2 w-full">
                        <div class="flex-1 p-4 rounded-md bg-zinc-800 shadow-md w-11/12 text-sm ">
                            <pre>{move || value.get().unwrap_or_default()}</pre>
                        </div>
                    </div>
                </div>
            </div>
        }
    }

    #[derive(Clone)]
    enum ColorOption {
        Blue,
        Green,
        Red,
        Yellow,
        Gray,
    }

    #[component]
    fn DotBadge(children: ChildrenFn, color: ColorOption) -> impl IntoView {
        match color {
            ColorOption::Blue => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-blue-100 px-2 py-1 text-xs font-medium text-blue-700">
                        <svg class="h-1.5 w-1.5 fill-blue-500" viewBox="0 0 6 6" aria-hidden="true">
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
            ColorOption::Green => {
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
            ColorOption::Red => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-red-100 px-2 py-1 text-xs font-medium text-red-700">
                        <svg class="h-1.5 w-1.5 fill-red-500" viewBox="0 0 6 6" aria-hidden="true">
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
            ColorOption::Gray => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-gray-100 px-2 py-1 text-xs font-medium text-gray-700">
                        <svg class="h-1.5 w-1.5 fill-gray-500" viewBox="0 0 6 6" aria-hidden="true">
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
            ColorOption::Yellow => {
                view! {
                    <span class="inline-flex items-center gap-x-1.5 rounded-md bg-yellow-100 px-2 py-1 text-xs font-medium text-yellow-700">
                        <svg
                            class="h-1.5 w-1.5 fill-yellow-500"
                            viewBox="0 0 6 6"
                            aria-hidden="true"
                        >
                            <circle cx="3" cy="3" r="3"></circle>
                        </svg>
                        {children}
                    </span>
                }
            }
        }
    }

    #[component]
    fn Button(
        children: ChildrenFn,
        color: ColorOption,
        #[prop(attrs)] attributes: Vec<(&'static str, Attribute)>,
    ) -> impl IntoView {
        match color {
            ColorOption::Blue => view! {
                <button
                    type="button"
                    class="text-white bg-blue-700 hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300 font-medium rounded-full text-xs px-2 py-1 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
                    {..attributes}
                >
                    {children}
                </button>
            },
            ColorOption::Green => view! {
                <button
                    type="button"
                    class="text-white bg-green-700 hover:bg-green-800 focus:outline-none focus:ring-4 focus:ring-green-300 font-medium rounded-full text-xs px-2 py-1 text-center  dark:bg-green-600 dark:hover:bg-green-700 dark:focus:ring-green-800"
                    {..attributes}
                >
                    {children}
                </button>
            },
            ColorOption::Red => view! {
                <button
                    type="button"
                    class="text-white bg-red-700 hover:bg-red-800 focus:outline-none focus:ring-4 focus:ring-red-300 font-medium rounded-full text-xs px-2 py-1 text-center  dark:bg-red-600 dark:hover:bg-red-700 dark:focus:ring-red-900"
                    {..attributes}
                >
                    {children}
                </button>
            },
            ColorOption::Yellow => view! {
                <button
                    type="button"
                    class="text-white bg-yellow-400 hover:bg-yellow-500 focus:outline-none focus:ring-4 focus:ring-yellow-300 font-medium rounded-full text-xs px-2 py-1 text-center  dark:focus:ring-yellow-900"
                    {..attributes}
                >
                    {children}
                </button>
            },
            ColorOption::Gray => view! {
                <button
                    type="button"
                    class="text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-xs px-2 py-1  dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 dark:focus:ring-gray-700"
                    {..attributes}
                >
                    {children}
                </button>
            },
        }
    }
}
