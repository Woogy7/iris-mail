//! Iris Search — query language parser and FTS5 search execution.
//!
//! Parses the GitHub-style query language (e.g. `from:terry subject:invoice
//! has:attachment after:2025-01-01 widgets`), translates it into SQL + FTS5
//! match expressions, and returns ranked results.

mod executor;
mod parser;

/// Errors that can occur in the search layer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until real search errors are implemented.
    #[error("not implemented")]
    NotImplemented,
}

/// Convenience alias for results using the search [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
