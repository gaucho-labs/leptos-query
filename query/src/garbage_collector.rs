use std::{cell::Cell, rc::Rc, time::Duration};

use leptos::{leptos_dom::helpers::TimeoutHandle, *};

use crate::query::Query;

#[derive(Clone)]
pub struct GarbageCollector<K, V> {
    query: Rc<Query<K, V>>,
    // Outer options is if option has been set, inner option is the actual value.
    // If inner option is none, then the query should not be garbage collected.
    gc_time: Rc<Cell<GcTime>>,
    handle: Rc<Cell<Option<TimeoutHandle>>>,
}

impl<K, V> std::fmt::Debug for GarbageCollector<K, V>
where
    K: crate::QueryKey,
    V: crate::QueryValue,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GarbageCollector")
            .field("query", &self.query)
            .field("gc_time", &self.gc_time)
            .field("handle", &self.handle)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GcTime {
    // No gc time set.
    None,
    // Some gc time set.
    Some(Duration),
    // Never expires.
    Never,
}

impl GcTime {
    fn from_option(duration: Option<Duration>) -> Self {
        match duration {
            Some(duration) => GcTime::Some(duration),
            None => GcTime::None,
        }
    }
}

impl<K, V> GarbageCollector<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub fn new(query: Query<K, V>) -> Self {
        Self {
            query: Rc::new(query),
            gc_time: Rc::new(Cell::new(GcTime::None)),
            handle: Rc::new(Cell::new(None)),
        }
    }

    /// Keep max gc time.
    pub fn update_gc_time(&self, gc_time: Option<Duration>) {
        match (self.gc_time.get(), gc_time) {
            // Set gc time first time.
            (GcTime::None, gc_time) => {
                self.gc_time.set(GcTime::from_option(gc_time));
            }
            // Greater than current gc time.
            (GcTime::Some(current), Some(gc_time)) if gc_time > current => {
                self.gc_time.set(GcTime::Some(gc_time));
            }
            // Never expires.
            (GcTime::Some(_), None) => {
                self.gc_time.set(GcTime::Never);
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

        if let (GcTime::Some(gc_time), Some(updated_at)) = (gc_time, updated_at) {
            let time_until_gc = crate::util::time_until_stale(updated_at, gc_time);
            let query = self.query.clone();
            let new_handle = set_timeout_with_handle(
                move || {
                    let client = crate::use_query_client();
                    let key = query.get_key();
                    client.cache.evict_query::<K, V>(key);
                },
                time_until_gc,
            )
            .ok();

            self.handle.set(new_handle);
        }
    }

    pub fn disable_gc(&self) {
        if let Some(handle) = self.handle.take() {
            handle.clear();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn create_query() -> GarbageCollector<String, String> {
        let query = Query::<String, String>::new("key".into());
        let gc = query.get_gc().expect("gc should be present");
        gc
    }

    #[test]
    fn test_gc() {
        let gc = create_query();
        assert_eq!(gc.gc_time.get(), GcTime::None);

        gc.update_gc_time(Some(Duration::from_secs(10)));

        assert_eq!(gc.gc_time.get(), GcTime::Some(Duration::from_secs(10)));

        gc.update_gc_time(Some(Duration::from_secs(5)));

        assert_eq!(gc.gc_time.get(), GcTime::Some(Duration::from_secs(10)));

        gc.update_gc_time(None);

        assert_eq!(gc.gc_time.get(), GcTime::Never);
    }
}
