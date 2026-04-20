//! Word frequency analyzer
//!
//! Extracts words from text and counts frequency with stopword exclusion

use crate::engines::traits::WordAnalyzerEngine;
use crate::resource_access::config::WordAnalysisConfig;
use crate::utilities::error::IrieBookError;
use crate::utilities::types::WordCount;
use anyhow::Result;

const TOP_WORDS_COUNT: usize = 100;
const MIN_WORD_LENGTH: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    /// Total number of words analyzed (including excluded)
    pub total_words: usize,
    /// Number of unique words (after exclusions)
    pub unique_words: usize,
    /// Top frequent words with counts
    pub top_words: Vec<(String, WordCount)>,
    /// Number of words excluded by stopword list
    pub excluded_count: usize,
}

/// Concrete implementation of the WordAnalyzerEngine trait
pub struct WordAnalyzer;

impl WordAnalyzerEngine for WordAnalyzer {
    fn analyze(
        &self,
        content: &str,
        config: &WordAnalysisConfig,
    ) -> Result<AnalysisResult, IrieBookError> {
        Ok(analyze_words_impl(content, config))
    }
}

/// Analyze words in content (free function for backward compatibility)
pub fn analyze_words(content: &str, config: &WordAnalysisConfig) -> Result<AnalysisResult> {
    Ok(WordAnalyzer.analyze(content, config)?)
}

/// Implementation of the word analysis logic (infallible)
fn analyze_words_impl(content: &str, config: &WordAnalysisConfig) -> AnalysisResult {
    use std::collections::HashMap;

    let mut word_counts: HashMap<String, usize> = HashMap::new();
    let mut total_words = 0;
    let mut excluded_count = 0;

    for word in extract_words(content) {
        total_words += 1;
        let normalized = word.to_lowercase();

        if config.excluded_words.contains(&normalized) {
            excluded_count += 1;
            continue;
        }

        *word_counts.entry(normalized).or_insert(0) += 1;
    }

    let mut word_vec: Vec<(String, usize)> = word_counts.into_iter().collect();
    word_vec.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let unique_words = word_vec.len();

    let top_words: Vec<(String, WordCount)> = word_vec
        .into_iter()
        .filter(|w| w.0.chars().count() > MIN_WORD_LENGTH)
        .take(TOP_WORDS_COUNT)
        .map(|(word, count)| (word, WordCount(count)))
        .collect();

    AnalysisResult {
        total_words,
        unique_words,
        top_words,
        excluded_count,
    }
}

