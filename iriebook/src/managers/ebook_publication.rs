//! Ebook Publication Manager
//!
//! Orchestrates the complete ebook publication pipeline following the
//! Righting Software Method. This Manager component coordinates the workflow
//! but implements ZERO business logic - all processing is delegated to Engines.
//!
//! Pipeline stages:
//! 1. Input & Validation
//! 2. Content Processing (quotes, whitespace, word analysis)
//! 3. Output Generation (files, EPUB, Kindle)
//! 4. Results Display

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::engines::traits::{
    MarkdownTransformEngine, QuoteFixerEngine, ValidatorEngine, WhitespaceTrimmerEngine,
    WordAnalyzerEngine, WordReplacementEngine,
};
use crate::resource_access::traits::{ArchiveAccess, CalibreAccess, GitAccess, PandocAccess};
use crate::resource_access::{config, file};
use crate::utilities::types::{BookMetadata, BookRevisionInfo, ReplacePair};

/// Result of the complete ebook publication process
/// Contains all data produced by the pipeline for the Client to display
#[derive(Debug, Default)]
pub struct PublicationResult {
    /// Number of bytes read from input
    pub bytes_read: usize,
    /// Whether validation passed
    pub validation_passed: bool,
    /// Validation error message (if validation failed)
    pub validation_error: Option<String>,
    /// Number of quotes converted
    pub quotes_converted: usize,
    /// Number of apostrophes converted
    pub apostrophes_converted: usize,
    /// Number of spaces collapsed
    pub spaces_collapsed: usize,
    /// Number of tabs converted
    pub tabs_converted: usize,
    /// Number of blank lines removed
    pub blank_lines_removed: usize,
    /// Number of lines trimmed
    pub lines_trimmed: usize,
    /// Number of word replacements made
    pub replacements_made: usize,
    /// Word analysis results (None if disabled)
    pub word_analysis: Option<WordAnalysisResult>,
    /// Output file path (if written)
    pub output_path: Option<PathBuf>,
    /// Summary file path (if written)
    pub summary_path: Option<PathBuf>,
    /// Command outputs from external tools (pandoc, calibre, archive)
    pub command_outputs: Vec<String>,
}

impl PublicationResult {
    pub fn validation_failure(bytes_read: usize, error: impl Into<String>) -> Self {
        Self {
            bytes_read,
            validation_passed: false,
            validation_error: Some(error.into()),
            ..Default::default()
        }
    }
}

/// Word analysis results for display
#[derive(Debug)]
pub struct WordAnalysisResult {
    /// Total words in document
    pub total_words: usize,
    /// Unique words after filtering
    pub unique_words: usize,
    /// Number of stopwords excluded
    pub excluded_count: usize,
    /// Top words with their counts
    pub top_words: Vec<(String, usize)>,
}

/// Arguments for the publish method
#[derive(Debug)]
pub struct PublishArgs<'a> {
    pub input_path: &'a Path,
    pub output_path: Option<&'a Path>,
    pub enable_word_stats: bool,
    pub enable_publishing: bool,
    pub embed_cover: bool,
    pub replace_pairs: Option<&'a [ReplacePair]>,
}

impl<'a> Default for PublishArgs<'a> {
    fn default() -> Self {
        Self {
            input_path: Path::new(""),
            output_path: None,
            enable_word_stats: false,
            enable_publishing: false,
            embed_cover: true,
            replace_pairs: None,
        }
    }
}

/// Manager for orchestrating ebook publication workflow
pub struct EbookPublicationManager {
    validator: Arc<dyn ValidatorEngine>,
    quote_fixer: Arc<dyn QuoteFixerEngine>,
    whitespace_trimmer: Arc<dyn WhitespaceTrimmerEngine>,
    word_analyzer: Arc<dyn WordAnalyzerEngine>,
    markdown_transformer: Arc<dyn MarkdownTransformEngine>,
    word_replacer: Arc<dyn WordReplacementEngine>,
    pandoc_access: Arc<dyn PandocAccess>,
    calibre_access: Arc<dyn CalibreAccess>,
    archive_access: Arc<dyn ArchiveAccess>,
    git_access: Arc<dyn GitAccess>,
}

