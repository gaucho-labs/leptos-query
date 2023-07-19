use crate::instant::Instant;
use leptos::*;

#[derive(Clone)]
pub struct QueryResult<V>
where
    V: 'static,
{
    pub data: Signal<Option<V>>,
    pub is_loading: Signal<bool>,
    pub is_stale: Signal<bool>,
    pub is_refetching: Signal<bool>,
    pub updated_at: Signal<Option<Instant>>,
    pub refetch: SignalSetter<()>,
}
impl<V> QueryResult<V> {
    pub fn refetch(&self) {
        self.refetch.set(())
    }
}

impl<V: Copy> Copy for QueryResult<V> where V: 'static {}
