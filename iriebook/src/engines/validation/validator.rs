//! Quote validation logic
//!
//! Validates quotes before conversion:
//! - Distinguishes apostrophes from quotation marks
//! - Detects single quotation marks (errors!)
//! - Ensures double quotes are balanced (even count)
//! - Extracts context for error reporting

use crate::engines::traits::ValidatorEngine;
use crate::utilities::error::{IrieBookError, QuoteOccurrence};
use crate::utilities::types::{Column, LineNumber, QuoteCount};
use anyhow::Result;

const CONTEXT_CHARS: usize = 20;

/// Type of single quote character
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleQuoteType {
    /// Legitimate apostrophe (will be converted to curly)
    Apostrophe,
    /// Actual quotation mark (error!)
    QuotationMark,
}

/// Concrete implementation of the ValidatorEngine trait
pub struct Validator;

impl ValidatorEngine for Validator {
    fn validate(&self, content: &str) -> Result<(), IrieBookError> {
        validate_impl(content)
    }
}

/// Validate quotes in the content (free function for backward compatibility)
///
/// Returns Ok(()) if valid, or IrieBookError if problems found
pub fn validate(content: &str) -> Result<(), IrieBookError> {
    Validator.validate(content)
}

/// Implementation of the validation logic
fn validate_impl(content: &str) -> Result<(), IrieBookError> {
    // First check for single quotes
    let single_quotes = find_single_quotes(content);
    if !single_quotes.is_empty() {
        return Err(IrieBookError::SingleQuotesFound {
            count: single_quotes.len(),
            occurrences: single_quotes,
        });
    }

    // Then check for balanced double quotes
    let double_quote_count = count_double_quotes(content);
    if !double_quote_count.0.is_multiple_of(2) {
        // Find the last occurrence for error reporting
        let last_occurrence = find_last_double_quote(content)?;
        return Err(IrieBookError::UnbalancedQuotes {
            count: double_quote_count,
            last_occurrence,
        });
    }

    Ok(())
}

/// Check if a character is a word boundary
fn is_word_boundary(ch: Option<char>) -> bool {
    match ch {
        None => true, // Start or end of string
        Some(c) => c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\n' | '\r' | '\t'),
    }
}

/// Classify a single quote as apostrophe or quotation mark
pub fn classify_single_quote(prev_char: Option<char>, next_char: Option<char>) -> SingleQuoteType {
    // 1. Contraction: letter + ' + letter (it's, can't, don't)
    if matches!(prev_char, Some(c) if c.is_alphabetic())
        && matches!(next_char, Some(c) if c.is_alphabetic())
    {
        return SingleQuoteType::Apostrophe;
    }

    // 2. Possessive: specifically letter + ' + s (John's) OR s + ' + boundary (James')
    if matches!(prev_char, Some(c) if c.is_alphabetic()) {
        // Pattern 1: John's, Mary's (any letter + 's)
        if matches!(next_char, Some('s')) {
            return SingleQuoteType::Apostrophe;
        }
        // Pattern 2: Word-final apostrophe at word boundary
        // Supports English possessives (James'), Romanian contractions (dracu'),
        // and other languages with word-final apostrophes
        if is_word_boundary(next_char) {
            return SingleQuoteType::Apostrophe;
        }
    }

    // 3. Abbreviated year: (space|start) + ' + digit
    if is_word_boundary(prev_char)
        && matches!(next_char, Some(c) if c.is_ascii_digit())
    {
        return SingleQuoteType::Apostrophe;
    }

    // 4. Omitted letter at start: only specific known contractions
    // 'cause, 'til, 'em, 'bout, 'round, 'tis, 'twas, etc.
    if is_word_boundary(prev_char)
        && let Some(nc) = next_char
    {
        // Check for known contraction starts
        if matches!(nc, 'c' | 't' | 'e' | 'b' | 'r' | 'n') {
            // These are common starts of contractions
            // Be conservative: only classify as apostrophe if it's lowercase
            // This helps distinguish 'cause from 'Hello'
            if nc.is_lowercase() {
                return SingleQuoteType::Apostrophe;
            }
        }
    }

    // Default: likely quotation mark
    // This includes cases like 'Hello', 'goodbye', etc. (uppercase or not in known list)
    SingleQuoteType::QuotationMark
}

