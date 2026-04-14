//! IMAP folder discovery via LIST command.
//!
//! Issues a LIST command to enumerate all mailboxes, maps IMAP special-use
//! attributes (RFC 6154) to [`SpecialFolder`] variants, and provides
//! name-based heuristics for servers that don't advertise attributes.

use async_imap::types::NameAttribute;
use futures::TryStreamExt;
use iris_core::SpecialFolder;

use crate::imap::client::ImapClient;

/// A folder discovered via IMAP LIST command.
///
/// This is a protocol-level type. The caller is responsible for converting
/// it into an [`iris_core::Folder`] and persisting it.
#[derive(Debug, Clone)]
pub struct DiscoveredFolder {
    /// Full IMAP path (e.g. "INBOX", "INBOX/Receipts", "[Gmail]/Sent Mail").
    pub full_path: String,
    /// Display name (last segment of the path after splitting on delimiter).
    pub name: String,
    /// The hierarchy delimiter for this folder (usually "/" or ".").
    pub delimiter: Option<String>,
    /// The detected special-use role.
    pub special: SpecialFolder,
}

/// Discover all folders on the IMAP server.
///
/// Issues a LIST command to enumerate all mailboxes, maps special-use
/// attributes to [`SpecialFolder`] variants, and skips non-selectable folders.
pub async fn discover_folders(client: &mut ImapClient) -> crate::Result<Vec<DiscoveredFolder>> {
    let names_stream = client
        .session
        .list(None, Some("*"))
        .await
        .map_err(|e| crate::Error::Imap(format!("LIST command failed: {e}")))?;

    let names: Vec<_> = names_stream
        .try_collect()
        .await
        .map_err(|e| crate::Error::Imap(format!("failed to read LIST response: {e}")))?;

    let mut folders = Vec::new();

    for name in &names {
        if is_noselect(name.attributes()) {
            continue;
        }

        let full_path = name.name().to_owned();
        let delimiter = name.delimiter().map(|d| d.to_owned());

        let display_name = match &delimiter {
            Some(delim) => full_path
                .rsplit_once(delim.as_str())
                .map_or_else(|| full_path.clone(), |(_, last)| last.to_owned()),
            None => full_path.clone(),
        };

        let special = map_special_use(&full_path, name.attributes());

        folders.push(DiscoveredFolder {
            full_path,
            name: display_name,
            delimiter,
            special,
        });
    }

    tracing::info!("Discovered {} folders", folders.len());
    Ok(folders)
}

