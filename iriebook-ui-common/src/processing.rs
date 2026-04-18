use crate::load_metadata;
use crate::ui_state::{PublishEnabled, WordStatsEnabled};
use anyhow::Result;
use iriebook::utilities::types::ReplacePair;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri_specta::Event;

/// Word analysis statistics for UI display
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WordAnalysisStats {
    /// Total words in document
    #[specta(type = u32)]
    pub total_words: usize,
    /// Unique words after filtering
    #[specta(type = u32)]
    pub unique_words: usize,
    /// Number of stopwords excluded
    #[specta(type = u32)]
    pub excluded_count: usize,
    /// Top words with their counts (up to 100)
    #[specta(type = Vec<(String, u32)>)]
    pub top_words: Vec<(String, usize)>,
}

/// Result type for book processing containing output message, optional word statistics, and optional output path
pub type ProcessingResult = Result<(String, Option<WordAnalysisStats>, Option<PathBuf>)>;

/// Trait for book processing (for testability)
///
/// Implementations handle the actual book processing pipeline.
/// This abstraction allows for easy mocking in tests.
pub trait BookProcessor: Send + Sync {
    /// Process a book file
    ///
    /// # Arguments
    /// * `book_path` - Path to the book's markdown file
    /// * `config_root` - Library root containing config.json overrides
    /// * `publish` - Whether to generate ebook files (EPUB/Kindle)
    /// * `word_stats` - Whether to generate word frequency statistics
    ///
    /// # Returns
    /// * `Ok((output, top_words, output_path))` on success
    /// * `Err` if processing fails
    fn process(
        &self,
        book_path: &Path,
        config_root: Option<&Path>,
        publish: PublishEnabled,
        word_stats: WordStatsEnabled,
    ) -> ProcessingResult;
}

/// Default book processor using the real processing pipeline
pub struct DefaultBookProcessor;

impl BookProcessor for DefaultBookProcessor {
    fn process(
        &self,
        book_path: &Path,
        config_root: Option<&Path>,
        publish: PublishEnabled,
        word_stats: WordStatsEnabled,
    ) -> ProcessingResult {
        process_single_book(book_path, config_root, publish, word_stats, true)
    }
}

/// Messages sent from background processing to UI
#[derive(Debug, Clone)]
pub enum ProcessingMessage {
    /// Processing started for a book
    Started { book_name: String },
    /// Progress update for a book
    Progress { book_name: String, message: String },
    /// Book processing completed
    Completed {
        book_name: String,
        success: bool,
        message: String,
        word_stats: Option<WordAnalysisStats>,
        output_path: Option<PathBuf>,
    },
    /// All books processed
    AllDone,
}

/// Events emitted during book processing (serializable for Tauri events)
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type")]
pub enum ProcessingEvent {
    /// Processing started for a book
    #[serde(rename = "started")]
    Started {
        #[specta(type = u32)]
        book_index: usize,
        book_name: String,
    },
    /// Book processing completed
    #[serde(rename = "completed")]
    Completed {
        #[specta(type = u32)]
        book_index: usize,
        /// Book name for identification in analysis results
        book_name: String,
        success: bool,
        message: String,
        /// Full word analysis statistics (when word stats are enabled)
        #[serde(skip_serializing_if = "Option::is_none")]
        word_stats: Option<WordAnalysisStats>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_path: Option<String>,
    },
    /// All books processed
    #[serde(rename = "all_done")]
    AllDone,
}

/// Event wrapper for processing updates (for tauri-specta type-safe events)
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct ProcessingUpdateEvent(pub ProcessingEvent);

/// Event wrapper for git operation progress messages (for tauri-specta type-safe events)
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct GitOperationProgressEvent(pub String);

/// Event wrapper for Google Docs sync progress messages (for tauri-specta type-safe events)
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct GoogleDocsProgressEvent(pub String);

