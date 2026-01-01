//! Google Docs Sync Manager
//!
//! Orchestrates high-level Google Docs synchronization workflows.
//! This Manager coordinates sync operations but delegates low-level
//! operations to the GoogleDocsAccess trait implementation and file I/O.
//!
//! Workflows:
//! 1. Sync document from Google Docs to local markdown file
//! 2. Link book to Google Doc
//! 3. Unlink book from Google Doc

use crate::resource_access::file;
use crate::resource_access::traits::{DocumentSyncer, GoogleDocsAccess};
use crate::utilities::error::IrieBookError;
use crate::utilities::types::GoogleDocsSyncInfo;
use std::path::Path;
use std::sync::Arc;

// Re-export SyncResult for backward compatibility
pub use crate::resource_access::traits::SyncResult;

/// Google Docs Sync Manager for orchestrating sync workflows
pub struct GoogleDocsSyncManager {
    docs_access: Arc<dyn GoogleDocsAccess>,
}

impl GoogleDocsSyncManager {
    /// Create a new Google Docs sync manager
    ///
    /// # Arguments
    /// * `docs_access` - Google Docs access implementation (trait object)
    pub fn new(docs_access: Arc<dyn GoogleDocsAccess>) -> Self {
        Self { docs_access }
    }

    /// Sync a book from its linked Google Doc
    ///
    /// # Arguments
    /// * `book_path` - Path to the book's markdown file
    /// * `token` - Valid Google OAuth access token
    ///
    /// # Returns
    /// * `Ok(SyncResult)` with sync details
    /// * `Err(IrieBookError)` if sync fails
    pub async fn sync_document(&self, book_path: &Path, token: &str) -> Result<SyncResult, IrieBookError> {
        // Load sync info - first get the Option result
        let sync_info_option = file::load_google_docs_sync_info(book_path)
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to load sync info: {}", e)))
            ?;
            
        // Then convert Option to Result and unwrap
        let mut sync_info = sync_info_option
            .ok_or_else(|| IrieBookError::GoogleDocsApi("Book not linked to Google Doc".to_string()))
            ?;

        let doc_id = &sync_info.google_doc_id;

        // Fetch document content as markdown from Google Docs
        let markdown_content = self.docs_access.export_as_markdown(doc_id, token).await?;

        // Write new content to file (git will track changes)
        file::write_file(book_path, &markdown_content)
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to write file: {}", e)))?;

        // Update sync info status
        sync_info.mark_synced();
        file::save_google_docs_sync_info(book_path, &sync_info)
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to save sync info: {}", e)))?;

        Ok(SyncResult::Synced)
    }

    /// Link a book to a Google Doc
    ///
    /// # Arguments
    /// * `book_path` - Path to the book's markdown file
    /// * `doc_id` - Google Doc ID to link to
    ///
    /// # Returns
    /// * `Ok(())` if link successful
    /// * `Err(IrieBookError)` if link fails
    pub fn link_document(&self, book_path: &Path, doc_id: String) -> Result<(), IrieBookError> {
        // Create new sync info
        let sync_info = GoogleDocsSyncInfo::new(doc_id);

        // Save sync info file
        file::save_google_docs_sync_info(book_path, &sync_info)
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to save sync info: {}", e)))?;

        Ok(())
    }

    /// Unlink a book from its Google Doc
    ///
    /// # Arguments
    /// * `book_path` - Path to the book's markdown file
    ///
    /// # Returns
    /// * `Ok(())` if unlink successful
    /// * `Err(IrieBookError)` if unlink fails
    pub fn unlink_document(&self, book_path: &Path) -> Result<(), IrieBookError> {
        // Delete sync info file
        file::delete_google_docs_sync_info(book_path)
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to delete sync info: {}", e)))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl DocumentSyncer for GoogleDocsSyncManager {
    async fn sync_document(&self, book_path: &Path, token: &str) -> Result<SyncResult, IrieBookError> {
        // Delegate to the inherent method
        self.sync_document(book_path, token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_access::traits::GoogleDocInfo;
    use std::sync::Arc;

    // Mock implementation for testing
    struct MockGoogleDocsAccess;

    #[async_trait::async_trait]
    impl GoogleDocsAccess for MockGoogleDocsAccess {
        async fn list_documents(&self, _token: &str, _max_results: u32) -> Result<Vec<GoogleDocInfo>, IrieBookError> {
            Ok(vec![])
        }

        async fn export_as_markdown(&self, _doc_id: &str, _token: &str) -> Result<String, IrieBookError> {
            Ok("# Test Document\n\nMocked markdown content".to_string())
        }
    }

    #[test]
    fn manager_can_be_created() {
        let mock_access = Arc::new(MockGoogleDocsAccess);
        let _manager = GoogleDocsSyncManager::new(mock_access);
        // Just testing that construction works
    }
}
