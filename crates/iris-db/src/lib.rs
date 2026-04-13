//! Iris DB — SQLite schema, migrations, and typed repository queries.
//!
//! This crate owns all SQL in the Iris Mail application. Other crates call
//! repository methods; they never construct queries directly. Migrations are
//! applied automatically on startup.

mod error;
pub mod repo;

pub use error::{Error, Result};
pub use repo::{AccountRepo, AttachmentRepo, FolderRepo, MessageRepo};

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
        MessageFlags, MessageId, Provider, SpecialFolder, SyncPreferences,
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
}
