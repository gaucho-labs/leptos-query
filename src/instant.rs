use std::{
    ops::{Add, Sub},
    time::Duration,
};

#[derive(Copy, Clone, Debug, Hash)]
pub struct Instant(std::time::Duration);

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

pub fn get_instant() -> Instant {
    use cfg_if::cfg_if;
    cfg_if! { if #[cfg(feature = "hydrate")] {
        let millis = js_sys::Date::now();
        let duration = std::time::Duration::from_millis(millis as u64);
        Instant(duration)
    }}
    cfg_if! { if #[cfg(not(feature = "hydrate"))] {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("System clock was before 1970.");
        Instant(duration)
    }}
}
