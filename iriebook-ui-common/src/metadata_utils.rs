use crate::ui_state::BookInfo;
use iriebook::utilities::types::BookMetadata;

/// Collect distinct author names from all books with metadata
pub fn collect_distinct_authors(books: &[BookInfo]) -> Vec<String> {
    let mut authors: Vec<String> = books
        .iter()
        .filter_map(|book| book.metadata.as_ref())
        .map(|metadata| metadata.author.trim().to_string())
        .filter(|author| !author.is_empty())
        .collect();
    authors.sort();
    authors.dedup();
    authors
}

/// Collect distinct series/collection names from all books
pub fn collect_distinct_series(books: &[BookInfo]) -> Vec<String> {
    let mut series: Vec<String> = books
        .iter()
        .filter_map(|book| book.metadata.as_ref())
        .filter_map(|metadata| metadata.belongs_to_collection.as_ref())
        .map(|collection| collection.trim().to_string())
        .filter(|collection| !collection.is_empty())
        .collect();
    series.sort();
    series.dedup();
    series
}

/// Helper struct to manage metadata editing state
/// separating data management from UI framework details
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataEditState {
    pub title: String,
    pub author: String,
    pub belongs_to_collection: String,
    pub group_position: String,
    pub original: BookMetadata,
}

impl MetadataEditState {
    pub fn from_metadata(metadata: &BookMetadata) -> Self {
        Self {
            title: metadata.title.clone(),
            author: metadata.author.clone(),
            belongs_to_collection: metadata.belongs_to_collection.clone().unwrap_or_default(),
            group_position: metadata
                .group_position
                .map(|p| p.to_string())
                .unwrap_or_default(),
            original: metadata.clone(),
        }
    }

