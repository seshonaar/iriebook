//! E2E Workflow Tests
//!
//! This module contains headless workflow integration tests exercising
//! complete user workflows through the application (mocked externals, no UI/Tauri).
//!
//! ## Available Workflow Tests
//!
//! - **publication_workflow**: Tests for EPUB/Kindle generation
//! - **git_sync_workflow**: Tests for Git repository synchronization
//! - **google_docs_workflow**: Tests for Google Docs integration
//! - **diff_workflow**: Tests for viewing changes and diffs

mod diff_workflow;
mod git_sync_workflow;
mod google_docs_workflow;
mod publication_workflow;