fn extract_words(content: &str) -> impl Iterator<Item = String> + '_ {
    content
        .split(|c: char| {
            // Split on whitespace
            c.is_whitespace()
                // Split on most punctuation (but NOT apostrophe)
                || (c.is_ascii_punctuation() && c != '\'')
                // Split on curly quotes
                || c == '\u{201C}' || c == '\u{201D}'
        })
        .filter_map(|word| {
            let cleaned = word
                .trim_matches(|c: char| {
                    // Strip markdown formatting
                    c == '*' || c == '_' || c == '`'
                        // Strip remaining punctuation at word boundaries
                        || c.is_ascii_punctuation() && c != '\''
                })
                .to_string();

            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn counts_simple_words() -> Result<()> {
        let input = "hello world hello rust world world";
        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(input, &config)?;

        // world=3, hello=2, rust=1
        assert_eq!(result.total_words, 6);
        assert_eq!(result.unique_words, 3);
        assert_eq!(result.top_words[0].0, "world");
        assert_eq!(result.top_words[0].1.0, 3);
        assert_eq!(result.top_words[1].0, "hello");
        assert_eq!(result.top_words[1].1.0, 2);
        assert_eq!(result.top_words[2].0, "rust");
        assert_eq!(result.top_words[2].1.0, 1);

        Ok(())
    }

    #[test]
    fn case_insensitive_counting() -> Result<()> {
        let input = "Dară dară DARĂ Dară";
        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(input, &config)?;

        assert_eq!(result.unique_words, 1);
        assert_eq!(result.top_words[0].1.0, 4); // All counted as "dară"

        Ok(())
    }

    #[test]
    fn excludes_configured_words() -> Result<()> {
        let input = "într hello sincer world sincer test";
        let mut excluded = HashSet::new();
        excluded.insert("sincer".to_string());
        let config = WordAnalysisConfig {
            excluded_words: excluded,
        };

        let result = analyze_words(input, &config)?;

        assert_eq!(result.total_words, 6);
        assert_eq!(result.excluded_count, 2);
        assert_eq!(result.unique_words, 4); //într, hello, world, test

        Ok(())
    }

    #[test]
    fn handles_romanian_diacritics() -> Result<()> {
        let input = "bună ziua și salut bună";
        let config = WordAnalysisConfig::default();
        let result = analyze_words(input, &config)?;

        // "bună" should appear twice
        let buna_count = result
            .top_words
            .iter()
            .find(|(word, _)| word == "bună")
            .map(|(_, count)| count.0);
        assert_eq!(buna_count, Some(2));

        Ok(())
    }

    #[test]
    fn strips_markdown_formatting() -> Result<()> {
        let input = "This is *italic* and **bold** and _underline_";
        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(input, &config)?;

        // Words should be clean
        assert!(result.top_words.iter().any(|(word, _)| word == "italic"));
        assert!(result.top_words.iter().any(|(word, _)| word == "bold"));
        assert!(result.top_words.iter().any(|(word, _)| word == "underline"));

        // No asterisks
        assert!(!result.top_words.iter().any(|(word, _)| word.contains('*')));
        assert!(!result.top_words.iter().any(|(word, _)| word.contains('_')));

        Ok(())
    }

    #[test]
    fn handles_curly_quotes() -> Result<()> {
        let input = "She said \u{201C}hello\u{201D} and \u{201C}world\u{201D}";
        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(input, &config)?;

        assert!(result.top_words.iter().any(|(word, _)| word == "hello"));
        assert!(result.top_words.iter().any(|(word, _)| word == "world"));
        assert!(
            !result
                .top_words
                .iter()
                .any(|(word, _)| word.contains('\u{201C}'))
        );
        assert!(
            !result
                .top_words
                .iter()
                .any(|(word, _)| word.contains('\u{201D}'))
        );

        Ok(())
    }

    #[test]
    fn strips_punctuation() -> Result<()> {
        let input = "Hello, world! How are you? Fine.";
        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(input, &config)?;

        assert!(result.top_words.iter().any(|(word, _)| word == "hello"));
        assert!(result.top_words.iter().any(|(word, _)| word == "world"));
        assert!(!result.top_words.iter().any(|(word, _)| word.contains(',')));
        assert!(!result.top_words.iter().any(|(word, _)| word.contains('.')));
        assert!(!result.top_words.iter().any(|(word, _)| word.contains('!')));

        Ok(())
    }

    #[test]
    fn returns_top_100_words() -> Result<()> {
        // Create TOP_WORDS_COUNT unique words with different frequencies
        let mut words = Vec::new();
        for i in 0..TOP_WORDS_COUNT {
            for _ in 0..(TOP_WORDS_COUNT - i) {
                words.push(format!("word{}", i));
            }
        }
        let input = words.join(" ");

        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(&input, &config)?;

        assert_eq!(result.top_words.len(), TOP_WORDS_COUNT);
        assert!(result.top_words[0].1.0 >= result.top_words[1].1.0);
        assert!(result.top_words[1].1.0 >= result.top_words[2].1.0);

        Ok(())
    }

    #[test]
    fn handles_empty_content() -> Result<()> {
        let input = "";
        let config = WordAnalysisConfig::default();
        let result = analyze_words(input, &config)?;

        assert_eq!(result.total_words, 0);
        assert_eq!(result.unique_words, 0);
        assert_eq!(result.top_words.len(), 0);

        Ok(())
    }

    #[test]
    fn handles_only_stopwords() -> Result<()> {
        let input = "într într într";
        let config = WordAnalysisConfig::default();
        let result = analyze_words(input, &config)?;

        assert_eq!(result.total_words, 3);
        assert_eq!(result.excluded_count, 3);
        assert_eq!(result.top_words.len(), 0);

        Ok(())
    }

    #[test]
    fn preserves_apostrophes_in_contractions() -> Result<()> {
        let input = "It's working don't worry";
        let config = WordAnalysisConfig {
            excluded_words: HashSet::new(),
        };
        let result = analyze_words(input, &config)?;

        // Apostrophes should be preserved in contractions
        assert!(result.top_words.iter().any(|(word, _)| word == "it's"));
        assert!(result.top_words.iter().any(|(word, _)| word == "don't"));

        Ok(())
    }
}
