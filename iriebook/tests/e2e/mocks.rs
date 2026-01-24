//! Mock implementations for E2E testing
//!
//! This module provides configurable mock implementations of all resource access traits,
//! enabling comprehensive E2E testing without external dependencies.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::e2e::mocks::{MockGitAccess, MockGoogleDocsAccess, MockPandocAccess};
//!
//! let mock_git = Arc::new(MockGitAccess::new()
//!     .with_repo_state(true, false)
//!     .with_sync_state(0, 1));
//!
//! let mock_docs = Arc::new(MockGoogleDocsAccess::new()
//!     .with_document("doc1", "My Novel", "# Chapter 1"));
//! ```

mod git_access;
mod google_docs_access;
mod publication_access;

pub use git_access::{GitCall, MockGitAccess};
pub use google_docs_access::{GoogleDocsCall, MockGoogleDocsAccess};
pub use publication_access::{
    ArchiveCall, CalibreCall, MockArchiveAccess, MockCalibreAccess, MockPandocAccess, PandocCall,
};
