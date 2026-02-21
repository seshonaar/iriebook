use crate::engines::analysis::word_analyzer::AnalysisResult;
use crate::engines::text_processing::quote_fixer::ConversionResult;
use crate::engines::text_processing::whitespace_trimmer::TrimmingResult;
use crate::resource_access::config::WordAnalysisConfig;
use crate::utilities::error::IrieBookError;
use crate::utilities::types::{DiffResult, ReplacePair};

/// Trait for quote validation engines
///
/// Implementations of this trait validate text content for proper quote usage,
/// checking for dialogue quotes (single quotation marks) and balanced double quotes.
pub trait ValidatorEngine: Send + Sync {
    /// Validates that the content contains no dialogue quotes and has balanced double quotes
    ///
    /// # Arguments
    /// * `content` - The text content to validate
    ///
    /// # Returns
    /// * `Ok(())` if validation passes
    /// * `Err(IrieBookError)` if single quotes or unbalanced quotes are found
    fn validate(&self, content: &str) -> Result<(), IrieBookError>;
}

/// Trait for quote conversion engines
///
/// Implementations of this trait convert straight quotes to curly quotes
/// and straight apostrophes to curly apostrophes.
pub trait QuoteFixerEngine: Send + Sync {
    /// Converts straight quotes to curly quotes and apostrophes to curly apostrophes
    ///
    /// # Arguments
    /// * `content` - The text content to process
    ///
    /// # Returns
    /// * `Ok(ConversionResult)` containing the converted content and conversion statistics
    /// * `Err(IrieBookError)` if conversion fails
    fn convert(&self, content: &str) -> Result<ConversionResult, IrieBookError>;
}

/// Trait for whitespace cleaning engines
///
/// Implementations of this trait clean up excessive whitespace in text content,
/// including collapsing multiple spaces, converting tabs, and limiting blank lines.
pub trait WhitespaceTrimmerEngine: Send + Sync {
    /// Trims and normalizes whitespace in the content
    ///
    /// # Arguments
    /// * `content` - The text content to process
    ///
    /// # Returns
    /// * `Ok(TrimmingResult)` containing the cleaned content and trimming statistics
    /// * `Err(IrieBookError)` if trimming fails
    fn trim(&self, content: &str) -> Result<TrimmingResult, IrieBookError>;
}

/// Trait for word frequency analysis engines
///
/// Implementations of this trait analyze text content to extract word frequencies,
/// with support for stopword exclusion and language-specific processing.
pub trait WordAnalyzerEngine: Send + Sync {
    /// Analyzes word frequency in the content
    ///
    /// # Arguments
    /// * `content` - The text content to analyze
    /// * `config` - Configuration for analysis (stopwords, etc.)
    ///
    /// # Returns
    /// * `Ok(AnalysisResult)` containing word counts and statistics
    /// * `Err(IrieBookError)` if analysis fails
    fn analyze(
        &self,
        content: &str,
        config: &WordAnalysisConfig,
    ) -> Result<AnalysisResult, IrieBookError>;
}

/// Trait for markdown transformation engines
///
/// Implementations of this trait transform markdown structure for ebook formatting,
/// including chapter headings, scene breaks, and paragraph spacing.
pub trait MarkdownTransformEngine: Send + Sync {
    /// Transforms markdown structure for ebook formatting
    ///
    /// # Arguments
    /// * `content` - The markdown content to transform
    ///
    /// # Returns
    /// * `Ok(String)` containing the transformed markdown
    /// * `Err(IrieBookError)` if transformation fails
    fn transform(&self, content: &str) -> Result<String, IrieBookError>;
}

/// Result of word replacement
#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementResult {
    pub content: String,
    pub replacements_made: usize,
}

/// Trait for diff computation engines
///
/// Implementations of this trait compute word-level diffs between two text sources.
/// Whitespace is included by default as it's important for manuscript editing.
pub trait DifferEngine: Send + Sync {
    /// Computes word-level diff between two text sources
    ///
    /// # Arguments
    /// * `left_content` - Left side content
    /// * `right_content` - Right side content
    ///
    /// # Returns
    /// * `Ok(DiffResult)` containing segments and statistics
    /// * `Err(IrieBookError)` if diff computation fails
    fn diff(&self, left_content: &str, right_content: &str) -> Result<DiffResult, IrieBookError>;
}

/// Trait for word replacement engines
///
/// Implementations of this trait perform case-sensitive whole-word replacements
/// using replace pairs defined in book metadata.
pub trait WordReplacementEngine: Send + Sync {
    /// Replaces words in content according to the provided replace pairs
    ///
    /// # Arguments
    /// * `content` - The text content to process
    /// * `replace_pairs` - List of source->target word pairs
    ///
    /// # Returns
    /// * `Ok(ReplacementResult)` containing the modified content and count of replacements
    /// * `Err(IrieBookError)` if replacement fails
    fn replace(
        &self,
        content: &str,
        replace_pairs: &[ReplacePair],
    ) -> Result<ReplacementResult, IrieBookError>;
}
