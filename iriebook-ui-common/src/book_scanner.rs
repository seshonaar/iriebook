use crate::ui_state::{BookInfo, BookPath};
use anyhow::Result;
use iriebook::resource_access::file::OUTPUT_DIR_NAME;
use iriebook::resource_access::git::GitClient;
use iriebook::resource_access::traits::GitAccess;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::warn;
use walkdir::WalkDir;

/// Get all folders that have git changes with their changed files
/// Returns a map from folder path to list of changed file names (relative to folder)
fn get_changed_folders(repo_path: &Path, git_client: &GitClient) -> HashMap<PathBuf, Vec<String>> {
    let mut changed_folders: HashMap<PathBuf, Vec<String>> = HashMap::new();

    // Get all changed files at once
    if let Ok(changed_files) = git_client.get_all_changed_files(repo_path) {
        // Group changed files by their parent folder
        for file_path in changed_files {
            if let Some(parent) = file_path.parent()
                && let Some(file_name) = file_path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_string();
                    changed_folders
                        .entry(parent.to_path_buf())
                        .or_default()
                        .push(file_name_str);
                }
        }
    }

    changed_folders
}

/// Scan a directory for book files (.md files in subfolders)
///
/// Excludes:
/// - Files directly in the root directory
/// - Files named `summary.md` (case-insensitive)
/// - Files containing `fixed.md` in the name
///
/// Returns a sorted list of BookInfo by display name
pub fn scan_for_books(root_dir: &Path) -> Result<Vec<BookInfo>> {
    let mut books = Vec::new();

    // Check if workspace is a git repository and get changed folders once
    let git_client = GitClient;
    let changed_folders = if git_client.is_repository(root_dir) {
        get_changed_folders(root_dir, &git_client)
    } else {
        HashMap::new()
    };

    let walker = WalkDir::new(root_dir)
        .min_depth(2) // Skip files in root, only look in subfolders
        .max_depth(10) // Reasonable depth limit
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            // Skip hidden directories
            !entry.file_name().to_str().is_some_and(|s| s.starts_with('.'))
        });

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                // Log and skip entries we can't read
                warn!(error = %err, "Failed to read entry during book scan");
                continue;
            }
        };

        // Only process files
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Skip files in hidden directories (check relative path components only)
        let is_in_hidden_dir = match path.strip_prefix(root_dir) {
            Ok(relative) => relative.components().any(|comp| {
                comp.as_os_str()
                    .to_str()
                    .is_some_and(|s| s.starts_with('.'))
            }),
            Err(_) => false, // If we can't get relative path, don't skip
        };

        if is_in_hidden_dir {
            continue;
        }

        // Skip files in irie/ workspace directories
        let is_in_irie_dir = match path.strip_prefix(root_dir) {
            Ok(relative) => relative.components().any(|comp| {
                comp.as_os_str().to_str() == Some("irie")
            }),
            Err(_) => false,
        };

        if is_in_irie_dir {
            continue;
        }

        // Skip files in output directories
        let is_in_output_dir = match path.strip_prefix(root_dir) {
            Ok(relative) => relative.components().any(|comp| {
                comp.as_os_str().to_str() == Some(OUTPUT_DIR_NAME)
            }),
            Err(_) => false,
        };

        if is_in_output_dir {
            continue;
        }

        // Check if it's a markdown file
        match path.extension() {
            Some(ext) if ext == "md" => {}
            _ => continue,
        }

        // Get the filename for exclusion checks
        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(name) => name.to_lowercase(),
            None => continue,
        };

        // Exclude summary.md
        if filename == "summary.md" {
            continue;
        }

        // Exclude files with "fixed" in the name
        if filename.contains("fixed") {
            continue;
        }

        // Create display name from the file name
        let display_name = path.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();

        let book_path = BookPath::new(path.to_path_buf());
        let cover_image = find_cover_image(path);
        let metadata = iriebook::resource_access::file::load_metadata(path).ok().flatten();
        let google_docs_sync_info = iriebook::resource_access::file::load_google_docs_sync_info(path).ok().flatten();

        // Get changed files for this book's folder (efficient lookup in pre-computed map)
        let git_changed_files = if let Some(book_folder) = path.parent() {
            changed_folders.get(book_folder)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        books.push(
            BookInfo::new(book_path, display_name)
                .with_cover_image(cover_image)
                .with_metadata(metadata)
                .with_google_docs_sync_info(google_docs_sync_info)
                .with_git_changed_files(git_changed_files),
        );
    }

    // Sort by display name for consistent ordering
    books.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    Ok(books)
}

