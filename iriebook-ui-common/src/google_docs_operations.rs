//! Google Docs sync operations for UI layer
//!
//! This module provides UI-agnostic functions for Google Docs synchronization
//! that can be reused across different UI implementations (Tauri, Web, TUI, etc.).
//!
//! Following Volatility-Based Design principles, this orchestration logic lives
//! in ui-common rather than in specific UI frameworks, making UIs thin and replaceable.

use crate::processing::BookProcessor;
use crate::ui_state::{PublishEnabled, WordStatsEnabled};
use iriebook::managers::google_docs_sync::GoogleDocsSyncManager;
use iriebook::resource_access::traits::{DocumentSyncer, SyncResult, TokenProvider};
use std::path::Path;

/// Link a book to a Google Doc
///
/// Creates a link between a local book file and a Google Doc by storing
/// the document ID in the book's metadata or configuration.
///
/// # Arguments
/// * `book_path` - Path to the book's markdown file
/// * `doc_id` - Google Doc ID to link (extracted from Doc URL)
/// * `google_docs_manager` - Google Docs sync manager instance
///
/// # Returns
/// * `Ok(())` if link successful
/// * `Err(String)` if link fails (e.g., invalid path, metadata write error)
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::link_document;
/// use iriebook::managers::google_docs_sync::GoogleDocsSyncManager;
/// use iriebook::resource_access::google_docs::GoogleDocsClient;
/// use std::path::Path;
/// use std::sync::Arc;
///
/// # fn example() -> Result<(), String> {
/// let client = Arc::new(GoogleDocsClient::new());
/// let manager = GoogleDocsSyncManager::new(client);
/// link_document(
///     Path::new("/path/to/book.md"),
///     "1234567890abcdef".to_string(),
///     &manager
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn link_document(
    book_path: &Path,
    doc_id: String,
    google_docs_manager: &GoogleDocsSyncManager,
) -> Result<(), String> {
    google_docs_manager
        .link_document(book_path, doc_id)
        .map_err(|e| e.to_string())
}

/// Sync a book from its linked Google Doc with progress events
///
/// Downloads the content from the linked Google Doc, updates the local file,
/// and automatically processes the book (generates ebook files for viewing).
///
/// # Arguments
/// * `book_path` - Path to the book's markdown file
/// * `token_provider` - Token provider for getting valid OAuth token
/// * `document_syncer` - Document syncer for fetching content from remote source
/// * `book_processor` - Book processor for generating ebook files
/// * `progress_callback` - Optional callback for progress events
///
/// # Returns
/// * `Ok(String)` with success message if sync and processing successful
/// * `Err(String)` if sync fails, book not linked, or processing fails
///
/// # Progress Events
///
/// The callback will be invoked with these messages:
/// 1. "Authenticating with Google..."
/// 2. "Syncing document from Google Docs..."
/// 3. "Processing book..."
/// 4. "Sync and processing completed successfully"
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::{sync_document, processing::DefaultBookProcessor};
/// use iriebook::managers::google_docs_sync::GoogleDocsSyncManager;
/// use iriebook::resource_access::google_auth::GoogleAuthenticator;
/// use iriebook::resource_access::google_docs::GoogleDocsClient;
/// use std::path::Path;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<String, String> {
/// // GoogleAuthenticator implements TokenProvider trait
/// let authenticator = GoogleAuthenticator::new();
/// let client = Arc::new(GoogleDocsClient::new());
/// let manager = GoogleDocsSyncManager::new(client);
/// let processor = DefaultBookProcessor;
///
/// let result = sync_document(
///     Path::new("/path/to/book.md"),
///     &authenticator,  // implements TokenProvider
///     &manager,        // implements DocumentSyncer
///     &processor,      // implements BookProcessor
///     Some(|msg: String| println!("Progress: {}", msg))
/// ).await?;
/// # Ok(result)
/// # }
/// ```
pub async fn sync_document<F, T, S, P>(
    book_path: &Path,
    token_provider: &T,
    document_syncer: &S,
    book_processor: &P,
    mut progress_callback: Option<F>,
) -> Result<String, String>
where
    F: FnMut(String),
    T: TokenProvider,
    S: DocumentSyncer,
    P: BookProcessor,
{
    // Report progress: Authentication
    if let Some(ref mut cb) = progress_callback {
        cb("Authenticating with Google...".to_string());
    }

    // Get valid token (will refresh if expired)
    let token = token_provider
        .get_valid_token()
        .await
        .map_err(|e| format!("Not authenticated: {}", e))?;

    // Report progress: Syncing
    if let Some(ref mut cb) = progress_callback {
        cb("Syncing document from Google Docs...".to_string());
    }

    // Sync document
    let result = document_syncer
        .sync_document(book_path, &token)
        .await
        .map_err(|e| e.to_string())?;

    match result {
        SyncResult::Synced => {
            // Report progress: Processing
            if let Some(ref mut cb) = progress_callback {
                cb("Generating ebook...".to_string());
            }

            // Auto-process the book after sync (generate ebook for viewing)
            let processing_result = book_processor.process(
                book_path,
                PublishEnabled::new(true),
                WordStatsEnabled::new(false),
            );

            match processing_result {
                Ok((output, _, _)) => {
                    if let Some(ref mut cb) = progress_callback {
                        cb("Sync and processing completed successfully".to_string());
                    }
                    Ok(format!("Synced and processed successfully\n{}", output))
                }
                Err(e) => {
                    // Sync succeeded but processing failed
                    Err(format!("Synced but processing failed: {}", e))
                }
            }
        }
        SyncResult::NotLinked => Err("Book not linked to Google Doc".to_string()),
    }
}

