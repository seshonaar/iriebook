//! Markdown transformation logic for ebook formatting
//!
//! Transforms markdown structure to prepare for EPUB generation:
//! - Splits chapter headings with numbers or dashes
//! - Handles scene breaks (blank lines)
//! - Converts newlines to paragraph breaks
//! - Cleans up formatting artifacts

use crate::engines::traits::MarkdownTransformEngine;
use crate::utilities::error::IrieBookError;
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::rest,
    sequence::{preceded, tuple},
};
use regex::Regex;

/// Token representing a single line with metadata
#[derive(Debug, Clone, PartialEq)]
struct Token {
    content: String,
    line_number: usize,
    kind: TokenKind,
}

/// Classification of token types based on line content
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum TokenKind {
    H2Line,     // Lines starting with "## "
    H1Line,     // Lines starting with "# "
    H3Line,     // Lines starting with "### "
    ItalicLine, // Lines matching *text*
    BlankLine,  // Empty or whitespace-only
    TextLine,   // Everything else
}

/// AST node representing different content types in the markdown document
#[derive(Debug, Clone, PartialEq)]
enum ContentItem {
    Chapter {
        title: String,
        subtitle: Option<String>,
    },
    Header {
        level: u8,
        text: String,
    },
    Paragraph(String),
    ItalicLine(String),
    SceneBreak,
    BlankLine,
    Dedication(String), // H3 that is entirely italic (dedication page)
}

/// Represents a parsed chapter number with its prefix
/// e.g., "Chapter 5" -> ChapterNumber { prefix: "Chapter", number: 5 }
#[derive(Debug, Clone, PartialEq)]
struct ChapterNumber {
    prefix: String,
    number: u32,
}

/// Extract chapter number and prefix from a title string
/// Returns None for titles without numbers (Prologue, Epilogue, etc.)
fn extract_chapter_number(title: &str) -> Option<ChapterNumber> {
    let words: Vec<&str> = title.split_whitespace().collect();

    match words.len() {
        0 => None,
        1 => {
            // Single word - check if it's just a number
            words[0].parse::<u32>().ok().map(|n| ChapterNumber {
                prefix: String::new(),
                number: n,
            })
        }
        _ => {
            // Last word should be the number
            let last = words.last()?;
            match last.parse::<u32>() {
                Ok(number) => {
                    let prefix = words[..words.len() - 1].join(" ");
                    Some(ChapterNumber { prefix, number })
                }
                Err(_) => None, // No number found - e.g., "Prologue"
            }
        }
    }
}