/// Events emitted during batch Google Docs sync
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type")]
pub enum GoogleDocsBatchSyncEvent {
    /// Sync started for a book
    #[serde(rename = "started")]
    Started {
        #[specta(type = u32)]
        book_index: usize,
        book_name: String,
    },
    /// Progress update for a book
    #[serde(rename = "progress")]
    Progress {
        #[specta(type = u32)]
        book_index: usize,
        book_name: String,
        message: String,
    },
    /// Book sync completed
    #[serde(rename = "completed")]
    Completed {
        #[specta(type = u32)]
        book_index: usize,
        book_name: String,
        success: bool,
        message: String,
    },
    /// All books synced
    #[serde(rename = "all_done")]
    AllDone {
        #[specta(type = u32)]
        total_books: usize,
        #[specta(type = u32)]
        success_count: usize,
        #[specta(type = u32)]
        fail_count: usize,
    },
}

/// Event wrapper for batch Google Docs sync updates (for tauri-specta type-safe events)
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct GoogleDocsBatchSyncUpdateEvent(pub GoogleDocsBatchSyncEvent);

/// Event to signal that the book list has changed and UI should refresh
/// Emitted after git operations that could modify the book list (clone, sync, save)
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct BookListChangedEvent {}

/// Event emitted when a cover should be reloaded
/// The UI should query cover status after receiving this event
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct CoverReloadEvent {
    pub book_path: String,
}

/// Status of cover loading operation
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoverStatus {
    /// Not started loading yet
    NotStarted,
    /// Currently loading thumbnail
    Loading,
    /// Thumbnail ready with data URL
    Ready {
        data_url: String,
        #[specta(type = u32)]
        width: u32,
        #[specta(type = u32)]
        height: u32,
    },
    /// Error during loading
    Error { message: String },
}

/// Framework-agnostic state machine for processing multiple books sequentially
///
/// This struct manages the processing queue and determines which book to process next.
/// It contains no framework-specific code and can be used by any UI implementation.
#[derive(Debug, Clone)]
pub struct BookProcessingQueue {
    books: Vec<crate::ui_state::BookInfo>,
    current_index: usize,
    publish: PublishEnabled,
    word_stats: WordStatsEnabled,
}

impl BookProcessingQueue {
    /// Create a new processing queue
    pub fn new(
        books: Vec<crate::ui_state::BookInfo>,
        publish: PublishEnabled,
        word_stats: WordStatsEnabled,
    ) -> Self {
        Self {
            books,
            current_index: 0,
            publish,
            word_stats,
        }
    }

    /// Get the current book to process (if any)
    pub fn current_book(&self) -> Option<&crate::ui_state::BookInfo> {
        self.books.get(self.current_index)
    }

    /// Move to the next book in the queue
    pub fn advance(&mut self) {
        self.current_index += 1;
    }

    /// Check if there are more books to process
    pub fn has_more(&self) -> bool {
        self.current_index < self.books.len()
    }

    /// Get the publish setting
    pub fn publish_enabled(&self) -> PublishEnabled {
        self.publish
    }

    /// Get the word stats setting
    pub fn word_stats_enabled(&self) -> WordStatsEnabled {
        self.word_stats
    }

    /// Get total number of books in queue
    pub fn total_books(&self) -> usize {
        self.books.len()
    }

    /// Get current processing index
    pub fn current_index(&self) -> usize {
        self.current_index
    }
}

