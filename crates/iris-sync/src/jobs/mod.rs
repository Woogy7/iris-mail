//! Background sync jobs.
//!
//! Each job type handles one phase of the sync lifecycle. Jobs are queued
//! in the database and survive app restarts.

pub mod backfill;
pub mod idle;
pub mod initial;
pub mod tiering;
