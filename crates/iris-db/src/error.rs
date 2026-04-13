/// Errors that can occur in the database layer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A SQLx database operation failed.
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A database migration failed to apply.
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// A requested record was not found.
    #[error("record not found: {entity} with id {id}")]
    NotFound {
        /// The kind of entity that was looked up (e.g. "account", "folder").
        entity: &'static str,
        /// The identifier value that was not matched.
        id: String,
    },

    /// JSON serialisation or deserialisation of a database column failed.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convenience alias for results using the database [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
