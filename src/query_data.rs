// use leptos::*;

// use crate::*;

// /// Read only Query Data.
// /// Used when introspecting the query cache.
// #[derive(Clone)]
// pub struct QueryData<V>
// where
//     V: Clone + 'static,
// {
//     pub data: Signal<Option<V>>,
//     pub state: Signal<QueryState<V>>,
//     /// If the query is considered stale.
//     pub is_stale: Signal<bool>,
//     /// The last time the query was updated. None if it has not been fetched yet.
//     pub updated_at: Signal<Option<Instant>>,
// }

// impl<V> QueryData<V>
// where
//     V: Clone + 'static,
// {
//     pub(crate) fn from_state<K: Clone>(cx: Scope, state: &Query<K, V>) -> Self {
//         let is_stale = create_rw_signal(cx, false);
//         let data = {
//             let data = state.data;
//             Signal::derive(cx, move || match data.get() {
//                 QueryState::Loading => None,
//                 QueryState::Stale(data)
//                 | QueryState::Fetching { data }
//                 | QueryState::Loaded { data }
//                 | QueryState::Invalid { data } => Some(data),
//             })
//         };

//         let is_stale = is_stale;
//         let updated_at = state.updated_at.into();

//         cleanup_observers(cx, state);

//         Self {
//             data,
//             state: state.data.into(),
//             is_stale: is_stale.into(),
//             updated_at,
//         }
//     }
// }

// fn cleanup_observers<K, V>(cx: Scope, state: &Query<K, V>) {
//     let observers = state.observers.clone();
//     observers.set(observers.get() + 1);

//     on_cleanup(cx, {
//         let observers = observers.clone();
//         move || {
//             observers.set(observers.get() - 1);
//         }
//     });
// }