/// Unlink a book from its Google Doc
///
/// Removes the link between a local book file and its Google Doc.
/// The book content is preserved, but future syncs will not be possible
/// until the book is linked again.
///
/// # Arguments
/// * `book_path` - Path to the book's markdown file
/// * `google_docs_manager` - Google Docs sync manager instance
///
/// # Returns
/// * `Ok(())` if unlink successful
/// * `Err(String)` if unlink fails (e.g., book not linked, metadata write error)
///
/// # Example
/// ```no_run
/// use iriebook_ui_common::unlink_document;
/// use iriebook::managers::google_docs_sync::GoogleDocsSyncManager;
/// use iriebook::resource_access::google_docs::GoogleDocsClient;
/// use std::path::Path;
/// use std::sync::Arc;
///
/// # fn example() -> Result<(), String> {
/// let client = Arc::new(GoogleDocsClient::new());
/// let manager = GoogleDocsSyncManager::new(client);
/// unlink_document(
///     Path::new("/path/to/book.md"),
///     &manager
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn unlink_document(
    book_path: &Path,
    google_docs_manager: &GoogleDocsSyncManager,
) -> Result<(), String> {
    google_docs_manager
        .unlink_document(book_path)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::ProcessingResult;
    use iriebook::utilities::error::IrieBookError;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Mock TokenProvider
    struct MockTokenProvider {
        token: String,
        should_fail: bool,
    }

    impl MockTokenProvider {
        fn new(token: &str) -> Self {
            Self {
                token: token.to_string(),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                token: String::new(),
                should_fail: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl TokenProvider for MockTokenProvider {
        async fn get_valid_token(&self) -> Result<String, IrieBookError> {
            if self.should_fail {
                Err(IrieBookError::GoogleAuth("Mock auth failure".to_string()))
            } else {
                Ok(self.token.clone())
            }
        }
    }

    // Mock DocumentSyncer
    struct MockDocumentSyncer {
        sync_called: Arc<AtomicBool>,
        should_fail: bool,
        return_not_linked: bool,
    }

    impl MockDocumentSyncer {
        fn new(sync_called: Arc<AtomicBool>) -> Self {
            Self {
                sync_called,
                should_fail: false,
                return_not_linked: false,
            }
        }

        fn failing(sync_called: Arc<AtomicBool>) -> Self {
            Self {
                sync_called,
                should_fail: true,
                return_not_linked: false,
            }
        }

        fn not_linked(sync_called: Arc<AtomicBool>) -> Self {
            Self {
                sync_called,
                should_fail: false,
                return_not_linked: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl DocumentSyncer for MockDocumentSyncer {
        async fn sync_document(
            &self,
            _book_path: &Path,
            _token: &str,
        ) -> Result<SyncResult, IrieBookError> {
            self.sync_called.store(true, Ordering::SeqCst);
            if self.should_fail {
                Err(IrieBookError::GoogleDocsApi(
                    "Mock sync failure".to_string(),
                ))
            } else if self.return_not_linked {
                Ok(SyncResult::NotLinked)
            } else {
                Ok(SyncResult::Synced)
            }
        }
    }

    // Mock BookProcessor
    struct MockBookProcessor {
        process_called: Arc<AtomicBool>,
        should_fail: bool,
    }

    impl MockBookProcessor {
        fn new(process_called: Arc<AtomicBool>) -> Self {
            Self {
                process_called,
                should_fail: false,
            }
        }

        fn failing(process_called: Arc<AtomicBool>) -> Self {
            Self {
                process_called,
                should_fail: true,
            }
        }
    }

    impl BookProcessor for MockBookProcessor {
        fn process(
            &self,
            _book_path: &Path,
            _publish: PublishEnabled,
            _word_stats: WordStatsEnabled,
        ) -> ProcessingResult {
            self.process_called.store(true, Ordering::SeqCst);
            if self.should_fail {
                Err(anyhow::anyhow!("Mock processing failure"))
            } else {
                Ok((
                    "Mock processing output".to_string(),
                    None,
                    Some(PathBuf::from("/output/book.epub")),
                ))
            }
        }
    }

    #[tokio::test]
    async fn test_sync_triggers_processing_after_successful_sync() {
        let sync_called = Arc::new(AtomicBool::new(false));
        let process_called = Arc::new(AtomicBool::new(false));

        let token_provider = MockTokenProvider::new("test-token");
        let syncer = MockDocumentSyncer::new(sync_called.clone());
        let processor = MockBookProcessor::new(process_called.clone());

        let result = sync_document(
            Path::new("/test/book.md"),
            &token_provider,
            &syncer,
            &processor,
            None::<fn(String)>,
        )
        .await;

        assert!(result.is_ok());
        assert!(sync_called.load(Ordering::SeqCst), "Sync should be called");
        assert!(
            process_called.load(Ordering::SeqCst),
            "Processing should be called after sync"
        );
        assert!(
            result
                .unwrap()
                .contains("Synced and processed successfully")
        );
    }

    #[tokio::test]
    async fn test_sync_does_not_process_when_not_linked() {
        let sync_called = Arc::new(AtomicBool::new(false));
        let process_called = Arc::new(AtomicBool::new(false));

        let token_provider = MockTokenProvider::new("test-token");
        let syncer = MockDocumentSyncer::not_linked(sync_called.clone());
        let processor = MockBookProcessor::new(process_called.clone());

        let result = sync_document(
            Path::new("/test/book.md"),
            &token_provider,
            &syncer,
            &processor,
            None::<fn(String)>,
        )
        .await;

        assert!(result.is_err());
        assert!(sync_called.load(Ordering::SeqCst), "Sync should be called");
        assert!(
            !process_called.load(Ordering::SeqCst),
            "Processing should NOT be called when not linked"
        );
        assert!(result.unwrap_err().contains("not linked"));
    }

    #[tokio::test]
    async fn test_sync_returns_error_when_processing_fails() {
        let sync_called = Arc::new(AtomicBool::new(false));
        let process_called = Arc::new(AtomicBool::new(false));

        let token_provider = MockTokenProvider::new("test-token");
        let syncer = MockDocumentSyncer::new(sync_called.clone());
        let processor = MockBookProcessor::failing(process_called.clone());

        let result = sync_document(
            Path::new("/test/book.md"),
            &token_provider,
            &syncer,
            &processor,
            None::<fn(String)>,
        )
        .await;

        assert!(result.is_err());
        assert!(sync_called.load(Ordering::SeqCst), "Sync should be called");
        assert!(
            process_called.load(Ordering::SeqCst),
            "Processing should be attempted"
        );
        assert!(result.unwrap_err().contains("Synced but processing failed"));
    }

    #[tokio::test]
    async fn test_sync_does_not_process_when_sync_fails() {
        let sync_called = Arc::new(AtomicBool::new(false));
        let process_called = Arc::new(AtomicBool::new(false));

        let token_provider = MockTokenProvider::new("test-token");
        let syncer = MockDocumentSyncer::failing(sync_called.clone());
        let processor = MockBookProcessor::new(process_called.clone());

        let result = sync_document(
            Path::new("/test/book.md"),
            &token_provider,
            &syncer,
            &processor,
            None::<fn(String)>,
        )
        .await;

        assert!(result.is_err());
        assert!(
            sync_called.load(Ordering::SeqCst),
            "Sync should be attempted"
        );
        assert!(
            !process_called.load(Ordering::SeqCst),
            "Processing should NOT be called when sync fails"
        );
    }

    #[tokio::test]
    async fn test_sync_does_not_proceed_when_auth_fails() {
        let sync_called = Arc::new(AtomicBool::new(false));
        let process_called = Arc::new(AtomicBool::new(false));

        let token_provider = MockTokenProvider::failing();
        let syncer = MockDocumentSyncer::new(sync_called.clone());
        let processor = MockBookProcessor::new(process_called.clone());

        let result = sync_document(
            Path::new("/test/book.md"),
            &token_provider,
            &syncer,
            &processor,
            None::<fn(String)>,
        )
        .await;

        assert!(result.is_err());
        assert!(
            !sync_called.load(Ordering::SeqCst),
            "Sync should NOT be called when auth fails"
        );
        assert!(
            !process_called.load(Ordering::SeqCst),
            "Processing should NOT be called when auth fails"
        );
        assert!(result.unwrap_err().contains("Not authenticated"));
    }

    #[tokio::test]
    async fn test_sync_reports_progress_messages() {
        let progress_messages = Arc::new(std::sync::Mutex::new(Vec::new()));
        let messages_clone = progress_messages.clone();

        let callback = move |msg: String| {
            messages_clone.lock().unwrap().push(msg);
        };

        let sync_called = Arc::new(AtomicBool::new(false));
        let process_called = Arc::new(AtomicBool::new(false));

        let token_provider = MockTokenProvider::new("test-token");
        let syncer = MockDocumentSyncer::new(sync_called);
        let processor = MockBookProcessor::new(process_called);

        let _ = sync_document(
            Path::new("/test/book.md"),
            &token_provider,
            &syncer,
            &processor,
            Some(callback),
        )
        .await;

        let messages = progress_messages.lock().unwrap();
        assert!(messages.contains(&"Authenticating with Google...".to_string()));
        assert!(messages.contains(&"Syncing document from Google Docs...".to_string()));
        assert!(messages.contains(&"Generating ebook...".to_string()));
        assert!(messages.contains(&"Sync and processing completed successfully".to_string()));
    }
}
