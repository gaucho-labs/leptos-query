#![forbid(unsafe_code)]

//! # About Query
//!
//!
//! Leptos Query is a asynchronous state management library for [Leptos](https://github.com/leptos-rs/leptos).
//!
//! Heavily inspired by [Tanstack Query](https://tanstack.com/query/latest/).
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
//!
//! In the root of your App, provide a query client:
//!
//! ```rust
//! use leptos_query::*;
//! use leptos::*;
//!
//! #[component]
//! pub fn App(cx: Scope) -> impl IntoView {
//!     // Provides Query Client for entire app.
//!     provide_query_client(cx);
//!
//!     // Rest of App...
//! }
//! ```
//!
//! Then make a query funciton:
//!
//! ```
//! use leptos::*;
//! use leptos_query::*;
//! use std::time::Duration;
//! use serde::*;
//!
//! // Data type.
//! #[derive(Clone, Deserialize, Serialize)]
//! struct Monkey {
//!     name: String,
//! }
//!
//! // Create a Newtype for MonkeyId.
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! struct MonkeyId(String);
//!
//! // Monkey fetcher.
//! async fn get_monkey(id: MonkeyId) -> Monkey {
//!     todo!()
//! }
//!
//! // Query for a Monkey.
//! fn use_monkey_query(cx: Scope, id: impl Fn() -> MonkeyId + 'static) -> QueryResult<Monkey> {
//!     leptos_query::use_query(
//!         cx,
//!         id,
//!         get_monkey,
//!         QueryOptions {
//!             default_value: None,
//!             refetch_interval: None,
//!             resource_option: ResourceOption::NonBlocking,
//!             stale_time: Some(Duration::from_secs(5)),
//!             cache_time: Some(Duration::from_secs(60)),
//!         },
//!     )
//! }
//!
//! ```
//!
//! Now you can use the query in any component in your app.
//!
//! ```rust
//!
//! #[component]
//! fn MonkeyView(cx: Scope, id: MonkeyId) -> impl IntoView {
//!     let query = use_monkey_query(cx, move || id.clone());
//!     let QueryResult {
//!         data,
//!         is_loading,
//!         is_fetching,
//!         is_stale
//!         ..
//!     } = query;
//!
//!     view! { cx,
//!       // You can use the query result data here.
//!       // Everything is reactive.
//!        <div>
//!            <div>
//!                <span>"Loading Status: "</span>
//!                <span>{move || { if is_loading.get() { "Loading..." } else { "Loaded" } }}</span>
//!            </div>
//!            <div>
//!                <span>"Fetching Status: "</span>
//!                <span>
//!                    {move || { if is_fetching.get() { "Fetching..." } else { "Idle" } }}
//!                </span>
//!            </div>
//!            <div>
//!                <span>"Stale Status: "</span>
//!                <span>
//!                    {move || { if is_stale.get() { "Stale" } else { "Fresh" } }}
//!                </span>
//!            </div>
//!            // Query data should be read inside a Transition/Suspense component.
//!            <Transition
//!                fallback=move || {
//!                    view! { cx, <h2>"Loading..."</h2> }
//!                }>
//!                {move || {
//!                    data()
//!                        .map(|monkey| {
//!                            view! { cx, <h2>{monkey.name}</h2> }
//!                        })
//!                }}
//!            </Transition>
//!        </div>
//!     }
//! }
//! ```
//!

mod instant;
mod query;
mod query_client;
mod query_data;
mod query_executor;
mod query_options;
mod query_result;
mod query_state;
mod use_query;
mod util;

pub use instant::*;
use query::*;
pub use query_client::*;
pub use query_data::*;
pub use query_executor::*;
pub use query_options::*;
pub use query_result::*;
pub use query_state::*;
pub use use_query::*;
