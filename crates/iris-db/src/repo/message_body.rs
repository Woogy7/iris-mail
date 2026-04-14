//! Repository for message body persistence.

use iris_core::{MessageBody, MessageId};
use sqlx::{Row, SqlitePool};

use crate::{Error, Result};

/// Repository for [`MessageBody`] persistence.
///
/// Handles HTML sanitisation and plain text derivation automatically
/// when those fields are not provided by the caller.
pub struct MessageBodyRepo;

impl MessageBodyRepo {
    /// Insert or replace a message body.
    ///
    /// If `html` is provided but `sanitised_html` is `None`, sanitisation is
    /// performed automatically via `ammonia`. If `plain_text` is `None` but
    /// `html` is present, plain text is derived via `html2text`.
    pub async fn upsert(pool: &SqlitePool, body: &MessageBody) -> Result<()> {
        let sanitised = body
            .sanitised_html
            .clone()
            .or_else(|| body.html.as_ref().map(|h| ammonia::clean(h)));

        let plain = body.plain_text.clone().or_else(|| {
            body.html
                .as_ref()
                .and_then(|h| html2text::from_read(h.as_bytes(), 80).ok())
        });

        let msg_id = body.message_id.0.to_string();

        sqlx::query(
            "INSERT OR REPLACE INTO message_bodies \
             (message_id, html, sanitised_html, plain_text) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&msg_id)
        .bind(body.html.as_deref())
        .bind(sanitised.as_deref())
        .bind(plain.as_deref())
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieve the body for a message, if it exists.
    pub async fn get_by_message_id(
        pool: &SqlitePool,
        message_id: &MessageId,
    ) -> Result<Option<MessageBody>> {
        let id_str = message_id.0.to_string();

        let row = sqlx::query("SELECT * FROM message_bodies WHERE message_id = ?1")
            .bind(&id_str)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => {
                let msg_id_str: String = row.get("message_id");
                let msg_id = uuid::Uuid::parse_str(&msg_id_str).map_err(|e| {
                    Error::Sqlx(sqlx::Error::Decode(
                        format!("invalid message_id: {e}").into(),
                    ))
                })?;

                Ok(Some(MessageBody {
                    message_id: MessageId(msg_id),
                    html: row.get("html"),
                    sanitised_html: row.get("sanitised_html"),
                    plain_text: row.get("plain_text"),
                }))
            }
            None => Ok(None),
        }
    }
}
