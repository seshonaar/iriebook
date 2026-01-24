//! Mock implementation of GitAccess for E2E testing
//!
//! Provides a configurable mock that records all calls for verification.

use iriebook::resource_access::traits::GitAccess;
use iriebook::utilities::error::IrieBookError;
use iriebook::utilities::types::{GitCommit, GitStatus};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Records of git operations for test verification
#[derive(Debug, Clone, PartialEq)]
pub enum GitCall {
    Clone { url: String, path: PathBuf },
    GetRemoteUrl { path: PathBuf },
    IsRepository { path: PathBuf },
    AddAll { path: PathBuf },
    Commit { path: PathBuf, message: String },
    PullRebaseOurs { path: PathBuf },
    Push { path: PathBuf },
    GetLog { path: PathBuf, limit: usize },
    GetStatus { path: PathBuf },
    HasUncommittedChanges { path: PathBuf },
    GetChangedFiles { path: PathBuf, commit_hash: String },
    DiscardLocalChanges { path: PathBuf },
    PrepareForRebase { path: PathBuf },
    GetFolderStatus { repo_path: PathBuf, folder_path: PathBuf },
    GetAllChangedFiles { path: PathBuf },
}

/// Mock GitAccess implementation with configurable behavior
pub struct MockGitAccess {
    /// Whether paths should be treated as git repositories
    pub is_repo: bool,
    /// Whether there are uncommitted changes
    pub has_uncommitted: bool,
    /// How many commits ahead of remote
    pub ahead_by: usize,
    /// How many commits behind remote
    pub behind_by: usize,
    /// List of changed files to return
    pub changed_files: Vec<String>,
    /// Remote URL to return
    pub remote_url: String,
    /// Commit log to return
    pub commits: Vec<GitCommit>,
    /// Whether operations should fail
    pub should_fail: bool,
    /// Error message when failing
    pub error_message: String,
    /// Recorded calls for verification
    calls: Mutex<Vec<GitCall>>,
    /// Simulated file contents (for more complex scenarios)
    pub file_contents: Mutex<HashMap<PathBuf, String>>,
}

impl Default for MockGitAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGitAccess {
    /// Create a new mock with default configuration
    pub fn new() -> Self {
        Self {
            is_repo: true,
            has_uncommitted: false,
            ahead_by: 0,
            behind_by: 0,
            changed_files: vec![],
            remote_url: "https://github.com/test/repo.git".to_string(),
            commits: vec![],
            should_fail: false,
            error_message: "Mock error".to_string(),
            calls: Mutex::new(vec![]),
            file_contents: Mutex::new(HashMap::new()),
        }
    }

    /// Configure repository state
    pub fn with_repo_state(mut self, is_repo: bool, uncommitted: bool) -> Self {
        self.is_repo = is_repo;
        self.has_uncommitted = uncommitted;
        self
    }

    /// Configure sync state (ahead/behind remote)
    pub fn with_sync_state(mut self, ahead: usize, behind: usize) -> Self {
        self.ahead_by = ahead;
        self.behind_by = behind;
        self
    }

    /// Configure changed files list
    pub fn with_changed_files(mut self, files: Vec<String>) -> Self {
        self.changed_files = files;
        self
    }

    /// Configure remote URL
    pub fn with_remote_url(mut self, url: &str) -> Self {
        self.remote_url = url.to_string();
        self
    }

    /// Configure commit history
    pub fn with_commits(mut self, commits: Vec<GitCommit>) -> Self {
        self.commits = commits;
        self
    }

    /// Make all operations fail
    pub fn with_failure(mut self, message: &str) -> Self {
        self.should_fail = true;
        self.error_message = message.to_string();
        self
    }

    /// Get all recorded calls
    pub fn get_calls(&self) -> Vec<GitCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Check if a specific call was made
    pub fn was_called(&self, expected: &GitCall) -> bool {
        self.calls.lock().unwrap().contains(expected)
    }

    /// Clear recorded calls
    pub fn clear_calls(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Record a call
    fn record(&self, call: GitCall) {
        self.calls.lock().unwrap().push(call);
    }

    /// Return error if configured to fail
    fn maybe_fail(&self) -> Result<(), IrieBookError> {
        if self.should_fail {
            Err(IrieBookError::Git(self.error_message.clone()))
        } else {
            Ok(())
        }
    }
}

impl GitAccess for MockGitAccess {
    fn clone_repository(&self, url: &str, path: &Path, _token: &str) -> Result<(), IrieBookError> {
        self.record(GitCall::Clone {
            url: url.to_string(),
            path: path.to_path_buf(),
        });
        self.maybe_fail()
    }

