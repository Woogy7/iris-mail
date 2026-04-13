//! Storage tiering enforcement job.
//!
//! Monitors server-side mailbox size and archives the oldest locally-confirmed
//! messages when the synced tier threshold is approached. Includes integrity
//! verification, minimum age checks, and dry-run support.
