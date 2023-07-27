use crate::Instant;

#[derive(Clone, PartialEq, Eq)]
pub enum QueryState<V> {
    Loading,
    Fetching(QueryData<V>),
    Loaded(QueryData<V>),
    Stale(QueryData<V>),
    Invalid(QueryData<V>),
}

impl<V> QueryState<V> {
    pub fn data(&self) -> Option<&V> {
        match self {
            QueryState::Loading => None,
            QueryState::Fetching(data)
            | QueryState::Loaded(data)
            | QueryState::Stale(data)
            | QueryState::Invalid(data) => Some(&data.data),
        }
    }

    pub fn updated_at(&self) -> Option<Instant> {
        match self {
            QueryState::Loading => None,
            QueryState::Fetching(data)
            | QueryState::Loaded(data)
            | QueryState::Stale(data)
            | QueryState::Invalid(data) => Some(data.updated_at),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct QueryData<V> {
    pub data: V,
    pub updated_at: Instant,
}

impl<V> QueryState<V> {
    pub fn is_loading(&self) -> bool {
        match self {
            Self::Loading => true,
            _ => false,
        }
    }
}
