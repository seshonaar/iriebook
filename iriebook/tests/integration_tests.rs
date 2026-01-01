//! Integration tests for the IrieBook CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// Repository Manager Integration Tests
mod repository_manager_tests {
    use super::*;
    use iriebook::managers::repository_manager::{RepositoryManager, SyncResult, SaveResult};
    use iriebook::resource_access::git::GitClient;
    use iriebook::resource_access::traits::GitAccess;
    use std::path::Path;
    use std::sync::Arc;

    // Helper to initialize a git repo using git CLI
    fn init_git_repo_with_cli(path: &Path) {
        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .expect("Failed to set git user.name");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .expect("Failed to set git user.email");
    }

    // Helper to initialize a bare repository (acts as remote)
    fn init_bare_repo(path: &Path) {
        use std::process::Command;
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(path)
            .output()
            .expect("Failed to init bare repo");
    }

    // Helper to add a remote to a repository
    fn add_remote(repo_path: &Path, remote_name: &str, remote_path: &Path) {
        use std::process::Command;
        let remote_url = format!("file://{}", remote_path.display());
        Command::new("git")
            .args(["remote", "add", remote_name, &remote_url])
            .current_dir(repo_path)
            .output()
            .expect("Failed to add remote");
    }

    #[tokio::test]
    async fn rebase_fails_with_uncommitted_changes() {
        let bare_dir = TempDir::new().unwrap();
        let workspace1_dir = TempDir::new().unwrap();
        let workspace2_dir = TempDir::new().unwrap();

        // Setup bare repo
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace1_dir.path());
        add_remote(workspace1_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);

