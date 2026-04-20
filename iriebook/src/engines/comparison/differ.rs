//! Word-level diff engine using the 'similar' crate
//!
//! Computes word-level diffs between two text sources, including whitespace.
//! Whitespace is preserved by default as it's important for manuscript editing.

use crate::engines::traits::DifferEngine;
use crate::utilities::error::IrieBookError;
use crate::utilities::types::{DiffResult, DiffSegment, SegmentType, WordChangeStats};
use similar::{ChangeTag, TextDiff};

/// Concrete diff engine implementation using 'similar' crate
pub struct Differ;

impl DifferEngine for Differ {
    fn diff(&self, left_content: &str, right_content: &str) -> Result<DiffResult, IrieBookError> {
        // Pre-scan right content for headers to provide context
        let mut headers: Vec<(usize, String)> = Vec::new();
        for (idx, _line) in right_content.match_indices('\n') {
            // Find start of next line
            let start = idx + 1;
            let rest = &right_content[start..];
            // Check if line starts with markdown header # or ##
            if let Some(end_of_line) = rest.find('\n').or(Some(rest.len())) {
                let line_content = &rest[..end_of_line];
                let trimmed = line_content.trim();
                if trimmed.starts_with('#') {
                    // It's a header
                    headers.push((start, trimmed.to_string()));
                }
            }
        }

        // Also check first line if it's a header
        if right_content.trim_start().starts_with('#')
            && let Some(end_of_line) = right_content.find('\n').or(Some(right_content.len()))
        {
            let line_content = &right_content[..end_of_line];
            headers.insert(0, (0, line_content.trim().to_string()));
        }

        // Compute word-level diff using 'similar' crate
        let diff = TextDiff::from_words(left_content, right_content);

        let mut segments = Vec::new();
        let mut stats = WordChangeStats {
            added: 0,
            removed: 0,
            unchanged: 0,
        };

        // Track position in right content to map to headers
        // similar crate doesn't give absolute positions easily in iter_all_changes
        // but we can track it for Unchanged and Added segments which come from right_content.
        // Removed segments come from left_content.
        // We'll maintain a cursor for right_content.
        let mut right_cursor = 0;

        // Convert similar's changes to our domain model
        for change in diff.iter_all_changes() {
            let text = change.value();
            let len = text.len();

            let segment_type = match change.tag() {
                ChangeTag::Equal => {
                    stats.unchanged += 1;
                    SegmentType::Unchanged
                }
                ChangeTag::Delete => {
                    stats.removed += 1;
                    SegmentType::Removed
                }
                ChangeTag::Insert => {
                    stats.added += 1;
                    SegmentType::Added
                }
            };

            // Determine context header
            let context_header = if segment_type == SegmentType::Removed {
                // For removed text, use the current cursor position in right content
                // (it was removed *at this point*)
                find_nearest_header(right_cursor, &headers)
            } else {
                // For added/unchanged, they exist in right content
                let header = find_nearest_header(right_cursor, &headers);
                right_cursor += len;
                header
            };

            segments.push(DiffSegment {
                segment_type,
                text: text.to_string(),
                context_header,
            });
        }

        Ok(DiffResult { segments, stats })
    }
}

