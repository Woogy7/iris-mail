//! Microsoft Graph REST client.
//!
//! Provides an authenticated HTTP client that handles bearer token injection
//! and error mapping for all Graph API calls.

/// Base URL for Microsoft Graph API v1.0 endpoints.
const GRAPH_BASE_URL: &str = "https://graph.microsoft.com/v1.0";

/// An authenticated Microsoft Graph API client.
///
/// Wraps a `reqwest::Client` with a bearer token. Create one per access token
/// and share it across folder and message operations for the same account.
pub struct GraphClient {
    pub(crate) http: reqwest::Client,
    pub(crate) access_token: String,
}

impl GraphClient {
    /// Creates a new Graph client with the given access token.
    pub fn new(access_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            access_token,
        }
    }

    /// Sends an authenticated GET request to a Graph API path.
    ///
    /// The `path` is appended to the Graph base URL (e.g. `"/me/mailFolders"`).
    pub(crate) async fn get(&self, path: &str) -> crate::Result<reqwest::Response> {
        let url = format!("{GRAPH_BASE_URL}{path}");
        self.get_url(&url).await
    }

    /// Sends an authenticated GET request to an absolute URL.
    ///
    /// Used for pagination when following `@odata.nextLink` URLs.
    pub(crate) async fn get_url(&self, url: &str) -> crate::Result<reqwest::Response> {
        let resp = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| crate::Error::Graph(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_else(|_| "no body".to_string());
            return Err(crate::Error::Graph(format!("HTTP {status}: {body}")));
        }

        Ok(resp)
    }
}
