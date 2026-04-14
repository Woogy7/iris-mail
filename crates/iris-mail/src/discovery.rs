//! Email server auto-discovery via well-known configurations and DNS lookups.
//!
//! When a user adds an email account, we attempt to automatically determine the
//! correct IMAP and SMTP server settings. This avoids manual configuration for
//! the vast majority of email providers.

use iris_core::{ImapServer, ServerConfig, SmtpServer};

/// Attempts to discover IMAP and SMTP server configuration for an email address.
///
/// Discovery strategy (in order):
/// 1. Well-known provider lookup by domain
/// 2. SRV record lookup (`_imaps._tcp.domain`, `_submission._tcp.domain`)
/// 3. MX record lookup with common IMAP/SMTP hostname inference
pub async fn discover_servers(email_address: &str) -> crate::Result<ServerConfig> {
    let domain = email_address
        .rsplit_once('@')
        .map(|(_, d)| d)
        .ok_or_else(|| crate::Error::Discovery {
            domain: email_address.to_owned(),
            reason: "invalid email address — no @ found".to_owned(),
        })?;

    // 1. Check well-known providers first — fast and reliable.
    if let Some(config) = well_known_config(domain) {
        tracing::info!("using well-known config for domain {domain}");
        return Ok(config);
    }

    // 2. Try SRV records — the proper standards-based approach.
    if let Some(config) = discover_via_srv(domain).await {
        tracing::info!("discovered servers via SRV records for {domain}");
        return Ok(config);
    }

    // 3. Try MX records — heuristic fallback.
    if let Some(config) = discover_via_mx(domain).await {
        tracing::info!("inferred servers from MX records for {domain}");
        return Ok(config);
    }

    Err(crate::Error::Discovery {
        domain: domain.to_owned(),
        reason: "could not auto-discover servers — please enter details manually".to_owned(),
    })
}

/// Returns a pre-configured [`ServerConfig`] for well-known email providers.
///
/// This covers the major providers where DNS-based discovery is unnecessary.
fn well_known_config(domain: &str) -> Option<ServerConfig> {
    let domain_lower = domain.to_ascii_lowercase();
    match domain_lower.as_str() {
        // Microsoft 365 / Outlook
        "outlook.com" | "hotmail.com" | "live.com" | "office365.com" => Some(server_config(
            "outlook.office365.com",
            993,
            "smtp.office365.com",
            587,
        )),

        // Google
        "gmail.com" | "googlemail.com" => {
            Some(server_config("imap.gmail.com", 993, "smtp.gmail.com", 587))
        }

        // Yahoo
        "yahoo.com" | "ymail.com" => Some(server_config(
            "imap.mail.yahoo.com",
            993,
            "smtp.mail.yahoo.com",
            465,
        )),

        // Apple iCloud
        "icloud.com" | "me.com" | "mac.com" => Some(server_config(
            "imap.mail.me.com",
            993,
            "smtp.mail.me.com",
            587,
        )),

        // Fastmail
        "fastmail.com" => Some(server_config(
            "imap.fastmail.com",
            993,
            "smtp.fastmail.com",
            587,
        )),

        _ => None,
    }
}

/// Attempts server discovery via DNS SRV records.
///
/// Queries `_imaps._tcp.{domain}` for the IMAP server and
/// `_submission._tcp.{domain}` for the SMTP server, per RFC 6186.
/// Returns `None` if either lookup fails — we fall through to the next
/// strategy rather than treating DNS failures as hard errors.
async fn discover_via_srv(domain: &str) -> Option<ServerConfig> {
    let resolver = match hickory_resolver::Resolver::builder_tokio() {
        Ok(builder) => builder.build(),
        Err(err) => {
            tracing::warn!("failed to create DNS resolver: {err}");
            return None;
        }
    };

    let imap_name = format!("_imaps._tcp.{domain}.");
    let smtp_name = format!("_submission._tcp.{domain}.");

    let imap_srv = match resolver.srv_lookup(imap_name).await {
        Ok(lookup) => lookup,
        Err(err) => {
            tracing::debug!("SRV lookup for IMAP failed for {domain}: {err}");
            return None;
        }
    };

    let smtp_srv = match resolver.srv_lookup(smtp_name).await {
        Ok(lookup) => lookup,
        Err(err) => {
            tracing::debug!("SRV lookup for SMTP failed for {domain}: {err}");
            return None;
        }
    };

    let imap_record = imap_srv.iter().next()?;
    let smtp_record = smtp_srv.iter().next()?;

    Some(ServerConfig {
        imap: ImapServer {
            host: strip_trailing_dot(&imap_record.target().to_utf8()),
            port: imap_record.port(),
            use_tls: true,
        },
        smtp: SmtpServer {
            host: strip_trailing_dot(&smtp_record.target().to_utf8()),
            port: smtp_record.port(),
            use_tls: true,
        },
    })
}

