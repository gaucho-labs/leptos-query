use std::time::Duration;

use crate::instant::Instant;

pub(crate) fn time_until_stale(updated_at: Instant, stale_time: Duration) -> Duration {
    let updated_at = updated_at.0.as_millis() as i64;
    let now = Instant::now().0.as_millis() as i64;
    let stale_time = stale_time.as_millis() as i64;
    let result = (updated_at + stale_time) - now;
    let ensure_non_negative = result.max(0);
    Duration::from_millis(ensure_non_negative as u64)
}
