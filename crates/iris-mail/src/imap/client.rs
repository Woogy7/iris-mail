//! IMAP client connection and authentication.
//!
//! Wraps `async-imap` to provide a higher-level async client that handles
//! authentication (XOAUTH2 for M365/Gmail, LOGIN for generic IMAP) and
//! TLS connection establishment.

use async_imap::Session;
use async_native_tls::TlsStream;
use base64::Engine;
use iris_core::ImapServer;
use tokio::net::TcpStream;

/// Type alias for the authenticated IMAP session over TLS.
type ImapSession = Session<TlsStream<TcpStream>>;

/// An authenticated IMAP client wrapping an async-imap session.
///
/// Created via [`ImapClient::connect`], which establishes a TLS connection
/// and authenticates with the server.
pub struct ImapClient {
    /// The underlying authenticated session. Exposed as `pub(crate)` so
    /// sibling modules (folders, fetch) can use it directly.
    pub(crate) session: ImapSession,
}

/// Authentication method for IMAP connections.
pub enum ImapAuth<'a> {
    /// XOAUTH2 bearer token authentication (for M365/Gmail).
    Xoauth2 {
        /// The user's email address.
        user: &'a str,
        /// A valid OAuth2 access token.
        access_token: &'a str,
    },
    /// Plain LOGIN authentication (for generic IMAP).
    Plain {
        /// The username (usually email address).
        user: &'a str,
        /// The password.
        password: &'a str,
    },
}

/// SASL XOAUTH2 authenticator for async-imap.
///
/// Builds the base64-encoded SASL response at construction time and
/// returns it verbatim when challenged by the server.
struct Xoauth2Auth {
    /// Pre-built base64-encoded SASL XOAUTH2 response.
    response: String,
}

impl Xoauth2Auth {
    /// Create a new XOAUTH2 authenticator.
    ///
    /// The SASL XOAUTH2 format is:
    /// `"user={user}\x01auth=Bearer {token}\x01\x01"`
    /// which is then base64-encoded.
    fn new(user: &str, access_token: &str) -> Self {
        let sasl = format!("user={user}\x01auth=Bearer {access_token}\x01\x01");
        let encoded = base64::engine::general_purpose::STANDARD.encode(sasl.as_bytes());
        Self { response: encoded }
    }
}

impl async_imap::Authenticator for Xoauth2Auth {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> Self::Response {
        self.response.clone()
    }
}

impl ImapClient {
    /// Connect to an IMAP server and authenticate.
    ///
    /// Establishes a TLS connection to `server` and authenticates using the
    /// provided [`ImapAuth`] method. Returns an authenticated client ready
    /// for mailbox operations.
    pub async fn connect(server: &ImapServer, auth: ImapAuth<'_>) -> crate::Result<Self> {
        tracing::info!("Connecting to IMAP server {}:{}", server.host, server.port);

        let tcp = TcpStream::connect((server.host.as_str(), server.port))
            .await
            .map_err(|e| crate::Error::Imap(format!("TCP connection failed: {e}")))?;

        let tls = async_native_tls::connect(&server.host, tcp)
            .await
            .map_err(|e| crate::Error::Imap(format!("TLS handshake failed: {e}")))?;

        let client = async_imap::Client::new(tls);

        let session = match auth {
            ImapAuth::Xoauth2 { user, access_token } => {
                tracing::debug!("Authenticating with XOAUTH2 for {user}");
                let authenticator = Xoauth2Auth::new(user, access_token);
                client
                    .authenticate("XOAUTH2", authenticator)
                    .await
                    .map_err(|(e, _client)| {
                        crate::Error::Imap(format!("XOAUTH2 auth failed: {e}"))
                    })?
            }
            ImapAuth::Plain { user, password } => {
                tracing::debug!("Authenticating with LOGIN for {user}");
                client
                    .login(user, password)
                    .await
                    .map_err(|(e, _client)| crate::Error::Imap(format!("LOGIN failed: {e}")))?
            }
        };

        tracing::info!("IMAP authenticated successfully");
        Ok(Self { session })
    }

    /// Select a folder (mailbox) for subsequent operations.
    ///
    /// Returns mailbox metadata including message count and recent count.
    pub async fn select(&mut self, folder_path: &str) -> crate::Result<async_imap::types::Mailbox> {
        self.session.select(folder_path).await.map_err(|e| {
            crate::Error::Imap(format!("failed to select folder '{folder_path}': {e}"))
        })
    }

    /// Cleanly log out from the IMAP server.
    pub async fn logout(&mut self) -> crate::Result<()> {
        self.session
            .logout()
            .await
            .map_err(|e| crate::Error::Imap(format!("logout failed: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoauth2_sasl_string_is_correctly_encoded() {
        let auth = Xoauth2Auth::new("user@example.com", "ya29.vF9dft4qmTc2");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(auth.response.as_bytes())
            .expect("response should be valid base64");
        let sasl = String::from_utf8(decoded).expect("SASL string should be valid UTF-8");
        assert_eq!(
            sasl,
            "user=user@example.com\x01auth=Bearer ya29.vF9dft4qmTc2\x01\x01"
        );
    }

    #[test]
    fn xoauth2_with_special_characters_in_token() {
        // OAuth tokens can contain dots, underscores, hyphens, and slashes.
        let auth = Xoauth2Auth::new("alice@corp.example.com", "ya29.a0ARr_dash/sl-ash.more");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(auth.response.as_bytes())
            .expect("response should be valid base64");
        let sasl = String::from_utf8(decoded).expect("SASL string should be valid UTF-8");
        assert!(sasl.starts_with("user=alice@corp.example.com\x01"));
        assert!(sasl.contains("Bearer ya29.a0ARr_dash/sl-ash.more"));
        assert!(sasl.ends_with("\x01\x01"));
    }

    #[test]
    fn xoauth2_with_empty_token_still_produces_valid_structure() {
        let auth = Xoauth2Auth::new("u@x.com", "");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(auth.response.as_bytes())
            .expect("response should be valid base64");
        let sasl = String::from_utf8(decoded).expect("SASL string should be valid UTF-8");
        assert_eq!(sasl, "user=u@x.com\x01auth=Bearer \x01\x01");
    }

    #[test]
    fn xoauth2_authenticator_returns_same_response_on_each_call() {
        use async_imap::Authenticator;
        let mut auth = Xoauth2Auth::new("u@x.com", "token");
        let first = auth.process(b"");
        let second = auth.process(b"some challenge");
        assert_eq!(
            first, second,
            "response should be identical regardless of challenge"
        );
    }
}
