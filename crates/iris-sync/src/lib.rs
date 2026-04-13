//! Iris Sync — orchestrator for mail synchronisation and storage tiering.
//!
//! Coordinates `iris-mail` (fetching from remote servers) and `iris-db`
//! (storing locally), manages the sync state machine, runs background jobs
//! for progressive sync and storage tiering, and emits events to the UI.

mod engine;
pub mod jobs;
mod outbox;

/// Errors that can occur in the sync engine.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until real sync errors are implemented.
    #[error("not implemented")]
    NotImplemented,
}

/// Convenience alias for results using the sync [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
