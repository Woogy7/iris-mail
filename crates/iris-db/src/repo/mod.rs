//! Repository implementations for each domain entity.
//!
//! Each repo is a unit struct with associated async functions that take a
//! `&SqlitePool` and operate on the corresponding database table.

mod account;
mod attachment;
mod folder;
mod message;
mod message_body;

pub use account::AccountRepo;
pub use attachment::AttachmentRepo;
pub use folder::FolderRepo;
pub use message::MessageRepo;
pub use message_body::MessageBodyRepo;
