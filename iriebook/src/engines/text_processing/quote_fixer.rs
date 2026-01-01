//! Quote conversion logic using a state machine
//!
//! Converts straight double quotes (") to curly quotes (" and ")
//! Converts straight apostrophes (') to curly apostrophes (')
//! while preserving asterisks and other markdown formatting

use crate::engines::traits::QuoteFixerEngine;
use crate::utilities::error::IrieBookError;
use crate::engines::validation::validator::{classify_single_quote, SingleQuoteType};
use anyhow::Result;

/// State of the quote parser
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuoteState {
    /// Not currently inside quotes
    Outside,
    /// Inside double quotes
    InsideDouble,
}

/// Result of quote conversion
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionResult {
    /// The converted content
    pub content: String,
    /// Number of double quotes converted
    pub quotes_converted: usize,
    /// Number of apostrophes converted
    pub apostrophes_converted: usize,
}

/// Concrete implementation of the QuoteFixerEngine trait
pub struct QuoteFixer;

impl QuoteFixerEngine for QuoteFixer {
    fn convert(&self, content: &str) -> Result<ConversionResult, IrieBookError> {
        Ok(convert_quotes_impl(content))
    }
}

/// Convert straight quotes and apostrophes to curly versions (free function for backward compatibility)
///
/// Assumes validation has already passed (no quotation marks, balanced quotes)
pub fn convert_quotes(content: &str) -> Result<ConversionResult> {
    Ok(QuoteFixer.convert(content)?)
}