/// Calculate Levenshtein edit distance between two strings
/// Returns the minimum number of single-character edits (insertions, deletions, substitutions)
/// needed to transform one string into the other
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    // Edge cases
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows for space efficiency
    let mut prev_row: Vec<usize> = (0..=n).collect();
    let mut curr_row: Vec<usize> = vec![0; n + 1];

    for i in 1..=m {
        curr_row[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = (prev_row[j] + 1) // deletion
                .min(curr_row[j - 1] + 1) // insertion
                .min(prev_row[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[n]
}

/// Check if two prefixes are similar enough to be considered the same series
/// Uses case-insensitive comparison and allows small edit distance for typo tolerance
fn prefixes_match(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // Exact match (case-insensitive)
    if a_lower == b_lower {
        return true;
    }

    // Allow edit distance of 2 for typo tolerance
    levenshtein_distance(&a_lower, &b_lower) <= 2
}

/// Get or create a counter for a prefix, returning the next number and incrementing
/// Uses fuzzy matching to find existing series with similar prefixes
fn get_or_create_counter(counters: &mut Vec<(String, u32)>, prefix: &str) -> (String, u32) {
    // Find existing series with matching prefix (fuzzy)
    for (existing_prefix, count) in counters.iter_mut() {
        if prefixes_match(existing_prefix, prefix) {
            let num = *count;
            *count += 1;
            // Return the canonical (first seen) prefix for consistency
            return (existing_prefix.clone(), num);
        }
    }
    // New series - start at 1, store 2 as next
    counters.push((prefix.to_string(), 2));
    (prefix.to_string(), 1)
}

/// Renumber all chapters sequentially per series
/// Each unique prefix (e.g., "Chapter", "Capitolul", "Epilog") maintains its own counter
/// Non-numbered chapters (Prologue, Epilogue, etc.) are preserved as-is
/// Fuzzy matching handles minor typos in prefixes (e.g., "Capitol" and "Capitolul" are same series)
fn renumber_chapters(items: Vec<ContentItem>) -> Vec<ContentItem> {
    // Map from canonical prefix → next number
    let mut series_counters: Vec<(String, u32)> = Vec::new();

    items
        .into_iter()
        .map(|item| match item {
            ContentItem::Chapter { title, subtitle } => {
                match extract_chapter_number(&title) {
                    Some(parsed) => {
                        // Find or create counter for this prefix
                        let (canonical_prefix, next_number) =
                            get_or_create_counter(&mut series_counters, &parsed.prefix);
                        // Build new title with canonical prefix and new number
                        let new_title = if canonical_prefix.is_empty() {
                            next_number.to_string()
                        } else {
                            format!("{} {}", canonical_prefix, next_number)
                        };
                        ContentItem::Chapter {
                            title: new_title,
                            subtitle,
                        }
                    }
                    None => {
                        // Non-numbered chapter (Prologue, etc.) - keep as-is
                        ContentItem::Chapter { title, subtitle }
                    }
                }
            }
            other => other, // Non-chapter items pass through unchanged
        })
        .collect()
}

/// Concrete implementation of the MarkdownTransformEngine trait
pub struct MarkdownTransformer;

impl MarkdownTransformEngine for MarkdownTransformer {
    fn transform(&self, content: &str) -> Result<String, IrieBookError> {
        Ok(transform_impl(content))
    }
}

/// Extract prefix and optional number from the beginning
/// Cleans escaped dashes first (same as clean_subtitle does)
fn extract_prefix_with_number(s: &str) -> Option<String> {
    // Clean escaped dashes first (same as clean_subtitle does)
    // Also handle trailing backslash (when dash is separated by parser)
    let cleaned = s
        .trim()
        .replace("\\-", "")
        .replace("\\–", "")
        .replace("\\—", "");
    let trimmed = cleaned.trim().trim_end_matches('\\').trim();

    let words: Vec<&str> = trimmed.split_whitespace().collect();

    if words.len() >= 2 {
        let last_word = words.last()?;
        if last_word.parse::<u32>().is_ok() {
            let prefix = words[..words.len() - 1].join(" ");
            return Some(format!("{} {}", prefix, last_word));
        }
    }

    Some(trimmed.to_string())
}

/// Clean subtitle text by removing escaped dashes and normalizing whitespace
fn clean_subtitle(s: &str) -> String {
    s.trim()
        .replace("\\-", "")
        .replace("\\–", "")
        .replace("\\—", "")
        .trim_matches(|c: char| c == '-' || c == '–' || c == '—')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Check if text looks like a paragraph rather than a subtitle
fn should_keep_as_subtitle(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let looks_like_paragraph =
        text.contains(',') || text.contains('.') || text.contains('!') || text.contains('?');
    !looks_like_paragraph
}

/// Check if a line is italicized (markdown format: *text*)
fn is_italic_line(line: &str) -> bool {
    let trimmed = line.trim();
    // Line is italicized if it starts with * and ends with *
    // Must have content between the asterisks (length > 2)
    trimmed.starts_with('*') && trimmed.ends_with('*') && trimmed.len() > 2
}

/// Parse chapter heading with Unicode en-dash (–) separator
fn parse_unicode_endash(input: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, (prefix, subtitle)) =
        tuple((take_until(" – "), preceded(tag(" – "), rest)))(input)?;

    Ok((remaining, (prefix, subtitle)))
}

/// Parse chapter heading with Unicode em-dash (—) separator
fn parse_unicode_emdash(input: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, (prefix, subtitle)) =
        tuple((take_until(" — "), preceded(tag(" — "), rest)))(input)?;

    Ok((remaining, (prefix, subtitle)))
}

/// Parse chapter heading with hyphen and spaces separator
fn parse_hyphen_with_spaces(input: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, (prefix, subtitle)) =
        tuple((take_until(" - "), preceded(tag(" - "), rest)))(input)?;

    Ok((remaining, (prefix, subtitle)))
}

/// Parse chapter heading with hyphen (no spaces)
fn parse_hyphen_no_spaces(input: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, (prefix, subtitle)) =
        tuple((take_until("-"), preceded(tag("-"), rest)))(input)?;

    Ok((remaining, (prefix, subtitle)))
}

/// Parse chapter heading using nom combinators
/// Tries patterns in order: en-dash, em-dash, hyphen with spaces, hyphen no spaces
fn parse_chapter_with_dash(input: &str) -> IResult<&str, (&str, &str)> {
    alt((
        parse_unicode_endash,
        parse_unicode_emdash,
        parse_hyphen_with_spaces,
        parse_hyphen_no_spaces,
    ))(input)
}

/// Parse chapter heading content and extract title and optional subtitle
/// Returns (title, Some(subtitle)) if a subtitle is found and valid, or (title, None) otherwise
fn parse_chapter_content(rest: &str) -> (String, Option<String>) {
    // Try to parse with dash patterns
    if let Ok((_, (prefix, subtitle))) = parse_chapter_with_dash(rest) {
        // Try to extract prefix with number
        if let Some(title) = extract_prefix_with_number(prefix) {
            let processed_subtitle = clean_subtitle(subtitle);
            if should_keep_as_subtitle(&processed_subtitle) {
                return (title, Some(processed_subtitle));
            }

            // Subtitle rejected, check if empty (e.g., "Chapter 1 -")
            if processed_subtitle.is_empty() {
                return (title, None);
            }
        }
    }

    // Try space-separated pattern: "Chapter 1 The Beginning"
    let words: Vec<&str> = rest.split_whitespace().collect();
    if words.len() >= 3 && words[1].parse::<u32>().is_ok() {
        let prefix = format!("{} {}", words[0], words[1]);
        let subtitle_clean = clean_subtitle(&words[2..].join(" "));

        if should_keep_as_subtitle(&subtitle_clean) {
            return (prefix, Some(subtitle_clean));
        }
    }

    // No valid pattern found, return the whole thing as title
    (rest.to_string(), None)
}