/// Find all single quote QUOTATION MARKS in the content (not apostrophes)
fn find_single_quotes(content: &str) -> Vec<QuoteOccurrence> {
    let mut occurrences = Vec::new();
    let mut line_number = LineNumber(1);
    let mut column = Column(1);
    let chars: Vec<char> = content.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '\'' => {
                let prev = i.checked_sub(1).map(|idx| chars[idx]);
                let next = chars.get(i + 1).copied();

                let quote_type = classify_single_quote(prev, next);

                // Only report quotation marks, not apostrophes
                if matches!(quote_type, SingleQuoteType::QuotationMark) {
                    let context = extract_context(content, line_number, column);
                    occurrences.push(QuoteOccurrence {
                        line_number,
                        column,
                        context,
                        char_found: '\'',
                    });
                }
                column.0 += 1;
            }
            '\n' => {
                line_number.0 += 1;
                column = Column(1);
            }
            _ => {
                column.0 += 1;
            }
        }
    }

    occurrences
}

/// Count straight double quotes in content
fn count_double_quotes(content: &str) -> QuoteCount {
    let count = content.chars().filter(|&ch| ch == '"').count();
    QuoteCount(count)
}

/// Find the last double quote occurrence
fn find_last_double_quote(content: &str) -> Result<QuoteOccurrence, IrieBookError> {
    let mut last_occurrence = None;
    let mut line_number = LineNumber(1);
    let mut column = Column(1);

    for ch in content.chars() {
        match ch {
            '"' => {
                let context = extract_context(content, line_number, column);
                last_occurrence = Some(QuoteOccurrence {
                    line_number,
                    column,
                    context,
                    char_found: '"',
                });
                column.0 += 1;
            }
            '\n' => {
                line_number.0 += 1;
                column = Column(1);
            }
            _ => {
                column.0 += 1;
            }
        }
    }

    match last_occurrence {
        Some(occ) => Ok(occ),
        None => Err(IrieBookError::OutputPathError {
            input: "no quotes found".to_string(),
        }),
    }
}

