//! Book collection management logic
//!
//! Framework-agnostic orchestration for managing book collections.
//! This module provides functions for adding, updating, and managing books
//! with automatic rescanning and index tracking.

use crate::book_scanner::scan_for_books;
use crate::ui_state::BookInfo;
use anyhow::{Context, Result};
use iriebook::resource_access::file;
use serde::Serialize;
use specta::Type;
use std::path::{Path, PathBuf};

// Serde helper module to serialize PathBuf as String
mod pathbuf_as_string {
    use serde::{Serializer, Serialize};

    pub fn serialize<S>(path: &std::path::Path, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        path.to_string_lossy().to_string().serialize(serializer)
    }
}

/// Result of adding a book to the workspace
#[derive(Debug, Clone, Serialize, Type)]
pub struct AddBookResult {
    #[serde(with = "pathbuf_as_string")]
    #[specta(type = String)]
    pub book_path: PathBuf,
    pub is_duplicate: bool,
    pub books: Vec<BookInfo>,
    #[specta(type = Option<u32>)]
    pub new_book_index: Option<usize>,
}

/// Result of changing a book's source file
#[derive(Debug, Clone, Serialize, Type)]
pub struct ChangeBookResult {
    #[serde(with = "pathbuf_as_string")]
    #[specta(type = String)]
    pub new_book_path: PathBuf,
    pub books: Vec<BookInfo>,
    #[specta(type = Option<u32>)]
    pub updated_book_index: Option<usize>,
}

/// Check if a book with the same name already exists in the workspace
///
/// Extracts folder name from md_filename and checks if workspace_root/<folder_name>/ exists
///
/// Returns: Some(folder_name) if exists, None if not
pub fn check_for_duplicate(workspace_root: &Path, md_filename: &str) -> Result<Option<String>> {
    // Extract folder name (filename without extension)
    let folder_name = md_filename
        .strip_suffix(".md")
        .or_else(|| md_filename.strip_suffix(".MD"))
        .unwrap_or(md_filename);

    if folder_name.is_empty() {
        anyhow::bail!("Filename cannot be empty");
    }

    let folder_path = workspace_root.join(folder_name);

    if folder_path.exists() {
        Ok(Some(folder_name.to_string()))
    } else {
        Ok(None)
    }
}

/// Add a book to the workspace with automatic rescanning and index tracking
///
/// This function:
/// 1. Calls `iriebook::resource_access::file::add_book_to_workspace()`
/// 2. Rescans the workspace to get updated book list
/// 3. Finds the newly added book in the list
/// 4. Returns comprehensive result for UI to consume
///
/// Framework-agnostic, testable without UI widgets
pub fn add_book_with_rescan(
    workspace_root: &Path,
    source_md: &Path,
) -> Result<AddBookResult> {
    // Add book using core file operations
    let (book_path, is_duplicate) = file::add_book_to_workspace(workspace_root, source_md)?;

    // Rescan workspace to get updated book list
    let books = scan_for_books(workspace_root)
        .with_context(|| format!("Failed to rescan workspace: {}", workspace_root.display()))?;

    // Find the newly added book in the list (by path matching)
    let new_book_index = books.iter().position(|book| {
        book.path.as_path() == book_path
    });

    Ok(AddBookResult {
        book_path,
        is_duplicate,
        books,
        new_book_index,
    })
}

/// Change a book's source file with automatic rescanning and index tracking
///
/// This function:
/// 1. Calls `iriebook::resource_access::file::change_book_file()`
/// 2. Rescans the workspace to get updated book list
/// 3. Finds the updated book in the list
/// 4. Returns result for UI to consume
pub fn change_book_with_rescan(
    book_path: &Path,
    new_source: &Path,
    workspace_root: &Path,
) -> Result<ChangeBookResult> {
    // Change book file using core file operations
    let new_book_path = file::change_book_file(book_path, new_source)?;

    // Rescan workspace to get updated book list
    let books = scan_for_books(workspace_root)
        .with_context(|| format!("Failed to rescan workspace: {}", workspace_root.display()))?;

    // Find the updated book in the list
    let updated_book_index = books.iter().position(|book| {
        book.path.as_path() == new_book_path
    });

    Ok(ChangeBookResult {
        new_book_path,
        books,
        updated_book_index,
    })
}

