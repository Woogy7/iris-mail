//! IMAP sync primitives: UID-based incremental fetch, UIDVALIDITY handling.
//!
//! Provides the building blocks that the sync engine in `iris-sync` orchestrates
//! to implement progressive initial sync and ongoing synchronisation.
