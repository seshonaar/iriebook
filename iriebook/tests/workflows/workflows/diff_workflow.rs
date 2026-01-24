//! E2E tests for viewing changes and diffs
//!
//! These tests exercise the diff computation and revision history flows.

use iriebook::utilities::types::GitCommit;
use iriebook_test_support::{MockGitAccess, TestWorkspace};
use iriebook_ui_common::app_state::AppStateBuilder;
use std::sync::Arc;

/// Test: Get commit history for a repository
#[tokio::test]
async fn test_get_commit_history() {
    let workspace = TestWorkspace::new().unwrap();

    let commits = vec![
        GitCommit {
            hash: "abc123".to_string(),
            message: "Updated chapter 3".to_string(),
            author: "Author Name".to_string(),
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        },
        GitCommit {
            hash: "def456".to_string(),
            message: "Fixed typos in chapter 2".to_string(),
            author: "Author Name".to_string(),
            timestamp: "2024-01-14T15:00:00Z".to_string(),
        },
        GitCommit {
            hash: "ghi789".to_string(),
            message: "Initial commit".to_string(),
            author: "Author Name".to_string(),
            timestamp: "2024-01-10T09:00:00Z".to_string(),
        },
    ];

    let mock_git = Arc::new(MockGitAccess::new().with_commits(commits.clone()));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Get commit log
    let history = repo_manager
        .get_history(&workspace.workspace_path, 10)
        .unwrap();

    assert_eq!(history.len(), 3);
    assert_eq!(history[0].message, "Updated chapter 3");
    assert_eq!(history[1].message, "Fixed typos in chapter 2");
    assert_eq!(history[2].message, "Initial commit");
}

/// Test: Empty commit history for new repository
#[tokio::test]
async fn test_empty_commit_history() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(MockGitAccess::new().with_commits(vec![]));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    let history = repo_manager
        .get_history(&workspace.workspace_path, 10)
        .unwrap();

    assert!(history.is_empty());
}

/// Test: DiffManager is accessible from AppState
#[tokio::test]
async fn test_diff_manager_accessible() {
    let mut workspace = TestWorkspace::new().unwrap();
    workspace.add_book("test-book").unwrap();

    let mock_git = Arc::new(MockGitAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    // Verify diff manager is accessible
    let diff_manager = app_state.diff_manager();

    // The diff manager should be properly initialized
    // (actual diff operations would require real git history)
    let _dm = diff_manager; // Just verify it exists
}

/// Test: Sync status reflects dirty state for diff visualization
#[tokio::test]
async fn test_dirty_status_indicates_changes() {
    let mut workspace = TestWorkspace::new().unwrap();
    workspace.add_book("modified-book").unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, true) // Has uncommitted changes
            .with_changed_files(vec!["modified-book/book.md".to_string()]),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Check sync status - should be dirty
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(
        status,
        iriebook::managers::repository_manager::GitSyncStatus::Dirty
    );
}

/// Test: Clean workspace has no changes to show
#[tokio::test]
async fn test_clean_workspace_no_changes() {
    let mut workspace = TestWorkspace::new().unwrap();
    workspace.add_book("clean-book").unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false) // No uncommitted changes
            .with_changed_files(vec![]), // No changed files
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Check sync status - should be clean
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(
        status,
        iriebook::managers::repository_manager::GitSyncStatus::Clean
    );
}