/// Find cover image in a book's root directory
///
/// Looks for cover.jpg in the book's root folder
fn find_cover_image(book_path: &Path) -> Option<std::path::PathBuf> {
    let book_dir = book_path.parent()?;
    let cover_path = book_dir.join("cover.jpg");

    if cover_path.exists() && cover_path.is_file() {
        Some(cover_path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_structure() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create test structure
        // root/
        //   book1/
        //     chapter1.md  ✓
        //     chapter2.md  ✓
        //   book2/
        //     intro.md     ✓
        //   should-ignore.md  ✗ (in root)
        //   book3/
        //     summary.md   ✗ (excluded)
        //     chapter-fixed.md  ✗ (contains "fixed")
        //     valid.md     ✓
        //   .hidden/
        //     secret.md    ✗ (hidden dir)

        fs::create_dir(root.join("book1"))?;
        fs::write(root.join("book1/chapter1.md"), "# Chapter 1")?;
        fs::write(root.join("book1/chapter2.md"), "# Chapter 2")?;

        fs::create_dir(root.join("book2"))?;
        fs::write(root.join("book2/intro.md"), "# Intro")?;

        fs::write(root.join("should-ignore.md"), "# Root file")?;

        fs::create_dir(root.join("book3"))?;
        fs::write(root.join("book3/summary.md"), "# Summary")?;
        fs::write(root.join("book3/chapter-fixed.md"), "# Fixed")?;
        fs::write(root.join("book3/valid.md"), "# Valid")?;

        fs::create_dir(root.join(".hidden"))?;
        fs::write(root.join(".hidden/secret.md"), "# Secret")?;

        Ok(temp_dir)
    }

    #[test]
    fn test_scan_for_books_finds_correct_files() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // Should find exactly 4 books: chapter1.md, chapter2.md, intro.md, valid.md
        assert_eq!(books.len(), 4);

        // Check the display names are sorted
        assert_eq!(books[0].display_name, "chapter1.md");
        assert_eq!(books[1].display_name, "chapter2.md");
        assert_eq!(books[2].display_name, "intro.md");
        assert_eq!(books[3].display_name, "valid.md");

        Ok(())
    }

    #[test]
    fn test_scan_for_books_excludes_root_files() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // Verify no file from root is included
        for book in &books {
            assert!(!book.display_name.eq("should-ignore.md"));
        }

        Ok(())
    }

    #[test]
    fn test_scan_for_books_excludes_summary() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // Verify summary.md is excluded
        for book in &books {
            assert!(!book.display_name.contains("summary.md"));
        }

        Ok(())
    }

    #[test]
    fn test_scan_for_books_excludes_fixed() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // Verify files with "fixed" are excluded
        for book in &books {
            assert!(!book.display_name.contains("fixed"));
        }

        Ok(())
    }

    #[test]
    fn test_scan_for_books_excludes_hidden_directories() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // Verify files in hidden directories are excluded
        for book in &books {
            assert!(!book.display_name.contains(".hidden"));
        }

        Ok(())
    }

    #[test]
    fn test_scan_for_books_empty_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let books = scan_for_books(temp_dir.path())?;
        assert!(books.is_empty());
        Ok(())
    }

    #[test]
    fn test_scan_for_books_no_markdown_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        fs::create_dir(root.join("subdir"))?;
        fs::write(root.join("subdir/file.txt"), "Not markdown")?;
        fs::write(root.join("subdir/file.rs"), "Rust code")?;

        let books = scan_for_books(root)?;
        assert!(books.is_empty());
        Ok(())
    }

    #[test]
    fn test_scan_for_books_all_selected_false() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // All books should start with selected = false
        for book in &books {
            assert!(!book.selected);
        }

        Ok(())
    }

    #[test]
    fn test_scan_for_books_nonexistent_directory() {
        let result = scan_for_books(Path::new("/nonexistent/path/that/does/not/exist"));
        // Should either return empty or error, but not panic
        match result {
            Ok(books) => assert!(books.is_empty()),
            Err(_) => {} // Error is acceptable
        }
    }

    #[test]
    fn test_find_cover_image_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        let book_file = book_dir.join("chapter1.md");
        fs::write(&book_file, "# Chapter 1")?;

        // Cover is now in root folder
        let cover_file = book_dir.join("cover.jpg");
        fs::write(&cover_file, "fake image data")?;

        let result = find_cover_image(&book_file);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), cover_file);
        Ok(())
    }

    #[test]
    fn test_find_cover_image_not_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        let book_file = book_dir.join("chapter1.md");
        fs::write(&book_file, "# Chapter 1")?;

        let result = find_cover_image(&book_file);
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_find_cover_image_in_root_folder() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        let book_file = book_dir.join("chapter1.md");
        fs::write(&book_file, "# Chapter 1")?;

        // Cover is now in root folder
        let cover_file = book_dir.join("cover.jpg");
        fs::write(&cover_file, "fake image data")?;

        let result = find_cover_image(&book_file);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), cover_file);
        Ok(())
    }

    #[test]
    fn test_scan_excludes_irie_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create a book with irie/ workspace
        fs::create_dir(root.join("book1"))?;
        fs::write(root.join("book1/chapter.md"), "# Chapter")?;

        // Create irie/ folder with workspace files
        fs::create_dir(root.join("book1/irie"))?;
        fs::write(root.join("book1/irie/fixed.md"), "# Fixed")?;
        fs::write(root.join("book1/irie/summary.md"), "# Summary")?;
        fs::write(root.join("book1/irie/metadata.yaml"), "title: Test")?;

        let books = scan_for_books(root)?;

        // Should only find chapter.md, not files in irie/
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].display_name, "chapter.md");

        Ok(())
    }

    #[test]
    fn test_scan_for_books_includes_git_status_in_git_repo() -> Result<()> {
        use iriebook::resource_access::git::GitClient;
        use iriebook::resource_access::traits::GitAccess;

        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Initialize git repo
        std::process::Command::new("git")
            .args(&["init"])
            .current_dir(root)
            .output()?;
        std::process::Command::new("git")
            .args(&["config", "user.name", "Test"])
            .current_dir(root)
            .output()?;
        std::process::Command::new("git")
            .args(&["config", "user.email", "test@test.com"])
            .current_dir(root)
            .output()?;

        // Create two books
        fs::create_dir(root.join("book1"))?;
        fs::create_dir(root.join("book2"))?;
        fs::write(root.join("book1/chapter.md"), "content")?;
        fs::write(root.join("book2/chapter.md"), "content")?;

        // Commit both books
        let git_client = GitClient;
        git_client.add_all(root)?;
        git_client.commit(root, "initial")?;

        // Modify book1, leave book2 clean
        fs::write(root.join("book1/chapter.md"), "modified")?;

        let books = scan_for_books(root)?;

        // Find the books
        let book1 = books.iter().find(|b| b.path.as_path().ends_with("book1/chapter.md")).unwrap();
        let book2 = books.iter().find(|b| b.path.as_path().ends_with("book2/chapter.md")).unwrap();

        assert!(book1.has_git_changes());
        assert!(book1.git_changed_files.contains(&"chapter.md".to_string()));
        assert!(!book2.has_git_changes());
        assert!(book2.git_changed_files.is_empty());

        Ok(())
    }

    #[test]
    fn test_scan_for_books_no_git_status_in_non_repo() -> Result<()> {
        let temp_dir = create_test_structure()?;
        let books = scan_for_books(temp_dir.path())?;

        // All books should have no git changes in non-git directory
        for book in &books {
            assert!(!book.has_git_changes());
            assert!(book.git_changed_files.is_empty());
        }

        Ok(())
    }

    #[test]
    fn test_scan_for_books_includes_non_markdown_git_changes_for_status() -> Result<()> {
        use iriebook::resource_access::git::GitClient;
        use iriebook::resource_access::traits::GitAccess;

        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()?;
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .output()?;
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(root)
            .output()?;

        // Create book with manuscript + cover image, then commit
        fs::create_dir(root.join("book1"))?;
        fs::write(root.join("book1/chapter.md"), "content")?;
        fs::write(root.join("book1/cover.jpg"), "cover-v1")?;

        let git_client = GitClient;
        git_client.add_all(root)?;
        git_client.commit(root, "initial")?;

        // Modify only the cover image
        fs::write(root.join("book1/cover.jpg"), "cover-v2")?;

        let books = scan_for_books(root)?;
        assert_eq!(books.len(), 1);
        assert!(books[0].has_git_changes());
        assert!(books[0].git_changed_files.contains(&"cover.jpg".to_string()));

        Ok(())
    }
}
