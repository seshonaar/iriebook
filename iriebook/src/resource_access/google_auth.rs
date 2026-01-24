//! Google OAuth Authorization Code Flow authentication
//!
//! Implements OAuth 2.0 Authorization Code Flow for Google authentication
//! with a local loopback server for token reception.

use crate::resource_access::CredentialStore;
use crate::utilities::error::IrieBookError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

// Google OAuth Credentials (Base64 encoded JSON)
//
// This is injected by the embed_credentials tool.
// DO NOT EDIT MANUALLY if you want to use the tool.
const GOOGLE_CREDENTIALS_B64: &str = "eyJpbnN0YWxsZWQiOnsiY2xpZW50X2lkIjoiNjM1NjYxNDM3MDY4LWVsZDA2dDVqN2txZnJkdWJla3M1OXJuYjZnaGRoaDUxLmFwcHMuZ29vZ2xldXNlcmNvbnRlbnQuY29tIiwicHJvamVjdF9pZCI6ImlyaWVib29rLWxpYnJhcnkiLCJhdXRoX3VyaSI6Imh0dHBzOi8vYWNjb3VudHMuZ29vZ2xlYXBpcy5jb20vbS9vYXV0aDIvdjEvY2VydHMiLCJ0b2tlbl91cmkiOiJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6Imh0dHBzOi8vd3d3Lmdvb2dsZWFwaXMuY29tL29hdXRoMi92MS9jZXJ0cyIsImNsaWVudF9zZWNyZXQiOiJHT0NTUFgtd2oweVdsMzNpU21WaWdIaVJsQ1JqZFY5U0xTcyIsInJlZGlyZWN0X3VyaXMiOlsiaHR0cDovL2xvY2FsaG9zdCJdfX0=";

/// Token response from Authorization Code exchange
#[derive(Debug, Deserialize)]
pub struct AuthCodeTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: String,
}

/// Token response from Refresh Token exchange
#[derive(Debug, Deserialize)]
struct RefreshTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: String,
}

/// Stored Google Credentials
///
/// Contains the access token, refresh token (if available), and expiry time.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredGoogleCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64, // Unix timestamp (seconds)
}

/// Represents an active authorization flow session.
///
/// Holds the local listener and the configuration used to generate the auth URL.
pub struct AuthFlow {
    pub url: String,
    pub redirect_uri: String,
    listener: TcpListener,
}

impl AuthFlow {
    /// Waits for the browser to redirect to the local server with the code.
    ///
    /// This method accepts a single connection, parses the request for the `code` parameter,
    /// sends a success response to the browser, and returns the code.
    pub async fn wait_for_code(self) -> Result<String, IrieBookError> {
        let (mut socket, _) = self.listener.accept().await.map_err(|e| {
            IrieBookError::Network(format!("Failed to accept connection: {}", e))
        })?;

        let mut buffer = [0; 2048];
        let n = socket.read(&mut buffer).await.map_err(|e| {
            IrieBookError::Network(format!("Failed to read request: {}", e))
        })?;

        if n == 0 {
            return Err(IrieBookError::GoogleAuth("Empty request received".to_string()));
        }

        let request = String::from_utf8_lossy(&buffer[..n]);

        // Simple parsing to extract "code" query param
        // Look for "GET /?code=... " or "GET /callback?code=..."
        
        let code = request
            .lines()
            .next()
            .and_then(|line| {
                let start = line.find("code=")?;
                let rest = &line[start + 5..];
                // End at space (HTTP/1.1) or next query param (&)
                let end = rest.find(['&', ' ']).unwrap_or(rest.len());
                Some(rest[..end].to_string())
            })
            .ok_or_else(|| IrieBookError::GoogleAuth("No authorization code found in request".to_string()))?;

        // Send response
        let response_body = "<html><body><h1>Authorization Successful!</h1><p>You can close this window and return to the application.</p><script>window.close()</script></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}\r\n",
            response_body.len(),
            response_body
        );

        socket.write_all(response.as_bytes()).await.ok();
        socket.flush().await.ok();
        
        // Ensure we properly unescape the code (URL decoding)
        let decoded_code = urlencoding::decode(&code)
            .map_err(|e| IrieBookError::GoogleAuth(format!("Failed to decode code: {}", e)))?;

