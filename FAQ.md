## FAQ

- [How's this different from a Leptos Resource?](#how's-this-different-from-a-leptos-resource)
- [What's the difference between `stale_time` and `cache_time`?](#what's-the-difference-between-stale_time-and-cache_time)
- [What's a QueryClient?](#what's-a-queryclient)
- [What's invalidating a query do?](#what's-invalidating-a-query-do)
- [What's the difference between `is_loading` and `is_fetching`?](#what's-the-difference-between-is_loading-and-is_fetching)

### <ins>How's this different from a Leptos Resource?</ins>

A Query uses a resource under the hood, but provides additional functionality like caching, de-duplication, and invalidation.

Resources are individually bound to the `Scope` they are created in. Queries are all bound to the `QueryClient` they are created in. Meaning, once you have a `QueryClient` in your app, you can access the value for a query anywhere in your app, and you have a single cache for your entire app.

With a resource, you have to manually lift it to a higher scope if you want to preserve it. And this can be cumbersome if you have a many resources.

Also, queries are stateful on a per-key basis, meaning you can use the same query with for the same key in multiple places and only one request will be made, and they all share the same state.

### What's the difference between `stale_time` and `cache_time`?

`staleTime` is the duration until a query transitions from fresh to stale. As long as the query is fresh, data will always be read from the cache only.

When a query is stale, it will be refetched on its next usage.

`cacheTime` is the duration until inactive queries will be removed from cache.

- Default value for `stale_time` is 0 seconds.
- Default value for `cache_time` is 5 minutes.

These can be configured per-query using `QueryOptions`

If you want infinite cache/stale time, you can set `stale_time` and `cache_time` to `None`.

> NOTE: `stale_time` can never be greater than `cache_time`. If `stale_time` is greater than `cache_time`, `stale_time` will be set to `cache_time`.

### <ins> What's a QueryClient? </ins>

A `QueryClient` contains the query cache. It exposes methods to interact with the query cache. You can invalidate queries, prefetch them, and introspect the query cache.

`use_query_client()` will return the `QueryClient` for the current scope.

### <ins> What's invalidating a query do? </ins>

Sometimes you can't wait for a query to become stale before you refetch it. QueryClient has an `invalidate_query` method that lets you intelligently mark queries as stale and potentially refetch them too!

When a query is invalidated, the following happens:

- It is marked as `invalid`. This `invalid` state overrides any `stale_time` configuration.
- The next time the query is used, it will be refetched in the background.
  - If a query is currently being used, it will be refetched immediately.

### <ins>What's the difference between `is_loading` and `is_fetching`? </ins>

`is_fetching` is true when the query is in the process of fetching data.

`is_loading` is true when the query is in the process of fetching data for the first time.
