use chrono::DateTime;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use iris_core::{AccentColour, Account, AccountId, Provider};

use crate::{Error, Result};

/// Repository for [`Account`] persistence.
pub struct AccountRepo;

impl AccountRepo {
    /// Inserts a new account into the database.
    pub async fn insert(pool: &SqlitePool, account: &Account) -> Result<()> {
        let id = account.id.0.to_string();
        let provider = provider_to_str(account.provider);
        let keychain_ref = account.keychain_ref.to_string();
        let accent_colour = account.accent_colour.as_str();
        let sync_prefs = serde_json::to_string(&account.sync_preferences)?;
        let created_at = account.created_at.to_rfc3339();
        let updated_at = account.updated_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO accounts (id, provider, display_name, email_address, keychain_ref, \
             accent_colour, sync_preferences, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&id)
        .bind(provider)
        .bind(&account.display_name)
        .bind(&account.email_address)
        .bind(&keychain_ref)
        .bind(accent_colour)
        .bind(&sync_prefs)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieves an account by its identifier.
    pub async fn get_by_id(pool: &SqlitePool, id: &AccountId) -> Result<Account> {
        let id_str = id.0.to_string();

        let row = sqlx::query("SELECT * FROM accounts WHERE id = ?1")
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| Error::NotFound {
                entity: "account",
                id: id_str,
            })?;

        account_from_row(&row)
    }

    /// Lists all accounts ordered by display name.
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Account>> {
        let rows = sqlx::query("SELECT * FROM accounts ORDER BY display_name")
            .fetch_all(pool)
            .await?;

        rows.iter().map(account_from_row).collect()
    }

    /// Updates an existing account.
    pub async fn update(pool: &SqlitePool, account: &Account) -> Result<()> {
        let id = account.id.0.to_string();
        let provider = provider_to_str(account.provider);
        let keychain_ref = account.keychain_ref.to_string();
        let accent_colour = account.accent_colour.as_str();
        let sync_prefs = serde_json::to_string(&account.sync_preferences)?;
        let updated_at = account.updated_at.to_rfc3339();

        let result = sqlx::query(
            "UPDATE accounts SET provider = ?1, display_name = ?2, email_address = ?3, \
             keychain_ref = ?4, accent_colour = ?5, sync_preferences = ?6, updated_at = ?7 \
             WHERE id = ?8",
        )
        .bind(provider)
        .bind(&account.display_name)
        .bind(&account.email_address)
        .bind(&keychain_ref)
        .bind(accent_colour)
        .bind(&sync_prefs)
        .bind(&updated_at)
        .bind(&id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "account",
                id,
            });
        }

        Ok(())
    }

    /// Store the server configuration for an account.
    ///
    /// Persists IMAP/SMTP host, port, and TLS settings so they do not need to
    /// be re-discovered on every connection.
    pub async fn set_server_config(
        pool: &SqlitePool,
        account_id: &AccountId,
        config: &iris_core::ServerConfig,
    ) -> Result<()> {
        let id_str = account_id.0.to_string();
        let json = serde_json::to_string(config)?;

        let result = sqlx::query("UPDATE accounts SET server_config = ?1 WHERE id = ?2")
            .bind(&json)
            .bind(&id_str)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "account",
                id: id_str,
            });
        }

        Ok(())
    }

    /// Retrieve the server configuration for an account.
    ///
    /// Returns `None` if the server config has not been set (i.e. the stored
    /// value is the default empty JSON object or the account does not exist).
    pub async fn get_server_config(
        pool: &SqlitePool,
        account_id: &AccountId,
    ) -> Result<Option<iris_core::ServerConfig>> {
        let id_str = account_id.0.to_string();

        let row = sqlx::query("SELECT server_config FROM accounts WHERE id = ?1")
            .bind(&id_str)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => {
                let json: String = row.get("server_config");
                if json == "{}" || json.is_empty() {
                    return Ok(None);
                }
                let config: iris_core::ServerConfig = serde_json::from_str(&json)?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Deletes an account by its identifier.
    pub async fn delete(pool: &SqlitePool, id: &AccountId) -> Result<()> {
        let id_str = id.0.to_string();

        let result = sqlx::query("DELETE FROM accounts WHERE id = ?1")
            .bind(&id_str)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                entity: "account",
                id: id_str,
            });
        }

        Ok(())
    }
}

/// Maps a [`Provider`] variant to its database string representation.
fn provider_to_str(provider: Provider) -> &'static str {
    match provider {
        Provider::M365 => "m365",
        Provider::Gmail => "gmail",
        Provider::ImapGeneric => "imap_generic",
    }
}

/// Parses a database string into a [`Provider`] variant.
fn provider_from_str(s: &str) -> Result<Provider> {
    match s {
        "m365" => Ok(Provider::M365),
        "gmail" => Ok(Provider::Gmail),
        "imap_generic" => Ok(Provider::ImapGeneric),
        other => Err(Error::Sqlx(sqlx::Error::Decode(
            format!("unknown provider: {other}").into(),
        ))),
    }
}

/// Parses a database string into an [`AccentColour`] variant.
fn accent_colour_from_str(s: &str) -> Result<AccentColour> {
    match s {
        "red" => Ok(AccentColour::Red),
        "peach" => Ok(AccentColour::Peach),
        "yellow" => Ok(AccentColour::Yellow),
        "green" => Ok(AccentColour::Green),
        "sapphire" => Ok(AccentColour::Sapphire),
        "mauve" => Ok(AccentColour::Mauve),
        "lavender" => Ok(AccentColour::Lavender),
        other => Err(Error::Sqlx(sqlx::Error::Decode(
            format!("unknown accent colour: {other}").into(),
        ))),
    }
}

/// Constructs an [`Account`] from a SQLite row.
fn account_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Account> {
    let id_str: String = row.get("id");
    let provider_str: String = row.get("provider");
    let keychain_str: String = row.get("keychain_ref");
    let accent_str: String = row.get("accent_colour");
    let sync_prefs_str: String = row.get("sync_preferences");
    let created_str: String = row.get("created_at");
    let updated_str: String = row.get("updated_at");

    let id = Uuid::parse_str(&id_str).map_err(|e| {
        Error::Sqlx(sqlx::Error::Decode(
            format!("invalid account id: {e}").into(),
        ))
    })?;
    let keychain_ref = Uuid::parse_str(&keychain_str).map_err(|e| {
        Error::Sqlx(sqlx::Error::Decode(
            format!("invalid keychain_ref: {e}").into(),
        ))
    })?;
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map_err(|e| {
            Error::Sqlx(sqlx::Error::Decode(
                format!("invalid created_at: {e}").into(),
            ))
        })?
        .to_utc();
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map_err(|e| {
            Error::Sqlx(sqlx::Error::Decode(
                format!("invalid updated_at: {e}").into(),
            ))
        })?
        .to_utc();

    Ok(Account {
        id: AccountId(id),
        provider: provider_from_str(&provider_str)?,
        display_name: row.get("display_name"),
        email_address: row.get("email_address"),
        keychain_ref,
        accent_colour: accent_colour_from_str(&accent_str)?,
        sync_preferences: serde_json::from_str(&sync_prefs_str)?,
        created_at,
        updated_at,
    })
}
