use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strongly-typed identifier for an attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentId(pub Uuid);

impl AttachmentId {
    /// Creates a new random attachment identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AttachmentId {
    fn default() -> Self {
        Self::new()
    }
}

/// A file attached to one or more messages.
///
/// Attachments are deduplicated by SHA-256 hash: sending the same PDF to
/// twelve people stores the content once. The `filename` and `mime_type`
/// are per-attachment since different messages may present the same file
/// under different names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// Unique identifier for this attachment record.
    pub id: AttachmentId,
    /// SHA-256 hash of the attachment content, hex-encoded.
    pub sha256: String,
    /// Size of the attachment in bytes.
    pub size_bytes: u64,
    /// MIME type as declared in the message (e.g. "application/pdf").
    pub mime_type: String,
    /// Original filename as declared in the message, if any.
    pub filename: Option<String>,
}