/// Tokenize the input content into classified tokens with line numbers
fn tokenize(content: &str) -> Vec<Token> {
    // Normalize line endings
    let content = content.replace("\r\n", "\n");

    content
        .lines()
        .enumerate()
        .map(|(idx, line)| {
            let kind = if line.starts_with("### ") {
                TokenKind::H3Line
            } else if line.starts_with("## ") {
                TokenKind::H2Line
            } else if line.starts_with("# ") {
                TokenKind::H1Line
            } else if line.trim().is_empty() {
                TokenKind::BlankLine
            } else if is_italic_line(line) {
                TokenKind::ItalicLine
            } else {
                TokenKind::TextLine
            };

            Token {
                content: line.to_string(),
                line_number: idx + 1,
                kind,
            }
        })
        .collect()
}

/// Parse tokens into ContentItems (AST)
fn parse_tokens(tokens: Vec<Token>) -> Vec<ContentItem> {
    let mut items = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        match token.kind {
            TokenKind::H2Line => {
                // Parse chapter heading: "## Chapter 1 - Subtitle"
                let rest = &token.content[3..]; // Skip "## "
                let (title, subtitle) = parse_chapter_content(rest);

                items.push(ContentItem::Chapter { title, subtitle });
                i += 1;
            }
            TokenKind::H1Line => {
                // H1 header
                let text = token.content[2..].to_string(); // Skip "# "
                items.push(ContentItem::Header { level: 1, text });
                i += 1;
            }
            TokenKind::H3Line => {
                // H3 header
                let text = token.content[4..].to_string(); // Skip "### "
                // Check if entire content is italic (*text*) -> dedication page
                if is_italic_line(&text) {
                    // Strip the asterisks for clean dedication text
                    let inner = text.trim().trim_matches('*').to_string();
                    items.push(ContentItem::Dedication(inner));
                } else {
                    items.push(ContentItem::Header { level: 3, text });
                }
                i += 1;
            }
            TokenKind::BlankLine => {
                // Group consecutive blank lines into a single BlankLine item
                while i < tokens.len() && matches!(tokens[i].kind, TokenKind::BlankLine) {
                    i += 1;
                }
                items.push(ContentItem::BlankLine);
            }
            TokenKind::ItalicLine => {
                items.push(ContentItem::ItalicLine(token.content.clone()));
                i += 1;
            }
            TokenKind::TextLine => {
                items.push(ContentItem::Paragraph(token.content.clone()));
                i += 1;
            }
        }
    }

    items
}

/// Check if a ContentItem is a header or chapter
fn is_header_or_chapter(item: &ContentItem) -> bool {
    matches!(
        item,
        ContentItem::Chapter { .. } | ContentItem::Header { .. }
    )
}

/// Check if a ContentItem is normal prose (paragraph or blank)
fn is_normal_prose(item: &ContentItem) -> bool {
    matches!(item, ContentItem::Paragraph(_))
}

/// Check if a ContentItem is an italic line
fn is_italic_content(item: &ContentItem) -> bool {
    matches!(item, ContentItem::ItalicLine(_))
}

/// Minimum consecutive text lines required on both sides of a blank line for scene break
const CHUNKY_PARAGRAPH_THRESHOLD: usize = 10;