    fn get_remote_url(&self, repo_path: &Path) -> Result<String, IrieBookError> {
        self.record(GitCall::GetRemoteUrl {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()?;
        Ok(self.remote_url.clone())
    }

    fn is_repository(&self, path: &Path) -> bool {
        self.record(GitCall::IsRepository {
            path: path.to_path_buf(),
        });
        self.is_repo
    }

    fn add_all(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        self.record(GitCall::AddAll {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()
    }

    fn commit(&self, repo_path: &Path, message: &str) -> Result<String, IrieBookError> {
        self.record(GitCall::Commit {
            path: repo_path.to_path_buf(),
            message: message.to_string(),
        });
        self.maybe_fail()?;
        Ok("abc123def456".to_string())
    }

    fn pull_rebase_ours(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        self.record(GitCall::PullRebaseOurs {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()
    }

    fn push(&self, repo_path: &Path, _token: &str) -> Result<(), IrieBookError> {
        self.record(GitCall::Push {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()
    }

    fn get_log(&self, repo_path: &Path, limit: usize) -> Result<Vec<GitCommit>, IrieBookError> {
        self.record(GitCall::GetLog {
            path: repo_path.to_path_buf(),
            limit,
        });
        self.maybe_fail()?;
        Ok(self.commits.clone())
    }

    fn get_status(&self, repo_path: &Path) -> Result<GitStatus, IrieBookError> {
        self.record(GitCall::GetStatus {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()?;
        Ok(GitStatus {
            ahead_by: self.ahead_by,
            behind_by: self.behind_by,
            has_uncommitted: self.has_uncommitted,
        })
    }

    fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool, IrieBookError> {
        self.record(GitCall::HasUncommittedChanges {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()?;
        Ok(self.has_uncommitted)
    }

    fn get_changed_files(
        &self,
        repo_path: &Path,
        commit_hash: &str,
    ) -> Result<Vec<String>, IrieBookError> {
        self.record(GitCall::GetChangedFiles {
            path: repo_path.to_path_buf(),
            commit_hash: commit_hash.to_string(),
        });
        self.maybe_fail()?;
        Ok(self.changed_files.clone())
    }

    fn discard_local_changes(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        self.record(GitCall::DiscardLocalChanges {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()
    }

    fn prepare_for_rebase(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        self.record(GitCall::PrepareForRebase {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()
    }

    fn get_folder_status(
        &self,
        repo_path: &Path,
        folder_path: &Path,
    ) -> Result<bool, IrieBookError> {
        self.record(GitCall::GetFolderStatus {
            repo_path: repo_path.to_path_buf(),
            folder_path: folder_path.to_path_buf(),
        });
        self.maybe_fail()?;
        Ok(self.has_uncommitted)
    }

    fn get_all_changed_files(&self, repo_path: &Path) -> Result<Vec<PathBuf>, IrieBookError> {
        self.record(GitCall::GetAllChangedFiles {
            path: repo_path.to_path_buf(),
        });
        self.maybe_fail()?;
        Ok(self
            .changed_files
            .iter()
            .map(|f| repo_path.join(f))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_git_default_is_clean_repo() {
        let mock = MockGitAccess::new();
        assert!(mock.is_repo);
        assert!(!mock.has_uncommitted);
        assert_eq!(mock.ahead_by, 0);
        assert_eq!(mock.behind_by, 0);
    }

    #[test]
    fn test_mock_git_records_calls() {
        let mock = MockGitAccess::new();
        let path = Path::new("/test/repo");

        mock.is_repository(path);
        let _ = mock.get_status(path);

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2);
        assert!(mock.was_called(&GitCall::IsRepository {
            path: path.to_path_buf()
        }));
    }

    #[test]
    fn test_mock_git_configurable_state() {
        let mock = MockGitAccess::new()
            .with_repo_state(true, true)
            .with_sync_state(2, 1)
            .with_changed_files(vec!["file.txt".to_string()]);

        assert!(mock.has_uncommitted);
        assert_eq!(mock.ahead_by, 2);
        assert_eq!(mock.behind_by, 1);
        assert_eq!(mock.changed_files.len(), 1);
    }

    #[test]
    fn test_mock_git_can_fail() {
        let mock = MockGitAccess::new().with_failure("Simulated network error");
        let result = mock.commit(Path::new("/test"), "message");
        assert!(result.is_err());
    }
}
