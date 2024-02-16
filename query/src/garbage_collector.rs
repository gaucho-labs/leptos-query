use std::{marker::PhantomData, time::Duration};

use leptos::{leptos_dom::helpers::TimeoutHandle, *};

use crate::Instant;

#[derive(Clone)]
pub(crate) struct GarbageCollector<K, V> {
    key: K,
    _value: PhantomData<V>,
    enabled: RwSignal<bool>,
    last_update: RwSignal<crate::Instant>,
    gc_time: Signal<Option<Duration>>,
}

impl<K, V> GarbageCollector<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn new(key: K, gc_time: Signal<Option<Duration>>) -> Self {
        let gc = Self {
            key,
            _value: PhantomData,
            enabled: RwSignal::new(true),
            gc_time,
            last_update: RwSignal::new(crate::Instant::now()),
        };

        gc.start_effect();

        gc
    }

    pub(crate) fn new_update(&self, instant: Instant) {
        self.last_update.set(instant);
    }

    pub fn enable_gc(&self) {
        self.enabled.set(true);
    }

    pub fn disable_gc(&self) {
        self.enabled.set(false);
    }

    pub fn start_effect(&self) {
        let gc_time = self.gc_time.clone();
        let last_update = self.last_update.clone();
        let key = self.key.clone();
        let enabled = self.enabled.clone();

        create_effect({
            move |maybe_timeout_handle: Option<Option<TimeoutHandle>>| {
                if let Some(timeout_handle) = maybe_timeout_handle.flatten() {
                    timeout_handle.clear();
                }
                // Ensure enabled
                if !enabled.get() {
                    return None;
                }

                let gc_time = gc_time.get();
                let last_update = last_update.get();
                let key = key.clone();

                if let Some(gc_time) = gc_time {
                    let time_until_gc = crate::util::time_until_stale(last_update, gc_time);

                    set_timeout_with_handle(
                        move || {
                            let client = crate::use_query_client();
                            client.cache.evict_query::<K, V>(&key);
                        },
                        time_until_gc,
                    )
                    .ok()
                } else {
                    None
                }
            }
        });
    }
}
