//! Error types for the quote fixer
//!
//! Uses thiserror for ergonomic error definitions and anyhow for context

use crate::utilities::types::{Column, LineNumber, QuoteCount};
use std::fmt;
use thiserror::Error;

/// Location of a quote with surrounding context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteOccurrence {
    /// Line number where the quote appears
    pub line_number: LineNumber,
    /// Column number where the quote appears
    pub column: Column,
    /// Surrounding text (up to 20 chars before and after)
    pub context: String,
    /// The problematic character itself
    pub char_found: char,
}

impl fmt::Display for QuoteOccurrence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Location (line {}, column {}):",
            self.line_number, self.column
        )?;
        writeln!(f, "  {}", self.context)?;
        write!(f, "  {}^", " ".repeat(self.column.0.saturating_sub(1)))
    }
}

/// Errors that can occur during quote fixing
#[derive(Error, Debug)]
pub enum IrieBookError {
    /// Single quotation marks (for dialogue) were found in the file
    #[error("Found {count} single quotation mark(s) used for dialogue\n\n{}\n\nThese appear to be dialogue quotes, not apostrophes.\nConsider using double quotes instead for dialogue.", format_occurrences(.occurrences))]
    SingleQuotesFound {
        count: usize,
        occurrences: Vec<QuoteOccurrence>,
    },

    /// Quotes are unbalanced (odd number)
    #[error(
        "Unbalanced quotes: found {count} straight double quotes (must be even)\n\n{last_occurrence}"
    )]
    UnbalancedQuotes {
        count: QuoteCount,
        last_occurrence: QuoteOccurrence,
    },

    /// File read error
    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// File write error
    #[error("Failed to write file '{path}': {source}")]
    FileWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Invalid UTF-8
    #[error("File contains invalid UTF-8 at byte position {position}")]
    InvalidUtf8 { position: usize },

    /// Cannot determine output path
    #[error("Cannot determine output filename for input: {input}")]
    OutputPathError { input: String },

    /// Git operation failed
    #[error("Git operation failed: {0}")]
    Git(String),

    /// GitHub authentication failed
    #[error("GitHub authentication failed: {0}")]
    GitHubAuth(String),

    /// Repository not initialized
    #[error("Repository not initialized")]
    RepositoryNotInitialized,

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Credential storage error
    #[error("Credential storage error: {0}")]
    CredentialStorage(String),

    /// Google authentication failed
    #[error("Google authentication failed: {0}")]
    GoogleAuth(String),

    /// Google Docs API error
    #[error("Google Docs API error: {0}")]
    GoogleDocsApi(String),

    /// Google Docs document not found
    #[error("Google Docs document not found: {0}")]
    GoogleDocNotFound(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Git revision not found
    #[error("Git revision not found: {0}")]
    GitRevisionNotFound(String),

    /// File not found in git revision
    #[error("File '{file}' not found in revision '{revision}'")]
    FileNotFoundInRevision { file: String, revision: String },

    /// Invalid UTF-8 in git blob
    #[error("File contains invalid UTF-8 in git revision")]
    InvalidUtf8InGitBlob,

    /// Diff source not accessible
    #[error("Diff source not accessible: {0}")]
    DiffSourceNotAccessible(String),

    /// Diff computation error
    #[error("Diff computation error: {0}")]
    Diff(String),
}

/// Format multiple quote occurrences for display
fn format_occurrences(occurrences: &[QuoteOccurrence]) -> String {
    occurrences
        .iter()
        .enumerate()
        .map(|(i, occ)| format!("{}. {}", i + 1, occ))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_occurrence_displays_correctly() {
        let occ = QuoteOccurrence {
            line_number: LineNumber(42),
            column: Column(15),
            context: "she said 'hello' to me".to_string(),
            char_found: '\'',
        };

        let display = format!("{}", occ);
        assert!(display.contains("line 42"));
        assert!(display.contains("column 15"));
        assert!(display.contains("she said 'hello' to me"));
        assert!(display.contains("^"));
    }

    #[test]
    fn single_quotes_error_shows_all_occurrences() {
        let occurrences = vec![
            QuoteOccurrence {
                line_number: LineNumber(10),
                column: Column(5),
                context: "she said 'hello'".to_string(),
                char_found: '\'',
            },
            QuoteOccurrence {
                line_number: LineNumber(20),
                column: Column(8),
                context: "'goodbye' he said".to_string(),
                char_found: '\'',
            },
        ];

        let err = IrieBookError::SingleQuotesFound {
            count: 2,
            occurrences,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("Found 2 single quotation mark(s)"));
        assert!(msg.contains("used for dialogue"));
        assert!(msg.contains("not apostrophes"));
        assert!(msg.contains("line 10"));
        assert!(msg.contains("line 20"));
        assert!(msg.contains("'hello'"));
        assert!(msg.contains("'goodbye'"));
    }

    #[test]
    fn unbalanced_quotes_error_shows_context() {
        let occ = QuoteOccurrence {
            line_number: LineNumber(100),
            column: Column(12),
            context: r#"she said "hello"#.to_string(),
            char_found: '"',
        };

        let err = IrieBookError::UnbalancedQuotes {
            count: QuoteCount(3),
            last_occurrence: occ,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("Unbalanced quotes"));
        assert!(msg.contains("found 3"));
        assert!(msg.contains("must be even"));
        assert!(msg.contains("line 100"));
    }

    #[test]
    fn file_errors_include_path() {
        let err = IrieBookError::FileRead {
            path: "/some/file.md".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("/some/file.md"));
        assert!(msg.contains("Failed to read"));
    }
}
