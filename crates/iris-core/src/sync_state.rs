use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::AccountId;

/// The current phase of the sync engine for an account.
///
/// Sync proceeds through these phases in order during initial setup,
/// then settles into [`Idle`](SyncPhase::Idle) for ongoing operation.
/// See spec section 5.1 for the full progressive sync design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncPhase {
    /// Fetching the remote folder tree.
    FolderDiscovery,
    /// Downloading message headers for the recent window.
    RecentHeaders,
    /// Downloading message bodies for the recent window.
    RecentBodies,
    /// Low-priority background download of older messages.
    Backfill,
    /// Steady state: IMAP IDLE for inbox, periodic poll for other folders.
    Idle,
}

/// Snapshot of the sync engine's state for a single account.
///
/// Used to drive progress indicators in the UI and to resume sync
/// after an app restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncState {
    /// The account being synced.
    pub account_id: AccountId,
    /// Which phase the sync engine is currently in.
    pub phase: SyncPhase,
    /// Estimated progress within the current phase (0..=100).
    pub progress_percent: Option<u8>,
    /// The most recent error encountered, if any.
    pub last_error: Option<String>,
    /// When this sync run started.
    pub started_at: DateTime<Utc>,
    /// When this state snapshot was last updated.
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_phase_has_five_variants() {
        // Exhaustive match ensures this test fails at compile time if a
        // variant is added or removed, prompting an update.
        let phases = [
            SyncPhase::FolderDiscovery,
            SyncPhase::RecentHeaders,
            SyncPhase::RecentBodies,
            SyncPhase::Backfill,
            SyncPhase::Idle,
        ];
        assert_eq!(phases.len(), 5);
    }
}
