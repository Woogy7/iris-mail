use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AccountId;

/// Strongly-typed identifier for a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FolderId(pub Uuid);

impl FolderId {
    /// Creates a new random folder identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FolderId {
    fn default() -> Self {
        Self::new()
    }
}

/// The well-known role of a folder, if any.
///
/// Used to apply special behaviour: Inbox and Other folders are eligible for
/// storage tiering, while Sent, Drafts, Trash, and Archive are never tiered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecialFolder {
    /// The primary inbox.
    Inbox,
    /// Sent messages.
    Sent,
    /// Message drafts.
    Drafts,
    /// Deleted messages.
    Trash,
    /// Archived messages.
    Archive,
    /// A user-created or unrecognised folder.
    Other,
}

impl SpecialFolder {
    /// Returns `true` if this folder type is eligible for storage tiering.
    ///
    /// Per the spec, only Inbox and user-created (Other) folders can have
    /// their oldest messages archived to free server quota. Sent, Drafts,
    /// Trash, and Archive are never touched by the tiering engine.
    pub fn is_tierable(&self) -> bool {
        matches!(self, Self::Inbox | Self::Other)
    }
}

/// A mail folder within an account's folder tree.
///
/// Folders form a hierarchy via `parent_id` and track per-folder IMAP sync
/// state so incremental sync can resume efficiently after interruption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    /// Unique identifier for this folder.
    pub id: FolderId,
    /// The account this folder belongs to.
    pub account_id: AccountId,
    /// Parent folder for nested hierarchies, or `None` for top-level folders.
    pub parent_id: Option<FolderId>,
    /// Display name of this folder (e.g. "Inbox", "Receipts").
    pub name: String,
    /// Full IMAP path including hierarchy separator (e.g. "INBOX/Receipts").
    pub full_path: String,
    /// The well-known role of this folder, if any.
    pub special: SpecialFolder,
    /// IMAP UIDVALIDITY for change detection.
    pub uid_validity: Option<u32>,
    /// The highest UID we have synced so far.
    pub last_seen_uid: Option<u32>,
    /// Total number of messages in this folder (local count).
    pub message_count: u32,
    /// Number of unread messages in this folder (local count).
    pub unread_count: u32,
    /// When this folder was last successfully synced.
    pub last_synced_at: Option<DateTime<Utc>>,
    /// When this folder record was created.
    pub created_at: DateTime<Utc>,
    /// When this folder record was last updated.
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_is_tierable() {
        assert!(SpecialFolder::Inbox.is_tierable());
    }

    #[test]
    fn other_folders_are_tierable() {
        assert!(SpecialFolder::Other.is_tierable());
    }

    #[test]
    fn sent_is_not_tierable() {
        assert!(!SpecialFolder::Sent.is_tierable());
    }

    #[test]
    fn drafts_is_not_tierable() {
        assert!(!SpecialFolder::Drafts.is_tierable());
    }

    #[test]
    fn trash_is_not_tierable() {
        assert!(!SpecialFolder::Trash.is_tierable());
    }

    #[test]
    fn archive_is_not_tierable() {
        assert!(!SpecialFolder::Archive.is_tierable());
    }
}
