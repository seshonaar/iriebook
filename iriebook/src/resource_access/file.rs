//! File I/O operations
//!
//! Handles reading input files and writing output files safely

use crate::utilities::types::{BookMetadata, GoogleDocsSyncInfo};
use anyhow::{Context, Result};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Cached analysis data stored in irie/analysis.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysis {
    /// Cache format version for future compatibility
    pub version: u32,
    /// Unix timestamp when the book was last modified
    pub book_modified_timestamp: u64,
    /// Unix timestamp when analysis was performed
    pub analysis_timestamp: u64,
    /// The actual analysis statistics
    pub stats: CachedAnalysisStats,
}

/// Statistics stored in the analysis cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysisStats {
    /// Total words in document
    pub total_words: usize,
    /// Unique words after filtering
    pub unique_words: usize,
    /// Number of stopwords excluded
    pub excluded_count: usize,
    /// Top words with their counts (up to 100)
    pub top_words: Vec<(String, usize)>,
}

/// Embedded CSS content for EPUB generation
/// This is embedded at compile time from assets/default.css
const EMBEDDED_CSS: &str = include_str!("../../assets/default.css");

/// Name of the output directory for generated ebooks (epub, azw3)
///
/// To rename this folder: change this constant and update .gitignore
pub const OUTPUT_DIR_NAME: &str = "yard";

/// Read a file as UTF-8 text
///
/// Strips UTF-8 BOM (Byte Order Mark) if present.
/// Google Docs and Windows editors sometimes add BOM to UTF-8 files,
/// but it's not needed and can cause issues with parsers.
pub fn read_file(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read file: {}. Check path and permissions, mon!",
            path.display()
        )
    })?;

    // Strip UTF-8 BOM if present (defensive programming)
    // BOM is U+FEFF (Zero Width No-Break Space)
    const BOM: char = '\u{FEFF}';
    Ok(content.strip_prefix(BOM).unwrap_or(&content).to_string())
}

/// Generate output path in the irie/ workspace subfolder
///
/// Examples:
/// - "book.md" -> "irie/fixed.md"
/// - "/path/to/story.md" -> "/path/to/irie/fixed.md"
pub fn generate_output_path(input: &Path) -> Result<PathBuf, anyhow::Error> {
    get_irie_folder_file(input, "fixed.md")
}

/// Generate summary output path in the irie/ workspace subfolder
///
/// Examples:
/// - "book.md" -> "irie/summary.md"
/// - "/path/to/story.md" -> "/path/to/irie/summary.md"
pub fn generate_summary_output_path(input: &Path) -> Result<PathBuf, anyhow::Error> {
    get_irie_folder_file(input, "summary.md")
}

/// Get path to CSS file for Pandoc EPUB generation
///
/// The CSS is embedded in the binary and written to a temporary file
/// when needed. This ensures the CSS is always available, even when
/// the binary is bundled (e.g., in AppImage format).
pub fn get_css_path() -> Result<String, anyhow::Error> {
    use std::io::Write;

    // Create temp file path in system temp directory
    let temp_dir = std::env::temp_dir();
    let css_path = temp_dir.join("iriebook-default.css");

    // Check if file exists and has correct content (idempotent)
    if css_path.exists() {
        let existing_content = fs::read_to_string(&css_path)
            .with_context(|| format!("Failed to read existing CSS file: {}", css_path.display()))?;

        if existing_content == EMBEDDED_CSS {
            return Ok(css_path.display().to_string());
        }
    }

    // Write embedded CSS to temp file (atomic write pattern)
    let temp_path = temp_dir.join("iriebook-default.css.tmp");

    let mut file = fs::File::create(&temp_path).with_context(|| {
        format!(
            "Failed to create temporary CSS file: {}",
            temp_path.display()
        )
    })?;

    file.write_all(EMBEDDED_CSS.as_bytes())
        .with_context(|| "Failed to write CSS content to temporary file")?;

    file.sync_all()
        .with_context(|| "Failed to sync CSS file to disk")?;

    drop(file);

    // Atomic rename to final location
    fs::rename(&temp_path, &css_path).with_context(|| {
        format!(
            "Failed to move temporary CSS file to {}",
            css_path.display()
        )
    })?;

    Ok(css_path.display().to_string())
}

pub fn change_extension(input_path: &Path, extension: &str) -> String {
    input_path.with_extension(extension).display().to_string()
}

pub fn get_output_file_name(input_path: &Path) -> Result<PathBuf, anyhow::Error> {
    let metadata_path = get_book_folder_file(input_path, "metadata.yaml")?;

    // Read and parse the metadata file
    let metadata_content = read_file(&metadata_path).context("Failed to read metadata.yaml")?;

    // Strip YAML frontmatter delimiters (---) if present
    let yaml_content = metadata_content
        .lines()
        .filter(|line| *line != "---")
        .collect::<Vec<_>>()
        .join("\n");

    let metadata: BookMetadata =
        serde_yaml::from_str(&yaml_content).context("Failed to parse metadata.yaml")?;

    // Build the filename based on metadata
    let author = &metadata.author;
    let title = &metadata.title;

    let filename = match (&metadata.belongs_to_collection, metadata.group_position) {
        (Some(series), Some(position)) => {
            format!(
                "{}-{}-{}-{}.epub",
                slugify(author),
                slugify(title),
                get_short_series_name_by_length(series),
                position
            )
        }
        _ => {
            format!("{}-{}.epub", slugify(author), slugify(title))
        }
    };

    let output_path = get_output_folder_file(input_path, &filename)?;
    Ok(output_path)
}

pub fn get_book_folder_file(input_path: &Path, file: &str) -> Result<PathBuf, anyhow::Error> {
    let parent = input_path.parent();

    match parent {
        Some(dir) => Ok(dir.join(file)),
        None => Ok(PathBuf::from(file)),
    }
}

/// Get a file path in the book's irie/ workspace subfolder
///
/// Creates the irie/ directory if it doesn't exist.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use iriebook::resource_access::file::get_irie_folder_file;
/// let input = Path::new("/books/mybook.md");
/// let metadata_path = get_irie_folder_file(input, "metadata.yaml")?;
/// // Returns: /books/irie/metadata.yaml (and creates /books/irie/ if needed)
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_irie_folder_file(input_path: &Path, file: &str) -> Result<PathBuf, anyhow::Error> {
    let parent = input_path.parent();

    match parent {
        Some(dir) => {
            let irie_dir = dir.join("irie");

            // Create irie/ directory if it doesn't exist
            if !irie_dir.exists() {
                fs::create_dir(&irie_dir).with_context(|| {
                    format!("Failed to create irie directory: {}", irie_dir.display())
                })?;
            }

            Ok(irie_dir.join(file))
        }
        None => Ok(PathBuf::from("irie").join(file)),
    }
}

