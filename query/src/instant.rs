use std::{
    ops::{Add, Sub},
    time::Duration,
};

/// Instant that can be used in both wasm and non-wasm environments.
/// Contains Duration since Unix Epoch (Unix Timestamp).
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(pub std::time::Duration);

impl Instant {
    /// Get the current time as a Unix Timestamp.
    pub fn now() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(any(feature = "hydrate", feature = "csr"))] {
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

impl std::fmt::Display for Instant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_millis())
    }
}

impl std::fmt::Debug for Instant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Instant").field(&self.0.as_millis()).finish()
    }
}
