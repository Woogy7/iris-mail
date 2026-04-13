use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AccountId, FolderId};

/// Strongly-typed identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    /// Creates a new random message identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Boolean flags that track user-visible message state.
///
/// All flags default to `false` (unread, unflagged, unanswered).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFlags {
    /// Whether the message has been read.
    pub is_read: bool,
    /// Whether the message has been flagged/starred by the user.
    pub is_flagged: bool,
    /// Whether the message has been replied to.
    pub is_answered: bool,
}

/// Where a message's content is currently stored.
///
/// Drives the local/remote indicator in the UI (the two-dot glyph described
/// in the spec). See [`Message::storage_state`] for how this is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageState {
    /// Full body and attachments exist both locally and on the server.
    SyncedBoth,
    /// The message has been archived locally; it no longer exists on the server.
    LocalOnly,
    /// Headers are present locally but the body is still on the server only.
    RemoteOnly,
}

/// A single email message.
///
/// Contains headers, flags, and storage metadata. The full body is stored
/// separately in [`MessageBody`] so that list views can load thousands of
/// message rows without pulling megabytes of HTML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message.
    pub id: MessageId,
    /// The account this message belongs to.
    pub account_id: AccountId,
    /// The folder this message is filed in.
    pub folder_id: FolderId,
    /// IMAP UID within the folder.
    pub uid: Option<u32>,
    /// The RFC 2822 Message-ID header value.
    pub message_id_header: Option<String>,
    /// Thread identifier for conversation grouping.
    pub thread_id: Option<String>,
    /// Message subject line.
    pub subject: Option<String>,
    /// Sender display name.
    pub from_name: Option<String>,
    /// Sender email address.
    pub from_address: Option<String>,
    /// Comma-separated To recipients.
    pub to_addresses: Option<String>,
    /// Comma-separated CC recipients.
    pub cc_addresses: Option<String>,
    /// Comma-separated BCC recipients.
    pub bcc_addresses: Option<String>,
    /// When the message was sent (from the Date header).
    pub date: Option<DateTime<Utc>>,
    /// Total message size in bytes.
    pub size_bytes: Option<u64>,
    /// User-visible boolean flags (read, flagged, answered).
    pub flags: MessageFlags,
    /// Whether the full body and attachments are stored locally.
    pub is_stored_local: bool,
    /// Whether the message is still present on the remote server.
    pub is_stored_remote: bool,
    /// When this message record was created in the local database.
    pub created_at: DateTime<Utc>,
    /// When this message record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Message {
    /// Derives the storage state from the local/remote storage flags.
    ///
    /// Returns `None` if both flags are false, which indicates a bug — a message
    /// should always exist in at least one location.
    pub fn storage_state(&self) -> Option<StorageState> {
        match (self.is_stored_local, self.is_stored_remote) {
            (true, true) => Some(StorageState::SyncedBoth),
            (true, false) => Some(StorageState::LocalOnly),
            (false, true) => Some(StorageState::RemoteOnly),
            (false, false) => None,
        }
    }
}

/// The full body content of a message, stored separately from headers.
///
/// Keeping bodies in a separate table means the message list can load thousands
/// of rows without pulling megabytes of HTML into memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageBody {
    /// The message this body belongs to.
    pub message_id: MessageId,
    /// The original HTML body as received.
    pub html: Option<String>,
    /// The HTML body after sanitisation (scripts removed, styles cleaned).
    pub sanitised_html: Option<String>,
    /// Plain text version, used for search indexing and fallback display.
    pub plain_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_state_returns_synced_both_when_both_flags_are_true() {
        let msg = make_message(true, true);
        assert_eq!(msg.storage_state(), Some(StorageState::SyncedBoth));
    }

    #[test]
    fn storage_state_returns_local_only_when_only_local_is_true() {
        let msg = make_message(true, false);
        assert_eq!(msg.storage_state(), Some(StorageState::LocalOnly));
    }

    #[test]
    fn storage_state_returns_remote_only_when_only_remote_is_true() {
        let msg = make_message(false, true);
        assert_eq!(msg.storage_state(), Some(StorageState::RemoteOnly));
    }

    #[test]
    fn storage_state_returns_none_when_both_flags_are_false() {
        let msg = make_message(false, false);
        assert_eq!(msg.storage_state(), None);
    }

    fn make_message(is_stored_local: bool, is_stored_remote: bool) -> Message {
        let now = Utc::now();
        Message {
            id: MessageId::new(),
            account_id: crate::AccountId::new(),
            folder_id: crate::FolderId::new(),
            uid: None,
            message_id_header: None,
            thread_id: None,
            subject: None,
            from_name: None,
            from_address: None,
            to_addresses: None,
            cc_addresses: None,
            bcc_addresses: None,
            date: None,
            size_bytes: None,
            flags: MessageFlags::default(),
            is_stored_local,
            is_stored_remote,
            created_at: now,
            updated_at: now,
        }
    }
}
