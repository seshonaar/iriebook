//! Test support crate for workflow and UI E2E tests.
//! Provides fixtures and mocked resource access implementations.

pub mod fixtures;
pub mod mocks;

// Re-export commonly used items
pub use fixtures::{TestBook, TestWorkspace};
pub use mocks::{
    ArchiveCall, CalibreCall, GitCall, GoogleDocsCall, MockArchiveAccess, MockCalibreAccess,
    MockGitAccess, MockGoogleDocsAccess, MockPandocAccess, PandocCall,
};
