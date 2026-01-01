//! Repository Manager
//!
//! Orchestrates high-level git workflows for the library folder.
//! This Manager coordinates version control operations but delegates
//! all low-level git operations to the GitAccess trait implementation.
//!
//! Workflows:
//! 1. Initialize workspace (clone from GitHub)
//! 2. Sync workspace (pull with rebase, retry pending pushes)
//! 3. Save workspace (commit and push changes)
//! 4. Query status and history

use crate::resource_access::file::OUTPUT_DIR_NAME;
use crate::resource_access::traits::GitAccess;
use crate::utilities::error::IrieBookError;
use crate::utilities::types::GitCommit;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Result of sync operation
#[derive(Debug, Clone, PartialEq)]
pub enum SyncResult {
    /// Successfully synced from remote
    Synced,
    /// Successfully synced and pushed pending commits
    SyncedAndPushed,
    /// Synced successfully but push failed (will retry later)
    SyncedPushFailed(String),
}

/// Result of save operation
#[derive(Debug, Clone, PartialEq)]
pub enum SaveResult {
    /// Successfully committed and pushed
    SavedAndPushed,
    /// Successfully committed but push pending (offline or failed)
    SavedPushPending(String),
    /// No changes to commit
    NothingToCommit,
}

/// Git repository status for UI display
#[derive(Debug, Clone, PartialEq)]
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

/// Repository Manager for orchestrating git workflows
pub struct RepositoryManager {
    git_access: Arc<dyn GitAccess>,
}

impl RepositoryManager {
    /// Create a new repository manager
    ///
    /// # Arguments
    /// * `git_access` - Git access implementation (trait object)
    pub fn new(git_access: Arc<dyn GitAccess>) -> Self {
        Self { git_access }
    }

    /// Initialize workspace by cloning from GitHub
    ///
    /// # Arguments
    /// * `workspace_path` - Path where repository should be cloned
    /// * `github_url` - GitHub repository URL (HTTPS)
    /// * `token` - GitHub OAuth token for authentication
    ///
    /// # Returns
    /// * `Ok(())` if workspace initialized successfully
    /// * `Err(IrieBookError)` if initialization fails
    #[instrument(skip(self, token), fields(path = %workspace_path.display()))]
    pub async fn initialize_workspace(
        &self,
        workspace_path: &Path,
        github_url: &str,
        token: &str,
    ) -> Result<(), IrieBookError> {
        info!("Initializing workspace");

        // Clone repository
        self.git_access.clone_repository(github_url, workspace_path, token)?;
        debug!("Repository cloned");

        // Ensure .gitignore exists with irie/ and output folder
        self.ensure_gitignore(workspace_path)?;

        info!("Workspace initialized successfully");

        Ok(())
    }

    /// Sync workspace: clean working dir + rebase local commits + retry pending push
    ///
    /// Preserves local commits via rebase while discarding uncommitted changes.
    /// Rebase is the core operation for merging local and remote work.
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace repository
    /// * `token` - GitHub OAuth token for authentication
    ///
    /// # Returns
    /// * `Ok(SyncResult)` with sync result
    /// * `Err(IrieBookError)` if sync fails
    #[instrument(skip(self, token), fields(path = %workspace_path.display()))]
    pub async fn sync_workspace(
        &self,
        workspace_path: &Path,
        token: &str,
    ) -> Result<SyncResult, IrieBookError> {
        info!("Starting workspace sync");

        // Prepare for rebase: discard uncommitted changes + clean untracked files
        // This ensures rebase can proceed without conflicts
        self.git_access.prepare_for_rebase(workspace_path)?;
        debug!("Prepared for rebase");

        // Pull with rebase: keeps local commits, merges with remote
        self.git_access.pull_rebase_ours(workspace_path)?;
        debug!("Pulled with rebase");

        // Retry pending push if any
        match self.has_pending_push(workspace_path) {
            Ok(true) => match self.git_access.push(workspace_path, token) {
                Ok(()) => {
                    info!("Sync complete with push");
                    Ok(SyncResult::SyncedAndPushed)
                }
                Err(e) => {
                    warn!(error = %e, "Sync complete but push failed");
                    Ok(SyncResult::SyncedPushFailed(e.to_string()))
                }
            },
            Ok(false) => {
                info!("Sync complete");
                Ok(SyncResult::Synced)
            }
            Err(e) => Err(e),
        }
    }