/// Get a file path in the book's output subfolder (configured by [`OUTPUT_DIR_NAME`])
///
/// Creates the output directory if it doesn't exist.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use iriebook::resource_access::file::get_output_folder_file;
/// let input = Path::new("/books/mybook.md");
/// let epub_path = get_output_folder_file(input, "output.epub")?;
/// // Returns: /books/yard/output.epub (and creates /books/yard/ if needed)
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_output_folder_file(input_path: &Path, file: &str) -> Result<PathBuf, anyhow::Error> {
    let parent = input_path.parent();

    match parent {
        Some(dir) => {
            let output_dir = dir.join(OUTPUT_DIR_NAME);

            // Create output directory if it doesn't exist
            if !output_dir.exists() {
                fs::create_dir(&output_dir).with_context(|| {
                    format!(
                        "Failed to create {} directory: {}",
                        OUTPUT_DIR_NAME,
                        output_dir.display()
                    )
                })?;
            }

            Ok(output_dir.join(file))
        }
        None => Ok(PathBuf::from(OUTPUT_DIR_NAME).join(file)),
    }
}

pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'ă' | 'â' => 'a',
            'î' => 'i',
            'ș' => 's',
            'ț' => 't',
            _ if c.is_alphanumeric() => c,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn get_short_series_name_by_length(full_name: &str) -> String {
    full_name
        .split_whitespace()
        // Filter out words like 'din', 'de', 'ai' (length < 4)
        .filter(|word| {
            // We use .chars().count() to handle Romanian diacritics correctly
            word.chars().count() >= 4
        })
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_lowercase()
}

/// Write content to a file atomically
///
/// Uses a temporary file and then renames it to avoid partial writes
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    // Write to temporary file first
    let temp_path = path.with_extension("tmp");

    fs::write(&temp_path, content)
        .with_context(|| format!("Failed to write temporary file: {}", temp_path.display()))?;

    // Atomic rename
    fs::rename(&temp_path, path)
        .with_context(|| format!("Failed to move file to final location: {}", path.display()))?;

    Ok(())
}

/// Load metadata from metadata.yaml in book's root directory
///
/// Returns None if file doesn't exist (not an error)
pub fn load_metadata(book_path: &Path) -> Result<Option<BookMetadata>> {
    let metadata_path = get_book_folder_file(book_path, "metadata.yaml")?;

    match metadata_path.exists() {
        false => Ok(None),
        true => {
            let content = read_file(&metadata_path).with_context(|| {
                format!("Failed to read metadata file: {}", metadata_path.display())
            })?;

            // Strip YAML frontmatter delimiters (---) if present
            let yaml_content = content
                .lines()
                .filter(|line| *line != "---")
                .collect::<Vec<_>>()
                .join("\n");

            let metadata: BookMetadata = serde_yaml::from_str(&yaml_content)
                .with_context(|| "Failed to parse metadata YAML")?;

            Ok(Some(metadata))
        }
    }
}

/// Save metadata to metadata.yaml in book's root directory
///
/// Validates before saving, uses atomic write pattern
pub fn save_metadata(book_path: &Path, metadata: &BookMetadata) -> Result<()> {
    // Validate first
    metadata
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    let metadata_path = get_book_folder_file(book_path, "metadata.yaml")?;

    // Ensure predefined defaults are set
    let metadata_with_defaults = metadata.clone().with_predefined_defaults();

    // Serialize with frontmatter delimiters
    let yaml_content = serde_yaml::to_string(&metadata_with_defaults)
        .context("Failed to serialize metadata to YAML")?;
    let full_content = format!("---\n{}---\n", yaml_content);

    write_file(&metadata_path, &full_content)
}

/// Load Google Docs sync info from google-docs-sync.yaml in book's root directory
///
/// Returns None if file doesn't exist (not an error)
pub fn load_google_docs_sync_info(book_path: &Path) -> Result<Option<GoogleDocsSyncInfo>> {
    let sync_info_path = get_book_folder_file(book_path, "google-docs-sync.yaml")?;

    match sync_info_path.exists() {
        false => Ok(None),
        true => {
            let content = read_file(&sync_info_path).with_context(|| {
                format!(
                    "Failed to read sync info file: {}",
                    sync_info_path.display()
                )
            })?;

            // Strip YAML frontmatter delimiters (---) if present
            let yaml_content = content
                .lines()
                .filter(|line| *line != "---")
                .collect::<Vec<_>>()
                .join("\n");

            let sync_info: GoogleDocsSyncInfo = serde_yaml::from_str(&yaml_content)
                .with_context(|| "Failed to parse Google Docs sync info YAML")?;

            Ok(Some(sync_info))
        }
    }
}

/// Save Google Docs sync info to google-docs-sync.yaml in book's root directory
///
/// Uses atomic write pattern
pub fn save_google_docs_sync_info(book_path: &Path, sync_info: &GoogleDocsSyncInfo) -> Result<()> {
    let sync_info_path = get_book_folder_file(book_path, "google-docs-sync.yaml")?;

    // Serialize with frontmatter delimiters
    let yaml_content = serde_yaml::to_string(sync_info)
        .context("Failed to serialize Google Docs sync info to YAML")?;
    let full_content = format!("---\n{}---\n", yaml_content);

    write_file(&sync_info_path, &full_content)
}

/// Delete Google Docs sync info file from book's root directory
///
/// Returns Ok(()) even if file doesn't exist
pub fn delete_google_docs_sync_info(book_path: &Path) -> Result<()> {
    let sync_info_path = get_book_folder_file(book_path, "google-docs-sync.yaml")?;

    if sync_info_path.exists() {
        std::fs::remove_file(&sync_info_path).with_context(|| {
            format!(
                "Failed to delete sync info file: {}",
                sync_info_path.display()
            )
        })?;
    }

    Ok(())
}

/// Load cached analysis from irie/analysis.json
///
/// Returns None if the cache file doesn't exist (not an error)
pub fn load_analysis_cache(book_path: &Path) -> Result<Option<CachedAnalysis>> {
    let cache_path = get_irie_folder_file(book_path, "analysis.json")?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let content = read_file(&cache_path)
        .with_context(|| format!("Failed to read analysis cache: {}", cache_path.display()))?;

    let cache: CachedAnalysis =
        serde_json::from_str(&content).with_context(|| "Failed to parse analysis cache JSON")?;

    Ok(Some(cache))
}

