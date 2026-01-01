//! Git state types for UI
//!
//! This module defines git-related state types that are used by the UI layer.
//! All types are serializable and have TypeScript bindings for the Tauri frontend.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Git synchronization status for UI display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "status")]
pub enum GitSyncStatus {
    /// Repository not initialized (.git not present)
    Uninitialized,
    /// Working directory clean, in sync with remote
    Clean,
    /// Has commits that need to be pushed
    NeedsPush,
    /// Remote has commits that need to be pulled
    NeedsPull,
    /// Has uncommitted changes
    Dirty,
}

/// GitHub authentication status for UI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "status")]
pub enum GitAuthStatus {
    /// Not authenticated with GitHub
    NotAuthenticated,
    /// Successfully authenticated
    Authenticated,
    /// Token expired, need to re-authenticate
    TokenExpired,
}

/// GitHub Device Flow information for UI display
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFlowInfo {
    /// Device code (internal, not shown to user)
    pub device_code: String,
    /// User code to enter in browser
    pub user_code: String,
    /// URL where user should authorize
    pub verification_uri: String,
    /// Seconds until code expires
    #[specta(type = u32)]
    pub expires_in: u64,
}

// Re-export GitCommit from iriebook for convenience
pub use iriebook::utilities::types::GitCommit;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_sync_status_serialization() {
        let status = GitSyncStatus::Clean;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Clean"));

        let deserialized: GitSyncStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, GitSyncStatus::Clean);
    }

    #[test]
    fn git_auth_status_serialization() {
        let status = GitAuthStatus::Authenticated;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Authenticated"));

        let deserialized: GitAuthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, GitAuthStatus::Authenticated);
    }

    #[test]
    fn device_flow_info_serialization() {
        let info = DeviceFlowInfo {
            device_code: "device123".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            expires_in: 900,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("ABCD-1234"));
        assert!(json.contains("userCode")); // Check camelCase conversion

        let deserialized: DeviceFlowInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_code, "ABCD-1234");
        assert_eq!(deserialized.expires_in, 900);
    }

    #[test]
    fn git_sync_status_variants() {
        let statuses = vec![
            GitSyncStatus::Uninitialized,
            GitSyncStatus::Clean,
            GitSyncStatus::NeedsPush,
            GitSyncStatus::NeedsPull,
            GitSyncStatus::Dirty,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: GitSyncStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }
}