        // Create initial commit from workspace1
        let file1 = workspace1_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "initial").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Setup workspace2 and pull
        init_git_repo_with_cli(workspace2_dir.path());
        add_remote(workspace2_dir.path(), "origin", bare_dir.path());
        use std::process::Command;
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["reset", "--hard", "origin/master"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();

        // Make remote change in workspace1
        std::fs::write(&file1, "remote change").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "remote commit").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Make uncommitted change in workspace2 (should block rebase)
        let file1_ws2 = workspace2_dir.path().join("file1.txt");
        std::fs::write(&file1_ws2, "LOCAL UNCOMMITTED CHANGE").unwrap();

        // Test: Rebase should FAIL with uncommitted changes
        let result = git_client.pull_rebase_ours(workspace2_dir.path());
        assert!(result.is_err(), "Rebase should fail with uncommitted changes");
    }

    #[tokio::test]
    async fn rebase_works_with_untracked_files() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);

        // Create initial commit and push
        let tracked_file = workspace_dir.path().join("tracked.txt");
        std::fs::write(&tracked_file, "initial").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "initial").unwrap();
        git_client.push(workspace_dir.path(), "").unwrap();

        // Add untracked file (should NOT interfere with rebase)
        let untracked_file = workspace_dir.path().join("untracked.txt");
        std::fs::write(&untracked_file, "untracked content").unwrap();

        // Modify tracked file (uncommitted - should be discarded)
        std::fs::write(&tracked_file, "LOCAL CHANGES - uncommitted").unwrap();

        // Make a local commit (should be preserved via rebase)
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "local commit").unwrap();

        // Test: Can we pull-rebase with untracked files present?
        let result = git_client.pull_rebase_ours(workspace_dir.path());

        // This should succeed - untracked files don't interfere
        assert!(result.is_ok(), "Rebase should work with untracked files: {:?}", result);

        // Untracked file should still exist
        assert!(untracked_file.exists(), "Untracked files should be preserved");
    }

    #[tokio::test]
    async fn rebase_fails_when_untracked_file_conflicts_with_incoming() {
        let bare_dir = TempDir::new().unwrap();
        let workspace1_dir = TempDir::new().unwrap();
        let workspace2_dir = TempDir::new().unwrap();

        // Setup bare repo
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace1_dir.path());
        add_remote(workspace1_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);

        // Create initial commit from workspace1
        let file1 = workspace1_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "initial").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Setup workspace2 and pull
        init_git_repo_with_cli(workspace2_dir.path());
        add_remote(workspace2_dir.path(), "origin", bare_dir.path());
        use std::process::Command;
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["reset", "--hard", "origin/master"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();

        // In workspace1: add a NEW file and push
        let newfile_ws1 = workspace1_dir.path().join("newfile.txt");
        std::fs::write(&newfile_ws1, "REMOTE CONTENT").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "add newfile").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // In workspace2: create UNTRACKED file with same name but different content
        let newfile_ws2 = workspace2_dir.path().join("newfile.txt");
        std::fs::write(&newfile_ws2, "LOCAL UNTRACKED CONTENT").unwrap();

        // Test: Rebase should FAIL - untracked file conflicts with incoming
        let result = git_client.pull_rebase_ours(workspace2_dir.path());
        assert!(result.is_err(), "Rebase should fail when untracked file conflicts with incoming commit");
    }

    #[tokio::test]
    async fn prepare_for_rebase_recovers_from_stale_index_lock() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);

        // Create initial commit and push
        let file1 = workspace_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "initial").unwrap();
        git_client.push(workspace_dir.path(), "").unwrap();

        // Make a local change
        std::fs::write(&file1, "modified").unwrap();

        // Simulate a stale index.lock (from previous failed git operation)
        let index_lock_path = workspace_dir.path().join(".git").join("index.lock");
        std::fs::write(&index_lock_path, "stale lock data").unwrap();

        // Verify the lock exists
        assert!(index_lock_path.exists(), "Index.lock should exist");

        // Attempt prepare_for_rebase - should clean up index.lock and succeed
        let result = git_client.prepare_for_rebase(workspace_dir.path());
        assert!(result.is_ok(), "Should recover from stale index.lock: {:?}", result);

        // Verify the lock was cleaned up
        assert!(!index_lock_path.exists(), "Index.lock should be cleaned up");

        // Verify the uncommitted change was discarded
        let content = std::fs::read_to_string(&file1).unwrap();
        assert_eq!(content, "initial", "Uncommitted change should be discarded");
    }

    #[tokio::test]
    async fn rebase_recovers_from_stale_rebase_state() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);

        // Create initial commit and push
        let file1 = workspace_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "initial").unwrap();
        git_client.push(workspace_dir.path(), "").unwrap();

        // Simulate a stale rebase state by creating the rebase-merge directory
        let rebase_merge_path = workspace_dir.path().join(".git").join("rebase-merge");
        std::fs::create_dir(&rebase_merge_path).unwrap();
        std::fs::write(rebase_merge_path.join("dummy"), "stale rebase data").unwrap();

        // Verify the directory exists
        assert!(rebase_merge_path.exists(), "Rebase-merge directory should exist");

        // Attempt pull_rebase_ours - should clean up stale state and succeed
        let result = git_client.pull_rebase_ours(workspace_dir.path());
        assert!(result.is_ok(), "Should recover from stale rebase state: {:?}", result);

        // Verify the stale directory was cleaned up
        assert!(!rebase_merge_path.exists(), "Rebase-merge directory should be cleaned up");
    }

    #[tokio::test]
    async fn sync_workspace_discards_local_changes() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());

        // Setup workspace repo
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        // Create git client and repository manager
        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create initial commit and push
        let tracked_file = workspace_dir.path().join("tracked.txt");
        std::fs::write(&tracked_file, "initial content").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "initial commit").unwrap();
        git_client.push(workspace_dir.path(), "").unwrap();

        // Make local changes (both tracked and untracked)
        // 1. Modify tracked file
        std::fs::write(&tracked_file, "LOCAL CHANGES - should be discarded").unwrap();

        // 2. Add untracked file
        let untracked_file = workspace_dir.path().join("untracked.txt");
        std::fs::write(&untracked_file, "untracked content").unwrap();

        // 3. Add untracked directory
        let untracked_dir = workspace_dir.path().join("temp_folder");
        std::fs::create_dir(&untracked_dir).unwrap();
        std::fs::write(untracked_dir.join("temp.txt"), "temporary").unwrap();

        // Verify we have local changes
        assert!(git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());
        assert!(untracked_file.exists());
        assert!(untracked_dir.exists());

        // Run sync_workspace - should discard all local changes
        let result = repo_manager.sync_workspace(workspace_dir.path(), "").await;
        assert!(result.is_ok(), "sync_workspace should succeed: {:?}", result);
        assert_eq!(result.unwrap(), SyncResult::Synced);

        // Verify all local changes were discarded
        assert!(!git_client.has_uncommitted_changes(workspace_dir.path()).unwrap(),
                "Should have no uncommitted changes after sync");

        // Tracked file should be restored to original content
        let content = std::fs::read_to_string(&tracked_file).unwrap();
        assert_eq!(content, "initial content",
                   "Tracked file should be restored to original content");

        // Untracked file should be removed
        assert!(!untracked_file.exists(),
                "Untracked file should be removed");

        // Untracked directory should be removed
        assert!(!untracked_dir.exists(),
                "Untracked directory should be removed");
    }

    #[tokio::test]
    async fn sync_workspace_pulls_remote_changes() {
        let bare_dir = TempDir::new().unwrap();
        let workspace1_dir = TempDir::new().unwrap();
        let workspace2_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());

        // Setup first workspace
        init_git_repo_with_cli(workspace1_dir.path());
        add_remote(workspace1_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create initial commit from workspace1
        let file1 = workspace1_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial content").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "initial commit").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Setup second workspace and pull initial state
        init_git_repo_with_cli(workspace2_dir.path());
        add_remote(workspace2_dir.path(), "origin", bare_dir.path());

        use std::process::Command;
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["reset", "--hard", "origin/master"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();

        // Make change in workspace1 and push (simulating remote change)
        std::fs::write(&file1, "updated content from workspace1").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "update from workspace1").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Sync workspace2 - should pull the remote changes
        let result = repo_manager.sync_workspace(workspace2_dir.path(), "").await;
        assert!(result.is_ok(), "sync_workspace should succeed: {:?}", result);

        // Verify workspace2 has the remote changes
        let file1_in_workspace2 = workspace2_dir.path().join("file1.txt");
        let content = std::fs::read_to_string(&file1_in_workspace2).unwrap();
        assert_eq!(content, "updated content from workspace1",
                   "Workspace2 should have pulled remote changes");
    }

    #[tokio::test]
    async fn sync_workspace_discards_local_then_pulls_remote() {
        let bare_dir = TempDir::new().unwrap();
        let workspace1_dir = TempDir::new().unwrap();
        let workspace2_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());

        // Setup workspace1
        init_git_repo_with_cli(workspace1_dir.path());
        add_remote(workspace1_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create initial commit from workspace1
        let file1 = workspace1_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial content").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "initial commit").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Setup workspace2 and pull initial state
        init_git_repo_with_cli(workspace2_dir.path());
        add_remote(workspace2_dir.path(), "origin", bare_dir.path());

        use std::process::Command;
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["reset", "--hard", "origin/master"])
            .current_dir(workspace2_dir.path())
            .output()
            .unwrap();

        // Make change in workspace1 and push
        std::fs::write(&file1, "REMOTE: updated from workspace1").unwrap();
        git_client.add_all(workspace1_dir.path()).unwrap();
        git_client.commit(workspace1_dir.path(), "remote update").unwrap();
        git_client.push(workspace1_dir.path(), "").unwrap();

        // Make CONFLICTING local changes in workspace2
        let file1_workspace2 = workspace2_dir.path().join("file1.txt");
        std::fs::write(&file1_workspace2, "LOCAL: conflicting changes").unwrap();

        // Verify workspace2 has uncommitted changes
        assert!(git_client.has_uncommitted_changes(workspace2_dir.path()).unwrap());

        // Sync workspace2 - should discard local changes AND pull remote
        let result = repo_manager.sync_workspace(workspace2_dir.path(), "").await;
        assert!(result.is_ok(), "sync_workspace should succeed: {:?}", result);

        // Verify:
        // 1. No uncommitted changes
        assert!(!git_client.has_uncommitted_changes(workspace2_dir.path()).unwrap());

        // 2. File has REMOTE content (local changes were discarded)
        let content = std::fs::read_to_string(&file1_workspace2).unwrap();
        assert_eq!(content, "REMOTE: updated from workspace1",
                   "Should have remote content, local changes discarded");
    }

    #[tokio::test]
    async fn sync_workspace_retries_pending_push() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());

        // Setup workspace
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create initial commit
        let file1 = workspace_dir.path().join("file1.txt");
        std::fs::write(&file1, "initial content").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "initial commit").unwrap();
        git_client.push(workspace_dir.path(), "").unwrap();

        // Create a pending commit (committed but not pushed)
        std::fs::write(&file1, "pending content").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "pending commit").unwrap();

        // Verify we have commits ahead
        let status_before = git_client.get_status(workspace_dir.path()).unwrap();
        assert!(status_before.ahead_by > 0, "Should have commits ahead before sync");

        // Sync workspace - should push pending commits
        let result = repo_manager.sync_workspace(workspace_dir.path(), "").await;
        assert!(result.is_ok(), "sync_workspace should succeed: {:?}", result);
        assert_eq!(result.unwrap(), SyncResult::SyncedAndPushed,
                   "Should report SyncedAndPushed when pending commits are pushed");

        // Verify commits were pushed
        let status_after = git_client.get_status(workspace_dir.path()).unwrap();
        assert_eq!(status_after.ahead_by, 0, "Should have no commits ahead after sync");
        assert_eq!(status_after.behind_by, 0);
    }

    // Helper to create a book library structure
    fn create_book_library(workspace_path: &Path) {
        // Create book folders with manuscripts
        let books = vec![
            ("The Great Adventure", "Chapter 1\n\nIt was a dark and stormy night..."),
            ("Mystery Manor", "Prologue\n\nThe old house stood silent..."),
            ("Journey to Tomorrow", "Part 1\n\nIn the year 2150..."),
        ];

        for (book_name, content) in books {
            let book_dir = workspace_path.join(book_name);
            std::fs::create_dir(&book_dir).unwrap();
            let manuscript = book_dir.join("manuscript.md");
            std::fs::write(&manuscript, content).unwrap();
        }
    }

    #[tokio::test]
    async fn save_workspace_commits_and_pushes_book_changes() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup bare repo as remote
        init_bare_repo(bare_dir.path());

        // Setup workspace with book library structure
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create initial book library
        create_book_library(workspace_dir.path());

        // Save workspace - should commit and push
        let result = repo_manager.save_workspace(
            workspace_dir.path(),
            "Initial book library",
            ""
        ).await;

        assert!(result.is_ok(), "save_workspace should succeed: {:?}", result);
        assert_eq!(result.unwrap(), SaveResult::SavedAndPushed,
                   "Should commit and push successfully");

        // Verify no uncommitted changes
        assert!(!git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());

        // Verify no pending commits
        let status = git_client.get_status(workspace_dir.path()).unwrap();
        assert_eq!(status.ahead_by, 0, "Should have no commits ahead after save");
    }

    #[tokio::test]
    async fn save_workspace_handles_modified_book() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create and save initial library
        create_book_library(workspace_dir.path());
        repo_manager.save_workspace(workspace_dir.path(), "Initial", "").await.unwrap();

        // Modify one book's manuscript
        let manuscript = workspace_dir.path().join("The Great Adventure").join("manuscript.md");
        std::fs::write(&manuscript, "Chapter 1\n\nUpdated content with new scenes...").unwrap();

        // Verify we have uncommitted changes
        assert!(git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());

        // Save workspace again
        let result = repo_manager.save_workspace(
            workspace_dir.path(),
            "Update The Great Adventure manuscript",
            ""
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SaveResult::SavedAndPushed);

        // Verify clean state
        assert!(!git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());
        let status = git_client.get_status(workspace_dir.path()).unwrap();
        assert_eq!(status.ahead_by, 0);
    }

    #[tokio::test]
    async fn save_workspace_handles_new_book_added() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create and save initial library
        create_book_library(workspace_dir.path());
        repo_manager.save_workspace(workspace_dir.path(), "Initial", "").await.unwrap();

        // Add a new book
        let new_book_dir = workspace_dir.path().join("The Fourth Book");
        std::fs::create_dir(&new_book_dir).unwrap();
        let new_manuscript = new_book_dir.join("manuscript.md");
        std::fs::write(&new_manuscript, "Introduction\n\nA brand new story begins...").unwrap();

        // Also add metadata file
        let metadata = new_book_dir.join("metadata.yaml");
        std::fs::write(&metadata, "title: The Fourth Book\nauthor: Test Author").unwrap();

        // Save workspace
        let result = repo_manager.save_workspace(
            workspace_dir.path(),
            "Add new book: The Fourth Book",
            ""
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SaveResult::SavedAndPushed);

        // Verify clean state
        assert!(!git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());
    }

    #[tokio::test]
    async fn save_workspace_handles_book_deleted() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create and save initial library
        create_book_library(workspace_dir.path());
        repo_manager.save_workspace(workspace_dir.path(), "Initial", "").await.unwrap();

        // Delete one book folder
        let book_to_delete = workspace_dir.path().join("Mystery Manor");
        std::fs::remove_dir_all(&book_to_delete).unwrap();

        // Verify we have uncommitted changes (deletion)
        assert!(git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());

        // Save workspace
        let result = repo_manager.save_workspace(
            workspace_dir.path(),
            "Remove Mystery Manor",
            ""
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SaveResult::SavedAndPushed);

        // Verify clean state and book is gone
        assert!(!git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());
        assert!(!book_to_delete.exists());
    }

    #[tokio::test]
    async fn save_workspace_succeeds_when_already_clean() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create and save initial library
        create_book_library(workspace_dir.path());
        repo_manager.save_workspace(workspace_dir.path(), "Initial", "").await.unwrap();

        // Try to save again without any changes
        // Note: Implementation returns SavedAndPushed even with no changes
        // because push succeeds (it's a no-op when nothing to push)
        let result = repo_manager.save_workspace(
            workspace_dir.path(),
            "Nothing changed",
            ""
        ).await;

        assert!(result.is_ok());
        // When clean and in sync, save still succeeds (push is a no-op)
        assert_eq!(result.unwrap(), SaveResult::SavedAndPushed);

        // Verify workspace is still clean
        assert!(!git_client.has_uncommitted_changes(workspace_dir.path()).unwrap());
    }

    #[tokio::test]
    async fn save_workspace_pushes_pending_commits() {
        let bare_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(workspace_dir.path());
        add_remote(workspace_dir.path(), "origin", bare_dir.path());

        let git_client = Arc::new(GitClient);
        let repo_manager = RepositoryManager::new(git_client.clone());

        // Create and save initial library
        create_book_library(workspace_dir.path());
        repo_manager.save_workspace(workspace_dir.path(), "Initial", "").await.unwrap();

        // Create a pending commit manually (committed but not pushed)
        let manuscript = workspace_dir.path().join("The Great Adventure").join("manuscript.md");
        std::fs::write(&manuscript, "Updated content").unwrap();
        git_client.add_all(workspace_dir.path()).unwrap();
        git_client.commit(workspace_dir.path(), "Pending update").unwrap();

        // Verify we have pending commits
        let status_before = git_client.get_status(workspace_dir.path()).unwrap();
        assert!(status_before.ahead_by > 0, "Should have commits ahead");

        // Save workspace with no new changes - should push pending commits
        let result = repo_manager.save_workspace(
            workspace_dir.path(),
            "Should push pending",
            ""
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SaveResult::SavedAndPushed,
                   "Should push pending commits even with no new changes");

        // Verify commits were pushed
        let status_after = git_client.get_status(workspace_dir.path()).unwrap();
        assert_eq!(status_after.ahead_by, 0, "Should have no commits ahead after save");
    }

}

