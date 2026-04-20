//! Secure credential storage using OS keyring
//!
//! Stores OAuth tokens in the OS-specific secure credential store:
//! - Linux: Secret Service (GNOME Keyring, KDE Wallet)
//! - macOS: Keychain
//! - Windows: Credential Manager
//!
//! For e2e tests (with e2e-mocks feature), uses in-memory storage instead

use crate::utilities::error::IrieBookError;

#[cfg(not(feature = "e2e-mocks"))]
use keyring::Entry;

#[cfg(feature = "e2e-mocks")]
use std::collections::HashMap;
#[cfg(feature = "e2e-mocks")]
use std::sync::Mutex;

const GITHUB_SERVICE_NAME: &str = "iriebook-github";
const GITHUB_USERNAME: &str = "oauth-token";

const GOOGLE_SERVICE_NAME: &str = "iriebook-google";
const GOOGLE_USERNAME: &str = "oauth-token";

#[cfg(feature = "e2e-mocks")]
static MOCK_STORAGE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

#[cfg(feature = "e2e-mocks")]
fn get_mock_key(service: &str, username: &str) -> String {
    format!("{}:{}", service, username)
}

/// Credential storage using OS keyring (or in-memory for e2e tests)
pub struct CredentialStore;

impl CredentialStore {
    /// Generic method to store a token in OS keyring (or mock storage for e2e tests)
    fn store(service: &str, username: &str, token: &str) -> Result<(), IrieBookError> {
        #[cfg(feature = "e2e-mocks")]
        {
            use std::io::Write;
            let log_msg = format!(
                "[E2E-MOCK-STORAGE] 💾 STORE {}:{} = {}\n",
                service, username, token
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/e2e-mock-storage.log")
                .and_then(|mut f| f.write_all(log_msg.as_bytes()));

            let mut storage = MOCK_STORAGE.lock().unwrap();
            if storage.is_none() {
                *storage = Some(HashMap::new());
                let init_msg = "[E2E-MOCK-STORAGE] 🔧 Init empty storage\n";
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/e2e-mock-storage.log")
                    .and_then(|mut f| f.write_all(init_msg.as_bytes()));
            }
            storage
                .as_mut()
                .unwrap()
                .insert(get_mock_key(service, username), token.to_string());
            Ok(())
        }

        #[cfg(not(feature = "e2e-mocks"))]
        {
            let entry = Entry::new(service, username).map_err(|e| {
                IrieBookError::CredentialStorage(format!("Failed to create keyring entry: {}", e))
            })?;

            match entry.set_password(token) {
                Ok(_) => Ok(()),
                Err(e) => Err(IrieBookError::CredentialStorage(format!(
                    "Failed to store token: {}",
                    e
                ))),
            }
        }
    }

    /// Generic method to retrieve a token from OS keyring (or mock storage for e2e tests)
    fn retrieve(service: &str, username: &str) -> Result<String, IrieBookError> {
        #[cfg(feature = "e2e-mocks")]
        {
            use std::io::Write;
            let storage = MOCK_STORAGE.lock().unwrap();
            if let Some(map) = storage.as_ref()
                && let Some(token) = map.get(&get_mock_key(service, username))
            {
                let log_msg = format!(
                    "[E2E-MOCK-STORAGE] 🔍 RETRIEVE {}:{} = {}\n",
                    service, username, token
                );
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/e2e-mock-storage.log")
                    .and_then(|mut f| f.write_all(log_msg.as_bytes()));
                return Ok(token.clone());
            }
            let log_msg = format!("[E2E-MOCK-STORAGE] ❌ NOT FOUND {}:{}\n", service, username);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/e2e-mock-storage.log")
                .and_then(|mut f| f.write_all(log_msg.as_bytes()));
            Err(IrieBookError::CredentialStorage(
                "Token not found".to_string(),
            ))
        }

        #[cfg(not(feature = "e2e-mocks"))]
        {
            let entry = Entry::new(service, username).map_err(|e| {
                IrieBookError::CredentialStorage(format!("Failed to create keyring entry: {}", e))
            })?;

            match entry.get_password() {
                Ok(token) => Ok(token),
                Err(e) => Err(IrieBookError::CredentialStorage(format!(
                    "Failed to retrieve token: {}",
                    e
                ))),
            }
        }
    }

    /// Generic method to delete a token from OS keyring (or mock storage for e2e tests)
    fn delete(service: &str, username: &str) -> Result<(), IrieBookError> {
        #[cfg(feature = "e2e-mocks")]
        {
            let mut storage = MOCK_STORAGE.lock().unwrap();
            if let Some(map) = storage.as_mut() {
                map.remove(&get_mock_key(service, username));
            }
            Ok(())
        }

        #[cfg(not(feature = "e2e-mocks"))]
        {
            let entry = Entry::new(service, username).map_err(|e| {
                IrieBookError::CredentialStorage(format!("Failed to create keyring entry: {}", e))
            })?;

            entry.delete_credential().map_err(|e| {
                IrieBookError::CredentialStorage(format!("Failed to delete token: {}", e))
            })?;

            Ok(())
        }
    }

    /// Generic method to check if a token exists in OS keyring (or mock storage for e2e tests)
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
