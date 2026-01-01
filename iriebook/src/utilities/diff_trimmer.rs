use crate::utilities::types::{DiffSegment, SegmentType};
use std::collections::HashSet;

/// Configuration for context trimming
#[derive(Debug, Clone, Copy)]
pub struct ContextConfig {
    /// Number of words to keep before and after each change
    pub context_words: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self { context_words: 20 }
    }
}

/// Trims unchanged segments to keep only context around changes
///
/// This function analyzes a list of diff segments and keeps only those that are changed
/// (added or removed) along with a configurable number of unchanged segments before and
/// after each change. This is useful for reducing the amount of data sent across process
/// boundaries when comparing large files like book manuscripts.
///
/// # Arguments
///
/// * `segments` - The list of diff segments to trim
/// * `config` - Configuration specifying how many words of context to keep
///
/// # Returns
///
/// A new vector of segments with only the relevant context. Ellipsis markers are inserted
/// where content was trimmed to provide visual feedback to the user.
///
/// # Examples
///
/// ```
/// use iriebook::utilities::diff_trimmer::{trim_segments_with_context, ContextConfig};
/// use iriebook::utilities::types::{DiffSegment, SegmentType};
///
/// let segments = vec![
///     DiffSegment { segment_type: SegmentType::Unchanged, text: "many ".repeat(100), context_header: None },
///     DiffSegment { segment_type: SegmentType::Added, text: "new".to_string(), context_header: None },
///     DiffSegment { segment_type: SegmentType::Unchanged, text: " words".to_string(), context_header: None },
/// ];
///
/// let config = ContextConfig { context_words: 5 };
/// let trimmed = trim_segments_with_context(segments, config);
///
/// // The result will have fewer segments, with ellipsis markers where content was removed
/// assert!(trimmed.len() < 100);
/// ```
pub fn trim_segments_with_context(
    segments: Vec<DiffSegment>,
    config: ContextConfig,
) -> Vec<DiffSegment> {
    if segments.is_empty() {
        return segments;
    }

    // Step 1: Identify which segments to keep
    let mut keep_indices = HashSet::new();

    for (i, segment) in segments.iter().enumerate() {
        if segment.segment_type != SegmentType::Unchanged {
            // Keep the changed segment itself
            keep_indices.insert(i);

            // Mark context before (context_words segments back)
            let start = i.saturating_sub(config.context_words);
            for j in start..i {
                keep_indices.insert(j);
            }

            // Mark context after (context_words segments forward)
            let end = (i + config.context_words + 1).min(segments.len());
            for j in (i + 1)..end {
                keep_indices.insert(j);
            }
        }
    }

    // If no changes found, return empty (no context needed)
    if keep_indices.is_empty() {
        return Vec::new();
    }

    // Step 2: Build result with ellipsis markers
    let mut result = Vec::new();
    let mut last_kept_index = None;

    for (i, segment) in segments.into_iter().enumerate() {
        if keep_indices.contains(&i) {
            // Check if we skipped content (insert ellipsis marker)
            if let Some(last_idx) = last_kept_index
                && i > last_idx + 1
            {
                // Insert ellipsis marker for trimmed content
                result.push(DiffSegment {
                    segment_type: SegmentType::Unchanged,
                    text: "\n\n... [content trimmed] ...\n\n".to_string(),
                    context_header: None,
                });
            }

            result.push(segment);
            last_kept_index = Some(i);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_large_unchanged_blocks() {
        // Create two widely-separated changes to test ellipsis insertion
        let mut segments = Vec::new();

        // 50 unchanged segments
        for i in 0..50 {
            segments.push(DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: format!("word{} ", i),
                context_header: None,
            });
        }

        // First change at index 50
        segments.push(DiffSegment {
            segment_type: SegmentType::Added,
            text: "CHANGE1".to_string(),
            context_header: None,
        });

        // 50 unchanged segments in between
        for i in 0..50 {
            segments.push(DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: format!("middle{} ", i),
                context_header: None,
            });
        }

        // Second change at index 101
        segments.push(DiffSegment {
            segment_type: SegmentType::Added,
            text: "CHANGE2".to_string(),
            context_header: None,
        });

        // 50 unchanged segments after
        for i in 0..50 {
            segments.push(DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: format!("word{} ", i + 100),
                context_header: None,
            });
        }

        let config = ContextConfig { context_words: 5 };
        let trimmed = trim_segments_with_context(segments, config);

        // Should keep: context around CHANGE1 (11) + ellipsis (1) + context around CHANGE2 (11) = 23
        assert!(
            trimmed.len() < 50,
            "Should trim large unchanged blocks, got {} segments (expected ~23)",
            trimmed.len()
        );
        assert!(
            trimmed.iter().any(|s| s.text.contains("trimmed")),
            "Should insert ellipsis marker between the two context windows"
        );
    }

    #[test]
    fn merges_nearby_changes_into_one_context_window() {
        let segments = vec![
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "a ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Added,
                text: "change1".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "b ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "c ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Added,
                text: "change2".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "d ".to_string(),
                context_header: None,
            },
        ];

        let config = ContextConfig { context_words: 2 };
        let trimmed = trim_segments_with_context(segments, config);

        // Should keep all segments (changes are close enough to merge context)
        assert_eq!(
            trimmed.len(),
            6,
            "Nearby changes should share context window"
        );
    }

    #[test]
    fn handles_no_changes() {
        let segments = vec![
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "all".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "unchanged".to_string(),
                context_header: None,
            },
        ];

        let config = ContextConfig { context_words: 20 };
        let trimmed = trim_segments_with_context(segments, config);

        assert!(
            trimmed.is_empty(),
            "No changes = empty result (optimization)"
        );
    }

    #[test]
    fn handles_empty_input() {
        let segments: Vec<DiffSegment> = Vec::new();
        let config = ContextConfig::default();
        let trimmed = trim_segments_with_context(segments, config);

        assert!(trimmed.is_empty(), "Empty input = empty output");
    }

    #[test]
    fn preserves_all_changes() {
        let segments = vec![
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "before ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Added,
                text: "added".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Removed,
                text: "removed".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: " after".to_string(),
                context_header: None,
            },
        ];

        let config = ContextConfig { context_words: 1 };
        let trimmed = trim_segments_with_context(segments, config);

        // Should have: before + added + removed + after (all kept since changes are adjacent)
        assert_eq!(
            trimmed.len(),
            4,
            "All changes and immediate context should be preserved"
        );
        assert!(
            trimmed.iter().any(|s| s.text == "added"),
            "Should preserve added segment"
        );
        assert!(
            trimmed.iter().any(|s| s.text == "removed"),
            "Should preserve removed segment"
        );
    }

    #[test]
    fn inserts_ellipsis_at_boundaries() {
        let segments = vec![
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "a ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "b ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "c ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Added,
                text: "CHANGE1".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "d ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "e ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "f ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "g ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "h ".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Added,
                text: "CHANGE2".to_string(),
                context_header: None,
            },
            DiffSegment {
                segment_type: SegmentType::Unchanged,
                text: "i ".to_string(),
                context_header: None,
            },
        ];

        let config = ContextConfig { context_words: 1 };
        let trimmed = trim_segments_with_context(segments, config);

        // Should have ellipsis between the two changes' context windows
        let ellipsis_count = trimmed
            .iter()
            .filter(|s| s.text.contains("trimmed"))
            .count();
        assert_eq!(
            ellipsis_count, 1,
            "Should insert exactly one ellipsis marker between separate context windows"
        );
    }
}
