use std::collections::HashMap;

use leptos::*;
use leptos_query::*;

#[component]
pub fn LeptosQueryDevtools() -> impl IntoView {
    let client = use_query_client();
    let state = CacheState::new();
    client.register_cache_observer(state.clone());

    view! {
        <Portal>
            <Devtools state=state.clone()/>
        </Portal>
    }
}

#[derive(Clone)]
struct CacheState {
    query_state: RwSignal<HashMap<QueryCacheKey, Signal<QueryState<String>>>>,
}

impl CacheState {
    fn new() -> Self {
        CacheState {
            query_state: create_rw_signal(HashMap::new()),
        }
    }
}

impl CacheObserver for CacheState {
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
fn Devtools(state: CacheState) -> impl IntoView {
    let show = create_rw_signal(false);

    view! {
        <Show
            when=move || show.get()
            fallback=move || {
                view! { <button on:click=move |_| show.set(true)>OPEN</button> }
            }
        >

            <div
                style:width="500px"
                style:height="500px"
                style:position="relative"
            >
                <button on:click=move |_| show.set(false)>CLOSE</button>
                <div>{move || state.query_state.get().len()}</div>
                <ul>
                    <For
                        each=move || state.query_state.get()
                        key=|(key, _)| key.clone()
                        let:entry
                    >
                        <p>{move || {query_state_to_string(entry.1.get())}}</p>
                    </For>

                </ul>
            </div>
        </Show>
    }
}

fn query_state_to_string(q: QueryState<String>) -> String {
    format!("{q:?}")
}
