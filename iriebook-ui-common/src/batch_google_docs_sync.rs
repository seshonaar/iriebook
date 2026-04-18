//! Batch Google Docs sync orchestration
//!
//! This module moves batch sync orchestration logic out of the UI layer
//! into a framework-agnostic module, following the BatchProcessor pattern.
//!
//! Following Volatility-Based Design:
//! - The UI layer should NOT contain orchestration loops
//! - Progress callbacks abstract away framework-specific events
//! - This logic can be reused in CLI, web UI, or any other interface

use crate::{
    google_docs_operations::sync_document,
    processing::{BookProcessor, GoogleDocsBatchSyncEvent},
    ui_state::BookInfo,
};
use iriebook::resource_access::traits::{DocumentSyncer, TokenProvider};
use std::path::PathBuf;
use std::sync::Arc;

/// Batch processor for syncing multiple books from Google Docs
pub struct BatchGoogleDocsSyncProcessor {
    config_root: PathBuf,
}

impl BatchGoogleDocsSyncProcessor {
    pub fn new(config_root: PathBuf) -> Self {
        Self { config_root }
    }

    /// Sync multiple books from their linked Google Docs
    ///
    /// Spawns background task that:
    /// 1. Filters to linked books (safety check)
    /// 2. Processes each book sequentially
    /// 3. For each book: auth → sync → process (via sync_document)
    /// 4. Emits progress events
    /// 5. Continues on individual failures
    /// 6. Returns summary in AllDone event
    ///
    /// # Arguments
    /// * `books` - List of books to sync (will filter to linked books)
    /// * `token_provider` - OAuth token provider for authentication
    /// * `document_syncer` - Google Docs sync manager
    /// * `book_processor` - Book processor for generating ebooks after sync
    /// * `progress_callback` - Callback for progress events
    ///
    /// # Returns
    /// * `Ok(())` if batch sync task spawned successfully
    /// * `Err(String)` if no books are linked to Google Docs
    pub async fn sync_books<F, T, S, P>(
        &self,
        books: Vec<BookInfo>,
        token_provider: Arc<T>,
        document_syncer: Arc<S>,
        book_processor: Arc<P>,
        progress_callback: F,
    ) -> Result<(), String>
    where
        F: Fn(GoogleDocsBatchSyncEvent) + Send + 'static + Clone,
        T: TokenProvider + 'static,
        S: DocumentSyncer + 'static,
        P: BookProcessor + 'static,
    {
        // Filter to linked books
        let linked_books: Vec<_> = books
            .into_iter()
            .filter(|b| b.google_docs_sync_info.is_some())
            .collect();

        if linked_books.is_empty() {
            return Err("No books are linked to Google Docs".to_string());
        }

        // Spawn background task
        let config_root = self.config_root.clone();

        tokio::spawn(async move {
            let mut success_count = 0;
            let mut fail_count = 0;

            for (index, book) in linked_books.iter().enumerate() {
                // Emit Started event
                progress_callback(GoogleDocsBatchSyncEvent::Started {
                    book_index: index,
                    book_name: book.display_name.clone(),
                });

                // Create progress sub-callback for per-book updates
                let callback_clone = progress_callback.clone();
                let book_name = book.display_name.clone();
                let progress_sub_callback = move |msg: String| {
                    callback_clone(GoogleDocsBatchSyncEvent::Progress {
                        book_index: index,
                        book_name: book_name.clone(),
                        message: msg,
                    });
                };

                // Sync document (handles auth + sync + process)
                let result = sync_document(
                    book.path.as_path(),
                    Some(config_root.as_path()),
                    &*token_provider,
                    &*document_syncer,
                    &*book_processor,
                    Some(progress_sub_callback),
                )
                .await;

                // Emit Completed event
                match result {
                    Ok(msg) => {
                        success_count += 1;
                        progress_callback(GoogleDocsBatchSyncEvent::Completed {
                            book_index: index,
                            book_name: book.display_name.clone(),
                            success: true,
                            message: msg,
                        });
                    }
                    Err(e) => {
                        fail_count += 1;
                        progress_callback(GoogleDocsBatchSyncEvent::Completed {
                            book_index: index,
                            book_name: book.display_name.clone(),
                            success: false,
                            message: e,
                        });
                    }
                }
            }

            // Emit AllDone event
            progress_callback(GoogleDocsBatchSyncEvent::AllDone {
                total_books: linked_books.len(),
                success_count,
                fail_count,
            });
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        processing::ProcessingResult,
        ui_state::{BookPath, PublishEnabled, WordStatsEnabled},
    };
    use iriebook::{
        resource_access::traits::SyncResult,
        utilities::error::IrieBookError,
    };
    use std::{
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicBool, Ordering},
            Mutex,
        },
    };

