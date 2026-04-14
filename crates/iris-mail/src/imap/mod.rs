//! IMAP client operations: connection, folder discovery, message fetching.

pub mod client;
mod idle;
mod sync;

pub use client::ImapClient;
