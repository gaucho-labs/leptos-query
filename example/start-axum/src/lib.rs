use cfg_if::cfg_if;
pub mod app;
pub mod error_template;
pub mod fileserv;
pub mod todo;

cfg_if! { if #[cfg(feature = "hydrate")] {
    use leptos::*;
    use wasm_bindgen::prelude::wasm_bindgen;
    use crate::app::*;

    #[wasm_bindgen]
    pub fn hydrate() {
        setup_logging();
        leptos::mount_to_body(move |cx| {
            view! { cx, <App/> }
        });
    }

    /// Setup browser console logging using [tracing_subscriber_wasm]
    fn setup_logging() {
        tracing_subscriber::fmt()
          .with_writer(
            // To avoide trace events in the browser from showing their
            // JS backtrace, which is very annoying, in my opinion
            tracing_subscriber_wasm::MakeConsoleWriter::default().map_trace_level_to(tracing::Level::DEBUG),
          )
          // For some reason, if we don't do this in the browser, we get
          // a runtime error.
          .without_time()
          .init();
    }
}}
