# Leptos Query

<p align="center">
    <a href="https://github.com/nicoburniske/leptos_query">
        <img src="https://raw.githubusercontent.com/nicoburniske/leptos_query/main/logo.svg" alt="Leptos Query" width="150"/>
    </a>
</p>

[<img alt="github" src="https://img.shields.io/badge/github-nicoburniske/leptos_query-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/nicoburniske/leptos_query)
[<img alt="crates.io" src="https://img.shields.io/crates/v/leptos_query.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/leptos_query)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-leptos_query-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/leptos_query)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/nicoburniske/leptos_query/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/nicoburniske/leptos_query/actions?query=branch%3Amain)


[FAQ](https://github.com/nicoburniske/leptos_query/blob/main/FAQ.md) | [Examples](https://github.com/nicoburniske/leptos_query/tree/main/example/) | [Live Demo](https://leptos-query-demo.vercel.app/)

## About

Leptos Query is a robust asynchronous state management library for [Leptos](https://github.com/leptos-rs/leptos), providing simplified data fetching, integrated reactivity, server-side rendering support, and intelligent cache management.

Heavily inspired by [Tanstack Query](https://tanstack.com/query/latest/).

Read the introduction article here: [Forging Leptos Query](https://nicoburniske.com/thoughts/forging_leptos_query)

## Why Choose Leptos Query?

Leptos Query focuses on simplifying your data fetching process and keeping your application's state effortlessly synchronized and up-to-date. Here's how it's done:

### Key Features

- **Configurable Caching & SWR**: Queries are cached by default, ensuring quick access to your data. You can configure your stale and cache times per query with Stale While Revalidate (SWR) system.

- **Reactivity at the Core**: Leptos Query deeply integrates with Leptos' reactive system to transform asynchronous query fetchers into reactive Signals.

- **Server-Side Rendering (SSR) Compatibility**: Fetch your queries on the server and smoothly serialize them to the client, just as you would with a Leptos Resource.

- **Client-Side Resource Persistance**: Cache your queries to local storage, async-db, or another custom client side cache.

- **Efficient De-duplication**: If you make multiple queries with the same Key, Leptos Query smartly fetches only once.

- **Manual Invalidation**: Control when your queries should be invalidated and refetched for that ultimate flexibility.

- **Scheduled Refetching**: Set up your queries to refetch on a customized schedule, keeping your data fresh as per your needs.

- **Optimistic Updates**: Useful when you have updated a value and you want to manually set it in cache instead of waiting for query to refetch.

- **Query Cancellation**: Cancel your queries when you know their results are no longer needed. 

- **Introspection & Debugging**: Leptos Query provides [devtools](https://crates.io/crates/leptos_query_devtools), so you can make sure everything's working as intended.

## Feature Flags
- `csr` Client-side rendering: Use queries on the client.
- `ssr` Server-side rendering: Initiate queries on the server during SSR.
- `hydrate` Hydration: Ensure that queries are hydrated on the client, when using server-side rendering.
- `local_storage` - Enables local storage persistance for queries.
- `index_db` - Enables index db persistance for queries.

## Version compatibility for Leptos and Leptos Query

The table below shows the compatible versions of `leptos_query` for each `leptos` version. Ensure you are using compatible versions to avoid potential issues.

| `leptos` version | `leptos_query` version |
|------------------|------------------------|
| 0.6.*            | 0.5.* or 0.4.*         |
| 0.5.*            | 0.3.*                  |


## Installation

```bash
cargo add leptos_query
```

Then add the relevant feature(s) to your `Cargo.toml`

```toml

[features]
hydrate = [
    "leptos_query/hydrate",
    # ...
]
ssr = [
    "leptos_query/ssr",
    # ...
 ]

```

## Quick Start

> If you are using SSR you may have to use [`supress_query_load`](https://docs.rs/leptos_query/latest/leptos_query/fn.suppress_query_load.html) in your server's main function. See the [FAQ](https://github.com/nicoburniske/leptos_query/blob/main/FAQ.md#why-am-i-getting-a-panic-on-my-leptos-main-function) for more information.

In the root of your App, provide a query client:

```rust 
 #[component]
 pub fn App() -> impl IntoView {
     // Provides Query Client for entire app.
     provide_query_client();

     // Rest of App...
 }
 ```

 Then create a query with `leptos_query::create_query`

 ```rust
 use leptos::*;
 use leptos_query::*;

  // Query for a track.
 fn track_query() -> QueryScope<TrackId, TrackData> {
     create_query(
         get_track,
         QueryOptions::default(),
     )
 }

 // Make a key type.
 #[derive(Debug, Clone, Hash, Eq, PartialEq)]
 struct TrackId(String);

 // The result of the query fetcher.
 #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
 struct TrackData {
    name: String,
 }

 // Query fetcher.
 async fn get_track(id: TrackId) -> TrackData {
     todo!()
 }

 ```

 Now you can use the query in any component in your app.

 ```rust

 use leptos::*;
 use leptos_query::*;

 #[component]
 fn TrackView(id: TrackId) -> impl IntoView {
     let QueryResult {
         data,
         ..
     } = track_query().use_query(move || id.clone());

     view! {
        <div>
            // Query data should be read inside a Transition/Suspense component.
            <Transition
                fallback=move || {
                    view! { <h2>"Loading..."</h2> }
                }>
                {move || {
                     data
                         .get()
                         .map(|track| {
                            view! { <h2>{track.name}</h2> }
                         })
                }}
            </Transition>
        </div>
     }
 }
```

For a complete working example see [the example directory](/example)

## Devtools Quickstart

To use the devtools, you need to add the devtools crate:

```bash
cargo add leptos_query_devtools
```

Then in your `cargo.toml` enable the `csr` feature.

#### Hydrate Example
- If your app is using SSR, then this should go under the "hydrate" feature. 
```toml
[features]
hydrate = [
    "leptos_query_devtools/csr",
]
```

#### CSR Example
- If your app is using CSR, then this should go under the "csr" feature.
```toml
[features]
csr = [
    "leptos_query_devtools/csr",
]
```

Then in your app, render the devtools component. Make sure you also provide the query client. 

Devtools will by default only show in development mode. It will not be shown, or included in binary, when you build your app in release mode. If you want to override this behaviour, you can enable the `force` feature.

```rust

use leptos_query_devtools::LeptosQueryDevtools;
use leptos::*;

#[component]
fn App() -> impl IntoView {
    provide_query_client();

    view!{
        <LeptosQueryDevtools />
        // Rest of App...
    }
}

```