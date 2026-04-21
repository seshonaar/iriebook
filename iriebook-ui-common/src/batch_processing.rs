use crate::{
    processing::{ProcessingEvent, process_single_book},
    ui_state::{BookInfo, PublishEnabled, WordStatsEnabled},
};
use iriebook::utilities::types::PublicationOptions;

/// Batch processor for processing multiple books sequentially
///
/// This module moves the batch processing orchestration logic
/// out of the Tauri UI layer into a framework-agnostic module.
///
/// Following Volatility-Based Design:
/// - The UI layer should NOT contain orchestration loops
/// - Progress callbacks abstract away framework-specific events
/// - This logic can be reused in CLI, web UI, or any other interface
pub struct BatchProcessor;

impl BatchProcessor {
    /// Process multiple books with progress callbacks
    ///
    /// The `progress_callback` is called for Started, Completed, and AllDone events.
    /// This function spawns processing on a background task and runs each book
    /// on a blocking thread (since processing is CPU-intensive).
    ///
    /// # Arguments
    /// * `books` - List of books to process
    /// * `config_root` - Library root containing editable config.json settings
    /// * `publish` - Whether to enable publishing (generate ebook files)
    /// * `word_stats` - Whether to enable word statistics analysis
    /// * `publication_options` - Output format and cover embedding choices
    /// * `progress_callback` - Callback for progress events
    ///
    /// # Returns
    /// Ok(()) if the batch processing task was spawned successfully
    pub async fn process_books<F>(
        books: Vec<BookInfo>,
        config_root: Option<std::path::PathBuf>,
        publish: PublishEnabled,
        word_stats: WordStatsEnabled,
        publication_options: PublicationOptions,
        progress_callback: F,
    ) -> Result<(), String>
    where
        F: Fn(ProcessingEvent) + Send + 'static + Clone,
    {
        // Spawn background task (non-blocking)
        tokio::spawn(async move {
            for (index, book) in books.iter().enumerate() {
                // Emit "Started" event
                progress_callback(ProcessingEvent::Started {
                    book_index: index,
                    book_name: book.display_name.clone(),
                });

                // Process the book on a blocking thread (CPU-intensive work)
                let book_path = book.path.as_path().to_path_buf();
                let config_root_clone = config_root.clone();
                let publish_clone = publish;
                let word_stats_clone = word_stats;

                let result = tokio::task::spawn_blocking(move || {
                    process_single_book(
                        &book_path,
                        config_root_clone.as_deref(),
                        publish_clone,
                        word_stats_clone,
                        publication_options,
                    )
                })
                .await;

                // Emit "Completed" event
                let callback_clone = progress_callback.clone();
                let book_name = book.display_name.clone();
                match result {
                    Ok(Ok((output, word_stats, output_path))) => {
                        callback_clone(ProcessingEvent::Completed {
                            book_index: index,
                            book_name,
                            success: true,
                            message: output,
                            word_stats,
                            output_path: output_path.map(|p| p.to_string_lossy().into_owned()),
                        });
                    }
                    Ok(Err(e)) => {
                        callback_clone(ProcessingEvent::Completed {
                            book_index: index,
                            book_name,
                            success: false,
                            message: e.to_string(),
                            word_stats: None,
                            output_path: None,
                        });
                    }
                    Err(e) => {
                        callback_clone(ProcessingEvent::Completed {
                            book_index: index,
                            book_name,
                            success: false,
                            message: format!("Task error: {}", e),
                            word_stats: None,
                            output_path: None,
                        });
                    }
                }
            }

            // Emit "AllDone" event
            progress_callback(ProcessingEvent::AllDone);
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_state::BookPath;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// Helper to create a test book
    fn create_test_book(name: &str, path: &str) -> BookInfo {
        BookInfo {
            path: BookPath::new(PathBuf::from(path)),
            display_name: name.to_string(),
            selected: true,
            cover_image_path: None,
            metadata: None,
            google_docs_sync_info: None,
            git_changed_files: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_batch_processor_emits_events_in_order() {
        let books = vec![
            create_test_book("Book1", "/nonexistent/book1.md"),
            create_test_book("Book2", "/nonexistent/book2.md"),
        ];

        // Collect events in a thread-safe vector
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let result = BatchProcessor::process_books(
            books,
            None,
            PublishEnabled::new(false),
            WordStatsEnabled::new(false),
            PublicationOptions::default(),
            move |event| {
                let mut ev = events_clone.lock().unwrap();
                match &event {
                    ProcessingEvent::Started { book_index, .. } => {
                        ev.push(format!("Started:{}", book_index));
                    }
                    ProcessingEvent::Completed { book_index, .. } => {
                        ev.push(format!("Completed:{}", book_index));
                    }
                    ProcessingEvent::AllDone => {
                        ev.push("AllDone".to_string());
                    }
                }
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let collected = events.lock().unwrap();

        // Verify event sequence (Started -> Completed for each book, then AllDone)
        assert!(collected.contains(&"Started:0".to_string()));
        assert!(collected.contains(&"Completed:0".to_string()));
        assert!(collected.contains(&"Started:1".to_string()));
        assert!(collected.contains(&"Completed:1".to_string()));
        assert!(collected.contains(&"AllDone".to_string()));

        // Verify AllDone is last
        assert_eq!(collected.last().unwrap(), "AllDone");
    }

    #[tokio::test]
    async fn test_batch_processor_handles_errors() {
        let books = vec![create_test_book("Invalid", "/nonexistent/invalid.md")];

        let success_count = Arc::new(Mutex::new(0));
        let error_count = Arc::new(Mutex::new(0));

        let success_clone = success_count.clone();
        let error_clone = error_count.clone();

        let result = BatchProcessor::process_books(
            books,
            None,
            PublishEnabled::new(false),
            WordStatsEnabled::new(false),
            PublicationOptions::default(),
            move |event| match event {
                ProcessingEvent::Completed { success, .. } => {
                    if success {
                        *success_clone.lock().unwrap() += 1;
                    } else {
                        *error_clone.lock().unwrap() += 1;
                    }
                }
                _ => {}
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Verify error was recorded (nonexistent file should fail)
        assert_eq!(*error_count.lock().unwrap(), 1);
        assert_eq!(*success_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_batch_processor_completes_all_books() {
        let books = vec![
            create_test_book("Book1", "/nonexistent/book1.md"),
            create_test_book("Book2", "/nonexistent/book2.md"),
            create_test_book("Book3", "/nonexistent/book3.md"),
        ];

        let completed_count = Arc::new(Mutex::new(0));
        let completed_clone = completed_count.clone();

        let result = BatchProcessor::process_books(
            books,
            None,
            PublishEnabled::new(false),
            WordStatsEnabled::new(false),
            PublicationOptions::default(),
            move |event| {
                if matches!(event, ProcessingEvent::Completed { .. }) {
                    *completed_clone.lock().unwrap() += 1;
                }
            },
        )
        .await;

        assert!(result.is_ok());

        // Give background task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Verify all 3 books were processed
        assert_eq!(*completed_count.lock().unwrap(), 3);
    }
}
