use crate::instant::get_instant;
use crate::query_executor::{create_executor, synchronize_state};
use crate::query_result::QueryResult;
use crate::{get_state, Query, QueryData, QueryOptions, QueryState, ResourceOption};
use leptos::*;
use std::future::Future;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// Creates a query. Useful for data fetching, caching, and synchronization with server state.
///
/// A Query provides:
/// - caching
/// - de-duplication
/// - invalidation
/// - background refetching
/// - refetch intervals
/// - memory management with cache lifetimes
///
///
/// Details:
/// - A query is unique per Key `K`.
/// - A query Key type `K` must only correspond to ONE UNIQUE Value `V` Type.
/// - Meaning a query Key type `K` cannot correspond to multiple Value `V` Types.
///
/// Example
/// ```
/// use leptos::*;
/// use leptos_query::*;
/// use std::time::Duration;
/// use serde::*;
///
/// // Data type.
/// #[derive(Clone, Deserialize, Serialize)]
/// struct Monkey {
///     name: String,
/// }
///
/// // Create a Newtype for MonkeyId.
/// #[derive(Clone, PartialEq, Eq, Hash)]
/// struct MonkeyId(String);
///
/// // Monkey fetcher.
/// async fn get_monkey(id: MonkeyId) -> Monkey {
///     todo!()
/// }
///
/// // Query for a Monkey.
/// fn use_monkey_query(cx: Scope, id: impl Fn() -> MonkeyId + 'static) -> QueryResult<Monkey> {
///     leptos_query::use_query(
///         cx,
///         id,
///         get_monkey,
///         QueryOptions {
///             default_value: None,
///             refetch_interval: None,
///             resource_option: ResourceOption::NonBlocking,
///             stale_time: Some(Duration::from_secs(5)),
///             cache_time: Some(Duration::from_secs(60)),
///         },
///     )
/// }
///
/// ```
///
pub fn use_query<K, V, Fu>(
    cx: Scope,
    key: impl Fn() -> K + 'static,
    fetcher: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryResult<V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
    Fu: Future<Output = V> + 'static,
{
    // Find relevant state.
    let state = get_state(cx, key);

    // Update options.
    create_isomorphic_effect(cx, {
        let options = options.clone();
        move |_| {
            let (state, new) = state.get();
            if new {
                state.overwrite_options(options.clone())
            } else {
                state.update_options(cx, options.clone())
            }
        }
    });

    let state = Signal::derive(cx, move || state.get().0);

    let resource_fetcher = move |state: Query<K, V>| {
        async move {
            match state.state.get_untracked() {
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
        match options.resource_option {
            ResourceOption::NonBlocking => create_resource_with_initial_value(
                cx,
                move || state.get(),
                resource_fetcher,
                if default.is_some() {
                    Some(ResourceData(default))
                } else {
                    None
                },
            ),
            ResourceOption::Blocking => {
                create_blocking_resource(cx, move || state.get(), resource_fetcher)
            }
        }
    };

    // Ensure always latest value.
    create_isomorphic_effect(cx, move |_| {
        let state = state.get().state.get();
        if let QueryState::Loaded(data) = state {
            // Interrupt Suspense.
            if resource.loading().get_untracked() {
                resource.set(ResourceData(Some(data.data)));
            } else {
                resource.refetch();
            }
        }
    });

    let executor = Rc::new(create_executor(state, fetcher));

    synchronize_state(cx, state, executor.clone());

    // Ensure key changes are considered.
    create_isomorphic_effect(cx, {
        let executor = executor.clone();
        move |prev_state: Option<Query<K, V>>| {
            let state = state.get();
            if let Some(prev_state) = prev_state {
                if prev_state != state {
                    if let QueryState::Created = state.state.get_untracked() {
                        executor()
                    }
                }
            }
            state
        }
    });

    let data = Signal::derive(cx, {
        let executor = executor.clone();
        move || {
            let read = resource.read(cx).and_then(|r| r.0);
            let state = state.get_untracked();

            // First Read.
            // Putting this in an effect will cause it to always refetch needlessly on the client after SSR.
            if read.is_none() && state.state.get_untracked().data().is_none() {
                executor()
            // SSR edge case.
            // Given hydrate can happen before resource resolves, signals on the client can be out of sync with resource.
            } else if let Some(ref data) = read {
                if let QueryState::Created = state.state.get_untracked() {
                    let updated_at = get_instant();
                    let data = QueryData {
                        data: data.clone(),
                        updated_at,
                    };
                    state.state.set(QueryState::Loaded(data))
                }
            }
            read
        }
    });

    QueryResult::new(cx, state, data, executor)
}

const LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24);

async fn sleep(duration: Duration) {
    use cfg_if::cfg_if;
    cfg_if! {
        if #[cfg(all(target_arch = "wasm32", any(feature = "hydrate")))] {
            gloo_timers::future::sleep(duration).await;
        } else if #[cfg(feature = "ssr")] {
            tokio::time::sleep(duration).await;
        } else {
            let _ = duration;
            debug_warn!("You are missing a Cargo feature for leptos_query. Please use one of 'ssr' or 'hydrate'")
        }
    }
}

/// Wrapper type to enable using `Serializable`
#[derive(Clone, Debug)]
struct ResourceData<V>(Option<V>);

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