/// Process a single book using the iriebook library
///
/// This function creates an EbookPublicationManager and processes the book
/// according to the provided options.
pub fn process_single_book(
    book_path: &Path,
    config_root: Option<&Path>,
    publish: PublishEnabled,
    word_stats: WordStatsEnabled,
    embed_cover: bool,
) -> ProcessingResult {
    use iriebook::{
        engines::{
            analysis::word_analyzer::WordAnalyzer,
            text_processing::{
                markdown_transform::MarkdownTransformer, quote_fixer::QuoteFixer,
                whitespace_trimmer::WhitespaceTrimmer, word_replacement::WordReplacer,
            },
            validation::validator::Validator,
        },
        managers::ebook_publication::{EbookPublicationManager, PublishArgs},
        resource_access::{
            archive::ZipArchiver, calibre::CalibreConverter, git::GitClient,
            pandoc::PandocConverter,
        },
    };

    // Create the manager with all dependencies
    let manager = EbookPublicationManager::new(
        Arc::new(Validator),
        Arc::new(QuoteFixer),
        Arc::new(WhitespaceTrimmer),
        Arc::new(WordAnalyzer),
        Arc::new(MarkdownTransformer),
        Arc::new(WordReplacer::new()),
        Arc::new(PandocConverter),
        Arc::new(CalibreConverter),
        Arc::new(ZipArchiver),
        Arc::new(GitClient),
    );

    // Load metadata to get replace pairs
    let replace_pairs: Option<Vec<ReplacePair>> = load_metadata(book_path)
        .ok()
        .flatten()
        .and_then(|m| m.replace_pairs);

    // Process the book
    let result = manager.publish(PublishArgs {
        input_path: book_path,
        output_path: None,
        enable_word_stats: word_stats.is_enabled(),
        enable_publishing: publish.is_enabled(),
        embed_cover,
        config_root,
        replace_pairs: replace_pairs.as_deref(),
    })?;

    // Format the result message
    let mut output = String::new();
    let mut word_stats = None;

    output.push_str(&format!(
        "✓ Processed: {}\n",
        book_path.file_name().unwrap_or_default().to_string_lossy()
    ));

    if !result.validation_passed
        && let Some(error) = &result.validation_error
    {
        output.push_str(&format!("  ✗ Validation failed: {}\n", error));
        return Ok((output, None, None));
    }

    output.push_str(&format!(
        "  - Quotes converted: {}\n",
        result.quotes_converted
    ));
    output.push_str(&format!(
        "  - Apostrophes converted: {}\n",
        result.apostrophes_converted
    ));

    let total_whitespace = result.spaces_collapsed
        + result.tabs_converted
        + result.blank_lines_removed
        + result.lines_trimmed;

    output.push_str(&format!(
        "  - Whitespace cleaned: {} locations\n",
        total_whitespace
    ));

    if let Some(word_analysis) = &result.word_analysis {
        output.push_str(&format!("  - Total words: {}\n", word_analysis.total_words));
        output.push_str(&format!(
            "  - Unique words: {}\n",
            word_analysis.unique_words
        ));

        // Extract top words for UI display (limit to 100)
        const TOP_WORDS_COUNT_IN_UI: usize = 100;
        let top_words: Vec<(String, usize)> = word_analysis
            .top_words
            .iter()
            .take(TOP_WORDS_COUNT_IN_UI)
            .map(|(w, c)| (w.clone(), *c))
            .collect();

        word_stats = Some(WordAnalysisStats {
            total_words: word_analysis.total_words,
            unique_words: word_analysis.unique_words,
            excluded_count: word_analysis.excluded_count,
            top_words,
        });
    }

    let output_path_clone = result.output_path.clone();

    if let Some(output_path) = &result.output_path {
        output.push_str(&format!("  - Output: {}\n", output_path.display()));
    }

    if let Some(pdf_output_path) = &result.pdf_output_path {
        output.push_str(&format!("  - PDF: {}\n", pdf_output_path.display()));
    }

    if let Some(summary_path) = &result.summary_path {
        output.push_str(&format!("  - Summary: {}\n", summary_path.display()));
    }

    // Add command outputs if any
    if !result.command_outputs.is_empty() {
        output.push_str("\n📋 Command Outputs:\n");
        for cmd_output in &result.command_outputs {
            output.push_str(&format!("  - {}\n", cmd_output));
        }
    }

    Ok((output, word_stats, output_path_clone))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_processing_message_debug() {
        let msg = ProcessingMessage::Started {
            book_name: "test.md".to_string(),
        };
        assert!(format!("{:?}", msg).contains("Started"));
    }

    #[test]
    fn test_processing_message_clone() {
        let msg = ProcessingMessage::Progress {
            book_name: "test.md".to_string(),
            message: "Processing...".to_string(),
        };
        let cloned = msg.clone();
        match cloned {
            ProcessingMessage::Progress { book_name, .. } => {
                assert_eq!(book_name, "test.md");
            }
            _ => panic!("Expected Progress message"),
        }
    }

    #[test]
    fn test_process_single_book_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/file.md");
        let result = process_single_book(
            &path,
            None,
            PublishEnabled::new(false),
            WordStatsEnabled::new(false),
            true,
        );
        assert!(result.is_err());
    }
}
