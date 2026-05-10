use crate::resource_access::file;
use crate::utilities::error::IrieBookError;
use crate::utilities::types::{BookMetadata, SeriesBook, roman_numeral};
use std::fs;
use std::path::{Path, PathBuf};

pub trait SeriesMetadataProvider: Send + Sync {
    fn previous_books_for(
        &self,
        book_path: &Path,
        metadata: &BookMetadata,
    ) -> Result<Vec<SeriesBook>, IrieBookError>;
}

pub struct WorkspaceSeriesMetadataProvider {
    workspace_path: PathBuf,
}

impl WorkspaceSeriesMetadataProvider {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }
}

impl SeriesMetadataProvider for WorkspaceSeriesMetadataProvider {
    fn previous_books_for(
        &self,
        book_path: &Path,
        metadata: &BookMetadata,
    ) -> Result<Vec<SeriesBook>, IrieBookError> {
        let Some(collection) = metadata.belongs_to_collection.as_deref() else {
            return Ok(Vec::new());
        };
        let Some(current_position) = metadata.group_position else {
            return Ok(Vec::new());
        };

        let mut metadata_paths = Vec::new();
        let search_root = series_search_root(&self.workspace_path);
        collect_metadata_paths(&search_root, &mut metadata_paths)?;

        let current_metadata_path = book_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("metadata.yaml");
        let current_metadata_path = current_metadata_path.canonicalize().ok();

        let mut books = Vec::new();
        for metadata_path in metadata_paths {
            if current_metadata_path.as_ref().is_some_and(|current| {
                metadata_path
                    .canonicalize()
                    .ok()
                    .as_ref()
                    .is_some_and(|candidate| candidate == current)
            }) {
                continue;
            }

            let candidate_book_path = metadata_path.with_file_name("book.md");
            let Ok(Some(candidate)) = file::load_metadata(&candidate_book_path) else {
                continue;
            };

            if candidate.belongs_to_collection.as_deref() != Some(collection) {
                continue;
            }

            let Some(position) = candidate.group_position else {
                continue;
            };

            if position >= current_position {
                continue;
            }

            books.push(SeriesBook {
                title: candidate.title,
                author: candidate.author,
                collection: collection.to_string(),
                position,
                roman_position: roman_numeral(position),
            });
        }

        books.sort_by_key(|book| book.position);
        Ok(books)
    }
}

fn series_search_root(path: &Path) -> PathBuf {
    if path.join("metadata.yaml").exists() {
        return path.parent().unwrap_or(path).to_path_buf();
    }

    path.to_path_buf()
}

fn collect_metadata_paths(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), IrieBookError> {
    let entries = fs::read_dir(dir).map_err(|source| IrieBookError::FileRead {
        path: dir.to_string_lossy().to_string(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| IrieBookError::FileRead {
            path: dir.to_string_lossy().to_string(),
            source,
        })?;
        let path = entry.path();

        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if matches!(name, ".git" | "target" | "yard" | "irie" | "node_modules") {
                continue;
            }
            collect_metadata_paths(&path, paths)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some("metadata.yaml") {
            paths.push(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_metadata(dir: &Path, title: &str, series: &str, position: u32) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("metadata.yaml"),
            format!(
                "---\ntitle: {}\nauthor: Author\nbelongs-to-collection: {}\ngroup-position: {}\n---\n",
                title, series, position
            ),
        )
        .unwrap();
    }

    #[test]
    fn roman_numerals_are_stylish() {
        assert_eq!(roman_numeral(1), "I");
        assert_eq!(roman_numeral(4), "IV");
        assert_eq!(roman_numeral(9), "IX");
        assert_eq!(roman_numeral(42), "XLII");
    }

    #[test]
    fn provider_returns_sorted_previous_books_from_same_series() {
        let temp_dir = TempDir::new().unwrap();
        write_metadata(&temp_dir.path().join("book-1"), "First", "Saga", 1);
        write_metadata(&temp_dir.path().join("book-3"), "Third", "Saga", 3);
        write_metadata(&temp_dir.path().join("other"), "Other", "Other Saga", 1);

        let provider = WorkspaceSeriesMetadataProvider::new(temp_dir.path().to_path_buf());
        let current = BookMetadata {
            title: "Second".to_string(),
            belongs_to_collection: Some("Saga".to_string()),
            group_position: Some(2),
            ..Default::default()
        };

        let books = provider
            .previous_books_for(&temp_dir.path().join("current/book.md"), &current)
            .unwrap();

        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "First");
        assert_eq!(books[0].roman_position, "I");
    }

    #[test]
    fn provider_scans_siblings_when_root_is_current_book_folder() {
        let temp_dir = TempDir::new().unwrap();
        let first_dir = temp_dir.path().join("first");
        let second_dir = temp_dir.path().join("second");
        write_metadata(&first_dir, "First", "Saga", 1);
        write_metadata(&second_dir, "Second", "Saga", 2);

        let provider = WorkspaceSeriesMetadataProvider::new(second_dir.clone());
        let current = BookMetadata {
            title: "Second".to_string(),
            belongs_to_collection: Some("Saga".to_string()),
            group_position: Some(2),
            ..Default::default()
        };

        let books = provider
            .previous_books_for(&second_dir.join("book.md"), &current)
            .unwrap();

        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "First");
    }
}