/// Attempts server discovery by resolving MX records and inferring hostnames.
///
/// This is a heuristic fallback: we look up the domain's MX record to confirm
/// mail service exists, then guess `imap.{domain}` and `smtp.{domain}` as the
/// server hostnames. Not guaranteed to work, but covers many smaller providers.
async fn discover_via_mx(domain: &str) -> Option<ServerConfig> {
    let resolver = match hickory_resolver::Resolver::builder_tokio() {
        Ok(builder) => builder.build(),
        Err(err) => {
            tracing::warn!("failed to create DNS resolver: {err}");
            return None;
        }
    };

    let mx_name = format!("{domain}.");
    let mx_lookup = match resolver.mx_lookup(mx_name).await {
        Ok(lookup) => lookup,
        Err(err) => {
            tracing::debug!("MX lookup failed for {domain}: {err}");
            return None;
        }
    };

    // We only need to confirm MX records exist — the actual MX hostname
    // is for SMTP relay, not for IMAP/submission access. We infer the
    // user-facing server names from the domain instead.
    mx_lookup.iter().next()?;

    Some(server_config(
        &format!("imap.{domain}"),
        993,
        &format!("smtp.{domain}"),
        587,
    ))
}

/// Builds a [`ServerConfig`] with TLS enabled on both servers.
fn server_config(imap_host: &str, imap_port: u16, smtp_host: &str, smtp_port: u16) -> ServerConfig {
    ServerConfig {
        imap: ImapServer {
            host: imap_host.to_owned(),
            port: imap_port,
            use_tls: true,
        },
        smtp: SmtpServer {
            host: smtp_host.to_owned(),
            port: smtp_port,
            use_tls: true,
        },
    }
}

