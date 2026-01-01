//! Git operations module
//!
//! Async helper functions for git operations with event emission support.
//! These functions orchestrate repository operations and provide user-friendly
//! result messages for the UI layer.

use crate::git_state::{GitCommit, GitSyncStatus};
use iriebook::managers::repository_manager::{RepositoryManager, SaveResult, SyncResult};
use iriebook::resource_access::{CredentialStore, GitClient, traits::GitAccess};
use std::path::Path;

/// Sync repository: pull with rebase + retry pending push
///
/// # Arguments
/// * `workspace_path` - Path to the workspace repository
/// * `repo_manager` - Repository manager instance
///
/// # Returns
/// * `Ok(String)` with success message
/// * `Err(String)` with error message
pub async fn sync_repository(
    workspace_path: &Path,
    repo_manager: &RepositoryManager,
) -> Result<String, String> {
    // Get token from keyring
    let token = CredentialStore::retrieve_github_token()
        .map_err(|e| format!("Failed to retrieve GitHub token: {}", e))?;

    // Perform sync
    let result = repo_manager
        .sync_workspace(workspace_path, &token)
        .await
        .map_err(|e| format!("Sync failed: {}", e))?;

    // Return message based on result
    match result {
        SyncResult::Synced => Ok("Synced successfully".to_string()),
        SyncResult::SyncedAndPushed => Ok("Synced and pushed pending commits".to_string()),
        SyncResult::SyncedPushFailed(msg) => {
            Ok(format!("Synced, but push failed: {}", msg))
        }
    }
}

/// Save repository: commit + push
///
/// # Arguments
/// * `workspace_path` - Path to the workspace repository
/// * `message` - Commit message
/// * `repo_manager` - Repository manager instance
///
/// # Returns
/// * `Ok(String)` with success message
/// * `Err(String)` with error message
pub async fn save_repository(
    workspace_path: &Path,
    message: &str,
    repo_manager: &RepositoryManager,
) -> Result<String, String> {
    let token = CredentialStore::retrieve_github_token()
        .map_err(|e| format!("Failed to retrieve GitHub token: {}", e))?;

    let result = repo_manager
        .save_workspace(workspace_path, message, &token)
        .await
        .map_err(|e| format!("Save failed: {}", e))?;

    match result {
        SaveResult::SavedAndPushed => Ok("Committed and pushed successfully".to_string()),
        SaveResult::SavedPushPending(msg) => {
            Ok(format!("Committed, push pending: {}", msg))
        }
        SaveResult::NothingToCommit => Ok("No changes to commit".to_string()),
    }
}

/// Get commit history
///
/// # Arguments
/// * `workspace_path` - Path to the workspace repository
/// * `repo_manager` - Repository manager instance
/// * `limit` - Maximum number of commits to retrieve
///
/// # Returns
/// * `Ok(Vec<GitCommit>)` with commit history
/// * `Err(String)` with error message
pub async fn get_commit_history(
    workspace_path: &Path,
    repo_manager: &RepositoryManager,
    limit: usize,
) -> Result<Vec<GitCommit>, String> {
    repo_manager
        .get_history(workspace_path, limit)
        .map_err(|e| format!("Failed to get commit history: {}", e))
}

/// Get sync status
///
/// # Arguments
/// * `workspace_path` - Path to the workspace repository
/// * `repo_manager` - Repository manager instance
///
/// # Returns
/// * `Ok(GitSyncStatus)` with sync status
/// * `Err(String)` with error message
pub async fn get_sync_status(
    workspace_path: &Path,
    repo_manager: &RepositoryManager,
) -> Result<GitSyncStatus, String> {
    let status = repo_manager
        .get_sync_status(workspace_path)
        .map_err(|e| format!("Failed to get sync status: {}", e))?;

    // Convert from manager GitSyncStatus to ui-common GitSyncStatus
    match status {
        iriebook::managers::repository_manager::GitSyncStatus::Uninitialized => {
            Ok(GitSyncStatus::Uninitialized)
        }
        iriebook::managers::repository_manager::GitSyncStatus::Clean => Ok(GitSyncStatus::Clean),
        iriebook::managers::repository_manager::GitSyncStatus::NeedsPush => {
            Ok(GitSyncStatus::NeedsPush)
        }
        iriebook::managers::repository_manager::GitSyncStatus::NeedsPull => {
            Ok(GitSyncStatus::NeedsPull)
        }
        iriebook::managers::repository_manager::GitSyncStatus::Dirty => Ok(GitSyncStatus::Dirty),
    }
}

/// Clone a GitHub repository and initialize workspace
///
/// # Arguments
/// * `github_url` - GitHub repository URL
/// * `workspace_path` - Path where the workspace will be initialized
/// * `repo_manager` - Repository manager instance
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message
pub async fn clone_repository(
    github_url: &str,
    workspace_path: &Path,
    repo_manager: &RepositoryManager,
) -> Result<(), String> {
    let token = CredentialStore::retrieve_github_token()
        .map_err(|e| format!("Failed to retrieve GitHub token: {}", e))?;

    repo_manager
        .initialize_workspace(workspace_path, github_url, &token)
        .await
        .map_err(|e| format!("Clone failed: {}", e))
}

/// Check if a directory is a git repository
///
/// # Arguments
/// * `workspace_path` - Path to check
///
/// # Returns
/// * `Ok(true)` if directory is a git repository
/// * `Ok(false)` otherwise
pub fn check_initialized(workspace_path: &Path) -> Result<bool, String> {
    Ok(GitClient.is_repository(workspace_path))
}

/// Get the remote URL of a git repository
///
/// # Arguments
/// * `workspace_path` - Path to the repository
///
/// # Returns
/// * `Ok(String)` with remote URL
/// * `Err(String)` with error message
pub fn get_remote_url(workspace_path: &Path) -> Result<String, String> {
    GitClient
        .get_remote_url(workspace_path)
        .map_err(|e| format!("Failed to get remote URL: {}", e))
}

#[cfg(test)]
mod tests {
    

    // Note: These are integration tests that would require a real repository
    // For now, we're just testing that the functions are callable
    // Real tests would use a mock RepositoryManager

    #[test]
    fn test_module_compiles() {
        // This test just ensures the module compiles correctly
        // Real tests would need mocks or a test repository
    }
}
