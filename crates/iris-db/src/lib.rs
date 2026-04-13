//! Iris DB — SQLite schema, migrations, and typed repository queries.
//!
//! This crate owns all SQL in the Iris Mail application. Other crates call
//! repository methods; they never construct queries directly. Migrations are
//! applied automatically on startup.

mod error;

pub use error::{Error, Result};
