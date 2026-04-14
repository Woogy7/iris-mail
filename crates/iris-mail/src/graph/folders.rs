//! Folder discovery via the Microsoft Graph API.
//!
//! Lists all mail folders for the authenticated user and maps them to
//! [`DiscoveredFolder`] values, the same type used by IMAP folder discovery.

use iris_core::SpecialFolder;

use crate::graph::client::GraphClient;
use crate::graph::types::{GraphFolder, GraphResponse};
use crate::imap::folders::DiscoveredFolder;

/// Lists all mail folders for the authenticated M365 user.
///
/// Returns [`DiscoveredFolder`] values where `full_path` is the opaque Graph
/// folder ID (used in subsequent API calls to list messages).
pub async fn list_graph_folders(client: &GraphClient) -> crate::Result<Vec<DiscoveredFolder>> {
    let resp = client.get("/me/mailFolders?$top=100").await?;
    let data: GraphResponse<GraphFolder> = resp
        .json()
        .await
        .map_err(|e| crate::Error::Graph(format!("failed to parse folder response: {e}")))?;

    let mut folders = Vec::new();
    for gf in data.value {
        let special = map_well_known_name(gf.well_known_name.as_deref(), &gf.display_name);
        folders.push(DiscoveredFolder {
            full_path: gf.id.clone(),
            name: gf.display_name,
            delimiter: None,
            special,
        });
    }

    tracing::info!("Graph: discovered {} folders", folders.len());
    Ok(folders)
}

/// Maps a Graph API `wellKnownName` to a [`SpecialFolder`] variant.
///
/// Falls back to a name-based heuristic when the well-known name is absent.
fn map_well_known_name(well_known: Option<&str>, display_name: &str) -> SpecialFolder {
    match well_known {
        Some("inbox") => SpecialFolder::Inbox,
        Some("sentitems" | "sentItems") => SpecialFolder::Sent,
        Some("drafts") => SpecialFolder::Drafts,
        Some("deleteditems" | "deletedItems") => SpecialFolder::Trash,
        Some("archive") => SpecialFolder::Archive,
        Some("junkemail" | "junkEmail") => SpecialFolder::Trash,
        _ => {
            // Fallback to name-based heuristic.
            let lower = display_name.to_lowercase();
            if lower == "inbox" {
                SpecialFolder::Inbox
            } else if lower.contains("sent") {
                SpecialFolder::Sent
            } else if lower.contains("draft") {
                SpecialFolder::Drafts
            } else if lower.contains("trash") || lower.contains("deleted") {
                SpecialFolder::Trash
            } else if lower.contains("archive") {
                SpecialFolder::Archive
            } else if lower.contains("junk") || lower.contains("spam") {
                SpecialFolder::Trash
            } else {
                SpecialFolder::Other
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_mapped_from_well_known_name() {
        assert_eq!(
            map_well_known_name(Some("inbox"), "Inbox"),
            SpecialFolder::Inbox
        );
    }

    #[test]
    fn sent_items_mapped_from_well_known_name() {
        assert_eq!(
            map_well_known_name(Some("sentitems"), "Sent Items"),
            SpecialFolder::Sent
        );
    }

    #[test]
    fn deleted_items_mapped_to_trash() {
        assert_eq!(
            map_well_known_name(Some("deleteditems"), "Deleted Items"),
            SpecialFolder::Trash
        );
    }

    #[test]
    fn junk_email_mapped_to_trash() {
        assert_eq!(
            map_well_known_name(Some("junkemail"), "Junk Email"),
            SpecialFolder::Trash
        );
    }

    #[test]
    fn unknown_folder_falls_back_to_name_heuristic() {
        assert_eq!(
            map_well_known_name(None, "Old Archive"),
            SpecialFolder::Archive
        );
    }

    #[test]
    fn completely_unknown_folder_maps_to_other() {
        assert_eq!(map_well_known_name(None, "Projects"), SpecialFolder::Other);
    }

    #[test]
    fn drafts_mapped_from_well_known_name() {
        assert_eq!(
            map_well_known_name(Some("drafts"), "Drafts"),
            SpecialFolder::Drafts
        );
    }

    #[test]
    fn archive_mapped_from_well_known_name() {
        assert_eq!(
            map_well_known_name(Some("archive"), "Archive"),
            SpecialFolder::Archive
        );
    }

    #[test]
    fn camel_case_well_known_names_are_handled() {
        assert_eq!(
            map_well_known_name(Some("sentItems"), "Sent Items"),
            SpecialFolder::Sent
        );
        assert_eq!(
            map_well_known_name(Some("deletedItems"), "Deleted Items"),
            SpecialFolder::Trash
        );
        assert_eq!(
            map_well_known_name(Some("junkEmail"), "Junk Email"),
            SpecialFolder::Trash
        );
    }

    #[test]
    fn name_heuristic_detects_spam_as_trash() {
        assert_eq!(map_well_known_name(None, "Spam"), SpecialFolder::Trash);
    }

    #[test]
    fn name_heuristic_detects_inbox_case_insensitively() {
        assert_eq!(map_well_known_name(None, "INBOX"), SpecialFolder::Inbox);
    }
}
