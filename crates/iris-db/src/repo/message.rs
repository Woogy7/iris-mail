use chrono::DateTime;
use iris_core::{AccountId, FolderId, Message, MessageFlags, MessageId};
use sqlx::{Row, SqlitePool};

use crate::{Error, Result};

/// Repository for [`Message`] persistence.
pub struct MessageRepo;

impl MessageRepo {
    /// Inserts a new message into the database.
    pub async fn insert(pool: &SqlitePool, message: &Message) -> Result<()> {
        let id = message.id.0.to_string();
        let folder_id = message.folder_id.0.to_string();
        let account_id = message.account_id.0.to_string();
        let date = message.date.map(|d| d.to_rfc3339());
        let size_bytes = message.size_bytes.map(|s| s as i64);
        let imap_uid = message.uid.map(|u| u as i64);
        let created_at = message.created_at.to_rfc3339();
        let updated_at = message.updated_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO messages (id, folder_id, account_id, subject, from_name, \
             from_address, to_addresses, cc_addresses, bcc_addresses, date, size_bytes, \
             is_read, is_flagged, is_answered, thread_id, message_id_header, imap_uid, \
             stored_local, stored_remote, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, \
             ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        )
        .bind(&id)
        .bind(&folder_id)
        .bind(&account_id)
        .bind(message.subject.as_deref().unwrap_or(""))
        .bind(message.from_name.as_deref())
        .bind(message.from_address.as_deref())
        .bind(message.to_addresses.as_deref().unwrap_or("[]"))
        .bind(message.cc_addresses.as_deref().unwrap_or("[]"))
        .bind(message.bcc_addresses.as_deref().unwrap_or("[]"))
        .bind(&date)
        .bind(size_bytes)
        .bind(message.flags.is_read)
        .bind(message.flags.is_flagged)
        .bind(message.flags.is_answered)
        .bind(message.thread_id.as_deref())
        .bind(message.message_id_header.as_deref())
        .bind(imap_uid)
        .bind(message.is_stored_local)
        .bind(message.is_stored_remote)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieves a message by its identifier.
    pub async fn get_by_id(pool: &SqlitePool, id: &MessageId) -> Result<Message> {
        let id_str = id.0.to_string();

        let row = sqlx::query("SELECT * FROM messages WHERE id = ?1")
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| Error::NotFound {
                entity: "message",
                id: id_str,
            })?;

        message_from_row(&row)
    }

    /// Lists messages in a folder, ordered by date descending with pagination.
    pub async fn list_by_folder(
        pool: &SqlitePool,
        folder_id: &FolderId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>> {
        let folder_id_str = folder_id.0.to_string();

        let rows = sqlx::query(
            "SELECT * FROM messages WHERE folder_id = ?1 ORDER BY date DESC LIMIT ?2 OFFSET ?3",
        )
        .bind(&folder_id_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        rows.iter().map(message_from_row).collect()
    }

    /// Updates only the boolean flags on a message (read, flagged, answered).
    pub async fn update_flags(
        pool: &SqlitePool,
        id: &MessageId,
        flags: &MessageFlags,
    ) -> Result<()> {
        let id_str = id.0.to_string();

        let result = sqlx::query(
            "UPDATE messages SET is_read = ?1, is_flagged = ?2, is_answered = ?3 WHERE id = ?4",
        )
        .bind(flags.is_read)
        .bind(flags.is_flagged)
        .bind(flags.is_answered)
        .bind(&id_str)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "message",
                id: id_str,
            });
        }

