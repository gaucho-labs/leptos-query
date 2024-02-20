# FAQ

- [How's this different from a Leptos Resource?](#hows-this-different-from-a-leptos-resource)
- [What's the difference between `stale_time` and `gc_time`?](#whats-the-difference-between-stale_time-and-gc_time)
- [What's a QueryClient?](#the-query-client)
- [What's Query Invalidation?](#query-invalidation)
- [What's the difference between `is_loading` and `is_fetching`?](#whats-the-difference-between-is_loading-and-is_fetching)
- [Why am I getting a Leptos Reactive Panic?](#why-am-i-getting-a-panic-on-my-leptos-main-function)

## How's this different from a Leptos Resource?

Leptos Query extends the functionality of [Leptos Resources](https://leptos-rs.github.io/leptos/async/10_resources.html) with features like caching, de-duplication, and invalidation, while also allowing easy access to cached data throughout your app.

Queries are all bound to the `QueryClient` they are created in, meaning that once you have a `QueryClient` in your app, you can access the value for a query anywhere in your app, and you have a single cache for your entire app. Queries are stateful on a per-key basis, meaning you can use the same query with the same key in multiple places and only one request will be made, and they all share the same state.

With a resource, you have to manually lift it to a higher scope if you want to preserve it, which can be cumbersome if you have many resources.

## What's the difference between `stale_time` and `gc_time`?

`stale_time` is the duration until a query transitions from fresh to stale. As long as the query is fresh, data will always be read from the cache only. When a query is stale, it will be refetched in the background on its next usage (specifically on the next use_query mount). While the refetch is executing to retrieve the latest value, the stale value will be used.

The active `stale_time` is the minimum of all active query `stale_time` values.

`gc_time` is the duration until inactive queries will be evicted from the cache.

The active `gc_time` is the maximum of all query `gc_time` values.

Default values:

- `stale_time`: 10 seconds.
- `gc_time`: 5 minutes.

These can be configured per-query using `QueryOptions`, or for the entire App using `DefaultQueryOptions`. If you want infinite cache/stale time, set `stale_time` and `gc_time` to `None`.

> NOTE: `stale_time` can never be greater than `gc_time`
> If `stale_time` is greater than `gc_time`, `stale_time` will be set to `gc_time`.

Consider a query that fetches a Stock Price, and that price is updated every minute. We should set our `stale_time` to be a minute. This would ensure that we would always have the latest stock price.

## The Query Client

A `QueryClient` contains the query cache and exposes methods to interact with it. `use_query_client()` will return the `QueryClient` for the current scope.

Some useful methods on `QueryClient` include:

- [Prefetching](https://docs.rs/leptos_query/latest/leptos_query/struct.QueryClient.html#method.prefetch_query): Query will start loading before you invoke [use_query](use_query::use_query), which is useful when you anticipate a query will be used soon.
- [Invalidation](https://docs.rs/leptos_query/latest/leptos_query/struct.QueryClient.html#method.invalidate_query): Query will refetch on next usage. Active queries are immediately refetched in the background, which is helpful for highly dynamic data.
- [Introspection](https://docs.rs/leptos_query/latest/leptos_query/struct.QueryClient.html#method.get_query_state): Lets you see what the current value of a query is.
- [Manual updates](https://docs.rs/leptos_query/latest/leptos_query/struct.QueryClient.html#method.set_query_data): Useful when you have updated a value and you want to manually set it in cache instead of waiting for the query to refetch.

## Query Invalidation

Sometimes you can't wait for a query to become stale before you refetch it. `QueryClient` has an `invalidate_query` method that lets you intelligently mark queries as stale and potentially refetch them too!

When a query is invalidated, the following happens:

- It is marked as `invalid`, which overrides any `stale_time` configuration.
- The next time the query is used, it will be refetched in the background.
- If a query is currently being used, it will be refetched immediately.

This can be particularly useful in cases where you have a highly dynamic data source, or when user actions in the application can directly modify data that other parts of your application rely on.

## What's the difference between `is_loading` and `is_fetching`?

`is_fetching` is true when the query is in the process of fetching data. `is_loading` is true when the query is in the process of fetching data for the first time.

Consider a scenario where you're fetching a list of items. `is_loading` would be true when you're initially loading this list, whereas `is_fetching` would be true for both the initial load and any subsequent data refreshes (e.g. due to user actions or background updates).

## Why am I getting a panic on my Leptos main function?

If you are getting a Rust Panic with the following cause:

```
tried to run untracked function in a runtime that has been disposed: ()

```

This is likely because queries are executing during some server side App introspection, such as SSR Router integrations for Actix/Axum.

Before you execute the introspection use the `supress_query_load` function to prevent queries from loading and causing the panic.

Then make sure you re-enable query loading for your app to behave properly.

Here's an example for the Axum integration:

```rust
// Disable query loading.
leptos_query::suppress_query_load(true);
// Introspect App Routes.
leptos_axum::generate_route_list(|| view! { <App/> }).await;
// Enable query loading.
leptos_query::suppress_query_load(false);
```