#[test]
fn test_basic_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Write test input with straight quotes
    fs::write(&input_path, r#"She said "hello" to me."#)?;

    // Run fixit
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"))
        .stdout(predicate::str::contains("Converted 2 quotes and 0 apostrophes"));

    // Check output exists
    assert!(output_path.exists());

    // Check content has curly quotes
    let output_content = fs::read_to_string(&output_path)?;
    assert!(output_content.contains('\u{201C}')); // Left curly quote
    assert!(output_content.contains('\u{201D}')); // Right curly quote
    assert!(!output_content.contains('"')); // No straight quotes

    Ok(())
}


#[test]
fn test_error_on_single_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");

    fs::write(&input_path, "She said 'hello' to me.")?;

    // Should fail with error about single quotation marks (dialogue, not apostrophes)
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("single quotation mark"))
        .stderr(predicate::str::contains("dialogue"))
        .stderr(predicate::str::contains("not apostrophes"))
        .stderr(predicate::str::contains("Validation failed"));

    Ok(())
}

#[test]
fn test_error_on_unbalanced_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");

    fs::write(&input_path, r#"She said "hello but never closed it"#)?;

    // Should fail with error about unbalanced quotes
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unbalanced quotes"))
        .stderr(predicate::str::contains("must be even"));

    Ok(())
}

