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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_id_generates_unique_values() {
        let a = AttachmentId::new();
        let b = AttachmentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn attachment_id_default_produces_non_nil_uuid() {
        let id = AttachmentId::default();
        assert_ne!(id.0, uuid::Uuid::nil());
    }

    #[test]
    fn attachment_with_filename_stores_value() {
        let att = Attachment {
            id: AttachmentId::new(),
            sha256: "abc123".to_owned(),
            size_bytes: 4096,
            mime_type: "application/pdf".to_owned(),
            filename: Some("document.pdf".to_owned()),
        };
        assert_eq!(att.filename.as_deref(), Some("document.pdf"));
    }

    #[test]
    fn attachment_without_filename_is_none() {
        let att = Attachment {
            id: AttachmentId::new(),
            sha256: "abc123".to_owned(),
            size_bytes: 4096,
            mime_type: "application/pdf".to_owned(),
            filename: None,
        };
        assert!(att.filename.is_none());
    }

    #[test]
    fn attachment_serialises_and_deserialises_via_json() {
        let att = Attachment {
            id: AttachmentId::new(),
            sha256: "deadbeef".to_owned(),
            size_bytes: 1024,
            mime_type: "image/png".to_owned(),
            filename: Some("photo.png".to_owned()),
        };

        let json = serde_json::to_string(&att).expect("serialize");
        let recovered: Attachment = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(att.id, recovered.id);
        assert_eq!(att.sha256, recovered.sha256);
        assert_eq!(att.size_bytes, recovered.size_bytes);
        assert_eq!(att.mime_type, recovered.mime_type);
        assert_eq!(att.filename, recovered.filename);
    }
}