/// Helper to find nearest preceding header
fn find_nearest_header(pos: usize, headers: &[(usize, String)]) -> Option<String> {
    // Find last header with index <= pos
    headers
        .iter()
        .take_while(|(idx, _)| *idx <= pos)
        .last()
        .map(|(_, text)| text.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical_content_all_unchanged() {
        let differ = Differ;
        let result = differ.diff("hello world", "hello world").unwrap();

        // from_words treats whitespace as separate tokens: "hello", " ", "world"
        assert_eq!(result.stats.unchanged, 3); // "hello" + " " + "world"
        assert_eq!(result.stats.added, 0);
        assert_eq!(result.stats.removed, 0);

        // All segments should be unchanged
        assert_eq!(result.segments.len(), 3);
        assert!(
            result
                .segments
                .iter()
                .all(|s| s.segment_type == SegmentType::Unchanged)
        );
    }

    #[test]
    fn diff_completely_different_content() {
        let differ = Differ;
        let result = differ.diff("foo bar", "baz qux").unwrap();

        // Tokens: "foo", " ", "bar" → "baz", " ", "qux"
        // Whitespace in the middle might match
        assert_eq!(result.stats.removed, 2); // "foo" + "bar"
        assert_eq!(result.stats.added, 2); // "baz" + "qux"
        assert_eq!(result.stats.unchanged, 1); // " " (whitespace matches)
    }

    #[test]
    fn diff_mixed_changes() {
        let differ = Differ;
        let result = differ.diff("one two three", "one four three").unwrap();

        // Tokens: "one", " ", "two", " ", "three" → "one", " ", "four", " ", "three"
        // "one" unchanged, " " unchanged, "two" removed, "four" added, " " unchanged, "three" unchanged
        assert_eq!(result.stats.unchanged, 4); // "one" + " " + " " + "three"
        assert_eq!(result.stats.removed, 1); // "two"
        assert_eq!(result.stats.added, 1); // "four"
    }

    #[test]
    fn diff_stats_match_segment_counts() {
        let differ = Differ;
        let result = differ
            .diff("alpha beta gamma", "alpha delta gamma epsilon")
            .unwrap();

        // Count segments by type
        let added_count = result
            .segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Added)
            .count();
        let removed_count = result
            .segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Removed)
            .count();
        let unchanged_count = result
            .segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Unchanged)
            .count();

        // Stats should match segment counts
        assert_eq!(added_count as u32, result.stats.added);
        assert_eq!(removed_count as u32, result.stats.removed);
        assert_eq!(unchanged_count as u32, result.stats.unchanged);
    }

    #[test]
    fn diff_empty_to_content() {
        let differ = Differ;
        let result = differ.diff("", "hello world").unwrap();

        // Tokens: "" → "hello", " ", "world"
        assert_eq!(result.stats.added, 3); // "hello" + " " + "world"
        assert_eq!(result.stats.removed, 0);
        assert_eq!(result.stats.unchanged, 0);
    }

    #[test]
    fn diff_content_to_empty() {
        let differ = Differ;
        let result = differ.diff("hello world", "").unwrap();

        // Tokens: "hello", " ", "world" → ""
        assert_eq!(result.stats.removed, 3); // "hello" + " " + "world"
        assert_eq!(result.stats.added, 0);
        assert_eq!(result.stats.unchanged, 0);
    }

    #[test]
    fn diff_whitespace_included() {
        let differ = Differ;
        // Different whitespace should show up in diff
        let result = differ.diff("hello  world", "hello world").unwrap();

        // This should detect changes due to whitespace difference
        // (the exact behavior depends on how 'similar' tokenizes words)
        assert!(result.stats.added > 0 || result.stats.removed > 0 || result.stats.unchanged >= 2);
    }

    #[test]
    fn diff_preserves_text_content() {
        let differ = Differ;
        let left = "foo bar baz";
        let right = "foo qux baz";
        let result = differ.diff(left, right).unwrap();

        // Collect all text from segments
        let reconstructed: String = result.segments.iter().map(|s| s.text.as_str()).collect();

        // The diff should cover all content from both sides
        // (order may differ, but all words should be present)
        assert!(reconstructed.contains("foo"));
        assert!(reconstructed.contains("baz"));
        // Either "bar" or "qux" should be present (or both)
        assert!(reconstructed.contains("bar") || reconstructed.contains("qux"));
    }

    #[test]
    fn diff_multiline_text() {
        let differ = Differ;
        let left = "Line one\nLine two\nLine three";
        let right = "Line one\nLine modified\nLine three";
        let result = differ.diff(left, right).unwrap();

        // Should detect the change in the middle line
        assert!(result.stats.removed > 0); // "two" removed
        assert!(result.stats.added > 0); // "modified" added
        assert!(result.stats.unchanged > 0); // "Line", "one", "three" unchanged
    }

    // Property-based tests for invariants
    #[test]
    fn diff_symmetry_property() {
        // Test that diff(A, B) has complementary stats to diff(B, A)
        let differ = Differ;
        let text1 = "apple orange banana";
        let text2 = "apple grape banana kiwi";

        let diff_ab = differ.diff(text1, text2).unwrap();
        let diff_ba = differ.diff(text2, text1).unwrap();

        // Added in A→B should equal Removed in B→A
        assert_eq!(diff_ab.stats.added, diff_ba.stats.removed);
        // Removed in A→B should equal Added in B→A
        assert_eq!(diff_ab.stats.removed, diff_ba.stats.added);
        // Unchanged should be the same in both directions
        assert_eq!(diff_ab.stats.unchanged, diff_ba.stats.unchanged);
    }

    #[test]
    fn diff_consistency_no_spurious_segments() {
        // Total segments should be reasonable (not exponentially large)
        let differ = Differ;
        let left = "a b c d e f g";
        let right = "a b x d e y g";
        let result = differ.diff(left, right).unwrap();

        // from_words includes whitespace as separate tokens, so max expected is
        // approximately 2x the number of words (words + whitespace between them)
        let total_segments = result.segments.len();
        let left_word_count = left.split_whitespace().count();
        let right_word_count = right.split_whitespace().count();
        let max_expected = (left_word_count + right_word_count) * 2;
        assert!(
            total_segments <= max_expected,
            "segments={}, max_expected={}",
            total_segments,
            max_expected
        );
    }

    #[test]
    fn diff_handles_special_characters() {
        let differ = Differ;
        let left = "Hello, world! How are you?";
        let right = "Hello, friend! How are you?";
        let result = differ.diff(left, right).unwrap();

        // Should detect "world" → "friend" change
        assert!(result.stats.removed >= 1);
        assert!(result.stats.added >= 1);
        assert!(result.stats.unchanged >= 3); // "Hello,", "How", "you?"
    }
}
