use chrono::Utc;
use uuid::Uuid;

/// List folders for a given account.
#[tauri::command]
pub async fn list_folders(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    account_id: String,
) -> Result<Vec<iris_core::Folder>, String> {
    let id = parse_account_id(&account_id)?;
    iris_db::repo::FolderRepo::list_by_account(&pool, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Discover folders from the remote IMAP server and persist them locally.
///
/// Connects to the account's IMAP server, lists all mailboxes, converts them
/// to [`iris_core::Folder`] records, and upserts them into the database.
/// Returns the full folder list for the account after sync.
#[tauri::command]
pub async fn sync_folders(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::setup::AppConfig>,
    account_id: String,
) -> Result<Vec<iris_core::Folder>, String> {
    let id = parse_account_id(&account_id)?;
    let account = iris_db::repo::AccountRepo::get_by_id(&pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    let mut imap_client = connect_imap_for_account(&account, &config)
        .await
        .map_err(|e| e.to_string())?;

    let discovered = iris_mail::discover_folders(&mut imap_client)
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    for d in &discovered {
        let folder = iris_core::Folder {
            id: iris_core::FolderId::new(),
            account_id: account.id,
            parent_id: None,
            name: d.name.clone(),
            full_path: d.full_path.clone(),
            special: d.special,
            uid_validity: None,
            last_seen_uid: None,
            message_count: 0,
            unread_count: 0,
            last_synced_at: None,
            created_at: now,
            updated_at: now,
        };
        iris_db::repo::FolderRepo::upsert(&pool, &folder)
            .await
            .map_err(|e| e.to_string())?;
    }

    let _ = imap_client.logout().await;

    iris_db::repo::FolderRepo::list_by_account(&pool, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Connects to the IMAP server for the given account.
///
/// Loads credentials from the OS keychain and uses auto-discovery to find
/// server settings. For M365, refreshes the OAuth token if it has expired.
pub(crate) async fn connect_imap_for_account(
    account: &iris_core::Account,
    config: &crate::setup::AppConfig,
) -> Result<iris_mail::ImapClient, String> {
    let server_config = iris_mail::discover_servers(&account.email_address)
        .await
        .map_err(|e| e.to_string())?;

    match account.provider {
        iris_core::Provider::M365 => connect_m365(account, config, &server_config.imap).await,
        iris_core::Provider::ImapGeneric => connect_plain(account, &server_config.imap).await,
        iris_core::Provider::Gmail => Err("Gmail not yet supported".to_string()),
    }
}

/// Connects to IMAP via XOAUTH2, refreshing the token if expired.
async fn connect_m365(
    account: &iris_core::Account,
    config: &crate::setup::AppConfig,
    imap_server: &iris_core::ImapServer,
) -> Result<iris_mail::ImapClient, String> {
    let kr = account.keychain_ref;
    let mut tokens =
        tokio::task::spawn_blocking(move || iris_mail::KeychainStore::new().load_oauth_tokens(&kr))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

    if tokens.is_expired() {
        let client_id = config
            .m365_client_id
            .as_deref()
            .ok_or("M365 client ID not configured — set IRIS_M365_CLIENT_ID env var")?;

        tokens = iris_mail::oauth::m365::refresh_m365_token(client_id, &tokens.refresh_token)
            .await
            .map_err(|e| e.to_string())?;

        let new_tokens = tokens.clone();
        let kr2 = account.keychain_ref;
        tokio::task::spawn_blocking(move || {
            iris_mail::KeychainStore::new().store_oauth_tokens(&kr2, &new_tokens)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    }

    let auth = iris_mail::ImapAuth::Xoauth2 {
        user: &account.email_address,
        access_token: &tokens.access_token,
    };
    iris_mail::ImapClient::connect(imap_server, auth)
        .await
        .map_err(|e| e.to_string())
}

/// Connects to IMAP via plain LOGIN authentication.
async fn connect_plain(
    account: &iris_core::Account,
    imap_server: &iris_core::ImapServer,
) -> Result<iris_mail::ImapClient, String> {
    let kr = account.keychain_ref;
    let password =
        tokio::task::spawn_blocking(move || iris_mail::KeychainStore::new().load_password(&kr))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

    let auth = iris_mail::ImapAuth::Plain {
        user: &account.email_address,
        password: &password,
    };
    iris_mail::ImapClient::connect(imap_server, auth)
        .await
        .map_err(|e| e.to_string())
}

/// Parses a string into an [`iris_core::AccountId`].
fn parse_account_id(s: &str) -> Result<iris_core::AccountId, String> {
    let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid account ID: {e}"))?;
    Ok(iris_core::AccountId(uuid))
}
