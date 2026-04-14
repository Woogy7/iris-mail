//! Message fetching via IMAP FETCH command.
//!
//! Fetches message headers and bodies from the IMAP server and parses them
//! using the `mail-parser` crate.

use chrono::{DateTime, TimeZone, Utc};
use futures::TryStreamExt;
use iris_core::MessageFlags;
use mail_parser::{Address, MimeHeaders};

use crate::imap::client::ImapClient;

/// A message's headers and metadata fetched from a mail server.
///
/// Protocol-level type. The caller converts this into an `iris_core::Message`.
/// Used by both IMAP and Graph API code paths.
#[derive(Debug, Clone)]
pub struct FetchedMessage {
    /// IMAP UID (0 for Graph API messages where UIDs don't apply).
    pub uid: u32,
    /// Provider-specific remote identifier.
    ///
    /// For M365 Graph messages this is the opaque Graph message ID.
    /// For IMAP messages this is `None` — the `uid` field is the identifier.
    pub remote_id: Option<String>,
    /// RFC 2822 Message-ID header.
    pub message_id: Option<String>,
    /// Subject line.
    pub subject: Option<String>,
    /// Sender display name.
    pub from_name: Option<String>,
    /// Sender email address.
    pub from_address: Option<String>,
    /// To addresses as formatted strings.
    pub to_addresses: Vec<String>,
    /// CC addresses as formatted strings.
    pub cc_addresses: Vec<String>,
    /// Date from the Date header, converted to UTC.
    pub date: Option<DateTime<Utc>>,
    /// Message size in bytes.
    pub size: u32,
    /// Parsed flags.
    pub flags: MessageFlags,
    /// Whether the message likely has attachments (heuristic based on
    /// Content-Type "multipart/mixed" for IMAP, `hasAttachments` for Graph).
    pub has_attachment: bool,
}

/// The full body content of a fetched message.
#[derive(Debug, Clone)]
pub struct FetchedBody {
    /// IMAP UID of the message.
    pub uid: u32,
    /// HTML body, if present.
    pub html: Option<String>,
    /// Plain text body, if present.
    pub plain_text: Option<String>,
}

/// Fetch message headers and flags for a range of UIDs.
///
/// The folder must already be selected via [`ImapClient::select`].
/// Returns parsed headers for each message in the UID range. Messages
/// that lack a UID or unparseable headers are silently skipped.
pub async fn fetch_message_headers(
    client: &mut ImapClient,
    uid_range: &str,
) -> crate::Result<Vec<FetchedMessage>> {
    let fetch_stream = client
        .session
        .uid_fetch(uid_range, "(UID FLAGS RFC822.SIZE BODY.PEEK[HEADER])")
        .await
        .map_err(|e| crate::Error::Imap(format!("FETCH headers failed: {e}")))?;

    let fetches: Vec<_> = fetch_stream
        .try_collect()
        .await
        .map_err(|e| crate::Error::Imap(format!("failed to read FETCH response: {e}")))?;

    let mut messages = Vec::with_capacity(fetches.len());

    for fetch in &fetches {
        let uid = match fetch.uid {
            Some(uid) => uid,
            None => continue,
        };

        let flags = parse_imap_flags(fetch.flags());
        let size = fetch.size.unwrap_or(0);

        let header_bytes = match fetch.header() {
            Some(bytes) => bytes,
            None => continue,
        };

        let parsed = match mail_parser::MessageParser::default().parse(header_bytes) {
            Some(msg) => msg,
            None => continue,
        };

        let (from_name, from_address) = extract_from(&parsed);
        let to_addresses = extract_address_list(parsed.to());
        let cc_addresses = extract_address_list(parsed.cc());
        let date = parsed.date().and_then(convert_datetime);
        let message_id = parsed.message_id().map(|s| s.to_owned());
        let subject = parsed.subject().map(|s| s.to_owned());
        let has_attachment = detect_attachment_from_headers(&parsed);

        messages.push(FetchedMessage {
            uid,
            remote_id: None,
            message_id,
            subject,
            from_name,
            from_address,
            to_addresses,
            cc_addresses,
            date,
            size,
            flags,
            has_attachment,
        });
    }

    Ok(messages)
}

