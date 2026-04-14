//! Iris Mail — IMAP, SMTP, Graph, and OAuth protocol layer.
//!
//! This crate wraps the mail protocol libraries (`async-imap`, `lettre`,
//! `reqwest`) and exposes a clean async API that returns domain types from
//! `iris-core`. It knows nothing about SQLite or the local database.
//!
//! M365 accounts use the Microsoft Graph REST API instead of IMAP. The
//! [`graph`] module provides folder listing, message fetching, and body
//! retrieval via HTTP/JSON.

pub mod discovery;
pub mod graph;
pub mod imap;
pub mod oauth;
pub mod smtp;

pub use discovery::discover_servers;
pub use graph::client::GraphClient;
pub use graph::folders::list_graph_folders;
pub use graph::messages::{fetch_graph_message_body, fetch_graph_messages};
pub use imap::client::{ImapAuth, ImapClient};
pub use imap::fetch::{FetchedBody, FetchedMessage, fetch_message_body, fetch_message_headers};
pub use imap::folders::{DiscoveredFolder, discover_folders};
pub use oauth::{OauthTokens, keychain::KeychainStore};
pub use smtp::validate_smtp_connection;

/// Errors that can occur in the mail protocol layer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error originating from an OAuth2 flow.
    #[error("OAuth error: {0}")]
    Oauth(String),

    /// The requested token was not found in the keychain.
    #[error("token not found for account {0}")]
    TokenNotFound(String),

    /// A token refresh attempt failed.
    #[error("token refresh failed: {0}")]
    TokenRefreshFailed(String),

    /// An error from the OS keychain.
    #[error("keychain error: {0}")]
    Keychain(String),

    /// A Microsoft Graph API error.
    #[error("Graph API error: {0}")]
    Graph(String),

    /// An IMAP protocol error.
    #[error("IMAP error: {0}")]
    Imap(String),

    /// An SMTP protocol error.
    #[error("SMTP error: {0}")]
    Smtp(String),

    /// Automatic server discovery failed for the given domain.
    #[error("server discovery failed for domain {domain}: {reason}")]
    Discovery {
        /// The domain that was being resolved.
        domain: String,
        /// Why discovery failed.
        reason: String,
    },

    /// A network operation timed out.
    #[error("connection timeout after {0} seconds")]
    Timeout(u64),

    /// Failed to parse a mail message.
    #[error("message parse error: {0}")]
    MessageParse(String),

    /// Functionality that is not yet implemented.
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

/// Convenience alias for results using the mail [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
