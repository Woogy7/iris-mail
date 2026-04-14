use chrono::Utc;
use uuid::Uuid;

/// List all configured email accounts.
#[tauri::command]
pub async fn list_accounts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<Vec<iris_core::Account>, String> {
    iris_db::repo::AccountRepo::list(&pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve a single account by its identifier.
#[tauri::command]
pub async fn get_account(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    account_id: String,
) -> Result<iris_core::Account, String> {
    let id = parse_account_id(&account_id)?;
    iris_db::repo::AccountRepo::get_by_id(&pool, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Add a new Microsoft 365 account via the OAuth2 PKCE browser flow.
///
/// Opens the system browser for user consent, exchanges the authorization code
/// for tokens, stores them in the OS keychain, and persists the account.
#[tauri::command]
pub async fn add_m365_account(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::setup::AppConfig>,
    email_address: String,
    display_name: String,
) -> Result<iris_core::Account, String> {
    let client_id = config
        .m365_client_id
        .as_deref()
        .ok_or("M365 client ID not configured — set IRIS_M365_CLIENT_ID env var")?;

    // Run the browser-based OAuth flow.
    let tokens = iris_mail::oauth::m365::start_m365_oauth(
        client_id,
        config.oauth_redirect_port,
        &email_address,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Create the account with a fresh ID and keychain reference.
    let account_id = iris_core::AccountId::new();
    let keychain_ref = account_id.0;

    // Store OAuth tokens in the OS keychain (blocking I/O).
    let tokens_clone = tokens.clone();
    tokio::task::spawn_blocking(move || {
        iris_mail::KeychainStore::new().store_oauth_tokens(&keychain_ref, &tokens_clone)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let account = iris_core::Account {
        id: account_id,
        display_name,
        email_address,
        provider: iris_core::Provider::M365,
        keychain_ref: account_id.0,
        sync_preferences: iris_core::SyncPreferences::default(),
        accent_colour: iris_core::AccentColour::default(),
        created_at: now,
        updated_at: now,
    };

    iris_db::repo::AccountRepo::insert(&pool, &account)
        .await
        .map_err(|e| e.to_string())?;

    // Discover and persist server configuration so we don't re-discover on every connect.
    if let Ok(server_config) = iris_mail::discover_servers(&account.email_address).await {
        let _ =
            iris_db::repo::AccountRepo::set_server_config(&pool, &account.id, &server_config).await;
    }

    Ok(account)
}

/// Add a new generic IMAP account with password authentication.
///
/// Stores the password in the OS keychain and persists the account.
/// Server settings are provided explicitly by the caller.
#[tauri::command]
pub async fn add_imap_account(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    email_address: String,
    display_name: String,
    password: String,
) -> Result<iris_core::Account, String> {
    let account_id = iris_core::AccountId::new();
    let keychain_ref = account_id.0;

    // Store password in the OS keychain (blocking I/O).
    let pw = password.clone();
    tokio::task::spawn_blocking(move || {
        iris_mail::KeychainStore::new().store_password(&keychain_ref, &pw)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let account = iris_core::Account {
        id: account_id,
        display_name,
        email_address,
        provider: iris_core::Provider::ImapGeneric,
        keychain_ref: account_id.0,
        sync_preferences: iris_core::SyncPreferences::default(),
        accent_colour: iris_core::AccentColour::default(),
        created_at: now,
        updated_at: now,
    };

    iris_db::repo::AccountRepo::insert(&pool, &account)
        .await
        .map_err(|e| e.to_string())?;

    // Best-effort server discovery and persistence for generic IMAP accounts.
    if let Ok(server_config) = iris_mail::discover_servers(&account.email_address).await {
        let _ =
            iris_db::repo::AccountRepo::set_server_config(&pool, &account.id, &server_config).await;
    }

    Ok(account)
}

/// Remove an account, deleting its keychain entries and database record.
///
/// Keychain deletion is best-effort — the account is removed from the database
/// even if the keychain entries cannot be found.
#[tauri::command]
pub async fn remove_account(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    account_id: String,
) -> Result<(), String> {
    let id = parse_account_id(&account_id)?;

    // Best-effort keychain cleanup (blocking I/O).
    let kr = id.0;
    let _ = tokio::task::spawn_blocking(move || {
        let keychain = iris_mail::KeychainStore::new();
        let _ = keychain.delete_oauth_tokens(&kr);
        let _ = keychain.delete_password(&kr);
    })
    .await;

    iris_db::repo::AccountRepo::delete(&pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Parses a string into an [`iris_core::AccountId`].
fn parse_account_id(s: &str) -> Result<iris_core::AccountId, String> {
    let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid account ID: {e}"))?;
    Ok(iris_core::AccountId(uuid))
}
