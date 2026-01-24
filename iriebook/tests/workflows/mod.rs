//! End-to-End Test Infrastructure for IrieBook
//!
//! Headless workflow integration tests bridging the core `iriebook` library and
//! ui-common layer. All external dependencies (Git, Google Docs, Pandoc, Calibre)
//! are mocked for reliable testing. Fixtures and mocks now live in the shared
//! `iriebook-test-support` crate for reuse across Rust and UI E2E tests.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                Workflow Integration Tests                   │
//! │  (tests/workflows/workflows/*.rs)                           │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    AppStateBuilder                          │
//! │  (iriebook-ui-common/src/app_state.rs)                     │
//! │  - Accepts mock trait implementations                       │
//! │  - Creates managers with injected dependencies              │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       Managers                              │
//! │  - RepositoryManager (uses MockGitAccess)                  │
//! │  - GoogleDocsSyncManager (uses MockGoogleDocsAccess)       │
//! │  - EbookPublicationManager (uses MockPandoc/Calibre)       │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │            Mock Implementations (shared crate)             │
//! │  (iriebook-test-support/src/mocks/*.rs)                    │
//! │  - MockGitAccess, MockGoogleDocsAccess, MockPandocAccess   │
//! │  - MockCalibreAccess, MockArchiveAccess                    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage Example
//!
//! ```ignore
//! use iriebook_test_support::{TestWorkspace, MockGitAccess, MockGoogleDocsAccess};
//! use iriebook_ui_common::app_state::AppStateBuilder;
//! use std::sync::Arc;
//!
//! #[tokio::test]
//! async fn test_full_workflow() {
//!     // 1. Create test workspace with sample book
//!     let mut workspace = TestWorkspace::new().unwrap();
//!     workspace.add_book("my-novel").unwrap();
//!
//!     // 2. Configure mocks
//!     let mock_git = Arc::new(MockGitAccess::new()
//!         .with_repo_state(true, false)
//!         .with_sync_state(0, 1));
//!
//!     let mock_docs = Arc::new(MockGoogleDocsAccess::new()
//!         .with_document("doc1", "My Novel", "# Updated content"));
//!
//!     // 3. Build AppState with mocks
//!     let app_state = AppStateBuilder::new()
//!         .workspace_path(workspace.workspace_path.clone())
//!         .with_git_access(mock_git.clone())
//!         .with_google_docs_access(mock_docs)
//!         .with_defaults_for_remaining()
//!         .build();
//!
//!     // 4. Run workflow and verify
//!     let repo_manager = app_state.repository_manager();
//!     let status = repo_manager.get_sync_status(&workspace.workspace_path).unwrap();
//!
//!     // 5. Verify mock was called correctly
//!     assert!(mock_git.was_called(&GitCall::GetStatus { ... }));
//! }
//! ```

pub mod workflows;

// Re-export commonly used items from shared test-support crate
pub use iriebook_test_support::{
    ArchiveCall, CalibreCall, GitCall, GoogleDocsCall, MockArchiveAccess, MockCalibreAccess,
    MockGitAccess, MockGoogleDocsAccess, MockPandocAccess, PandocCall, TestBook, TestWorkspace,
};
