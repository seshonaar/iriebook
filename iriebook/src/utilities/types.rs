//! Type-safe wrappers using the NewType pattern
//!
//! These types prevent mixing up line numbers with columns, counts, etc.

use chrono::Datelike;
use specta::Type;
use std::fmt;

/// Line number in a file (1-indexed, human-friendly)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineNumber(pub usize);

impl fmt::Display for LineNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Column number in a line (1-indexed, human-friendly)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Column(pub usize);

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Count of quotes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuoteCount(pub usize);

impl fmt::Display for QuoteCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Character index in a string (0-indexed, computer-friendly)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharIndex(pub usize);

impl fmt::Display for CharIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Number of times a word appears
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WordCount(pub usize);

impl fmt::Display for WordCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Book identifier (e.g., ISBN)
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize, Type)]
pub struct Identifier {
    #[serde(default)]
    pub scheme: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

impl Identifier {
    /// Returns the display text if available
    pub fn display_text(&self) -> Option<String> {
        match (&self.scheme, &self.text) {
            (Some(scheme), Some(text)) => Some(format!("{}: {}", scheme, text)),
            (Some(scheme), None) => Some(scheme.clone()),
            (None, Some(text)) => Some(text.clone()),
            (None, None) => None,
        }
    }
}

impl BookMetadata {
    /// Returns the first identifier's display text if available
    pub fn identifier_display_text(&self) -> Option<String> {
        self.identifier
            .as_ref()
            .and_then(|ids| ids.first())
            .and_then(|id| id.display_text())
    }
}

/// Book metadata from YAML frontmatter
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize, Type)]
pub struct BookMetadata {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(rename = "belongs-to-collection", default)]
    pub belongs_to_collection: Option<String>,
    #[serde(rename = "group-position", default)]
    pub group_position: Option<u32>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub rights: Option<String>,
    #[serde(rename = "cover-image", default)]
    pub cover_image: Option<String>,
    #[serde(rename = "replace-pairs", default)]
    pub replace_pairs: Option<Vec<ReplacePair>>,
    #[serde(default)]
    pub identifier: Option<Vec<Identifier>>,
}

/// Source control revision information for publication artifacts
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize, Type)]
pub struct BookRevisionInfo {
    pub short_hash: String,
    pub commit_date: String, // YYYY-MM-DD
}

/// A single word replacement pair (case-sensitive, whole-word)
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize, Type)]
pub struct ReplacePair {
    pub source: String,
    pub target: String,
}

impl BookMetadata {
    /// Create metadata with defaults from book path
    pub fn with_defaults(book_path: &std::path::Path) -> Self {
        let inferred_title = book_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Book")
            .to_string();

        let current_year = chrono::Utc::now().year();

        Self {
            title: inferred_title,
            author: String::new(),
            belongs_to_collection: None,
            group_position: None,
            language: Some("ro-RO".to_string()),
            rights: Some(format!("© {} All Rights Reserved", current_year)),
            cover_image: Some("cover.jpg".to_string()),
            replace_pairs: None,
            identifier: None,
        }
    }

    /// Validate required fields
    pub fn validate(&self) -> Result<(), String> {
        match self.title.trim().is_empty() {
            true => Err("Title is required".to_string()),
            false => match self.author.trim().is_empty() {
                true => Err("Author is required".to_string()),
                false => Ok(()),
            },
        }
    }

    /// Ensure predefined defaults are set
    pub fn with_predefined_defaults(mut self) -> Self {
        if self.language.is_none() {
            self.language = Some("ro-RO".to_string());
        }

        if self.rights.is_none() {
            let current_year = chrono::Utc::now().year();
            self.rights = Some(format!("© {} All Rights Reserved", current_year));
        }

        if self.cover_image.is_none() {
            self.cover_image = Some("cover.jpg".to_string());
        }

        self
    }
}

/// Git commit information
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, Type)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String, // Unix timestamp
}

/// Git repository status
#[derive(Debug, Clone, PartialEq)]
pub struct GitStatus {
    pub ahead_by: usize,
    pub behind_by: usize,
    pub has_uncommitted: bool,
}

/// Google Docs sync information (stored in google-docs-sync.yaml)
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, Type)]
pub struct GoogleDocsSyncInfo {
    #[serde(rename = "google-doc-id")]
    pub google_doc_id: String,
    #[serde(rename = "sync-status")]
    pub sync_status: String,
}

impl GoogleDocsSyncInfo {
    /// Create new sync info for a Google Doc
    pub fn new(google_doc_id: String) -> Self {
        Self {
            google_doc_id,
            sync_status: "never_synced".to_string(),
        }
    }

