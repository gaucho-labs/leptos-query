# Leptos Query

<p align="center">
    <a href="https://github.com/nicoburniske/leptos_query">
        <img src="https://raw.githubusercontent.com/nicoburniske/leptos_query/main/logo.svg" alt="Leptos Query" width="150"/>
    </a>
</p>
<p align="center">
    <a href="https://crates.io/crates/leptos_query">
        <img src="https://img.shields.io/crates/v/leptos_query.svg" alt="Crates.io"/>
    </a>
    <a href="https://docs.rs/leptos_query/latest/leptos_query/">
        <img src="https://docs.rs/leptos_query/badge.svg" alt="Crates.io"/>
    </a>
</p>

[FAQ](https://github.com/nicoburniske/leptos_query/blob/main/FAQ.md) | [Docs.rs](https://docs.rs/leptos_query/latest/leptos_query/) | [Examples](https://github.com/nicoburniske/leptos_query/tree/main/example/)

## About

Leptos Query is a robust asynchronous state management library for [Leptos](https://github.com/leptos-rs/leptos), providing simplified data fetching, integrated reactivity, server-side rendering support, and intelligent cache management.

Heavily inspired by [Tanstack Query](https://tanstack.com/query/latest/).

Read the introduction article here: [The Forging of Leptos Query](https://nicoburniske.com/thoughts/forging_leptos_query)

## Why Choose Leptos Query?

Leptos Query focuses on simplifying your data fetching process and keeping your application's state effortlessly synchronized and up-to-date. Here's how it's done:

### Key Features

- **Configurable Caching & SWR**: Queries are cached by default, ensuring quick access to your data. You can configure your stale and cache times per query with Stale While Revalidate (SWR) system.

- **Reactivity at the Core**: Leptos Query deeply integrates with Leptos' reactive system to transform asynchronous query fetchers into reactive Signals.

- **Server-Side Rendering (SSR) Compatibility**: Fetch your queries on the server and smoothly serialize them to the client, just as you would with a Leptos Resource.

- **Efficient De-duplication**: No unnecessary fetches here! If you make multiple queries with the same Key, Leptos Query smartly fetches only once.

- **Manual Invalidation**: Control when your queries should be invalidated and refetched for that ultimate flexibility.

- **Scheduled Refetching**: Set up your queries to refetch on a customized schedule, keeping your data fresh as per your needs.

- **Manual Query Data Mutations**: Useful when you have updated a value and you want to manually set it in cache instead of waiting for query to refetch.


## Version compatibility for Leptos and Leptos Query

The table below shows the compatible versions of `leptos_query` for each `leptos` version. Ensure you are using compatible versions to avoid potential issues.

| `leptos` version | `leptos_query` version |
|------------------|------------------------|
| 0.6.*            | 0.4.*                  |
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

> If you are using SSR you may have to use `supress_query_load` in your server's main function. See the [FAQ](https://github.com/nicoburniske/leptos_query/blob/main/FAQ.md#why-am-i-getting-a-panic-on-my-leptos-main-function) for more information.

In the root of your App, provide a query client:

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

Then make a query function.

```rust

 use leptos::*;
 use leptos_query::*;
 use std::time::Duration;
 use serde::*;

 // Data type.
 #[derive(Clone, Deserialize, Serialize)]
 struct Monkey {
     name: String,
 }


 // Monkey fetcher.
 async fn get_monkey(id: String) -> Monkey {
    todo!()
 }

 // Query for a Monkey.
 fn use_monkey_query(id: impl Fn() -> String + 'static) -> QueryResult<Monkey, impl RefetchFn> {
     leptos_query::use_query(
         id,
         get_monkey,
         QueryOptions {
             default_value: None,
             refetch_interval: None,
             resource_option: ResourceOption::NonBlocking,
             // Considered stale after 10 seconds.
             stale_time: Some(Duration::from_secs(10)),
             // Infinite cache time.
             cache_time: None,
         },
     )
 }

```

Now you can use the query in any component in your app.

```rust

#[component]
fn MonkeyView(id: String) -> impl IntoView {
    let QueryResult {
        data,
        is_loading,
        is_fetching,
        is_stale
        ..
    } = use_monkey_query(move || id.clone());

    view! {
      // You can use the query result data here.
      // Everything is reactive.
       <div>
           <div>
               <span>"Loading Status: "</span>
               <span>{move || { if is_loading.get() { "Loading..." } else { "Loaded" } }}</span>
           </div>
           <div>
               <span>"Fetching Status: "</span>
               <span>
                   {move || { if is_fetching.get() { "Fetching..." } else { "Idle" } }}
               </span>
           </div>
           <div>
               <span>"Stale Status: "</span>
               <span>
                   {move || { if is_stale.get() { "Stale" } else { "Fresh" } }}
               </span>
           </div>
           // Query data should be read inside a Transition/Suspense component.
           <Transition
               fallback=move || {
                   view! { <h2>"Loading..."</h2> }
               }>
               {move || {
                   data.get()
                       .map(|monkey| {
                           view! { <h2>{monkey.name}</h2> }
                       })
               }}
           </Transition>
       </div>
    }
}

```

For a complete working example see [the example directory](/example)
