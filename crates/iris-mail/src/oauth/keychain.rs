//! OS keychain storage for OAuth tokens and passwords.
//!
//! Uses the [`keyring`] crate to store secrets in the platform's native
//! credential store (e.g. GNOME Keyring, macOS Keychain, Windows Credential Manager).

use crate::oauth::OauthTokens;

const OAUTH_SERVICE: &str = "iris-mail-oauth";
const PASSWORD_SERVICE: &str = "iris-mail-imap";

/// Stores and retrieves credentials from the OS keychain.
pub struct KeychainStore;

impl KeychainStore {
    /// Create a new keychain store.
    pub fn new() -> Self {
        Self
    }

    /// Store OAuth tokens in the keychain, keyed by account ID.
    pub fn store_oauth_tokens(
        &self,
        account_id: &uuid::Uuid,
        tokens: &OauthTokens,
    ) -> crate::Result<()> {
        let json = serde_json::to_string(tokens)
            .map_err(|e| crate::Error::Keychain(format!("failed to serialize tokens: {e}")))?;
        let entry = keyring::Entry::new(OAUTH_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry
            .set_password(&json)
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        Ok(())
    }

    /// Load OAuth tokens from the keychain.
    pub fn load_oauth_tokens(&self, account_id: &uuid::Uuid) -> crate::Result<OauthTokens> {
        let entry = keyring::Entry::new(OAUTH_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        let json = entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => crate::Error::TokenNotFound(account_id.to_string()),
            other => crate::Error::Keychain(other.to_string()),
        })?;
        serde_json::from_str(&json)
            .map_err(|e| crate::Error::Keychain(format!("failed to deserialize tokens: {e}")))
    }

    /// Delete OAuth tokens from the keychain.
    pub fn delete_oauth_tokens(&self, account_id: &uuid::Uuid) -> crate::Result<()> {
        let entry = keyring::Entry::new(OAUTH_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry
            .delete_credential()
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        Ok(())
    }

    /// Store a password for a generic IMAP account.
    pub fn store_password(&self, account_id: &uuid::Uuid, password: &str) -> crate::Result<()> {
        let entry = keyring::Entry::new(PASSWORD_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry
            .set_password(password)
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        Ok(())
    }

    /// Load a password for a generic IMAP account.
    pub fn load_password(&self, account_id: &uuid::Uuid) -> crate::Result<String> {
        let entry = keyring::Entry::new(PASSWORD_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => crate::Error::TokenNotFound(account_id.to_string()),
            other => crate::Error::Keychain(other.to_string()),
        })
    }

    /// Delete a password for a generic IMAP account.
    pub fn delete_password(&self, account_id: &uuid::Uuid) -> crate::Result<()> {
        let entry = keyring::Entry::new(PASSWORD_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry
            .delete_credential()
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        Ok(())
    }
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}
