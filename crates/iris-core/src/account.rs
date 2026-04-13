use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strongly-typed identifier for an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Uuid);

impl AccountId {
    /// Creates a new random account identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::new()
    }
}

/// The email provider type for an account.
///
/// Determines which authentication flow and protocol extensions to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    /// Microsoft 365 (OAuth2 + IMAP with XOAUTH2).
    M365,
    /// Gmail (OAuth2 + IMAP with XOAUTH2, X-GM-EXT-1 extensions).
    Gmail,
    /// Generic IMAP server with username/password authentication.
    ImapGeneric,
}

/// Per-account accent colour drawn from the Catppuccin named colour palette.
///
/// Used in the sidebar to visually distinguish accounts at a glance.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccentColour {
    /// Catppuccin Red.
    Red,
    /// Catppuccin Peach.
    Peach,
    /// Catppuccin Yellow.
    Yellow,
    /// Catppuccin Green.
    Green,
    /// Catppuccin Sapphire.
    Sapphire,
    /// Catppuccin Mauve (the default).
    #[default]
    Mauve,
    /// Catppuccin Lavender.
    Lavender,
}

impl AccentColour {
    /// Returns the lowercase string representation of this colour.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Peach => "peach",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Sapphire => "sapphire",
            Self::Mauve => "mauve",
            Self::Lavender => "lavender",
        }
    }
}

/// Per-account sync and storage configuration.
///
/// These values control how aggressively Iris syncs and when storage tiering
/// kicks in. Defaults match the spec: 60-day window, 50 msg/min rate limit,
/// 120-second poll interval, 30 GB synced tier, archive disabled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPreferences {
    /// How many days of history to sync during initial setup.
    pub initial_sync_days: u32,
    /// Maximum messages per minute during background backfill.
    pub rate_limit_per_minute: u32,
    /// Seconds between poll cycles for non-IDLE folders.
    pub poll_interval_secs: u32,
    /// Server-side storage threshold in bytes before tiering activates.
    pub synced_tier_bytes: u64,
    /// Whether automatic archiving (tiering) is enabled.
    pub is_archive_enabled: bool,
}

impl Default for SyncPreferences {
    fn default() -> Self {
        Self {
            initial_sync_days: 60,
            rate_limit_per_minute: 50,
            poll_interval_secs: 120,
            synced_tier_bytes: 30 * 1024 * 1024 * 1024, // 30 GB
            is_archive_enabled: false,
        }
    }
}

/// A configured email account in Iris Mail.
///
/// Each account represents one email address with its authentication credentials
/// (stored in the OS keychain, referenced here by a UUID), provider-specific
/// settings, and user preferences for sync behaviour and visual appearance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier for this account.
    pub id: AccountId,
    /// Human-readable display name (e.g. "Work" or "Personal").
    pub display_name: String,
    /// The email address associated with this account.
    pub email_address: String,
    /// Which provider this account connects to.
    pub provider: Provider,
    /// UUID referencing the OAuth/credential entry in the OS keychain.
    pub keychain_ref: Uuid,
    /// Sync and storage tiering preferences.
    pub sync_preferences: SyncPreferences,
    /// Visual accent colour for sidebar identification.
    pub accent_colour: AccentColour,
    /// When this account was first added to Iris.
    pub created_at: DateTime<Utc>,
    /// When this account's configuration was last modified.
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_preferences_default_values_match_spec() {
        let prefs = SyncPreferences::default();
        assert_eq!(prefs.initial_sync_days, 60);
        assert_eq!(prefs.rate_limit_per_minute, 50);
        assert_eq!(prefs.poll_interval_secs, 120);
        assert_eq!(prefs.synced_tier_bytes, 30 * 1024 * 1024 * 1024);
        assert!(!prefs.is_archive_enabled);
    }

    #[test]
    fn accent_colour_as_str_returns_lowercase_name() {
        assert_eq!(AccentColour::Red.as_str(), "red");
        assert_eq!(AccentColour::Peach.as_str(), "peach");
        assert_eq!(AccentColour::Yellow.as_str(), "yellow");
        assert_eq!(AccentColour::Green.as_str(), "green");
        assert_eq!(AccentColour::Sapphire.as_str(), "sapphire");
        assert_eq!(AccentColour::Mauve.as_str(), "mauve");
        assert_eq!(AccentColour::Lavender.as_str(), "lavender");
    }

    #[test]
    fn account_id_generates_unique_values() {
        let a = AccountId::new();
        let b = AccountId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn default_accent_colour_is_mauve() {
        assert_eq!(AccentColour::default(), AccentColour::Mauve);
    }
}
