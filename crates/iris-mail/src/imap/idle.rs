//! IMAP IDLE support for real-time new mail notification.
//!
//! Maintains a persistent IDLE connection to the Inbox folder, breaking
//! and re-entering IDLE periodically to comply with server timeouts.
