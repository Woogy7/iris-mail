//! Microsoft Graph API client for M365 mail access.
//!
//! Replaces IMAP for M365 accounts. The Graph REST API provides folder
//! listing, message header fetching, and body retrieval via HTTP/JSON
//! instead of the IMAP protocol.

pub mod client;
pub mod folders;
pub mod messages;
pub(crate) mod types;