#[test]
fn test_custom_output_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let custom_output = temp_dir.path().join("custom-name.md");

    fs::write(&input_path, r#"Test "quote""#)?;

    // Run with custom output
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg("-o")
        .arg(&custom_output)
        .arg(&input_path)
        .assert()
        .success();

    // Custom output should exist
    assert!(custom_output.exists());

    // Default output should NOT exist
    let default_output = temp_dir.path().join("irie/fixed.md");
    assert!(!default_output.exists());

    Ok(())
}

#[test]
fn test_verbose_mode() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");

    fs::write(&input_path, r#""Test""#)?;

    // Run with verbose flag
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg("-v")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("IrieBook"))
        .stdout(predicate::str::contains("Input:"))
        .stdout(predicate::str::contains("Read"))
        .stdout(predicate::str::contains("Validation passed"))
        .stdout(predicate::str::contains("Converted"));

    Ok(())
}

#[test]
fn test_preserves_asterisks() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, r#"This is *italic* and "quoted"."#)?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;
    
    // Asterisks must be preserved
    assert!(output_content.contains("*italic*"));
    
    // Quotes should be curly
    assert!(output_content.contains('\u{201C}'));
    assert!(output_content.contains('\u{201D}'));

    Ok(())
}

#[test]
fn test_handles_romanian_text() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "Ea a spus \"bună ziua\" în română.")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;
    
    // Romanian characters must be preserved
    assert!(output_content.contains("ă"));
    
    // Quotes should be converted
    assert!(!output_content.contains('"'));

    Ok(())
}

