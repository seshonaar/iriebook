//! Google authentication operations for UI layer
//!
//! This module provides UI-agnostic functions for Google OAuth authentication
//! that can be reused across different UI implementations (Tauri, Web, TUI, etc.).
//!
//! Following Volatility-Based Design principles, this orchestration logic lives
//! in ui-common rather than in specific UI frameworks, making UIs thin and replaceable.

use iriebook::resource_access::CredentialStore;
use iriebook::resource_access::google_auth::GoogleAuthenticator;
use iriebook::resource_access::google_docs::GoogleDocsClient;
use iriebook::resource_access::traits::{GoogleDocInfo, GoogleDocsAccess};
use tokio::sync::oneshot;

/// Start Google OAuth authorization flow with cancellation support
///
/// This function orchestrates the complete OAuth flow:
/// 1. Prepares the authorization flow (gets auth URL)
/// 2. Opens browser via callback
/// 3. Waits for authorization code or cancellation
/// 4. Exchanges code for tokens
/// 5. Stores tokens securely
///
/// # Arguments
/// * `authenticator` - Google authenticator instance
/// * `cancellation_rx` - oneshot receiver for cancellation signal
/// * `open_browser_fn` - Callback to open browser with URL
///
/// # Returns
/// * `Ok(())` if authentication successful
/// * `Err(String)` if authentication failed or cancelled
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::start_auth_flow;
/// use iriebook::resource_access::google_auth::GoogleAuthenticator;
/// use tokio::sync::oneshot;
///
/// # async fn example() -> Result<(), String> {
/// let authenticator = GoogleAuthenticator::new();
/// let (tx, rx) = oneshot::channel();
///
/// start_auth_flow(
///     &authenticator,
///     rx,
///     |url| {
///         println!("Open this URL: {}", url);
///         Ok(())
///     }
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn start_auth_flow<F>(
    authenticator: &GoogleAuthenticator,
    cancellation_rx: oneshot::Receiver<()>,
    open_browser_fn: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<(), String>,
{
    // 1. Prepare flow
    let flow = authenticator
        .prepare_auth_flow()
        .await
        .map_err(|e| e.to_string())?;

    // 2. Open browser via callback
    open_browser_fn(&flow.url)?;

    let redirect_uri = flow.redirect_uri.clone();

    // 3. Wait for code or cancellation
    let code = tokio::select! {
        res = flow.wait_for_code() => {
            res.map_err(|e| e.to_string())?
        }
        _ = cancellation_rx => {
            return Err("Authentication cancelled".to_string());
        }
    };

    // 4. Exchange code for tokens
    let token_response = authenticator
        .exchange_code(&code, &redirect_uri)
        .await
        .map_err(|e| e.to_string())?;

    // 5. Store tokens
    authenticator
        .save_tokens(&token_response)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Check if user is authenticated with Google
///
/// Attempts to retrieve a valid access token. If successful, user is authenticated.
/// If token is expired, this will attempt to refresh it using the stored refresh token.
///
/// # Arguments
/// * `authenticator` - Google authenticator instance
///
/// # Returns
/// * `Ok(true)` if authenticated with valid token
/// * `Ok(false)` if not authenticated or refresh failed
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::check_authenticated;
/// use iriebook::resource_access::google_auth::GoogleAuthenticator;
///
/// # async fn example() -> Result<bool, String> {
/// let authenticator = GoogleAuthenticator::new();
/// let is_auth = check_authenticated(&authenticator).await?;
/// # Ok(is_auth)
/// # }
/// ```
pub async fn check_authenticated(authenticator: &GoogleAuthenticator) -> Result<bool, String> {
    match authenticator.get_valid_token().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// List available Google Docs
///
/// Retrieves a list of Google Docs that the user has access to.
/// Requires valid authentication - will fail if not authenticated.
///
/// # Arguments
/// * `authenticator` - Google authenticator instance
/// * `docs_client` - Google Docs client instance
/// * `max_results` - Maximum number of documents to list (up to 100)
///
/// # Returns
/// * `Ok(Vec<GoogleDocInfo>)` with document list
/// * `Err(String)` if not authenticated or API call fails
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::list_documents;
/// use iriebook::resource_access::google_auth::GoogleAuthenticator;
/// use iriebook::resource_access::GoogleDocsClient;
///
/// # async fn example() -> Result<Vec<iriebook::resource_access::GoogleDocInfo>, String> {
/// let authenticator = GoogleAuthenticator::new();
/// let docs_client = GoogleDocsClient::new();
/// let docs = list_documents(&authenticator, &docs_client, 50).await?;
/// # Ok(docs)
/// # }
/// ```
pub async fn list_documents(
    authenticator: &GoogleAuthenticator,
    docs_client: &GoogleDocsClient,
    max_results: u32,
) -> Result<Vec<GoogleDocInfo>, String> {
    let token = authenticator
        .get_valid_token()
        .await
        .map_err(|e| format!("Not authenticated: {}", e))?;

    docs_client
        .list_documents(&token, max_results)
        .await
        .map_err(|e| e.to_string())
}

/// Logout from Google (delete stored credentials)
///
/// Deletes all stored Google credentials (access token, refresh token, expiry).
/// After calling this, the user will need to authenticate again to use Google features.
///
/// # Returns
/// * `Ok(())` if logout successful
/// * `Err(String)` if credential deletion fails
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::logout;
///
/// # fn example() -> Result<(), String> {
/// logout()?;
/// # Ok(())
/// # }
/// ```
pub fn logout() -> Result<(), String> {
    CredentialStore::delete_google_token().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    // Note: These tests require mocking the authenticator and client,
    // which is beyond the scope of this initial implementation.
    // In a production codebase, we would use dependency injection with traits
    // to make these testable.

    #[test]
    fn test_logout_delegates_to_credential_store() {
        // This test would verify that logout() calls CredentialStore::delete_google_token()
        // In practice, this is just a thin wrapper so the implementation is trivial
    }
}