/// Fetch the full body of a single message by UID.
///
/// The folder must already be selected. Returns parsed HTML and plain text
/// parts. Fails if no message matches the UID or the body cannot be parsed.
pub async fn fetch_message_body(client: &mut ImapClient, uid: u32) -> crate::Result<FetchedBody> {
    let fetch_stream = client
        .session
        .uid_fetch(uid.to_string(), "(UID BODY[])")
        .await
        .map_err(|e| crate::Error::Imap(format!("FETCH body failed: {e}")))?;

    let fetches: Vec<_> = fetch_stream
        .try_collect()
        .await
        .map_err(|e| crate::Error::Imap(format!("failed to read FETCH body response: {e}")))?;

    let fetch = fetches
        .first()
        .ok_or_else(|| crate::Error::Imap(format!("no message found with UID {uid}")))?;

    let body_bytes = fetch
        .body()
        .ok_or_else(|| crate::Error::Imap(format!("no body in FETCH response for UID {uid}")))?;

    let parsed = mail_parser::MessageParser::default()
        .parse(body_bytes)
        .ok_or_else(|| {
            crate::Error::MessageParse(format!("failed to parse message body for UID {uid}"))
        })?;

    let html = parsed.body_html(0).map(|s| s.into_owned());
    let plain_text = parsed.body_text(0).map(|s| s.into_owned());

    Ok(FetchedBody {
        uid,
        html,
        plain_text,
    })
}

/// Map IMAP flags to iris-core [`MessageFlags`].
fn parse_imap_flags<'a>(flags: impl Iterator<Item = async_imap::types::Flag<'a>>) -> MessageFlags {
    let mut result = MessageFlags::default();
    for flag in flags {
        match flag {
            async_imap::types::Flag::Seen => result.is_read = true,
            async_imap::types::Flag::Flagged => result.is_flagged = true,
            async_imap::types::Flag::Answered => result.is_answered = true,
            _ => {}
        }
    }
    result
}

/// Extract sender name and address from a parsed message's From header.
fn extract_from(msg: &mail_parser::Message<'_>) -> (Option<String>, Option<String>) {
    let from = match msg.from() {
        Some(addr) => addr,
        None => return (None, None),
    };
    let first = match from.first() {
        Some(addr) => addr,
        None => return (None, None),
    };
    (
        first.name().map(|s| s.to_owned()),
        first.address().map(|s| s.to_owned()),
    )
}

/// Extract a list of formatted email addresses from an address header.
///
/// Returns a `Vec` of strings formatted as `"Name <email>"` when the address
/// has a display name, or just `"email"` otherwise.
fn extract_address_list(header: Option<&Address<'_>>) -> Vec<String> {
    let address = match header {
        Some(addr) => addr,
        None => return Vec::new(),
    };

    address
        .iter()
        .filter_map(|addr| {
            let email = addr.address()?;
            Some(match addr.name() {
                Some(name) => format!("{name} <{email}>"),
                None => email.to_owned(),
            })
        })
        .collect()
}

/// Heuristic attachment detection from message headers.
///
/// Checks whether the top-level Content-Type is `multipart/mixed`, which
/// is the standard MIME type for messages with attachments. This is a best
/// guess from headers alone; accurate detection requires parsing the full body.
fn detect_attachment_from_headers(msg: &mail_parser::Message<'_>) -> bool {
    msg.content_type().is_some_and(|ct| {
        ct.c_type.eq_ignore_ascii_case("multipart")
            && ct
                .c_subtype
                .as_ref()
                .is_some_and(|sub| sub.eq_ignore_ascii_case("mixed"))
    })
}

