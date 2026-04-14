//! Iris DB — SQLite schema, migrations, and typed repository queries.
//!
//! This crate owns all SQL in the Iris Mail application. Other crates call
//! repository methods; they never construct queries directly. Migrations are
//! applied automatically on startup.

mod error;
pub mod repo;

pub use error::{Error, Result};
pub use repo::{AccountRepo, AttachmentRepo, FolderRepo, MessageBodyRepo, MessageRepo};

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;

/// Creates a connection pool to the SQLite database and runs all pending migrations.
///
/// The pool is configured with WAL journal mode for concurrent readers and
/// foreign keys enabled for referential integrity.
pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use iris_core::{
        AccentColour, Account, AccountId, Attachment, AttachmentId, Folder, FolderId, Message,
        MessageBody, MessageFlags, MessageId, Provider, SpecialFolder, SyncPreferences,
    };

    use super::*;

    /// Helper to create a fresh in-memory pool for each test.
    async fn test_pool() -> SqlitePool {
        create_pool("sqlite::memory:")
            .await
            .expect("failed to create test pool")
    }

    /// Helper to build a minimal test account.
    fn make_account() -> Account {
        let now = Utc::now();
        Account {
            id: AccountId::new(),
            display_name: "Test Account".to_owned(),
            email_address: "test@example.com".to_owned(),
            provider: Provider::Gmail,
            keychain_ref: Uuid::new_v4(),
            sync_preferences: SyncPreferences::default(),
            accent_colour: AccentColour::Sapphire,
            created_at: now,
            updated_at: now,
        }
    }

    /// Helper to build a minimal test folder belonging to the given account.
    fn make_folder(account_id: AccountId) -> Folder {
        let now = Utc::now();
        Folder {
            id: FolderId::new(),
            account_id,
            parent_id: None,
            name: "Inbox".to_owned(),
            full_path: "INBOX".to_owned(),
            special: SpecialFolder::Inbox,
            uid_validity: Some(12345),
            last_seen_uid: None,
            message_count: 0,
            unread_count: 0,
            last_synced_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Helper to build a minimal test message belonging to the given folder.
    fn make_message(account_id: AccountId, folder_id: FolderId) -> Message {
        let now = Utc::now();
        Message {
            id: MessageId::new(),
            account_id,
            folder_id,
            uid: Some(42),
            message_id_header: Some("<abc@example.com>".to_owned()),
            thread_id: None,
            subject: Some("Hello World".to_owned()),
            from_name: Some("Alice".to_owned()),
            from_address: Some("alice@example.com".to_owned()),
            to_addresses: Some("[\"bob@example.com\"]".to_owned()),
            cc_addresses: Some("[]".to_owned()),
            bcc_addresses: Some("[]".to_owned()),
            date: Some(now),
            size_bytes: Some(1024),
            flags: MessageFlags::default(),
            is_stored_local: false,
            is_stored_remote: true,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn pool_creates_and_migrations_run() {
        let pool = test_pool().await;

        // Verify the accounts table exists by running a simple query.
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool)
            .await
            .expect("accounts table should exist");

        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn account_crud_round_trip() {
        let pool = test_pool().await;
        let account = make_account();

        // Insert
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert should succeed");

        // Get by id
        let fetched = AccountRepo::get_by_id(&pool, &account.id)
            .await
            .expect("get_by_id should succeed");

        assert_eq!(fetched.id, account.id);
        assert_eq!(fetched.display_name, account.display_name);
        assert_eq!(fetched.email_address, account.email_address);
        assert_eq!(fetched.provider, account.provider);
        assert_eq!(fetched.accent_colour, account.accent_colour);
        assert_eq!(
            fetched.sync_preferences.initial_sync_days,
            account.sync_preferences.initial_sync_days
        );

        // List
        let all = AccountRepo::list(&pool).await.expect("list should succeed");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, account.id);

        // Update
        let mut updated = account.clone();
        updated.display_name = "Updated Name".to_owned();
        updated.accent_colour = AccentColour::Red;
        AccountRepo::update(&pool, &updated)
            .await
            .expect("update should succeed");

        let fetched2 = AccountRepo::get_by_id(&pool, &updated.id)
            .await
            .expect("get after update should succeed");
        assert_eq!(fetched2.display_name, "Updated Name");
        assert_eq!(fetched2.accent_colour, AccentColour::Red);

        // Delete
        AccountRepo::delete(&pool, &account.id)
            .await
            .expect("delete should succeed");

        let all_after = AccountRepo::list(&pool)
            .await
            .expect("list after delete should succeed");
        assert!(all_after.is_empty());
    }

    #[tokio::test]
    async fn folder_crud_round_trip_with_account() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);

        // Insert
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        // Get by id
        let fetched = FolderRepo::get_by_id(&pool, &folder.id)
            .await
            .expect("get folder");
        assert_eq!(fetched.id, folder.id);
        assert_eq!(fetched.name, "Inbox");
        assert_eq!(fetched.special, SpecialFolder::Inbox);
        assert_eq!(fetched.uid_validity, Some(12345));

        // List by account
        let folders = FolderRepo::list_by_account(&pool, &account.id)
            .await
            .expect("list folders");
        assert_eq!(folders.len(), 1);

        // Update
        let mut updated = folder.clone();
        updated.message_count = 42;
        updated.unread_count = 5;
        FolderRepo::update(&pool, &updated)
            .await
            .expect("update folder");

        let fetched2 = FolderRepo::get_by_id(&pool, &folder.id)
            .await
            .expect("get after update");
        assert_eq!(fetched2.message_count, 42);
        assert_eq!(fetched2.unread_count, 5);

        // Delete
        FolderRepo::delete(&pool, &folder.id)
            .await
            .expect("delete folder");

        let folders_after = FolderRepo::list_by_account(&pool, &account.id)
            .await
            .expect("list after delete");
        assert!(folders_after.is_empty());
    }

    #[tokio::test]
    async fn message_crud_round_trip_with_folder() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);

        // Insert
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Get by id
        let fetched = MessageRepo::get_by_id(&pool, &message.id)
            .await
            .expect("get message");
        assert_eq!(fetched.id, message.id);
        assert_eq!(fetched.subject, Some("Hello World".to_owned()));
        assert_eq!(fetched.from_name, Some("Alice".to_owned()));
        assert_eq!(fetched.uid, Some(42));
        assert!(!fetched.flags.is_read);

        // List by folder
        let messages = MessageRepo::list_by_folder(&pool, &folder.id, 100, 0)
            .await
            .expect("list messages");
        assert_eq!(messages.len(), 1);

        // Delete
        MessageRepo::delete(&pool, &message.id)
            .await
            .expect("delete message");

        let messages_after = MessageRepo::list_by_folder(&pool, &folder.id, 100, 0)
            .await
            .expect("list after delete");
        assert!(messages_after.is_empty());
    }

    #[tokio::test]
    async fn attachment_insert_deduplicates_by_sha256() {
        let pool = test_pool().await;

        let attachment1 = Attachment {
            id: AttachmentId::new(),
            sha256: "abcdef1234567890".to_owned(),
            size_bytes: 2048,
            mime_type: "application/pdf".to_owned(),
            filename: None,
        };

        let attachment2 = Attachment {
            id: AttachmentId::new(),
            sha256: "abcdef1234567890".to_owned(), // same sha256
            size_bytes: 2048,
            mime_type: "application/pdf".to_owned(),
            filename: None,
        };

        AttachmentRepo::insert(&pool, &attachment1)
            .await
            .expect("insert first");
        AttachmentRepo::insert(&pool, &attachment2)
            .await
            .expect("insert duplicate");

        // Only one row should exist; the first one wins.
        let fetched = AttachmentRepo::get_by_sha256(&pool, "abcdef1234567890")
            .await
            .expect("get by sha256");
        assert_eq!(fetched.id, attachment1.id);
    }

    #[tokio::test]
    async fn message_flags_update_independently() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Initially all flags are false.
        let fetched = MessageRepo::get_by_id(&pool, &message.id)
            .await
            .expect("get message");
        assert!(!fetched.flags.is_read);
        assert!(!fetched.flags.is_flagged);
        assert!(!fetched.flags.is_answered);

        // Mark as read and flagged but not answered.
        let new_flags = MessageFlags {
            is_read: true,
            is_flagged: true,
            is_answered: false,
        };
        MessageRepo::update_flags(&pool, &message.id, &new_flags)
            .await
            .expect("update flags");

        let fetched2 = MessageRepo::get_by_id(&pool, &message.id)
            .await
            .expect("get after flag update");
        assert!(fetched2.flags.is_read);
        assert!(fetched2.flags.is_flagged);
        assert!(!fetched2.flags.is_answered);

        // The rest of the message is unchanged.
        assert_eq!(fetched2.subject, Some("Hello World".to_owned()));
        assert_eq!(fetched2.from_name, Some("Alice".to_owned()));
    }

    // --- Error path tests ---

    #[tokio::test]
    async fn get_nonexistent_account_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = AccountId::new();

        let result = AccountRepo::get_by_id(&pool, &fake_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotFound {
                    entity: "account",
                    ..
                }
            ),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn get_nonexistent_folder_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = FolderId::new();

        let result = FolderRepo::get_by_id(&pool, &fake_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotFound {
                    entity: "folder",
                    ..
                }
            ),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn get_nonexistent_message_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = MessageId::new();

        let result = MessageRepo::get_by_id(&pool, &fake_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotFound {
                    entity: "message",
                    ..
                }
            ),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn get_nonexistent_attachment_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = AttachmentId::new();

        let result = AttachmentRepo::get_by_id(&pool, &fake_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotFound {
                    entity: "attachment",
                    ..
                }
            ),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn get_attachment_by_nonexistent_sha256_returns_not_found() {
        let pool = test_pool().await;

        let result = AttachmentRepo::get_by_sha256(&pool, "nonexistent_hash").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotFound {
                    entity: "attachment",
                    ..
                }
            ),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn delete_nonexistent_account_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = AccountId::new();

        let result = AccountRepo::delete(&pool, &fake_id).await;
        assert!(matches!(
            result,
            Err(Error::NotFound {
                entity: "account",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn delete_nonexistent_folder_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = FolderId::new();

        let result = FolderRepo::delete(&pool, &fake_id).await;
        assert!(matches!(
            result,
            Err(Error::NotFound {
                entity: "folder",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn delete_nonexistent_message_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = MessageId::new();

        let result = MessageRepo::delete(&pool, &fake_id).await;
        assert!(matches!(
            result,
            Err(Error::NotFound {
                entity: "message",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn update_nonexistent_account_returns_not_found() {
        let pool = test_pool().await;
        let account = make_account();

        let result = AccountRepo::update(&pool, &account).await;
        assert!(matches!(
            result,
            Err(Error::NotFound {
                entity: "account",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn update_flags_on_nonexistent_message_returns_not_found() {
        let pool = test_pool().await;
        let fake_id = MessageId::new();
        let flags = MessageFlags::default();

        let result = MessageRepo::update_flags(&pool, &fake_id, &flags).await;
        assert!(matches!(
            result,
            Err(Error::NotFound {
                entity: "message",
                ..
            })
        ));
    }

    // --- Cascade delete tests ---

    #[tokio::test]
    async fn deleting_account_cascades_to_folders_and_messages() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Delete the account.
        AccountRepo::delete(&pool, &account.id)
            .await
            .expect("delete account");

        // Folder should be gone.
        let folder_result = FolderRepo::get_by_id(&pool, &folder.id).await;
        assert!(matches!(
            folder_result,
            Err(Error::NotFound {
                entity: "folder",
                ..
            })
        ));

        // Message should be gone.
        let message_result = MessageRepo::get_by_id(&pool, &message.id).await;
        assert!(matches!(
            message_result,
            Err(Error::NotFound {
                entity: "message",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn deleting_folder_cascades_to_messages() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Delete the folder; the account stays.
        FolderRepo::delete(&pool, &folder.id)
            .await
            .expect("delete folder");

        // Message should be gone.
        let message_result = MessageRepo::get_by_id(&pool, &message.id).await;
        assert!(matches!(
            message_result,
            Err(Error::NotFound {
                entity: "message",
                ..
            })
        ));

        // Account should still be there.
        let account_result = AccountRepo::get_by_id(&pool, &account.id).await;
        assert!(account_result.is_ok());
    }

    // --- FTS5 trigger tests ---

    #[tokio::test]
    async fn fts5_trigger_indexes_message_on_insert() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // The trigger should have created an FTS row with the subject.
        let fts_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'Hello'")
                .fetch_one(&pool)
                .await
                .expect("fts query");

        assert_eq!(fts_count.0, 1, "expected 1 FTS match for 'Hello'");
    }

    #[tokio::test]
    async fn fts5_trigger_removes_row_on_message_delete() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Confirm FTS row exists.
        let before: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'Hello'")
                .fetch_one(&pool)
                .await
                .expect("fts query before");
        assert_eq!(before.0, 1);

        // Delete the message.
        MessageRepo::delete(&pool, &message.id)
            .await
            .expect("delete message");

        // FTS row should be gone.
        let after: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'Hello'")
                .fetch_one(&pool)
                .await
                .expect("fts query after");
        assert_eq!(after.0, 0);
    }

    #[tokio::test]
    async fn fts5_body_trigger_indexes_plain_text() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Insert a message body with unique plain text.
        let msg_id = message.id.0.to_string();
        sqlx::query(
            "INSERT INTO message_bodies (message_id, html, sanitised_html, plain_text) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&msg_id)
        .bind("<p>Supercalifragilistic</p>")
        .bind("<p>Supercalifragilistic</p>")
        .bind("Supercalifragilistic")
        .execute(&pool)
        .await
        .expect("insert message body");

        // The body trigger should have re-indexed with the plain text.
        let fts_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'Supercalifragilistic'",
        )
        .fetch_one(&pool)
        .await
        .expect("fts body query");

        assert_eq!(fts_count.0, 1, "expected 1 FTS match for body text");
    }

    // --- Folder ordering test ---

    #[tokio::test]
    async fn folders_are_listed_in_special_folder_priority_order() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let now = Utc::now();
        let specials = [
            ("Trash", "TRASH", SpecialFolder::Trash),
            ("Zebra", "Zebra", SpecialFolder::Other),
            ("Inbox", "INBOX", SpecialFolder::Inbox),
            ("Drafts", "DRAFTS", SpecialFolder::Drafts),
            ("Sent", "SENT", SpecialFolder::Sent),
            ("Alpha", "Alpha", SpecialFolder::Other),
            ("Archive", "ARCHIVE", SpecialFolder::Archive),
        ];

        for (name, path, special) in &specials {
            let folder = Folder {
                id: FolderId::new(),
                account_id: account.id,
                parent_id: None,
                name: name.to_string(),
                full_path: path.to_string(),
                special: *special,
                uid_validity: None,
                last_seen_uid: None,
                message_count: 0,
                unread_count: 0,
                last_synced_at: None,
                created_at: now,
                updated_at: now,
            };
            FolderRepo::insert(&pool, &folder)
                .await
                .expect("insert folder");
        }

        let folders = FolderRepo::list_by_account(&pool, &account.id)
            .await
            .expect("list folders");

        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        // Expected order: Inbox(0), Sent(1), Drafts(2), Trash(3), Archive(4), then Other alphabetically
        assert_eq!(
            names,
            vec![
                "Inbox", "Sent", "Drafts", "Trash", "Archive", "Alpha", "Zebra"
            ]
        );
    }

    // --- Attachment link and list tests ---

    #[tokio::test]
    async fn attachment_link_to_message_and_list_by_message() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let attachment = Attachment {
            id: AttachmentId::new(),
            sha256: "unique_hash_001".to_owned(),
            size_bytes: 5000,
            mime_type: "image/jpeg".to_owned(),
            filename: None,
        };
        AttachmentRepo::insert(&pool, &attachment)
            .await
            .expect("insert attachment");

        // Link attachment to message with a filename.
        AttachmentRepo::link_to_message(
            &pool,
            &message.id,
            &attachment.id,
            Some("photo.jpg"),
            "image/jpeg",
        )
        .await
        .expect("link attachment");

        // List attachments for the message.
        let attachments = AttachmentRepo::list_by_message(&pool, &message.id)
            .await
            .expect("list attachments");

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].id, attachment.id);
        assert_eq!(attachments[0].sha256, "unique_hash_001");
        assert_eq!(attachments[0].filename.as_deref(), Some("photo.jpg"));
    }

    #[tokio::test]
    async fn list_attachments_for_message_with_no_attachments_returns_empty() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let attachments = AttachmentRepo::list_by_message(&pool, &message.id)
            .await
            .expect("list attachments");

        assert!(attachments.is_empty());
    }

    // --- Message pagination test ---

    #[tokio::test]
    async fn message_list_by_folder_respects_limit_and_offset() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        // Insert 5 messages with different dates.
        for i in 0..5 {
            let now = Utc::now();
            let mut msg = make_message(account.id, folder.id);
            msg.id = MessageId::new();
            msg.uid = Some(i + 1);
            msg.subject = Some(format!("Message {i}"));
            msg.date = Some(now + chrono::Duration::minutes(i as i64));
            msg.message_id_header = Some(format!("<msg{i}@example.com>"));
            MessageRepo::insert(&pool, &msg)
                .await
                .expect("insert message");
        }

        // List all.
        let all = MessageRepo::list_by_folder(&pool, &folder.id, 100, 0)
            .await
            .expect("list all");
        assert_eq!(all.len(), 5);

        // Limit to 2.
        let limited = MessageRepo::list_by_folder(&pool, &folder.id, 2, 0)
            .await
            .expect("list limited");
        assert_eq!(limited.len(), 2);

        // Offset 3, limit 100 should give 2 remaining.
        let offset = MessageRepo::list_by_folder(&pool, &folder.id, 100, 3)
            .await
            .expect("list offset");
        assert_eq!(offset.len(), 2);
    }

    // --- All database tables exist test ---

    #[tokio::test]
    async fn all_eight_tables_exist_after_migration() {
        let pool = test_pool().await;

        let expected_tables = [
            "accounts",
            "folders",
            "messages",
            "message_bodies",
            "attachments",
            "message_attachments",
            "messages_fts",
            "sync_jobs",
        ];

        for table in &expected_tables {
            let query = format!(
                "SELECT name FROM sqlite_master WHERE type IN ('table', 'view') AND name = '{table}'"
            );
            let row: Option<(String,)> = sqlx::query_as(&query)
                .fetch_optional(&pool)
                .await
                .unwrap_or_else(|_| panic!("query for table {table}"));
            assert!(row.is_some(), "expected table '{table}' to exist");
        }
    }

    // --- WAL mode test ---
    // Note: In-memory SQLite always returns "memory" for journal_mode,
    // so we test with a temporary file-backed database instead.

    #[tokio::test]
    async fn database_uses_wal_journal_mode() {
        let dir = std::env::temp_dir().join(format!("iris_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("test.db");
        let url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = create_pool(&url).await.expect("create file-backed pool");

        let mode: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .expect("pragma journal_mode");

        assert_eq!(mode.0, "wal", "expected WAL journal mode");

        pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Foreign keys enabled test ---

    #[tokio::test]
    async fn foreign_keys_are_enabled() {
        let pool = test_pool().await;

        let fk: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .expect("pragma foreign_keys");

        assert_eq!(fk.0, 1, "expected foreign_keys = 1 (enabled)");
    }

    // --- Multiple accounts test ---

    #[tokio::test]
    async fn list_accounts_returns_all_in_display_name_order() {
        let pool = test_pool().await;

        let mut account_a = make_account();
        account_a.display_name = "Zebra Account".to_owned();
        AccountRepo::insert(&pool, &account_a)
            .await
            .expect("insert a");

        let mut account_b = make_account();
        account_b.display_name = "Alpha Account".to_owned();
        AccountRepo::insert(&pool, &account_b)
            .await
            .expect("insert b");

        let all = AccountRepo::list(&pool).await.expect("list");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].display_name, "Alpha Account");
        assert_eq!(all[1].display_name, "Zebra Account");
    }

    // --- MessageBodyRepo tests ---

    #[tokio::test]
    async fn message_body_upsert_and_retrieve_round_trip() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some("<p>Hello, world!</p>".to_owned()),
            sanitised_html: None,
            plain_text: None,
        };

        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body");

        let fetched = fetched.expect("body should exist");
        assert_eq!(fetched.message_id, message.id);
        assert_eq!(fetched.html, Some("<p>Hello, world!</p>".to_owned()));
        // sanitised_html and plain_text should have been auto-derived.
        assert!(fetched.sanitised_html.is_some());
        assert!(fetched.plain_text.is_some());
    }

    #[tokio::test]
    async fn message_body_upsert_auto_sanitises_html() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some("<p>Safe</p><script>alert('xss')</script>".to_owned()),
            sanitised_html: None,
            plain_text: None,
        };

        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        let sanitised = fetched.sanitised_html.expect("sanitised should exist");
        assert!(
            !sanitised.contains("script"),
            "script tags should be stripped: {sanitised}"
        );
        assert!(
            sanitised.contains("Safe"),
            "safe content should remain: {sanitised}"
        );
    }

    #[tokio::test]
    async fn message_body_get_returns_none_when_missing() {
        let pool = test_pool().await;
        let fake_id = MessageId::new();

        let result = MessageBodyRepo::get_by_message_id(&pool, &fake_id)
            .await
            .expect("get should not error");

        assert!(result.is_none());
    }

    // --- MessageRepo batch / uid / count tests ---

    #[tokio::test]
    async fn message_batch_insert_creates_all_rows() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let messages: Vec<Message> = (0..5)
            .map(|i| {
                let mut msg = make_message(account.id, folder.id);
                msg.id = MessageId::new();
                msg.uid = Some(100 + i);
                msg.subject = Some(format!("Batch message {i}"));
                msg.message_id_header = Some(format!("<batch{i}@example.com>"));
                msg
            })
            .collect();

        let inserted = MessageRepo::insert_batch(&pool, &messages)
            .await
            .expect("batch insert");

        assert_eq!(inserted, 5);

        let all = MessageRepo::list_by_folder(&pool, &folder.id, 100, 0)
            .await
            .expect("list messages");

        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn message_batch_insert_skips_duplicates() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Batch-insert the same message again plus a new one.
        let mut new_msg = make_message(account.id, folder.id);
        new_msg.id = MessageId::new();
        new_msg.uid = Some(99);
        new_msg.message_id_header = Some("<new@example.com>".to_owned());

        let batch = vec![message, new_msg];
        let inserted = MessageRepo::insert_batch(&pool, &batch)
            .await
            .expect("batch insert");

        // Only the new one should have been inserted.
        assert_eq!(inserted, 1);
    }

    #[tokio::test]
    async fn get_message_by_uid_returns_correct_message() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let fetched = MessageRepo::get_by_uid(&pool, &folder.id, 42)
            .await
            .expect("get by uid");

        let fetched = fetched.expect("message should exist");
        assert_eq!(fetched.id, message.id);
        assert_eq!(fetched.uid, Some(42));
    }

    #[tokio::test]
    async fn get_message_by_uid_returns_none_when_not_found() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let result = MessageRepo::get_by_uid(&pool, &folder.id, 9999)
            .await
            .expect("get by uid should not error");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn count_by_folder_returns_correct_totals() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        // Insert 3 messages: 1 read, 2 unread.
        for i in 0..3 {
            let mut msg = make_message(account.id, folder.id);
            msg.id = MessageId::new();
            msg.uid = Some(200 + i);
            msg.message_id_header = Some(format!("<count{i}@example.com>"));
            if i == 0 {
                msg.flags.is_read = true;
            }
            MessageRepo::insert(&pool, &msg)
                .await
                .expect("insert message");
        }

        let (total, unread) = MessageRepo::count_by_folder(&pool, &folder.id)
            .await
            .expect("count by folder");

        assert_eq!(total, 3);
        assert_eq!(unread, 2);
    }

    #[tokio::test]
    async fn count_by_folder_returns_zero_for_empty_folder() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let (total, unread) = MessageRepo::count_by_folder(&pool, &folder.id)
            .await
            .expect("count by folder");

        assert_eq!(total, 0);
        assert_eq!(unread, 0);
    }

    // --- FolderRepo upsert tests ---

    #[tokio::test]
    async fn folder_upsert_inserts_new_folder() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::upsert(&pool, &folder)
            .await
            .expect("upsert folder");

        let fetched = FolderRepo::get_by_id(&pool, &folder.id)
            .await
            .expect("get folder");
        assert_eq!(fetched.name, "Inbox");
    }

    #[tokio::test]
    async fn folder_upsert_updates_existing_folder() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        // Upsert with a changed name and counts.
        let mut updated = folder.clone();
        updated.name = "Updated Inbox".to_owned();
        updated.message_count = 50;
        updated.unread_count = 10;
        FolderRepo::upsert(&pool, &updated)
            .await
            .expect("upsert folder");

        let fetched = FolderRepo::get_by_id(&pool, &folder.id)
            .await
            .expect("get folder");
        assert_eq!(fetched.name, "Updated Inbox");
        assert_eq!(fetched.message_count, 50);
        assert_eq!(fetched.unread_count, 10);

        // Should still be just one folder.
        let all = FolderRepo::list_by_account(&pool, &account.id)
            .await
            .expect("list folders");
        assert_eq!(all.len(), 1);
    }

    // --- ServerConfig tests ---

    #[tokio::test]
    async fn server_config_set_and_get_round_trip() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let config = iris_core::ServerConfig {
            imap: iris_core::ImapServer {
                host: "imap.example.com".to_owned(),
                port: 993,
                use_tls: true,
            },
            smtp: iris_core::SmtpServer {
                host: "smtp.example.com".to_owned(),
                port: 587,
                use_tls: true,
            },
        };

        AccountRepo::set_server_config(&pool, &account.id, &config)
            .await
            .expect("set server config");

        let fetched = AccountRepo::get_server_config(&pool, &account.id)
            .await
            .expect("get server config");

        assert!(fetched.is_some(), "server config should be present");
        let fetched = fetched.unwrap();
        assert_eq!(fetched.imap.host, "imap.example.com");
        assert_eq!(fetched.imap.port, 993);
        assert!(fetched.imap.use_tls);
        assert_eq!(fetched.smtp.host, "smtp.example.com");
        assert_eq!(fetched.smtp.port, 587);
        assert!(fetched.smtp.use_tls);
    }

    #[tokio::test]
    async fn server_config_returns_none_when_not_set() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let config = AccountRepo::get_server_config(&pool, &account.id)
            .await
            .expect("get server config");

        assert!(config.is_none(), "default empty config should return None");
    }

    #[tokio::test]
    async fn server_config_set_returns_not_found_for_missing_account() {
        let pool = test_pool().await;
        let fake_id = AccountId::new();
        let config = iris_core::ServerConfig {
            imap: iris_core::ImapServer {
                host: "imap.example.com".to_owned(),
                port: 993,
                use_tls: true,
            },
            smtp: iris_core::SmtpServer {
                host: "smtp.example.com".to_owned(),
                port: 587,
                use_tls: true,
            },
        };

        let result = AccountRepo::set_server_config(&pool, &fake_id, &config).await;
        assert!(matches!(
            result,
            Err(Error::NotFound {
                entity: "account",
                ..
            })
        ));
    }

    // --- Stable folder ID tests ---

    #[tokio::test]
    async fn folder_upsert_preserves_id_when_matching_by_full_path() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        // Upsert with a *different* ID but the same (account_id, full_path).
        let mut new_version = folder.clone();
        new_version.id = FolderId::new(); // different ID
        new_version.name = "Renamed Inbox".to_owned();
        new_version.message_count = 99;
        FolderRepo::upsert(&pool, &new_version)
            .await
            .expect("upsert with new id but same full_path");

        // The original ID should be preserved (ON CONFLICT keeps the existing row).
        let fetched = FolderRepo::get_by_id(&pool, &folder.id)
            .await
            .expect("get folder by original id");
        assert_eq!(fetched.id, folder.id);
        assert_eq!(fetched.name, "Renamed Inbox");
        assert_eq!(fetched.message_count, 99);

        // Only one folder should exist.
        let all = FolderRepo::list_by_account(&pool, &account.id)
            .await
            .expect("list folders");
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn folder_upsert_does_not_delete_messages() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        // Upsert the folder with updated metadata.
        let mut updated = folder.clone();
        updated.message_count = 42;
        updated.name = "Updated Inbox".to_owned();
        FolderRepo::upsert(&pool, &updated)
            .await
            .expect("upsert folder");

        // The message must still exist.
        let fetched_msg = MessageRepo::get_by_id(&pool, &message.id)
            .await
            .expect("message should survive folder upsert");
        assert_eq!(fetched_msg.id, message.id);
        assert_eq!(fetched_msg.subject, Some("Hello World".to_owned()));
    }

    #[tokio::test]
    async fn folder_get_by_account_and_full_path_returns_match() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let found = FolderRepo::get_by_account_and_full_path(&pool, &account.id, &folder.full_path)
            .await
            .expect("query should succeed");

        assert!(found.is_some(), "should find folder by full_path");
        assert_eq!(found.unwrap().id, folder.id);
    }

    #[tokio::test]
    async fn folder_get_by_account_and_full_path_returns_none_for_nonexistent() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let found =
            FolderRepo::get_by_account_and_full_path(&pool, &account.id, "NONEXISTENT/PATH")
                .await
                .expect("query should succeed");

        assert!(found.is_none(), "should return None for missing folder");
    }

    // --- MessageBodyRepo sanitisation edge cases ---

    #[tokio::test]
    async fn message_body_sanitises_onerror_attributes() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some(r#"<img src="x" onerror="alert('xss')"><p>Content</p>"#.to_owned()),
            sanitised_html: None,
            plain_text: None,
        };

        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        let sanitised = fetched.sanitised_html.expect("sanitised should exist");
        assert!(
            !sanitised.contains("onerror"),
            "onerror attribute should be stripped: {sanitised}"
        );
        assert!(
            sanitised.contains("Content"),
            "safe content should remain: {sanitised}"
        );
    }

    #[tokio::test]
    async fn message_body_sanitises_iframe_tags() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some(
                r#"<iframe src="https://evil.com/phish"></iframe><p>Real content</p>"#.to_owned(),
            ),
            sanitised_html: None,
            plain_text: None,
        };

        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        let sanitised = fetched.sanitised_html.expect("sanitised should exist");
        assert!(
            !sanitised.contains("iframe"),
            "iframe tags should be stripped: {sanitised}"
        );
        assert!(
            sanitised.contains("Real content"),
            "safe content should remain: {sanitised}"
        );
    }

    #[tokio::test]
    async fn message_body_derives_plain_text_from_html() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some("<h1>Title</h1><p>Paragraph text here.</p>".to_owned()),
            sanitised_html: None,
            plain_text: None,
        };

        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        let plain = fetched.plain_text.expect("plain text should be derived");
        assert!(
            plain.contains("Title"),
            "plain text should contain heading text: {plain}"
        );
        assert!(
            plain.contains("Paragraph text here"),
            "plain text should contain paragraph text: {plain}"
        );
        assert!(
            !plain.contains("<h1>"),
            "plain text should not contain HTML tags: {plain}"
        );
    }

    #[tokio::test]
    async fn message_body_preserves_provided_sanitised_html() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some("<p>Original</p>".to_owned()),
            sanitised_html: Some("<p>Pre-sanitised</p>".to_owned()),
            plain_text: Some("Pre-derived".to_owned()),
        };

        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        assert_eq!(
            fetched.sanitised_html.as_deref(),
            Some("<p>Pre-sanitised</p>"),
            "should use provided sanitised_html instead of re-sanitising"
        );
        assert_eq!(
            fetched.plain_text.as_deref(),
            Some("Pre-derived"),
            "should use provided plain_text instead of re-deriving"
        );
    }

    #[tokio::test]
    async fn message_body_upsert_replaces_existing_body() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body1 = MessageBody {
            message_id: message.id,
            html: Some("<p>First</p>".to_owned()),
            sanitised_html: None,
            plain_text: None,
        };
        MessageBodyRepo::upsert(&pool, &body1)
            .await
            .expect("upsert first body");

        let body2 = MessageBody {
            message_id: message.id,
            html: Some("<p>Second</p>".to_owned()),
            sanitised_html: None,
            plain_text: None,
        };
        MessageBodyRepo::upsert(&pool, &body2)
            .await
            .expect("upsert second body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        assert_eq!(fetched.html, Some("<p>Second</p>".to_owned()));
    }

    // --- Message count with all read ---

    #[tokio::test]
    async fn count_by_folder_returns_zero_unread_when_all_read() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        for i in 0..3 {
            let mut msg = make_message(account.id, folder.id);
            msg.id = MessageId::new();
            msg.uid = Some(300 + i);
            msg.message_id_header = Some(format!("<allread{i}@example.com>"));
            msg.flags.is_read = true;
            MessageRepo::insert(&pool, &msg)
                .await
                .expect("insert message");
        }

        let (total, unread) = MessageRepo::count_by_folder(&pool, &folder.id)
            .await
            .expect("count by folder");

        assert_eq!(total, 3);
        assert_eq!(unread, 0, "all messages are read so unread should be 0");
    }

    // --- Server config with non-TLS settings ---

    #[tokio::test]
    async fn server_config_preserves_non_tls_settings() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let config = iris_core::ServerConfig {
            imap: iris_core::ImapServer {
                host: "localhost".to_owned(),
                port: 143,
                use_tls: false,
            },
            smtp: iris_core::SmtpServer {
                host: "localhost".to_owned(),
                port: 25,
                use_tls: false,
            },
        };

        AccountRepo::set_server_config(&pool, &account.id, &config)
            .await
            .expect("set server config");

        let fetched = AccountRepo::get_server_config(&pool, &account.id)
            .await
            .expect("get server config")
            .expect("config should be present");

        assert!(!fetched.imap.use_tls, "IMAP TLS should be false");
        assert!(!fetched.smtp.use_tls, "SMTP TLS should be false");
        assert_eq!(fetched.imap.port, 143);
        assert_eq!(fetched.smtp.port, 25);
    }

    // --- FTS5 with message body via MessageBodyRepo ---

    #[tokio::test]
    async fn fts5_indexes_body_text_inserted_via_message_body_repo() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: Some("<p>Rhinoceros</p>".to_owned()),
            sanitised_html: None,
            plain_text: None,
        };
        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        // The body trigger should have indexed the derived plain text.
        let fts_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'Rhinoceros'",
        )
        .fetch_one(&pool)
        .await
        .expect("fts query");

        assert_eq!(fts_count.0, 1, "expected 1 FTS match for body text");
    }

    // --- Message body with only plain text (no HTML) ---

    #[tokio::test]
    async fn message_body_with_only_plain_text_round_trips() {
        let pool = test_pool().await;
        let account = make_account();
        AccountRepo::insert(&pool, &account)
            .await
            .expect("insert account");

        let folder = make_folder(account.id);
        FolderRepo::insert(&pool, &folder)
            .await
            .expect("insert folder");

        let message = make_message(account.id, folder.id);
        MessageRepo::insert(&pool, &message)
            .await
            .expect("insert message");

        let body = MessageBody {
            message_id: message.id,
            html: None,
            sanitised_html: None,
            plain_text: Some("Just plain text, no HTML.".to_owned()),
        };
        MessageBodyRepo::upsert(&pool, &body)
            .await
            .expect("upsert body");

        let fetched = MessageBodyRepo::get_by_message_id(&pool, &message.id)
            .await
            .expect("get body")
            .expect("body should exist");

        assert!(fetched.html.is_none());
        assert!(fetched.sanitised_html.is_none());
        assert_eq!(
            fetched.plain_text.as_deref(),
            Some("Just plain text, no HTML.")
        );
    }

    // --- Batch insert with zero messages ---

    #[tokio::test]
    async fn message_batch_insert_with_empty_slice_returns_zero() {
        let pool = test_pool().await;

        let inserted = MessageRepo::insert_batch(&pool, &[])
            .await
            .expect("batch insert empty");

        assert_eq!(inserted, 0);
    }
}
