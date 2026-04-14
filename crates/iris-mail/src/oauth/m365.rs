//! Microsoft 365 OAuth2 PKCE flow.
//!
//! Handles the browser-based consent flow and token exchange for M365 accounts.
//! Uses the Authorization Code flow with PKCE (RFC 7636) so that no client
//! secret is required on the desktop.

use super::OauthTokens;
use base64::Engine;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

/// Microsoft identity platform v2.0 authorization endpoint.
const AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";

/// Microsoft identity platform v2.0 token endpoint.
const TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";

/// Scopes required for Microsoft Graph API access.
const SCOPES: &[&str] = &[
    "https://graph.microsoft.com/Mail.Read",
    "https://graph.microsoft.com/Mail.ReadWrite",
    "https://graph.microsoft.com/Mail.Send",
    "offline_access",
];

/// Length in bytes for the PKCE code verifier (before base64 encoding).
const PKCE_VERIFIER_LENGTH: usize = 32;

/// The HTML page returned to the user's browser after a successful auth redirect.
const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html><head><title>Iris Mail</title></head>
<body><h2>Authentication complete</h2><p>You can close this tab.</p></body>
</html>"#;

/// Token response from the Microsoft identity platform.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

/// Starts the M365 OAuth2 PKCE flow.
///
/// Opens the system browser to the Microsoft login page, waits for the redirect
/// on `localhost:{redirect_port}`, exchanges the authorization code for tokens,
/// and returns the [`OauthTokens`].
///
/// The `email_hint` is passed as `login_hint` to Microsoft so the correct
/// account is pre-selected in the consent dialog.
pub async fn start_m365_oauth(
    client_id: &str,
    redirect_port: u16,
    email_hint: &str,
) -> crate::Result<OauthTokens> {
    let redirect_uri = format!("http://localhost:{redirect_port}");

    // Generate PKCE verifier and challenge.
    let verifier = generate_pkce_verifier();
    let challenge = generate_pkce_challenge(&verifier);

    // Build the authorization URL.
    let auth_url = build_auth_url(client_id, &redirect_uri, &challenge, email_hint)?;

    // Bind the local listener *before* opening the browser so we never miss the redirect.
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{redirect_port}"))
        .await
        .map_err(|e| crate::Error::Oauth(format!("failed to bind redirect listener: {e}")))?;

    // Open the authorization URL in the user's default browser.
    open::that(auth_url.as_str())
        .map_err(|e| crate::Error::Oauth(format!("failed to open browser: {e}")))?;

    // Wait for the redirect and extract the authorization code.
    let code = accept_redirect(&listener).await?;

    // Exchange the authorization code for tokens.
    exchange_code(client_id, &code, &redirect_uri, &verifier).await
}

/// Refreshes an M365 access token using the stored refresh token.
pub async fn refresh_m365_token(
    client_id: &str,
    refresh_token: &str,
) -> crate::Result<OauthTokens> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| crate::Error::TokenRefreshFailed(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "no response body".to_string());
        return Err(crate::Error::TokenRefreshFailed(format!(
            "token endpoint returned error: {body}"
        )));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| crate::Error::TokenRefreshFailed(format!("invalid token response: {e}")))?;

    Ok(token_response_to_tokens(token_resp, refresh_token))
}

/// Generates a cryptographically random PKCE code verifier (RFC 7636).
fn generate_pkce_verifier() -> String {
    use rand::Rng;
    let mut bytes = [0u8; PKCE_VERIFIER_LENGTH];
    rand::rng().fill(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Derives the S256 PKCE code challenge from a verifier.
fn generate_pkce_challenge(verifier: &str) -> String {
    use base64::Engine;
    let digest = <sha2::Sha256 as sha2::Digest>::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Constructs the full authorization URL with PKCE, scopes, and login hint.
fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    email_hint: &str,
) -> crate::Result<Url> {
    let scope = SCOPES.join(" ");
    Url::parse_with_params(
        AUTH_URL,
        &[
            ("client_id", client_id),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri),
            ("response_mode", "query"),
            ("scope", &scope),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("login_hint", email_hint),
            ("prompt", "select_account"),
        ],
    )
    .map_err(|e| crate::Error::Oauth(format!("failed to build auth URL: {e}")))
}

/// Accepts the first HTTP connection on `listener`, extracts the `code` query
/// parameter from the GET request, and sends back a success page.
async fn accept_redirect(listener: &tokio::net::TcpListener) -> crate::Result<String> {
    let (mut stream, _addr) = listener
        .accept()
        .await
        .map_err(|e| crate::Error::Oauth(format!("failed to accept redirect connection: {e}")))?;

    // Read enough of the HTTP request to get the first line.
    let mut buf = vec![0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| crate::Error::Oauth(format!("failed to read redirect request: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the request path from "GET /path?query HTTP/1.1".
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| crate::Error::Oauth("malformed HTTP request on redirect".to_string()))?;

    // Parse query parameters.
    let dummy_base = Url::parse("http://localhost").expect("static URL parses");
    let full_url = dummy_base
        .join(path)
        .map_err(|e| crate::Error::Oauth(format!("failed to parse redirect path: {e}")))?;

    // Check for an error parameter first.
    if let Some(error) = full_url
        .query_pairs()
        .find(|(k, _)| k == "error")
        .map(|(_, v)| v.to_string())
    {
        let description = full_url
            .query_pairs()
            .find(|(k, _)| k == "error_description")
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();
        send_response(&mut stream, "Authentication failed").await;
        return Err(crate::Error::Oauth(format!("{error}: {description}")));
    }

    let code = full_url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| {
            crate::Error::Oauth("redirect did not contain an authorization code".to_string())
        })?;

    send_response(&mut stream, SUCCESS_HTML).await;
    Ok(code)
}

/// Sends a minimal HTTP 200 response with the given HTML body.
async fn send_response(stream: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body,
    );
    // Best-effort write; the browser may have already disconnected.
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

/// Exchanges an authorization code for tokens at the Microsoft token endpoint.
async fn exchange_code(
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> crate::Result<OauthTokens> {
    let scope = SCOPES.join(" ");
    let params = [
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
        ("scope", &scope),
    ];

    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| crate::Error::Oauth(format!("token exchange request failed: {e}")))?;

    if !resp.status().is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "no response body".to_string());
        return Err(crate::Error::Oauth(format!(
            "token endpoint returned error: {body}"
        )));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| crate::Error::Oauth(format!("invalid token response: {e}")))?;

    Ok(token_response_to_tokens(token_resp, ""))
}

