use std::{marker::PhantomData, time::Duration};

pub trait Schedule {
    type Error;

    fn next(&mut self, error: &Self::Error) -> Option<Duration>;

    fn union<U>(self, other: U) -> Union<Self, U>
    where
        Self: Sized,
        U: Schedule<Error = Self::Error>,
    {
        Union { a: self, b: other }
    }

    fn intersect<U>(self, other: U) -> Intersect<Self, U>
    where
        Self: Sized,
        U: Schedule<Error = Self::Error>,
    {
        Intersect { a: self, b: other }
    }

    fn concat<U>(self, other: U) -> Sequence<Self, U>
    where
        Self: Sized,
        U: Schedule<Error = Self::Error>,
    {
        Sequence { a: self, b: other }
    }

    fn map<F>(self, func: F) -> Mapped<Self, F>
    where
        Self: Sized,
        F: Fn((&Self::Error, Option<Duration>)) -> Option<Duration>,
    {
        Mapped {
            schedule: self,
            func,
        }
    }

    fn take(self, n: u32) -> Take<Self>
    where
        Self: Sized,
    {
        Take { n, schedule: self }
    }

    fn take_while<F: 'static>(self, func: F) -> TakeWhile<Self, F>
    where
        Self: Sized,
        F: Fn((&Self::Error, Duration)) -> bool,
    {
        TakeWhile {
            schedule: self,
            func,
        }
    }

    fn clamp(self, min: Duration, max: Duration) -> Clamp<Self>
    where
        Self: Sized,
    {
        Clamp {
            schedule: self,
            max: Some(max),
            min: Some(min),
        }
    }

    fn clamp_max(self, duration: Duration) -> Clamp<Self>
    where
        Self: Sized,
    {
        Clamp {
            schedule: self,
            max: Some(duration),
            min: None,
        }
    }

    fn clamp_min(self, duration: Duration) -> Clamp<Self>
    where
        Self: Sized,
    {
        Clamp {
            schedule: self,
            max: None,
            min: Some(duration),
        }
    }

    fn build(self) -> ScheduleBuilt<Self::Error>
    where
        Self: Sized + 'static,
    {
        ScheduleBuilt(std::rc::Rc::new(self))
    }
}

#[derive(Clone)]
pub struct ScheduleBuilt<E>(std::rc::Rc<dyn Schedule<Error = E>>);

pub struct Recur<E> {
    n: u32,
    error_type: PhantomData<E>,
}

impl<E> Schedule for Recur<E> {
    type Error = E;
    fn next(&mut self, _: &Self::Error) -> Option<Duration> {
        if self.n > 0 {
            self.n -= 1;
            Some(Duration::ZERO)
        } else {
            None
        }
    }
}

pub struct Spaced<E> {
    duration: Duration,
    error_type: PhantomData<E>,
}

impl<E> Schedule for Spaced<E> {
    type Error = E;
    fn next(&mut self, _: &Self::Error) -> Option<Duration> {
        Some(self.duration)
    }
}

pub struct Sequence<A, B> {
    a: A,
    b: B,
}

impl<A, B, E> Schedule for Sequence<A, B>
where
    A: Schedule<Error = E>,
    B: Schedule<Error = E>,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        let a = self.a.next(error);
        if let Some(_) = a {
            a
        } else {
            self.b.next(error)
        }
    }
}

pub struct Union<A, B> {
    a: A,
    b: B,
}

impl<A, B, E> Schedule for Union<A, B>
where
    A: Schedule<Error = E>,
    B: Schedule<Error = E>,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        let a = self.a.next(error);
        let b = self.b.next(error);

        match (a, b) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(_), None) => a,
            (None, Some(_)) => b,
            _ => None,
        }
    }
}

pub struct Intersect<A, B> {
    a: A,
    b: B,
}

impl<A, B, E> Schedule for Intersect<A, B>
where
    A: Schedule<Error = E>,
    B: Schedule<Error = E>,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        let a = self.a.next(error);
        let b = self.b.next(error);

        match (a, b) {
            (Some(a), Some(b)) => Some(a.max(b)),
            _ => None,
        }
    }
}

pub struct Exponential<E> {
    base: Duration,
    n: u32,
    factor: f32,
    error_type: PhantomData<E>,
}

impl<E> Schedule for Exponential<E> {
    type Error = E;
    fn next(&mut self, _: &Self::Error) -> Option<Duration> {
        let n = self.n;
        self.n += 1;
        if n == 0 {
            Some(self.base)
        } else {
            let mult = self.factor.powf(n as f32);
            let delay = self.base.mul_f32(mult);
            Some(delay)
        }
    }
}

