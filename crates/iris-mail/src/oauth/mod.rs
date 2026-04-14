//! OAuth2 authentication flows and credential storage.

pub mod gmail;
pub mod keychain;
pub mod m365;

use serde::{Deserialize, Serialize};

/// Tokens received from an OAuth2 provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OauthTokens {
    /// The short-lived access token for API calls.
    pub access_token: String,
    /// The long-lived refresh token for obtaining new access tokens.
    pub refresh_token: String,
    /// When the access token expires, if known.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl OauthTokens {
    /// Returns `true` if the access token has expired or will expire within 60 seconds.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => chrono::Utc::now() + chrono::TimeDelta::seconds(60) >= exp,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_token_is_detected() {
        let tokens = OauthTokens {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: Some(chrono::Utc::now() - chrono::TimeDelta::seconds(10)),
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn future_token_is_not_expired() {
        let tokens = OauthTokens {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: Some(chrono::Utc::now() + chrono::TimeDelta::seconds(3600)),
        };
        assert!(!tokens.is_expired());
    }

    #[test]
    fn token_without_expiry_is_treated_as_expired() {
        let tokens = OauthTokens {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: None,
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn token_expiring_within_buffer_window_is_treated_as_expired() {
        // A token expiring in 30 seconds should be "expired" because the
        // 60-second buffer protects against using a token that will expire
        // during a request.
        let tokens = OauthTokens {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: Some(chrono::Utc::now() + chrono::TimeDelta::seconds(30)),
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn token_expiring_exactly_at_buffer_boundary_is_treated_as_expired() {
        // At exactly 60 seconds the >= comparison should make it expired.
        let tokens = OauthTokens {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: Some(chrono::Utc::now() + chrono::TimeDelta::seconds(60)),
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn token_just_beyond_buffer_window_is_not_expired() {
        // A token expiring in 120 seconds should be considered valid.
        let tokens = OauthTokens {
            access_token: "test".to_string(),
            refresh_token: "test".to_string(),
            expires_at: Some(chrono::Utc::now() + chrono::TimeDelta::seconds(120)),
        };
        assert!(!tokens.is_expired());
    }

    #[test]
    fn oauth_tokens_serialise_and_deserialise_via_json() {
        let tokens = OauthTokens {
            access_token: "access_123".to_string(),
            refresh_token: "refresh_456".to_string(),
            expires_at: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&tokens).expect("serialize");
        let recovered: OauthTokens = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(tokens.access_token, recovered.access_token);
        assert_eq!(tokens.refresh_token, recovered.refresh_token);
    }
}