    /// Save workspace: commit + push
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace repository
    /// * `message` - Commit message
    /// * `token` - GitHub OAuth token for authentication
    ///
    /// # Returns
    /// * `Ok(SaveResult)` with save result
    /// * `Err(IrieBookError)` if save fails
    #[instrument(skip(self, token), fields(path = %workspace_path.display()))]
    pub async fn save_workspace(
        &self,
        workspace_path: &Path,
        message: &str,
        token: &str,
    ) -> Result<SaveResult, IrieBookError> {
        info!("Saving workspace");

        // Check if there are changes to commit
        let has_changes = self.git_access.has_uncommitted_changes(workspace_path)?;

        if has_changes {
            self.git_access.add_all(workspace_path)?;
            let commit_hash = self.git_access.commit(workspace_path, message)?;
            debug!(hash = %commit_hash, "Changes committed");
        } else {
            debug!("No changes to commit");
        }

        // Try to push (whether new commit or pending)
        match self.git_access.push(workspace_path, token) {
            Ok(()) => {
                info!("Saved and pushed");
                Ok(SaveResult::SavedAndPushed)
            }
            Err(e) => {
                // Mark as pending push for next sync/save
                if has_changes {
                    warn!(error = %e, "Saved but push pending");
                    Ok(SaveResult::SavedPushPending(e.to_string()))
                } else {
                    debug!("Nothing to commit");
                    Ok(SaveResult::NothingToCommit)
                }
            }
        }
    }

    /// Get commit history
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace repository
    /// * `limit` - Maximum number of commits to retrieve
    ///
    /// # Returns
    /// * `Ok(Vec<GitCommit>)` with commit history
    /// * `Err(IrieBookError)` if operation fails
    pub fn get_history(
        &self,
        workspace_path: &Path,
        limit: usize,
    ) -> Result<Vec<GitCommit>, IrieBookError> {
        self.git_access.get_log(workspace_path, limit)
    }

    /// Get repository sync status
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace repository
    ///
    /// # Returns
    /// * `Ok(GitSyncStatus)` with sync status
    /// * `Err(IrieBookError)` if operation fails
    pub fn get_sync_status(&self, workspace_path: &Path) -> Result<GitSyncStatus, IrieBookError> {
        if !self.git_access.is_repository(workspace_path) {
            return Ok(GitSyncStatus::Uninitialized);
        }

        let status = self.git_access.get_status(workspace_path)?;

        // Determine sync status based on git status
        let sync_status = match (status.has_uncommitted, status.ahead_by > 0, status.behind_by > 0) {
            (true, _, _) => GitSyncStatus::Dirty,
            (false, true, _) => GitSyncStatus::NeedsPush,
            (false, false, true) => GitSyncStatus::NeedsPull,
            (false, false, false) => GitSyncStatus::Clean,
        };

        Ok(sync_status)
    }

    /// Ensure .gitignore exists with proper exclusions
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace repository
    ///
    /// # Returns
    /// * `Ok(())` if .gitignore created/updated successfully
    /// * `Err(IrieBookError)` if operation fails
    fn ensure_gitignore(&self, workspace_path: &Path) -> Result<(), IrieBookError> {
        let gitignore_path = workspace_path.join(".gitignore");
        let content = format!("irie/\n{}/\n", OUTPUT_DIR_NAME);

        // Always create/overwrite (remote clone has priority)
        std::fs::write(&gitignore_path, content).map_err(|e| IrieBookError::FileWrite {
            path: gitignore_path.display().to_string(),
            source: e,
        })?;

        Ok(())
    }

