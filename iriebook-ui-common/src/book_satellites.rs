use anyhow::{Result, bail};
use serde::Serialize;
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Type)]
pub struct BookSatelliteFile {
    pub file_name: String,
    pub label: String,
}

const BOOK_SATELLITE_FILES: [(&str, &str); 2] =
    [("copyright.txt", "Copyright"), ("blurb.md", "Blurb")];

pub fn known_book_satellite_files() -> Vec<BookSatelliteFile> {
    BOOK_SATELLITE_FILES
        .iter()
        .map(|(file_name, label)| BookSatelliteFile {
            file_name: (*file_name).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

pub fn is_known_book_satellite_file(file_name: &str) -> bool {
    BOOK_SATELLITE_FILES
        .iter()
        .any(|(known_file_name, _)| file_name.eq_ignore_ascii_case(known_file_name))
}

pub fn ensure_book_satellite_file(book_path: &Path, file_name: &str) -> Result<PathBuf> {
    if !is_known_book_satellite_file(file_name) {
        bail!("Unknown book satellite file: {file_name}");
    }

    let book_dir = book_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Book file has no parent directory"))?;
    let satellite_path = book_dir.join(file_name);

    if !satellite_path.exists() {
        fs::write(&satellite_path, "")?;
    }

    Ok(satellite_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ensures_known_satellite_file_next_to_book() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "# Book")?;

        let satellite_path = ensure_book_satellite_file(&book_path, "blurb.md")?;

        assert_eq!(satellite_path, temp_dir.path().join("blurb.md"));
        assert!(satellite_path.exists());
        assert_eq!(fs::read_to_string(satellite_path)?, "");

        Ok(())
    }

    #[test]
    fn rejects_unknown_satellite_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "# Book")?;

        let result = ensure_book_satellite_file(&book_path, "notes.md");

        assert!(result.is_err());
        assert!(!temp_dir.path().join("notes.md").exists());

        Ok(())
    }
}
