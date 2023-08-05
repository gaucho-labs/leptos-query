# Leptos Query

[![Crates.io](https://img.shields.io/crates/v/leptos_query.svg)](https://crates.io/crates/leptos_query)
[![docs.rs](https://docs.rs/leptos_query/badge.svg)](https://docs.rs/leptos_query)

<p align="center">
    <a href="https://docs.rs/leptos_query">
        <img src="https://raw.githubusercontent.com/nicoburniske/leptos_query/main/logo.svg" alt="Leptos Query" width="150"/>
    </a>
</p>

## About

Leptos Query is a robust asynchronous state management library for [Leptos](https://github.com/leptos-rs/leptos), providing simplified data fetching, integrated reactivity, server-side rendering support, and intelligent cache management.

Heavily inspired by [Tanstack Query](https://tanstack.com/query/latest/).

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

In the root of your App, provide a query client:

```rust
use leptos_query::*;
use leptos::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides Query Client for entire app.
    provide_query_client(cx);

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
 fn use_monkey_query(cx: Scope, id: impl Fn() -> String + 'static) -> QueryResult<Monkey, impl RefetchFn> {
     leptos_query::use_query(
         cx,
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
fn MonkeyView(cx: Scope, id: String) -> impl IntoView {
    let QueryResult {
        data,
        is_loading,
        is_fetching,
        is_stale
        ..
    } = use_monkey_query(cx, move || id.clone());

    view! { cx,
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
                   view! { cx, <h2>"Loading..."</h2> }
               }>
               {move || {
                   data.get()
                       .map(|monkey| {
                           view! { cx, <h2>{monkey.name}</h2> }
                       })
               }}
           </Transition>
       </div>
    }
}

```

For a complete working example see [the example directory](/example)

## FAQ

### <ins>How's this different from a leptos Resource?</ins>

A Query uses a resource under the hood, but provides additional functionality like caching, de-duplication, and invalidation.

Resources are individually bound to the `Scope` they are created in. Queries are all bound to the `QueryClient` they are created in. Meaning, once you have a `QueryClient` in your app, you can access the value for a query anywhere in your app.

With a resource, you have to manually lift it to a higher scope if you want to preserve it. And this can be cumbersome if you have a many resources.

Also, queries are stateful on a per-key basis, meaning you can use the same query with for the same key in multiple places and only one request will be made, and they all share the same state.

### <ins>What's the difference between `stale_time` and `cache_time`? </ins>

`staleTime` is the duration until a query transitions from fresh to stale. As long as the query is fresh, data will always be read from the cache only.

When a query is stale, it will be refetched on its next usage.

`cacheTime` is the duration until inactive queries will be removed from cache.

- Default value for `stale_time` is 0 seconds.
- Default value for `cache_time` is 5 minutes.

These can be configured per-query using `QueryOptions`

If you want infinite cache/stale time, you can set `stale_time` and `cache_time` to `None`.

> NOTE: `stale_time` can never be greater than `cache_time`. If `stale_time` is greater than `cache_time`, `stale_time` will be set to `cache_time`.

### <ins> What's a QueryClient? </ins>

A `QueryClient` allows you to interact with the query cache. You can invalidate queries, prefetch them, and introspect the query cache.

`use_query_client()` will return the `QueryClient` for the current scope.

### <ins> What's invalidating a query do? </ins>

Sometimes you can't wait for a query to become stale before you refetch it. QueryClient has an `invalidate_query` method that lets you intelligently mark queries as stale and potentially refetch them too!

When a query is invalidated, the following happens:

- It is marked as `invalid`. This `invalid` state overrides any `stale_time` configuration.
- The next time the query is used, it will be refetched in the background.
  - If a query is currently being used, it will be refetched immediately.

### <ins>What's the difference between `is_loading` and `is_fetching`? </ins>

`is_fetching` is true when the query is in the process of fetching data.

`is_loading` is true when the query is in the process of fetching data FOR THE FIRST TIME.