    /// Check if there are commits that need to be pushed
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace repository
    ///
    /// # Returns
    /// * `Ok(true)` if there are commits ahead of remote
    /// * `Ok(false)` if repository is in sync
    /// * `Err(IrieBookError)` if operation fails
    fn has_pending_push(&self, workspace_path: &Path) -> Result<bool, IrieBookError> {
        let status = self.git_access.get_status(workspace_path)?;
        Ok(status.ahead_by > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::types::GitStatus;

    // Mock implementation of GitAccess for testing
    struct MockGitAccess {
        is_repo: bool,
        has_uncommitted: bool,
        ahead_by: usize,
        behind_by: usize,
    }

    impl GitAccess for MockGitAccess {
        fn clone_repository(&self, _url: &str, _path: &Path, _token: &str) -> Result<(), IrieBookError> {
            Ok(())
        }

        fn get_remote_url(&self, _repo_path: &Path) -> Result<String, IrieBookError> {
            Ok("https://github.com/user/repo.git".to_string())
        }

        fn is_repository(&self, _path: &Path) -> bool {
            self.is_repo
        }

        fn add_all(&self, _repo_path: &Path) -> Result<(), IrieBookError> {
            Ok(())
        }

        fn commit(&self, _repo_path: &Path, _message: &str) -> Result<String, IrieBookError> {
            Ok("abc123".to_string())
        }

        fn pull_rebase_ours(&self, _repo_path: &Path) -> Result<(), IrieBookError> {
            Ok(())
        }

        fn push(&self, _repo_path: &Path, _token: &str) -> Result<(), IrieBookError> {
            Ok(())
        }

        fn get_log(&self, _repo_path: &Path, _limit: usize) -> Result<Vec<GitCommit>, IrieBookError> {
            Ok(vec![])
        }

        fn get_status(&self, _repo_path: &Path) -> Result<GitStatus, IrieBookError> {
            Ok(GitStatus {
                ahead_by: self.ahead_by,
                behind_by: self.behind_by,
                has_uncommitted: self.has_uncommitted,
            })
        }

        fn has_uncommitted_changes(&self, _repo_path: &Path) -> Result<bool, IrieBookError> {
            Ok(self.has_uncommitted)
        }

        fn get_changed_files(&self, _repo_path: &Path, _commit_hash: &str) -> Result<Vec<String>, IrieBookError> {
            Ok(vec![])
        }

        fn discard_local_changes(&self, _repo_path: &Path) -> Result<(), IrieBookError> {
            Ok(())
        }

        fn prepare_for_rebase(&self, _repo_path: &Path) -> Result<(), IrieBookError> {
            Ok(())
        }

        fn get_folder_status(&self, _repo_path: &Path, _folder_path: &Path) -> Result<bool, IrieBookError> {
            Ok(false)
        }

        fn get_all_changed_files(&self, _repo_path: &Path) -> Result<Vec<std::path::PathBuf>, IrieBookError> {
            Ok(vec![])
        }
    }

    #[test]
    fn sync_status_uninitialized_when_not_repo() {
        let mock_git = Arc::new(MockGitAccess {
            is_repo: false,
            has_uncommitted: false,
            ahead_by: 0,
            behind_by: 0,
        });

        let manager = RepositoryManager::new(mock_git);
        let status = manager.get_sync_status(Path::new("/tmp/test")).unwrap();

        assert_eq!(status, GitSyncStatus::Uninitialized);
    }

    #[test]
    fn sync_status_clean_when_no_changes() {
        let mock_git = Arc::new(MockGitAccess {
            is_repo: true,
            has_uncommitted: false,
            ahead_by: 0,
            behind_by: 0,
        });

        let manager = RepositoryManager::new(mock_git);
        let status = manager.get_sync_status(Path::new("/tmp/test")).unwrap();

        assert_eq!(status, GitSyncStatus::Clean);
    }

    #[test]
    fn sync_status_dirty_when_uncommitted_changes() {
        let mock_git = Arc::new(MockGitAccess {
            is_repo: true,
            has_uncommitted: true,
            ahead_by: 0,
            behind_by: 0,
        });

        let manager = RepositoryManager::new(mock_git);
        let status = manager.get_sync_status(Path::new("/tmp/test")).unwrap();

        assert_eq!(status, GitSyncStatus::Dirty);
    }

    #[test]
    fn sync_status_needs_push_when_ahead() {
        let mock_git = Arc::new(MockGitAccess {
            is_repo: true,
            has_uncommitted: false,
            ahead_by: 2,
            behind_by: 0,
        });

        let manager = RepositoryManager::new(mock_git);
        let status = manager.get_sync_status(Path::new("/tmp/test")).unwrap();

        assert_eq!(status, GitSyncStatus::NeedsPush);
    }

    #[test]
    fn sync_status_needs_pull_when_behind() {
        let mock_git = Arc::new(MockGitAccess {
            is_repo: true,
            has_uncommitted: false,
            ahead_by: 0,
            behind_by: 3,
        });

        let manager = RepositoryManager::new(mock_git);
        let status = manager.get_sync_status(Path::new("/tmp/test")).unwrap();

        assert_eq!(status, GitSyncStatus::NeedsPull);
    }
}
