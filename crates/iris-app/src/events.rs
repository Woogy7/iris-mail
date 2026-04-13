//! Event name constants for Tauri event emission.
//!
//! These events are emitted by the Rust backend and consumed by the Svelte
//! frontend. They are defined here as constants so that both the emitter
//! and the subscriber refer to the same strings.

/// Emitted during sync to report progress.
pub const SYNC_PROGRESS: &str = "sync-progress";

/// Emitted when a new message arrives.
pub const NEW_MESSAGE: &str = "new-message";

/// Emitted when an account's state changes (connected, disconnected, error).
pub const ACCOUNT_STATE_CHANGED: &str = "account-state-changed";
