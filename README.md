<!-- cargo-rdme start -->

# Leptos Query

<p align="center">
   <a href="https://github.com/gaucho-labs/leptos-query">
      <img src="https://raw.githubusercontent.com/gaucho-labs/leptos-query/main/logo.svg" alt="Leptos Query" width="150"/>
   </a>
</p>

[<img alt="github" src="https://img.shields.io/badge/github-gaucho--labs/leptos--query-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/gaucho-labs/leptos-query)
[<img alt="crates.io" src="https://img.shields.io/crates/v/leptos_query.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/leptos_query)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-leptos_query-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/leptos_query)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/gaucho-labs/leptos_query/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/gaucho-labs/leptos_query/actions?query=branch%3Amain)


[FAQ](https://github.com/gaucho-labs/leptos-query/blob/main/FAQ.md) | [Examples](https://github.com/gaucho-labs/leptos-query/tree/main/example/) | [Live Demo](https://leptos-query-demo.vercel.app/)

## About

Leptos Query is a async state management library for [Leptos](https://github.com/leptos-rs/leptos).

Heavily inspired by [Tanstack Query](https://tanstack.com/query/latest/).

Queries are useful for data fetching, caching, and synchronization with server state.

A Query provides:
- Caching
- Request de-duplication
- Invalidation
- Background refetching
- Refetch intervals
- Memory management with cache lifetimes
- Cancellation
- Debugging tools
- Optimistic updates
- Client side cache persistance (localstorage, indexdb, custom, etc.)


## The main entry points to using Queries are:
- [`create_query`](create_query::create_query) - **Recommended**: Creates a [`QueryScope`] which encapsulates `use_query` and other methods for managing queries.
- [`use_query`][use_query::use_query] - A query primitive for reading, caching, and refetching data.

## Feature Flags
- `csr` Client-side rendering: Use queries on the client.
- `ssr` Server-side rendering: Initiate queries on the server.
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

> If you are using SSR you may have to use [`supress_query_load`](https://docs.rs/leptos_query/latest/leptos_query/fn.suppress_query_load.html) in your server's main function. See the [FAQ](https://github.com/gaucho-labs/leptos_query/blob/main/FAQ.md#why-am-i-getting-a-panic-on-my-leptos-main-function) for more information.

In the root of your App, provide a query client with [provide_query_client] or [provide_query_client_with_options] if you want to override the default options.

```rust
use leptos_query::*;
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides Query Client for entire app.
    provide_query_client();

    // Rest of App...
}
```

Then make a query function with [`create_query`][crate::create_query::create_query()]

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
struct TrackId(i32);

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
    } = track_query().use_query(move|| id.clone());

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
use leptos_query::*;
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

<!-- cargo-rdme end -->
