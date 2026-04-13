//! IMAP IDLE monitoring job.
//!
//! Maintains persistent IDLE connections for real-time new mail delivery.
//! Falls back to periodic polling for servers that do not support IDLE.
