//! Session persistence for iriebook-ui
//!
//! Saves and restores session state including:
//! - Current folder path
//! - Selected book paths

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::ui_state::{BookPath, FolderPath};

/// Session data to persist between application runs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SessionData {
    /// Last selected folder path
    pub folder_path: FolderPath,

    /// Paths of books that were selected
    pub selected_book_paths: Vec<BookPath>,

    /// Whether "Current Book" mode is enabled (actions apply to viewed book only)
    /// Defaults to true for backward compatibility with existing session files
    #[serde(default = "default_current_book_mode")]
    pub current_book_mode: bool,
}

fn default_current_book_mode() -> bool {
    true
}

impl SessionData {
    pub fn new(
        folder_path: FolderPath,
        selected_book_paths: Vec<BookPath>,
        current_book_mode: bool,
    ) -> Self {
        Self {
            folder_path,
            selected_book_paths,
            current_book_mode,
        }
    }

    /// Validate session data - check if folder and selected books exist
    /// Returns validated session with only existing books, or None if folder doesn't exist
    pub fn validate(self) -> Option<Self> {
        // If folder doesn't exist, session is invalid
        match self.folder_path.exists() {
            false => None,
            true => {
                // Filter selected books to only those that exist
                let valid_books: Vec<BookPath> = self
                    .selected_book_paths
                    .into_iter()
                    .filter(|book_path| book_path.as_path().exists())
                    .collect();

                Some(Self {
                    folder_path: self.folder_path,
                    selected_book_paths: valid_books,
                    current_book_mode: self.current_book_mode,
                })
            }
        }
    }
}

/// Get the session file path based on OS
///
/// Linux: ~/.local/state/iriebook-ui/session.json
/// macOS/Windows: <data_local_dir>/iriebook-ui/session.json
fn get_session_file_path() -> Result<PathBuf> {
    // Try Linux-specific state directory first
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let session_path = home.join(".local/state/iriebook-ui/session.json");
            return Ok(session_path);
        }
        // Fall through to data_local_dir
    }

    // Fallback for macOS/Windows or if Linux home_dir fails
    match dirs::data_local_dir() {
        Some(data_dir) => {
            let session_path = data_dir.join("iriebook-ui/session.json");
            Ok(session_path)
        }
        None => anyhow::bail!("Could not determine session directory location"),
    }
}

/// Try to load session file (can fail)
fn try_load_session_file(path: &Path) -> Result<SessionData> {
    let content = fs::read_to_string(path)?;
    let session: SessionData = serde_json::from_str(&content)?;
    Ok(session)
}

/// Load session from file
///
/// Returns:
/// - Ok(Some(session)) if file exists and is valid
/// - Ok(None) if file doesn't exist (fresh start)
/// - Ok(None) if file is corrupted (logs error, returns fresh start)
pub fn load_session() -> Result<Option<SessionData>> {
    let session_path = get_session_file_path()?;

    // No session file? Fresh start!
    match session_path.exists() {
        false => Ok(None),
        true => {
            // Try to read and parse session file
            match try_load_session_file(&session_path) {
                Ok(session_data) => {
                    // Validate the session (check folder exists, filter valid books)
                    match session_data.validate() {
                        Some(validated_session) => Ok(Some(validated_session)),
                        None => {
                            // Folder doesn't exist - clear session
                            debug!("Session folder no longer exists, starting fresh");
                            Ok(None)
                        }
                    }
                }
                Err(e) => {
                    // Corrupted session file - log error and start fresh
                    warn!(error = %e, "Failed to load session file, starting fresh");
                    Ok(None)
                }
            }
        }
    }
}

/// Save session to file
///
/// Creates parent directories if they don't exist.
/// Uses atomic write pattern (temp file + rename).
pub fn save_session(session: &SessionData) -> Result<()> {
    let session_path = get_session_file_path()?;

    // Create parent directories if they don't exist
    match session_path.parent() {
        Some(parent) => {
            fs::create_dir_all(parent)?;
        }
        None => anyhow::bail!("Session path has no parent directory"),
    }

    // Serialize to JSON with pretty printing
    let json_content = serde_json::to_string_pretty(session)?;

    // Atomic write: temp file + rename
    let temp_path = session_path.with_extension("tmp");
    fs::write(&temp_path, json_content)?;
    fs::rename(&temp_path, &session_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_data_new() {
        let folder = FolderPath::from("/path/to/books".to_string());
        let books = vec![BookPath::from(PathBuf::from("/path/to/book1.md"))];

        let session = SessionData::new(folder.clone(), books.clone(), true);

        assert_eq!(session.folder_path, folder);
        assert_eq!(session.selected_book_paths, books);
        assert!(session.current_book_mode);
    }

    #[test]
    fn test_session_validate_with_missing_folder() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        let session = SessionData::new(FolderPath::from(nonexistent), vec![], true);

        let validated = session.validate();
        assert!(validated.is_none());
    }

    #[test]
    fn test_session_validate_filters_missing_books() {
        let temp_dir = TempDir::new().unwrap();

        // Create one book file, reference two
        let book1 = temp_dir.path().join("book1.md");
        let book2 = temp_dir.path().join("book2.md");
        fs::write(&book1, "content").unwrap();
        // book2 not created - missing

        let session = SessionData::new(
            FolderPath::from(temp_dir.path().to_path_buf()),
            vec![
                BookPath::from(book1.clone()),
                BookPath::from(book2), // This one missing
            ],
            true,
        );

        let validated = session.validate().expect("Should validate");

        // Only book1 should remain
        assert_eq!(validated.selected_book_paths.len(), 1);
        assert_eq!(validated.selected_book_paths[0].as_path(), book1.as_path());
    }

    #[test]
    fn test_save_and_load_session_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let book_file = temp_dir.path().join("book.md");
        fs::write(&book_file, "content").unwrap();

        let original_session = SessionData::new(
            FolderPath::from(temp_dir.path().to_path_buf()),
            vec![BookPath::from(book_file)],
            true,
        );

        // Save
        let session_file = temp_dir.path().join("session.json");
        let json = serde_json::to_string_pretty(&original_session).unwrap();
        fs::create_dir_all(session_file.parent().unwrap()).unwrap();
        fs::write(&session_file, json).unwrap();

        // Load
        let loaded_json = fs::read_to_string(&session_file).unwrap();
        let loaded_session: SessionData = serde_json::from_str(&loaded_json).unwrap();

        assert_eq!(loaded_session, original_session);
    }

    #[test]
    fn test_corrupted_json_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let session_file = temp_dir.path().join("session.json");

        // Write malformed JSON
        fs::write(&session_file, "{ invalid json }").unwrap();

        // Should return error when parsing
        let result = try_load_session_file(&session_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let book_file = temp_dir.path().join("book.md");
        fs::write(&book_file, "content").unwrap();

        let original = SessionData::new(
            FolderPath::from(temp_dir.path().to_path_buf()),
            vec![BookPath::from(book_file)],
            true,
        );

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SessionData = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }
}
