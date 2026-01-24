//! E2E tests for Git synchronization workflows
//!
//! These tests exercise the repository synchronization flow including
//! status checking, pulling, committing, and pushing.

use crate::e2e::fixtures::TestWorkspace;
use crate::e2e::mocks::{GitCall, MockGitAccess};
use iriebook::managers::repository_manager::GitSyncStatus;
use iriebook_ui_common::app_state::AppStateBuilder;
use std::sync::Arc;

/// Test: Workspace sync status - clean repository
#[tokio::test]
async fn test_sync_status_clean_repository() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false) // Is repo, no uncommitted
            .with_sync_state(0, 0),       // Not ahead, not behind
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(status, GitSyncStatus::Clean);

    // Verify status was checked
    assert!(mock_git.was_called(&GitCall::GetStatus {
        path: workspace.workspace_path.clone()
    }));
}

/// Test: Workspace sync status - needs pull (behind remote)
#[tokio::test]
async fn test_sync_status_needs_pull() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false)
            .with_sync_state(0, 3), // 3 commits behind
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(status, GitSyncStatus::NeedsPull);
}

/// Test: Workspace sync status - needs push (ahead of remote)
#[tokio::test]
async fn test_sync_status_needs_push() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false)
            .with_sync_state(2, 0), // 2 commits ahead
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(status, GitSyncStatus::NeedsPush);
}

/// Test: Workspace sync status - has uncommitted changes (dirty)
#[tokio::test]
async fn test_sync_status_dirty() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, true) // Has uncommitted changes
            .with_sync_state(0, 0),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(status, GitSyncStatus::Dirty);
}

/// Test: Workspace sync status - not a repository
#[tokio::test]
async fn test_sync_status_uninitialized() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(MockGitAccess::new().with_repo_state(false, false));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();
    let status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    assert_eq!(status, GitSyncStatus::Uninitialized);
}

/// Test: Full sync workflow - pull and push
#[tokio::test]
async fn test_full_sync_workflow() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false)
            .with_sync_state(1, 2), // 1 ahead, 2 behind - needs both pull and push
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Sync workspace (this should pull and push)
    let result = repo_manager
        .sync_workspace(&workspace.workspace_path, "fake-token")
        .await;

    assert!(result.is_ok());

    // Verify the operations were called
    let calls = mock_git.get_calls();

    // Should have prepared for rebase
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, GitCall::PrepareForRebase { .. })),
        "Should prepare for rebase"
    );

    // Should have pulled
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, GitCall::PullRebaseOurs { .. })),
        "Should pull from remote"
    );

    // Should have pushed (since we're ahead)
    assert!(
        calls.iter().any(|c| matches!(c, GitCall::Push { .. })),
        "Should push to remote"
    );
}

/// Test: Save workspace workflow - add, commit, push
#[tokio::test]
async fn test_save_workspace_workflow() {
    let mut workspace = TestWorkspace::new().unwrap();
    workspace.add_book("my-book").unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, true) // Has uncommitted changes
            .with_sync_state(0, 0),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Save workspace with a commit message
    let result = repo_manager
        .save_workspace(&workspace.workspace_path, "Updated my-book", "fake-token")
        .await;

    assert!(result.is_ok());

    // Verify the operations were called in order
    let calls = mock_git.get_calls();

    // Should have checked for uncommitted changes
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, GitCall::HasUncommittedChanges { .. })),
        "Should check for uncommitted changes"
    );

    // Should have added all changes
    assert!(
        calls.iter().any(|c| matches!(c, GitCall::AddAll { .. })),
        "Should add all changes"
    );

    // Should have committed with message
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, GitCall::Commit { message, .. } if message.contains("Updated my-book"))),
        "Should commit with correct message"
    );

    // Should have pushed
    assert!(
        calls.iter().any(|c| matches!(c, GitCall::Push { .. })),
        "Should push to remote"
    );
}

/// Test: Initialize workspace (clone repository)
#[tokio::test]
async fn test_initialize_workspace() {
    let workspace = TestWorkspace::new().unwrap();
    let clone_path = workspace.workspace_path.join("new-repo");

    // Create the directory that the clone would create
    std::fs::create_dir_all(&clone_path).unwrap();

    let mock_git = Arc::new(MockGitAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Initialize workspace (clones repository)
    let result = repo_manager
        .initialize_workspace(
            &clone_path,
            "https://github.com/user/repo.git",
            "fake-token",
        )
        .await;

    assert!(result.is_ok());

    // Verify clone was called with correct URL
    assert!(mock_git.was_called(&GitCall::Clone {
        url: "https://github.com/user/repo.git".to_string(),
        path: clone_path,
    }));
}

/// Test: Get commit history
#[tokio::test]
async fn test_get_commit_history() {
    let workspace = TestWorkspace::new().unwrap();

    let commits = vec![
        iriebook::utilities::types::GitCommit {
            hash: "abc123".to_string(),
            message: "Updated chapter 3".to_string(),
            author: "Author Name".to_string(),
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        },
        iriebook::utilities::types::GitCommit {
            hash: "def456".to_string(),
            message: "Fixed typos".to_string(),
            author: "Author Name".to_string(),
            timestamp: "2024-01-14T15:00:00Z".to_string(),
        },
    ];

    let mock_git = Arc::new(MockGitAccess::new().with_commits(commits));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    let history = repo_manager
        .get_history(&workspace.workspace_path, 10)
        .unwrap();

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].message, "Updated chapter 3");
    assert_eq!(history[1].message, "Fixed typos");
}

/// Test: Sync fails gracefully when git operations fail
#[tokio::test]
async fn test_sync_handles_git_failure() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false)
            .with_sync_state(0, 1)
            .with_failure("Network timeout"),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    // Sync should fail
    let result = repo_manager
        .sync_workspace(&workspace.workspace_path, "fake-token")
        .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Network timeout") || err_msg.contains("Git"),
        "Error should mention the failure: {}",
        err_msg
    );
}

/// Test: Save workspace when nothing to commit
#[tokio::test]
async fn test_save_workspace_nothing_to_commit() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false) // No uncommitted changes
            .with_sync_state(0, 0),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_defaults_for_remaining()
        .build();

    let repo_manager = app_state.repository_manager();

    let result = repo_manager
        .save_workspace(&workspace.workspace_path, "No changes", "fake-token")
        .await;

    assert!(result.is_ok());

    // Should have checked for changes but not committed
    let calls = mock_git.get_calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, GitCall::HasUncommittedChanges { .. })),
        "Should check for uncommitted changes"
    );

    // Should NOT have added or committed (no changes)
    assert!(
        !calls.iter().any(|c| matches!(c, GitCall::AddAll { .. })),
        "Should not add when no changes"
    );
}