/// Save analysis cache to irie/analysis.json using atomic write pattern
pub fn save_analysis_cache(book_path: &Path, cache: &CachedAnalysis) -> Result<()> {
    let cache_path = get_irie_folder_file(book_path, "analysis.json")?;

    let json_content = serde_json::to_string_pretty(cache)
        .context("Failed to serialize analysis cache to JSON")?;

    write_file(&cache_path, &json_content)
}

/// Get file modification timestamp as Unix seconds
///
/// Returns the time of last modification since Unix epoch
pub fn get_file_modified_timestamp(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

    let modified = metadata
        .modified()
        .with_context(|| format!("Failed to get modification time for: {}", path.display()))?;

    let duration = modified
        .duration_since(UNIX_EPOCH)
        .with_context(|| "System time before Unix epoch")?;

    Ok(duration.as_secs())
}

/// Copy an image file to a book's root folder as cover.jpg
///
/// Validates the source image, backs up existing cover if present,
/// converts the image to JPEG format, and atomically writes to cover.jpg.
///
/// **Format Conversion**: The source image can be in any format supported
/// by the `image` crate (PNG, JPEG, WEBP, BMP, etc.) and will be automatically
/// converted to JPEG.
///
/// # Arguments
/// * `book_path` - Path to the book's .md file
/// * `source_image` - Path to the image file to use as cover (any format)
///
/// # Returns
/// Path to the new cover.jpg file
///
/// # Errors
/// - Source image doesn't exist or isn't readable
/// - Source file is not a valid image format
/// - Insufficient permissions to write to book folder
pub fn replace_cover_image(book_path: &Path, source_image: &Path) -> Result<PathBuf> {
    // 1. Validate source exists
    if !source_image.exists() {
        anyhow::bail!("Source image does not exist: {}", source_image.display());
    }

    // 2. Validate source is a valid image and keep decoded result (optimization)
    let img = image::ImageReader::open(source_image)
        .with_context(|| format!("Failed to open source image: {}", source_image.display()))?
        .decode()
        .with_context(|| {
            format!(
                "Source file is not a valid image: {}",
                source_image.display()
            )
        })?;

    // 3. Get root folder cover.jpg path
    let cover_path = get_book_folder_file(book_path, "cover.jpg")?;

    // 4. Backup existing cover.jpg to irie/cover.jpg.bak (if exists)
    if cover_path.exists() {
        let backup_path = get_irie_folder_file(book_path, "cover.jpg.bak")?;
        fs::copy(&cover_path, &backup_path).with_context(|| {
            format!(
                "Failed to backup existing cover to {}",
                backup_path.display()
            )
        })?;
    }

    // 5. Encode as JPEG to temp file (atomic write pattern)
    let temp_path = cover_path.with_extension("jpg.tmp");
    img.save_with_format(&temp_path, ImageFormat::Jpeg)
        .with_context(|| format!("Failed to save image as JPEG to {}", temp_path.display()))?;

    // 6. Atomic rename temp -> cover.jpg
    fs::rename(&temp_path, &cover_path).with_context(|| {
        format!(
            "Failed to move file to final location: {}",
            cover_path.display()
        )
    })?;

    // 7. Invalidate thumbnail cache if it exists (thumbnail is in irie/)
    let thumbnail_path = get_irie_folder_file(book_path, "thumbnail.jpg")?;
    if thumbnail_path.exists() {
        let _ = fs::remove_file(&thumbnail_path); // Ignore failure, non-critical
    }

    Ok(cover_path)
}

/// Extract book folder name from a markdown file path
///
/// "my-awesome-book.md" -> "my-awesome-book"
/// "/path/to/story.md" -> "story"
fn extract_folder_name(md_path: &Path) -> Result<String> {
    let folder_name = md_path
        .file_stem()
        .and_then(|s| s.to_str())
        .with_context(|| {
            format!(
                "Failed to extract folder name from path: {}",
                md_path.display()
            )
        })?;

    if folder_name.is_empty() {
        anyhow::bail!("Folder name cannot be empty");
    }

    Ok(folder_name.to_string())
}

/// Add a book to the workspace by copying a .md file
///
/// Creates folder structure: workspace_root/<filename_without_ext>/<filename>.md
/// Also creates irie/ subfolder and generates default metadata.yaml
///
/// Returns: (book_path, is_duplicate)
/// - book_path: Path to the created book file
/// - is_duplicate: true if folder already existed (updated existing book)
pub fn add_book_to_workspace(workspace_root: &Path, source_md: &Path) -> Result<(PathBuf, bool)> {
    // 1. Validate source_md exists
    if !source_md.exists() {
        anyhow::bail!("Source file does not exist: {}", source_md.display());
    }

    // 2. Validate source_md is .md file
    if source_md.extension().and_then(|s| s.to_str()) != Some("md") {
        anyhow::bail!("Source file must be a .md file: {}", source_md.display());
    }

    // 3. Extract folder name
    let folder_name = extract_folder_name(source_md)?;

    // Get the source filename
    let filename = source_md
        .file_name()
        .and_then(|s| s.to_str())
        .with_context(|| format!("Failed to extract filename from: {}", source_md.display()))?;

    // 4. Check if folder exists (duplicate detection)
    let book_folder = workspace_root.join(&folder_name);
    let is_duplicate = book_folder.exists();

    // 5. Create folder if doesn't exist
    if !is_duplicate {
        fs::create_dir_all(&book_folder)
            .with_context(|| format!("Failed to create book folder: {}", book_folder.display()))?;
    }

    // 6. Copy source to <folder_name>/<filename>.md using atomic pattern
    let book_path = book_folder.join(filename);
    let temp_path = book_path.with_extension("tmp");

    // Copy to temp file
    fs::copy(source_md, &temp_path).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            source_md.display(),
            temp_path.display()
        )
    })?;

    // Atomic rename
    fs::rename(&temp_path, &book_path).with_context(|| {
        format!(
            "Failed to move {} to {}",
            temp_path.display(),
            book_path.display()
        )
    })?;

    // 7. Create irie/ subfolder if needed
    let irie_folder = book_folder.join("irie");
    fs::create_dir_all(&irie_folder)
        .with_context(|| format!("Failed to create irie folder: {}", irie_folder.display()))?;

    // 8. Generate default metadata.yaml if doesn't exist (don't overwrite on duplicate)
    let metadata_path = book_folder.join("metadata.yaml");
    if !metadata_path.exists() {
        let default_metadata = BookMetadata {
            title: folder_name,
            author: "Unknown Author".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };
        save_metadata(&book_path, &default_metadata)?;
    }

    Ok((book_path, is_duplicate))
}

