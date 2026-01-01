//! Secure credential storage using OS keyring
//!
//! Stores OAuth tokens in the OS-specific secure credential store:
//! - Linux: Secret Service (GNOME Keyring, KDE Wallet)
//! - macOS: Keychain
//! - Windows: Credential Manager

use crate::utilities::error::IrieBookError;
use keyring::Entry;
use tracing::{debug, warn};

const GITHUB_SERVICE_NAME: &str = "iriebook-github";
const GITHUB_USERNAME: &str = "oauth-token";

const GOOGLE_SERVICE_NAME: &str = "iriebook-google";
const GOOGLE_USERNAME: &str = "oauth-token";

/// Credential storage using OS keyring
pub struct CredentialStore;

impl CredentialStore {
    /// Generic method to store a token in OS keyring
    fn store(service: &str, username: &str, token: &str) -> Result<(), IrieBookError> {
        debug!(service = %service, username = %username, "Attempting to store token");
        let entry = Entry::new(service, username)
            .map_err(|e| {
                warn!(service = %service, error = %e, "Failed to create keyring entry");
                IrieBookError::CredentialStorage(format!("Failed to create keyring entry: {}", e))
            })?;

        match entry.set_password(token) {
            Ok(_) => {
                debug!(service = %service, "Token stored successfully");
                Ok(())
            }
            Err(e) => {
                warn!(service = %service, error = %e, "Failed to set password");
                Err(IrieBookError::CredentialStorage(format!("Failed to store token: {}", e)))
            }
        }
    }

    /// Generic method to retrieve a token from OS keyring
    fn retrieve(service: &str, username: &str) -> Result<String, IrieBookError> {
        debug!(service = %service, username = %username, "Attempting to retrieve token");
        let entry = Entry::new(service, username)
            .map_err(|e| {
                warn!(service = %service, error = %e, "Failed to create keyring entry");
                IrieBookError::CredentialStorage(format!("Failed to create keyring entry: {}", e))
            })?;

        match entry.get_password() {
            Ok(token) => {
                debug!(service = %service, "Token retrieved successfully");
                Ok(token)
            }
            Err(e) => {
                debug!(service = %service, error = %e, "Failed to retrieve token");
                Err(IrieBookError::CredentialStorage(format!("Failed to retrieve token: {}", e)))
            }
        }
    }

    /// Generic method to delete a token from OS keyring
    fn delete(service: &str, username: &str) -> Result<(), IrieBookError> {
        let entry = Entry::new(service, username)
            .map_err(|e| IrieBookError::CredentialStorage(format!("Failed to create keyring entry: {}", e)))?;

        entry
            .delete_credential()
            .map_err(|e| IrieBookError::CredentialStorage(format!("Failed to delete token: {}", e)))?;

        Ok(())
    }

    /// Generic method to check if a token exists in OS keyring
    fn has(service: &str, username: &str) -> bool {
        Self::retrieve(service, username).is_ok()
    }

    /// Store GitHub OAuth token in OS keyring
    ///
    /// # Arguments
    /// * `token` - The OAuth token to store
    ///
    /// # Returns
    /// * `Ok(())` if token stored successfully
    /// * `Err(IrieBookError)` if storage fails
    pub fn store_github_token(token: &str) -> Result<(), IrieBookError> {
        Self::store(GITHUB_SERVICE_NAME, GITHUB_USERNAME, token)
    }

    /// Retrieve GitHub OAuth token from OS keyring
    ///
    /// # Returns
    /// * `Ok(String)` with the token if found
    /// * `Err(IrieBookError)` if token not found or retrieval fails
    pub fn retrieve_github_token() -> Result<String, IrieBookError> {
        Self::retrieve(GITHUB_SERVICE_NAME, GITHUB_USERNAME)
    }

    /// Delete GitHub OAuth token from OS keyring
    ///
    /// # Returns
    /// * `Ok(())` if token deleted successfully
    /// * `Err(IrieBookError)` if deletion fails
    pub fn delete_github_token() -> Result<(), IrieBookError> {
        Self::delete(GITHUB_SERVICE_NAME, GITHUB_USERNAME)
    }

    /// Check if GitHub OAuth token exists in OS keyring
    ///
    /// # Returns
    /// * `true` if token exists
    /// * `false` if token doesn't exist or retrieval fails
    pub fn has_github_token() -> bool {
        Self::has(GITHUB_SERVICE_NAME, GITHUB_USERNAME)
    }

    /// Store Google OAuth token in OS keyring
    ///
    /// # Arguments
    /// * `token` - The OAuth token to store
    ///
    /// # Returns
    /// * `Ok(())` if token stored successfully
    /// * `Err(IrieBookError)` if storage fails
    pub fn store_google_token(token: &str) -> Result<(), IrieBookError> {
        Self::store(GOOGLE_SERVICE_NAME, GOOGLE_USERNAME, token)
    }

    /// Retrieve Google OAuth token from OS keyring
    ///
    /// # Returns
    /// * `Ok(String)` with the token if found
    /// * `Err(IrieBookError)` if token not found or retrieval fails
    pub fn retrieve_google_token() -> Result<String, IrieBookError> {
        Self::retrieve(GOOGLE_SERVICE_NAME, GOOGLE_USERNAME)
    }

    /// Delete Google OAuth token from OS keyring
    ///
    /// # Returns
    /// * `Ok(())` if token deleted successfully
    /// * `Err(IrieBookError)` if deletion fails
    pub fn delete_google_token() -> Result<(), IrieBookError> {
        Self::delete(GOOGLE_SERVICE_NAME, GOOGLE_USERNAME)
    }

    /// Check if Google OAuth token exists in OS keyring
    ///
    /// # Returns
    /// * `true` if token exists
    /// * `false` if token doesn't exist or retrieval fails
    pub fn has_google_token() -> bool {
        Self::has(GOOGLE_SERVICE_NAME, GOOGLE_USERNAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests interact with the actual OS keyring
    // They should be run carefully as they modify system state

    #[test]
    #[ignore] // Run with --ignored to test keyring integration
    fn test_store_and_retrieve_token() {
        let test_token = "ghp_test_token_123456789";

        // Store token
        CredentialStore::store_github_token(test_token).unwrap();

        // Retrieve token
        let retrieved = CredentialStore::retrieve_github_token().unwrap();
        assert_eq!(retrieved, test_token);

        // Clean up
        CredentialStore::delete_github_token().unwrap();
    }

    #[test]
    #[ignore]
    fn test_has_token() {
        let test_token = "ghp_test_token_987654321";

        // Initially should not have token
        let _ = CredentialStore::delete_github_token(); // Clean up any existing token

        // Store token
        CredentialStore::store_github_token(test_token).unwrap();

        // Should now have token
        assert!(CredentialStore::has_github_token());

        // Clean up
        CredentialStore::delete_github_token().unwrap();

        // Should no longer have token
        assert!(!CredentialStore::has_github_token());
    }

    #[test]
    #[ignore]
    fn test_delete_token() {
        let test_token = "ghp_test_token_delete";

        // Store token
        CredentialStore::store_github_token(test_token).unwrap();

        // Delete token
        CredentialStore::delete_github_token().unwrap();

        // Retrieving should now fail
        assert!(CredentialStore::retrieve_github_token().is_err());
    }
}