    /// Mark as successfully synced
    pub fn mark_synced(&mut self) {
        self.sync_status = "synced".to_string();
    }

    /// Mark as failed with error message
    pub fn mark_failed(&mut self) {
        self.sync_status = "sync_failed".to_string();
    }
}

// --- Diff View Types ---

/// Source identifier for diff operations (file path or git revision)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Type)]
pub struct DiffSourceId(pub String);

impl fmt::Display for DiffSourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Display name for UI presentation (user-friendly label)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Type)]
pub struct DisplayName(pub String);

impl fmt::Display for DisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of diff segment for UI rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum SegmentType {
    /// Text was added (green in UI)
    Added,
    /// Text was removed (red in UI)
    Removed,
    /// Text is unchanged (no highlighting)
    Unchanged,
}

/// Single segment in a word-level diff
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Type)]
pub struct DiffSegment {
    /// Type of change (added/removed/unchanged)
    pub segment_type: SegmentType,
    /// The text content of this segment
    pub text: String,
    /// Context header (nearest preceding header) for this segment
    pub context_header: Option<String>,
}

/// Word-level change statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Type)]
pub struct WordChangeStats {
    /// Number of words added
    pub added: u32,
    /// Number of words removed
    pub removed: u32,
    /// Number of words unchanged
    pub unchanged: u32,
}

/// Core diff result from engine (segments + statistics)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Type)]
pub struct DiffResult {
    /// List of diff segments with change types
    pub segments: Vec<DiffSegment>,
    /// Summary statistics of changes
    pub stats: WordChangeStats,
}

/// Diff request from UI (input to manager)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Type)]
pub struct DiffRequest {
    /// Left source identifier (file path or git revision)
    pub left_source: DiffSourceId,
    /// Display name for left side (e.g., "Working Copy", "Previous Commit")
    pub left_display_name: DisplayName,
    /// Right source identifier
    pub right_source: DiffSourceId,
    /// Display name for right side
    pub right_display_name: DisplayName,
    /// Relative path to file within source (used by git sources)
    pub relative_path: String,
}

/// Complete diff comparison result (manager output to UI)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Type)]
pub struct DiffComparison {
    /// Left display name for UI header
    pub left_display_name: DisplayName,
    /// Right display name for UI header
    pub right_display_name: DisplayName,
    /// Diff result with segments and statistics
    pub diff: DiffResult,
}

/// Single file diff result from a git revision
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Type)]
pub struct RevisionDiff {
    /// Relative file path in the repository
    pub file_path: String,
    /// Complete diff comparison for this file
    pub comparison: DiffComparison,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_number_displays_correctly() {
        let line = LineNumber(42);
        assert_eq!(format!("{}", line), "42");
    }

    #[test]
    fn column_displays_correctly() {
        let col = Column(15);
        assert_eq!(format!("{}", col), "15");
    }

    #[test]
    fn quote_count_displays_correctly() {
        let count = QuoteCount(10);
        assert_eq!(format!("{}", count), "10");
    }

    #[test]
    fn char_index_displays_correctly() {
        let idx = CharIndex(5);
        assert_eq!(format!("{}", idx), "5");
    }

    #[test]
    fn line_numbers_are_comparable() {
        let line1 = LineNumber(10);
        let line2 = LineNumber(20);
        assert!(line1 < line2);
        assert_eq!(line1, LineNumber(10));
    }

    #[test]
    fn types_prevent_mixing() {
        // This test just demonstrates type safety at compile time
        let line = LineNumber(10);
        let col = Column(10);

        // These would be compile errors:
        // let _ = line == col;  // Can't compare LineNumber with Column
        // let _ = line + col;   // Can't add different types

        // But these work:
        assert_eq!(line, LineNumber(10));
        assert_eq!(col, Column(10));
    }

    #[test]
    fn word_count_displays_correctly() {
        let count = WordCount(42);
        assert_eq!(format!("{}", count), "42");
    }

    #[test]
    fn word_counts_are_comparable() {
        let count1 = WordCount(10);
        let count2 = WordCount(20);
        assert!(count1 < count2);
        assert_eq!(count1, WordCount(10));
    }

    // BookMetadata tests
    #[test]
    fn metadata_with_defaults_infers_title_from_path() {
        use std::path::Path;
        let path = Path::new("/books/my-story.md");
        let metadata = BookMetadata::with_defaults(path);
        assert_eq!(metadata.title, "my-story");
        assert_eq!(metadata.author, "");
        assert_eq!(metadata.belongs_to_collection, None);
        assert_eq!(metadata.group_position, None);
    }

