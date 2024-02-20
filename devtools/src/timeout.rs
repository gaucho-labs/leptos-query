use std::{cell::Cell, rc::Rc, time::Duration};

use leptos::{create_effect, leptos_dom::helpers::TimeoutHandle, on_cleanup};
use leptos_query::Instant;

pub(crate) fn use_timeout(func: impl Fn() -> Option<TimeoutHandle> + 'static) {
    // Saves last interval to be cleared on cleanup.
    let timeout: Rc<Cell<Option<TimeoutHandle>>> = Rc::new(Cell::new(None));
    let clean_up = {
        let interval = timeout.clone();
        move || {
            if let Some(handle) = interval.take() {
                handle.clear();
            }
        }
    };

    on_cleanup(clean_up);

    create_effect(move |_| {
        if let Some(handle) = timeout.take() {
            handle.clear();
        }

        let result = func();
        timeout.set(result);

        result
    });
}

pub(crate) fn time_until_stale(updated_at: Instant, stale_time: Duration) -> Duration {
    let updated_at = updated_at.0.as_millis() as i64;
    let now = Instant::now().0.as_millis() as i64;
    let stale_time = stale_time.as_millis() as i64;
    let result = (updated_at + stale_time) - now;
    let ensure_non_negative = result.max(0);
    Duration::from_millis(ensure_non_negative as u64)
}
