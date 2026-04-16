//! Message fetching via the Microsoft Graph API.
//!
//! Provides functions to list message headers from a folder and to fetch
//! the full body of a single message by its Graph ID.

use crate::graph::client::GraphClient;
use crate::graph::types::{GraphAttachment, GraphMessage, GraphMessageWithBody, GraphResponse};
use crate::imap::fetch::{FetchedBody, FetchedMessage};
use iris_core::MessageFlags;

/// Fetches message headers from a Graph mail folder.
///
/// The `folder_id` is the opaque Graph folder ID returned by
/// [`super::folders::list_graph_folders`]. Returns up to `limit` messages,
/// newest first.
pub async fn fetch_graph_messages(
    client: &GraphClient,
    folder_id: &str,
    limit: u32,
) -> crate::Result<Vec<FetchedMessage>> {
    let path = format!(
        "/me/mailFolders/{folder_id}/messages\
         ?$top={limit}\
         &$orderby=receivedDateTime desc\
         &$select=id,subject,from,toRecipients,ccRecipients,\
         receivedDateTime,isRead,flag,hasAttachments,bodyPreview,\
         internetMessageId,conversationId"
    );

    let resp = client.get(&path).await?;
    let data: GraphResponse<GraphMessage> = resp
        .json()
        .await
        .map_err(|e| crate::Error::Graph(format!("failed to parse message response: {e}")))?;

    let count = data.value.len();
    let messages = data
        .value
        .into_iter()
        .map(graph_message_to_fetched)
        .collect();

    tracing::info!("Graph: fetched {count} messages from folder {folder_id}");
    Ok(messages)
}

