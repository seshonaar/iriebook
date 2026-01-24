//! Workflow Integration Tests for IrieBook
//!
//! Headless workflow coverage across managers using mocked externals (no UI/Tauri).

#[path = "workflows/mod.rs"]
pub mod workflows;

// Re-export for convenience
pub use workflows::*;

// Basic sanity tests to verify the test infrastructure works
#[cfg(test)]
mod infrastructure_tests {
    use super::workflows::{MockGitAccess, MockGoogleDocsAccess, MockPandocAccess, TestWorkspace};

    #[test]
    fn test_fixtures_work() {
        let mut workspace = TestWorkspace::new().unwrap();
        workspace.add_book("test-book").unwrap();

        let book = workspace.get_book("test-book").unwrap();
        assert!(book.md_path.exists());
        assert!(book.content.contains("Chapter 1"));
    }

    #[test]
    fn test_mock_git_works() {
        let mock = MockGitAccess::new()
            .with_repo_state(true, true)
            .with_sync_state(2, 0);

        assert!(mock.is_repo);
        assert!(mock.has_uncommitted);
        assert_eq!(mock.ahead_by, 2);
    }

    #[test]
    fn test_mock_google_docs_works() {
        let mock =
            MockGoogleDocsAccess::new().with_document("doc1", "My Novel", "# Chapter 1\n\nContent");

        assert_eq!(mock.documents.len(), 1);
        assert!(mock.document_content.contains_key("doc1"));
    }

    #[test]
    fn test_mock_pandoc_works() {
        let mock = MockPandocAccess::new();
        assert!(!mock.should_fail);
    }
}
