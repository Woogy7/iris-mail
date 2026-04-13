//! PST file import.
//!
//! Parses Microsoft PST files using `libpff` bindings and imports messages
//! into the local database. Imported messages are marked `stored_local = true,
//! stored_remote = false` and indexed for search immediately.
