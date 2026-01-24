//! Google Docs API client
//!
//! Handles listing Google Docs and exporting them as markdown using Google Drive API

use crate::resource_access::traits::{GoogleDocInfo, GoogleDocsAccess};
use crate::utilities::error::IrieBookError;
use serde::Deserialize;

/// Google Docs API client
pub struct GoogleDocsClient {
    client: reqwest::Client,
}

/// Google Drive API response for files.list
#[derive(Debug, Deserialize)]
struct FilesListResponse {
    files: Vec<FileInfo>,
}

/// File information from Google Drive API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FileInfo {
    id: String,
    name: String,
    #[serde(rename = "modifiedTime")]
    modified_time: String,
}

impl GoogleDocsClient {
    /// Create a new Google Docs client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn get_api_base_url(&self) -> String {
        std::env::var("GOOGLE_DOCS_API_URL")
            .unwrap_or_else(|_| "https://www.googleapis.com/drive/v3".to_string())
    }
}

#[async_trait::async_trait]
impl GoogleDocsAccess for GoogleDocsClient {
    async fn list_documents(&self, token: &str, max_results: u32) -> Result<Vec<GoogleDocInfo>, IrieBookError> {
        // Use Drive API to list Google Docs files
        let base_url = self.get_api_base_url();
        let url = format!(
            "{}/files?\
             q=mimeType='application/vnd.google-apps.document'&\
             fields=files(id,name,modifiedTime)&\
             pageSize={}&\
             orderBy=modifiedTime desc",
            base_url, max_results
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| IrieBookError::Network(format!("Failed to list documents: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(IrieBookError::GoogleDocsApi(format!(
                "Google Drive API returned status: {}",
                status
            )));
        }

        let files_response: FilesListResponse = response
            .json()
            .await
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to parse response: {}", e)))?;

        let docs = files_response
            .files
            .into_iter()
            .map(|f| GoogleDocInfo::new(f.id, f.name, f.modified_time))
            .collect();

        Ok(docs)
    }

    async fn export_as_markdown(&self, doc_id: &str, token: &str) -> Result<String, IrieBookError> {
        // Use Drive API export endpoint with markdown mime type
        let base_url = self.get_api_base_url();
        let url = format!(
            "{}/files/{}/export?mimeType=text/markdown",
            base_url, doc_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| IrieBookError::Network(format!("Failed to export document: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(IrieBookError::GoogleDocNotFound(format!(
                    "Document {} not found or access denied",
                    doc_id
                )));
            }
            return Err(IrieBookError::GoogleDocsApi(format!(
                "Google Drive API returned status: {}",
                status
            )));
        }

        let markdown_content = response
            .text()
            .await
            .map_err(|e| IrieBookError::GoogleDocsApi(format!("Failed to read response: {}", e)))?;

        Ok(markdown_content)
    }
}

impl Default for GoogleDocsClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_can_be_created() {
        let _client = GoogleDocsClient::new();
        // Just testing that construction works
    }

    #[test]
    fn google_doc_info_can_be_created() {
        let info = GoogleDocInfo::new(
            "doc-id-123".to_string(),
            "My Document".to_string(),
            "2025-01-10T12:00:00Z".to_string(),
        );

        assert_eq!(info.id, "doc-id-123");
        assert_eq!(info.name, "My Document");
        assert_eq!(info.modified_time, "2025-01-10T12:00:00Z");
    }
}