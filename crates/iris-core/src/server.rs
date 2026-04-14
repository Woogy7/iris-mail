//! Server connection configuration types for IMAP and SMTP.

use serde::{Deserialize, Serialize};

/// IMAP server connection configuration.
///
/// Stores the host, port, and TLS settings needed to connect to an IMAP server.
/// These values come from auto-discovery or manual user entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImapServer {
    /// Hostname (e.g. "outlook.office365.com").
    pub host: String,
    /// Port number (typically 993 for IMAPS).
    pub port: u16,
    /// Whether to use TLS (STARTTLS not currently supported).
    pub use_tls: bool,
}

/// SMTP server connection configuration.
///
/// Stores the host, port, and TLS settings needed to connect to an SMTP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmtpServer {
    /// Hostname (e.g. "smtp.office365.com").
    pub host: String,
    /// Port number (typically 587 for STARTTLS, 465 for implicit TLS).
    pub port: u16,
    /// Whether to use TLS.
    pub use_tls: bool,
}

/// Server configuration discovered or manually entered for an account.
///
/// Bundles the IMAP and SMTP server settings together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// IMAP server settings for reading mail.
    pub imap: ImapServer,
    /// SMTP server settings for sending mail.
    pub smtp: SmtpServer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_config_serializes_and_deserializes() {
        let config = ServerConfig {
            imap: ImapServer {
                host: "outlook.office365.com".to_string(),
                port: 993,
                use_tls: true,
            },
            smtp: SmtpServer {
                host: "smtp.office365.com".to_string(),
                port: 587,
                use_tls: true,
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
