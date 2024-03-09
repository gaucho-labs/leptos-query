<!-- cargo-rdme start -->


# Leptos Query Devtools

[<img alt="github" src="https://img.shields.io/badge/github-gaucho--labs/leptos--query-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/gaucho-labs/leptos-query)
[<img alt="crates.io" src="https://img.shields.io/crates/v/leptos_query_devtools.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/leptos_query_devtools)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-leptos_query_devtools-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/leptos_query_devtools)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/gaucho-labs/leptos-query/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/gaucho-labs/leptos-query/actions?query=branch%3Amain)

This crate provides a devtools component for [leptos_query](https://crates.io/crates/leptos_query).
The devtools help visualize all of the inner workings of Leptos Query and will likely save you hours of debugging if you find yourself in a pinch!

## Features
- `csr` Client side rendering: Needed to use browser apis, if this is not enabled your app (under a feature), you will not be able to use the devtools.
- `force`: Always show the devtools, even in release mode.

Then in your app, render the devtools component. Make sure you also provide the query client.

Devtools will by default only show in development mode. It will not be shown, or included in binary when you build your app in release mode. If you want to override this behaviour, you can enable the `force` feature.

## Quickstart

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

```rust

use leptos_query_devtools::LeptosQueryDevtools;
use leptos_query::provide_query_client;
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

Now you should be able to see the devtools mounted to the bottom right of your app!

<!-- cargo-rdme end -->