/// Change a book's source .md file while preserving irie/ folder
///
/// Replaces the .md file in the book's folder, keeps all irie/ contents intact
///
/// Returns: Path to the replaced book file
pub fn change_book_file(book_path: &Path, new_source_md: &Path) -> Result<PathBuf> {
    // 1. Validate new_source_md exists
    if !new_source_md.exists() {
        anyhow::bail!("Source file does not exist: {}", new_source_md.display());
    }

    // 2. Validate new_source_md is .md file
    if new_source_md.extension().and_then(|s| s.to_str()) != Some("md") {
        anyhow::bail!(
            "Source file must be a .md file: {}",
            new_source_md.display()
        );
    }

    // 3. Copy new_source_md to book_path using atomic pattern (temp + rename)
    let temp_path = book_path.with_extension("tmp");

    // Copy to temp file
    fs::copy(new_source_md, &temp_path).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            new_source_md.display(),
            temp_path.display()
        )
    })?;

    // Atomic rename
    fs::rename(&temp_path, book_path).with_context(|| {
        format!(
            "Failed to move {} to {}",
            temp_path.display(),
            book_path.display()
        )
    })?;

    // irie/ folder is automatically preserved (we only replace the .md file)
    Ok(book_path.to_path_buf())
}

/// Delete a book's folder and all its contents
///
/// CAUTION: This recursively removes the parent directory of the given book path.
///
/// # Arguments
/// * `book_path` - Path to the book's .md file (e.g., `workspace/book_name/book_name.md`)
///
/// # Safety
/// This function performs basic validation:
/// 1. `book_path` must exist
/// 2. `book_path` must have a parent directory
/// 3. The parent directory is what gets removed
pub fn delete_book_folder(book_path: &Path) -> Result<()> {
    if !book_path.exists() {
        anyhow::bail!("Book file does not exist: {}", book_path.display());
    }

    let parent_dir = book_path.parent().ok_or_else(|| {
        anyhow::anyhow!("Book path has no parent directory: {}", book_path.display())
    })?;

    // Safety check: Don't delete if parent is root or empty
    if parent_dir.as_os_str().is_empty() {
        anyhow::bail!("Cannot delete empty parent path");
    }

    fs::remove_dir_all(parent_dir)
        .with_context(|| format!("Failed to delete book folder: {}", parent_dir.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    fn reads_utf8_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.md");

        fs::write(&file_path, "Hello, world!")?;

        let content = read_file(&file_path)?;
        assert_eq!(content, "Hello, world!");

        Ok(())
    }

    #[test]
    fn reads_romanian_utf8() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.md");

        fs::write(&file_path, "Bună ziua! Ăsta e un test.")?;

        let content = read_file(&file_path)?;
        assert!(content.contains("ă"));
        assert!(content.contains("Ă"));

        Ok(())
    }

    #[test]
    fn generates_output_path_simple() -> Result<()> {
        let input = Path::new("book.md");
        let output = generate_output_path(input)?;

        assert_eq!(output, PathBuf::from("irie/fixed.md"));

        Ok(())
    }

    #[test]
    fn generates_output_path_with_directory() -> Result<()> {
        // Use temp directory since we need to create irie/ subfolder
        let temp_dir = TempDir::new()?;
        let input = temp_dir.path().join("book.md");
        let output = generate_output_path(&input)?;

        assert_eq!(output, temp_dir.path().join("irie/fixed.md"));

        Ok(())
    }

    #[test]
    fn generates_output_path_no_extension() -> Result<()> {
        let input = Path::new("book");
        let output = generate_output_path(input)?;

        assert_eq!(output, PathBuf::from("irie/fixed.md"));

        Ok(())
    }

    #[test]
    fn generates_output_path_preserves_directory() -> Result<()> {
        let input = Path::new("/home/andrei/work/ebook_processing/books/Vampires.md");
        let output = generate_output_path(input)?;

        assert!(output.to_str().unwrap().contains("/books/"));
        assert!(output.to_str().unwrap().ends_with("fixed.md"));

        Ok(())
    }

    #[test]
    fn writes_file_atomically() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("output.md");

        write_file(&file_path, "Test content")?;

        // File should exist
        assert!(file_path.exists());

        // Content should match
        let content = fs::read_to_string(&file_path)?;
        assert_eq!(content, "Test content");

        // Temp file should be gone
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists());

        Ok(())
    }

    #[test]
    fn overwrites_existing_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("output.md");

        // Write first time
        write_file(&file_path, "First")?;
        assert_eq!(fs::read_to_string(&file_path)?, "First");

        // Overwrite
        write_file(&file_path, "Second")?;
        assert_eq!(fs::read_to_string(&file_path)?, "Second");

        Ok(())
    }

    #[test]
    fn preserves_utf8_when_writing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("romanian.md");

        let content = "Cronicile vampirilor din București";
        write_file(&file_path, content)?;

        let read_content = read_file(&file_path)?;
        assert_eq!(read_content, content);
        assert!(read_content.contains("ș"));

        Ok(())
    }

    #[test]
    fn strips_utf8_bom_from_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("with-bom.md");

        // Write file with UTF-8 BOM prefix
        let content_with_bom = "\u{FEFF}Hello world";
        fs::write(&file_path, content_with_bom)?;

        // Read via read_file - BOM should be stripped
        let content = read_file(&file_path)?;

        // BOM should be gone
        assert_eq!(content, "Hello world");
        assert!(!content.starts_with('\u{FEFF}'));

        Ok(())
    }

    #[test]
    fn handles_file_without_bom() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("no-bom.md");

        // Write file without BOM
        let content_no_bom = "Hello world";
        fs::write(&file_path, content_no_bom)?;

        // Read via read_file - should be unchanged
        let content = read_file(&file_path)?;

        // Content should be unchanged
        assert_eq!(content, "Hello world");

        Ok(())
    }

    #[test]
    fn writes_utf8_without_bom() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("output.md");

        // Write content via write_file
        write_file(&file_path, "Test content")?;

        // Read raw bytes to check for BOM
        let bytes = fs::read(&file_path)?;

        // UTF-8 BOM is bytes [0xEF, 0xBB, 0xBF]
        // File should NOT start with BOM
        assert!(bytes.len() >= 3);
        let has_bom = bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF;
        assert!(!has_bom, "File should not start with UTF-8 BOM");

        // Verify content is valid UTF-8
        let content = String::from_utf8(bytes)?;
        assert_eq!(content, "Test content");

        Ok(())
    }

    #[test]
    fn preserves_utf8_with_bom_stripped() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("romanian-bom.md");

        // Write file with BOM + Romanian characters
        let content_with_bom = "\u{FEFF}Bună ziua și la revedere";
        fs::write(&file_path, content_with_bom)?;

        // Read via read_file
        let content = read_file(&file_path)?;

        // BOM should be stripped
        assert!(!content.starts_with('\u{FEFF}'));
        assert_eq!(content, "Bună ziua și la revedere");

        // Romanian characters should be preserved
        assert!(content.contains('ă'));
        assert!(content.contains('ș'));

        Ok(())
    }

    // Metadata operations tests
    #[test]
    fn load_metadata_returns_none_when_file_missing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content")?;

        let result = load_metadata(&book_path)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn load_metadata_parses_existing_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content")?;

        // Create metadata file in root directory
        let metadata_path = temp_dir.path().join("metadata.yaml");
        fs::write(&metadata_path, "title: Test Book\nauthor: Test Author")?;

        let result = load_metadata(&book_path)?;
        assert!(result.is_some());

        match result {
            Some(metadata) => {
                assert_eq!(metadata.title, "Test Book");
                assert_eq!(metadata.author, "Test Author");
            }
            None => panic!("Expected metadata to be loaded"),
        }

        Ok(())
    }

    #[test]
    fn load_metadata_parses_all_fields_including_identifier() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content")?;

        // Create metadata file with all fields
        let metadata_path = temp_dir.path().join("metadata.yaml");
        let content = r#"---
