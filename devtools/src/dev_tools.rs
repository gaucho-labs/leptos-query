use leptos::*;
use leptos_query::{
    cache_observer::{
        CacheEvent, CacheObserver, CreatedQuery, ObserverAdded, QueryCacheKey, SerializedQuery,
    },
    *,
};
use std::{collections::HashMap, time::Duration};

use crate::component::*;
use crate::timeout::{time_until_stale, use_timeout};

#[component]
pub(crate) fn InnerDevtools() -> impl IntoView {
    let mounted = create_rw_signal(false);

    create_effect(move |_| {
        mounted.set(true);
    });

    move || {
        if mounted.get() {
            view! { <DevtoolsClient/> }
        } else {
            ().into_view()
        }
    }
}

#[component]
fn DevtoolsClient() -> impl IntoView {
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
            <div class="leptos-query-devtools lq-font-mono">
                <Devtools/>
            </div>
        </Portal>
    }
}

#[derive(Clone)]
struct DevtoolsContext {
    owner: Owner,
    query_state: RwSignal<HashMap<QueryCacheKey, QueryCacheEntry>>,
    open: RwSignal<bool>,
    filter: RwSignal<String>,
    sort: RwSignal<SortOption>,
    order_asc: RwSignal<bool>,
    selected_query: RwSignal<Option<QueryCacheEntry>>,
}

#[derive(Debug, Clone, Copy)]
enum SortOption {
    Time,
    Ascii,
}

impl SortOption {
    fn as_str(&self) -> &str {
        match self {
            SortOption::Time => "Time",
            SortOption::Ascii => "Ascii",
        }
    }
    fn from_string(s: &str) -> Self {
        match s {
            "Ascii" => SortOption::Ascii,
            "Time" => SortOption::Time,
            _ => SortOption::Time,
        }
    }
}

#[derive(Clone)]
struct QueryCacheEntry {
    key: QueryCacheKey,
    state: RwSignal<QueryState<String>>,
    observer_count: RwSignal<usize>,
    gc_time: RwSignal<SettingTime>,
    stale_time: RwSignal<SettingTime>,
    is_stale: Signal<bool>,
    mark_invalid: std::rc::Rc<dyn Fn() -> bool>,
}

fn use_devtools_context() -> DevtoolsContext {
    use_context::<DevtoolsContext>().expect("Devtools Context to be present.")
}

