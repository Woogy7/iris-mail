//! Gmail OAuth2 PKCE flow.
//!
//! Not yet implemented. Gmail requires Google's OAuth app verification process.
//! For now, Gmail accounts should use generic IMAP with an app password.

use super::OauthTokens;

/// Starts the Gmail OAuth2 PKCE flow.
///
/// Not yet implemented. Returns an error directing the user to use generic IMAP.
pub async fn start_gmail_oauth(
    _client_id: &str,
    _redirect_port: u16,
) -> crate::Result<OauthTokens> {
    Err(crate::Error::NotImplemented(
        "Gmail OAuth not yet implemented — use generic IMAP with an app password".to_owned(),
    ))
}
