//! E2E tests for Google Docs synchronization workflows
//!
//! These tests exercise the Google Docs linking, syncing, and unlinking flows.

use iriebook_test_support::{GoogleDocsCall, MockGoogleDocsAccess, TestWorkspace};
use iriebook_ui_common::app_state::AppStateBuilder;
use std::sync::Arc;

/// Test: List available Google Docs
#[tokio::test]
async fn test_list_google_docs() {
    let workspace = TestWorkspace::new().unwrap();

    let mock_docs = Arc::new(
        MockGoogleDocsAccess::new()
            .with_document("doc1", "Novel Draft 1", "# Draft content")
            .with_document("doc2", "Novel Draft 2", "# More content")
            .with_document("doc3", "Short Story", "# Story"),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_google_docs_access(mock_docs.clone())
        .with_defaults_for_remaining()
        .build();

    // Get the docs client through AppState
    let docs_client = app_state.google_docs_client();

    // List documents
    let docs = docs_client.list_documents("fake-token", 10).await.unwrap();

    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].name, "Novel Draft 1");
    assert_eq!(docs[1].name, "Novel Draft 2");
    assert_eq!(docs[2].name, "Short Story");

    // Verify the call was made
    assert!(mock_docs.was_called(&GoogleDocsCall::ListDocuments { max_results: 10 }));
}

/// Test: Export Google Doc as markdown
#[tokio::test]
async fn test_export_google_doc_as_markdown() {
    let workspace = TestWorkspace::new().unwrap();

    let expected_content = r#"# My Vampire Novel

## Chapter 1: The Awakening

She opened her eyes to darkness...

## Chapter 2: The Hunt

The moonlight cast long shadows...
"#;

    let mock_docs = Arc::new(MockGoogleDocsAccess::new().with_document(
        "vampire-doc-id",
        "My Vampire Novel",
        expected_content,
    ));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_google_docs_access(mock_docs.clone())
        .with_defaults_for_remaining()
        .build();

    let docs_client = app_state.google_docs_client();

    // Export document
    let content = docs_client
        .export_as_markdown("vampire-doc-id", "fake-token")
        .await
        .unwrap();

    assert!(content.contains("My Vampire Novel"));
    assert!(content.contains("The Awakening"));
    assert!(content.contains("The Hunt"));

    // Verify the call was made
    assert!(mock_docs.was_called(&GoogleDocsCall::ExportAsMarkdown {
        doc_id: "vampire-doc-id".to_string()
    }));
}

/// Test: Link book to Google Doc and sync
#[tokio::test]
async fn test_link_and_sync_google_doc() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("my-novel").unwrap();
    let _book_md_path = book.md_path.clone();

    let updated_content = r#"# My Novel - Updated

## Chapter 1

New content from Google Docs!
"#;

    let mock_docs = Arc::new(MockGoogleDocsAccess::new().with_document(
        "linked-doc-id",
        "My Novel",
        updated_content,
    ));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_google_docs_access(mock_docs.clone())
        .with_defaults_for_remaining()
        .build();

    let docs_manager = app_state.google_docs_manager();

    // Link the book to a Google Doc
    let link_result = docs_manager.link_document(&book.md_path, "linked-doc-id".to_string());
    assert!(link_result.is_ok());

    // Sync the document
    let sync_result = docs_manager
        .sync_document(&book.md_path, "fake-token")
        .await;

    assert!(sync_result.is_ok());

    // Verify export was called
    assert!(mock_docs.was_called(&GoogleDocsCall::ExportAsMarkdown {
        doc_id: "linked-doc-id".to_string()
    }));

    // The book content should now be updated
    let new_content = std::fs::read_to_string(&book.md_path).unwrap();
    assert!(
        new_content.contains("Updated") || new_content.contains("New content"),
        "Book should be updated with Google Docs content"
    );
}

/// Test: Sync unlinked document returns NotLinked
#[tokio::test]
async fn test_sync_unlinked_document() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("unlinked-book").unwrap();

    let mock_docs = Arc::new(MockGoogleDocsAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_google_docs_access(mock_docs.clone())
        .with_defaults_for_remaining()
        .build();

    let docs_manager = app_state.google_docs_manager();

    // Try to sync without linking first
    let result = docs_manager
        .sync_document(&book.md_path, "fake-token")
        .await;

    // Should indicate not linked (not an error, just a status)
    assert!(result.is_ok());
    let sync_result = result.unwrap();

    // The result should indicate the document is not linked
    assert_eq!(
        sync_result,
        iriebook::resource_access::traits::SyncResult::NotLinked
    );

    // Google Docs should NOT have been called
    assert!(
        mock_docs.get_calls().is_empty(),
        "Should not call Google Docs for unlinked document"
    );
}

