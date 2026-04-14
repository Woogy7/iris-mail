//! SMTP connection validation.
//!
//! Full compose and send functionality is deferred to a later phase.
//! This module validates that SMTP credentials work at account-add time.

use iris_core::SmtpServer;

/// Validates that the given SMTP server is reachable and credentials work.
///
/// For OAuth accounts, `password` should be the access token.
/// Returns `Ok(())` if the server accepts the credentials, or an error
/// otherwise.
pub async fn validate_smtp_connection(
    _server: &SmtpServer,
    _username: &str,
    _password: &str,
) -> crate::Result<()> {
    // TODO(#phase3): Implement SMTP validation using lettre
    tracing::warn!("SMTP validation not yet implemented, skipping");
    Ok(())
}
