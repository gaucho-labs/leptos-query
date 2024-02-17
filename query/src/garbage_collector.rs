use std::{cell::Cell, rc::Rc, time::Duration};

use leptos::{leptos_dom::helpers::TimeoutHandle, *};

use crate::query::Query;

#[derive(Clone)]
pub(crate) struct GarbageCollector<K, V> {
    query: Rc<Query<K, V>>,
    gc_time: Rc<Cell<Option<Duration>>>,
    handle: Rc<Cell<Option<TimeoutHandle>>>,
}

impl<K, V> GarbageCollector<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn new(query: Query<K, V>) -> Self {
        Self {
            query: Rc::new(query),
            gc_time: Rc::new(Cell::new(None)),
            handle: Rc::new(Cell::new(None)),
        }
    }

    /// Keep max gc time.
    pub fn update_gc_time(&self, gc_time: Option<Duration>) {
        match (self.gc_time.get(), gc_time) {
            (Some(current), Some(gc_time)) if gc_time > current => {
                self.gc_time.set(Some(gc_time));
            }
            (None, Some(gc_time)) => {
                self.gc_time.set(Some(gc_time));
            }
            _ => {}
        }
    }

    pub fn enable_gc(&self) {
        if self.handle.get().is_some() {
            return;
        }

        let gc_time = self.gc_time.get();
        let updated_at = self.query.get_updated_at();

        match (gc_time, updated_at) {
            (Some(gc_time), Some(updated_at)) => {
                let time_until_gc = crate::util::time_until_stale(updated_at, gc_time);
                let query = self.query.clone();
                let new_handle = set_timeout_with_handle(
                    move || {
                        let client = crate::use_query_client();
                        let key = query.get_key();
                        client.cache.evict_query::<K, V>(&key);
                    },
                    time_until_gc,
                )
                .ok();

                self.handle.set(new_handle);
            }
            _ => (),
        }
    }

    pub fn disable_gc(&self) {
        if let Some(handle) = self.handle.take() {
            handle.clear();
        }
    }
}
