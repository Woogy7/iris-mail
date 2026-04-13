//! Sync engine orchestrator.
//!
//! Manages the lifecycle of sync for each account: drives the progressive
//! initial sync phases, transitions to steady-state IDLE, and coordinates
//! background backfill and tiering jobs.
