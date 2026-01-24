//! Mock implementation of GoogleDocsAccess for E2E testing
//!
//! Provides a configurable mock for Google Docs API interactions.

use async_trait::async_trait;
use iriebook::resource_access::traits::{GoogleDocInfo, GoogleDocsAccess};
use iriebook::utilities::error::IrieBookError;
use std::collections::HashMap;
use std::sync::Mutex;

/// Records of Google Docs operations for test verification
#[derive(Debug, Clone, PartialEq)]
pub enum GoogleDocsCall {
    ListDocuments { max_results: u32 },
    ExportAsMarkdown { doc_id: String },
}

/// Mock GoogleDocsAccess implementation with configurable behavior
pub struct MockGoogleDocsAccess {
    /// Documents to return from list_documents
    pub documents: Vec<GoogleDocInfo>,
    /// Document content keyed by doc_id
    pub document_content: HashMap<String, String>,
    /// Whether operations should fail
    pub should_fail: bool,
    /// Error message when failing
    pub error_message: String,
    /// Recorded calls for verification
    calls: Mutex<Vec<GoogleDocsCall>>,
}

impl Default for MockGoogleDocsAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGoogleDocsAccess {
    /// Create a new mock with default configuration
    pub fn new() -> Self {
        Self {
            documents: vec![],
            document_content: HashMap::new(),
            should_fail: false,
            error_message: "Mock Google Docs error".to_string(),
            calls: Mutex::new(vec![]),
        }
    }

    /// Add a document to the mock
    pub fn with_document(mut self, id: &str, name: &str, content: &str) -> Self {
        self.documents.push(GoogleDocInfo::new(
            id.to_string(),
            name.to_string(),
            "2024-01-01T00:00:00Z".to_string(),
        ));
        self.document_content
            .insert(id.to_string(), content.to_string());
        self
    }

    /// Add multiple documents
    pub fn with_documents(mut self, docs: Vec<(String, String, String)>) -> Self {
        for (id, name, content) in docs {
            self.documents.push(GoogleDocInfo::new(
                id.clone(),
                name,
                "2024-01-01T00:00:00Z".to_string(),
            ));
            self.document_content.insert(id, content);
        }
        self
    }

    /// Make all operations fail
    pub fn with_failure(mut self, message: &str) -> Self {
        self.should_fail = true;
        self.error_message = message.to_string();
        self
    }

    /// Get all recorded calls
    pub fn get_calls(&self) -> Vec<GoogleDocsCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Check if a specific call was made
    pub fn was_called(&self, expected: &GoogleDocsCall) -> bool {
        self.calls.lock().unwrap().contains(expected)
    }

    /// Clear recorded calls
    pub fn clear_calls(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Record a call
    fn record(&self, call: GoogleDocsCall) {
        self.calls.lock().unwrap().push(call);
    }
}

#[async_trait]
impl GoogleDocsAccess for MockGoogleDocsAccess {
    async fn list_documents(
        &self,
        _token: &str,
        max_results: u32,
    ) -> Result<Vec<GoogleDocInfo>, IrieBookError> {
        self.record(GoogleDocsCall::ListDocuments { max_results });

        if self.should_fail {
            return Err(IrieBookError::GoogleDocsApi(self.error_message.clone()));
        }

        let limit = max_results as usize;
        Ok(self.documents.iter().take(limit).cloned().collect())
    }

    async fn export_as_markdown(
        &self,
        doc_id: &str,
        _token: &str,
    ) -> Result<String, IrieBookError> {
        self.record(GoogleDocsCall::ExportAsMarkdown {
            doc_id: doc_id.to_string(),
        });

        if self.should_fail {
            return Err(IrieBookError::GoogleDocsApi(self.error_message.clone()));
        }

        self.document_content
            .get(doc_id)
            .cloned()
            .ok_or_else(|| IrieBookError::GoogleDocNotFound(doc_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_google_docs_list_documents() {
        let mock = MockGoogleDocsAccess::new()
            .with_document("doc1", "My Novel", "# Chapter 1\n\nContent here");

        let docs = mock.list_documents("fake-token", 10).await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "doc1");
        assert_eq!(docs[0].name, "My Novel");
    }

    #[tokio::test]
    async fn test_mock_google_docs_export_markdown() {
        let mock = MockGoogleDocsAccess::new()
            .with_document("doc1", "My Novel", "# Chapter 1\n\nContent here");

        let content = mock.export_as_markdown("doc1", "fake-token").await.unwrap();
        assert!(content.contains("Chapter 1"));
    }

    #[tokio::test]
    async fn test_mock_google_docs_missing_document() {
        let mock = MockGoogleDocsAccess::new();

        let result = mock.export_as_markdown("nonexistent", "fake-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_google_docs_can_fail() {
        let mock = MockGoogleDocsAccess::new().with_failure("API quota exceeded");

        let result = mock.list_documents("fake-token", 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_google_docs_records_calls() {
        let mock = MockGoogleDocsAccess::new()
            .with_document("doc1", "Test", "Content");

        let _ = mock.list_documents("token", 50).await;
        let _ = mock.export_as_markdown("doc1", "token").await;

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2);
        assert!(mock.was_called(&GoogleDocsCall::ListDocuments { max_results: 50 }));
        assert!(mock.was_called(&GoogleDocsCall::ExportAsMarkdown {
            doc_id: "doc1".to_string()
        }));
    }
}
