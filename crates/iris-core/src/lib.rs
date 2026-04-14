//! Iris Core — shared domain types for the Iris Mail application.
//!
//! This crate defines the vocabulary used across all other Iris crates: accounts,
//! folders, messages, attachments, sync state, and the core error type. It contains
//! no I/O and depends on no other Iris crates.

mod account;
mod attachment;
mod error;
mod folder;
mod message;
mod server;
mod sync_state;

pub use account::{AccentColour, Account, AccountId, Provider, SyncPreferences};
pub use attachment::{Attachment, AttachmentId};
pub use error::{Error, Result};
pub use folder::{Folder, FolderId, SpecialFolder};
pub use message::{Message, MessageBody, MessageFlags, MessageId, StorageState};
pub use server::{ImapServer, ServerConfig, SmtpServer};
pub use sync_state::{SyncPhase, SyncState};
