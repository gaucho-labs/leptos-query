#![warn(missing_docs)]

//! # Leptos Query Devtools
//!
//! This crate provides a devtools component for [leptos_query](https://crates.io/crates/leptos_query).
//! The devtools help visualize all of the inner workings of Leptos Query and will likely save you hours of debugging if you find yourself in a pinch!
//!
//! ## Features
//! - `csr` Client side rendering: Needed to use browser apis, if this is not enabled your app (under a feature), you will not be able to use the devtools.
//! - `force`: Always show the devtools, even in release mode.
//!
//! Then in your app, render the devtools component. Make sure you also provide the query client.
//!
//! Devtools will by default only show in development mode. It will not be shown, or included in binary when you build your app in release mode. If you want to override this behaviour, you can enable the `force` feature.
//!
//! ```
//!
//! use leptos_query_devtools::LeptosQueryDevtools;
//! use leptos_query::provide_query_client;
//! use leptos::*;
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     provide_query_client();
//!
//!     view!{
//!         <LeptosQueryDevtools />
//!         // Rest of App...
//!     }
//! }
//! ```

use leptos::*;

#[component]
pub fn LeptosQueryDevtools() -> impl IntoView {
    #[cfg(any(debug_assertions, feature = "force"))]
    {
        use dev_tools::InnerDevtools;
        view! { <InnerDevtools/> }
    }
}

#[cfg(any(debug_assertions, feature = "force"))]
mod dev_tools;

#[cfg(any(debug_assertions, feature = "force"))]
mod timeout;

#[cfg(any(debug_assertions, feature = "force"))]
mod component;