    // Mock TokenProvider
    struct MockTokenProvider {
        token: String,
    }

    impl MockTokenProvider {
        fn new(token: &str) -> Self {
            Self {
                token: token.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl TokenProvider for MockTokenProvider {
        async fn get_valid_token(&self) -> Result<String, IrieBookError> {
            Ok(self.token.clone())
        }
    }

    // Mock DocumentSyncer
    struct MockDocumentSyncer {
        should_fail_paths: Vec<String>,
    }

    impl MockDocumentSyncer {
        fn new() -> Self {
            Self {
                should_fail_paths: Vec::new(),
            }
        }

        fn with_failing_paths(paths: Vec<String>) -> Self {
            Self {
                should_fail_paths: paths,
            }
        }
    }

    #[async_trait::async_trait]
    impl DocumentSyncer for MockDocumentSyncer {
        async fn sync_document(
            &self,
            book_path: &Path,
            _token: &str,
        ) -> Result<SyncResult, IrieBookError> {
            let path_str = book_path.to_string_lossy().to_string();
            if self.should_fail_paths.contains(&path_str) {
                Err(IrieBookError::GoogleDocsApi(
                    "Mock sync failure".to_string(),
                ))
            } else {
                Ok(SyncResult::Synced)
            }
        }
    }

    // Mock BookProcessor
    struct MockBookProcessor {
        process_called: Arc<AtomicBool>,
    }

    impl MockBookProcessor {
        fn new(process_called: Arc<AtomicBool>) -> Self {
            Self { process_called }
        }
    }

    impl BookProcessor for MockBookProcessor {
        fn process(
            &self,
            _book_path: &Path,
            _config_root: Option<&Path>,
            _publish: PublishEnabled,
            _word_stats: WordStatsEnabled,
        ) -> ProcessingResult {
            self.process_called.store(true, Ordering::SeqCst);
            Ok(("Mock processing output".to_string(), None, None))
        }
    }

    /// Helper to create a test book
    fn create_test_book(name: &str, path: &str, linked: bool) -> BookInfo {
        BookInfo {
            path: BookPath::new(PathBuf::from(path)),
            display_name: name.to_string(),
            selected: true,
            cover_image_path: None,
            metadata: None,
            google_docs_sync_info: if linked {
                Some(iriebook::utilities::types::GoogleDocsSyncInfo {
                    google_doc_id: "test-doc-id".to_string(),
                    sync_status: "synced".to_string(),
                })
            } else {
                None
            },
            git_changed_files: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_empty_book_list_returns_error() {
        let books = Vec::new();
        let token_provider = Arc::new(MockTokenProvider::new("test-token"));
        let syncer = Arc::new(MockDocumentSyncer::new());
        let process_called = Arc::new(AtomicBool::new(false));
        let processor = Arc::new(MockBookProcessor::new(process_called));

        let result = BatchGoogleDocsSyncProcessor::new(PathBuf::from("/workspace")).sync_books(
            books,
            token_provider,
            syncer,
            processor,
            |_| {},
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No books are linked to Google Docs");
    }

    #[tokio::test]
    async fn test_filters_unlinked_books() {
        let books = vec![
            create_test_book("Linked", "/test/linked.md", true),
            create_test_book("Unlinked", "/test/unlinked.md", false),
        ];

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let token_provider = Arc::new(MockTokenProvider::new("test-token"));
        let syncer = Arc::new(MockDocumentSyncer::new());
        let process_called = Arc::new(AtomicBool::new(false));
        let processor = Arc::new(MockBookProcessor::new(process_called));

        let result = BatchGoogleDocsSyncProcessor::new(PathBuf::from("/workspace")).sync_books(
            books,
            token_provider,
            syncer,
            processor,
            move |event| {
                events_clone.lock().unwrap().push(event);
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let collected = events.lock().unwrap();

        // Should only process the linked book
        let started_events: Vec<_> = collected
            .iter()
            .filter_map(|e| match e {
                GoogleDocsBatchSyncEvent::Started { book_name, .. } => Some(book_name.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(started_events.len(), 1);
        assert_eq!(started_events[0], "Linked");
    }

    #[tokio::test]
    async fn test_emits_events_in_correct_order() {
        let books = vec![
            create_test_book("Book1", "/test/book1.md", true),
            create_test_book("Book2", "/test/book2.md", true),
        ];

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let token_provider = Arc::new(MockTokenProvider::new("test-token"));
        let syncer = Arc::new(MockDocumentSyncer::new());
        let process_called = Arc::new(AtomicBool::new(false));
        let processor = Arc::new(MockBookProcessor::new(process_called));

        let result = BatchGoogleDocsSyncProcessor::new(PathBuf::from("/workspace")).sync_books(
            books,
            token_provider,
            syncer,
            processor,
            move |event| {
                events_clone.lock().unwrap().push(event);
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let collected = events.lock().unwrap();

        // Verify event sequence
        let event_types: Vec<_> = collected
            .iter()
            .map(|e| match e {
                GoogleDocsBatchSyncEvent::Started { .. } => "Started",
                GoogleDocsBatchSyncEvent::Progress { .. } => "Progress",
                GoogleDocsBatchSyncEvent::Completed { .. } => "Completed",
                GoogleDocsBatchSyncEvent::AllDone { .. } => "AllDone",
            })
            .collect();

        // Should have: Started, Progress*, Completed for each book, then AllDone
        assert!(event_types.contains(&"Started"));
        assert!(event_types.contains(&"Completed"));
        assert!(event_types.contains(&"AllDone"));
        assert_eq!(event_types.last().unwrap(), &"AllDone");
    }

    #[tokio::test]
    async fn test_continues_on_individual_failures() {
        let books = vec![
            create_test_book("Good", "/test/good.md", true),
            create_test_book("Bad", "/test/bad.md", true),
            create_test_book("AlsoGood", "/test/alsogood.md", true),
        ];

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let token_provider = Arc::new(MockTokenProvider::new("test-token"));
        let syncer = Arc::new(MockDocumentSyncer::with_failing_paths(vec![
            "/test/bad.md".to_string(),
        ]));
        let process_called = Arc::new(AtomicBool::new(false));
        let processor = Arc::new(MockBookProcessor::new(process_called));

        let result = BatchGoogleDocsSyncProcessor::new(PathBuf::from("/workspace")).sync_books(
            books,
            token_provider,
            syncer,
            processor,
            move |event| {
                events_clone.lock().unwrap().push(event);
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let collected = events.lock().unwrap();

        // Verify all books were processed (including the failed one)
        let started_count = collected
            .iter()
            .filter(|e| matches!(e, GoogleDocsBatchSyncEvent::Started { .. }))
            .count();

        assert_eq!(started_count, 3);

        // Verify we got the AllDone event with correct counts
        let all_done = collected
            .iter()
            .find_map(|e| match e {
                GoogleDocsBatchSyncEvent::AllDone {
                    total_books,
                    success_count,
                    fail_count,
                } => Some((*total_books, *success_count, *fail_count)),
                _ => None,
            });

        assert!(all_done.is_some());
        let (total, success, fail) = all_done.unwrap();
        assert_eq!(total, 3);
        assert_eq!(success, 2);
        assert_eq!(fail, 1);
    }

    #[tokio::test]
    async fn test_correct_success_fail_counts() {
        let books = vec![
            create_test_book("Success1", "/test/success1.md", true),
            create_test_book("Fail1", "/test/fail1.md", true),
            create_test_book("Success2", "/test/success2.md", true),
            create_test_book("Fail2", "/test/fail2.md", true),
        ];

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let token_provider = Arc::new(MockTokenProvider::new("test-token"));
        let syncer = Arc::new(MockDocumentSyncer::with_failing_paths(vec![
            "/test/fail1.md".to_string(),
            "/test/fail2.md".to_string(),
        ]));
        let process_called = Arc::new(AtomicBool::new(false));
        let processor = Arc::new(MockBookProcessor::new(process_called));

        let result = BatchGoogleDocsSyncProcessor::new(PathBuf::from("/workspace")).sync_books(
            books,
            token_provider,
            syncer,
            processor,
            move |event| {
                events_clone.lock().unwrap().push(event);
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let collected = events.lock().unwrap();

        // Find AllDone event
        let all_done = collected
            .iter()
            .find_map(|e| match e {
                GoogleDocsBatchSyncEvent::AllDone {
                    total_books,
                    success_count,
                    fail_count,
                } => Some((*total_books, *success_count, *fail_count)),
                _ => None,
            });

        assert!(all_done.is_some());
        let (total, success, fail) = all_done.unwrap();
        assert_eq!(total, 4);
        assert_eq!(success, 2);
        assert_eq!(fail, 2);
    }
}
