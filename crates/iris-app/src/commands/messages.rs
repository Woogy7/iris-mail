use chrono::Utc;
use uuid::Uuid;

/// List messages in a folder with pagination.
#[tauri::command]
pub async fn list_messages(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<iris_core::Message>, String> {
    let id = parse_folder_id(&folder_id)?;
    iris_db::repo::MessageRepo::list_by_folder(&pool, &id, limit.unwrap_or(50), offset.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

/// Fetch recent message headers from the IMAP server for a folder.
///
/// Connects to the account's IMAP server, selects the folder, fetches the
/// most recent message headers, converts them to [`iris_core::Message`]
/// records, and batch-inserts them into the database. Returns the messages
/// for the folder from the database after sync.
#[tauri::command]
pub async fn fetch_folder_messages(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::setup::AppConfig>,
    account_id: String,
    folder_id: String,
) -> Result<Vec<iris_core::Message>, String> {
    let acct_id = parse_account_id(&account_id)?;
    let fld_id = parse_folder_id(&folder_id)?;

    let account = iris_db::repo::AccountRepo::get_by_id(&pool, &acct_id)
        .await
        .map_err(|e| e.to_string())?;
    let folder = iris_db::repo::FolderRepo::get_by_id(&pool, &fld_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut imap_client =
        crate::commands::folders::connect_imap_for_account(&account, &config).await?;

    imap_client
        .select(&folder.full_path)
        .await
        .map_err(|e| e.to_string())?;

    // Fetch the most recent 100 messages by UID range.
    let uid_range = "1:*";
    let fetched = iris_mail::fetch_message_headers(&mut imap_client, uid_range)
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let messages: Vec<iris_core::Message> = fetched
        .into_iter()
        .map(|f| iris_core::Message {
            id: iris_core::MessageId::new(),
            account_id: account.id,
            folder_id: folder.id,
            uid: Some(f.uid),
            message_id_header: f.message_id,
            thread_id: None,
            subject: f.subject,
            from_name: f.from_name,
            from_address: f.from_address,
            to_addresses: if f.to_addresses.is_empty() {
                None
            } else {
                Some(f.to_addresses.join(", "))
            },
            cc_addresses: if f.cc_addresses.is_empty() {
                None
            } else {
                Some(f.cc_addresses.join(", "))
            },
            bcc_addresses: None,
            date: f.date,
            size_bytes: Some(u64::from(f.size)),
            flags: f.flags,
            is_stored_local: false,
            is_stored_remote: true,
            created_at: now,
            updated_at: now,
        })
        .collect();

    iris_db::repo::MessageRepo::insert_batch(&pool, &messages)
        .await
        .map_err(|e| e.to_string())?;

    let _ = imap_client.logout().await;

    iris_db::repo::MessageRepo::list_by_folder(&pool, &fld_id, 50, 0)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve the full body of a message, fetching from IMAP if not cached.
///
/// Checks the local database first. If the body is not found, connects to
/// the account's IMAP server, selects the folder, fetches the body by UID,
/// sanitises the HTML, and caches it locally before returning.
#[tauri::command]
pub async fn get_message_body(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::setup::AppConfig>,
    message_id: String,
) -> Result<iris_core::MessageBody, String> {
    let msg_id = parse_message_id(&message_id)?;

    // Check local cache first.
    if let Some(body) = iris_db::repo::MessageBodyRepo::get_by_message_id(&pool, &msg_id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(body);
    }

    // Not cached — fetch from IMAP.
    let message = iris_db::repo::MessageRepo::get_by_id(&pool, &msg_id)
        .await
        .map_err(|e| e.to_string())?;
    let folder = iris_db::repo::FolderRepo::get_by_id(&pool, &message.folder_id)
        .await
        .map_err(|e| e.to_string())?;
    let account = iris_db::repo::AccountRepo::get_by_id(&pool, &message.account_id)
        .await
        .map_err(|e| e.to_string())?;

    let uid = message
        .uid
        .ok_or("message has no IMAP UID — cannot fetch body")?;

    let mut imap_client =
        crate::commands::folders::connect_imap_for_account(&account, &config).await?;

    imap_client
        .select(&folder.full_path)
        .await
        .map_err(|e| e.to_string())?;

    let fetched = iris_mail::fetch_message_body(&mut imap_client, uid)
        .await
        .map_err(|e| e.to_string())?;

    let _ = imap_client.logout().await;

    let body = iris_core::MessageBody {
        message_id: msg_id,
        html: fetched.html,
        sanitised_html: None, // MessageBodyRepo::upsert auto-sanitises
        plain_text: fetched.plain_text,
    };

    iris_db::repo::MessageBodyRepo::upsert(&pool, &body)
        .await
        .map_err(|e| e.to_string())?;

    // Re-read from DB to get the auto-sanitised HTML.
    iris_db::repo::MessageBodyRepo::get_by_message_id(&pool, &msg_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "failed to retrieve body after upsert".to_string())
}

/// Parses a string into an [`iris_core::AccountId`].
fn parse_account_id(s: &str) -> Result<iris_core::AccountId, String> {
    let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid account ID: {e}"))?;
    Ok(iris_core::AccountId(uuid))
}

/// Parses a string into an [`iris_core::FolderId`].
fn parse_folder_id(s: &str) -> Result<iris_core::FolderId, String> {
    let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid folder ID: {e}"))?;
    Ok(iris_core::FolderId(uuid))
}

/// Parses a string into an [`iris_core::MessageId`].
fn parse_message_id(s: &str) -> Result<iris_core::MessageId, String> {
    let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid message ID: {e}"))?;
    Ok(iris_core::MessageId(uuid))
}