/// Extract context around a position in the content
fn extract_context(content: &str, line_number: LineNumber, column: Column) -> String {
    // Find the line
    let lines: Vec<&str> = content.lines().collect();
    let line_index = line_number.0.saturating_sub(1);
    
    match lines.get(line_index) {
        Some(line) => {
            let col_index = column.0.saturating_sub(1);

            // Extract the substring
            let chars: Vec<char> = line.chars().collect();

            // Calculate start and end for context (use chars.len() for UTF-8 safety)
            let start = col_index.saturating_sub(CONTEXT_CHARS);
            let end = (col_index + CONTEXT_CHARS + 1).min(chars.len());

            let context_chars: String = chars[start..end].iter().collect();
            
            // Add ellipsis if truncated
            let prefix = if start > 0 { "..." } else { "" };
            let suffix = if end < chars.len() { "..." } else { "" };
            
            format!("{}{}{}", prefix, context_chars, suffix)
        }
        None => String::from("(context unavailable)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_clean_content() {
        let content = r#"She said "hello" and "goodbye"."#;
        assert!(validate(content).is_ok());
    }

    #[test]
    fn detects_single_quotes() {
        // Test isolated quotes at sentence boundaries
        let content = "She said. ' ' He replied.";
        let result = validate(content);

        match result {
            Err(IrieBookError::SingleQuotesFound { count, occurrences }) => {
                assert_eq!(count, 2);
                assert_eq!(occurrences.len(), 2);
                assert_eq!(occurrences[0].line_number, LineNumber(1));
                assert_eq!(occurrences[0].char_found, '\'');
            }
            _ => panic!("Expected SingleQuotesFound error"),
        }
    }

    #[test]
    fn detects_unbalanced_quotes() {
        let content = r#"She said "hello but never closed it"#;
        let result = validate(content);
        
        match result {
            Err(IrieBookError::UnbalancedQuotes { count, .. }) => {
                assert_eq!(count, QuoteCount(1));
            }
            _ => panic!("Expected UnbalancedQuotes error"),
        }
    }

    #[test]
    fn accepts_balanced_quotes() {
        let content = r#"First "quote" and second "quote" here."#;
        assert!(validate(content).is_ok());
    }

    #[test]
    fn extracts_context_correctly() {
        let content = "This is a test with 'quote' in the middle";
        let context = extract_context(content, LineNumber(1), Column(21));
        
        // Should contain the quote and surrounding text
        assert!(context.contains("with"));
        assert!(context.contains("quote"));
    }

    #[test]
    fn handles_context_at_line_start() {
        let content = "'quote at start";
        let context = extract_context(content, LineNumber(1), Column(1));
        assert!(context.contains("quote"));
        assert!(!context.starts_with("..."));
    }

    #[test]
    fn handles_context_at_line_end() {
        let content = "quote at end'";
        let context = extract_context(content, LineNumber(1), Column(13));
        assert!(context.contains("quote"));
        assert!(!context.ends_with("..."));
    }

    #[test]
    fn counts_double_quotes() {
        let content = r#"One "two" three "four" five"#;
        let count = count_double_quotes(content);
        assert_eq!(count, QuoteCount(4));
    }

    #[test]
    fn tracks_line_and_column_correctly() {
        // Use isolated quotes
        let content = "Line 1\nLine 2 with ' '\nLine 3";
        let single_quotes = find_single_quotes(content);

        assert_eq!(single_quotes.len(), 2);
        assert_eq!(single_quotes[0].line_number, LineNumber(2));
        assert_eq!(single_quotes[0].column, Column(13)); // after "Line 2 with "
    }

    #[test]
    fn single_quote_context_includes_surrounding_text() {
        // Use isolated quotes
        let content = "She said ' some ' to me";
        let occurrences = find_single_quotes(content);

        // Note: isolated quotes are quotation marks, not apostrophes
        assert_eq!(occurrences.len(), 2);
        assert!(occurrences[0].context.contains("said"));
        assert!(occurrences[0].context.contains("some"));
    }

    // Classification tests
    #[test]
    fn classifies_contractions_as_apostrophes() {
        // it's
        assert_eq!(
            classify_single_quote(Some('t'), Some('s')),
            SingleQuoteType::Apostrophe
        );
        // can't
        assert_eq!(
            classify_single_quote(Some('n'), Some('t')),
            SingleQuoteType::Apostrophe
        );
        // we're
        assert_eq!(
            classify_single_quote(Some('e'), Some('r')),
            SingleQuoteType::Apostrophe
        );
    }

    #[test]
    fn classifies_possessives_as_apostrophes() {
        // John's
        assert_eq!(
            classify_single_quote(Some('n'), Some('s')),
            SingleQuoteType::Apostrophe
        );
        // James' (no 's', at word boundary)
        assert_eq!(
            classify_single_quote(Some('s'), Some(' ')),
            SingleQuoteType::Apostrophe
        );
        // wife's
        assert_eq!(
            classify_single_quote(Some('e'), Some('s')),
            SingleQuoteType::Apostrophe
        );
    }

    #[test]
    fn classifies_abbreviated_years_as_apostrophes() {
        // '70
        assert_eq!(
            classify_single_quote(Some(' '), Some('7')),
            SingleQuoteType::Apostrophe
        );
        // '80s
        assert_eq!(
            classify_single_quote(None, Some('8')),
            SingleQuoteType::Apostrophe
        );
        // '90
        assert_eq!(
            classify_single_quote(Some('\n'), Some('9')),
            SingleQuoteType::Apostrophe
        );
    }

    #[test]
    fn classifies_omitted_letters_as_apostrophes() {
        // 'cause (lowercase)
        assert_eq!(
            classify_single_quote(Some(' '), Some('c')),
            SingleQuoteType::Apostrophe
        );
        // 'til
        assert_eq!(
            classify_single_quote(Some(' '), Some('t')),
            SingleQuoteType::Apostrophe
        );
        // 'em
        assert_eq!(
            classify_single_quote(Some(' '), Some('e')),
            SingleQuoteType::Apostrophe
        );
    }

    #[test]
    fn classifies_opening_quotes_as_quotation_marks() {
        // 'Hello (uppercase after quote)
        assert_eq!(
            classify_single_quote(Some(' '), Some('H')),
            SingleQuoteType::QuotationMark
        );
        // 'Goodbye
        assert_eq!(
            classify_single_quote(None, Some('G')),
            SingleQuoteType::QuotationMark
        );
    }

    #[test]
    fn classifies_closing_quotes_as_quotation_marks() {
        // Uppercase after quote indicates dialogue
        // 'Hello - uppercase H indicates dialogue, not a contraction
        assert_eq!(
            classify_single_quote(Some(' '), Some('H')),
            SingleQuoteType::QuotationMark
        );

        // Isolated quote
        assert_eq!(
            classify_single_quote(Some(' '), Some(' ')),
            SingleQuoteType::QuotationMark
        );
    }

    #[test]
    fn allows_apostrophes_in_validation() {
        // Content with contractions and possessives
        let content = "It's John's book from the '70s.";
        assert!(validate(content).is_ok());
    }

    #[test]
    fn allows_multiple_apostrophes() {
        let content = "It's can't won't '70s '80s '90s";
        assert!(validate(content).is_ok());
    }

    #[test]
    fn errors_on_actual_quotation_marks() {
        // Dialogue with isolated single quotes
        let content = "She said ' and then ' to me.";
        let result = validate(content);

        assert!(result.is_err());
        match result {
            Err(IrieBookError::SingleQuotesFound { count, .. }) => {
                assert_eq!(count, 2); // Opening and closing quote
            }
            _ => panic!("Expected SingleQuotesFound error"),
        }
    }

    #[test]
    fn mixed_apostrophes_and_quotes() {
        // Has apostrophes (ok) AND quotation marks (error - isolated quotes)
        let content = "It's wrong to say ' like ' that.";
        let result = validate(content);

        assert!(result.is_err());
        match result {
            Err(IrieBookError::SingleQuotesFound { count, .. }) => {
                // Should only find the 2 quotation marks, not the apostrophe in "It's"
                assert_eq!(count, 2);
            }
            _ => panic!("Expected SingleQuotesFound error for quotes, not apostrophe"),
        }
    }

    #[test]
    fn classifies_romanian_word_final_apostrophe_as_apostrophe() {
        // Romanian contraction: dracu' (from dracului)
        assert_eq!(
            classify_single_quote(Some('u'), Some(' ')),
            SingleQuoteType::Apostrophe
        );

        // Romanian contraction: într' (into)
        assert_eq!(
            classify_single_quote(Some('r'), Some(' ')),
            SingleQuoteType::Apostrophe
        );
    }

    #[test]
    fn word_final_apostrophes_after_any_letter() {
        // After 'u'
        assert_eq!(
            classify_single_quote(Some('u'), Some(' ')),
            SingleQuoteType::Apostrophe
        );

        // After 'n'
        assert_eq!(
            classify_single_quote(Some('n'), Some(' ')),
            SingleQuoteType::Apostrophe
        );

        // After 'r' with comma
        assert_eq!(
            classify_single_quote(Some('r'), Some(',')),
            SingleQuoteType::Apostrophe
        );

        // After 't' at end of string
        assert_eq!(
            classify_single_quote(Some('t'), None),
            SingleQuoteType::Apostrophe
        );
    }
}