/// Converts a raw [`TokenResponse`] into [`OauthTokens`], computing the
/// absolute expiry time from the relative `expires_in` field.
fn token_response_to_tokens(resp: TokenResponse, fallback_refresh: &str) -> OauthTokens {
    let expires_at = resp
        .expires_in
        .map(|secs| chrono::Utc::now() + chrono::TimeDelta::seconds(secs as i64));

    OauthTokens {
        access_token: resp.access_token,
        refresh_token: resp
            .refresh_token
            .unwrap_or_else(|| fallback_refresh.to_string()),
        expires_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_has_correct_length() {
        let verifier = generate_pkce_verifier();
        // 32 bytes base64url-encoded (no padding) = 43 chars.
        assert_eq!(verifier.len(), 43);
    }

    #[test]
    fn pkce_verifier_is_url_safe_base64() {
        let verifier = generate_pkce_verifier();
        // URL-safe base64 only uses [A-Za-z0-9_-].
        assert!(
            verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "verifier contains non-url-safe chars: {verifier}"
        );
    }

    #[test]
    fn pkce_verifier_generates_unique_values() {
        let a = generate_pkce_verifier();
        let b = generate_pkce_verifier();
        assert_ne!(a, b, "two verifiers should differ");
    }

    #[test]
    fn pkce_challenge_is_sha256_of_verifier() {
        // Use a known verifier to produce a deterministic challenge.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = generate_pkce_challenge(verifier);
        // The expected challenge is the base64url-encoded SHA-256 of the verifier.
        let expected = {
            use sha2::Digest;
            let digest = sha2::Sha256::digest(verifier.as_bytes());
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
        };
        assert_eq!(challenge, expected);
    }

    #[test]
    fn build_auth_url_contains_all_required_params() {
        let url = build_auth_url(
            "test-client-id",
            "http://localhost:8080",
            "test-challenge",
            "user@example.com",
        )
        .expect("should build URL");
        let url_str = url.as_str();
        assert!(url_str.contains("client_id=test-client-id"));
        assert!(url_str.contains("response_type=code"));
        assert!(url_str.contains("code_challenge=test-challenge"));
        assert!(url_str.contains("code_challenge_method=S256"));
        assert!(url_str.contains("Mail.Read"));
        assert!(url_str.contains("login_hint=user%40example.com"));
        assert!(url_str.contains("prompt=select_account"));
    }

    #[test]
    fn token_response_to_tokens_uses_response_refresh_token() {
        let resp = TokenResponse {
            access_token: "access_1".to_string(),
            refresh_token: Some("refresh_1".to_string()),
            expires_in: Some(3600),
        };

        let tokens = token_response_to_tokens(resp, "fallback");
        assert_eq!(tokens.access_token, "access_1");
        assert_eq!(tokens.refresh_token, "refresh_1");
        assert!(tokens.expires_at.is_some());
    }

    #[test]
    fn token_response_to_tokens_uses_fallback_when_no_refresh_token() {
        let resp = TokenResponse {
            access_token: "access_2".to_string(),
            refresh_token: None,
            expires_in: None,
        };

        let tokens = token_response_to_tokens(resp, "existing_refresh");
        assert_eq!(tokens.refresh_token, "existing_refresh");
        assert!(tokens.expires_at.is_none());
    }

    #[test]
    fn token_response_to_tokens_computes_expiry_from_now() {
        let before = chrono::Utc::now();
        let resp = TokenResponse {
            access_token: "a".to_string(),
            refresh_token: Some("r".to_string()),
            expires_in: Some(3600),
        };
        let tokens = token_response_to_tokens(resp, "");
        let after = chrono::Utc::now();

        let expires_at = tokens.expires_at.expect("should have expiry");
        let earliest = before + chrono::TimeDelta::seconds(3600);
        let latest = after + chrono::TimeDelta::seconds(3600);

        assert!(
            expires_at >= earliest && expires_at <= latest,
            "expires_at should be approximately now + 3600s"
        );
    }
}
