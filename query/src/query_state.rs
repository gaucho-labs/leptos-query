use crate::Instant;

/// The lifecycle of a query.
///
/// Each variant in the enum corresponds to a particular state of a query in its lifecycle,
/// starting from creation and covering all possible transitions up to invalidation.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum QueryState<V> {
    /// The initial state of a Query upon its creation.
    ///
    /// In this state, a query is instantiated but no fetching operation has been initiated yet.
    /// This means that no data has been requested or received, and the query is in a "pending" state,
    /// waiting to begin its first fetch operation.
    #[default]
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
    /// Returns the QueryData for the current QueryState, if present.
    pub fn query_data(&self) -> Option<&QueryData<V>> {
        match self {
            QueryState::Loading | QueryState::Created => None,
            QueryState::Fetching(data) | QueryState::Loaded(data) | QueryState::Invalid(data) => {
                Some(data)
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

    pub(crate) fn data_mut(&mut self) -> Option<&mut V> {
        match self {
            QueryState::Loading | QueryState::Created => None,
            QueryState::Fetching(data) | QueryState::Loaded(data) | QueryState::Invalid(data) => {
                Some(&mut data.data)
            }
        }
    }

    pub(crate) fn map_data<R>(&self, mapper: impl FnOnce(&V) -> R) -> QueryState<R> {
        match self {
            QueryState::Loading => QueryState::Loading,
            QueryState::Created => QueryState::Created,
            QueryState::Fetching(data) => QueryState::Fetching(QueryData {
                data: mapper(&data.data),
                updated_at: data.updated_at,
            }),
            QueryState::Loaded(data) => QueryState::Loaded(QueryData {
                data: mapper(&data.data),
                updated_at: data.updated_at,
            }),
            QueryState::Invalid(data) => QueryState::Invalid(QueryData {
                data: mapper(&data.data),
                updated_at: data.updated_at,
            }),
        }
    }
}

impl<V> std::fmt::Debug for QueryState<V>
where
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Loading => write!(f, "Loading"),
            Self::Fetching(arg0) => f.debug_tuple("Fetching").field(arg0).finish(),
            Self::Loaded(arg0) => f.debug_tuple("Loaded").field(arg0).finish(),
            Self::Invalid(arg0) => f.debug_tuple("Invalid").field(arg0).finish(),
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

impl<V> QueryData<V> {
    /// Creates a new QueryData with the given data and the current time as the updated_at timestamp.
    pub fn now(data: V) -> Self {
        Self {
            data,
            updated_at: Instant::now(),
        }
    }
}

impl<V> std::fmt::Debug for QueryData<V>
where
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryData")
            .field("data", &self.data)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}