    #[test]
    fn metadata_with_defaults_sets_predefined_fields() {
        use std::path::Path;
        let path = Path::new("/books/test.md");
        let metadata = BookMetadata::with_defaults(path);

        assert_eq!(metadata.language, Some("ro-RO".to_string()));
        assert_eq!(metadata.cover_image, Some("cover.jpg".to_string()));
        assert!(metadata.rights.is_some());
        assert!(metadata
            .rights
            .as_ref()
            .unwrap()
            .contains("All Rights Reserved"));
    }

    #[test]
    fn metadata_validate_rejects_empty_title() {
        let metadata = BookMetadata {
            title: "".to_string(),
            author: "Author".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };
        assert!(metadata.validate().is_err());
    }

    #[test]
    fn metadata_validate_rejects_empty_author() {
        let metadata = BookMetadata {
            title: "Title".to_string(),
            author: "   ".to_string(), // whitespace only
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };
        assert!(metadata.validate().is_err());
    }

    #[test]
    fn metadata_validate_accepts_valid_data() {
        let metadata = BookMetadata {
            title: "My Book".to_string(),
            author: "Jane Doe".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };
        assert!(metadata.validate().is_ok());
    }

    #[test]
    fn metadata_serialization_roundtrip_with_all_fields() {
        let metadata = BookMetadata {
            title: "Test".to_string(),
            author: "Author".to_string(),
            belongs_to_collection: Some("Series".to_string()),
            group_position: Some(2),
            language: Some("ro-RO".to_string()),
            rights: Some("© 2025 All Rights Reserved".to_string()),
            cover_image: Some("cover.jpg".to_string()),
            replace_pairs: Some(vec![ReplacePair {
                source: "Rene".to_string(),
                target: "René".to_string(),
            }]),
            identifier: Some(vec![Identifier {
                scheme: Some("ISBN-13".to_string()),
                text: Some("978-0-123456-78-9".to_string()),
            }]),
        };

        let yaml = serde_yaml::to_string(&metadata).unwrap();
        let deserialized: BookMetadata = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(deserialized.title, metadata.title);
        assert_eq!(deserialized.author, metadata.author);
        assert_eq!(deserialized.language, metadata.language);
        assert_eq!(deserialized.rights, metadata.rights);
        assert_eq!(deserialized.cover_image, metadata.cover_image);
        assert_eq!(deserialized.replace_pairs, metadata.replace_pairs);
        assert_eq!(
            deserialized.identifier_display_text(),
            metadata.identifier_display_text()
        );
    }

    #[test]
    fn metadata_with_predefined_defaults_fills_missing_fields() {
        let metadata = BookMetadata {
            title: "Test".to_string(),
            author: "Author".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };

        let with_defaults = metadata.with_predefined_defaults();

        assert_eq!(with_defaults.language, Some("ro-RO".to_string()));
        assert_eq!(with_defaults.cover_image, Some("cover.jpg".to_string()));
        assert!(with_defaults.rights.is_some());
        assert!(with_defaults
            .rights
            .as_ref()
            .unwrap()
            .contains("All Rights Reserved"));
    }

    #[test]
    fn metadata_with_predefined_defaults_preserves_existing() {
        let metadata = BookMetadata {
            title: "Test".to_string(),
            author: "Author".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: Some("en-US".to_string()),
            rights: Some("Custom rights".to_string()),
            cover_image: Some("custom.jpg".to_string()),
            replace_pairs: Some(vec![ReplacePair {
                source: "foo".to_string(),
                target: "bar".to_string(),
            }]),
            identifier: None,
        };

        let with_defaults = metadata.with_predefined_defaults();

        // Should preserve existing values
        assert_eq!(with_defaults.language, Some("en-US".to_string()));
        assert_eq!(with_defaults.rights, Some("Custom rights".to_string()));
        assert_eq!(with_defaults.cover_image, Some("custom.jpg".to_string()));
        assert!(with_defaults.replace_pairs.as_ref().map(|p| p.len()) == Some(1));
    }

    #[test]
    fn identifier_display_text_works() {
        let id = Identifier {
            scheme: Some("ISBN-13".to_string()),
            text: Some("978-0-123456-78-9".to_string()),
        };
        assert_eq!(
            id.display_text(),
            Some("ISBN-13: 978-0-123456-78-9".to_string())
        );

        let id_no_scheme = Identifier {
            scheme: None,
            text: Some("978-0-123456-78-9".to_string()),
        };
        assert_eq!(
            id_no_scheme.display_text(),
            Some("978-0-123456-78-9".to_string())
        );

        let id_empty = Identifier::default();
        assert_eq!(id_empty.display_text(), None);
    }
}
