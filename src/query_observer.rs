use leptos::*;

use crate::QueryState;

// TODO: On drop?

#[derive(Debug)]
pub(crate) struct QueryObserver<V: 'static> {
    id: u32,
    state: RwSignal<QueryState<V>>,
}

impl<V> QueryObserver<V> {
    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn new(id: u32, state: QueryState<V>) -> Self {
        QueryObserver {
            id,
            state: RwSignal::new(state),
        }
    }

    pub(crate) fn update(&self, new_state: QueryState<V>) {
        self.state.set(new_state)
    }

    pub(crate) fn state_signal(&self) -> Signal<QueryState<V>> {
        self.state.into()
    }
}

impl<V> Drop for QueryObserver<V> {
    fn drop(&mut self) {
        logging::log!("QueryObserver dropped");
    }
}

impl<V> PartialEq for QueryObserver<V> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
