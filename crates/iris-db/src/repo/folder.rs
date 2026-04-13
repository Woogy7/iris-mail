use chrono::DateTime;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use iris_core::{AccountId, Folder, FolderId, SpecialFolder};

use crate::{Error, Result};

/// Repository for [`Folder`] persistence.
pub struct FolderRepo;

impl FolderRepo {
    /// Inserts a new folder into the database.
    pub async fn insert(pool: &SqlitePool, folder: &Folder) -> Result<()> {
        let id = folder.id.0.to_string();
        let account_id = folder.account_id.0.to_string();
        let parent_id = folder.parent_id.map(|p| p.0.to_string());
        let special = special_folder_to_str(folder.special);
        let uid_validity = folder.uid_validity.map(|v| v as i64);
        let last_seen_uid = folder.last_seen_uid.map(|v| v as i64);
        let last_synced_at = folder.last_synced_at.map(|dt| dt.to_rfc3339());
        let message_count = folder.message_count as i64;
        let unread_count = folder.unread_count as i64;
        let created_at = folder.created_at.to_rfc3339();
        let updated_at = folder.updated_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO folders (id, account_id, name, full_path, parent_id, special, \
             uid_validity, last_seen_uid, last_synced_at, message_count, unread_count, \
             created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )
        .bind(&id)
        .bind(&account_id)
        .bind(&folder.name)
        .bind(&folder.full_path)
        .bind(&parent_id)
        .bind(special)
        .bind(uid_validity)
        .bind(last_seen_uid)
        .bind(&last_synced_at)
        .bind(message_count)
        .bind(unread_count)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieves a folder by its identifier.
    pub async fn get_by_id(pool: &SqlitePool, id: &FolderId) -> Result<Folder> {
        let id_str = id.0.to_string();

        let row = sqlx::query("SELECT * FROM folders WHERE id = ?1")
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| Error::NotFound {
                entity: "folder",
                id: id_str,
            })?;

        folder_from_row(&row)
    }

    /// Lists all folders for an account, ordered by special folder priority then name.
    ///
    /// The ordering ensures Inbox appears first, followed by Sent, Drafts, Trash,
    /// Archive, and then user-created folders alphabetically.
    pub async fn list_by_account(pool: &SqlitePool, account_id: &AccountId) -> Result<Vec<Folder>> {
        let account_id_str = account_id.0.to_string();

        let rows = sqlx::query(
            "SELECT * FROM folders WHERE account_id = ?1 \
             ORDER BY CASE special \
               WHEN 'inbox'   THEN 0 \
               WHEN 'sent'    THEN 1 \
               WHEN 'drafts'  THEN 2 \
               WHEN 'trash'   THEN 3 \
               WHEN 'archive' THEN 4 \
               ELSE 5 \
             END, name",
        )
        .bind(&account_id_str)
        .fetch_all(pool)
        .await?;

        rows.iter().map(folder_from_row).collect()
    }

    /// Updates an existing folder.
    pub async fn update(pool: &SqlitePool, folder: &Folder) -> Result<()> {
        let id = folder.id.0.to_string();
        let account_id = folder.account_id.0.to_string();
        let parent_id = folder.parent_id.map(|p| p.0.to_string());
        let special = special_folder_to_str(folder.special);
        let uid_validity = folder.uid_validity.map(|v| v as i64);
        let last_seen_uid = folder.last_seen_uid.map(|v| v as i64);
        let last_synced_at = folder.last_synced_at.map(|dt| dt.to_rfc3339());
        let message_count = folder.message_count as i64;
        let unread_count = folder.unread_count as i64;
        let updated_at = folder.updated_at.to_rfc3339();

        let result = sqlx::query(
            "UPDATE folders SET account_id = ?1, name = ?2, full_path = ?3, parent_id = ?4, \
             special = ?5, uid_validity = ?6, last_seen_uid = ?7, last_synced_at = ?8, \
             message_count = ?9, unread_count = ?10, updated_at = ?11 \
             WHERE id = ?12",
        )
        .bind(&account_id)
        .bind(&folder.name)
        .bind(&folder.full_path)
        .bind(&parent_id)
        .bind(special)
        .bind(uid_validity)
        .bind(last_seen_uid)
        .bind(&last_synced_at)
        .bind(message_count)
        .bind(unread_count)
        .bind(&updated_at)
        .bind(&id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "folder",
                id,
            });
        }

        Ok(())
    }

    /// Deletes a folder by its identifier.
    pub async fn delete(pool: &SqlitePool, id: &FolderId) -> Result<()> {
        let id_str = id.0.to_string();

        let result = sqlx::query("DELETE FROM folders WHERE id = ?1")
            .bind(&id_str)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "folder",
                id: id_str,
            });
        }

        Ok(())
    }
}