    pub fn to_metadata(&self) -> Result<BookMetadata, String> {
        let group_position = match self.group_position.trim().is_empty() {
            true => None,
            false => match self.group_position.parse::<u32>() {
                Ok(val) => Some(val),
                Err(_) => return Err("Group position must be a number".to_string()),
            },
        };

        let belongs_to_collection = match self.belongs_to_collection.trim().is_empty() {
            true => None,
            false => Some(self.belongs_to_collection.clone()),
        };

        let metadata = BookMetadata {
            title: self.title.clone(),
            author: self.author.clone(),
            belongs_to_collection,
            group_position,
            language: self.original.language.clone(),
            rights: self.original.rights.clone(),
            cover_image: self.original.cover_image.clone(),
            replace_pairs: self.original.replace_pairs.clone(),
        };

        // Validate the metadata
        metadata.validate()?;

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_state::BookPath;
    use std::path::PathBuf;

    fn create_test_metadata(author: &str, series: Option<&str>) -> BookMetadata {
        BookMetadata {
            title: "Test Title".to_string(),
            author: author.to_string(),
            belongs_to_collection: series.map(|s| s.to_string()),
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
        }
    }

    #[test]
    fn test_collect_distinct_authors() {
        let book1 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book1.md")),
            "Book 1".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author A", None)));

        let book2 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book2.md")),
            "Book 2".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author B", None)));

        let books = vec![book1, book2];
        let authors = collect_distinct_authors(&books);

        assert_eq!(authors.len(), 2);
        assert!(authors.contains(&"Author A".to_string()));
        assert!(authors.contains(&"Author B".to_string()));
    }

    #[test]
    fn test_collect_distinct_authors_empty() {
        let books: Vec<BookInfo> = vec![];
        let authors = collect_distinct_authors(&books);
        assert!(authors.is_empty());
    }

    #[test]
    fn test_collect_distinct_authors_deduplication() {
        let book1 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book1.md")),
            "Book 1".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Jane Doe", None)));

        let book2 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book2.md")),
            "Book 2".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Jane Doe", None)));

        let books = vec![book1, book2];
        let authors = collect_distinct_authors(&books);

        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0], "Jane Doe");
    }

    #[test]
    fn test_collect_distinct_authors_trims_whitespace() {
        let book = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book.md")),
            "Book".to_string(),
        )
        .with_metadata(Some(create_test_metadata("  Spaced Author  ", None)));

        let books = vec![book];
        let authors = collect_distinct_authors(&books);

        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0], "Spaced Author");
    }

    #[test]
    fn test_collect_distinct_authors_filters_empty() {
        let book = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book.md")),
            "Book".to_string(),
        )
        .with_metadata(Some(create_test_metadata("   ", None)));

        let books = vec![book];
        let authors = collect_distinct_authors(&books);

        assert!(authors.is_empty());
    }

    #[test]
    fn test_collect_distinct_authors_alphabetical_sort() {
        let book1 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book1.md")),
            "Book 1".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Zebra Author", None)));

        let book2 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book2.md")),
            "Book 2".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Alpha Author", None)));

        let books = vec![book1, book2];
        let authors = collect_distinct_authors(&books);

        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0], "Alpha Author");
        assert_eq!(authors[1], "Zebra Author");
    }

    #[test]
    fn test_collect_distinct_series() {
        let book1 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book1.md")),
            "Book 1".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("Series A"))));

        let book2 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book2.md")),
            "Book 2".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("Series B"))));

        let books = vec![book1, book2];
        let series = collect_distinct_series(&books);

        assert_eq!(series.len(), 2);
        assert!(series.contains(&"Series A".to_string()));
        assert!(series.contains(&"Series B".to_string()));
    }

    #[test]
    fn test_collect_distinct_series_empty() {
        let books: Vec<BookInfo> = vec![];
        let series = collect_distinct_series(&books);
        assert!(series.is_empty());
    }

    #[test]
    fn test_collect_distinct_series_with_none() {
        let book = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book.md")),
            "Book".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", None)));

        let books = vec![book];
        let series = collect_distinct_series(&books);

        assert!(series.is_empty());
    }

    #[test]
    fn test_collect_distinct_series_deduplication() {
        let book1 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book1.md")),
            "Book 1".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("Same Series"))));

        let book2 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book2.md")),
            "Book 2".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("Same Series"))));

        let books = vec![book1, book2];
        let series = collect_distinct_series(&books);

        assert_eq!(series.len(), 1);
        assert_eq!(series[0], "Same Series");
    }

    #[test]
    fn test_collect_distinct_series_trims_whitespace() {
        let book = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book.md")),
            "Book".to_string(),
        )
        .with_metadata(Some(create_test_metadata(
            "Author",
            Some("  Spaced Series  "),
        )));

        let books = vec![book];
        let series = collect_distinct_series(&books);

        assert_eq!(series.len(), 1);
        assert_eq!(series[0], "Spaced Series");
    }

    #[test]
    fn test_collect_distinct_series_filters_empty() {
        let book = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book.md")),
            "Book".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("   "))));

        let books = vec![book];
        let series = collect_distinct_series(&books);

        assert!(series.is_empty());
    }

    #[test]
    fn test_collect_distinct_series_alphabetical_sort() {
        let book1 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book1.md")),
            "Book 1".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("Zebra Series"))));

        let book2 = BookInfo::new(
            BookPath::from(PathBuf::from("/test/book2.md")),
            "Book 2".to_string(),
        )
        .with_metadata(Some(create_test_metadata("Author", Some("Alpha Series"))));

        let books = vec![book1, book2];
        let series = collect_distinct_series(&books);

        assert_eq!(series.len(), 2);
        assert_eq!(series[0], "Alpha Series");
        assert_eq!(series[1], "Zebra Series");
    }

    #[test]
    fn test_edit_state_roundtrip() {
        let original = create_test_metadata("Author", Some("Series"));
        let state = MetadataEditState::from_metadata(&original);

        assert_eq!(state.author, "Author");
        assert_eq!(state.belongs_to_collection, "Series");

        let result = state.to_metadata().unwrap();
        assert_eq!(result.author, "Author");
        assert_eq!(result.belongs_to_collection, Some("Series".to_string()));
    }

    #[test]
    fn test_edit_state_validation() {
        let original = create_test_metadata("Author", None);
        let mut state = MetadataEditState::from_metadata(&original);

        // Invalid: empty title
        state.title = "".to_string();
        assert!(state.to_metadata().is_err());

        // Restore title
        state.title = "Valid Title".to_string();

        // Invalid: bad group position
        state.group_position = "not a number".to_string();
        assert!(state.to_metadata().is_err());

        // Valid
        state.group_position = "10".to_string();
        assert!(state.to_metadata().is_ok());
    }
}
