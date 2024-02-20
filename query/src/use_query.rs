use crate::query::Query;
use crate::query_observer::{ListenerKey, QueryObserver};
use crate::query_result::QueryResult;
use crate::{
    query_is_supressed, use_query_client, QueryOptions, QueryState, RefetchFn, ResourceOption,
};
use leptos::*;
use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;

/// Creates a query. Useful for data fetching, caching, and synchronization with server state.
///
/// A Query provides:
/// - Caching
/// - De-duplication
/// - Invalidation
/// - Background refetching
/// - Refetch intervals
/// - Memory management with cache lifetimes
///
///
/// Example
/// ```
/// use leptos::*;
/// use leptos_query::*;
/// use std::time::Duration;
/// use serde::*;
///
/// // Query key.
/// #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
/// struct UserId(i32);
///
/// // Data type.
/// #[derive(Debug, Clone, Deserialize, Serialize)]
/// struct UserData {
///     name: String,
/// }
///
/// // Fetcher
/// async fn get_user(id: UserId) -> UserData {
///     todo!()
/// }
///
/// // Query for a User.
/// fn use_user_query(id: impl Fn() -> UserId + 'static) -> QueryResult<UserData, impl RefetchFn> {
///     leptos_query::use_query(
///         id,
///         get_user,
///         QueryOptions {
///             stale_time: Some(Duration::from_secs(5)),
///             gc_time: Some(Duration::from_secs(60)),
///             ..QueryOptions::default()
///         },
///     )
/// }
///
/// ```
///
pub fn use_query<K, V, Fu>(
    key: impl Fn() -> K + 'static,
    fetcher: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryResult<V, impl RefetchFn>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
    Fu: Future<Output = V> + 'static,
{
    let options = options.validate();
    // Find relevant state.
    let query = use_query_client().cache.get_query_signal(key);

    let query_state = register_observer_handle_cleanup(fetcher, query, options.clone());

    let resource_fetcher = move |query: Query<K, V>| {
        async move {
            match query.get_state() {
                // Immediately provide cached value.
                QueryState::Loaded(data)
                | QueryState::Invalid(data)
                | QueryState::Fetching(data) => ResourceData(Some(data.data)),

                // Suspend indefinitely and wait for interruption.
                QueryState::Created | QueryState::Loading => {
                    sleep(LONG_TIME).await;
                    ResourceData(None)
                }
            }
        }
    };

    let resource: Resource<Query<K, V>, ResourceData<V>> = {
        let default = options.default_value;
        match options.resource_option.unwrap_or_default() {
            ResourceOption::NonBlocking => create_resource_with_initial_value(
                move || query.get(),
                resource_fetcher,
                default.map(|default| ResourceData(Some(default))),
            ),
            ResourceOption::Blocking => {
                create_blocking_resource(move || query.get(), resource_fetcher)
            }
            ResourceOption::Local => create_local_resource_with_initial_value(
                move || query.get(),
                resource_fetcher,
                default.map(|default| ResourceData(Some(default))),
            ),
        }
    };

    // Ensure latest data in resource.
    create_isomorphic_effect(move |_| {
        query_state.track();
        // If query is supressed, we have to make sure we don't refetch to avoid calling spawn_local.
        if !query_is_supressed() {
            resource.refetch();
        }
    });

    let data = Signal::derive({
        move || {
            let read = resource.get().and_then(|r| r.0);
            let query = query.get_untracked();

            // First Read.
            // Putting this in an effect will cause it to always refetch needlessly on the client after SSR.
            if read.is_none() && query.with_state(|state| matches!(state, QueryState::Created)) {
                query.execute()
            }

            // SSR edge case.
            // Given hydrate can happen before resource resolves, signals on the client can be out of sync with resource.
            // Need to force insert the resource data into the query state.
            #[cfg(feature = "hydrate")]
            if let Some(ref data) = read {
                if query.with_state(|state| matches!(state, QueryState::Created)) {
                    let data = crate::QueryData::now(data.clone());
                    query.set_state(QueryState::Loaded(data));
                }
            }
            read
        }
    });

    QueryResult {
        data,
        state: query_state,
        is_loading: Signal::derive(move || {
            query_state.with(|state| matches!(state, QueryState::Loading))
        }),
        is_fetching: Signal::derive(move || {
            query_state.with(|state| matches!(state, QueryState::Loading | QueryState::Fetching(_)))
        }),
        is_invalid: Signal::derive(move || {
            query_state.with(|state| matches!(state, QueryState::Invalid(_)))
        }),
        refetch: move || query.with_untracked(|q| q.execute()),
    }
}

const LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24);

async fn sleep(duration: Duration) {
    use cfg_if::cfg_if;
    cfg_if! {
        if #[cfg(any(feature = "hydrate", feature = "csr"))] {
            gloo_timers::future::sleep(duration).await;
        } else if #[cfg(feature = "ssr")] {
            tokio::time::sleep(duration).await;
        } else {
            let _ = duration;
            logging::debug_warn!("You are missing a Cargo feature for leptos_query. Please enable one of 'ssr', 'hydrate', or 'csr'.");
        }
    }
}

/// Wrapper type to enable using `Serializable`
#[derive(Clone, Debug)]
pub struct ResourceData<V>(Option<V>);

impl<V> Serializable for ResourceData<V>
where
    V: Serializable,
{
    fn ser(&self) -> Result<String, SerializationError> {
        if let Some(ref value) = self.0 {
            value.ser()
        } else {
            Ok("null".to_string())
        }
    }

    fn de(bytes: &str) -> Result<Self, SerializationError> {
        match bytes {
            "" | "null" => Ok(ResourceData(None)),
            v => <V>::de(v).map(Some).map(ResourceData),
        }
    }
}

pub(crate) fn register_observer_handle_cleanup<K, V, Fu>(
    fetcher: impl Fn(K) -> Fu + 'static,
    query: Memo<Query<K, V>>,
    options: QueryOptions<V>,
) -> Signal<QueryState<V>>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
    Fu: Future<Output = V> + 'static,
{
    let state_signal = RwSignal::new(query.get_untracked().get_state());
    let observer = Rc::new(QueryObserver::with_fetcher(
        fetcher,
        options,
        query.get_untracked(),
    ));
    let listener = Rc::new(Cell::new(None::<ListenerKey>));

    create_isomorphic_effect({
        let observer = observer.clone();
        let listener = listener.clone();
        move |_| {
            // Ensure listener is set
            if listener.get().is_none() {
                let listener_id = observer.add_listener(move |state| {
                    state_signal.set(state.clone());
                });
                listener.set(Some(listener_id));
            }

            // Update
            let query = query.get();
            state_signal.set(query.get_state());
            observer.update_query(Some(query));
        }
    });

    on_cleanup(move || {
        if let Some(listener_id) = listener.take() {
            if !observer.remove_listener(listener_id) {
                logging::debug_warn!("Failed to remove listener.");
            }
        }
        observer.cleanup()
    });

    state_signal.into()
}