pub struct Mapped<A, F> {
    schedule: A,
    func: F,
}

impl<A, F, E> Schedule for Mapped<A, F>
where
    A: Schedule<Error = E>,
    F: Fn((&E, Option<Duration>)) -> Option<Duration>,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        let next = self.schedule.next(error);
        (self.func)((error, next))
    }
}

pub struct Take<A> {
    schedule: A,
    n: u32,
}

impl<A, E> Schedule for Take<A>
where
    A: Schedule<Error = E>,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        if self.n > 0 {
            self.n -= 1;
            self.schedule.next(error)
        } else {
            None
        }
    }
}

pub struct TakeWhile<A, F> {
    schedule: A,
    func: F,
}

impl<A, F, E> Schedule for TakeWhile<A, F>
where
    A: Schedule<Error = E>,
    F: Fn((&E, Duration)) -> bool,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        self.schedule
            .next(error)
            .filter(|d| (self.func)((error, *d)))
    }
}

pub struct Clamp<A> {
    schedule: A,
    min: Option<Duration>,
    max: Option<Duration>,
}

impl<A, E> Schedule for Clamp<A>
where
    A: Schedule<Error = E>,
{
    type Error = E;

    fn next(&mut self, error: &Self::Error) -> Option<Duration> {
        let next = self.schedule.next(error);
        next.map(|d| {
            let bottom = if let Some(min) = self.min {
                min.max(d)
            } else {
                d
            };

            if let Some(max) = self.max {
                max.min(bottom)
            } else {
                bottom
            }
        })
    }
}

pub fn recurs<E>(n: u32) -> impl Schedule<Error = E> {
    Recur {
        n,
        error_type: PhantomData,
    }
}

pub fn spaced<E>(d: Duration) -> impl Schedule<Error = E> {
    Spaced {
        duration: d,
        error_type: PhantomData,
    }
}

pub fn exponential<E>(base: Duration, factor: f32) -> impl Schedule<Error = E> {
    Exponential {
        n: 0,
        base,
        factor,
        error_type: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_recurs() {
        let mut r = recurs(2);
        assert_eq!(Some(Duration::ZERO), r.next(&()));
        assert_eq!(Some(Duration::ZERO), r.next(&()));
        assert_eq!(None, r.next(&()));
    }

    #[test]
    fn test_spaced() {
        let d = Duration::from_millis(500);
        let mut schedule = recurs(2).intersect(spaced(d));
        assert_eq!(Some(d), schedule.next(&()));
        assert_eq!(Some(d), schedule.next(&()));
        assert_eq!(None, schedule.next(&()));
    }

    #[test]
    fn test_sequence() {
        let d = Duration::from_millis(500);
        let left = recurs(2).intersect(spaced(d));
        let right = recurs(2);
        let mut schedule = left.concat(right);

        assert_eq!(Some(d), schedule.next(&()));
        assert_eq!(Some(d), schedule.next(&()));
        assert_eq!(Some(Duration::ZERO), schedule.next(&()));
        assert_eq!(Some(Duration::ZERO), schedule.next(&()));
        assert_eq!(None, schedule.next(&()));
    }

    #[test]
    fn test_exponential() {
        let mut schedule = exponential(Duration::from_millis(500), 2.0).take(6);

        assert_eq!(Some(Duration::from_millis(500)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(1000)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(2000)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(4000)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(8000)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(16000)), schedule.next(&()));
        assert_eq!(None, schedule.next(&()));
    }

    #[test]
    fn test_exponential_while() {
        let mut schedule = exponential(Duration::from_millis(500), 2.0)
            .take_while(|(_, d)| d < Duration::from_millis(2001));

        assert_eq!(Some(Duration::from_millis(500)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(1000)), schedule.next(&()));
        assert_eq!(Some(Duration::from_millis(2000)), schedule.next(&()));
        assert_eq!(None, schedule.next(&()));
    }

    #[test]
    fn test_exponential_clamp() {
        let mut schedule = exponential(Duration::from_millis(100), 2.0)
            .clamp_min(Duration::from_secs(1))
            .clamp_max(Duration::from_secs(4));

        let mut next = move || schedule.next(&()).map(|d| d.as_millis());

        // 100
        assert_eq!(Some(1000), next());
        // 200
        assert_eq!(Some(1000), next());
        // 400
        assert_eq!(Some(1000), next());
        // 800
        assert_eq!(Some(1000), next());
        assert_eq!(Some(1600), next());
        assert_eq!(Some(3200), next());
        assert_eq!(Some(4000), next());
        assert_eq!(Some(4000), next());
    }
}
