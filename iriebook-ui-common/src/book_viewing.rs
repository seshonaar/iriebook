//! Book output discovery.

use anyhow::{Context, Result};
use iriebook::resource_access::file;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum BookOutputFormat {
    Epub,
    Pdf,
    Azw3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BookOutputLink {
    pub format: BookOutputFormat,
    pub path: String,
}

pub fn get_available_book_outputs(book_path: &Path) -> Result<Vec<BookOutputLink>> {
    let epub_path = file::get_output_file_name(book_path).context(
        "Failed to get output paths. Ensure metadata.yaml exists with title and author fields.",
    )?;

    let pdf_path = epub_path.with_extension("pdf");
    let azw3_path = epub_path.with_extension("azw3");

    let mut outputs = Vec::new();

    if epub_path.exists() {
        outputs.push(BookOutputLink {
            format: BookOutputFormat::Epub,
            path: epub_path.to_string_lossy().into_owned(),
        });
    }

    if pdf_path.exists() {
        outputs.push(BookOutputLink {
            format: BookOutputFormat::Pdf,
            path: pdf_path.to_string_lossy().into_owned(),
        });
    }

    if azw3_path.exists() {
        outputs.push(BookOutputLink {
            format: BookOutputFormat::Azw3,
            path: azw3_path.to_string_lossy().into_owned(),
        });
    }

    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_available_book_outputs_requires_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();

        let result = get_available_book_outputs(&book_path);
        assert!(
            result.is_err(),
            "Expected error when metadata.yaml is missing"
        );
    }

    #[test]
    fn test_get_available_book_outputs_only_returns_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();
        fs::write(
            temp_dir.path().join("metadata.yaml"),
            "title: Test Book\nauthor: Test Author\n",
        )
        .unwrap();

        let epub_path = file::get_output_file_name(&book_path).unwrap();
        fs::write(&epub_path, "epub").unwrap();
        fs::write(epub_path.with_extension("pdf"), "pdf").unwrap();

        let outputs = get_available_book_outputs(&book_path).unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].format, BookOutputFormat::Epub);
        assert_eq!(outputs[1].format, BookOutputFormat::Pdf);
    }
}
