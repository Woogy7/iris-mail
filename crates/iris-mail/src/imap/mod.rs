//! IMAP client operations: connection, folder discovery, message fetching.

pub mod client;
pub mod folders;
mod idle;
mod sync;

pub use client::{ImapAuth, ImapClient};
pub use folders::{DiscoveredFolder, discover_folders};
