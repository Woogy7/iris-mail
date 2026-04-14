//! IMAP client operations: connection, folder discovery, message fetching.

pub mod client;
pub mod fetch;
pub mod folders;
mod idle;
mod sync;

pub use client::{ImapAuth, ImapClient};
pub use fetch::{FetchedBody, FetchedMessage, fetch_message_body, fetch_message_headers};
pub use folders::{DiscoveredFolder, discover_folders};