#[test]
fn test_multiple_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, r#"First "one" and "two" and "three"."#)?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Converted 6 quotes and 0 apostrophes"));

    let output_content = fs::read_to_string(&output_path)?;
    
    // Should have 3 opening and 3 closing curly quotes
    assert_eq!(output_content.matches('\u{201C}').count(), 3);
    assert_eq!(output_content.matches('\u{201D}').count(), 3);

    Ok(())
}

#[test]
fn test_help_message() -> Result<(), Box<dyn std::error::Error>> {
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("IrieBook"))
        .stdout(predicate::str::contains("Ebook publication pipeline"))
        .stdout(predicate::str::contains("Usage:"));

    Ok(())
}

#[test]
fn test_version() -> Result<(), Box<dyn std::error::Error>> {
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("iriebook"));

    Ok(())
}

#[test]
fn test_quotes_and_whitespace_together() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // File with both quote and whitespace issues
    fs::write(&input_path, "  She  said  \"hello  world\"  and  he  replied  \"goodbye\".  \n\n\nNext  line.  ")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"))
        .stdout(predicate::str::contains("Converted"))
        .stdout(predicate::str::contains("Trimmed"));

    // Check output file exists
    assert!(output_path.exists());

    let output_content = fs::read_to_string(&output_path)?;

    // Verify quotes are curly
    assert!(output_content.contains("\u{201C}")); // Left curly quote
    assert!(output_content.contains("\u{201D}")); // Right curly quote
    assert!(!output_content.contains('"')); // No straight quotes

    // Verify whitespace is cleaned
    assert!(!output_content.contains("  ")); // No double spaces
    // Short paragraphs don't get scene breaks (chunky threshold requires 20+ lines each side)
    // Output: paragraph + blank line + paragraph = 3 lines
    assert_eq!(output_content.lines().count(), 3);
    assert!(!output_content.contains("<div class='scene-break'></div>"));

    // Verify no leading/trailing whitespace on lines
    for line in output_content.lines() {
        assert_eq!(line, line.trim());
    }

    Ok(())
}