/// Count consecutive text lines (paragraphs) backwards from index (exclusive)
/// Stops at any non-paragraph item (blank line, header, italic, etc.)
fn count_chunk_lines_before(items: &[ContentItem], index: usize) -> usize {
    let mut count = 0;
    let mut i = index;
    while i > 0 {
        i -= 1;
        if matches!(items[i], ContentItem::Paragraph(_)) {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Count consecutive text lines (paragraphs) forwards from index (exclusive)
/// Stops at any non-paragraph item (blank line, header, italic, etc.)
fn count_chunk_lines_after(items: &[ContentItem], index: usize) -> usize {
    let mut count = 0;
    let mut i = index + 1;
    while i < items.len() {
        if matches!(items[i], ContentItem::Paragraph(_)) {
            count += 1;
            i += 1;
        } else {
            break;
        }
    }
    count
}

/// Transform ContentItems by applying business rules
fn transform_items(items: Vec<ContentItem>) -> Vec<ContentItem> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < items.len() {
        let item = &items[i];

        match item {
            ContentItem::BlankLine => {
                // Check if we should convert this blank line to a scene break
                let prev_item = if i > 0 { Some(&items[i - 1]) } else { None };
                let next_item = if i + 1 < items.len() {
                    Some(&items[i + 1])
                } else {
                    None
                };

                // Add scene break if both surrounding items are normal prose
                // (not italic, not header, not blank) AND both chunks are "chunky" (20+ lines)
                let should_add_scene_break = match (prev_item, next_item) {
                    (Some(prev), Some(next)) => {
                        is_normal_prose(prev)
                            && is_normal_prose(next)
                            && !is_italic_content(prev)
                            && !is_italic_content(next)
                            && count_chunk_lines_before(&items, i) >= CHUNKY_PARAGRAPH_THRESHOLD
                            && count_chunk_lines_after(&items, i) >= CHUNKY_PARAGRAPH_THRESHOLD
                    }
                    _ => false,
                };

                if should_add_scene_break {
                    result.push(ContentItem::SceneBreak);
                } else {
                    result.push(ContentItem::BlankLine);
                }

                i += 1;
            }
            ContentItem::SceneBreak => {
                // Remove scene breaks adjacent to headers/chapters
                let prev_is_header = i > 0 && is_header_or_chapter(&items[i - 1]);
                let next_is_header = i + 1 < items.len() && is_header_or_chapter(&items[i + 1]);

                if !prev_is_header && !next_is_header {
                    result.push(ContentItem::SceneBreak);
                }

                i += 1;
            }
            _ => {
                result.push(item.clone());
                i += 1;
            }
        }
    }

    result
}

/// Check if two consecutive items need paragraph spacing
fn needs_paragraph_spacing(current: &ContentItem, next: &ContentItem) -> bool {
    // Add paragraph spacing between prose/italic items when there's no blank/scene break
    matches!(
        (current, next),
        (ContentItem::Paragraph(_), ContentItem::Paragraph(_))
            | (ContentItem::Paragraph(_), ContentItem::ItalicLine(_))
            | (ContentItem::ItalicLine(_), ContentItem::Paragraph(_))
            | (ContentItem::ItalicLine(_), ContentItem::ItalicLine(_))
    )
}

/// Render ContentItems back to markdown string
fn render_items(items: Vec<ContentItem>) -> String {
    let mut lines = Vec::new();

    for (i, item) in items.iter().enumerate() {
        match item {
            ContentItem::Chapter { title, subtitle } => {
                // Add proper spacing before chapter if previous item is not blank
                if i > 0
                    && !matches!(
                        items[i - 1],
                        ContentItem::BlankLine | ContentItem::SceneBreak
                    )
                {
                    lines.push(String::new());
                    lines.push(String::new()); // Add 2 blank lines before chapter
                }

                lines.push(format!("# {}", title));
                if let Some(sub) = subtitle {
                    lines.push(format!("<p class=\"subtitle\">{}</p>", sub));
                }
            }
            ContentItem::Header { level, text } => {
                // Add proper spacing before header if previous item is not blank
                if i > 0
                    && !matches!(
                        items[i - 1],
                        ContentItem::BlankLine | ContentItem::SceneBreak
                    )
                {
                    lines.push(String::new());
                    lines.push(String::new()); // Add 2 blank lines before header
                }

                let hashes = "#".repeat(*level as usize);
                lines.push(format!("{} {}", hashes, text));
            }
            ContentItem::Paragraph(text) => {
                lines.push(text.clone());
            }
            ContentItem::ItalicLine(text) => {
                lines.push(text.clone());
            }
            ContentItem::SceneBreak => {
                lines.push(String::new()); // Blank line before scene break (for Pandoc to close preceding paragraph)
                lines.push("<div class='scene-break'></div>".to_string());
                lines.push(String::new()); // Blank line after scene break
            }
            ContentItem::BlankLine => {
                lines.push(String::new());
            }
            ContentItem::Dedication(text) => {
                // Dedication page: needs its own section for proper page break
                // The H1 with .unlisted won't appear in ToC, .unnumbered removes chapter number
                lines.push(String::new());
                lines.push("# {.dedication-page .unnumbered .unlisted}".to_string());
                lines.push(String::new());
                lines.push(format!(
                    "<div class=\"dedication\"><p><em>{}</em></p></div>",
                    text
                ));
            }
        }

        // Add paragraph spacing if needed
        if i + 1 < items.len() {
            let next = &items[i + 1];

            if needs_paragraph_spacing(item, next) {
                lines.push(String::new()); // Add blank line between paragraphs
            }
        }
    }

    // Ensure proper spacing before headers (at least 2 blank lines)
    let spacing_re = Regex::new(r"(?m)([^\n])\n\n(#|##|###)").unwrap();
    let with_spacing = lines.join("\n");
    let result = spacing_re.replace_all(&with_spacing, "$1\n\n\n$2");

    result.to_string()
}

/// Implementation of the markdown transformation logic (infallible)
///
/// This uses a 5-stage parser pipeline:
/// 1. Lexer: Tokenize input into classified tokens
/// 2. Parser: Convert tokens to AST (ContentItems) using nom combinators
/// 3. Renumber: Consolidate chapter numbers sequentially
/// 4. Transformer: Apply business rules (scene breaks, spacing)
/// 5. Renderer: Convert AST back to markdown string
fn transform_impl(content: &str) -> String {
    let tokens = tokenize(content);
    let items = parse_tokens(tokens);
    let items = renumber_chapters(items);
    let items = transform_items(items);
    render_items(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // Chapter Renumbering Tests (TDD)
    // ==========================================

    #[test]
    fn extract_chapter_number_basic() {
        let result = extract_chapter_number("Chapter 5");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.prefix, "Chapter");
        assert_eq!(parsed.number, 5);
    }

    #[test]
    fn extract_chapter_number_romanian() {
        let result = extract_chapter_number("Capitolul 3");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.prefix, "Capitolul");
        assert_eq!(parsed.number, 3);
    }

    #[test]
    fn extract_chapter_number_part() {
        let result = extract_chapter_number("Part 10");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.prefix, "Part");
        assert_eq!(parsed.number, 10);
    }

    #[test]
    fn extract_chapter_number_multi_word_prefix() {
        let result = extract_chapter_number("Book One Chapter 7");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.prefix, "Book One Chapter");
        assert_eq!(parsed.number, 7);
    }

    #[test]
    fn extract_chapter_number_no_number_prologue() {
        let result = extract_chapter_number("Prologue");
        assert!(result.is_none());
    }

    #[test]
    fn extract_chapter_number_no_number_epilogue() {
        let result = extract_chapter_number("Epilogue");
        assert!(result.is_none());
    }

    #[test]
    fn extract_chapter_number_just_number() {
        let result = extract_chapter_number("42");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.prefix, "");
        assert_eq!(parsed.number, 42);
    }

    fn assert_chapter_title(item: &ContentItem, expected_title: &str) {
        match item {
            ContentItem::Chapter { title, .. } => {
                assert_eq!(title, expected_title);
            }
            _ => panic!("Expected Chapter, got {:?}", item),
        }
    }

    #[test]
    fn renumber_chapters_sequential_input() {
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 2".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 3".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Chapter 1");
        assert_chapter_title(&result[1], "Chapter 2");
        assert_chapter_title(&result[2], "Chapter 3");
    }

    #[test]
    fn renumber_chapters_gaps_in_numbering() {
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 3".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 5".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 7".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Chapter 1");
        assert_chapter_title(&result[1], "Chapter 2");
        assert_chapter_title(&result[2], "Chapter 3");
        assert_chapter_title(&result[3], "Chapter 4");
    }

    #[test]
    fn renumber_chapters_duplicates() {
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 2".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 2".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 4".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Chapter 1");
        assert_chapter_title(&result[1], "Chapter 2");
        assert_chapter_title(&result[2], "Chapter 3");
        assert_chapter_title(&result[3], "Chapter 4");
    }

    #[test]
    fn renumber_chapters_skips_prologue() {
        let items = vec![
            ContentItem::Chapter {
                title: "Prologue".to_string(),
                subtitle: Some("The Start".to_string()),
            },
            ContentItem::Chapter {
                title: "Chapter 5".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 10".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Prologue");
        assert_chapter_title(&result[1], "Chapter 1");
        assert_chapter_title(&result[2], "Chapter 2");
    }

    #[test]
    fn renumber_chapters_skips_epilogue() {
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 3".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 7".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Epilogue".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Chapter 1");
        assert_chapter_title(&result[1], "Chapter 2");
        assert_chapter_title(&result[2], "Epilogue");
    }

    #[test]
    fn renumber_chapters_mixed_with_non_chapter_items() {
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 5".to_string(),
                subtitle: None,
            },
            ContentItem::Paragraph("Some text".to_string()),
            ContentItem::BlankLine,
            ContentItem::Chapter {
                title: "Chapter 10".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Chapter 1");
        assert!(matches!(result[1], ContentItem::Paragraph(_)));
        assert!(matches!(result[2], ContentItem::BlankLine));
        assert_chapter_title(&result[3], "Chapter 2");
    }

    #[test]
    fn renumber_chapters_preserves_subtitles() {
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 5".to_string(),
                subtitle: Some("The Beginning".to_string()),
            },
            ContentItem::Chapter {
                title: "Chapter 10".to_string(),
                subtitle: Some("The Middle".to_string()),
            },
        ];

        let result = renumber_chapters(items);

        match &result[0] {
            ContentItem::Chapter { title, subtitle } => {
                assert_eq!(title, "Chapter 1");
                assert_eq!(subtitle.as_deref(), Some("The Beginning"));
            }
            _ => panic!("Expected Chapter"),
        }

        match &result[1] {
            ContentItem::Chapter { title, subtitle } => {
                assert_eq!(title, "Chapter 2");
                assert_eq!(subtitle.as_deref(), Some("The Middle"));
            }
            _ => panic!("Expected Chapter"),
        }
    }

    #[test]
    fn renumber_chapters_romanian_prefix() {
        let items = vec![
            ContentItem::Chapter {
                title: "Capitolul 3".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Capitolul 7".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Capitolul 15".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Capitolul 1");
        assert_chapter_title(&result[1], "Capitolul 2");
        assert_chapter_title(&result[2], "Capitolul 3");
    }

    // ==========================================
    // Multi-Series Renumbering Tests (TDD)
    // ==========================================

    #[test]
    fn levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn levenshtein_distance_one_char_diff() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    }

    #[test]
    fn levenshtein_distance_suffix_added() {
        // "Capitol" vs "Capitolul" - 2 chars added
        assert_eq!(levenshtein_distance("capitol", "capitolul"), 2);
    }

    #[test]
    fn levenshtein_distance_completely_different() {
        assert!(levenshtein_distance("chapter", "epilog") > 2);
    }

    #[test]
    fn prefixes_match_exact() {
        assert!(prefixes_match("Chapter", "Chapter"));
    }

    #[test]
    fn prefixes_match_case_insensitive() {
        assert!(prefixes_match("Chapter", "chapter"));
        assert!(prefixes_match("CAPITOLUL", "capitolul"));
    }

    #[test]
    fn prefixes_match_typo_tolerance() {
        // "Capitol" and "Capitolul" should match (distance 2)
        assert!(prefixes_match("Capitol", "Capitolul"));
    }

    #[test]
    fn prefixes_match_different_series() {
        // "Capitolul" and "Epilog" should NOT match
        assert!(!prefixes_match("Capitolul", "Epilog"));
        assert!(!prefixes_match("Chapter", "Part"));
    }

    #[test]
    fn renumber_chapters_multiple_series() {
        // Different prefixes should have separate counters
        let items = vec![
            ContentItem::Chapter {
                title: "Capitolul 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Capitolul 5".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Epilog 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Epilog 2".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        // Capitolul series: 1, 2
        assert_chapter_title(&result[0], "Capitolul 1");
        assert_chapter_title(&result[1], "Capitolul 2");
        // Epilog series: 1, 2 (separate counter!)
        assert_chapter_title(&result[2], "Epilog 1");
        assert_chapter_title(&result[3], "Epilog 2");
    }

    #[test]
    fn renumber_chapters_fuzzy_prefix_match() {
        // Typos in prefix should be treated as same series
        let items = vec![
            ContentItem::Chapter {
                title: "Capitol 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Capitolul 5".to_string(), // Typo fixed mid-book
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Capitolul 10".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        // All should be in same series, using first prefix encountered
        assert_chapter_title(&result[0], "Capitol 1");
        assert_chapter_title(&result[1], "Capitol 2"); // Uses "Capitol" from first chapter
        assert_chapter_title(&result[2], "Capitol 3");
    }

    #[test]
    fn renumber_chapters_interleaved_series() {
        // Series can be interleaved in document
        let items = vec![
            ContentItem::Chapter {
                title: "Chapter 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Part 1".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Chapter 5".to_string(),
                subtitle: None,
            },
            ContentItem::Chapter {
                title: "Part 3".to_string(),
                subtitle: None,
            },
        ];

        let result = renumber_chapters(items);

        assert_chapter_title(&result[0], "Chapter 1");
        assert_chapter_title(&result[1], "Part 1");
        assert_chapter_title(&result[2], "Chapter 2");
        assert_chapter_title(&result[3], "Part 2");
    }

    #[test]
    fn full_transform_multiple_series() {
        let input = r#"## Capitolul 1 - Inceput

Some text.

## Capitolul 5 - Mijloc

More text.

## Epilog 1 - Final

End text."#;

        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Capitolul series renumbered 1, 2
        assert!(result.contains("# Capitolul 1"));
        assert!(result.contains("# Capitolul 2"));
        // Epilog series stays at 1 (separate counter)
        assert!(result.contains("# Epilog 1"));
        // Should NOT have Epilog become Capitolul 3
        assert!(!result.contains("# Capitolul 3"));
    }

    #[test]
    fn full_transform_renumbers_chapters() {
        let input = r#"## Prologue - The Start

First paragraph.

## Chapter 3 - Beginning

Some text here.

## Chapter 7 - Middle

More text.

## Epilogue"#;

        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert!(result.contains("# Prologue"));
        assert!(result.contains("# Chapter 1")); // Was Chapter 3
        assert!(result.contains("# Chapter 2")); // Was Chapter 7
        assert!(result.contains("# Epilogue"));

        assert!(result.contains("<p class=\"subtitle\">The Start</p>"));
        assert!(result.contains("<p class=\"subtitle\">Beginning</p>"));
        assert!(result.contains("<p class=\"subtitle\">Middle</p>"));
    }

    // ==========================================
    // Original Tests
    // ==========================================

    #[test]
    fn transforms_chapter_heading_with_number() {
        let input = "## Chapter 1 The Beginning";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert!(result.contains("# Chapter 1"));
        assert!(result.contains("<p class=\"subtitle\">The Beginning</p>"));
    }

    #[test]
    fn transforms_chapter_heading_with_dash() {
        let input = "## Chapter - The Beginning";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert!(result.contains("# Chapter"));
        assert!(result.contains("<p class=\"subtitle\">The Beginning</p>"));
    }

    #[test]
    fn no_scene_break_with_short_prose() {
        // Short paragraphs (< 20 lines each) should NOT get scene breaks
        let input = "First paragraph\n\nSecond paragraph";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // With chunky threshold, short prose does NOT get scene breaks
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn normalizes_line_endings() {
        let input = "Line 1\r\nLine 2\r\nLine 3";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert!(!result.contains("\r\n"));
        assert!(result.contains("Line 1"));
    }

    #[test]
    fn handles_empty_subtitle() {
        let input = "## Chapter 1 -";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert_eq!(result.trim(), "# Chapter 1");
    }

    #[test]
    fn ignores_paragraph_like_subtitle() {
        let input = "## Chapter 1 This is a sentence.";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should not create subtitle because it contains a period, but should convert to H1
        assert!(!result.contains("<p class=\"subtitle\">"));
        assert!(result.contains("# Chapter 1 This is a sentence."));
    }

    #[test]
    fn does_not_break_chapter_numbers() {
        let input = "## Chapter 15";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should not create subtitle for plain chapter numbers
        assert!(!result.contains("<p class=\"subtitle\">"));
        // Chapter 15 becomes Chapter 1 due to sequential renumbering
        assert!(result.contains("# Chapter 1"));
    }

    #[test]
    fn removes_scene_break_after_h1_header() {
        let input = "# Chapter 1\n\nSome text";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break after H1
        assert!(!result.contains("# Chapter 1\n<div class='scene-break'></div>"));
        // H1 should still be present
        assert!(result.contains("# Chapter 1"));
    }

    #[test]
    fn removes_scene_break_before_h1_header() {
        let input = "Some text\n\n# Chapter 1";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break before H1
        assert!(!result.contains("<div class='scene-break'></div>\n# Chapter 1"));
    }

    #[test]
    fn removes_scene_break_after_h2_header() {
        let input = "## Section\n\nSome text";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break after header (H2 converted to H1)
        assert!(!result.contains("# Section\n<div class='scene-break'></div>"));
        // Should have the header as H1
        assert!(result.contains("# Section"));
    }

    #[test]
    fn removes_scene_break_before_h2_header() {
        let input = "Some text\n\n## Section";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break before header (H2 converted to H1)
        assert!(!result.contains("<div class='scene-break'></div>\n# Section"));
        // Should have the header as H1
        assert!(result.contains("# Section"));
    }

    #[test]
    fn removes_scene_break_after_h3_header() {
        let input = "### Subsection\n\nSome text";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break after H3
        assert!(!result.contains("### Subsection\n<div class='scene-break'></div>"));
    }

    #[test]
    fn removes_scene_break_before_h3_header() {
        let input = "Some text\n\n### Subsection";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break before H3
        assert!(!result.contains("<div class='scene-break'></div>\n### Subsection"));
    }

    #[test]
    fn removes_scene_break_after_subtitle() {
        let input = "## Chapter 1 - The Beginning\n\nSome text";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break after subtitle
        assert!(
            !result.contains(
                "<p class=\"subtitle\">The Beginning</p>\n<div class='scene-break'></div>"
            )
        );
    }

    #[test]
    fn removes_scene_break_before_subtitle() {
        let input = "Some text\n\n## Chapter 1 - The Beginning";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break before subtitle (converted to H1 + subtitle)
        assert!(!result.contains("<div class='scene-break'></div>\n# Chapter 1"));
    }

    #[test]
    fn short_paragraphs_do_not_get_scene_breaks() {
        // Short paragraphs should NOT get scene breaks (chunky threshold applies)
        let input = "First paragraph\n\nSecond paragraph";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // With chunky threshold, short prose does NOT get scene breaks
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn no_scene_break_before_italics() {
        let input = "Normal prose here.\n\n*Italicized line.*";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break before italics
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn no_scene_break_after_italics() {
        let input = "*Italicized line.*\n\nNormal prose here.";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have scene break after italics
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn no_scene_breaks_in_poetry() {
        let input = "*First line of poem.*\n\n*Second line of poem.*\n\n*Third line of poem.*";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should NOT have any scene breaks (all lines are italicized)
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn mixed_prose_and_italics_no_scene_breaks_for_short_content() {
        // Short paragraphs do not get scene breaks (chunky threshold applies)
        let input = "Normal paragraph one.\n\nNormal paragraph two.\n\n*Italic line.*\n\nNormal paragraph three.";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Content is preserved
        assert!(result.contains("Normal paragraph one."));
        assert!(result.contains("Normal paragraph two."));

        // No scene breaks because paragraphs are short (< 20 lines each)
        let scene_break_count = result.matches("<div class='scene-break'></div>").count();
        assert_eq!(scene_break_count, 0);
    }

    // ==========================================
    // Escaped Dash Handling Tests (TDD)
    // ==========================================

    #[test]
    fn extract_prefix_cleans_escaped_dashes() {
        // The backslash should be removed during prefix extraction
        let result = extract_prefix_with_number("Capitolul 13 \\");
        assert_eq!(result, Some("Capitolul 13".to_string()));
    }

    #[test]
    fn extract_prefix_cleans_escaped_endash() {
        let result = extract_prefix_with_number("Capitolul 13 \\–");
        assert_eq!(result, Some("Capitolul 13".to_string()));
    }

    #[test]
    fn extract_prefix_cleans_escaped_emdash() {
        let result = extract_prefix_with_number("Capitolul 13 \\—");
        assert_eq!(result, Some("Capitolul 13".to_string()));
    }

    #[test]
    fn full_transform_handles_escaped_dashes() {
        let input =
            "## Capitolul 13 \\- Bianca\n\nSome text.\n\n## Capitolul 21 \\- Eliza\n\nMore text.";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should have clean, sequential chapter numbers
        assert!(result.contains("# Capitolul 1"));
        assert!(result.contains("# Capitolul 2"));
        // No backslashes in output
        assert!(!result.contains("\\"));
    }

    // ==========================================
    // Chunky Paragraph Threshold Tests (TDD)
    // ==========================================

    #[test]
    fn no_scene_break_with_short_paragraphs() {
        // 5 lines before, 5 lines after - should NOT have scene break
        let input =
            "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(&input).unwrap();
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn scene_break_with_chunky_paragraphs() {
        // 20+ lines before, 20+ lines after - SHOULD have scene break
        let lines_before: Vec<String> = (1..=25).map(|i| format!("Before line {}", i)).collect();
        let lines_after: Vec<String> = (1..=25).map(|i| format!("After line {}", i)).collect();
        let input = format!("{}\n\n{}", lines_before.join("\n"), lines_after.join("\n"));
        let transformer = MarkdownTransformer;
        let result = transformer.transform(&input).unwrap();
        assert!(result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn no_scene_break_when_only_before_is_chunky() {
        // 25 lines before, 5 lines after - should NOT have scene break
        let lines_before: Vec<String> = (1..=25).map(|i| format!("Before line {}", i)).collect();
        let lines_after: Vec<String> = (1..=5).map(|i| format!("After line {}", i)).collect();
        let input = format!("{}\n\n{}", lines_before.join("\n"), lines_after.join("\n"));
        let transformer = MarkdownTransformer;
        let result = transformer.transform(&input).unwrap();
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    #[test]
    fn no_scene_break_when_only_after_is_chunky() {
        // 5 lines before, 25 lines after - should NOT have scene break
        let lines_before: Vec<String> = (1..=5).map(|i| format!("Before line {}", i)).collect();
        let lines_after: Vec<String> = (1..=25).map(|i| format!("After line {}", i)).collect();
        let input = format!("{}\n\n{}", lines_before.join("\n"), lines_after.join("\n"));
        let transformer = MarkdownTransformer;
        let result = transformer.transform(&input).unwrap();
        assert!(!result.contains("<div class='scene-break'></div>"));
    }

    // ==========================================
    // Dedication Page Tests (TDD)
    // ==========================================

    #[test]
    fn detects_italic_h3_as_dedication() {
        let input = "### *For my family*";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should have hidden H1 for Pandoc section split
        assert!(result.contains("# {.dedication-page .unnumbered .unlisted}"));
        assert!(result.contains("<div class=\"dedication\">"));
        assert!(result.contains("<em>For my family</em>"));
    }

    #[test]
    fn regular_h3_not_treated_as_dedication() {
        let input = "### Regular Section";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert!(result.contains("### Regular Section"));
        assert!(!result.contains("dedication"));
    }

    #[test]
    fn partial_italic_h3_not_treated_as_dedication() {
        let input = "### Some *italic* word";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        assert!(!result.contains("dedication"));
    }

    #[test]
    fn dedication_preserves_content() {
        // Test dedication with content that might have internal structure
        let input = "### *To those who dream*";
        let transformer = MarkdownTransformer;
        let result = transformer.transform(input).unwrap();

        // Should have the hidden H1 marker and the dedication div
        assert!(result.contains("# {.dedication-page .unnumbered .unlisted}"));
        assert!(result.contains("<div class=\"dedication\">"));
        assert!(result.contains("To those who dream"));
    }
}
