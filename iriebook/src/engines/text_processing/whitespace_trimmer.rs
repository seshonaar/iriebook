//! Whitespace trimming logic
//!
//! Cleans up excessive whitespace in markdown files:
//! - Collapses multiple consecutive spaces to single space
//! - Converts tabs to single space
//! - Limits consecutive blank lines to max 1
//! - Trims leading/trailing whitespace from lines

use crate::engines::traits::WhitespaceTrimmerEngine;
use crate::utilities::error::IrieBookError;
use anyhow::Result;

/// Result of whitespace trimming
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimmingResult {
    /// The trimmed content
    pub content: String,
    /// Number of spaces collapsed (multiple spaces → single)
    pub spaces_collapsed: usize,
    /// Number of tabs converted to spaces
    pub tabs_converted: usize,
    /// Number of blank lines removed (kept max 1 consecutive)
    pub blank_lines_removed: usize,
    /// Number of lines with trailing/leading whitespace trimmed
    pub lines_trimmed: usize,
}

/// Concrete implementation of the WhitespaceTrimmerEngine trait
pub struct WhitespaceTrimmer;

impl WhitespaceTrimmerEngine for WhitespaceTrimmer {
    fn trim(&self, content: &str) -> Result<TrimmingResult, IrieBookError> {
        Ok(trim_whitespace_impl(content))
    }
}

/// Trim excessive whitespace from content (free function for backward compatibility)
///
/// Performs four types of cleaning:
/// 1. Collapses multiple consecutive spaces to single space
/// 2. Converts tabs to single space
/// 3. Limits consecutive blank lines to max 1
/// 4. Trims leading/trailing whitespace from each line
pub fn trim_whitespace(content: &str) -> Result<TrimmingResult> {
    Ok(WhitespaceTrimmer.trim(content)?)
}