#[test]
fn test_bom_handling_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test-bom.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Create input file with BOM + quotes + whitespace
    let input_with_bom = "\u{FEFF}She said \"hello\" and  he  replied \"goodbye\".";
    fs::write(&input_path, input_with_bom)?;

    // Run fixit
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"));

    // Check output exists
    assert!(output_path.exists());

    // Read output as raw bytes to check for BOM
    let output_bytes = fs::read(&output_path)?;

    // Verify NO BOM in output
    // UTF-8 BOM is bytes [0xEF, 0xBB, 0xBF]
    assert!(output_bytes.len() >= 3);
    let has_bom = output_bytes[0] == 0xEF
        && output_bytes[1] == 0xBB
        && output_bytes[2] == 0xBF;
    assert!(!has_bom, "Output file should not have UTF-8 BOM");

    // Read as string and verify content
    let output_content = fs::read_to_string(&output_path)?;

    // BOM should be gone
    assert!(!output_content.starts_with('\u{FEFF}'));

    // Quotes should be curly
    assert!(output_content.contains('\u{201C}')); // Left curly quote
    assert!(output_content.contains('\u{201D}')); // Right curly quote
    assert!(!output_content.contains('\"')); // No straight quotes

    // Whitespace should be cleaned
    assert!(!output_content.contains("  ")); // No double spaces

    Ok(())
}

#[test]
fn test_word_analysis_verbose_output() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");

    fs::write(&input_path, "hello world hello rust world world")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("-v")
        .arg("--word-stats")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Word Analysis"))
        .stdout(predicate::str::contains("world"))
        .stdout(predicate::str::contains("Top words"));

    Ok(())
}

#[test]
fn test_word_analysis_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let config_path = temp_dir.path().join("config.json");

    fs::write(&config_path, r#"{"word_analysis": {"excluded_words": ["test"]}}"#)?;
    fs::write(&input_path, "test hello test world test")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .current_dir(&temp_dir)
        .arg("-v")
        .arg("--word-stats")
        .arg("test.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"))
        .stdout(predicate::str::contains("world"))
        .stdout(predicate::str::contains("stopwords excluded"));

    Ok(())
}

#[test]
fn test_word_analysis_romanian_text() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");

    fs::write(&input_path, "bună ziua și salut bună")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("-v")
        .arg("--word-stats")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("bună"));

    Ok(())
}

#[test]
fn test_romanian_text_with_apostrophes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Romanian text with valid word-final apostrophes
    fs::write(&input_path, r#"Ce dracu' vrei? El a spus într'un fel ciudat."#)?;

    // Should succeed (not fail validation)
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"));

    // Check output exists and apostrophes are converted to curly
    assert!(output_path.exists());
    let output_content = fs::read_to_string(&output_path)?;

    // Should have curly apostrophes (U+2019), not straight ones
    assert!(output_content.contains('\u{2019}')); // Right single quotation mark (curly apostrophe)
    assert!(!output_content.contains('\'')); // No straight apostrophes remain

    // Verify the Romanian text is preserved correctly
    assert!(output_content.contains("dracu"));
    assert!(output_content.contains("într"));

    Ok(())
}

// Chapter heading splitting tests

#[test]
fn test_chapter_heading_with_number() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Chapter 1 The Beginning")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should be split into H1 and styled subtitle paragraph
    assert!(output_content.contains("# Chapter 1"));
    assert!(output_content.contains("<p class=\"subtitle\">The Beginning</p>"));

    Ok(())
}

#[test]
fn test_chapter_heading_romanian_with_dash() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Capitolul 5 - Sfârșitul")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should split and trim edge dash (leading dash removed)
    // Chapter 5 becomes Chapter 1 due to sequential renumbering
    assert!(output_content.contains("# Capitolul 1"));
    assert!(output_content.contains("<p class=\"subtitle\">Sfârșitul</p>"));
    assert!(!output_content.contains("-"));

    Ok(())
}

#[test]
fn test_chapter_heading_multiple_dashes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Part 3 - The - End")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should keep internal dash
    // Part 3 becomes Part 1 due to sequential renumbering
    assert!(output_content.contains("# Part 1"));
    assert!(output_content.contains("<p class=\"subtitle\">The - End</p>"));

    Ok(())
}

