use std::{cell::Cell, rc::Rc, time::Duration};

use leptos::{leptos_dom::helpers::TimeoutHandle, *};

use crate::instant::{get_instant, Instant};

pub(crate) fn use_timeout(cx: Scope, func: impl Fn() -> Option<TimeoutHandle> + 'static) {
    // Saves last interval to be cleared on cleanup.
    let interval: Rc<Cell<Option<TimeoutHandle>>> = Rc::new(Cell::new(None));
    let clean_up = {
        let interval = interval.clone();
        move || {
            if let Some(handle) = interval.take() {
                handle.clear();
            }
        }
    };
    on_cleanup(cx, clean_up);

    create_effect(cx, move |maybe_handle: Option<Option<TimeoutHandle>>| {
        let maybe_handle = maybe_handle.flatten();
        if let Some(handle) = maybe_handle {
            handle.clear();
        };
        let result = func();
        interval.set(result);
        result
    })
}

pub(crate) fn time_until_stale(updated_at: Instant, stale_time: Duration) -> Duration {
    let updated_at = updated_at.0.as_millis() as i64;
    let now = get_instant().0.as_millis() as i64;
    let stale_time = stale_time.as_millis() as i64;
    let result = (updated_at + stale_time) - now;
    let ensure_non_negative = result.max(0);
    Duration::from_millis(ensure_non_negative as u64)
}