/// Implementation of the whitespace trimming logic (infallible)
fn trim_whitespace_impl(content: &str) -> TrimmingResult {
    let mut spaces_collapsed = 0;
    let mut tabs_converted = 0;
    let mut lines_trimmed = 0;

    // Pass 1: Clean each line
    let cleaned_lines: Vec<String> = content
        .lines()
        .map(|line| {
            // Track if line needs trimming
            if line.trim() != line {
                lines_trimmed += 1;
            }

            let trimmed = line.trim();

            // Convert tabs
            let tabs_in_line = trimmed.chars().filter(|&c| c == '\t').count();
            tabs_converted += tabs_in_line;
            let no_tabs = trimmed.replace('\t', " ");

            // Collapse multiple spaces
            let mut result = String::with_capacity(no_tabs.len());
            let mut last_was_space = false;

            for ch in no_tabs.chars() {
                if ch == ' ' {
                    if !last_was_space {
                        result.push(ch);
                        last_was_space = true;
                    } else {
                        spaces_collapsed += 1;
                    }
                } else {
                    result.push(ch);
                    last_was_space = false;
                }
            }

            result
        })
        .collect();

    // Pass 2: Collapse blank lines (max 1 consecutive)
    let mut final_lines = Vec::new();
    let mut blank_count = 0;
    let mut blank_lines_removed = 0;

    for line in cleaned_lines {
        if line.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                final_lines.push(line);
            } else {
                blank_lines_removed += 1;
            }
        } else {
            blank_count = 0;
            final_lines.push(line);
        }
    }

    let content = final_lines.join("\n");

    TrimmingResult {
        content,
        spaces_collapsed,
        tabs_converted,
        blank_lines_removed,
        lines_trimmed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Space collapsing tests
    #[test]
    fn collapses_double_spaces_to_single() {
        let input = "hello  world";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world");
        assert_eq!(result.spaces_collapsed, 1);
    }

    #[test]
    fn collapses_multiple_spaces_to_single() {
        let input = "hello    world";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world");
        assert_eq!(result.spaces_collapsed, 3);
    }

    #[test]
    fn collapses_multiple_instances_in_line() {
        let input = "hello  world  from  rust";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world from rust");
        assert_eq!(result.spaces_collapsed, 3);
    }

    #[test]
    fn preserves_single_spaces() {
        let input = "hello world from rust";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world from rust");
        assert_eq!(result.spaces_collapsed, 0);
    }

    // Tab conversion tests
    #[test]
    fn converts_single_tab_to_space() {
        let input = "hello\tworld";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world");
        assert_eq!(result.tabs_converted, 1);
    }

    #[test]
    fn converts_multiple_tabs() {
        let input = "hello\tworld\tfrom\trust";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world from rust");
        assert_eq!(result.tabs_converted, 3);
    }

    #[test]
    fn converts_tabs_mixed_with_spaces() {
        let input = "hello\t world\tfrom  rust";
        let result = trim_whitespace(input).unwrap();

        // Tabs become spaces, then multiple spaces collapse
        // "hello\t world" → "hello  world" (tab+space → 2 spaces) → collapse
        // "\tfrom" → " from" (tab → space) → OK
        // "from  rust" → already 2 spaces → collapse
        assert_eq!(result.content, "hello world from rust");
        assert_eq!(result.tabs_converted, 2);
        assert_eq!(result.spaces_collapsed, 2);
    }

    // Line trimming tests
    #[test]
    fn trims_leading_whitespace() {
        let input = "  hello world";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world");
        assert_eq!(result.lines_trimmed, 1);
    }

    #[test]
    fn trims_trailing_whitespace() {
        let input = "hello world  ";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world");
        assert_eq!(result.lines_trimmed, 1);
    }

    #[test]
    fn trims_both_ends() {
        let input = "  hello world  ";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "hello world");
        assert_eq!(result.lines_trimmed, 1);
    }

    #[test]
    fn trims_multiple_lines() {
        let input = "  line1  \n  line2  \nline3";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "line1\nline2\nline3");
        assert_eq!(result.lines_trimmed, 2); // line1 and line2
    }

    // Blank line collapsing tests
    #[test]
    fn collapses_two_blank_lines_to_one() {
        let input = "line1\n\n\nline2";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "line1\n\nline2");
        assert_eq!(result.blank_lines_removed, 1);
    }

    #[test]
    fn collapses_many_blank_lines_to_one() {
        let input = "line1\n\n\n\n\nline2";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "line1\n\nline2");
        assert_eq!(result.blank_lines_removed, 3); // 4 blanks → 1 blank = 3 removed
    }

    #[test]
    fn preserves_single_blank_lines() {
        let input = "line1\n\nline2\n\nline3";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "line1\n\nline2\n\nline3");
        assert_eq!(result.blank_lines_removed, 0);
    }

    #[test]
    fn handles_multiple_sections_with_excess_blanks() {
        let input = "section1\n\n\nsection2\n\n\n\nsection3";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "section1\n\nsection2\n\nsection3");
        assert_eq!(result.blank_lines_removed, 3); // 2 from first, 3 from second
    }

    // Combined scenario tests
    #[test]
    fn all_rules_applied_together() {
        let input = "  hello  world\t\tfrom  rust  \n\n\n  next  line  ";
        let result = trim_whitespace(input).unwrap();

        // All whitespace issues should be fixed:
        // Line 1: leading trim, space collapse, tab conversion, trailing trim
        // Blank lines: 2 extra removed
        // Line 2: leading trim, space collapse, trailing trim
        assert_eq!(result.content, "hello world from rust\n\nnext line");
        assert_eq!(result.lines_trimmed, 2);
        assert!(result.tabs_converted > 0);
        assert!(result.spaces_collapsed > 0);
        assert_eq!(result.blank_lines_removed, 1);
    }

    #[test]
    fn handles_empty_file() {
        let input = "";
        let result = trim_whitespace(input).unwrap();

        assert_eq!(result.content, "");
        assert_eq!(result.spaces_collapsed, 0);
        assert_eq!(result.tabs_converted, 0);
        assert_eq!(result.blank_lines_removed, 0);
        assert_eq!(result.lines_trimmed, 0);
    }

    #[test]
    fn handles_only_whitespace() {
        let input = "   \n\t\t\n   ";
        let result = trim_whitespace(input).unwrap();

        // All lines become empty after trimming, collapse to single empty line = ""
        assert_eq!(result.content, "");
        assert_eq!(result.lines_trimmed, 3);
        assert_eq!(result.blank_lines_removed, 2); // 3 blanks → 1 blank = 2 removed
    }

    #[test]
    fn handles_already_clean_content() {
        let input = "hello world\n\nfrom rust";
        let result = trim_whitespace(input).unwrap();

        // No changes needed
        assert_eq!(result.content, "hello world\n\nfrom rust");
        assert_eq!(result.spaces_collapsed, 0);
        assert_eq!(result.tabs_converted, 0);
        assert_eq!(result.blank_lines_removed, 0);
        assert_eq!(result.lines_trimmed, 0);
    }
}