impl EbookPublicationManager {
    /// Creates a new EbookPublicationManager with injected Engine and Resource Access dependencies
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        validator: Arc<dyn ValidatorEngine>,
        quote_fixer: Arc<dyn QuoteFixerEngine>,
        whitespace_trimmer: Arc<dyn WhitespaceTrimmerEngine>,
        word_analyzer: Arc<dyn WordAnalyzerEngine>,
        markdown_transformer: Arc<dyn MarkdownTransformEngine>,
        word_replacer: Arc<dyn WordReplacementEngine>,
        pandoc_access: Arc<dyn PandocAccess>,
        calibre_access: Arc<dyn CalibreAccess>,
        archive_access: Arc<dyn ArchiveAccess>,
        git_access: Arc<dyn GitAccess>,
    ) -> Self {
        Self {
            validator,
            quote_fixer,
            whitespace_trimmer,
            word_analyzer,
            markdown_transformer,
            word_replacer,
            pandoc_access,
            calibre_access,
            archive_access,
            git_access,
        }
    }

    /// Execute the complete publication pipeline
    ///
    /// This method orchestrates the entire ebook processing workflow:
    /// - Reads and validates input file
    /// - Processes content (quotes, whitespace, word analysis, replacements)
    /// - Generates output files (markdown, EPUB, Kindle) if publishing enabled
    /// - Returns structured result data for Client to display
    ///
    /// # Arguments
    /// * `args` - PublishArgs containing all parameters
    ///
    /// # Returns
    /// * `Ok(PublicationResult)` - Structured processing results
    /// * `Err` - Processing error
    pub fn publish(&self, args: PublishArgs<'_>) -> Result<PublicationResult> {
        let input_path = args.input_path;
        let output_path = args.output_path;
        let enable_word_stats = args.enable_word_stats;
        let enable_publishing = args.enable_publishing;
        let embed_cover = args.embed_cover;
        let replace_pairs = args.replace_pairs;

        // Vector to collect command outputs
        let mut command_outputs = Vec::new();

        // Stage 1: Input & Validation
        let content = file::read_file(input_path)?;
        let bytes_read = content.len();

        // Validate content - capture errors to display nicely instead of bubbling up
        let (validation_passed, validation_error) = match self.validator.validate(&content) {
            Ok(()) => (true, None),
            Err(err) => {
                return Ok(PublicationResult::validation_failure(
                    bytes_read,
                    err.to_string(),
                ));
            }
        };

        // Stage 2: Content Processing
        let quote_result = self.quote_fixer.convert(&content)?;
        let trimming_result = self.whitespace_trimmer.trim(&quote_result.content)?;

        // Word replacement (last in the chain)
        let replacement_result = match replace_pairs {
            Some(pairs) if !pairs.is_empty() => self
                .word_replacer
                .replace(&trimming_result.content, pairs)?,
            _ => crate::engines::traits::ReplacementResult {
                content: trimming_result.content.clone(),
                replacements_made: 0,
            },
        };

        // Load config
        let current_dir = input_path.parent().unwrap_or(Path::new("."));
        let loaded_config = config::load_config(current_dir).unwrap_or_default();

        // Conditionally analyze words based on enable_word_stats flag
        let word_analysis = match enable_word_stats {
            true => {
                let analysis_result = self
                    .word_analyzer
                    .analyze(&replacement_result.content, &loaded_config.word_analysis)?;
                Some(WordAnalysisResult {
                    total_words: analysis_result.total_words,
                    unique_words: analysis_result.unique_words,
                    excluded_count: analysis_result.excluded_count,
                    top_words: analysis_result
                        .top_words
                        .into_iter()
                        .map(|(word, count)| (word, count.0))
                        .collect(),
                })
            }
            false => None,
        };

        // Stage 3: Output Generation
        let (output_path_result, summary_path_result) = if !enable_publishing {
            // Publishing disabled: don't write files
            (None, None)
        } else {
            // Publishing enabled: write files and generate ebooks
            let final_output_path = match output_path {
                Some(path) => path.to_path_buf(),
                None => file::generate_output_path(input_path)?,
            };

            // Get book folder for copyright.txt lookup
            let book_folder = input_path.parent().unwrap_or(Path::new("."));

            // Load metadata for copyright page generation (pass input_path, not folder - load_metadata calls .parent() internally)
            let metadata = file::load_metadata(input_path)?.unwrap_or_default();

            // Generate copyright page if copyright.txt exists
            let revision_info = self.get_book_revision_info(book_folder);
            let copyright_page = self.markdown_transformer.generate_copyright_page(
                book_folder,
                &metadata,
                revision_info.as_ref(),
            )?;

            // Prepend copyright page to content if it exists
            let has_copyright_page = copyright_page.is_some();
            let content_with_copyright = if let Some(ref copyright) = copyright_page {
                format!("{}\n\n{}", copyright, replacement_result.content)
            } else {
                replacement_result.content.clone()
            };

            // Transform markdown structure
            let formatted_text = self
                .markdown_transformer
                .transform(&content_with_copyright)?;

            // Write the fixed content
            file::write_file(&final_output_path, &formatted_text)?;

            // Write summary (we'll generate it in the Client now, so just mark the path)
            let summary_path = match output_path {
                Some(path) => {
                    let mut path = path.to_path_buf();
                    path.set_extension("summary.txt");
                    path
                }
                None => file::generate_summary_output_path(input_path)?,
            };

            // Track the final artifact path to return (defaults to markdown, updates to epub if generated)
            let mut result_path = final_output_path.clone();

            // Generate ebook artifacts if metadata exists
            match file::get_output_file_name(input_path) {
                Ok(output_epub) => {
                    // Create temp metadata when we need to suppress Pandoc's
                    // own frontmatter generation or metadata-driven cover lookup.
                    let temp_metadata_path = if let Some(pandoc_metadata) =
                        prepare_pandoc_metadata(&metadata, has_copyright_page, embed_cover)
                    {
                        let temp_yaml = serialize_pandoc_metadata(&pandoc_metadata)?;
                        let temp_path = final_output_path
                            .parent()
                            .unwrap_or(Path::new("."))
                            .join("temp_metadata.yaml");
                        std::fs::write(&temp_path, temp_yaml)?;
                        Some(temp_path)
                    } else {
                        None
                    };

                    // Use custom metadata if we generated a copyright page (to suppress pandoc's auto copyright page)
                    let pandoc_output = self.pandoc_access.convert_to_epub(
                        input_path,
                        &final_output_path,
                        &output_epub,
                        temp_metadata_path.as_deref(),
                        embed_cover,
                    )?;
                    command_outputs.push(format!("pandoc: {}", pandoc_output));

                    // Clean up temp metadata file
                    if let Some(temp_meta) = temp_metadata_path {
                        let _ = std::fs::remove_file(temp_meta);
                    }

                    // Pass original input path for metadata lookup
                    let calibre_output = self
                        .calibre_access
                        .convert_to_kindle(input_path, &output_epub)?;
                    command_outputs.push(format!("calibre: {}", calibre_output));

                    let archive_output = self.archive_access.create_book_archive(&output_epub)?;
                    command_outputs.push(format!("archive: {}", archive_output));

                    // If we successfully generated the EPUB, that's our primary output to show
                    result_path = output_epub;
                }
                Err(_) => {
                    // Metadata file doesn't exist, skip ebook generation
                }
            }

            (Some(result_path), Some(summary_path))
        };

        // Return structured result
        Ok(PublicationResult {
            bytes_read,
            validation_passed,
            validation_error,
            quotes_converted: quote_result.quotes_converted,
            apostrophes_converted: quote_result.apostrophes_converted,
            spaces_collapsed: trimming_result.spaces_collapsed,
            tabs_converted: trimming_result.tabs_converted,
            blank_lines_removed: trimming_result.blank_lines_removed,
            lines_trimmed: trimming_result.lines_trimmed,
            replacements_made: replacement_result.replacements_made,
            word_analysis,
            output_path: output_path_result,
            summary_path: summary_path_result,
            command_outputs,
        })
    }

    fn get_book_revision_info(&self, book_folder: &Path) -> Option<BookRevisionInfo> {
        for repo_candidate in book_folder.ancestors() {
            if !self.git_access.is_repository(repo_candidate) {
                continue;
            }

            let Some(commit) = self
                .git_access
                .get_log(repo_candidate, 1)
                .ok()
                .and_then(|mut commits| commits.drain(..).next())
            else {
                continue;
            };

            let short_hash = commit.hash.chars().take(6).collect::<String>();
            if short_hash.is_empty() {
                continue;
            }

            let timestamp = match commit.timestamp.parse::<i64>() {
                Ok(ts) => ts,
                Err(_) => continue,
            };

            let Some(date) = DateTime::<Utc>::from_timestamp(timestamp, 0) else {
                continue;
            };

            return Some(BookRevisionInfo {
                short_hash,
                commit_date: date.date_naive().format("%Y-%m-%d").to_string(),
            });
        }

        None
    }
}

