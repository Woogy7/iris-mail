//! Full account synchronisation command.
//!
//! Syncs folders and recent messages for all folders in a single operation.

use uuid::Uuid;

/// Perform a full sync of an account: discover folders, then fetch recent
/// messages for every folder.
///
/// This is the main sync entry point called on account add, app launch,
/// manual sync, and periodic background sync.
#[tauri::command]
pub async fn sync_account(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::setup::AppConfig>,
    account_id: String,
) -> Result<(), String> {
    let id = parse_account_id(&account_id)?;
    let account = iris_db::repo::AccountRepo::get_by_id(&pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "Full sync starting for {} ({})",
        account.display_name,
        account.email_address
    );

    // Step 1: Sync folders.
    let folders = crate::commands::folders::sync_folders_inner(&pool, &config, &account)
        .await
        .map_err(|e| {
            tracing::error!("Folder sync failed for {}: {e}", account.email_address);
            e
        })?;

    tracing::info!("Synced {} folders, now fetching messages...", folders.len());

    // Step 2: Fetch recent messages for each folder.
    for folder in &folders {
        match sync_folder_messages(&pool, &config, &account, folder).await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(
                        "  {} — fetched {} new messages",
                        folder.name,
                        count
                    );
                }
            }
            Err(e) => {
                tracing::warn!("  {} — message fetch failed: {e}", folder.name);
                // Continue with other folders rather than aborting.
            }
        }
    }

    tracing::info!("Full sync complete for {}", account.email_address);
    Ok(())
}

/// Sync all configured accounts.
#[tauri::command]
pub async fn sync_all_accounts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::setup::AppConfig>,
) -> Result<(), String> {
    let accounts = iris_db::repo::AccountRepo::list(&pool)
        .await
        .map_err(|e| e.to_string())?;

    for account in &accounts {
        // Use the inner function directly to avoid Tauri State wrapping issues.
        if let Err(e) = sync_account_inner(&pool, &config, account).await {
            tracing::error!("Sync failed for {}: {e}", account.email_address);
        }
    }

    Ok(())
}

/// Inner sync logic without Tauri State wrappers (reusable).
async fn sync_account_inner(
    pool: &sqlx::SqlitePool,
    config: &crate::setup::AppConfig,
    account: &iris_core::Account,
) -> Result<(), String> {
    let folders = crate::commands::folders::sync_folders_inner(pool, config, account).await?;

    for folder in &folders {
        match sync_folder_messages(pool, config, account, folder).await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(
                        "  {} — fetched {} new messages",
                        folder.name,
                        count
                    );
                }
            }
            Err(e) => {
                tracing::warn!("  {} — message fetch failed: {e}", folder.name);
            }
        }
    }

    Ok(())
}

/// Fetch recent messages for a single folder.
async fn sync_folder_messages(
    pool: &sqlx::SqlitePool,
    config: &crate::setup::AppConfig,
    account: &iris_core::Account,
    folder: &iris_core::Folder,
) -> Result<u64, String> {
    match account.provider {
        iris_core::Provider::M365 => {
            let token =
                crate::commands::folders::load_m365_access_token(account, config).await?;
            let client = iris_mail::GraphClient::new(token);
            let fetched = iris_mail::fetch_graph_messages(&client, &folder.full_path, 100)
                .await
                .map_err(|e| e.to_string())?;

            // Filter out messages we already have (by remote_id).
            let mut new_messages = Vec::new();
            let now = chrono::Utc::now();
            for f in fetched {
                if let Some(ref rid) = f.remote_id {
                    let exists = iris_db::repo::MessageRepo::get_by_remote_id(
                        pool, &account.id, rid,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    if exists.is_some() {
                        continue;
                    }
                }
                new_messages.push(iris_core::Message {
                    id: iris_core::MessageId::new(),
                    account_id: account.id,
                    folder_id: folder.id,
                    uid: None,
                    remote_id: f.remote_id,
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
                    is_stored_local: true,
                    is_stored_remote: true,
                    created_at: now,
                    updated_at: now,
                });
            }

            if new_messages.is_empty() {
                return Ok(0);
            }

            let count = iris_db::repo::MessageRepo::insert_batch(pool, &new_messages)
                .await
                .map_err(|e| e.to_string())?;

            Ok(count)
        }
        iris_core::Provider::ImapGeneric => {
            let mut imap_client =
                crate::commands::folders::connect_imap_for_account(account, config, pool).await?;

            imap_client
                .select(&folder.full_path)
                .await
                .map_err(|e| e.to_string())?;

            let fetched = iris_mail::fetch_message_headers(&mut imap_client, "1:*")
                .await
                .map_err(|e| e.to_string())?;

            // Filter out messages we already have (by imap_uid).
            let mut new_messages = Vec::new();
            let now = chrono::Utc::now();
            for f in fetched {
                let exists = iris_db::repo::MessageRepo::get_by_uid(
                    pool, &folder.id, f.uid,
                )
                .await
                .map_err(|e| e.to_string())?;
                if exists.is_some() {
                    continue;
                }
                new_messages.push(iris_core::Message {
                    id: iris_core::MessageId::new(),
                    account_id: account.id,
                    folder_id: folder.id,
                    uid: Some(f.uid),
                    remote_id: None,
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
                });
            }

            let _ = imap_client.logout().await;

            if new_messages.is_empty() {
                return Ok(0);
            }

            let count = iris_db::repo::MessageRepo::insert_batch(pool, &new_messages)
                .await
                .map_err(|e| e.to_string())?;

            Ok(count)
        }
        iris_core::Provider::Gmail => Err("Gmail not yet supported".to_string()),
    }
}

fn parse_account_id(s: &str) -> Result<iris_core::AccountId, String> {
    let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid account ID: {e}"))?;
    Ok(iris_core::AccountId(uuid))
}
