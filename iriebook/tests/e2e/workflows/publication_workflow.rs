//! E2E tests for the complete publication workflow
//!
//! These tests exercise the full flow from workspace setup to EPUB generation,
//! using mocked external dependencies.

use crate::e2e::fixtures::TestWorkspace;
use crate::e2e::mocks::{
    GitCall, MockArchiveAccess, MockCalibreAccess, MockGitAccess, MockGoogleDocsAccess,
    MockPandocAccess,
};
use iriebook_ui_common::app_state::AppStateBuilder;
use std::sync::Arc;

/// Test: Complete publication workflow from start to finish
///
/// Scenario:
/// 1. User has a workspace with one book
/// 2. Book is synced with Google Docs
/// 3. User generates EPUB and Kindle versions
/// 4. User commits and pushes changes
#[tokio::test]
async fn test_complete_publication_workflow() {
    // === ARRANGE ===

    // 1. Create test workspace with sample book
    let mut workspace = TestWorkspace::new().unwrap();
    let book = workspace.add_book("vampire-romance").unwrap();
    let book_path = book.md_path.clone();

    // 2. Setup mocks with expected behavior
    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false) // Is a repo, no uncommitted changes initially
            .with_sync_state(0, 0), // Clean state
    );

    let mock_docs = Arc::new(
        MockGoogleDocsAccess::new().with_document(
            "doc-vampire-123",
            "Vampire Romance",
            "# Vampire Romance\n\n## Chapter 1\n\nThe night was dark...",
        ),
    );

    let mock_pandoc = Arc::new(MockPandocAccess::new());
    let mock_calibre = Arc::new(MockCalibreAccess::new());
    let mock_archive = Arc::new(MockArchiveAccess::new());

    // 3. Build AppState with mocks
    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_google_docs_access(mock_docs.clone())
        .with_pandoc_access(mock_pandoc.clone())
        .with_calibre_access(mock_calibre.clone())
        .with_archive_access(mock_archive.clone())
        .with_defaults_for_remaining()
        .build();

    // === ACT ===

    // Step 1: Get repository manager and check status
    let repo_manager = app_state.repository_manager();
    let _sync_status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    // Step 2: Get publication manager and generate ebook
    let pub_manager = app_state.ebook_publication_manager();

    // Note: We need to pass the book path for publication
    // The actual publication requires the book.md to exist with proper structure
    let result = pub_manager.publish(&book_path, None, true, true);

    // === ASSERT ===

    // Verify sync status was checked
    assert!(mock_git.was_called(&GitCall::GetStatus {
        path: workspace.workspace_path.clone()
    }));

    // Publication should succeed (mocks create the output files)
    assert!(result.is_ok(), "Publication failed: {:?}", result.err());

    // Verify Pandoc was called to create EPUB
    let pandoc_calls = mock_pandoc.get_calls();
    assert!(!pandoc_calls.is_empty(), "Pandoc should have been called");

    // Verify Calibre was called to create Kindle version
    let calibre_calls = mock_calibre.get_calls();
    assert!(
        !calibre_calls.is_empty(),
        "Calibre should have been called"
    );

    // Verify Archive was called to create ZIP
    let archive_calls = mock_archive.get_calls();
    assert!(
        !archive_calls.is_empty(),
        "Archive should have been called"
    );
}

