//! Analysis caching functionality
//!
//! Provides disk caching for word analysis results with timestamp-based invalidation.
//! Cache is stored in `irie/analysis.json` per book.

use crate::processing::WordAnalysisStats;
use iriebook::resource_access::file::{
    get_file_modified_timestamp, load_analysis_cache, save_analysis_cache, CachedAnalysis,
    CachedAnalysisStats,
};
use serde::Serialize;
use specta::Type;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current cache version - increment when format changes
const CACHE_VERSION: u32 = 1;

/// Response from get_or_compute_analysis
#[derive(Debug, Clone, Serialize, Type)]
pub struct AnalysisResponse {
    /// The word analysis statistics
    pub stats: WordAnalysisStats,
    /// Whether the result was loaded from cache (true) or freshly computed (false)
    pub was_cached: bool,
    /// Unix timestamp when the analysis was performed/cached (as f64 for JS compatibility)
    #[specta(type = f64)]
    pub cache_timestamp: u64,
}

/// Get cached analysis if valid, or compute fresh analysis.
///
/// Cache is considered valid if:
/// - Cache file exists
/// - Cache version matches current version
/// - Book's modification timestamp matches the cached timestamp
///
/// # Arguments
/// * `book_path` - Path to the book's markdown file
/// * `force_refresh` - If true, ignore cache and recompute
///
/// # Returns
/// * `Ok(AnalysisResponse)` with stats and cache info
/// * `Err` if analysis fails
pub fn get_or_compute_analysis(
    book_path: &Path,
    force_refresh: bool,
) -> Result<AnalysisResponse, String> {
    let book_modified = get_file_modified_timestamp(book_path)
        .map_err(|e| format!("Failed to get book modification time: {}", e))?;

    // Try to load from cache if not forcing refresh
    if !force_refresh
        && let Ok(Some(cached)) = load_analysis_cache(book_path)
        && cached.version == CACHE_VERSION
        && cached.book_modified_timestamp == book_modified
    {
        // Cache hit! Convert cached stats to WordAnalysisStats
        let stats = WordAnalysisStats {
            total_words: cached.stats.total_words,
            unique_words: cached.stats.unique_words,
            excluded_count: cached.stats.excluded_count,
            top_words: cached.stats.top_words,
        };

        return Ok(AnalysisResponse {
            stats,
            was_cached: true,
            cache_timestamp: cached.analysis_timestamp,
        });
    }

    // Cache miss or force refresh - compute analysis
    let stats = compute_analysis(book_path)?;

    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Save to cache
    let cache = CachedAnalysis {
        version: CACHE_VERSION,
        book_modified_timestamp: book_modified,
        analysis_timestamp: now,
        stats: CachedAnalysisStats {
            total_words: stats.total_words,
            unique_words: stats.unique_words,
            excluded_count: stats.excluded_count,
            top_words: stats.top_words.clone(),
        },
    };

    // Try to save cache, but don't fail if it doesn't work
    if let Err(e) = save_analysis_cache(book_path, &cache) {
        tracing::warn!("Failed to save analysis cache: {}", e);
    }

    Ok(AnalysisResponse {
        stats,
        was_cached: false,
        cache_timestamp: now,
    })
}

/// Compute word analysis for a book
fn compute_analysis(book_path: &Path) -> Result<WordAnalysisStats, String> {
    use iriebook::engines::analysis::word_analyzer::WordAnalyzer;
    use iriebook::engines::traits::WordAnalyzerEngine;
    use iriebook::resource_access::config::WordAnalysisConfig;
    use iriebook::resource_access::file::read_file;

    // Read the book content
    let content = read_file(book_path)
        .map_err(|e| format!("Failed to read book: {}", e))?;

    // Run word analysis with default config
    let analyzer = WordAnalyzer;
    let config = WordAnalysisConfig::default();
    let analysis = analyzer
        .analyze(&content, &config)
        .map_err(|e| format!("Word analysis failed: {}", e))?;

    // Convert to WordAnalysisStats (limit to 100 top words)
    // WordCount is a newtype wrapper around usize, so we extract the inner value
    const TOP_WORDS_COUNT: usize = 100;
    let top_words: Vec<(String, usize)> = analysis
        .top_words
        .iter()
        .take(TOP_WORDS_COUNT)
        .map(|(word, count)| (word.clone(), count.0))
        .collect();

    Ok(WordAnalysisStats {
        total_words: analysis.total_words,
        unique_words: analysis.unique_words,
        excluded_count: analysis.excluded_count,
        top_words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_compute_analysis_basic() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");

        // Create a simple book
        fs::write(&book_path, "The quick brown fox jumps over the lazy dog.").unwrap();

        let result = compute_analysis(&book_path);
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert!(stats.total_words > 0);
        assert!(stats.unique_words > 0);
    }

    #[test]
    fn test_get_or_compute_analysis_creates_cache() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");

        // Create a simple book
        fs::write(&book_path, "Hello world hello again world.").unwrap();

        // First call - should compute fresh
        let result1 = get_or_compute_analysis(&book_path, false);
        assert!(result1.is_ok());
        let response1 = result1.unwrap();
        assert!(!response1.was_cached);

        // Check cache file was created
        let cache_path = temp_dir.path().join("irie").join("analysis.json");
        assert!(cache_path.exists());

        // Second call - should use cache
        let result2 = get_or_compute_analysis(&book_path, false);
        assert!(result2.is_ok());
        let response2 = result2.unwrap();
        assert!(response2.was_cached);

        // Stats should be the same
        assert_eq!(response1.stats.total_words, response2.stats.total_words);
    }

    #[test]
    fn test_force_refresh_ignores_cache() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");

        fs::write(&book_path, "Test content for analysis.").unwrap();

        // First call - compute and cache
        let result1 = get_or_compute_analysis(&book_path, false);
        assert!(result1.is_ok());
        assert!(!result1.unwrap().was_cached);

        // Force refresh - should recompute even though cache exists
        let result2 = get_or_compute_analysis(&book_path, true);
        assert!(result2.is_ok());
        assert!(!result2.unwrap().was_cached);
    }
}