impl DevtoolsContext {
    fn new() -> Self {
        DevtoolsContext {
            owner: Owner::current().expect("Owner to be present"),
            query_state: create_rw_signal(HashMap::new()),
            open: create_rw_signal(false),
            filter: create_rw_signal("".to_string()),
            sort: create_rw_signal(SortOption::Time),
            order_asc: create_rw_signal(false),
            selected_query: create_rw_signal(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SettingTime {
    // No time set.
    None,
    // Some gc time set.
    Some(Duration),
    // Never expires.
    Infinity,
}

impl SettingTime {
    fn max(&self, other: Self) -> Self {
        match (self, other) {
            (SettingTime::None, other) => other,
            (current, SettingTime::None) => *current,
            (SettingTime::Some(current), SettingTime::Some(other)) if other > *current => {
                SettingTime::Some(other)
            }
            (SettingTime::Some(current), SettingTime::Some(_)) => SettingTime::Some(*current),
            (SettingTime::Infinity, _) => SettingTime::Infinity,
            (_, SettingTime::Infinity) => SettingTime::Infinity,
        }
    }

    fn min(&self, other: Self) -> Self {
        match (self, other) {
            (SettingTime::None, other) => other,
            (current, SettingTime::None) => *current,
            (SettingTime::Some(current), SettingTime::Some(other)) if other < *current => {
                SettingTime::Some(other)
            }
            (SettingTime::Some(current), SettingTime::Some(_)) => SettingTime::Some(*current),
            (SettingTime::Infinity, other) => other,
            (current, SettingTime::Infinity) => *current,
        }
    }

    fn from_option(duration: Option<Duration>) -> Self {
        match duration {
            Some(duration) => SettingTime::Some(duration),
            None => SettingTime::Infinity,
        }
    }

    fn to_expiration(self) -> Option<Duration> {
        match self {
            SettingTime::None => None,
            SettingTime::Infinity => None,
            SettingTime::Some(duration) => Some(duration),
        }
    }
}

impl std::fmt::Display for SettingTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingTime::None => write!(f, "None"),
            SettingTime::Some(duration) => write!(f, "{}ms", duration.as_millis()),
            SettingTime::Infinity => write!(f, "Infinity"),
        }
    }
}

impl CacheObserver for DevtoolsContext {
    fn process_cache_event(&self, event: CacheEvent) {
        match event {
            CacheEvent::Created(CreatedQuery {
                key,
                state,
                mark_invalid,
            }) => {
                // Need to create signals with root owner, or else they will be disposed of.
                let entry = with_owner(self.owner, || {
                    let stale_time = create_rw_signal(SettingTime::None);
                    let state = create_rw_signal(state);

                    let is_stale = {
                        let (stale, set_stale) = create_signal(false);

                        let updated_at = Signal::derive(move || state.with(|s| s.updated_at()));

                        use_timeout(move || {
                            match (updated_at.get(), stale_time.get().to_expiration()) {
                                (Some(updated_at), Some(stale_time)) => {
                                    let duration = time_until_stale(updated_at, stale_time);
                                    if duration.is_zero() {
                                        set_stale.set(true);
                                        None
                                    } else {
                                        set_stale.set(false);
                                        set_timeout_with_handle(
                                            move || {
                                                set_stale.set(true);
                                            },
                                            duration,
                                        )
                                        .ok()
                                    }
                                }
                                _ => None,
                            }
                        });

                        stale.into()
                    };

                    QueryCacheEntry {
                        key: key.clone(),
                        state,
                        stale_time,
                        gc_time: create_rw_signal(SettingTime::None),
                        observer_count: create_rw_signal(0),
                        is_stale,
                        mark_invalid,
                    }
                });

                self.query_state.update(|map| {
                    map.insert(key, entry);
                })
            }
            CacheEvent::Removed(key) => self.query_state.update(|map| {
                map.remove(&key);
            }),
            // TODO: Fix this borrow error when using signal update.
            CacheEvent::Updated(SerializedQuery { key, state }) => {
                let map = self.query_state.get_untracked();
                if let Some(entry) = map.get(&key) {
                    entry.state.set(state);
                }
                self.query_state.set(map);
            }
            CacheEvent::ObserverAdded(observer) => {
                let ObserverAdded { key, options } = observer;
                let QueryOptions {
                    stale_time,
                    gc_time,
                    ..
                } = options;
                self.query_state.update(|map| {
                    if let Some(entry) = map.get_mut(&key) {
                        entry.observer_count.update(|c| *c += 1);
                        {
                            let current_gc = entry.gc_time.get_untracked();
                            let setting_gc = SettingTime::from_option(gc_time);

                            let new_gc = current_gc.max(setting_gc);
                            entry.gc_time.set(new_gc);
                        }
                        {
                            let current_stale = entry.stale_time.get_untracked();
                            let setting_stale = SettingTime::from_option(stale_time);
                            let new_stale = current_stale.min(setting_stale);
                            entry.stale_time.set(new_stale);
                        }
                    }
                });
            }
            CacheEvent::ObserverRemoved(key) => {
                self.query_state.update(|map| {
                    if let Some(entry) = map.get_mut(&key) {
                        entry.observer_count.update(|c| {
                            if *c > 0 {
                                *c -= 1
                            }
                        });
                    }
                });
            }
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
        sort,
        order_asc,
        ..
    } = use_devtools_context();

    let query_state = Signal::derive(move || {
        let filter = filter.get().to_ascii_lowercase();

        // Filtered
        let mut query_state = query_state.with(|map| {
            map.iter()
                .filter(|(key, _)| key.0.to_ascii_lowercase().contains(&filter))
                .map(|(_, q)| q)
                .cloned()
                .collect::<Vec<_>>()
        });

        match sort.get() {
            SortOption::Ascii => query_state.sort_by(|a, b| a.key.0.cmp(&b.key.0)),
            SortOption::Time => {
                query_state.sort_by(|a, b| {
                    let a_updated = a.state.with(|s| s.updated_at()).unwrap_or(Instant::now());
                    let b_updated = b.state.with(|s| s.updated_at()).unwrap_or(Instant::now());
                    a_updated.cmp(&b_updated)
                });
            }
        };

        if !order_asc.get() {
            query_state.reverse();
        }

        query_state
    });

    let container_ref = leptos::create_node_ref::<leptos::html::Div>();

    let height_signal = create_rw_signal(500);

    #[cfg(not(feature = "csr"))]
    let handle_drag_start = move |_| ();

    // Drag start handler
    #[cfg(feature = "csr")]
    let handle_drag_start = move |event: web_sys::MouseEvent| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let bounding = container_ref
            .get()
            .expect("container to be mounted")
            .get_bounding_client_rect();

        let height = bounding.height();

        let start_y = event.client_y() as f64;

        let move_closure = Closure::wrap(Box::new(move |move_event: web_sys::MouseEvent| {
            move_event.prevent_default();

            let val_to_add = start_y - move_event.client_y() as f64;

            let new_height = (height + val_to_add).max(200.0);

            height_signal.set(new_height as i32);
        }) as Box<dyn FnMut(_)>)
        .into_js_value();

        // Register the move event listener
        if let Some(window) = web_sys::window() {
            let end = std::rc::Rc::new(std::cell::Cell::new(None::<Closure<dyn FnMut()>>));
            let end_closure = Closure::wrap({
                let window = window.clone();
                let move_closure = move_closure.clone();
                Box::new(move || {
                    window
                        .remove_event_listener_with_callback(
                            "mousemove",
                            move_closure.as_ref().unchecked_ref(),
                        )
                        .unwrap();

                    if let Some(end) = end.take() {
                        let _ = window.remove_event_listener_with_callback(
                            "mouseup",
                            end.as_ref().unchecked_ref(),
                        );
                    }
                }) as Box<dyn FnMut()>
            })
            .into_js_value();

            window
                .add_event_listener_with_callback(
                    "mousemove",
                    move_closure.as_ref().clone().unchecked_ref(),
                )
                .unwrap();

            window
                .add_event_listener_with_callback("mouseup", end_closure.as_ref().unchecked_ref())
                .unwrap();
        }
    };

