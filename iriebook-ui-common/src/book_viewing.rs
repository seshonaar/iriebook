//! Book viewing orchestration
//!
//! Handles the workflow for viewing books in ebook-viewer:
//! 1. Check if EPUB exists
//! 2. Generate EPUB if missing (requires metadata.yaml)
//! 3. Launch ebook-viewer

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use iriebook::managers::ebook_publication::EbookPublicationManager;
use iriebook::resource_access::file;
use iriebook::resource_access::traits::CalibreAccess;

/// View a book in ebook-viewer
///
/// This function orchestrates the complete workflow:
/// - Determines expected EPUB path from book metadata
/// - If EPUB doesn't exist, generates it using publication manager
/// - Launches ebook-viewer with the EPUB
///
/// # Arguments
/// * `book_path` - Path to the book's markdown file
/// * `publication_manager` - Manager for generating EPUB if needed
/// * `calibre_access` - Calibre resource access for launching viewer
///
/// # Returns
/// * `Ok(())` if viewer launched successfully
/// * `Err` if metadata missing, EPUB generation fails, or viewer launch fails
pub fn view_book(
    book_path: &Path,
    publication_manager: &EbookPublicationManager,
    calibre_access: &Arc<dyn CalibreAccess>,
) -> Result<()> {
    // Step 1: Determine EPUB path from metadata
    let epub_path = file::get_output_file_name(book_path)
        .context("Failed to get EPUB path. Ensure metadata.yaml exists with title and author fields.")?;

    // Step 2: Check if EPUB exists, generate if needed
    if !epub_path.exists() {
        // Generate EPUB using publication pipeline
        // Parameters: input_path, output_path=None (auto),
        //             enable_word_stats=false, enable_publishing=true
        publication_manager
            .publish(book_path, None, false, true)
            .context("Failed to generate EPUB for viewing")?;

        // Verify EPUB was created
        if !epub_path.exists() {
            anyhow::bail!("EPUB generation completed but file not found at: {}", epub_path.display());
        }
    }

    // Step 3: Launch ebook-viewer
    calibre_access
        .view_ebook(&epub_path)
        .context("Failed to launch ebook-viewer")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_view_book_requires_metadata() {
        // Verify that get_output_file_name fails without metadata
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();

        let result = file::get_output_file_name(&book_path);
        assert!(result.is_err(), "Expected error when metadata.yaml is missing");
    }
}
