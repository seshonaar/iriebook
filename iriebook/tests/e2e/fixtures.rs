//! Test fixtures for E2E testing
//!
//! Provides utilities for creating test workspaces with sample books, covers, and metadata.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A test workspace with sample book data
pub struct TestWorkspace {
    /// Temporary directory (dropped when TestWorkspace is dropped)
    pub temp_dir: TempDir,
    /// Path to the workspace root
    pub workspace_path: PathBuf,
    /// Books created in this workspace
    pub books: Vec<TestBook>,
}

/// A sample book created for testing
#[derive(Clone)]
pub struct TestBook {
    /// Book name (directory name)
    pub name: String,
    /// Path to the book directory
    pub path: PathBuf,
    /// Path to the book.md file
    pub md_path: PathBuf,
    /// Content of the book
    pub content: String,
    /// Path to cover image (if any)
    pub cover_path: Option<PathBuf>,
}

impl TestWorkspace {
    /// Create a new empty test workspace
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_path = temp_dir.path().to_path_buf();

        Ok(Self {
            temp_dir,
            workspace_path,
            books: vec![],
        })
    }

    /// Add a sample book to the workspace
    pub fn add_book(&mut self, name: &str) -> anyhow::Result<&TestBook> {
        let book_dir = self.workspace_path.join(name);
        fs::create_dir_all(&book_dir)?;

        // Create book.md with sample content
        let content = format!(
            r#"# {name}

## Chapter 1: The Beginning

"Hello," she said, looking up from her book. "I didn't expect to see you here."

He smiled warmly. "Life is full of surprises, isn't it?"

The rain continued to fall outside, drumming against the windows like a thousand tiny fingers.

## Chapter 2: The Journey

They set out at dawn, the morning mist curling around their ankles like friendly cats.

"Do you think we'll make it?" she asked.

"We have to try," he replied. "That's all anyone can do."

## Chapter 3: The End

And so their adventure came to a close, but the memories would last forever.

*The End*
"#
        );
        let md_path = book_dir.join("book.md");
        fs::write(&md_path, &content)?;

        // Create metadata.yaml
        let metadata = format!(
            r#"title: "{name}"
author: "Test Author"
language: en
description: "A sample book for testing"
"#
        );
        fs::write(book_dir.join("metadata.yaml"), metadata)?;

        // Create a minimal cover image (1x1 pixel JPEG)
        let cover_path = book_dir.join("cover.jpg");
        create_minimal_jpeg(&cover_path)?;

        self.books.push(TestBook {
            name: name.to_string(),
            path: book_dir.clone(),
            md_path: md_path.clone(),
            content,
            cover_path: Some(cover_path),
        });

        Ok(self.books.last().unwrap())
    }

    /// Add a book with custom content
    pub fn add_book_with_content(
        &mut self,
        name: &str,
        content: &str,
    ) -> anyhow::Result<&TestBook> {
        let book_dir = self.workspace_path.join(name);
        fs::create_dir_all(&book_dir)?;

        let md_path = book_dir.join("book.md");
        fs::write(&md_path, content)?;

        // Create minimal metadata
        let metadata = format!(
            r#"title: "{name}"
author: "Test Author"
language: en
"#
        );
        fs::write(book_dir.join("metadata.yaml"), metadata)?;

        self.books.push(TestBook {
            name: name.to_string(),
            path: book_dir,
            md_path: md_path.clone(),
            content: content.to_string(),
            cover_path: None,
        });

        Ok(self.books.last().unwrap())
    }

    /// Get a book by name
    pub fn get_book(&self, name: &str) -> Option<&TestBook> {
        self.books.iter().find(|b| b.name == name)
    }

    /// Create .irie output folder for a book
    pub fn create_irie_folder(&self, book_name: &str) -> anyhow::Result<PathBuf> {
        let book = self
            .get_book(book_name)
            .ok_or_else(|| anyhow::anyhow!("Book not found: {}", book_name))?;
        let irie_folder = book.path.join(".irie");
        fs::create_dir_all(&irie_folder)?;
        Ok(irie_folder)
    }

    /// Simulate that a book has been linked to Google Docs
    pub fn link_book_to_google_doc(&self, book_name: &str, doc_id: &str) -> anyhow::Result<()> {
        let book = self
            .get_book(book_name)
            .ok_or_else(|| anyhow::anyhow!("Book not found: {}", book_name))?;
        let irie_folder = book.path.join(".irie");
        fs::create_dir_all(&irie_folder)?;

        let link_file = irie_folder.join("google_doc_link.json");
        let link_content = format!(r#"{{"doc_id": "{}"}}"#, doc_id);
        fs::write(link_file, link_content)?;

        Ok(())
    }
}

/// Create a minimal valid JPEG file (smallest possible)
fn create_minimal_jpeg(path: &Path) -> anyhow::Result<()> {
    // Minimal 1x1 red JPEG (smallest valid JPEG)
    let minimal_jpeg: [u8; 135] = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06,
        0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B,
        0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
        0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31,
        0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF,
        0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00,
        0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];

    fs::write(path, &minimal_jpeg)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_workspace() {
        let workspace = TestWorkspace::new().unwrap();
        assert!(workspace.workspace_path.exists());
    }

    #[test]
    fn test_add_book() {
        let mut workspace = TestWorkspace::new().unwrap();
        workspace.add_book("my-novel").unwrap();

        assert_eq!(workspace.books.len(), 1);
        let book = workspace.get_book("my-novel").unwrap();
        assert!(book.md_path.exists());
        assert!(book.cover_path.as_ref().unwrap().exists());
    }

    #[test]
    fn test_add_book_with_custom_content() {
        let mut workspace = TestWorkspace::new().unwrap();
        workspace
            .add_book_with_content("custom", "# Custom\n\nContent")
            .unwrap();

        let book = workspace.get_book("custom").unwrap();
        assert!(book.content.contains("Custom"));
    }

    #[test]
    fn test_link_book_to_google_doc() {
        let mut workspace = TestWorkspace::new().unwrap();
        workspace.add_book("my-book").unwrap();
        workspace
            .link_book_to_google_doc("my-book", "google-doc-id-123")
            .unwrap();

        let book = workspace.get_book("my-book").unwrap();
        let link_file = book.path.join(".irie/google_doc_link.json");
        assert!(link_file.exists());

        let content = fs::read_to_string(link_file).unwrap();
        assert!(content.contains("google-doc-id-123"));
    }
}