title: Test Book
author: Jane Doe
belongs-to-collection: Test Series
group-position: 1
language: ro-RO
rights: © 2026 All Rights Reserved
cover-image: cover.jpg
identifier:
  - scheme: ISBN-13
    text: 978-0-123456-78-9
---"#;
        fs::write(&metadata_path, content)?;

        let result = load_metadata(&book_path)?;
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.title, "Test Book");
        assert_eq!(metadata.author, "Jane Doe");
        assert_eq!(
            metadata.belongs_to_collection,
            Some("Test Series".to_string())
        );
        assert_eq!(metadata.group_position, Some(1));
        assert_eq!(metadata.language, Some("ro-RO".to_string()));
        assert_eq!(
            metadata.rights,
            Some("© 2026 All Rights Reserved".to_string())
        );
        assert_eq!(metadata.cover_image, Some("cover.jpg".to_string()));

        // Validate identifier (list of identifiers)
        assert!(metadata.identifier.is_some());
        let identifiers = metadata.identifier.as_ref().unwrap();
        assert_eq!(identifiers.len(), 1);
        assert_eq!(identifiers[0].scheme, Some("ISBN-13".to_string()));
        assert_eq!(identifiers[0].text, Some("978-0-123456-78-9".to_string()));

        // Validate helper method (re-read to get fresh reference)
        let result = load_metadata(&book_path)?.unwrap();
        assert_eq!(
            result.identifier_display_text(),
            Some("ISBN-13: 978-0-123456-78-9".to_string())
        );

        Ok(())
    }

    #[test]
    fn load_metadata_handles_frontmatter_delimiters() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content")?;

        // Create metadata file in root directory
        let metadata_path = temp_dir.path().join("metadata.yaml");
        let content = "---\ntitle: Test\nauthor: Author\n---\n";
        fs::write(&metadata_path, content)?;

        let result = load_metadata(&book_path)?;
        assert!(result.is_some());

        match result {
            Some(metadata) => {
                assert_eq!(metadata.title, "Test");
                assert_eq!(metadata.author, "Author");
            }
            None => panic!("Expected metadata to be loaded"),
        }

        Ok(())
    }

    #[test]
    fn save_metadata_creates_file_with_frontmatter() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content")?;

        let metadata = BookMetadata {
            title: "My Book".to_string(),
            author: "Jane Doe".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };

        save_metadata(&book_path, &metadata)?;

        // Check metadata file was created in root directory
        let metadata_path = temp_dir.path().join("metadata.yaml");
        assert!(metadata_path.exists());

        let content = fs::read_to_string(&metadata_path)?;
        assert!(content.starts_with("---\n"));
        assert!(content.ends_with("---\n"));
        assert!(content.contains("title: My Book"));
        assert!(content.contains("author: Jane Doe"));

        Ok(())
    }

    #[test]
    fn save_metadata_adds_predefined_defaults() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content")?;

        let metadata = BookMetadata {
            title: "Test".to_string(),
            author: "Author".to_string(),
            belongs_to_collection: None,
            group_position: None,
            language: None,
            rights: None,
            cover_image: None,
            replace_pairs: None,
            identifier: None,
        };

        save_metadata(&book_path, &metadata)?;

        // Reload and verify defaults were added
        let loaded = load_metadata(&book_path)?;
        assert!(loaded.is_some());

        match loaded {
            Some(metadata) => {
                assert_eq!(metadata.language, Some("ro-RO".to_string()));
                assert_eq!(metadata.cover_image, Some("cover.jpg".to_string()));
                assert!(metadata.rights.is_some());
                assert!(
                    metadata
                        .rights
                        .as_ref()
                        .unwrap()
                        .contains("All Rights Reserved")
                );
            }
            None => panic!("Expected metadata to be loaded"),
        }

        Ok(())
    }

    #[test]
    fn save_metadata_rejects_invalid_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("book.md");
        fs::write(&book_path, "content").unwrap();

        let metadata = BookMetadata::default();

        let result = save_metadata(&book_path, &metadata);
        assert!(result.is_err());

        // Temp file should not exist after failed write
        let metadata_path = temp_dir.path().join("metadata.yaml");
        let temp_path = metadata_path.with_extension("tmp");
        assert!(!temp_path.exists());

        // Verify metadata file was NOT created (validation failed before writing)
        assert!(!metadata_path.exists());
    }

    // extract_folder_name tests
    #[test]
    fn test_extract_folder_name_basic() -> Result<()> {
        let path = Path::new("book.md");
        let folder_name = extract_folder_name(path)?;
        assert_eq!(folder_name, "book");
        Ok(())
    }

    #[test]
    fn test_extract_folder_name_with_path() -> Result<()> {
        let path = Path::new("/path/to/story.md");
        let folder_name = extract_folder_name(path)?;
        assert_eq!(folder_name, "story");
        Ok(())
    }

    #[test]
    fn test_extract_folder_name_multiple_dots() -> Result<()> {
        let path = Path::new("my.book.title.md");
        let folder_name = extract_folder_name(path)?;
        assert_eq!(folder_name, "my.book.title");
        Ok(())
    }

    #[test]
    fn test_extract_folder_name_no_extension() {
        let path = Path::new("book");
        let result = extract_folder_name(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "book");
    }

    #[test]
    fn test_extract_folder_name_empty() {
        let path = Path::new("");
        let result = extract_folder_name(path);
        assert!(result.is_err());
    }

    // add_book_to_workspace tests
    #[test]
    fn test_add_book_creates_structure() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create a source .md file
        let source_md = temp_dir.path().join("test_book.md");
        fs::write(&source_md, "# Test Book\n\nThis is a test book.")?;

        // Add book to workspace
        let (book_path, is_duplicate) = add_book_to_workspace(workspace_root, &source_md)?;

        // Should not be a duplicate
        assert!(!is_duplicate);

        // Book folder should exist
        let book_folder = workspace_root.join("test_book");
        assert!(book_folder.exists());
        assert!(book_folder.is_dir());

        // Book file should exist in the folder
        assert_eq!(book_path, book_folder.join("test_book.md"));
        assert!(book_path.exists());

        // File content should match
        let content = fs::read_to_string(&book_path)?;
        assert_eq!(content, "# Test Book\n\nThis is a test book.");

        // irie/ subfolder should exist
        let irie_folder = book_folder.join("irie");
        assert!(irie_folder.exists());
        assert!(irie_folder.is_dir());

        // metadata.yaml should exist in root with defaults
        let metadata_path = book_folder.join("metadata.yaml");
        assert!(metadata_path.exists());

        // Load and verify metadata
        let metadata = load_metadata(&book_path)?;
        assert!(metadata.is_some());
        match metadata {
            Some(m) => {
                assert_eq!(m.title, "test_book");
                assert_eq!(m.author, "Unknown Author");
                assert_eq!(m.language, Some("ro-RO".to_string()));
            }
            None => panic!("Expected metadata to exist"),
        }

        Ok(())
    }

    #[test]
    fn test_add_book_detects_duplicate() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create a source .md file
        let source_md = temp_dir.path().join("story.md");
        fs::write(&source_md, "# Original content")?;

        // Add book first time
        let (first_path, first_is_duplicate) = add_book_to_workspace(workspace_root, &source_md)?;
        assert!(!first_is_duplicate);
        assert!(first_path.exists());

        // Create a different source file with same name
        let source_md2 = temp_dir.path().join("story_new.md");
        fs::write(&source_md2, "# Updated content")?;

        // Rename it to same name
        fs::rename(&source_md2, &source_md)?;

        // Add book second time (duplicate)
        let (second_path, second_is_duplicate) = add_book_to_workspace(workspace_root, &source_md)?;
        assert!(second_is_duplicate, "Should detect duplicate");
        assert_eq!(second_path, first_path);

        // Content should be updated
        let content = fs::read_to_string(&second_path)?;
        assert_eq!(content, "# Updated content");

        Ok(())
    }

    #[test]
    fn test_add_book_preserves_metadata_on_duplicate() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add first book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original")?;

        let (book_path, _) = add_book_to_workspace(workspace_root, &source_md)?;

        // Customize metadata
        let custom_metadata = BookMetadata {
            title: "My Custom Title".to_string(),
            author: "Jane Doe".to_string(),
            belongs_to_collection: Some("My Series".to_string()),
            group_position: Some(1),
            language: Some("en-US".to_string()),
            rights: Some("© 2026 Jane Doe".to_string()),
            cover_image: Some("custom_cover.jpg".to_string()),
            replace_pairs: None,
            identifier: None,
        };
        save_metadata(&book_path, &custom_metadata)?;

        // Add duplicate book
        let source_md2 = temp_dir.path().join("mybook_v2.md");
        fs::write(&source_md2, "# Updated content")?;
        fs::rename(&source_md2, &source_md)?;

        let (updated_path, is_duplicate) = add_book_to_workspace(workspace_root, &source_md)?;
        assert!(is_duplicate);

        // Metadata should be preserved
        let loaded_metadata = load_metadata(&updated_path)?;
        assert!(loaded_metadata.is_some());

        match loaded_metadata {
            Some(m) => {
                assert_eq!(m.title, "My Custom Title");
                assert_eq!(m.author, "Jane Doe");
                assert_eq!(m.belongs_to_collection, Some("My Series".to_string()));
                assert_eq!(m.group_position, Some(1));
            }
            None => panic!("Expected metadata to be preserved"),
        }

        Ok(())
    }

    #[test]
    fn test_add_book_validates_source() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Test 1: Non-existent file
        let nonexistent = temp_dir.path().join("doesnt_exist.md");
        let result = add_book_to_workspace(workspace_root, &nonexistent);
        assert!(result.is_err(), "Should reject non-existent file");

        // Test 2: Non-.md file
        let txt_file = temp_dir.path().join("not_markdown.txt");
        fs::write(&txt_file, "content").unwrap();
        let result = add_book_to_workspace(workspace_root, &txt_file);
        assert!(result.is_err(), "Should reject non-.md file");
    }

    #[test]
    fn test_add_book_uses_temp_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create source file
        let source_md = temp_dir.path().join("atomic_test.md");
        fs::write(&source_md, "# Atomic test")?;

        // Add book
        add_book_to_workspace(workspace_root, &source_md)?;

        // Check that no .tmp files exist
        let book_folder = workspace_root.join("atomic_test");
        let tmp_files: Vec<_> = fs::read_dir(&book_folder)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();

        assert!(
            tmp_files.is_empty(),
            "No .tmp files should remain after operation"
        );

        Ok(())
    }

    // change_book_file tests
    #[test]
    fn test_change_book_file_replaces_content() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add a book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original content")?;
        let (book_path, _) = add_book_to_workspace(workspace_root, &source_md)?;

        // Verify original content
        let original_content = fs::read_to_string(&book_path)?;
        assert_eq!(original_content, "# Original content");

        // Create new source file with different content
        let new_source = temp_dir.path().join("updated.md");
        fs::write(&new_source, "# Updated content")?;

        // Change book file
        let result_path = change_book_file(&book_path, &new_source)?;
        assert_eq!(result_path, book_path);

        // Verify content was updated
        let updated_content = fs::read_to_string(&book_path)?;
        assert_eq!(updated_content, "# Updated content");

        Ok(())
    }

    #[test]
    fn test_change_book_file_preserves_irie() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create and add a book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original")?;
        let (book_path, _) = add_book_to_workspace(workspace_root, &source_md)?;

        // Customize metadata
        let custom_metadata = BookMetadata {
            title: "My Custom Title".to_string(),
            author: "Jane Doe".to_string(),
            belongs_to_collection: Some("My Series".to_string()),
            group_position: Some(1),
            language: Some("en-US".to_string()),
            rights: Some("© 2026 Jane Doe".to_string()),
            cover_image: Some("custom_cover.jpg".to_string()),
            ..Default::default()
        };
        save_metadata(&book_path, &custom_metadata)?;

        // Add a cover image to irie/
        let book_folder = book_path.parent().unwrap();
        let irie_folder = book_folder.join("irie");
        let cover_path = irie_folder.join("cover.jpg");
        create_test_image(&cover_path)?;

        // Change book file
        let new_source = temp_dir.path().join("updated.md");
        fs::write(&new_source, "# Updated content")?;
        change_book_file(&book_path, &new_source)?;

        // Metadata should be preserved
        let loaded_metadata = load_metadata(&book_path)?;
        assert!(loaded_metadata.is_some());
        match loaded_metadata {
            Some(m) => {
                assert_eq!(m.title, "My Custom Title");
                assert_eq!(m.author, "Jane Doe");
            }
            None => panic!("Metadata should be preserved"),
        }

        // Cover should still exist
        assert!(cover_path.exists(), "Cover image should be preserved");

        Ok(())
    }

    #[test]
    fn test_change_book_file_validates_source() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create a book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original").unwrap();
        let (book_path, _) = add_book_to_workspace(workspace_root, &source_md).unwrap();

        // Test 1: Non-existent file
        let nonexistent = temp_dir.path().join("doesnt_exist.md");
        let result = change_book_file(&book_path, &nonexistent);
        assert!(result.is_err(), "Should reject non-existent file");

        // Test 2: Non-.md file
        let txt_file = temp_dir.path().join("not_markdown.txt");
        fs::write(&txt_file, "content").unwrap();
        let result = change_book_file(&book_path, &txt_file);
        assert!(result.is_err(), "Should reject non-.md file");
    }

    #[test]
    fn test_change_book_file_atomic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create a book
        let source_md = temp_dir.path().join("mybook.md");
        fs::write(&source_md, "# Original")?;
        let (book_path, _) = add_book_to_workspace(workspace_root, &source_md)?;

        // Change book file
        let new_source = temp_dir.path().join("updated.md");
        fs::write(&new_source, "# Updated")?;
        change_book_file(&book_path, &new_source)?;

        // No .tmp files should exist
        let book_folder = book_path.parent().unwrap();
        let tmp_files: Vec<_> = fs::read_dir(book_folder)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();

        assert!(
            tmp_files.is_empty(),
            "No .tmp files should remain after operation"
        );

        Ok(())
    }

    // Helper to create a minimal valid test image
    fn create_test_image(path: &Path) -> Result<()> {
        use image::{ImageBuffer, Rgb};
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0, 0]));
        img.save(path)?;
        Ok(())
    }

    #[test]
    fn replace_cover_creates_new_cover_when_none_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1")?;

        // Create a test source image
        let source_image = temp_dir.path().join("test.jpg");
        create_test_image(&source_image)?;

        let result = replace_cover_image(&book_path, &source_image)?;

        // Cover should be in root folder
        assert_eq!(result, book_dir.join("cover.jpg"));
        assert!(result.exists());

        Ok(())
    }

    #[test]
    fn replace_cover_backs_up_existing_cover() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        // Create irie/ folder
        fs::create_dir(book_dir.join("irie"))?;

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1")?;

        // Create existing cover in root
        let existing_cover = book_dir.join("cover.jpg");
        create_test_image(&existing_cover)?;

        // Create new source image
        let source_image = temp_dir.path().join("new_cover.jpg");
        create_test_image(&source_image)?;

        let result = replace_cover_image(&book_path, &source_image)?;

        // Should have backed up the old cover in irie/
        let backup = book_dir.join("irie/cover.jpg.bak");
        assert!(backup.exists(), "Backup file should exist in irie/");

        // New cover should exist in root
        assert!(result.exists());
        assert_eq!(result, book_dir.join("cover.jpg"));

        Ok(())
    }

    #[test]
    fn replace_cover_rejects_invalid_image() {
        let temp_dir = TempDir::new().unwrap();
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir).unwrap();

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1").unwrap();

        // Create a text file (not an image)
        let invalid_file = temp_dir.path().join("not_an_image.txt");
        fs::write(&invalid_file, "This is not an image").unwrap();

        let result = replace_cover_image(&book_path, &invalid_file);
        assert!(result.is_err(), "Should reject invalid image");
    }

    #[test]
    fn replace_cover_handles_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir).unwrap();

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1").unwrap();

        let nonexistent = temp_dir.path().join("doesnt_exist.jpg");

        let result = replace_cover_image(&book_path, &nonexistent);
        assert!(result.is_err(), "Should fail when source doesn't exist");
    }

    #[test]
    fn replace_cover_atomic_operation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1")?;

        let source_image = temp_dir.path().join("test.jpg");
        create_test_image(&source_image)?;

        replace_cover_image(&book_path, &source_image)?;

        // Temp file should not exist after successful operation
        let temp_path = book_dir.join("cover.jpg.tmp");
        assert!(!temp_path.exists(), "Temp file should be cleaned up");

        Ok(())
    }

    #[test]
    fn replace_cover_preserves_image_after_copy() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1")?;

        let source_image = temp_dir.path().join("test.jpg");
        create_test_image(&source_image)?;

        let result = replace_cover_image(&book_path, &source_image)?;

        // Verify the cover can be loaded as a valid image
        let img = image::ImageReader::open(&result)?;
        assert!(img.decode().is_ok(), "Cover should be a valid image");

        Ok(())
    }

    #[test]
    fn replace_cover_invalidates_thumbnail() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_dir = temp_dir.path().join("book1");
        fs::create_dir(&book_dir)?;

        // Create irie/ folder
        fs::create_dir(book_dir.join("irie"))?;

        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Chapter 1")?;

        // Create existing cover in irie/
        let existing_cover = book_dir.join("irie/cover.jpg");
        create_test_image(&existing_cover)?;

        // Create existing thumbnail in irie/
        let thumbnail_path = book_dir.join("irie/thumbnail.jpg");
        fs::write(&thumbnail_path, "fake thumbnail data")?;
        assert!(thumbnail_path.exists());

        // Create new source image
        let source_image = temp_dir.path().join("new_cover.jpg");
        create_test_image(&source_image)?;

        let _ = replace_cover_image(&book_path, &source_image)?;

        // Thumbnail should have been deleted
        assert!(
            !thumbnail_path.exists(),
            "Thumbnail should be invalidated (deleted)"
        );

        Ok(())
    }

    #[test]
    fn replace_cover_converts_png_to_jpeg() -> Result<()> {
        use image::{ImageBuffer, ImageFormat, Rgb};

        let temp_dir = TempDir::new()?;
        let source_png = temp_dir.path().join("source.png");

        // Create PNG test image
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([0u8, 255, 0]));
        img.save(&source_png)?; // Saves as PNG due to extension

        // Set up book
        let book_dir = temp_dir.path().join("book");
        fs::create_dir(&book_dir)?;
        fs::create_dir(book_dir.join("irie"))?;
        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Test")?;

        // Replace cover with PNG source
        let result = replace_cover_image(&book_path, &source_png)?;

        // CRITICAL: Verify actual format is JPEG, not just .jpg extension
        let reader = image::ImageReader::open(&result)?.with_guessed_format()?;
        assert_eq!(
            reader.format(),
            Some(ImageFormat::Jpeg),
            "Output file should be actual JPEG format, not just .jpg extension"
        );

        // Verify it decodes successfully
        assert!(reader.decode().is_ok());

        Ok(())
    }

    #[test]
    fn replace_cover_converts_bmp_to_jpeg() -> Result<()> {
        use image::{ImageBuffer, ImageFormat, Rgb};

        let temp_dir = TempDir::new()?;
        let source_bmp = temp_dir.path().join("source.bmp");

        // Create BMP test image
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0, 0]));
        img.save(&source_bmp)?; // Saves as BMP

        let book_dir = temp_dir.path().join("book");
        fs::create_dir(&book_dir)?;
        fs::create_dir(book_dir.join("irie"))?;
        let book_path = book_dir.join("chapter1.md");
        fs::write(&book_path, "# Test")?;

        let result = replace_cover_image(&book_path, &source_bmp)?;

        // Verify actual format is JPEG
        let reader = image::ImageReader::open(&result)?.with_guessed_format()?;
        assert_eq!(reader.format(), Some(ImageFormat::Jpeg));

        Ok(())
    }

    #[test]
    fn get_irie_folder_file_creates_subfolder() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("mybook.md");
        fs::write(&book_path, "content")?;

        let result = get_irie_folder_file(&book_path, "metadata.yaml")?;

        // Should return irie/metadata.yaml path
        assert_eq!(result, temp_dir.path().join("irie/metadata.yaml"));

        // irie/ directory should be created
        assert!(temp_dir.path().join("irie").exists());
        assert!(temp_dir.path().join("irie").is_dir());

        Ok(())
    }

    #[test]
    fn get_irie_folder_file_handles_existing_folder() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_path = temp_dir.path().join("mybook.md");
        fs::write(&book_path, "content")?;

        // Pre-create irie/ directory
        fs::create_dir(temp_dir.path().join("irie"))?;

        let result = get_irie_folder_file(&book_path, "fixed.md")?;

        assert_eq!(result, temp_dir.path().join("irie/fixed.md"));

        Ok(())
    }

    #[test]
    fn get_irie_folder_file_no_parent_fallback() -> Result<()> {
        let input = Path::new("book.md");
        let result = get_irie_folder_file(input, "metadata.yaml")?;

        // Should fallback to irie/metadata.yaml in current directory
        assert_eq!(result, PathBuf::from("irie/metadata.yaml"));

        Ok(())
    }

    #[test]
    fn delete_book_folder_removes_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let book_folder = temp_dir.path().join("to_delete");
        fs::create_dir(&book_folder)?;

        let book_path = book_folder.join("book.md");
        fs::write(&book_path, "content")?;

        // Also add some other files/subdirs
        fs::create_dir(book_folder.join("irie"))?;
        fs::write(book_folder.join("irie/metadata.yaml"), "meta")?;

        delete_book_folder(&book_path)?;

        assert!(!book_folder.exists(), "Book folder should be removed");
        assert!(temp_dir.path().exists(), "Parent directory should remain");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_css_path_creates_temp_file() -> Result<()> {
        // Clean up any existing file first
        let temp_dir = std::env::temp_dir();
        let css_file = temp_dir.join("iriebook-default.css");
        let _ = fs::remove_file(&css_file);

        let css_path = get_css_path()?;

        // Verify it's in temp directory
        let expected_path = temp_dir.join("iriebook-default.css");
        assert_eq!(css_path, expected_path.display().to_string());

        // Verify file exists and contains correct content
        assert!(Path::new(&css_path).exists());
        let content = fs::read_to_string(&css_path)?;
        assert_eq!(content, EMBEDDED_CSS);
        assert!(content.contains("@charset \"utf-8\""));

        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_css_path_idempotent() -> Result<()> {
        // Clean up any existing files first
        let temp_dir = std::env::temp_dir();
        let css_file = temp_dir.join("iriebook-default.css");
        let tmp_file = temp_dir.join("iriebook-default.css.tmp");
        let _ = fs::remove_file(&css_file);
        let _ = fs::remove_file(&tmp_file);

        let path1 = get_css_path()?;
        let path2 = get_css_path()?;

        assert_eq!(path1, path2);

        let content = fs::read_to_string(&path1)?;
        assert_eq!(content, EMBEDDED_CSS);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_css_path_overwrites_corrupted_file() -> Result<()> {
        // Clean up any existing temp file and create a corrupted file
        let temp_dir = std::env::temp_dir();
        let css_path = temp_dir.join("iriebook-default.css");
        let tmp_file = temp_dir.join("iriebook-default.css.tmp");
        let _ = fs::remove_file(&tmp_file);
        fs::write(&css_path, "corrupted content")?;

        let _result_path = get_css_path()?;

        let content = fs::read_to_string(&css_path)?;
        assert_eq!(content, EMBEDDED_CSS);
        assert_ne!(content, "corrupted content");

        Ok(())
    }

    #[test]
    fn test_embedded_css_content() {
        assert!(!EMBEDDED_CSS.is_empty());
        assert!(EMBEDDED_CSS.contains("@charset"));
        assert!(EMBEDDED_CSS.contains("body"));
        assert!(EMBEDDED_CSS.contains("h1:not(.unnumbered)"));
    }

    #[test]
    fn test_css_prevents_double_page_breaks() {
        assert!(
            EMBEDDED_CSS.contains("section > h1:first-child"),
            "CSS must contain fix for double page breaks"
        );
        assert!(
            EMBEDDED_CSS.contains("page-break-before: auto"),
            "CSS must set page-break-before: auto for first h1"
        );
    }

    #[test]
    fn test_css_has_titlepage_and_toc_guards() {
        assert!(
            EMBEDDED_CSS.contains(".titlepage {"),
            "CSS must include title page container styles"
        );
        assert!(
            EMBEDDED_CSS.contains("page-break-after: always"),
            "CSS must force clean break after title page"
        );
        assert!(
            EMBEDDED_CSS.contains(".titlepage .title"),
            "CSS must include title styling for title page"
        );
        assert!(
            EMBEDDED_CSS.contains(".titlepage .title")
                && EMBEDDED_CSS.contains("break-before: auto"),
            "CSS must prevent chapter page-break rules from splitting title page"
        );
        assert!(
            EMBEDDED_CSS.contains("nav[type=\"toc\"] h1"),
            "CSS must override TOC heading page-break behavior for Kindle"
        );
    }
}