/// Fetches the full body of a single message by its Graph ID.
///
/// Also fetches inline attachments and resolves `cid:` references in the HTML
/// body by replacing them with `data:` URIs so embedded images render correctly.
pub async fn fetch_graph_message_body(
    client: &GraphClient,
    message_id: &str,
) -> crate::Result<FetchedBody> {
    let path = format!("/me/messages/{message_id}");
    let resp = client.get(&path).await?;
    let data: GraphMessageWithBody = resp
        .json()
        .await
        .map_err(|e| crate::Error::Graph(format!("failed to parse message body: {e}")))?;

    let (mut html, plain_text) = match data.body {
        Some(body) => match body.content_type.as_deref() {
            Some("html") => (body.content, None),
            Some("text") => (None, body.content),
            _ => (body.content, None),
        },
        None => (None, None),
    };

    // Resolve inline images: fetch attachments and replace cid: references.
    if html.is_some() {
        match fetch_inline_attachments(client, message_id).await {
            Ok(attachments) => {
                if let Some(ref mut html_content) = html {
                    for att in &attachments {
                        if let (Some(cid), Some(content_type), Some(content_bytes)) =
                            (&att.content_id, &att.content_type, &att.content_bytes)
                        {
                            let data_uri = format!("data:{content_type};base64,{content_bytes}");
                            let stripped_cid = cid.trim_start_matches('<').trim_end_matches('>');
                            *html_content = html_content.replace(
                                &format!("cid:{cid}"),
                                &data_uri,
                            );
                            *html_content = html_content.replace(
                                &format!("cid:{stripped_cid}"),
                                &data_uri,
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch inline attachments for {message_id}: {e}");
            }
        }
    }

    Ok(FetchedBody {
        uid: 0,
        html,
        plain_text,
    })
}

/// Fetches inline attachments for a message.
async fn fetch_inline_attachments(
    client: &GraphClient,
    message_id: &str,
) -> crate::Result<Vec<GraphAttachment>> {
    let path = format!("/me/messages/{message_id}/attachments");
    let resp = client.get(&path).await?;
    let data: GraphResponse<GraphAttachment> = resp
        .json()
        .await
        .map_err(|e| crate::Error::Graph(format!("failed to parse attachments: {e}")))?;

    Ok(data.value.into_iter().filter(|a| a.is_inline).collect())
}

/// Converts a [`GraphMessage`] into a [`FetchedMessage`].
fn graph_message_to_fetched(gm: GraphMessage) -> FetchedMessage {
    let (from_name, from_address) = match &gm.from {
        Some(r) => (
            r.email_address.name.clone(),
            r.email_address.address.clone(),
        ),
        None => (None, None),
    };

    let to_addresses = format_recipients(&gm.to_recipients);
    let cc_addresses = format_recipients(&gm.cc_recipients);
    let date = gm
        .received_date_time
        .as_deref()
        .and_then(parse_graph_datetime);

    let is_flagged = gm
        .flag
        .as_ref()
        .and_then(|f| f.flag_status.as_deref())
        .is_some_and(|s| s == "flagged");

    FetchedMessage {
        uid: 0,
        remote_id: Some(gm.id),
        message_id: gm.internet_message_id,
        subject: gm.subject,
        from_name,
        from_address,
        to_addresses,
        cc_addresses,
        date,
        size: 0,
        flags: MessageFlags {
            is_read: gm.is_read,
            is_flagged,
            is_answered: false,
        },
        has_attachment: gm.has_attachments,
    }
}

/// Formats a list of Graph recipients into display strings.
fn format_recipients(recipients: &[crate::graph::types::GraphRecipient]) -> Vec<String> {
    recipients
        .iter()
        .filter_map(|r| {
            let addr = r.email_address.address.as_deref()?;
            match &r.email_address.name {
                Some(name) if !name.is_empty() => Some(format!("{name} <{addr}>")),
                _ => Some(addr.to_string()),
            }
        })
        .collect()
}

/// Parses a Graph API datetime string (ISO 8601) to `chrono::DateTime<Utc>`.
///
/// Graph returns dates like `"2026-04-14T12:00:00Z"` or with fractional
/// seconds like `"2026-04-14T12:00:00.0000000Z"`. Falls back to parsing
/// without a timezone suffix for edge cases.
fn parse_graph_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|ndt| ndt.and_utc())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn parse_graph_datetime_with_z_suffix() {
        let dt = parse_graph_datetime("2026-04-14T12:00:00Z");
        assert!(dt.is_some());
        assert_eq!(dt.expect("should parse").year(), 2026);
    }

    #[test]
    fn parse_graph_datetime_with_fractional_seconds() {
        let dt = parse_graph_datetime("2026-04-14T12:00:00.0000000Z");
        assert!(dt.is_some());
    }

    #[test]
    fn parse_graph_datetime_returns_none_for_garbage() {
        assert!(parse_graph_datetime("not a date").is_none());
    }

    #[test]
    fn parse_graph_datetime_without_timezone() {
        let dt = parse_graph_datetime("2026-04-14T12:00:00");
        assert!(dt.is_some());
    }

    #[test]
    fn format_recipients_with_name_and_address() {
        let recipients = vec![crate::graph::types::GraphRecipient {
            email_address: crate::graph::types::GraphEmailAddress {
                name: Some("Alice".to_string()),
                address: Some("alice@example.com".to_string()),
            },
        }];
        let formatted = format_recipients(&recipients);
        assert_eq!(formatted, vec!["Alice <alice@example.com>"]);
    }

    #[test]
    fn format_recipients_without_name() {
        let recipients = vec![crate::graph::types::GraphRecipient {
            email_address: crate::graph::types::GraphEmailAddress {
                name: None,
                address: Some("bob@example.com".to_string()),
            },
        }];
        let formatted = format_recipients(&recipients);
        assert_eq!(formatted, vec!["bob@example.com"]);
    }

    #[test]
    fn format_recipients_skips_entries_without_address() {
        let recipients = vec![crate::graph::types::GraphRecipient {
            email_address: crate::graph::types::GraphEmailAddress {
                name: Some("No Email".to_string()),
                address: None,
            },
        }];
        let formatted = format_recipients(&recipients);
        assert!(formatted.is_empty());
    }

    #[test]
    fn format_recipients_with_empty_name_uses_address_only() {
        let recipients = vec![crate::graph::types::GraphRecipient {
            email_address: crate::graph::types::GraphEmailAddress {
                name: Some(String::new()),
                address: Some("carol@example.com".to_string()),
            },
        }];
        let formatted = format_recipients(&recipients);
        assert_eq!(formatted, vec!["carol@example.com"]);
    }

    #[test]
    fn graph_message_to_fetched_sets_remote_id() {
        let gm = GraphMessage {
            id: "abc123".to_string(),
            subject: Some("Test".to_string()),
            from: None,
            to_recipients: vec![],
            cc_recipients: vec![],
            received_date_time: None,
            is_read: false,
            flag: None,
            has_attachments: false,
            body_preview: None,
            internet_message_id: None,
            conversation_id: None,
        };

        let fetched = graph_message_to_fetched(gm);
        assert_eq!(fetched.remote_id.as_deref(), Some("abc123"));
        assert_eq!(fetched.uid, 0);
    }

    #[test]
    fn graph_message_to_fetched_parses_flagged_status() {
        let gm = GraphMessage {
            id: "msg1".to_string(),
            subject: None,
            from: None,
            to_recipients: vec![],
            cc_recipients: vec![],
            received_date_time: None,
            is_read: true,
            flag: Some(crate::graph::types::GraphFlag {
                flag_status: Some("flagged".to_string()),
            }),
            has_attachments: true,
            body_preview: None,
            internet_message_id: Some("<test@example.com>".to_string()),
            conversation_id: None,
        };

        let fetched = graph_message_to_fetched(gm);
        assert!(fetched.flags.is_read);
        assert!(fetched.flags.is_flagged);
        assert!(!fetched.flags.is_answered);
        assert!(fetched.has_attachment);
        assert_eq!(fetched.message_id.as_deref(), Some("<test@example.com>"));
    }
}
