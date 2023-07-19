#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! # About Query
//!
//!
//! Leptos Query is a asynchronous state management library for [Leptos](https://leptos.dev/),
//!
//! Heavily inspired by [react-query](https://react-query.tanstack.com/)
//!
//! Queries are useful for data fetching, caching, and synchronization with server state.
//!
//! A Query provides:
//! - caching
//! - de-duplication
//! - invalidation
//! - background refetching
//! - refetch intervals
//! - memory management with cache lifetimes
//!
//! # A Simple Example
//! ```
//!
//! // Create a Newtype for MonkeyId.
//! #[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
//! struct MonkeyId(String);
//!
//!
//! // Monkey fetcher.
//! async fn get_monkey(id: MonkeyId) -> Monkey {
//! ...
//! }
//!
//! // Query for a Monkey.
//! fn use_monkey_query(cx: Scope, id: MonkeyId) -> QueryResult<Monkey> {
//!     leptos_query::use_query(
//!         cx,
//!         id,
//!         |id| async move { get_monkey(id).await },
//!         QueryOptions {
//!             default_value: None,
//!             refetch_interval: None,
//!             resource_option: ResourceOption::NonBlocking,
//!             stale_time: Some(Duration::from_secs(5)),
//!             cache_time: Some(Duration::from_secs(30)),
//!         },
//!     )
//! }
//!
//! #[component]
//! fn MonkeyView(cx: Scope, id: MonkeyId) -> impl IntoView {
//!     let query = use_monkey_query(cx, id);
//!     let QueryResult {
//!         data,
//!         is_loading,
//!         is_refetching,
//!         ..
//!     } = query;
//!
//!     view! { cx,
//!       // You can use the query result data here.
//!       // Everything is reactive.
//!     }
//! }
//!
//! ```
//!

mod instant;
mod query_client;
mod query_options;
mod query_result;
mod query_state;
mod use_query;
mod util;

pub use query_client::*;
pub use query_options::*;
pub use query_result::*;
use query_state::*;
pub use use_query::*;
