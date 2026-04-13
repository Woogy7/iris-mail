//! Offline outbox and pending action queue.
//!
//! Queues compose, reply, forward, flag, and move operations when offline,
//! replaying them to the server when connectivity returns. Handles conflict
//! resolution for actions on messages that were modified server-side.