/// Maps a [`SpecialFolder`] variant to its database string representation.
fn special_folder_to_str(special: SpecialFolder) -> &'static str {
    match special {
        SpecialFolder::Inbox => "inbox",
        SpecialFolder::Sent => "sent",
        SpecialFolder::Drafts => "drafts",
        SpecialFolder::Trash => "trash",
        SpecialFolder::Archive => "archive",
        SpecialFolder::Other => "other",
    }
}

/// Parses a database string into a [`SpecialFolder`] variant.
fn special_folder_from_str(s: &str) -> Result<SpecialFolder> {
    match s {
        "inbox" => Ok(SpecialFolder::Inbox),
        "sent" => Ok(SpecialFolder::Sent),
        "drafts" => Ok(SpecialFolder::Drafts),
        "trash" => Ok(SpecialFolder::Trash),
        "archive" => Ok(SpecialFolder::Archive),
        "other" => Ok(SpecialFolder::Other),
        other => Err(Error::Sqlx(sqlx::Error::Decode(
            format!("unknown special folder: {other}").into(),
        ))),
    }
}

/// Constructs a [`Folder`] from a SQLite row.
fn folder_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Folder> {
    let id_str: String = row.get("id");
    let account_id_str: String = row.get("account_id");
    let parent_id_str: Option<String> = row.get("parent_id");
    let special_str: String = row.get("special");
    let uid_validity: Option<i64> = row.get("uid_validity");
    let last_seen_uid: Option<i64> = row.get("last_seen_uid");
    let last_synced_str: Option<String> = row.get("last_synced_at");
    let message_count: i64 = row.get("message_count");
    let unread_count: i64 = row.get("unread_count");
    let created_str: String = row.get("created_at");
    let updated_str: String = row.get("updated_at");

    let id = parse_uuid(&id_str, "folder id")?;
    let account_id = parse_uuid(&account_id_str, "folder account_id")?;
    let parent_id = parent_id_str
        .map(|s| parse_uuid(&s, "folder parent_id"))
        .transpose()?
        .map(FolderId);

    Ok(Folder {
        id: FolderId(id),
        account_id: AccountId(account_id),
        parent_id,
        name: row.get("name"),
        full_path: row.get("full_path"),
        special: special_folder_from_str(&special_str)?,
        uid_validity: uid_validity.map(|v| v as u32),
        last_seen_uid: last_seen_uid.map(|v| v as u32),
        message_count: message_count as u32,
        unread_count: unread_count as u32,
        last_synced_at: parse_optional_datetime(&last_synced_str)?,
        created_at: parse_datetime(&created_str, "folder created_at")?,
        updated_at: parse_datetime(&updated_str, "folder updated_at")?,
    })
}

/// Parses a UUID string, returning a decode error on failure.
fn parse_uuid(s: &str, field: &str) -> Result<Uuid> {
    Uuid::parse_str(s)
        .map_err(|e| Error::Sqlx(sqlx::Error::Decode(format!("invalid {field}: {e}").into())))
}

/// Parses an RFC 3339 datetime string to `DateTime<Utc>`.
fn parse_datetime(s: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.to_utc())
        .map_err(|e| Error::Sqlx(sqlx::Error::Decode(format!("invalid {field}: {e}").into())))
}

/// Parses an optional RFC 3339 datetime string.
fn parse_optional_datetime(s: &Option<String>) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
    match s {
        Some(s) => Ok(Some(parse_datetime(s, "datetime")?)),
        None => Ok(None),
    }
}
