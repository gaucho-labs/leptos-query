# Leptos Query

Leptos Query is a asynchronous state management library for [Leptos](https://github.com/leptos-rs/leptos).

Heavily inspired by [Tanstack Query](https://tanstack.com/query/latest/).

Queries are useful for data fetching, caching, and synchronization with server state.

A Query provides:

- Caching
- De-duplication
- Invalidation
- Background refetching
- Refetch intervals
- Memory management with cache lifetimes

## Installation

```bash
cargo add leptos_query
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

**NOTE**:

- A query is unique per Key `K`.
- A query Key type `K` must only correspond to ONE UNIQUE Value `V` Type.
  - Meaning a query Key type `K` cannot correspond to multiple `V` Types.

TLDR: Wrap your key in a [Newtype](https://doc.rust-lang.org/rust-by-example/generics/new_types.html) when needed to ensure uniqueness.

```rust

 // Create a Newtype for MonkeyId.
 #[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
 struct MonkeyId(String);


 // Monkey fetcher.
 async fn get_monkey(id: MonkeyId) -> Monkey {
    todo!()
 }

 // Query for a Monkey.
 fn use_monkey_query(cx: Scope, id: impl Fn() -> MonkeyId + 'static) -> QueryResult<Monkey> {
     leptos_query::use_query(
         cx,
         id,
         get_monkey,
         QueryOptions {
             default_value: None,
             refetch_interval: None,
             resource_option: ResourceOption::NonBlocking,
             // Considered stale after 5 seconds.
             stale_time: Some(Duration::from_secs(5)),
             // Infinite cache time.
             cache_time: None,
         },
     )
 }

```

Now you can use the query in any component in your app.

```rust

#[component]
fn MonkeyView(cx: Scope, id: MonkeyId) -> impl IntoView {
    let query = use_monkey_query(cx, move || id.clone());
    let QueryResult {
        data,
        is_loading,
        is_fetching,
        is_stale
        ..
    } = query;

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
                   data()
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

### How's this different from a leptos Resource?

A Query uses a resource under the hood, but provides additional functionality like caching, de-duplication, and invalidation.

Resources are individually bound to the `Scope` they are created in.

Queries are all bound to the `QueryClient` they are created in.

Meaning, once you have a `QueryClient` in your app, you can access the value for a query anywhere in your app.

With a resource, you have to manually lift it to a higher scope if you want to preserve it. And this can be cumbersome if you have a many resources.

### What is the difference between `stale_time` and `cache_time`?

`staleTime` is the duration until a query transitions from fresh to stale. As long as the query is fresh, data will always be read from the cache only.

When a query is stale, it will be refetched on its next usage.

`cacheTime` is the duration until inactive queries will be removed from cache.

- Default value for `stale_time` is 0 seconds.
- Default value for `cache_time` is 5 minutes.

These can be configured per-query using `QueryOptions`

If you want infinite cache/stale time, you can set `stale_time` and `cache_time` to `None`.
