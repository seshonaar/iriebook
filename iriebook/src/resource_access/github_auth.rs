//! GitHub Device Flow authentication
//!
//! Implements OAuth Device Flow for GitHub authentication

use crate::utilities::error::IrieBookError;
use serde::{Deserialize, Serialize};

// GitHub OAuth Credentials (Base64 encoded JSON)
//
// This is injected by the embed_credentials tool.
// DO NOT EDIT MANUALLY if you want to use the tool.
const GITHUB_CREDENTIALS_B64: &str = "eyJpbnN0YWxsZWQiOnsiY2xpZW50X2lkIjoiT3YyM2xpM2czQ3p2bWFiV2V6TVQiLCJwcm9qZWN0X2lkIjoiaXJpZWJvb2stbGlicmFyeSIsImF1dGhfdXJpIjoiaHR0cHM6Ly9naXRodWIuY29tL2xvZ2luL2RldmljZS9jb2RlIiwidG9rZW5fdXJpIjoiaHR0cHM6Ly9naXRodWIuY29tL2xvZ2luL2RldmljZS9jb2RlIiwiYXV0aF9wcm92aWRlcl94NTA5X2NlcnRfdXJsIjoiIiwiY2xpZW50X3NlY3JldCI6IiIsInJlZGlyZWN0X3VyaXMiOlsiaHR0cDovL2xvY2FsaG9zdCJdfX0K";

/// GitHub Device Flow data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFlowData {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Result of polling for device flow token
#[derive(Debug, Clone)]
pub enum PollResult {
    Pending,
    Success(String), // OAuth token
    Expired,
    Denied,
}

/// GitHub API response for device flow initiation
#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

/// GitHub API response for token polling
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

/// GitHub authenticator using Device Flow
pub struct GitHubAuthenticator {
    client: reqwest::Client,
}

impl GitHubAuthenticator {
    /// Create a new GitHub authenticator
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn get_client_id(&self) -> Result<String, IrieBookError> {
        // First check environment variable
        if let Ok(client_id) = std::env::var("GITHUB_CLIENT_ID") {
            return Ok(client_id);
        }

        // Then check embedded credentials
        if !GITHUB_CREDENTIALS_B64.is_empty() {
             let config = crate::resource_access::embedded_config::decode_config(GITHUB_CREDENTIALS_B64)
                .map_err(|e| IrieBookError::GitHubAuth(e.to_string()))?;
             return Ok(config.client_id);
        }
        
        Err(IrieBookError::GitHubAuth("Failed to parse github authentication config".to_string()))
    }

    /// Initiate device flow and get user code
    ///
    /// # Returns
    /// * `Ok(DeviceFlowData)` with device code, user code, and verification URI
    /// * `Err(IrieBookError)` if the request fails
    pub async fn initiate_device_flow(&self) -> Result<DeviceFlowData, IrieBookError> {
        let client_id = self.get_client_id()?;
        let params = [
            ("client_id", client_id.as_str()),
            ("scope", "repo"), // Request full repository access
        ];

        let response = self
            .client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| IrieBookError::Network(format!("Failed to initiate device flow: {}", e)))?;

        if !response.status().is_success() {
            return Err(IrieBookError::GitHubAuth(format!(
                "GitHub returned status: {}",
                response.status()
            )));
        }

        let device_code_response: DeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| IrieBookError::GitHubAuth(format!("Failed to parse response: {}", e)))?;

        Ok(DeviceFlowData {
            device_code: device_code_response.device_code,
            user_code: device_code_response.user_code,
            verification_uri: device_code_response.verification_uri,
            expires_in: device_code_response.expires_in,
            interval: device_code_response.interval,
        })
    }

    /// Poll for device flow token
    ///
    /// # Arguments
    /// * `device_code` - The device code from initiate_device_flow
    ///
    /// # Returns
    /// * `Ok(PollResult::Success(token))` if user authorized
    /// * `Ok(PollResult::Pending)` if authorization pending
    /// * `Ok(PollResult::Expired)` if device code expired
    /// * `Ok(PollResult::Denied)` if user denied authorization
    /// * `Err(IrieBookError)` if the request fails
    pub async fn poll_for_token(&self, device_code: &str) -> Result<PollResult, IrieBookError> {
        let client_id = self.get_client_id()?;
        let params = [
            ("client_id", client_id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let response = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| IrieBookError::Network(format!("Failed to poll for token: {}", e)))?;

        if !response.status().is_success() {
            return Err(IrieBookError::GitHubAuth(format!(
                "GitHub returned status: {}",
                response.status()
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| IrieBookError::GitHubAuth(format!("Failed to parse response: {}", e)))?;

        match (token_response.access_token, token_response.error.as_deref()) {
            (Some(token), _) => Ok(PollResult::Success(token)),
            (None, Some("authorization_pending")) => Ok(PollResult::Pending),
            (None, Some("slow_down")) => Ok(PollResult::Pending), // Caller should increase interval
            (None, Some("expired_token")) => Ok(PollResult::Expired),
            (None, Some("access_denied")) => Ok(PollResult::Denied),
            (None, Some(error)) => Err(IrieBookError::GitHubAuth(format!(
                "Unknown error: {}",
                error
            ))),
            (None, None) => Err(IrieBookError::GitHubAuth(
                "Invalid response from GitHub".to_string(),
            )),
        }
    }
}

impl Default for GitHubAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_flow_data_can_be_created() {
        let data = DeviceFlowData {
            device_code: "test_device_code".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            expires_in: 900,
            interval: 5,
        };

        assert_eq!(data.device_code, "test_device_code");
        assert_eq!(data.user_code, "ABCD-1234");
    }

    #[test]
    fn authenticator_can_be_created() {
        let _auth = GitHubAuthenticator::new();
        // Just testing that construction works
    }

    // Note: Integration tests with real GitHub API would go in tests/ directory
    // Unit tests here focus on data structures and logic
}