/// Test: Unlink document from Google Docs
#[tokio::test]
async fn test_unlink_google_doc() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("linked-book").unwrap();
    let book_md_path = book.md_path.clone();

    // Simulate that the book is already linked
    workspace
        .link_book_to_google_doc("linked-book", "existing-doc-id")
        .unwrap();

    let mock_docs = Arc::new(MockGoogleDocsAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_google_docs_access(mock_docs)
        .with_defaults_for_remaining()
        .build();

    let docs_manager = app_state.google_docs_manager();

    // Unlink the document
    let unlink_result = docs_manager.unlink_document(&book_md_path);
    assert!(unlink_result.is_ok());

    // After unlinking, sync should return NotLinked
    let sync_result = docs_manager
        .sync_document(&book_md_path, "fake-token")
        .await
        .unwrap();

    assert_eq!(
        sync_result,
        iriebook::resource_access::traits::SyncResult::NotLinked
    );
}

/// Test: Google Docs API failure is handled gracefully
#[tokio::test]
async fn test_google_docs_api_failure() {
    let mut workspace = TestWorkspace::new().unwrap();
    let _book = workspace.add_book("api-fail-book").unwrap();

    let mock_docs = Arc::new(MockGoogleDocsAccess::new().with_failure("API quota exceeded"));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_google_docs_access(mock_docs)
        .with_defaults_for_remaining()
        .build();

    let docs_client = app_state.google_docs_client();

    // List should fail
    let result = docs_client.list_documents("fake-token", 10).await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("quota") || err_msg.contains("API"),
        "Error should mention API failure: {}",
        err_msg
    );
}

/// Test: Document not found in Google Docs
#[tokio::test]
async fn test_google_doc_not_found() {
    let workspace = TestWorkspace::new().unwrap();

    // Create mock with some documents, but not the one we'll request
    let mock_docs =
        Arc::new(MockGoogleDocsAccess::new().with_document("doc1", "Some Doc", "content"));

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_google_docs_access(mock_docs)
        .with_defaults_for_remaining()
        .build();

    let docs_client = app_state.google_docs_client();

    // Try to export a non-existent document
    let result = docs_client
        .export_as_markdown("nonexistent-doc-id", "fake-token")
        .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("nonexistent"),
        "Error should indicate document not found: {}",
        err_msg
    );
}

/// Test: Multiple books linked to different Google Docs
#[tokio::test]
async fn test_multiple_books_linked() {
    let mut workspace = TestWorkspace::new().unwrap();

    workspace.add_book("book-one").unwrap();
    workspace.add_book("book-two").unwrap();
    workspace.add_book("book-three").unwrap();

    let mock_docs = Arc::new(
        MockGoogleDocsAccess::new()
            .with_document("doc-one", "Book One", "# Book One\n\nContent one")
            .with_document("doc-two", "Book Two", "# Book Two\n\nContent two")
            .with_document("doc-three", "Book Three", "# Book Three\n\nContent three"),
    );

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_google_docs_access(mock_docs.clone())
        .with_defaults_for_remaining()
        .build();

    let docs_manager = app_state.google_docs_manager();

    // Link all three books
    let book_one = workspace.get_book("book-one").unwrap();
    let book_two = workspace.get_book("book-two").unwrap();
    let book_three = workspace.get_book("book-three").unwrap();

    docs_manager
        .link_document(&book_one.md_path, "doc-one".to_string())
        .unwrap();
    docs_manager
        .link_document(&book_two.md_path, "doc-two".to_string())
        .unwrap();
    docs_manager
        .link_document(&book_three.md_path, "doc-three".to_string())
        .unwrap();

    // Sync all three
    docs_manager
        .sync_document(&book_one.md_path, "token")
        .await
        .unwrap();
    docs_manager
        .sync_document(&book_two.md_path, "token")
        .await
        .unwrap();
    docs_manager
        .sync_document(&book_three.md_path, "token")
        .await
        .unwrap();

    // Verify all three docs were exported
    let calls = mock_docs.get_calls();
    let export_calls: Vec<_> = calls
        .iter()
        .filter(|c| matches!(c, GoogleDocsCall::ExportAsMarkdown { .. }))
        .collect();

    assert_eq!(export_calls.len(), 3, "Should export all three documents");
}