/// Strips the trailing dot from a fully-qualified DNS name.
///
/// DNS names like `imap.gmail.com.` need the trailing dot removed before
/// being used as hostnames in TLS connections.
fn strip_trailing_dot(name: &str) -> String {
    name.strip_suffix('.').unwrap_or(name).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_config_returns_m365_for_outlook() {
        let config = well_known_config("outlook.com");
        assert!(config.is_some());
        let config = config.expect("already checked is_some");
        assert_eq!(config.imap.host, "outlook.office365.com");
        assert_eq!(config.imap.port, 993);
        assert!(config.imap.use_tls);
    }

    #[test]
    fn well_known_config_returns_gmail_settings() {
        let config = well_known_config("gmail.com");
        assert!(config.is_some());
        let config = config.expect("already checked is_some");
        assert_eq!(config.imap.host, "imap.gmail.com");
        assert_eq!(config.imap.port, 993);
        assert_eq!(config.smtp.host, "smtp.gmail.com");
        assert_eq!(config.smtp.port, 587);
    }

    #[test]
    fn well_known_config_returns_yahoo_settings() {
        let config = well_known_config("yahoo.com");
        assert!(config.is_some());
        let config = config.expect("already checked is_some");
        assert_eq!(config.imap.host, "imap.mail.yahoo.com");
        assert_eq!(config.smtp.port, 465);
    }

    #[test]
    fn well_known_config_returns_icloud_settings() {
        let config = well_known_config("icloud.com");
        assert!(config.is_some());
        let config = config.expect("already checked is_some");
        assert_eq!(config.imap.host, "imap.mail.me.com");
        assert_eq!(config.smtp.host, "smtp.mail.me.com");
    }

    #[test]
    fn well_known_config_returns_fastmail_settings() {
        let config = well_known_config("fastmail.com");
        assert!(config.is_some());
        let config = config.expect("already checked is_some");
        assert_eq!(config.imap.host, "imap.fastmail.com");
        assert_eq!(config.smtp.host, "smtp.fastmail.com");
    }

    #[test]
    fn well_known_config_returns_none_for_unknown_domain() {
        assert!(well_known_config("randomdomain123.com").is_none());
    }

    #[test]
    fn well_known_config_is_case_insensitive() {
        assert!(well_known_config("Gmail.COM").is_some());
        assert!(well_known_config("Outlook.Com").is_some());
    }

    #[test]
    fn strip_trailing_dot_removes_fqdn_dot() {
        assert_eq!(strip_trailing_dot("imap.gmail.com."), "imap.gmail.com");
    }

    #[test]
    fn strip_trailing_dot_preserves_names_without_dot() {
        assert_eq!(strip_trailing_dot("imap.gmail.com"), "imap.gmail.com");
    }

    #[tokio::test]
    async fn discover_servers_finds_m365_for_outlook_address() {
        let config = discover_servers("test@outlook.com").await.unwrap();
        assert_eq!(config.imap.host, "outlook.office365.com");
        assert_eq!(config.smtp.host, "smtp.office365.com");
    }

    #[tokio::test]
    async fn discover_servers_rejects_invalid_email() {
        let result = discover_servers("not-an-email").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn discover_servers_rejects_empty_string() {
        let result = discover_servers("").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::Error::Discovery { reason, .. } => {
                assert!(
                    reason.contains("no @"),
                    "error should mention missing @: {reason}"
                );
            }
            other => panic!("expected Discovery error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn discover_servers_rejects_only_at_sign() {
        let result = discover_servers("@").await;
        // Domain after @ is empty, which should fail (either well-known miss
        // or DNS errors), but should not panic.
        assert!(result.is_err());
    }

    #[test]
    fn well_known_config_returns_m365_for_hotmail() {
        let config = well_known_config("hotmail.com");
        assert!(config.is_some());
        assert_eq!(config.unwrap().imap.host, "outlook.office365.com");
    }

    #[test]
    fn well_known_config_returns_m365_for_live() {
        let config = well_known_config("live.com");
        assert!(config.is_some());
        assert_eq!(config.unwrap().smtp.host, "smtp.office365.com");
    }

    #[test]
    fn well_known_config_returns_icloud_for_me_com() {
        let config = well_known_config("me.com");
        assert!(config.is_some());
        assert_eq!(config.unwrap().imap.host, "imap.mail.me.com");
    }

    #[test]
    fn well_known_config_returns_icloud_for_mac_com() {
        let config = well_known_config("mac.com");
        assert!(config.is_some());
        assert_eq!(config.unwrap().smtp.host, "smtp.mail.me.com");
    }

    #[test]
    fn well_known_config_returns_gmail_for_googlemail() {
        let config = well_known_config("googlemail.com");
        assert!(config.is_some());
        assert_eq!(config.unwrap().imap.host, "imap.gmail.com");
    }

    #[test]
    fn well_known_config_returns_yahoo_for_ymail() {
        let config = well_known_config("ymail.com");
        assert!(config.is_some());
        assert_eq!(config.unwrap().imap.host, "imap.mail.yahoo.com");
    }

    #[test]
    fn server_config_helper_sets_tls_on_both_servers() {
        let config = server_config("imap.test.com", 993, "smtp.test.com", 587);
        assert!(config.imap.use_tls);
        assert!(config.smtp.use_tls);
    }

    #[test]
    fn strip_trailing_dot_handles_empty_string() {
        assert_eq!(strip_trailing_dot(""), "");
    }

    #[test]
    fn strip_trailing_dot_handles_single_dot() {
        assert_eq!(strip_trailing_dot("."), "");
    }

    #[tokio::test]
    async fn discover_servers_finds_gmail_for_gmail_address() {
        let config = discover_servers("user@gmail.com").await.unwrap();
        assert_eq!(config.imap.host, "imap.gmail.com");
        assert_eq!(config.smtp.host, "smtp.gmail.com");
    }
}