/// Convert a `mail_parser::DateTime` to a `chrono::DateTime<Utc>`.
///
/// Returns `None` if the date components are out of range (e.g. month 0).
fn convert_datetime(dt: &mail_parser::DateTime) -> Option<DateTime<Utc>> {
    let timestamp = dt.to_timestamp();
    Utc.timestamp_opt(timestamp, 0).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_imap::types::Flag;

    #[test]
    fn parse_imap_flags_maps_seen_to_is_read() {
        let flags = vec![Flag::Seen];
        let result = parse_imap_flags(flags.into_iter());
        assert!(result.is_read);
        assert!(!result.is_flagged);
        assert!(!result.is_answered);
    }

    #[test]
    fn parse_imap_flags_maps_multiple_flags() {
        let flags = vec![Flag::Seen, Flag::Flagged, Flag::Answered];
        let result = parse_imap_flags(flags.into_iter());
        assert!(result.is_read);
        assert!(result.is_flagged);
        assert!(result.is_answered);
    }

    #[test]
    fn parse_imap_flags_ignores_unknown_flags() {
        let flags = vec![Flag::Recent, Flag::Draft];
        let result = parse_imap_flags(flags.into_iter());
        assert!(!result.is_read);
        assert!(!result.is_flagged);
        assert!(!result.is_answered);
    }

    #[test]
    fn empty_flags_returns_default() {
        let flags: Vec<Flag<'_>> = vec![];
        let result = parse_imap_flags(flags.into_iter());
        assert_eq!(result, MessageFlags::default());
    }

    #[test]
    fn extract_from_returns_name_and_address() {
        let raw = b"From: Alice <alice@example.com>\r\nSubject: test\r\n\r\n";
        let parsed = mail_parser::MessageParser::default()
            .parse(raw)
            .expect("valid message");
        let (name, addr) = extract_from(&parsed);
        assert_eq!(name.as_deref(), Some("Alice"));
        assert_eq!(addr.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn extract_from_returns_none_when_missing() {
        let raw = b"Subject: no from\r\n\r\n";
        let parsed = mail_parser::MessageParser::default()
            .parse(raw)
            .expect("valid message");
        let (name, addr) = extract_from(&parsed);
        assert!(name.is_none());
        assert!(addr.is_none());
    }

    #[test]
    fn extract_address_list_formats_with_name() {
        let raw = b"From: x@x.com\r\nTo: Bob <bob@example.com>, carol@example.com\r\n\r\n";
        let parsed = mail_parser::MessageParser::default()
            .parse(raw)
            .expect("valid message");
        let addrs = extract_address_list(parsed.to());
        assert_eq!(addrs.len(), 2);
        assert_eq!(addrs[0], "Bob <bob@example.com>");
        assert_eq!(addrs[1], "carol@example.com");
    }

    #[test]
    fn extract_address_list_returns_empty_for_none() {
        let addrs = extract_address_list(None);
        assert!(addrs.is_empty());
    }

    #[test]
    fn detect_attachment_from_multipart_mixed() {
        let raw = b"Content-Type: multipart/mixed; boundary=\"abc\"\r\n\r\n";
        let parsed = mail_parser::MessageParser::default()
            .parse(raw)
            .expect("valid message");
        assert!(detect_attachment_from_headers(&parsed));
    }

    #[test]
    fn detect_attachment_returns_false_for_plain_text() {
        let raw = b"Content-Type: text/plain\r\n\r\nHello";
        let parsed = mail_parser::MessageParser::default()
            .parse(raw)
            .expect("valid message");
        assert!(!detect_attachment_from_headers(&parsed));
    }

    #[test]
    fn convert_datetime_produces_correct_utc() {
        let raw = b"Date: Sat, 20 Nov 2021 14:22:01 -0800\r\n\r\n";
        let parsed = mail_parser::MessageParser::default()
            .parse(raw)
            .expect("valid message");
        let date = parsed.date().and_then(convert_datetime);
        let date = date.expect("should parse date");
        assert_eq!(
            date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2021-11-20 22:22:01"
        );
    }
}
