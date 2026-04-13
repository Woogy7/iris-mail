use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use iris_core::{Attachment, AttachmentId, MessageId};

use crate::{Error, Result};

/// Repository for [`Attachment`] persistence.
///
/// Attachments are deduplicated by SHA-256 hash. The `message_attachments`
/// join table links attachments to the messages they appear in.
pub struct AttachmentRepo;

impl AttachmentRepo {
    /// Inserts an attachment, deduplicating by SHA-256 hash.
    ///
    /// If an attachment with the same SHA-256 already exists, this is a no-op
    /// (the existing row is kept unchanged via `ON CONFLICT DO NOTHING`).
    pub async fn insert(pool: &SqlitePool, attachment: &Attachment) -> Result<()> {
        let id = attachment.id.0.to_string();
        let size_bytes = attachment.size_bytes as i64;

        sqlx::query(
            "INSERT INTO attachments (id, sha256, size_bytes, mime_type) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (sha256) DO NOTHING",
        )
        .bind(&id)
        .bind(&attachment.sha256)
        .bind(size_bytes)
        .bind(&attachment.mime_type)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieves an attachment by its identifier.
    pub async fn get_by_id(pool: &SqlitePool, id: &AttachmentId) -> Result<Attachment> {
        let id_str = id.0.to_string();

        let row = sqlx::query("SELECT * FROM attachments WHERE id = ?1")
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| Error::NotFound {
                entity: "attachment",
                id: id_str,
            })?;

        attachment_from_row(&row)
    }

    /// Retrieves an attachment by its SHA-256 hash.
    pub async fn get_by_sha256(pool: &SqlitePool, sha256: &str) -> Result<Attachment> {
        let row = sqlx::query("SELECT * FROM attachments WHERE sha256 = ?1")
            .bind(sha256)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| Error::NotFound {
                entity: "attachment",
                id: sha256.to_owned(),
            })?;

        attachment_from_row(&row)
    }

    /// Creates a link between a message and an attachment in the join table.
    pub async fn link_to_message(
        pool: &SqlitePool,
        message_id: &MessageId,
        attachment_id: &AttachmentId,
        filename: Option<&str>,
        mime_type: &str,
    ) -> Result<()> {
        let msg_id = message_id.0.to_string();
        let att_id = attachment_id.0.to_string();

        sqlx::query(
            "INSERT INTO message_attachments (message_id, attachment_id, filename, mime_type) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&msg_id)
        .bind(&att_id)
        .bind(filename)
        .bind(mime_type)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Lists all attachments linked to a message.
    ///
    /// Returns the attachment metadata along with per-message filename from the
    /// join table.
    pub async fn list_by_message(
        pool: &SqlitePool,
        message_id: &MessageId,
    ) -> Result<Vec<Attachment>> {
        let msg_id = message_id.0.to_string();

        let rows = sqlx::query(
            "SELECT a.id, a.sha256, a.size_bytes, a.mime_type, ma.filename \
             FROM attachments a \
             INNER JOIN message_attachments ma ON a.id = ma.attachment_id \
             WHERE ma.message_id = ?1",
        )
        .bind(&msg_id)
        .fetch_all(pool)
        .await?;

        rows.iter().map(attachment_with_filename_from_row).collect()
    }
}

/// Constructs an [`Attachment`] from a row that has the standard attachments columns.
fn attachment_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Attachment> {
    let id_str: String = row.get("id");
    let size_bytes: i64 = row.get("size_bytes");

    let id = Uuid::parse_str(&id_str).map_err(|e| {
        Error::Sqlx(sqlx::Error::Decode(
            format!("invalid attachment id: {e}").into(),
        ))
    })?;

    Ok(Attachment {
        id: AttachmentId(id),
        sha256: row.get("sha256"),
        size_bytes: size_bytes as u64,
        mime_type: row.get("mime_type"),
        filename: None,
    })
}

/// Constructs an [`Attachment`] from a joined row that includes a `filename` column.
fn attachment_with_filename_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Attachment> {
    let id_str: String = row.get("id");
    let size_bytes: i64 = row.get("size_bytes");

    let id = Uuid::parse_str(&id_str).map_err(|e| {
        Error::Sqlx(sqlx::Error::Decode(
            format!("invalid attachment id: {e}").into(),
        ))
    })?;

    Ok(Attachment {
        id: AttachmentId(id),
        sha256: row.get("sha256"),
        size_bytes: size_bytes as u64,
        mime_type: row.get("mime_type"),
        filename: row.get("filename"),
    })
}