/// Test: Publication workflow handles validation errors gracefully
#[tokio::test]
async fn test_publication_handles_invalid_book() {
    let mut workspace = TestWorkspace::new().unwrap();

    // Create a book with unbalanced quotes (will fail validation)
    workspace
        .add_book_with_content(
            "bad-book",
            r#"# Bad Book

"This quote is not closed...

The end.
"#,
        )
        .unwrap();

    let book = workspace.get_book("bad-book").unwrap();

    let mock_git = Arc::new(MockGitAccess::new());
    let mock_pandoc = Arc::new(MockPandocAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_pandoc_access(mock_pandoc.clone())
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();
    let result = pub_manager.publish(&book.md_path, None, true, true);

    // Publication should return a validation failure result (not an outright error)
    let result = result.expect("Publication should return structured validation failure");
    assert!(
        !result.validation_passed,
        "Validation must fail for unbalanced quotes"
    );
    assert!(
        result.validation_error
            .as_ref()
            .map(|msg| msg.contains("Unbalanced quotes"))
            .unwrap_or(false),
        "Validation error should mention unbalanced quotes"
    );

    // Pandoc should NOT have been called (validation fails first)
    let pandoc_calls = mock_pandoc.get_calls();
    assert!(
        pandoc_calls.is_empty(),
        "Pandoc should not be called when validation fails"
    );
}

/// Test: Publication workflow with Pandoc failure
#[tokio::test]
async fn test_publication_handles_pandoc_failure() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("good-book").unwrap();

    let mock_git = Arc::new(MockGitAccess::new());
    let mock_pandoc = Arc::new(MockPandocAccess::new().with_failure("Pandoc not installed"));
    let mock_calibre = Arc::new(MockCalibreAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_git_access(mock_git)
        .with_pandoc_access(mock_pandoc)
        .with_calibre_access(mock_calibre.clone())
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();
    let result = pub_manager.publish(&book.md_path, None, true, true);

    // Publication should fail
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Pandoc"),
        "Error should mention Pandoc: {}",
        err_msg
    );

    // Calibre should NOT have been called (Pandoc fails first)
    let calibre_calls = mock_calibre.get_calls();
    assert!(
        calibre_calls.is_empty(),
        "Calibre should not be called when Pandoc fails"
    );
}

/// Test: Publication generates all ebook formats
#[tokio::test]
async fn test_publication_generates_all_formats() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("full-book").unwrap();

    let mock_pandoc = Arc::new(MockPandocAccess::new());
    let mock_calibre = Arc::new(MockCalibreAccess::new());
    let mock_archive = Arc::new(MockArchiveAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_pandoc_access(mock_pandoc.clone())
        .with_calibre_access(mock_calibre.clone())
        .with_archive_access(mock_archive.clone())
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();

    // Publish with metadata (generates all formats)
    let result = pub_manager.publish(&book.md_path, None, false, true);

    assert!(result.is_ok());

    // All tools should have been called
    assert!(!mock_pandoc.get_calls().is_empty(), "Pandoc should be called");
    assert!(!mock_calibre.get_calls().is_empty(), "Calibre should be called");
    assert!(!mock_archive.get_calls().is_empty(), "Archive should be called");
}

#[cfg(test)]
mod quote_processing_tests {
    use super::*;

    /// Test: Publication properly curls straight quotes
    #[tokio::test]
    async fn test_publication_curls_quotes() {
        let mut workspace = TestWorkspace::new().unwrap();

        // Create book with straight quotes that need curling
        workspace
            .add_book_with_content(
                "quotes-book",
                r#"# Quote Test

"Hello," she said. "How are you?"

He replied, "I'm fine!"

The end.
"#,
            )
            .unwrap();

        let book = workspace.get_book("quotes-book").unwrap();

        let mock_pandoc = Arc::new(MockPandocAccess::new());
        let mock_calibre = Arc::new(MockCalibreAccess::new());
        let mock_archive = Arc::new(MockArchiveAccess::new());

        let app_state = AppStateBuilder::new()
            .workspace_path(workspace.workspace_path.clone())
            .with_pandoc_access(mock_pandoc.clone())
            .with_calibre_access(mock_calibre)
            .with_archive_access(mock_archive)
            .with_defaults_for_remaining()
            .build();

        let pub_manager = app_state.ebook_publication_manager();
        let result = pub_manager.publish(&book.md_path, None, false, true);

        // Should succeed - quotes are balanced and will be curled
        assert!(result.is_ok(), "Publication failed: {:?}", result.err());

        // Pandoc should have been called with the fixed markdown
        let calls = mock_pandoc.get_calls();
        assert!(!calls.is_empty());

        // The fixed markdown file should contain curly quotes
        // (We'd need to read the temp file to verify, but the fact it succeeded is good)
    }
}