#[test]
fn test_chapter_heading_no_subtitle() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Chapter 1")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should remain unchanged (no subtitle to split)
    assert!(output_content.contains("# Chapter 1"));
    assert!(!output_content.contains("subtitle"));

    Ok(())
}

#[test]
fn test_chapter_heading_no_number() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Introduction")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should be converted to H1 (all chapters become H1 for proper TOC)
    assert!(output_content.contains("# Introduction"));
    assert!(!output_content.contains("subtitle"));

    Ok(())
}

#[test]
fn test_chapter_heading_roman_numerals() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Chapter IX The End")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should remain unchanged (Roman numerals not matched)
    assert!(output_content.contains("# Chapter IX The End"));
    assert!(!output_content.contains("subtitle"));

    Ok(())
}

#[test]
fn test_chapter_heading_empty_subtitle() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Chapter 1 ---")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Subtitle becomes empty after removing dashes - should just have H2
    assert!(output_content.contains("# Chapter 1"));
    assert!(!output_content.contains("subtitle"));
    assert!(!output_content.contains("-"));

    Ok(())
}

#[test]
fn test_multiple_chapter_headings() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "## Chapter 1 The Beginning\n\nSome text.\n\n## Chapter 2 The Middle")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Both chapters should be split with styled subtitle paragraphs
    assert!(output_content.contains("# Chapter 1"));
    assert!(output_content.contains("<p class=\"subtitle\">The Beginning</p>"));
    assert!(output_content.contains("# Chapter 2"));
    assert!(output_content.contains("<p class=\"subtitle\">The Middle</p>"));

    Ok(())
}

#[test]
fn test_chapter_heading_unicode_dashes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Test with en-dash (–) and em-dash (—) - use same prefix to test unicode dash handling
    fs::write(&input_path, "## Chapter 1 – Bianca\n\n## Chapter 2 — The End")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    eprintln!("DEBUG output:\n{}", output_content);

    // Should trim Unicode dashes (en-dash and em-dash) from subtitle edges
    assert!(output_content.contains("# Chapter 1"));
    assert!(output_content.contains("<p class=\"subtitle\">Bianca</p>"));
    assert!(output_content.contains("# Chapter 2"));
    assert!(output_content.contains("<p class=\"subtitle\">The End</p>"));
    assert!(!output_content.contains("–")); // No en-dash
    assert!(!output_content.contains("—")); // No em-dash

    Ok(())
}

#[test]
fn test_chapter_heading_no_space_before_dash() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // No space between number and dash: "20-" instead of "20 -"
    fs::write(&input_path, "## Capitolul 20- Bianca")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should split even without space before dash
    // Chapter 20 becomes Chapter 1 due to sequential renumbering
    assert!(output_content.contains("# Capitolul 1"));
    assert!(output_content.contains("<p class=\"subtitle\">Bianca</p>"));
    assert!(!output_content.contains("-"));

    Ok(())
}

#[test]
fn test_chapter_heading_escaped_dash() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Escaped dash in markdown: \-
    fs::write(&input_path, "## Capitolul 1 \\- Bianca")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should remove escaped dash
    assert!(output_content.contains("# Capitolul 1"));
    assert!(output_content.contains("<p class=\"subtitle\">Bianca</p>"));
    assert!(!output_content.contains("\\-"));
    assert!(!output_content.contains("-"));

    Ok(())
}

#[test]
fn test_chapter_heading_long_text_on_same_line() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Long paragraph text after number - should NOT be treated as subtitle
    fs::write(&input_path, "## Capitolul 9 În concluzie, n-am decât să încep ușor partea a doua a planului meu. Un plan atât de obscur, încât nici eu nu știu sigur unde va ajunge.")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should NOT create subtitle paragraph for long text - just keep as H2
    assert!(output_content.contains("# Capitolul 9"));
    // The long text should remain as regular paragraph, not subtitle
    assert!(!output_content.contains("subtitle"));

    Ok(())
}

#[test]
fn test_chapter_heading_with_dash_no_number() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Dash as delimiter instead of number
    fs::write(&input_path, "## Prologue - The Beginning")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should split using dash as delimiter
    assert!(output_content.contains("# Prologue"));
    assert!(output_content.contains("<p class=\"subtitle\">The Beginning</p>"));

    Ok(())
}

#[test]
fn test_chapter_heading_epilogue_with_dash() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Another non-numbered chapter with dash
    fs::write(&input_path, "## Epilogue - Final Words")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should split using dash as delimiter
    assert!(output_content.contains("# Epilogue"));
    assert!(output_content.contains("<p class=\"subtitle\">Final Words</p>"));

    Ok(())
}

#[test]
fn test_chapter_heading_dash_no_space() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Dash attached to last word of prefix (no space before dash)
    fs::write(&input_path, "## Epilog Bianca- Un an mai târziu")?;

    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should split using dash as delimiter even without space before it
    assert!(output_content.contains("# Epilog Bianca"));
    assert!(output_content.contains("<p class=\"subtitle\">Un an mai târziu</p>"));

    Ok(())
}

