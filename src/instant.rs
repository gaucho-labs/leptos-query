use std::{
    ops::{Add, Sub},
    time::Duration,
};

/// Instant that can be used in both wasm and non-wasm environments.
/// Contains Duration since Unix Epoch (Unix Timestamp).
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Instant(pub std::time::Duration);

impl Instant {
    /// Get the current time as a Unix Timestamp.
    pub fn now() -> Self {
        get_instant()
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Instant) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<Instant> for Instant {
    type Output = Duration;
    #[inline]
    fn add(self, rhs: Instant) -> Self::Output {
        self.0 + rhs.0
    }
}

pub(crate) fn get_instant() -> Instant {
    use cfg_if::cfg_if;
    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            let millis = js_sys::Date::now();
            let duration = std::time::Duration::from_millis(millis as u64);
            Instant(duration)
        } else {
            let duration = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .expect("System clock was before 1970.");
            Instant(duration)
        }
    }
}
