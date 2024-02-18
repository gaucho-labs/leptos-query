use std::cell::Cell;

/// Disable or enable query loading.
///
/// Useful for disabling query loads during App introspection, such as SSR Router integrations for Actix/Axum.
///
/// Example for `generate_route_list`
/// ```
/// use leptos::*;
/// use leptos_query::*;
/// use leptos_axum::*;
///
/// fn make_routes()  {
///     // Disable query loading.
///     leptos_query::suppress_query_load(true);
///     // Introspect App Routes.
///     leptos_axum::generate_route_list(App);
///     // Enable query loading.
///     leptos_query::suppress_query_load(false);
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     ()
/// }
///
///
///
/// ```
pub fn suppress_query_load(suppress: bool) {
    SUPPRESS_QUERY_LOAD.with(|w| w.set(suppress));
}

pub(crate) fn with_query_supressed<T>(func: impl FnOnce(bool) -> T) -> T {
    SUPPRESS_QUERY_LOAD.with(|w| func(w.get()))
}

thread_local! {
    static SUPPRESS_QUERY_LOAD: Cell<bool> = Cell::new(false);
}