/// Implementation of the quote conversion logic (infallible)
fn convert_quotes_impl(content: &str) -> ConversionResult {
    let mut result = String::with_capacity(content.len());
    let mut state = QuoteState::Outside;
    let mut quotes_converted = 0;
    let mut apostrophes_converted = 0;

    let chars: Vec<char> = content.chars().collect();

    for (idx, &ch) in chars.iter().enumerate() {
        match ch {
            '"' => {
                // Context for decision-making (unused for now, but useful for future enhancements)
                let _prev = idx.checked_sub(1).map(|i| chars[i]);
                let _next = chars.get(idx + 1).copied();

                // Decide if opening or closing based on state
                let (replacement, new_state) = match state {
                    QuoteState::Outside => {
                        // Should be opening quote (U+201C: LEFT DOUBLE QUOTATION MARK)
                        ('\u{201C}', QuoteState::InsideDouble)
                    }
                    QuoteState::InsideDouble => {
                        // Should be closing quote (U+201D: RIGHT DOUBLE QUOTATION MARK)
                        ('\u{201D}', QuoteState::Outside)
                    }
                };

                result.push(replacement);
                state = new_state;
                quotes_converted += 1;
            }

            '\'' => {
                // Get context for classification
                let prev = idx.checked_sub(1).map(|i| chars[i]);
                let next = chars.get(idx + 1).copied();

                // Classify this single quote
                let quote_type = classify_single_quote(prev, next);

                match quote_type {
                    SingleQuoteType::Apostrophe => {
                        // Convert to curly apostrophe (U+2019: RIGHT SINGLE QUOTATION MARK)
                        result.push('\u{2019}');
                        apostrophes_converted += 1;
                    }
                    SingleQuoteType::QuotationMark => {
                        // Leave as-is (will be caught by validator in earlier stage)
                        result.push(ch);
                    }
                }
            }

            // Preserve everything else as-is (including asterisks!)
            _ => {
                result.push(ch);
            }
        }
    }

    ConversionResult {
        content: result,
        quotes_converted,
        apostrophes_converted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_simple_dialogue() {
        let input = r#"She said "hello" to me."#;
        let result = convert_quotes(input).unwrap();
        
        // Check that quotes were converted
        assert!(result.content.contains('\u{201C}')); // Left curly quote
        assert!(result.content.contains('\u{201D}')); // Right curly quote
        assert!(!result.content.contains('"')); // No straight quotes
        assert_eq!(result.quotes_converted, 2);
        
        // Expected: She said "hello" to me.
        assert_eq!(result.content, "She said \u{201C}hello\u{201D} to me.");
    }

    #[test]
    fn converts_multiple_quotes() {
        let input = r#"First "one" and "two" and "three"."#;
        let result = convert_quotes(input).unwrap();
        
        assert_eq!(result.quotes_converted, 6);
        assert!(!result.content.contains('"'));
    }

    #[test]
    fn preserves_asterisks() {
        let input = r#"She *really* said "hello""#;
        let result = convert_quotes(input).unwrap();
        
        // Asterisks must be completely unchanged
        assert!(result.content.contains("*really*"));
        assert_eq!(result.quotes_converted, 2);
    }

    #[test]
    fn handles_empty_quotes() {
        let input = r#"She said """#;
        let result = convert_quotes(input).unwrap();
        
        // Should have opening and closing curly quotes right next to each other
        assert!(result.content.contains("\u{201C}\u{201D}"));
        assert_eq!(result.quotes_converted, 2);
    }

    #[test]
    fn handles_quotes_at_line_start() {
        let input = r#""Hello," she said."#;
        let result = convert_quotes(input).unwrap();
        
        // Should start with left curly quote
        assert!(result.content.starts_with('\u{201C}'));
        assert_eq!(result.quotes_converted, 2);
    }

    #[test]
    fn handles_quotes_at_line_end() {
        let input = r#"She said "hello""#;
        let result = convert_quotes(input).unwrap();
        
        // Should end with right curly quote
        assert!(result.content.ends_with('\u{201D}'));
        assert_eq!(result.quotes_converted, 2);
    }

    #[test]
    fn handles_multiline_content() {
        let input = "Line 1 with \"quote\"\nLine 2 with \"another\"\nLine 3";
        let result = convert_quotes(input).unwrap();
        
        assert_eq!(result.quotes_converted, 4);
        assert!(!result.content.contains('"'));
        assert!(result.content.contains('\n')); // Newlines preserved
    }

    #[test]
    fn alternates_quotes_correctly() {
        let input = r#""One" "Two" "Three""#;
        let result = convert_quotes(input).unwrap();
        
        assert_eq!(result.quotes_converted, 6);
        
        // Count curly quotes
        let left_count = result.content.matches('\u{201C}').count();
        let right_count = result.content.matches('\u{201D}').count();
        
        assert_eq!(left_count, 3);
        assert_eq!(right_count, 3);
    }

    #[test]
    fn preserves_romanian_characters() {
        let input = "Ea a spus \"bună ziua\" și \"la revedere\".";
        let result = convert_quotes(input).unwrap();
        
        assert_eq!(result.quotes_converted, 4);
        // UTF-8 characters must be preserved
        assert!(result.content.contains("ă"));
        assert!(result.content.contains("ș"));
    }

    #[test]
    fn handles_dialogue_with_punctuation() {
        let input = r#""Hello!" she exclaimed. "How are you?""#;
        let result = convert_quotes(input).unwrap();
        
        assert_eq!(result.quotes_converted, 4);
        assert!(!result.content.contains('"'));
    }

    #[test]
    fn preserves_other_markdown() {
        let input = r#"This is *italic* and **bold** and "quoted"."#;
        let result = convert_quotes(input).unwrap();
        
        assert!(result.content.contains("*italic*"));
        assert!(result.content.contains("**bold**"));
        assert_eq!(result.quotes_converted, 2);
    }

    #[test]
    fn quote_count_matches_input() {
        let input = r#"One "two" three "four" five "six""#;
        let result = convert_quotes(input).unwrap();
        
        // Should have 6 curly quotes total (3 opening + 3 closing)
        let left_count = result.content.matches('\u{201C}').count();
        let right_count = result.content.matches('\u{201D}').count();
        
        assert_eq!(left_count, 3);
        assert_eq!(right_count, 3);
        assert_eq!(result.quotes_converted, 6);
    }

    #[test]
    fn no_straight_quotes_remain() {
        let input = r#"Test "one" and "two""#;
        let result = convert_quotes(input).unwrap();

        // Absolutely no straight quotes should remain
        assert!(!result.content.contains('"'));
    }

    // Apostrophe conversion tests
    #[test]
    fn converts_contractions_to_curly_apostrophe() {
        let input = "It's working and can't be stopped.";
        let result = convert_quotes(input).unwrap();

        // Should convert apostrophes to curly (U+2019)
        assert!(result.content.contains("It\u{2019}s"));
        assert!(result.content.contains("can\u{2019}t"));
        assert_eq!(result.apostrophes_converted, 2);
        assert_eq!(result.quotes_converted, 0);
    }

    #[test]
    fn converts_possessives_to_curly_apostrophe() {
        let input = "John's book and James' car.";
        let result = convert_quotes(input).unwrap();

        // Should convert possessive apostrophes
        assert!(result.content.contains("John\u{2019}s"));
        assert!(result.content.contains("James\u{2019}"));
        assert_eq!(result.apostrophes_converted, 2);
    }

    #[test]
    fn converts_abbreviated_years_to_curly() {
        let input = "Back in the '70s and '80s.";
        let result = convert_quotes(input).unwrap();

        // Should convert year apostrophes
        assert!(result.content.contains("\u{2019}70s"));
        assert!(result.content.contains("\u{2019}80s"));
        assert_eq!(result.apostrophes_converted, 2);
    }

    #[test]
    fn converts_omitted_letters_to_curly() {
        let input = "'cause I said 'til tomorrow.";
        let result = convert_quotes(input).unwrap();

        // Should convert omitted letter apostrophes
        assert!(result.content.contains("\u{2019}cause"));
        assert!(result.content.contains("\u{2019}til"));
        assert_eq!(result.apostrophes_converted, 2);
    }

    #[test]
    fn preserves_single_quotation_marks_unchanged() {
        // Isolated quotation marks should NOT be converted (they'll be caught by validator)
        let input = "She said ' and ' to me.";
        let result = convert_quotes(input).unwrap();

        // Isolated quote marks should remain as straight quotes
        assert!(result.content.contains("' and '"));
        assert_eq!(result.apostrophes_converted, 0);
    }

    #[test]
    fn mixed_quotes_and_apostrophes() {
        let input = r#"It's "working" in the '70s"#;
        let result = convert_quotes(input).unwrap();

        // Double quotes should be curly
        assert!(result.content.contains("\u{201C}working\u{201D}"));
        assert_eq!(result.quotes_converted, 2);

        // Apostrophes should be curly
        assert!(result.content.contains("It\u{2019}s"));
        assert!(result.content.contains("\u{2019}70s"));
        assert_eq!(result.apostrophes_converted, 2);

        // No straight quotes or apostrophes remain
        assert!(!result.content.contains('"'));
    }

    #[test]
    fn counts_apostrophes_and_quotes_separately() {
        let input = r#"John's "book" from the '70s is "great""#;
        let result = convert_quotes(input).unwrap();

        assert_eq!(result.quotes_converted, 4); // Two pairs of double quotes
        assert_eq!(result.apostrophes_converted, 2); // John's and '70s
    }
}
