//! Background backfill job.
//!
//! Low-priority worker that downloads message bodies older than the initial
//! sync window, rate-limited to avoid hammering the server or saturating
//! the user's connection.