        Ok(decoded_code.into_owned())
    }
}

/// Google authenticator using Authorization Code Flow
pub struct GoogleAuthenticator {
    client: reqwest::Client,
}

impl GoogleAuthenticator {
    /// Create a new Google authenticator
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn get_credentials(&self) -> Result<(String, Option<String>), IrieBookError> {
        // First check environment variable
        if let Ok(client_id) = std::env::var("GOOGLE_CLIENT_ID") {
            let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").ok();
            return Ok((client_id, client_secret));
        }

        // Then check embedded credentials
        if !GOOGLE_CREDENTIALS_B64.is_empty() {
             let config = crate::resource_access::embedded_config::decode_config(GOOGLE_CREDENTIALS_B64)
                .map_err(|e| IrieBookError::GoogleAuth(e.to_string()))?;
             return Ok((config.client_id, config.client_secret));
        }

        Err(IrieBookError::GoogleAuth("No Google Client ID found. Please set GOOGLE_CLIENT_ID env var or embed credentials.".to_string()))
    }

    fn get_oauth_endpoints(&self) -> (String, String) {
        let auth_url = std::env::var("GOOGLE_OAUTH_AUTH_URL")
            .unwrap_or_else(|_| "https://accounts.google.com/o/oauth2/v2/auth".to_string());
        let token_url = std::env::var("GOOGLE_OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| "https://oauth2.googleapis.com/token".to_string());
        (auth_url, token_url)
    }

    /// Prepares the authorization flow by binding a local port and generating the URL.
    ///
    /// # Returns
    /// * `Ok(AuthFlow)` containing the URL to open, the redirect URI, and the listener.
    pub async fn prepare_auth_flow(&self) -> Result<AuthFlow, IrieBookError> {
        let (client_id, _) = self.get_credentials()?;
        
        // Bind to localhost on a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            IrieBookError::Network(format!("Failed to bind local listener: {}", e))
        })?;

        let port = listener.local_addr().map_err(|e| {
            IrieBookError::Network(format!("Failed to get local address: {}", e))
        })?.port();

        // Use IP literal instead of localhost to avoid DNS issues and ambiguity
        let redirect_uri = format!("http://127.0.0.1:{}", port);

        // Construct Authorization URL
        let (auth_url, _) = self.get_oauth_endpoints();
        let mut url = Url::parse(&auth_url)
            .map_err(|e| IrieBookError::GoogleAuth(format!("Invalid base URL: {}", e)))?;

        url.query_pairs_mut()
            .append_pair("client_id", &client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "https://www.googleapis.com/auth/documents.readonly https://www.googleapis.com/auth/drive.readonly")
            .append_pair("access_type", "offline") // Crucial for getting refresh token
            .append_pair("prompt", "consent"); // Force consent to ensure refresh token is returned

