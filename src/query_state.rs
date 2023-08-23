use crate::Instant;
use std::fmt::Debug;

/// The lifecycle of a query.
///
/// Each variant in the enum corresponds to a particular state of a query in its lifecycle,
/// starting from creation and covering all possible transitions up to invalidation.
#[derive(Clone, PartialEq, Eq)]
pub enum QueryState<V, E> {
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

    /// Error has occured during fetching.
    Error(QueryError<V, E>),

    /// Retrying after error.
    Retrying(QueryError<V, E>),

    /// Query has errored the maximum number of times.
    Fatal(QueryError<V, E>),
}

impl<V, E> QueryState<V, E> {
    /// Returns the QueryData for the current QueryState, if present.
    pub fn query_data(&self) -> Option<&QueryData<V>> {
        match self {
            QueryState::Loading | QueryState::Created => None,
            QueryState::Fetching(data) | QueryState::Loaded(data) | QueryState::Invalid(data) => {
                Some(data)
            }
            QueryState::Error(QueryError { prev_data, .. })
            | QueryState::Retrying(QueryError { prev_data, .. })
            | QueryState::Fatal(QueryError { prev_data, .. }) => prev_data.as_ref(),
        }
    }

    pub fn result(self) -> Option<Result<V, E>> {
        match self {
            QueryState::Fatal(QueryError { error, .. }) => Some(Err(error)),
            QueryState::Loading | QueryState::Created => None,

            QueryState::Fetching(data) | QueryState::Loaded(data) | QueryState::Invalid(data) => {
                Some(Ok(data.data))
            }
            QueryState::Error(QueryError { prev_data, .. })
            | QueryState::Retrying(QueryError { prev_data, .. }) => {
                prev_data.map(|d| d.data).map(Ok)
            }
        }
    }

    /// Returns the data contained within the QueryState, if present.
    pub fn data(&self) -> Option<&V> {
        self.query_data().map(|s| &s.data)
    }

    /// Returns the last updated timestamp for the QueryState, if present.
    pub fn updated_at(&self) -> Option<Instant> {
        self.query_data().map(|s| s.updated_at)
    }
}

impl<V, E> Debug for QueryState<V, E>
where
    V: Debug,
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Loading => write!(f, "Loading"),
            Self::Fetching(arg0) => f.debug_tuple("Fetching").field(arg0).finish(),
            Self::Loaded(arg0) => f.debug_tuple("Loaded").field(arg0).finish(),
            Self::Invalid(arg0) => f.debug_tuple("Invalid").field(arg0).finish(),
            QueryState::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
            QueryState::Retrying(arg0) => f.debug_tuple("Retrying").field(arg0).finish(),
            QueryState::Fatal(arg0) => f.debug_tuple("Panic").field(arg0).finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct QueryError<V, E> {
    pub(crate) error: E,
    pub(crate) error_count: usize,
    pub(crate) updated_at: Instant,
    pub(crate) prev_data: Option<QueryData<V>>,
}

impl<V, E> Debug for QueryError<V, E>
where
    V: Debug,
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryError")
            .field("error", &self.error)
            .field("error_count", &self.error_count)
            .field("updated_at", &self.updated_at)
            .field("prev_data", &self.prev_data)
            .finish()
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

impl<V> QueryData<V> {
    /// Creates a new QueryData with the given data and the current time as the updated_at timestamp.
    pub fn now(data: V) -> Self {
        Self {
            data,
            updated_at: Instant::now(),
        }
    }
}

impl<V> Debug for QueryData<V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryData")
            .field("data", &self.data)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}
