//! Internal serde response types for Microsoft Graph API JSON.
//!
//! These types mirror the Graph API JSON schema for mail-related endpoints.
//! They are `pub(crate)` because callers should use the higher-level functions
//! in [`super::folders`] and [`super::messages`] instead.

use serde::Deserialize;

/// A paginated response wrapper used by all Graph API list endpoints.
#[derive(Debug, Deserialize)]
pub(crate) struct GraphResponse<T> {
    /// The items in this page of results.
    pub value: Vec<T>,
    /// URL to fetch the next page, if more results exist.
    #[serde(rename = "@odata.nextLink")]
    pub next_link: Option<String>,
}

/// A mail folder from the Graph API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Count fields used by sync progress reporting in upcoming tasks.
pub(crate) struct GraphFolder {
    /// Opaque Graph folder identifier.
    pub id: String,
    /// User-visible folder name.
    pub display_name: String,
    /// Well-known name (e.g. "inbox", "sentitems", "drafts").
    #[serde(default)]
    pub well_known_name: Option<String>,
    /// Total number of items in the folder.
    #[serde(default)]
    pub total_item_count: u32,
    /// Number of unread items in the folder.
    #[serde(default)]
    pub unread_item_count: u32,
}

/// An email address within a Graph API recipient object.
#[derive(Debug, Deserialize)]
pub(crate) struct GraphEmailAddress {
    /// Display name (may be absent).
    pub name: Option<String>,
    /// The email address string.
    pub address: Option<String>,
}

/// A recipient object wrapping an email address.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphRecipient {
    /// The recipient's email address and name.
    pub email_address: GraphEmailAddress,
}

/// The flag status on a message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphFlag {
    /// One of "notFlagged", "flagged", or "complete".
    pub flag_status: Option<String>,
}

/// A message from the Graph API (headers only, no body content).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Preview/conversation fields used by threading in upcoming tasks.
pub(crate) struct GraphMessage {
    /// Opaque Graph message identifier.
    pub id: String,
    /// Subject line.
    #[serde(default)]
    pub subject: Option<String>,
    /// The sender.
    pub from: Option<GraphRecipient>,
    /// To recipients.
    #[serde(default)]
    pub to_recipients: Vec<GraphRecipient>,
    /// CC recipients.
    #[serde(default)]
    pub cc_recipients: Vec<GraphRecipient>,
    /// When the message was received (ISO 8601).
    pub received_date_time: Option<String>,
    /// Whether the message has been read.
    #[serde(default)]
    pub is_read: bool,
    /// Flag status.
    pub flag: Option<GraphFlag>,
    /// Whether the message has attachments.
    #[serde(default)]
    pub has_attachments: bool,
    /// A short plain-text preview of the body.
    #[serde(default)]
    pub body_preview: Option<String>,
    /// The RFC 2822 Message-ID header value.
    #[serde(default)]
    pub internet_message_id: Option<String>,
    /// Conversation identifier for threading.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// The body content of a message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphBody {
    /// "html" or "text".
    pub content_type: Option<String>,
    /// The actual body content.
    pub content: Option<String>,
}

/// A message with its full body, returned when fetching a single message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Header fields included for complete deserialization; used by sync.
pub(crate) struct GraphMessageWithBody {
    /// Opaque Graph message identifier.
    pub id: String,
    /// The message body.
    pub body: Option<GraphBody>,
    /// Subject line.
    #[serde(default)]
    pub subject: Option<String>,
    /// The sender.
    pub from: Option<GraphRecipient>,
    /// To recipients.
    #[serde(default)]
    pub to_recipients: Vec<GraphRecipient>,
    /// CC recipients.
    #[serde(default)]
    pub cc_recipients: Vec<GraphRecipient>,
    /// When the message was received (ISO 8601).
    pub received_date_time: Option<String>,
}

/// An attachment from the Graph API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphAttachment {
    /// Whether this attachment is inline (embedded in the body via cid:).
    #[serde(default)]
    pub is_inline: bool,
    /// The Content-ID for inline attachments (without angle brackets).
    #[serde(default)]
    pub content_id: Option<String>,
    /// MIME type (e.g. "image/png").
    #[serde(default)]
    pub content_type: Option<String>,
    /// Base64-encoded content bytes.
    #[serde(default)]
    pub content_bytes: Option<String>,
}
