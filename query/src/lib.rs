#![warn(missing_docs)]
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
//! - cancellation
//! - debugging tools
//! - optimistic updates
//! - client side cache persistance (localstorage, indexdb, custom, etc.)
//!
//!
//! ## The main entry points to using Queries are:
//! - [`create_query`](crate::create_query::create_query()) - **Recommended**: Creates a [`QueryScope`] which encapsulates `use_query` and other methods for managing queries.
//! - [`use_query`][crate::use_query::use_query()] - A query primitive for reading, caching, and refetching data.
//!
//! # Feature Flags
//! - `csr` Client-side rendering: Use queries on the client.
//! - `ssr` Server-side rendering: Initiate queries on the server.
//! - `hydrate` Hydration: Ensure that queries are hydrated on the client, when using server-side rendering.
//! - `local_storage` - Enables local storage persistance for queries.
//!
//! # A Simple Example
//!
//! In the root of your App, provide a query client with [provide_query_client] or [provide_query_client_with_options] if you want to override the default options.
//!
//! ```rust
//! use leptos_query::*;
//! use leptos::*;
//!
//! #[component]
//! pub fn App() -> impl IntoView {
//!     // Provides Query Client for entire app.
//!     provide_query_client();
//!
//!     // Rest of App...
//! }
//! ```
//!
//! Then make a query function with [`create_query`][crate::create_query::create_query()]
//!
//! ```
//! use leptos::*;
//! use leptos_query::*;
//!
//!
//! // Query for a track.
//! fn track_query() -> QueryScope<TrackId, TrackData> {
//!     create_query(
//!         get_track,
//!         QueryOptions::default(),
//!     )
//! }
//!
//! // Make a key type.
//! #[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
//! struct TrackId(i32);
//!
//! // The result of the query fetcher.
//! #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
//! struct TrackData {
//!    name: String,
//! }
//!
//! // Query fetcher.
//! async fn get_track(id: TrackId) -> TrackData {
//!     todo!()
//! }
//!
//! ```
//!
//! Now you can use the query in any component in your app.
//!
//! ```rust
//! # use serde::*;
//! #
//! # // Make a key type.
//! # #[derive(Debug, Copy, Clone,  Hash, Eq, PartialEq)]
//! # struct TrackId(i32);
//! #
//! # // The result of the query fetcher.
//! # #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
//! # struct TrackData {
//! #    name: String,
//! # }
//! #
//! # fn track_query() -> QueryScope<TrackId, TrackData> {
//! #     create_query(
//! #         get_track,
//! #         QueryOptions::default(),
//! #     )
//! # }
//! # async fn get_track(id: TrackId) -> TrackData {
//! #    todo!()
//! # }
//! #
//! use leptos::*;
//! use leptos_query::*;
//!
//! #[component]
//! fn TrackView(id: TrackId) -> impl IntoView {
//!     let QueryResult {
//!         data,
//!         ..
//!     } = track_query().use_query(move || id);
//!
//!     view! {
//!        <div>
//!            // Query data should be read inside a Transition/Suspense component.
//!            <Transition
//!                fallback=move || {
//!                    view! { <h2>"Loading..."</h2> }
//!                }>
//!                {move || {
//!                     data
//!                         .get()
//!                         .map(|track| {
//!                            view! { <h2>{track.name}</h2> }
//!                         })
//!                }}
//!            </Transition>
//!        </div>
//!     }
//! }
//! ```
//!

/// Subcriptions to cache-wide query events.
pub mod cache_observer;
mod create_query;
mod garbage_collector;
mod instant;
mod query;
mod query_cache;
mod query_client;
mod query_executor;
mod query_observer;
mod query_options;
/// Utitities for client side query persistance.
pub mod query_persister;
mod query_result;
mod query_state;
mod use_query;
mod util;

pub use create_query::*;
pub use instant::*;
pub use query_client::*;
pub use query_executor::*;
pub use query_options::*;
pub use query_result::*;
pub use query_state::*;
pub use use_query::*;

/// Convenience trait for query key requirements.
pub trait QueryKey: std::fmt::Debug + Clone + std::hash::Hash + Eq {}
impl<K> QueryKey for K where K: std::fmt::Debug + Clone + std::hash::Hash + Eq {}

/// Convenience trait for query value requirements.
pub trait QueryValue: std::fmt::Debug + Clone + leptos::Serializable {}
impl<V> QueryValue for V where V: std::fmt::Debug + Clone + leptos::Serializable {}
