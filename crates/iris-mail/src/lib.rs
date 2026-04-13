//! Iris Mail — IMAP, SMTP, and OAuth protocol layer.
//!
//! This crate wraps the mail protocol libraries (`async-imap`, `lettre`,
//! `oauth2`) and exposes a clean async API that returns domain types from
//! `iris-core`. It knows nothing about SQLite or the local database.

pub mod imap;
pub mod oauth;
mod smtp;

/// Errors that can occur in the mail protocol layer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until real protocol errors are implemented.
    #[error("not implemented")]
    NotImplemented,
}

/// Convenience alias for results using the mail [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
