//! Iris Import — PST and mbox file import.
//!
//! Isolated into its own crate so that heavy dependencies (e.g. `libpff`
//! bindings for PST parsing) do not bloat the main application binary.

mod pst;

/// Errors that can occur during file import.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until real import errors are implemented.
    #[error("not implemented")]
    NotImplemented,
}

/// Convenience alias for results using the import [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