#[test]
fn test_scene_break_no_empty_line_above() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Header followed by double newline (scene break) followed by text
    // Headers should NOT have scene breaks after them
    fs::write(&input_path, "## Chapter 1\n\n\nSome text after break.")?;

    Command::new(assert_cmd::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    eprintln!("DEBUG output:\n{}", output_content);

    // Scene break should be REMOVED after the header
    assert!(!output_content.contains("# Chapter 1\n<div class='scene-break'></div>"));
    assert!(!output_content.contains("# Chapter 1\n\n<div class='scene-break'></div>"));

    // Header should still be present (may be converted to H1)
    assert!(output_content.contains("Chapter 1"));

    Ok(())
}

#[test]
fn test_scene_break_before_header() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Text followed by blank line (scene break) followed by header
    // Scene break should be removed before header
    fs::write(&input_path, "Some text.\n\n\n## Chapter 1")?;

    Command::new(assert_cmd::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Scene break should be REMOVED before the header
    assert!(!output_content.contains("<div class='scene-break'></div>\n# Chapter 1"));

    Ok(())
}

#[test]
fn test_scene_break_before_h3_header() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Text followed by blank line (scene break) followed by H3 header
    // Scene break should be removed before H3
    fs::write(&input_path, "Some text.\n\n\n### Section")?;

    Command::new(assert_cmd::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Scene break should be REMOVED before the H3
    assert!(!output_content.contains("<div class='scene-break'></div>\n### Section"));

    Ok(())
}

#[test]
fn test_scene_break_after_subtitle() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Subtitle followed by blank line (scene break) followed by text
    // Scene break should be removed after subtitle
    fs::write(&input_path, "## Chapter 1 - The Beginning\n\n\nSome text")?;

    Command::new(assert_cmd::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Scene break should be REMOVED after the subtitle
    assert!(!output_content.contains("<p class=\"subtitle\">The Beginning</p>\n<div class='scene-break'></div>"));

    Ok(())
}

#[test]
fn test_scene_break_after_h3_header() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // H3 header followed by blank line (scene break) followed by text
    // Scene break should be removed after H3
    fs::write(&input_path, "### Section\n\n\nSome text")?;

    Command::new(assert_cmd::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Scene break should be REMOVED after the H3
    assert!(!output_content.contains("### Section\n<div class='scene-break'></div>"));

    Ok(())
}

#[test]
fn test_proper_spacing_before_header_no_blank_line() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    // Text followed by single newline then header (no blank line originally)
    // Should ensure proper spacing in output
    fs::write(&input_path, "grijile și toate poverile... nu neapărat ale universului.\n## Capitolul 18")?;

    Command::new(assert_cmd::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path)?;

    // Should have at least 2 blank lines (3 newlines) before the header for proper spacing
    // Chapter 18 becomes Chapter 1 due to sequential renumbering
    assert!(output_content.contains("universului.\n\n\n## Capitolul 1") ||
            output_content.contains("universului.\n\n\n# Capitolul 1"));

    // Should NOT have text running directly into header
    assert!(!output_content.contains("universului.\n## Capitolul 1"));
    assert!(!output_content.contains("universului.\n# Capitolul 1"));

    Ok(())
}

#[test]
fn test_no_flags_produces_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");

    fs::write(&input_path, "hello world")?;

    // Run with no flags - should produce error
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg(&input_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No action specified"))
        .stderr(predicate::str::contains("--publish"))
        .stderr(predicate::str::contains("--word-stats"));

    Ok(())
}

#[test]
fn test_publish_only_no_word_stats() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, r#"She said "hello" to me."#)?;

    // Run with --publish only
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"));

    // Verify output file was created
    assert!(output_path.exists());

    // Verify word stats NOT shown in output
    let output = Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg("-v")
        .arg(&input_path)
        .output()?;
    let stdout_str = String::from_utf8(output.stdout)?;
    assert!(!stdout_str.contains("Word Analysis"), "Should not show word analysis without --word-stats");

    Ok(())
}

#[test]
fn test_word_stats_only_no_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "hello world hello rust world world")?;

    // Run with --word-stats only
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--word-stats")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Word Analysis"))
        .stdout(predicate::str::contains("world"));

    // Verify output file was NOT created
    assert!(!output_path.exists());

    Ok(())
}

#[test]
fn test_both_flags_together() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_path = temp_dir.path().join("test.md");
    let output_path = temp_dir.path().join("irie/fixed.md");

    fs::write(&input_path, "hello world hello rust world world")?;

    // Run with both flags
    Command::new(assert_cmd::cargo::cargo_bin!("iriebook"))
        .arg("--publish")
        .arg("--word-stats")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Word Analysis"))
        .stdout(predicate::str::contains("Success"));

    // Verify output file was created
    assert!(output_path.exists());

    Ok(())
}