/// Delete a book from the workspace with automatic rescanning
///
/// This function:
/// 1. Validates that the book path is within the workspace root
/// 2. Calls `iriebook::resource_access::file::delete_book_folder()`
/// 3. Rescans the workspace to get updated book list
/// 4. Returns the updated book list
pub fn delete_book_with_rescan(
    book_path: &Path,
    workspace_root: &Path,
) -> Result<Vec<BookInfo>> {
    // 1. Safety check: book_path must be within workspace_root
    // We check if the parent of book_path (the book folder) is a child of workspace_root
    // e.g. workspace/book_folder/book.md -> parent is workspace/book_folder -> parent is workspace
    let book_folder = book_path.parent().ok_or_else(|| {
        anyhow::anyhow!("Book path has no parent: {}", book_path.display())
    })?;
    
    // Canonicalize paths for safer comparison if possible, but fallback to direct check
    // since temp paths might be complex.
    // For now, simple starts_with check on absolute paths or relative matching.
    
    // We assume both are absolute or relative in the same way.
    if !book_folder.starts_with(workspace_root) {
         anyhow::bail!(
            "Security Error: Cannot delete book outside workspace. Book: {}, Workspace: {}",
            book_folder.display(),
            workspace_root.display()
        );
    }
    
    // Double check that we are not deleting the workspace root itself
    if book_folder == workspace_root {
        anyhow::bail!("Security Error: Cannot delete workspace root");
    }

    // 2. Delete the book folder
    file::delete_book_folder(book_path)?;

    // 3. Rescan workspace
    let books = scan_for_books(workspace_root)
        .with_context(|| format!("Failed to rescan workspace: {}", workspace_root.display()))?;

    Ok(books)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_duplicate_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create a folder that would be created by a book named "test.md"
        fs::create_dir(workspace_root.join("test"))?;

        let result = check_for_duplicate(workspace_root, "test.md")?;
        assert_eq!(result, Some("test".to_string()));

        Ok(())
    }

    #[test]
    fn test_check_duplicate_not_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        let result = check_for_duplicate(workspace_root, "test.md")?;
        assert_eq!(result, None);

        Ok(())
    }

    #[test]
    fn test_add_book_with_rescan_new() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create a source .md file
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# My Book")?;

        // Add book with rescan
        let result = add_book_with_rescan(workspace_root, &source_md)?;

        // Should not be a duplicate
        assert!(!result.is_duplicate);

        // Book should exist
        assert!(result.book_path.exists());

        // Books list should not be empty
        assert!(!result.books.is_empty());

        // New book should be found
        assert!(result.new_book_index.is_some());

        Ok(())
    }

    #[test]
    fn test_add_book_with_rescan_duplicate() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add first book
        let source_md = temp_dir.path().join("story.md");
        fs::write(&source_md, "# Original")?;
        let first_result = add_book_with_rescan(workspace_root, &source_md)?;
        assert!(!first_result.is_duplicate);

        // Update source and add again
        fs::write(&source_md, "# Updated")?;
        let second_result = add_book_with_rescan(workspace_root, &source_md)?;

        // Should be detected as duplicate
        assert!(second_result.is_duplicate);

        // Book path should be same
        assert_eq!(second_result.book_path, first_result.book_path);

        // Books list should still have entries
        assert!(!second_result.books.is_empty());

        Ok(())
    }

    #[test]
    fn test_add_book_with_rescan_finds_book() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create source file
        let source_md = temp_dir.path().join("findme.md");
        fs::write(&source_md, "# Find Me")?;

        // Add book
        let result = add_book_with_rescan(workspace_root, &source_md)?;

        // New book index should be valid
        assert!(result.new_book_index.is_some());
        let index = result.new_book_index.unwrap();

        // Verify the book at that index is the one we added
        assert!(index < result.books.len());
        assert_eq!(result.books[index].path.as_path(), result.book_path);

        Ok(())
    }

    #[test]
    fn test_change_book_with_rescan() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add a book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original")?;
        let add_result = add_book_with_rescan(workspace_root, &source_md)?;

        // Create new source file
        let new_source = temp_dir.path().join("updated.md");
        fs::write(&new_source, "# Updated")?;

        // Change book file
        let change_result = change_book_with_rescan(
            &add_result.book_path,
            &new_source,
            workspace_root,
        )?;

        // Path should be same
        assert_eq!(change_result.new_book_path, add_result.book_path);

        // Books list should not be empty
        assert!(!change_result.books.is_empty());

        // Content should be updated
        let content = fs::read_to_string(&change_result.new_book_path)?;
        assert_eq!(content, "# Updated");

        Ok(())
    }

    #[test]
    fn test_change_book_with_rescan_finds_book() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add a book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original")?;
        let add_result = add_book_with_rescan(workspace_root, &source_md)?;

        // Change book file
        let new_source = temp_dir.path().join("updated.md");
        fs::write(&new_source, "# Updated")?;
        let change_result = change_book_with_rescan(
            &add_result.book_path,
            &new_source,
            workspace_root,
        )?;

        // Updated book index should be valid
        assert!(change_result.updated_book_index.is_some());
        let index = change_result.updated_book_index.unwrap();

        // Verify the book at that index is the one we changed
        assert!(index < change_result.books.len());
        assert_eq!(change_result.books[index].path.as_path(), change_result.new_book_path);

        Ok(())
    }

    #[test]
    fn test_delete_book_with_rescan() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add a book
        let source_md = temp_dir.path().join("todelete.md");
        fs::write(&source_md, "# To Delete")?;
        let add_result = add_book_with_rescan(workspace_root, &source_md)?;

        assert!(!add_result.books.is_empty());
        let book_path = add_result.book_path;

        // Delete the book
        let new_books = delete_book_with_rescan(&book_path, workspace_root)?;

        // List should be empty
        assert!(new_books.is_empty());

        // Folder should be gone
        assert!(!book_path.exists());
        assert!(!book_path.parent().unwrap().exists());

        Ok(())
    }
}