        Ok(AuthFlow {
            url: url.to_string(),
            redirect_uri,
            listener,
        })
    }

    /// Exchanges the authorization code for tokens.
    ///
    /// # Arguments
    /// * `code` - The authorization code received from the callback.
    /// * `redirect_uri` - The redirect URI used in the initial request (must match exactly).
    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<AuthCodeTokenResponse, IrieBookError> {
        let (client_id, client_secret) = self.get_credentials()?;
        
        let mut params = vec![
            ("client_id", client_id.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];

        if let Some(secret) = &client_secret {
            params.push(("client_secret", secret.as_str()));
        }

        let (_, token_url) = self.get_oauth_endpoints();
        let response = self
            .client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| IrieBookError::Network(format!("Failed to exchange code: {}", e)))?;

        if !response.status().is_success() {
             let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
             return Err(IrieBookError::GoogleAuth(format!(
                "Google returned error during token exchange: {}",
                error_text
            )));
        }

        let token_response: AuthCodeTokenResponse = response
            .json()
            .await
            .map_err(|e| IrieBookError::GoogleAuth(format!("Failed to parse token response: {}", e)))?;

        Ok(token_response)
    }

    /// Refresh access token using refresh token
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<AuthCodeTokenResponse, IrieBookError> {
        let (client_id, client_secret) = self.get_credentials()?;
        
        let mut params = vec![
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        if let Some(secret) = &client_secret {
            params.push(("client_secret", secret.as_str()));
        }

        let (_, token_url) = self.get_oauth_endpoints();
        let response = self
            .client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| IrieBookError::Network(format!("Failed to refresh token: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
             return Err(IrieBookError::GoogleAuth(format!(
                "Google returned error during token refresh: {}",
                error_text
            )));
        }

        let refresh_response: RefreshTokenResponse = response
            .json()
            .await
            .map_err(|e| IrieBookError::GoogleAuth(format!("Failed to parse refresh response: {}", e)))?;

        // Construct AuthCodeTokenResponse from refresh response (re-using struct)
        // Refresh response usually doesn't include a new refresh token, so we keep the old one.
        // We'll return None here and let the caller merge it.
        Ok(AuthCodeTokenResponse {
            access_token: refresh_response.access_token,
            expires_in: refresh_response.expires_in,
            refresh_token: None, // Will be filled by caller from stored credentials
            token_type: refresh_response.token_type,
            scope: refresh_response.scope,
        })
    }

    /// Retrieve a valid access token, refreshing if necessary.
    /// 
    /// Reads from CredentialStore, checks expiry, refreshes if expired, updates CredentialStore,
    /// and returns the valid access token.
    pub async fn get_valid_token(&self) -> Result<String, IrieBookError> {
        #[cfg(feature = "e2e-mocks")]
        {
            use tracing::warn;
            warn!("🔍 [E2E-AUTH] get_valid_token called");
        }

        let stored_json = match CredentialStore::retrieve_google_token() {
            Ok(json) => {
                #[cfg(feature = "e2e-mocks")]
                {
                    use tracing::warn;
                    warn!("✅ [E2E-AUTH] Found stored credentials, length: {}", json.len());
                }
                json
            }
            Err(e) => {
                #[cfg(feature = "e2e-mocks")]
                {
                    use tracing::warn;
                    warn!("❌ [E2E-AUTH] No credentials found: {}", e);
                }
                return Err(e);
            }
        };

        let mut credentials: StoredGoogleCredentials = serde_json::from_str(&stored_json)
            .map_err(|e| IrieBookError::GoogleAuth(format!("Failed to parse stored credentials: {}", e)))?;

        let now = Utc::now().timestamp();
        // Check if expired (with 5 minute buffer)
        if now < credentials.expires_at - 300 {
            return Ok(credentials.access_token);
        }

        // Token expired, try to refresh
        if let Some(refresh_token) = &credentials.refresh_token {
            let new_tokens = self.refresh_access_token(refresh_token).await?;
            
            // Update credentials
            credentials.access_token = new_tokens.access_token;
            credentials.expires_at = now + new_tokens.expires_in as i64;
            // Note: Refresh token usually stays the same, but if a new one is returned, use it.
            if let Some(new_rt) = new_tokens.refresh_token {
                credentials.refresh_token = Some(new_rt);
            }

            // Save updated credentials
            let new_json = serde_json::to_string(&credentials)
                .map_err(|e| IrieBookError::GoogleAuth(format!("Failed to serialize credentials: {}", e)))?;
            
            CredentialStore::store_google_token(&new_json)?;

            Ok(credentials.access_token)
        } else {
            Err(IrieBookError::GoogleAuth("Access token expired and no refresh token available. Please re-authenticate.".to_string()))
        }
    }

    /// Save tokens to CredentialStore
    pub fn save_tokens(&self, tokens: &AuthCodeTokenResponse) -> Result<(), IrieBookError> {
        let now = Utc::now().timestamp();
        let credentials = StoredGoogleCredentials {
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone(),
            expires_at: now + tokens.expires_in as i64,
        };

        let json = serde_json::to_string(&credentials)
            .map_err(|e| IrieBookError::GoogleAuth(format!("Failed to serialize credentials: {}", e)))?;

        CredentialStore::store_google_token(&json)
    }
}

impl Default for GoogleAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::resource_access::traits::TokenProvider for GoogleAuthenticator {
    async fn get_valid_token(&self) -> Result<String, IrieBookError> {
        self.get_valid_token().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticator_can_be_created() {
        let _auth = GoogleAuthenticator::new();
    }
}