/// Returns `true` if the folder attributes indicate it cannot be selected.
fn is_noselect(attributes: &[NameAttribute<'_>]) -> bool {
    attributes
        .iter()
        .any(|attr| matches!(attr, NameAttribute::NoSelect))
}

/// Maps IMAP folder attributes and name to a [`SpecialFolder`] variant.
///
/// Checks IMAP special-use attributes first (RFC 6154), then falls back
/// to name-based heuristics for servers that don't advertise attributes.
fn map_special_use(name: &str, attributes: &[NameAttribute<'_>]) -> SpecialFolder {
    // Check RFC 6154 special-use attributes first.
    for attr in attributes {
        match attr {
            NameAttribute::Sent => return SpecialFolder::Sent,
            NameAttribute::Drafts => return SpecialFolder::Drafts,
            NameAttribute::Trash => return SpecialFolder::Trash,
            NameAttribute::Junk => return SpecialFolder::Trash,
            NameAttribute::Archive => return SpecialFolder::Archive,
            NameAttribute::All => return SpecialFolder::Archive,
            _ => {}
        }
    }

    // Name-based fallback (case-insensitive, last path segment only).
    let lower = name.to_lowercase();
    let last_segment = last_path_segment(&lower);

    if last_segment == "inbox" {
        return SpecialFolder::Inbox;
    }
    if last_segment.contains("sent") {
        return SpecialFolder::Sent;
    }
    if last_segment.contains("draft") {
        return SpecialFolder::Drafts;
    }
    if last_segment.contains("trash") || last_segment.contains("deleted") {
        return SpecialFolder::Trash;
    }
    if last_segment.contains("archive") {
        return SpecialFolder::Archive;
    }
    if last_segment.contains("junk") || last_segment.contains("spam") {
        return SpecialFolder::Trash;
    }

    SpecialFolder::Other
}

/// Extracts the last segment from a folder path, splitting on common
/// hierarchy delimiters ("/" and ".").
fn last_path_segment(path: &str) -> &str {
    let after_slash = path.rsplit_once('/').map_or(path, |(_, s)| s);
    after_slash.rsplit_once('.').map_or(after_slash, |(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_is_detected_from_name() {
        let special = map_special_use("INBOX", &[]);
        assert_eq!(special, SpecialFolder::Inbox);
    }

    #[test]
    fn sent_is_detected_from_attribute() {
        let special = map_special_use("Whatever", &[NameAttribute::Sent]);
        assert_eq!(special, SpecialFolder::Sent);
    }

    #[test]
    fn drafts_is_detected_from_attribute() {
        let special = map_special_use("Whatever", &[NameAttribute::Drafts]);
        assert_eq!(special, SpecialFolder::Drafts);
    }

    #[test]
    fn trash_is_detected_from_attribute() {
        let special = map_special_use("Whatever", &[NameAttribute::Trash]);
        assert_eq!(special, SpecialFolder::Trash);
    }

    #[test]
    fn junk_attribute_maps_to_trash() {
        let special = map_special_use("Whatever", &[NameAttribute::Junk]);
        assert_eq!(special, SpecialFolder::Trash);
    }

    #[test]
    fn archive_is_detected_from_attribute() {
        let special = map_special_use("Whatever", &[NameAttribute::Archive]);
        assert_eq!(special, SpecialFolder::Archive);
    }

    #[test]
    fn all_attribute_maps_to_archive() {
        let special = map_special_use("Whatever", &[NameAttribute::All]);
        assert_eq!(special, SpecialFolder::Archive);
    }

    #[test]
    fn sent_is_detected_from_name_heuristic() {
        let special = map_special_use("Sent Items", &[]);
        assert_eq!(special, SpecialFolder::Sent);
    }

    #[test]
    fn trash_is_detected_from_deleted_items_name() {
        let special = map_special_use("Deleted Items", &[]);
        assert_eq!(special, SpecialFolder::Trash);
    }

    #[test]
    fn nested_folder_uses_last_segment() {
        let special = map_special_use("[Gmail]/Sent Mail", &[]);
        assert_eq!(special, SpecialFolder::Sent);
    }

    #[test]
    fn unknown_folder_maps_to_other() {
        let special = map_special_use("Work Projects", &[]);
        assert_eq!(special, SpecialFolder::Other);
    }

    #[test]
    fn junk_name_maps_to_trash() {
        let special = map_special_use("Junk", &[]);
        assert_eq!(special, SpecialFolder::Trash);
    }

    #[test]
    fn spam_name_maps_to_trash() {
        let special = map_special_use("Spam", &[]);
        assert_eq!(special, SpecialFolder::Trash);
    }

    #[test]
    fn dotted_path_uses_last_segment() {
        let special = map_special_use("INBOX.Drafts", &[]);
        assert_eq!(special, SpecialFolder::Drafts);
    }

    #[test]
    fn attribute_takes_precedence_over_name() {
        // A folder named "Archive" but with \Sent attribute should be Sent.
        let special = map_special_use("Archive", &[NameAttribute::Sent]);
        assert_eq!(special, SpecialFolder::Sent);
    }

    #[test]
    fn noselect_is_detected() {
        assert!(is_noselect(&[NameAttribute::NoSelect]));
    }

    #[test]
    fn selectable_folder_is_not_noselect() {
        assert!(!is_noselect(&[NameAttribute::Marked]));
    }

    #[test]
    fn empty_attributes_is_not_noselect() {
        assert!(!is_noselect(&[]));
    }

    #[test]
    fn last_path_segment_returns_full_string_with_no_delimiter() {
        assert_eq!(last_path_segment("INBOX"), "INBOX");
    }

    #[test]
    fn last_path_segment_splits_on_slash() {
        assert_eq!(last_path_segment("Work/Projects/Active"), "Active");
    }

    #[test]
    fn last_path_segment_splits_on_dot() {
        assert_eq!(last_path_segment("INBOX.subfolder"), "subfolder");
    }

    #[test]
    fn last_path_segment_handles_both_slash_and_dot() {
        assert_eq!(last_path_segment("[Gmail]/All.Mail"), "Mail");
    }

    #[test]
    fn last_path_segment_handles_empty_string() {
        assert_eq!(last_path_segment(""), "");
    }

    #[test]
    fn inbox_detected_case_insensitively() {
        assert_eq!(map_special_use("inbox", &[]), SpecialFolder::Inbox);
        assert_eq!(map_special_use("Inbox", &[]), SpecialFolder::Inbox);
        assert_eq!(map_special_use("INBOX", &[]), SpecialFolder::Inbox);
    }

    #[test]
    fn archive_detected_from_name_heuristic() {
        assert_eq!(map_special_use("Archive", &[]), SpecialFolder::Archive);
    }

    #[test]
    fn draft_detected_from_name_heuristic() {
        // "Draft" (singular, no 's') should also match the contains("draft") check.
        assert_eq!(map_special_use("Draft", &[]), SpecialFolder::Drafts);
    }

    #[test]
    fn multiple_attributes_first_match_wins() {
        // If a folder has both \Sent and \Drafts (unusual but possible),
        // the first match in the loop should win.
        let special = map_special_use("Ambiguous", &[NameAttribute::Sent, NameAttribute::Drafts]);
        assert_eq!(special, SpecialFolder::Sent);
    }
}