    view! {
        <Show
            when=move || open.get()
            fallback=move || {
                view! {
                    <button
                        on:click=move |_| open.set(true)
                        class="lq-bg-zinc-200 text-lq-foreground lq-fixed lq-bottom-3 lq-right-3 lq-rounded-full lq-w-12 lq-h-12 hover:-lq-translate-y-1 hover:lq-bg-zinc-300 lq-transition-all lq-duration-200"
                        inner_html=include_str!("logo.svg")
                    ></button>
                }
            }
        >

            <div
                class="lq-bg-lq-background lq-text-lq-foreground lq-px-0 lq-fixed lq-bottom-0 lq-left-0 lq-right-0 lq-z-[1000]"
                style:height=move || format!("{}px", height_signal.get())
                ref=container_ref
            >
                <div
                    class="lq-w-full lq-py-1 lq-bg-lq-background lq-cursor-ns-resize lq-transition-colors hover:lq-bg-lq-border"
                    on:mousedown=handle_drag_start
                ></div>
                <div class="lq-h-full lq-flex lq-flex-col lq-relative">
                    <div class="lq-flex-1 lq-overflow-hidden lq-flex">
                        <div class="lq-flex lq-flex-col lq-flex-1  lq-overflow-x-hidden">
                            <div class="lq-flex-none">
                                <Header/>
                                <div class="lq-py-1 lq-px-2 lq-border-lq-border lq-border-b lq-flex lq-items-center lq-w-full lq-justify-between lq-max-w-full lq-overflow-x-auto lq-gap-2 lq-no-scrollbar">
                                    <div class="lq-flex lq-items-center lq-gap-2">
                                        <SearchInput/>
                                        <SetSort/>
                                        <SetSortOrder/>
                                    </div>
                                    <div class="lq-flex lq-items-center">
                                        <ClearCache/>
                                    </div>
                                </div>
                            </div>

                            <ul class="lq-flex lq-flex-col lq-gap-1 lq-overflow-y-auto">
                                <For each=move || query_state.get() key=|q| q.key.clone() let:entry>
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
                    <div class="lq-absolute -lq-top-6 lq-right-2">
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
            class="lq-bg-lq-background lq-text-lq-foreground lq-rounded-t-sm lq-w-6 lq-h-6 lq-p-1 lq-transition-colors lq-hover:bg-lq-accent"
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

    let label_class = "lq-hidden lg:lq-inline-block";
    view! {
        <div class="lq-flex-none lq-flex lq-justify-between lq-w-full lq-overflow-y-hidden lq-items-center lq-border-b lq-border-lq-border lq-pb-2 lq-px-1">
            <h3 class="lq-pl-2 lq-tracking-tighter lq-text-lg lq-italic lq-text-transparent lq-bg-clip-text lq-font-bold bg-gradient-to-r lq-from-red-800 lq-to-orange-400">
                Leptos Query
            </h3>

            <div class="lq-flex lq-gap-2 lq-px-2">
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
        <div class="lq-relative lq-w-64">
            <div class="lq-pointer-events-none lq-absolute lq-inset-y-0 lq-left-0 lq-flex lq-items-center lq-pl-3 lq-text-zinc-400">
                <svg
                    class="lq-h-4 lq-w-4"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    aria-hidden="true"
                >
                    <path
                        fill-rule="evenodd"
                        d="M9 3.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2 9a7 7 0 1112.452 4.391l3.328 3.329a.75.75 0 11-1.06 1.06l-3.329-3.328A7 7 0 012 9z"
                        clip-rule="evenodd"
                    ></path>
                </svg>
            </div>
            <input
                id="search"
                class="lq-form-input lq-block lq-w-full lq-rounded-md lq-bg-lq-input lq-py-0 lq-pl-10 lq-pr-3 lq-text-lq-input-foreground lq-text-xs lq-leading-6 lq-placeholder-lq-input-foreground lq-border lq-border-lq-border"
                placeholder="Search"
                name="search"
                autocomplete="off"
                type="search"
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
fn SetSort() -> impl IntoView {
    let DevtoolsContext { sort, .. } = use_devtools_context();

    view! {
        <select
            id="countries"
            class="lq-form-select lq-border-lq-border lq-border lq-text-xs lq-rounded-md lq-block lq-w-48 lq-py-1 lq-px-2 lq-bg-lq-input lq-text-lq-input-foreground lq-line-clamp-1"
            value=move || sort.get().as_str().to_string()
            on:change=move |ev| {
                let new_value = event_target_value(&ev);
                let option = SortOption::from_string(&new_value);
                sort.set(option);
            }
        >

            <option value=SortOption::Time.as_str()>Sort by last updated</option>
            <option value=SortOption::Ascii.as_str()>Sort by query key</option>
        </select>
    }
}

#[component]
fn SetSortOrder() -> impl IntoView {
    let DevtoolsContext { order_asc, .. } = use_devtools_context();

    view! {
        <button
            class="lq-bg-lq-input lq-text-lq-input-foreground lq-rounded-md lq-px-2 lq-py-1 lq-text-xs lq-inline-flex lq-items-center lq-gap-1 lq-border lq-border-lq-border"
            on:click=move |_| {
                order_asc.set(!order_asc.get());
            }
        >

            <span class="w-8">{move || { if order_asc.get() { "Asc " } else { "Desc" } }}</span>
            {move || {
                if order_asc.get() {
                    view! {
                        <svg
                            width="15"
                            height="15"
                            viewBox="0 0 15 15"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path
                                d="M7.14645 2.14645C7.34171 1.95118 7.65829 1.95118 7.85355 2.14645L11.8536 6.14645C12.0488 6.34171 12.0488 6.65829 11.8536 6.85355C11.6583 7.04882 11.3417 7.04882 11.1464 6.85355L8 3.70711L8 12.5C8 12.7761 7.77614 13 7.5 13C7.22386 13 7 12.7761 7 12.5L7 3.70711L3.85355 6.85355C3.65829 7.04882 3.34171 7.04882 3.14645 6.85355C2.95118 6.65829 2.95118 6.34171 3.14645 6.14645L7.14645 2.14645Z"
                                fill="currentColor"
                                fill-rule="evenodd"
                                clip-rule="evenodd"
                            ></path>
                        </svg>
                    }
                } else {
                    view! {
                        <svg
                            width="15"
                            height="15"
                            viewBox="0 0 15 15"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path
                                d="M7.5 2C7.77614 2 8 2.22386 8 2.5L8 11.2929L11.1464 8.14645C11.3417 7.95118 11.6583 7.95118 11.8536 8.14645C12.0488 8.34171 12.0488 8.65829 11.8536 8.85355L7.85355 12.8536C7.75979 12.9473 7.63261 13 7.5 13C7.36739 13 7.24021 12.9473 7.14645 12.8536L3.14645 8.85355C2.95118 8.65829 2.95118 8.34171 3.14645 8.14645C3.34171 7.95118 3.65829 7.95118 3.85355 8.14645L7 11.2929L7 2.5C7 2.22386 7.22386 2 7.5 2Z"
                                fill="currentColor"
                                fill-rule="evenodd"
                                clip-rule="evenodd"
                            ></path>
                        </svg>
                    }
                }
            }}

        </button>
    }
}

#[component]
fn ClearCache() -> impl IntoView {
    let cache = leptos_query::use_query_client();

    view! {
        <button
            class="lq-bg-lq-input lq-text-lq-input-foreground lq-rounded-md lq-px-2 lq-py-1 lq-text-xs lq-inline-flex lq-items-center lq-gap-1 lq-border lq-border-lq-border"
            on:click=move |_| {
                cache.clear();
            }
        >

            <svg
                width="15"
                height="15"
                viewBox="0 0 15 15"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <path
                    d="M5.5 1C5.22386 1 5 1.22386 5 1.5C5 1.77614 5.22386 2 5.5 2H9.5C9.77614 2 10 1.77614 10 1.5C10 1.22386 9.77614 1 9.5 1H5.5ZM3 3.5C3 3.22386 3.22386 3 3.5 3H5H10H11.5C11.7761 3 12 3.22386 12 3.5C12 3.77614 11.7761 4 11.5 4H11V12C11 12.5523 10.5523 13 10 13H5C4.44772 13 4 12.5523 4 12V4L3.5 4C3.22386 4 3 3.77614 3 3.5ZM5 4H10V12H5V4Z"
                    fill="currentColor"
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                ></path>
            </svg>
        </button>
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
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-gray-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-gray-700">
                    {count}
                </span>
            }
        } else {
            view! {
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-green-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-green-700">
                    {count}
                </span>
            }
        }
    };
    view! {
        <li
            class="hover:lq-bg-lq-accent lq-transition-colors lq-flex lq-w-full lq-gap-4 lq-items-center lq-border-lq-border lq-border-b lq-p-1"
            on:click={
                let key = key.clone();
                move |_| {
                    if selected_query.get_untracked().map_or(false, |q| q.key == key) {
                        selected_query.set(None);
                    } else {
                        selected_query.set(Some(entry.clone()))
                    }
                }
            }
        >

            {observer}
            <span class="lq-w-[4.5rem]">
                <RowStateLabel state=state.into() is_stale/>
            </span>
            <span class="lq-text-sm">{key.0}</span>
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
            <DotBadge color=badge.get() dot=false>
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
        stale_time,
        gc_time,
    } = query;

    #[cfg(feature = "csr")]
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

    #[cfg(not(feature = "csr"))]
    let last_update = Signal::derive(move || query_state.get().updated_at().map(|i| i.to_string()));

    // Pretty print the JSON
    #[cfg(feature = "csr")]
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

    #[cfg(not(feature = "csr"))]
    let value: Signal<Option<String>> = Signal::derive(move || query_state.get().data().cloned());

    let section_class = "lq-px-2 lq-py-1 lq-flex lq-flex-col lq-items-center lq-gap-1 lq-w-full";
    let entry_class =
        "lq-flex lq-items-center lq-justify-between lq-text-xs lq-font-medium lq-w-full";

    let stale_time = Signal::derive(move || stale_time.get().to_string());

    let gc_time = Signal::derive(move || gc_time.get().to_string());

    view! {
        <div class="lq-w-1/2 lq-overflow-y-scroll lq-max-h-full lq-border-black lq-border-l-4">
            <div class="lq-flex lq-flex-col lq-w-full lq-h-full lq-items-center">
                <div class="lq-w-full">
                    <div class="lq-text-sm lq-text-lq-foreground lq-p-1 lq-bg-lq-accent">
                        Query Details
                    </div>
                    <dl class=section_class>
                        <div class=entry_class>
                            <dt class="lq-text-zinc-100">Status</dt>
                            <dd class="lq-text-zinc-200">
                                <RowStateLabel state=query_state.into() is_stale/>
                            </dd>
                        </div>
                        <div class=entry_class>
                            <dt class="lq-text-zinc-100">Key</dt>
                            <dd class="lq-text-zinc-200">{query_key.0}</dd>
                        </div>
                        <div class=entry_class>
                            <dt class="lq-text-zinc-100">Last Update</dt>
                            <dd class="lq-text-zinc-200">{last_update}</dd>
                        </div>
                        <div class=entry_class>
                            <dt class="lq-text-zinc-100">Active Observers</dt>
                            <dd class="lq-text-zinc-200">{observer_count}</dd>
                        </div>

                        <div class=entry_class>
                            <dt class="lq-text-zinc-100">Stale Time</dt>
                            <dd class="text-zinc-200">{stale_time}</dd>
                        </div>
                        <div class=entry_class>
                            <dt class="lq-text-zinc-100">GC Time</dt>
                            <dd class="lq-text-zinc-200">{gc_time}</dd>
                        </div>
                    </dl>
                </div>
                <div class="lq-w-full">
                    <div class="lq-text-sm lq-text-lq-foreground lq-p-1 lq-bg-lq-accent">
                        Query Actions
                    </div>
                    <div class="lq-flex lq-items-center lq-gap-2 lq-p-1">
                        <Button
                            color=ColorOption::Red
                            on:click=move |_| {
                                mark_invalid();
                            }
                        >

                            Invalidate
                        </Button>
                    </div>
                </div>
                <div class="lq-text-sm lq-text-lq-foreground lq-p-1 lq-bg-lq-accent lq-w-full">
                    Query Data
                </div>
                <div class="lq-flex-1 lq-flex lq-p-2 lq-w-full">
                    <div class="lq-flex-1 lq-p-4 lq-rounded-md lq-bg-zinc-800 lq-shadow-md lq-w-11/12 lq-text-xs lq-overflow-hidden">
                        <pre class="lq-whitespace-pre-wrap lq-break-words">
                            {move || value.get().unwrap_or_default()}
                        </pre>
                    </div>
                </div>
            </div>
        </div>
    }
}
