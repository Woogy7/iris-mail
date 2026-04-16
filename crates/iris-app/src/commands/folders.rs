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

/// Discover folders from the remote server and persist them locally.
///
/// For M365 accounts, uses the Microsoft Graph API. For generic IMAP
/// accounts, connects to the IMAP server and issues a LIST command.
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

    sync_folders_inner(&pool, &config, &account).await
}

/// Inner folder sync logic, reusable without Tauri State wrappers.
pub(crate) async fn sync_folders_inner(
    pool: &sqlx::SqlitePool,
    config: &crate::setup::AppConfig,
    account: &iris_core::Account,
) -> Result<Vec<iris_core::Folder>, String> {
    tracing::info!(
        "Syncing folders for {} ({})",
        account.display_name,
        account.email_address
    );

    let discovered = match account.provider {
        iris_core::Provider::M365 => {
            let token = load_m365_access_token(account, config).await?;
            let client = iris_mail::GraphClient::new(token);
            iris_mail::list_graph_folders(&client).await.map_err(|e| {
                tracing::error!("Graph folder sync failed: {e}");
                e.to_string()
            })?
        }
        iris_core::Provider::ImapGeneric => {
            let mut imap_client = connect_imap_for_account(account, config, pool).await?;
            let result = iris_mail::discover_folders(&mut imap_client)
                .await
                .map_err(|e| e.to_string())?;
            let _ = imap_client.logout().await;
            result
        }
        iris_core::Provider::Gmail => {
            return Err("Gmail not yet supported".to_string());
        }
    };

    let now = Utc::now();
    for d in &discovered {
        // Look up existing folder by (account_id, full_path) to preserve its ID.
        let existing = iris_db::repo::FolderRepo::get_by_account_and_full_path(
            pool,
            &account.id,
            &d.full_path,
        )
        .await
        .map_err(|e| e.to_string())?;

        let folder = iris_core::Folder {
            id: existing
                .as_ref()
                .map(|f| f.id)
                .unwrap_or_else(iris_core::FolderId::new),
            account_id: account.id,
            parent_id: None,
            name: d.name.clone(),
            full_path: d.full_path.clone(),
            special: d.special,
            uid_validity: existing.as_ref().and_then(|f| f.uid_validity),
            last_seen_uid: existing.as_ref().and_then(|f| f.last_seen_uid),
            message_count: existing.as_ref().map_or(0, |f| f.message_count),
            unread_count: existing.as_ref().map_or(0, |f| f.unread_count),
            last_synced_at: existing.as_ref().and_then(|f| f.last_synced_at),
            created_at: existing.as_ref().map_or(now, |f| f.created_at),
            updated_at: now,
        };
        iris_db::repo::FolderRepo::upsert(pool, &folder)
            .await
            .map_err(|e| e.to_string())?;
    }

    tracing::info!("Synced {} folders", discovered.len());

    iris_db::repo::FolderRepo::list_by_account(pool, &account.id)
        .await
        .map_err(|e| e.to_string())
}

/// Loads a valid M365 access token for the given account.
///
/// Reads the refresh token from the keychain and exchanges it for a fresh
/// access token via the Microsoft token endpoint. The refresh token in the
/// keychain is updated if the server issues a new one.
pub(crate) async fn load_m365_access_token(
    account: &iris_core::Account,
    config: &crate::setup::AppConfig,
) -> Result<String, String> {
    let client_id = config
        .m365_client_id
        .as_deref()
        .ok_or("M365 client ID not configured — set IRIS_M365_CLIENT_ID env var")?;

    let kr = account.keychain_ref;
    let refresh_token = tokio::task::spawn_blocking(move || {
        iris_mail::KeychainStore::new().load_refresh_token(&kr)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let tokens = iris_mail::oauth::m365::refresh_m365_token(client_id, &refresh_token)
        .await
        .map_err(|e| {
            tracing::error!("Token refresh failed for {}: {e}", account.email_address);
            e.to_string()
        })?;

    // Store the (possibly rotated) refresh token back to keychain.
    if tokens.refresh_token != refresh_token {
        let new_rt = tokens.refresh_token.clone();
        let kr2 = account.keychain_ref;
        tokio::task::spawn_blocking(move || {
            iris_mail::KeychainStore::new().store_refresh_token(&kr2, &new_rt)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    }

    Ok(tokens.access_token)
}

/// Connects to the IMAP server for the given account.
///
/// First tries to load server settings from the database. Falls back to
/// auto-discovery if none are persisted, then stores the discovered config
/// for next time. Loads credentials from the OS keychain and refreshes
/// OAuth tokens when needed.
pub(crate) async fn connect_imap_for_account(
    account: &iris_core::Account,
    config: &crate::setup::AppConfig,
    pool: &sqlx::SqlitePool,
) -> Result<iris_mail::ImapClient, String> {
    let server_config = match iris_db::repo::AccountRepo::get_server_config(pool, &account.id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(cached) => cached,
        None => {
            let discovered = iris_mail::discover_servers(&account.email_address)
                .await
                .map_err(|e| e.to_string())?;
            iris_db::repo::AccountRepo::set_server_config(pool, &account.id, &discovered)
                .await
                .map_err(|e| e.to_string())?;
            discovered
        }
    };

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
    let access_token = load_m365_access_token(account, config).await?;

    let auth = iris_mail::ImapAuth::Xoauth2 {
        user: &account.email_address,
        access_token: &access_token,
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
