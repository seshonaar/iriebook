//! Help-page orchestration
//!
//! Renders the bundled authoring guide to a self-contained HTML file on disk
//! and returns its path, ready to be opened in the user's default browser.
//! Pure logic, no UI — the Tauri/web/CLI layer calls this and opens the path.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use iriebook::engines::help::render_authoring_guide;

/// Filename used for the rendered guide inside the data directory.
const HELP_PAGE_FILENAME: &str = "authoring-guide.html";

/// Write the rendered authoring guide to `<data_dir>/authoring-guide.html`.
///
/// Creates `data_dir` if it does not exist. Overwrites on every call so the
/// guide always reflects the currently installed engine (important when the
/// app is upgraded — the bundled guide may have changed). Returns the path
/// that was written, for the caller to open.
pub fn write_help_page(data_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("Failed to create help-page directory {}", data_dir.display()))?;

    let html = render_authoring_guide();
    let path = data_dir.join(HELP_PAGE_FILENAME);
    std::fs::write(&path, html)
        .with_context(|| format!("Failed to write help page {}", path.display()))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_html_file_under_data_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_help_page(tmp.path()).unwrap();

        assert!(path.is_file(), "help page file should exist");
        assert_eq!(path.file_name().unwrap(), HELP_PAGE_FILENAME);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("<!DOCTYPE html>"));
        assert!(content.contains("IrieBook Authoring Guide"));
    }

    #[test]
    fn creates_data_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("deeply").join("nested").join("missing");
        assert!(!nested.exists());

        let path = write_help_page(&nested).unwrap();

        assert!(nested.exists(), "parent directories should be created");
        assert!(path.is_file());
    }

    #[test]
    fn is_idempotent_and_overwrites() {
        let tmp = tempfile::tempdir().unwrap();

        let first = write_help_page(tmp.path()).unwrap();
        let first_content = fs::read_to_string(&first).unwrap();
        let first_mtime = fs::metadata(&first).unwrap().modified().unwrap();

        // Tiny delay so the second write's mtime can differ if the filesystem tracks it.
        std::thread::sleep(std::time::Duration::from_millis(20));

        let second = write_help_page(tmp.path()).unwrap();
        let second_content = fs::read_to_string(&second).unwrap();
        let second_mtime = fs::metadata(&second).unwrap().modified().unwrap();

        assert_eq!(first, second, "same path both times");
        assert_eq!(first_content, second_content, "content stable across calls");
        assert!(
            second_mtime >= first_mtime,
            "second write should not predate the first"
        );
    }

    #[test]
    fn interpolated_tokens_are_resolved_in_written_file() {
        // The written file must carry the real magic numbers, not {{TOKEN}}.
        let tmp = tempfile::tempdir().unwrap();
        let path = write_help_page(tmp.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("{{CHUNKY_PARAGRAPH_THRESHOLD}}"));
        assert!(!content.contains("{{FUZZY_PREFIX_MAX_EDIT_DISTANCE}}"));
        // The threshold value must appear in the rendered scene-break section.
        assert!(content.contains(">10</strong>"), "threshold value 10 missing: {content}");
    }
}
