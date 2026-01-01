use serde::{Deserialize, Serialize, Serializer, Deserializer};
use specta::Type;
use std::path::{Path, PathBuf};
use iriebook::utilities::types::{BookMetadata, GoogleDocsSyncInfo};

/// NewType wrapper for book file path
#[derive(Debug, Clone, PartialEq, Eq, Hash, Type)]
#[specta(transparent)]
pub struct BookPath(PathBuf);

impl Serialize for BookPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BookPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        PathBuf::deserialize(deserializer).map(BookPath)
    }
}

impl BookPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl From<PathBuf> for BookPath {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}

/// NewType wrapper for publish checkbox state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Type)]
#[specta(transparent)]
pub struct PublishEnabled(bool);

impl Serialize for PublishEnabled {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PublishEnabled {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        bool::deserialize(deserializer).map(PublishEnabled)
    }
}

impl PublishEnabled {
    pub fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    pub fn is_enabled(&self) -> bool {
        self.0
    }
}

impl From<bool> for PublishEnabled {
    fn from(enabled: bool) -> Self {
        Self::new(enabled)
    }
}

/// NewType wrapper for word stats checkbox state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Type)]
#[specta(transparent)]
pub struct WordStatsEnabled(bool);

impl Serialize for WordStatsEnabled {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WordStatsEnabled {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        bool::deserialize(deserializer).map(WordStatsEnabled)
    }
}

impl WordStatsEnabled {
    pub fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    pub fn is_enabled(&self) -> bool {
        self.0
    }
}

impl From<bool> for WordStatsEnabled {
    fn from(enabled: bool) -> Self {
        Self::new(enabled)
    }
}

/// NewType wrapper for folder path in session
#[derive(Debug, Clone, PartialEq, Eq, Hash, Type)]
#[specta(transparent)]
pub struct FolderPath(PathBuf);

impl Serialize for FolderPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FolderPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        PathBuf::deserialize(deserializer).map(FolderPath)
    }
}

impl FolderPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }

    pub fn exists(&self) -> bool {
        self.0.exists()
    }
}

impl From<PathBuf> for FolderPath {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}

impl From<String> for FolderPath {
    fn from(s: String) -> Self {
        Self::new(PathBuf::from(s))
    }
}

/// Information about a book file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct BookInfo {
    pub path: BookPath,
    pub display_name: String,
    pub selected: bool,
    pub cover_image_path: Option<PathBuf>,
    pub metadata: Option<BookMetadata>,
    pub google_docs_sync_info: Option<GoogleDocsSyncInfo>,
    /// List of changed files (relative to book folder) with uncommitted git changes
    pub git_changed_files: Vec<String>,
}

