use crate::Instant;

/// The lifecycle of a query.
///
/// Each variant in the enum corresponds to a particular state of a query in its lifecycle,
/// starting from creation and covering all possible transitions up to invalidation.
#[derive(Clone, PartialEq, Eq)]
pub enum QueryState<V> {
    /// The initial state of a Query upon its creation.
    ///
    /// In this state, a query is instantiated but no fetching operation has been initiated yet.
    /// This means that no data has been requested or received, and the query is in a "pending" state,
    /// waiting to begin its first fetch operation.
    Created,

    /// Query is fetching for the first time.
    ///
    /// In this state, the query has started its first data fetching process. It is actively communicating
    /// with the data source and waiting for the data to be returned.
    Loading,

    /// A Query is in the process of fetching, not being its first fetch.
    ///
    /// In this state, a query is undergoing another fetch operation following a previous one.
    /// The associated `QueryData<V>` object holds the previous data was fetched.
    Fetching(QueryData<V>),

    /// The state indicating that a query has successfully completed a fetch operation.
    ///
    /// In this state, the query has finished fetching data.
    /// The associated `QueryData<V>` object holds the successfully loaded data.
    Loaded(QueryData<V>),

    /// The state indicating that a query has completed a fetch, but the fetched data is marked as invalid.
    ///
    /// The associated `QueryData<V>` object holds the invalidated data.
    Invalid(QueryData<V>),
}

impl<V> QueryState<V> {
    /// Returns the data contained within the QueryState, if present.
    pub fn data(&self) -> Option<&V> {
        match self {
            QueryState::Loading | QueryState::Created => None,
            QueryState::Fetching(data) | QueryState::Loaded(data) | QueryState::Invalid(data) => {
                Some(&data.data)
            }
        }
    }

    /// Returns the last updated timestamp for the QueryState, if present.
    pub fn updated_at(&self) -> Option<Instant> {
        match self {
            QueryState::Loading | QueryState::Created => None,
            QueryState::Fetching(data) | QueryState::Loaded(data) | QueryState::Invalid(data) => {
                Some(data.updated_at)
            }
        }
    }
}

/// The latest data for a Query.
#[derive(Clone, PartialEq, Eq)]
pub struct QueryData<V> {
    /// The Data.
    pub data: V,
    /// The instant this data was retrieved.
    pub updated_at: Instant,
}
