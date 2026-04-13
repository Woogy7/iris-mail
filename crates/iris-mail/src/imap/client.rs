//! IMAP connection management and command execution.
//!
//! Wraps `async-imap` to provide a higher-level async client that handles
//! authentication (XOAUTH2 for M365/Gmail, PLAIN for generic IMAP),
//! connection pooling, and automatic reconnection.
