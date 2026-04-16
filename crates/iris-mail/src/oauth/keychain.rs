//! OS keychain storage for OAuth refresh tokens and passwords.
//!
//! Uses the [`keyring`] crate to store secrets in the platform's native
//! credential store (e.g. GNOME Keyring, macOS Keychain, Windows Credential Manager).
//!
//! Only the **refresh token** is stored — not the full access token, which can
//! be very large (1500+ chars JWT) and exceed Secret Service size limits on some
//! Linux backends. Access tokens are short-lived and re-obtained on demand.

const OAUTH_SERVICE: &str = "iris-mail-oauth";
const PASSWORD_SERVICE: &str = "iris-mail-imap";

/// Stores and retrieves credentials from the OS keychain.
pub struct KeychainStore;

impl KeychainStore {
    /// Create a new keychain store.
    pub fn new() -> Self {
        Self
    }

    /// Store an OAuth refresh token in the keychain, keyed by account ID.
    ///
    /// Only the refresh token is stored — access tokens are short-lived and
    /// obtained on demand via the refresh token.
    pub fn store_refresh_token(
        &self,
        account_id: &uuid::Uuid,
        refresh_token: &str,
    ) -> crate::Result<()> {
        let entry = keyring::Entry::new(OAUTH_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry
            .set_password(refresh_token)
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        Ok(())
    }

    /// Load an OAuth refresh token from the keychain.
    pub fn load_refresh_token(&self, account_id: &uuid::Uuid) -> crate::Result<String> {
        let entry = keyring::Entry::new(OAUTH_SERVICE, &account_id.to_string())
            .map_err(|e| crate::Error::Keychain(e.to_string()))?;
        entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => crate::Error::TokenNotFound(account_id.to_string()),
            other => crate::Error::Keychain(other.to_string()),
        })
    }

    /// Delete an OAuth refresh token from the keychain.
    pub fn delete_refresh_token(&self, account_id: &uuid::Uuid) -> crate::Result<()> {
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