impl BookInfo {
    pub fn new(path: BookPath, display_name: String) -> Self {
        Self {
            path,
            display_name,
            selected: false,
            cover_image_path: None,
            metadata: None,
            google_docs_sync_info: None,
            git_changed_files: Vec::new(),
        }
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn with_cover_image(mut self, cover_path: Option<PathBuf>) -> Self {
        self.cover_image_path = cover_path;
        self
    }

    pub fn with_metadata(mut self, metadata: Option<BookMetadata>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_google_docs_sync_info(mut self, sync_info: Option<GoogleDocsSyncInfo>) -> Self {
        self.google_docs_sync_info = sync_info;
        self
    }

    pub fn with_git_changed_files(mut self, changed_files: Vec<String>) -> Self {
        self.git_changed_files = changed_files;
        self
    }

    /// Check if book has any uncommitted git changes
    pub fn has_git_changes(&self) -> bool {
        !self.git_changed_files.is_empty()
    }
}

/// Overall UI state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct UiState {
    pub books: Vec<BookInfo>,
    pub publish_enabled: PublishEnabled,
    pub word_stats_enabled: WordStatsEnabled,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            books: Vec::new(),
            publish_enabled: PublishEnabled::default(),
            word_stats_enabled: WordStatsEnabled::default(),
        }
    }

    pub fn with_books(mut self, books: Vec<BookInfo>) -> Self {
        self.books = books;
        self
    }

    pub fn set_publish_enabled(&mut self, enabled: bool) {
        self.publish_enabled = PublishEnabled::new(enabled);
    }

    pub fn set_word_stats_enabled(&mut self, enabled: bool) {
        self.word_stats_enabled = WordStatsEnabled::new(enabled);
    }

    pub fn selected_books(&self) -> impl Iterator<Item = &BookInfo> {
        self.books.iter().filter(|book| book.selected)
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_path_new() {
        let path = PathBuf::from("/path/to/book.md");
        let book_path = BookPath::new(path.clone());
        assert_eq!(book_path.as_path(), path.as_path());
    }

    #[test]
    fn test_book_path_into_inner() {
        let path = PathBuf::from("/path/to/book.md");
        let book_path = BookPath::new(path.clone());
        assert_eq!(book_path.into_inner(), path);
    }

    #[test]
    fn test_book_path_from() {
        let path = PathBuf::from("/path/to/book.md");
        let book_path: BookPath = path.clone().into();
        assert_eq!(book_path.as_path(), path.as_path());
    }

    #[test]
    fn test_publish_enabled_default() {
        let publish = PublishEnabled::default();
        assert!(!publish.is_enabled());
    }

    #[test]
    fn test_publish_enabled_new() {
        let publish = PublishEnabled::new(true);
        assert!(publish.is_enabled());
    }

    #[test]
    fn test_word_stats_enabled_default() {
        let word_stats = WordStatsEnabled::default();
        assert!(!word_stats.is_enabled());
    }

    #[test]
    fn test_word_stats_enabled_new() {
        let word_stats = WordStatsEnabled::new(true);
        assert!(word_stats.is_enabled());
    }

    #[test]
    fn test_book_info_new() {
        let path = BookPath::from(PathBuf::from("/path/to/book.md"));
        let info = BookInfo::new(path.clone(), "Book Title".to_string());
        assert_eq!(info.path, path);
        assert_eq!(info.display_name, "Book Title");
        assert!(!info.selected);
    }

    #[test]
    fn test_book_info_with_selected() {
        let path = BookPath::from(PathBuf::from("/path/to/book.md"));
        let info = BookInfo::new(path, "Book Title".to_string()).with_selected(true);
        assert!(info.selected);
    }

    #[test]
    fn test_book_info_with_git_changes() {
        let path = BookPath::from(PathBuf::from("/path/to/book.md"));
        let info = BookInfo::new(path, "Book".to_string())
            .with_git_changed_files(vec!["metadata.yaml".to_string(), "cover.jpg".to_string()]);
        assert!(info.has_git_changes());
        assert_eq!(info.git_changed_files.len(), 2);
    }

    #[test]
    fn test_book_info_default_git_changes_false() {
        let path = BookPath::from(PathBuf::from("/path/to/book.md"));
        let info = BookInfo::new(path, "Book".to_string());
        assert!(!info.has_git_changes());
        assert!(info.git_changed_files.is_empty());
    }

    #[test]
    fn test_ui_state_default() {
        let state = UiState::default();
        assert!(state.books.is_empty());
        assert!(!state.publish_enabled.is_enabled());
        assert!(!state.word_stats_enabled.is_enabled());
    }

    #[test]
    fn test_ui_state_with_books() {
        let path = BookPath::from(PathBuf::from("/path/to/book.md"));
        let book = BookInfo::new(path, "Book Title".to_string());
        let state = UiState::new().with_books(vec![book.clone()]);
        assert_eq!(state.books.len(), 1);
        assert_eq!(state.books[0], book);
    }

    #[test]
    fn test_ui_state_set_publish_enabled() {
        let mut state = UiState::new();
        state.set_publish_enabled(true);
        assert!(state.publish_enabled.is_enabled());
    }

    #[test]
    fn test_ui_state_set_word_stats_enabled() {
        let mut state = UiState::new();
        state.set_word_stats_enabled(true);
        assert!(state.word_stats_enabled.is_enabled());
    }

    #[test]
    fn test_ui_state_selected_books_empty() {
        let state = UiState::new();
        assert_eq!(state.selected_books().count(), 0);
    }

    #[test]
    fn test_ui_state_selected_books_with_selection() {
        let path1 = BookPath::from(PathBuf::from("/path/to/book1.md"));
        let path2 = BookPath::from(PathBuf::from("/path/to/book2.md"));
        let book1 = BookInfo::new(path1, "Book 1".to_string()).with_selected(true);
        let book2 = BookInfo::new(path2, "Book 2".to_string());

        let state = UiState::new().with_books(vec![book1.clone(), book2]);
        let selected: Vec<_> = state.selected_books().collect();

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0], &book1);
    }

    // Serialization roundtrip tests
    #[test]
    fn test_book_path_serialization_roundtrip() {
        let original = BookPath::from(PathBuf::from("/path/to/book.md"));
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: BookPath = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_publish_enabled_serialization_roundtrip() {
        let original = PublishEnabled::new(true);
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: PublishEnabled = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_word_stats_enabled_serialization_roundtrip() {
        let original = WordStatsEnabled::new(false);
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: WordStatsEnabled = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_folder_path_serialization_roundtrip() {
        let original = FolderPath::from("/home/user/books".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: FolderPath = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_folder_path_exists() {
        let temp_dir = std::env::temp_dir();
        let folder = FolderPath::from(temp_dir);
        assert!(folder.exists());

        let nonexistent = FolderPath::from("/nonexistent/path".to_string());
        assert!(!nonexistent.exists());
    }
}