fn prepare_pandoc_metadata(
    metadata: &BookMetadata,
    has_copyright_page: bool,
    embed_cover: bool,
) -> Option<BookMetadata> {
    if !has_copyright_page && embed_cover {
        return None;
    }

    let mut pandoc_metadata = metadata.clone();
    if has_copyright_page {
        pandoc_metadata.rights = None;
    }
    if !embed_cover {
        pandoc_metadata.cover_image = None;
    }

    Some(pandoc_metadata)
}

fn serialize_pandoc_metadata(metadata: &BookMetadata) -> Result<String> {
    let mut yaml_value = serde_yaml::to_value(metadata)
        .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;

    if let Some(mapping) = yaml_value.as_mapping_mut() {
        mapping.retain(|_, value| !value.is_null());
    }

    serde_yaml::to_string(&yaml_value)
        .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::analysis::word_analyzer::WordAnalyzer;
    use crate::engines::text_processing::markdown_transform::MarkdownTransformer;
    use crate::engines::text_processing::quote_fixer::QuoteFixer;
    use crate::engines::text_processing::whitespace_trimmer::WhitespaceTrimmer;
    use crate::engines::text_processing::word_replacement::WordReplacer;
    use crate::engines::validation::validator::Validator;
    use crate::resource_access::archive::ZipArchiver;
    use crate::resource_access::calibre::CalibreConverter;
    use crate::resource_access::git::GitClient;
    use crate::resource_access::pandoc::PandocConverter;
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to create a Manager with default Engine and Resource Access implementations
    fn create_test_manager() -> EbookPublicationManager {
        EbookPublicationManager::new(
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
        )
    }

    #[test]
    fn test_manager_without_publishing() -> Result<()> {
        // Setup test directory and input file
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");

        // Write valid input with straight quotes
        fs::write(&input_path, r#"She said "hello" to me."#)?;

        // Create manager with Engine dependencies
        let manager = create_test_manager();

        // Execute without publishing
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: false,
            enable_publishing: false,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify result
        assert!(result.validation_passed, "Expected validation to pass");
        assert!(
            result.quotes_converted > 0,
            "Expected quotes to be converted"
        );
        assert!(
            result.output_path.is_none(),
            "Expected no output path when publishing disabled"
        );

        Ok(())
    }

    #[test]
    fn test_pandoc_metadata_removes_cover_image_when_embedding_disabled() {
        let metadata = BookMetadata {
            title: "Book".to_string(),
            author: "Author".to_string(),
            rights: Some("Copyright".to_string()),
            cover_image: Some("cover.jpg".to_string()),
            ..Default::default()
        };

        let pandoc_metadata = prepare_pandoc_metadata(&metadata, false, false).unwrap();

        assert_eq!(pandoc_metadata.rights, Some("Copyright".to_string()));
        assert_eq!(pandoc_metadata.cover_image, None);

        let temp_yaml = serialize_pandoc_metadata(&pandoc_metadata).unwrap();
        assert!(!temp_yaml.contains("cover-image"));
    }

    #[test]
    fn test_pandoc_metadata_keeps_original_metadata_when_no_suppression_needed() {
        let metadata = BookMetadata {
            title: "Book".to_string(),
            author: "Author".to_string(),
            cover_image: Some("cover.jpg".to_string()),
            ..Default::default()
        };

        assert!(prepare_pandoc_metadata(&metadata, false, true).is_none());
    }

    #[test]
    fn test_manager_real_run_creates_output() -> Result<()> {
        // Setup test directory and files
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");
        let output_path = temp_dir.path().join("fixed.md");

        // Write valid input with straight quotes
        fs::write(&input_path, r#"She said "hello" to me."#)?;

        // Create manager
        let manager = create_test_manager();

        // Execute with publishing enabled
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: Some(&output_path),
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify output file was created
        assert!(output_path.exists(), "Expected output file to be created");

        // Verify content has curly quotes
        let content = fs::read_to_string(&output_path)?;
        assert!(
            content.contains('\u{201C}'),
            "Expected left curly quote in output"
        );
        assert!(
            content.contains('\u{201D}'),
            "Expected right curly quote in output"
        );

        // Verify result structure
        assert!(result.validation_passed, "Expected validation to pass");
        assert!(result.quotes_converted > 0, "Expected quotes converted");
        assert!(result.output_path.is_some(), "Expected output path");

        Ok(())
    }

    #[test]
    fn test_manager_custom_output_path() -> Result<()> {
        // Setup test directory and files
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("input.md");
        let custom_output = temp_dir.path().join("my-custom-output.md");

        // Write valid input
        fs::write(&input_path, r#"Test "quote" text"#)?;

        // Create manager
        let manager = create_test_manager();

        // Execute with custom output path
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: Some(&custom_output),
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify custom output exists
        assert!(
            custom_output.exists(),
            "Expected custom output path to be used"
        );

        // Verify content
        let content = fs::read_to_string(&custom_output)?;
        assert!(content.contains('\u{201C}'), "Expected curly quotes");

        // Verify result has custom path
        assert_eq!(
            result.output_path.as_ref(),
            Some(&custom_output),
            "Expected custom output path in result"
        );

        Ok(())
    }

    #[test]
    fn test_manager_verbose_mode() -> Result<()> {
        // Setup test directory
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");

        // Write valid input
        fs::write(&input_path, "hello world")?;

        // Create manager
        let manager = create_test_manager();

        // Execute without publishing (verbose is now handled by Client, not Manager)
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: false,
            enable_publishing: false,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify result has expected data (verbose display is Client's job now)
        assert!(result.validation_passed, "Expected validation to pass");
        assert!(result.bytes_read > 0, "Expected bytes to be read");

        Ok(())
    }

    #[test]
    fn test_manager_romanian_text() -> Result<()> {
        // Setup test directory
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("romanian.md");
        let output_path = temp_dir.path().join("romanian-output.md");

        // Write Romanian text with quotes
        fs::write(&input_path, "Ea a spus \"bună ziua\" prietenului său.")?;

        // Create manager
        let manager = create_test_manager();

        // Execute processing with explicit output path
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: Some(&output_path),
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify processing succeeded
        assert!(result.validation_passed, "Expected validation to pass");
        assert!(result.quotes_converted > 0, "Expected quote conversion");

        // Verify output file exists and has curly quotes
        assert!(output_path.exists(), "Expected output file");

        let content = fs::read_to_string(&output_path)?;
        assert!(
            content.contains('\u{201C}'),
            "Expected Romanian text preserved with curly quotes"
        );

        Ok(())
    }

    #[test]
    fn test_manager_no_output_path_auto_generates() -> Result<()> {
        // Setup test directory
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("input.md");

        // Write valid input
        fs::write(&input_path, r#""test""#)?;

        // Create manager
        let manager = create_test_manager();

        // Execute without specifying output path (should auto-generate)
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify result has auto-generated output path
        assert!(result.validation_passed, "Expected validation to pass");
        assert!(
            result.output_path.is_some(),
            "Expected auto-generated output path"
        );

        // Verify the auto-generated file exists
        if let Some(output_path) = result.output_path {
            assert!(
                output_path.exists(),
                "Expected auto-generated file to exist"
            );
        }

        Ok(())
    }

    #[test]
    fn test_manager_word_stats_disabled_by_default() -> Result<()> {
        // Setup test directory and input file
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");

        // Write input with some words
        fs::write(&input_path, "The quick brown fox jumps over the lazy dog")?;

        // Create manager
        let manager = create_test_manager();

        // Execute with enable_word_stats=false (default behavior)
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: false,
            enable_publishing: false,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify word analysis is None when disabled
        assert!(
            result.word_analysis.is_none(),
            "Expected word analysis to be None when disabled"
        );
        assert!(result.validation_passed, "Expected validation to pass");

        Ok(())
    }

    #[test]
    fn test_manager_word_stats_enabled() -> Result<()> {
        // Setup test directory and input file
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");

        // Write input with some words
        fs::write(&input_path, "The quick brown fox jumps over the lazy dog")?;

        // Create manager
        let manager = create_test_manager();

        // Execute with enable_word_stats=true
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: true,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify word analysis is Some when enabled
        assert!(
            result.word_analysis.is_some(),
            "Expected word analysis to be Some when enabled"
        );

        // Verify analysis contains expected data
        let analysis = result.word_analysis.unwrap();
        assert!(analysis.total_words > 0, "Expected total words count");
        assert!(analysis.unique_words > 0, "Expected unique words count");

        Ok(())
    }

    #[test]
    fn test_manager_publishing_disabled() -> Result<()> {
        // Setup test directory and input file
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");

        // Write valid input with straight quotes
        fs::write(&input_path, r#"She said "hello" to me."#)?;

        // Create manager
        let manager = create_test_manager();

        // Execute with enable_publishing=false
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: false,
            enable_publishing: false,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify no output files were created
        assert!(
            result.output_path.is_none(),
            "Expected no output path when publishing disabled"
        );
        assert!(
            result.summary_path.is_none(),
            "Expected no summary path when publishing disabled"
        );

        // Verify processing still happened
        assert!(result.validation_passed, "Expected validation to pass");
        assert!(
            result.quotes_converted > 0,
            "Expected quotes to be converted"
        );

        Ok(())
    }

    #[test]
    fn test_manager_publishing_enabled() -> Result<()> {
        // Setup test directory and files
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");
        let output_path = temp_dir.path().join("fixed.md");

        // Write valid input with straight quotes
        fs::write(&input_path, r#"She said "hello" to me."#)?;

        // Create manager
        let manager = create_test_manager();

        // Execute with enable_publishing=true
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: Some(&output_path),
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify output file was created
        assert!(output_path.exists(), "Expected output file to be created");
        assert!(
            result.output_path.is_some(),
            "Expected output path when publishing enabled"
        );

        // Verify content has curly quotes
        let content = fs::read_to_string(&output_path)?;
        assert!(
            content.contains('\u{201C}'),
            "Expected left curly quote in output"
        );
        assert!(
            content.contains('\u{201D}'),
            "Expected right curly quote in output"
        );

        Ok(())
    }

    #[test]
    fn test_manager_word_stats_with_no_publishing() -> Result<()> {
        // Setup test directory and input file
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");

        // Write input with some words
        fs::write(&input_path, "The quick brown fox jumps over the lazy dog")?;

        // Create manager
        let manager = create_test_manager();

        // Execute with enable_word_stats=true, enable_publishing=false
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: None,
            enable_word_stats: true,
            enable_publishing: false,
            embed_cover: true,
            replace_pairs: None,
        })?;

        // Verify word analysis is Some
        assert!(
            result.word_analysis.is_some(),
            "Expected word analysis when enabled"
        );

        // Verify no files written
        assert!(
            result.output_path.is_none(),
            "Expected no output when publishing disabled"
        );
        assert!(
            result.summary_path.is_none(),
            "Expected no summary when publishing disabled"
        );

        Ok(())
    }

    #[test]
    fn test_replace_pairs_applied_during_processing() -> Result<()> {
        use crate::utilities::types::ReplacePair;

        // Setup test directory and input file
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");
        let output_path = temp_dir.path().join("fixed.md");

        // Write input with words to replace
        fs::write(&input_path, "Hello Rene world. Rene is here.")?;

        // Create manager
        let manager = create_test_manager();

        // Define replace pairs
        let replace_pairs = vec![ReplacePair {
            source: "Rene".to_string(),
            target: "René".to_string(),
        }];

        // Execute with replace pairs and publishing enabled
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: Some(&output_path),
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: Some(&replace_pairs),
        })?;

        // Verify replacements were made
        assert!(result.validation_passed, "Expected validation to pass");
        assert_eq!(result.replacements_made, 2, "Expected 2 replacements");

        // Verify output file has replacements
        let output_content = fs::read_to_string(&output_path)?;
        assert!(output_content.contains("René"), "Expected René in output");
        assert!(
            !output_content.contains("Rene "),
            "Expected no standalone Rene in output"
        );

        Ok(())
    }

    #[test]
    fn test_replace_pairs_case_sensitive() -> Result<()> {
        use crate::utilities::types::ReplacePair;

        // Setup test directory
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("test.md");
        let output_path = temp_dir.path().join("fixed.md");

        // Write input with mixed case
        fs::write(&input_path, "rene Rene RENe")?;

        // Create manager
        let manager = create_test_manager();

        // Define replace pair (only matches exact case)
        let replace_pairs = vec![ReplacePair {
            source: "Rene".to_string(),
            target: "René".to_string(),
        }];

        // Execute
        let result = manager.publish(PublishArgs {
            input_path: &input_path,
            output_path: Some(&output_path),
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            replace_pairs: Some(&replace_pairs),
        })?;

        // Should only replace exact case matches
        assert_eq!(result.replacements_made, 1, "Expected 1 replacement");

        let output_content = fs::read_to_string(&output_path)?;
        assert!(
            output_content.contains("rene"),
            "Expected lowercase rene to remain"
        );
        assert!(
            output_content.contains("RENe"),
            "Expected uppercase RENe to remain"
        );

        Ok(())
    }
}