        Ok(())
    }

    /// Insert multiple messages in a single transaction.
    ///
    /// Skips messages whose primary key already exists. Returns the number
    /// of rows actually inserted.
    pub async fn insert_batch(pool: &SqlitePool, messages: &[Message]) -> Result<u64> {
        let mut tx = pool.begin().await?;
        let mut count = 0u64;

        for message in messages {
            let id = message.id.0.to_string();
            let folder_id = message.folder_id.0.to_string();
            let account_id = message.account_id.0.to_string();
            let date = message.date.map(|d| d.to_rfc3339());
            let size_bytes = message.size_bytes.map(|s| s as i64);
            let imap_uid = message.uid.map(|u| u as i64);
            let created_at = message.created_at.to_rfc3339();
            let updated_at = message.updated_at.to_rfc3339();

            let result = sqlx::query(
                "INSERT OR IGNORE INTO messages \
                 (id, folder_id, account_id, subject, from_name, \
                 from_address, to_addresses, cc_addresses, bcc_addresses, \
                 date, size_bytes, is_read, is_flagged, is_answered, \
                 thread_id, message_id_header, imap_uid, stored_local, \
                 stored_remote, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, \
                 ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            )
            .bind(&id)
            .bind(&folder_id)
            .bind(&account_id)
            .bind(message.subject.as_deref().unwrap_or(""))
            .bind(message.from_name.as_deref())
            .bind(message.from_address.as_deref())
            .bind(message.to_addresses.as_deref().unwrap_or("[]"))
            .bind(message.cc_addresses.as_deref().unwrap_or("[]"))
            .bind(message.bcc_addresses.as_deref().unwrap_or("[]"))
            .bind(&date)
            .bind(size_bytes)
            .bind(message.flags.is_read)
            .bind(message.flags.is_flagged)
            .bind(message.flags.is_answered)
            .bind(message.thread_id.as_deref())
            .bind(message.message_id_header.as_deref())
            .bind(imap_uid)
            .bind(message.is_stored_local)
            .bind(message.is_stored_remote)
            .bind(&created_at)
            .bind(&updated_at)
            .execute(&mut *tx)
            .await?;

            count += result.rows_affected();
        }

        tx.commit().await?;
        Ok(count)
    }

    /// Look up a message by its IMAP UID within a folder.
    ///
    /// Returns `None` if no message with that UID exists in the folder.
    pub async fn get_by_uid(
        pool: &SqlitePool,
        folder_id: &FolderId,
        uid: u32,
    ) -> Result<Option<Message>> {
        let folder_id_str = folder_id.0.to_string();

        let row = sqlx::query(
            "SELECT * FROM messages \
             WHERE folder_id = ?1 AND imap_uid = ?2",
        )
        .bind(&folder_id_str)
        .bind(uid as i64)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(message_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// Count total and unread messages in a folder.
    ///
    /// Returns a `(total, unread)` tuple.
    pub async fn count_by_folder(pool: &SqlitePool, folder_id: &FolderId) -> Result<(u32, u32)> {
        let folder_id_str = folder_id.0.to_string();

        let row = sqlx::query(
            "SELECT COUNT(*) AS total, \
             SUM(CASE WHEN is_read = 0 THEN 1 ELSE 0 END) AS unread \
             FROM messages WHERE folder_id = ?1",
        )
        .bind(&folder_id_str)
        .fetch_one(pool)
        .await?;

        let total: i64 = row.get("total");
        let unread: Option<i64> = row.get("unread");
        Ok((total as u32, unread.unwrap_or(0) as u32))
    }

    /// Deletes a message by its identifier.
    pub async fn delete(pool: &SqlitePool, id: &MessageId) -> Result<()> {
        let id_str = id.0.to_string();

        let result = sqlx::query("DELETE FROM messages WHERE id = ?1")
            .bind(&id_str)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "message",
                id: id_str,
            });
        }

        Ok(())
    }
}

/// Constructs a [`Message`] from a SQLite row.
fn message_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Message> {
    let id_str: String = row.get("id");
    let folder_id_str: String = row.get("folder_id");
    let account_id_str: String = row.get("account_id");
    let date_str: Option<String> = row.get("date");
    let size_bytes: Option<i64> = row.get("size_bytes");
    let imap_uid: Option<i64> = row.get("imap_uid");
    let is_read: bool = row.get("is_read");
    let is_flagged: bool = row.get("is_flagged");
    let is_answered: bool = row.get("is_answered");
    let stored_local: bool = row.get("stored_local");
    let stored_remote: bool = row.get("stored_remote");
    let created_str: String = row.get("created_at");
    let updated_str: String = row.get("updated_at");

    let id = parse_uuid(&id_str, "message id")?;
    let folder_id = parse_uuid(&folder_id_str, "message folder_id")?;
    let account_id = parse_uuid(&account_id_str, "message account_id")?;

    let subject: String = row.get("subject");
    let to_addresses: String = row.get("to_addresses");
    let cc_addresses: String = row.get("cc_addresses");
    let bcc_addresses: String = row.get("bcc_addresses");

    Ok(Message {
        id: MessageId(id),
        account_id: AccountId(account_id),
        folder_id: FolderId(folder_id),
        uid: imap_uid.map(|v| v as u32),
        message_id_header: row.get("message_id_header"),
        thread_id: row.get("thread_id"),
        subject: Some(subject),
        from_name: row.get("from_name"),
        from_address: row.get("from_address"),
        to_addresses: Some(to_addresses),
        cc_addresses: Some(cc_addresses),
        bcc_addresses: Some(bcc_addresses),
        date: parse_optional_datetime(&date_str)?,
        size_bytes: size_bytes.map(|v| v as u64),
        flags: MessageFlags {
            is_read,
            is_flagged,
            is_answered,
        },
        is_stored_local: stored_local,
        is_stored_remote: stored_remote,
        created_at: parse_datetime(&created_str, "message created_at")?,
        updated_at: parse_datetime(&updated_str, "message updated_at")?,
    })
}

/// Parses a UUID string, returning a decode error on failure.
fn parse_uuid(s: &str, field: &str) -> Result<uuid::Uuid> {
    uuid::Uuid::parse_str(s)
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
