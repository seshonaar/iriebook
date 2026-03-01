//! Resource Access trait definitions
//!
//! These traits define interfaces for accessing external resources and tools
//! like Pandoc, Calibre, archive creation utilities, and Google Docs API.

use crate::utilities::error::IrieBookError;
use crate::utilities::types::{GitCommit, GitStatus};
use std::path::Path;

/// Information about a Google Doc
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct GoogleDocInfo {
    pub id: String,
    pub name: String,
    pub modified_time: String,
}

impl GoogleDocInfo {
    pub fn new(id: String, name: String, modified_time: String) -> Self {
        Self {
            id,
            name,
            modified_time,
        }
    }
}

/// Trait for Pandoc access (EPUB conversion)
///
/// Implementations of this trait handle conversion from markdown to EPUB format
/// using the Pandoc tool.
pub trait PandocAccess: Send + Sync {
    /// Converts markdown to EPUB using Pandoc
    ///
    /// # Arguments
    /// * `original_input` - Path to the original input markdown file (for cover lookup)
    /// * `fixed_md` - Path to the fixed markdown file to convert
    /// * `output_epub` - Path where the EPUB should be written
    /// * `custom_metadata_path` - Optional path to a custom metadata.yaml file.
    ///   If provided, this will be used instead of the book's metadata.yaml.
    ///   Use this to suppress pandoc's auto-generated copyright page by providing
    ///   a metadata file without the `rights` field.
    ///
    /// # Returns
    /// * `Ok(String)` with command output if conversion succeeds
    /// * `Err(IrieBookError)` if conversion fails
    fn convert_to_epub(
        &self,
        original_input: &Path,
        fixed_md: &Path,
        output_epub: &Path,
        custom_metadata_path: Option<&Path>,
    ) -> Result<String, IrieBookError>;
}

/// Trait for Calibre access (Kindle conversion and metadata)
///
/// Implementations of this trait handle conversion from EPUB to Kindle format
/// and metadata stamping using Calibre tools (ebook-convert, ebook-meta).
pub trait CalibreAccess: Send + Sync {
    /// Converts EPUB to Kindle format (AZW3) using ebook-convert
    ///
    /// # Arguments
    /// * `input_md` - Path to the original markdown file (for metadata)
    /// * `input_epub` - Path to the input EPUB file
    ///
    /// # Returns
    /// * `Ok(String)` with command output if conversion succeeds
    /// * `Err(IrieBookError)` if conversion fails
    fn convert_to_kindle(&self, input_md: &Path, input_epub: &Path) -> Result<String, IrieBookError>;

    /// Stamps series metadata on a Kindle file using ebook-meta
    ///
    /// # Arguments
    /// * `file_path` - Path to the Kindle file
    /// * `series` - Name of the book series
    /// * `index` - Position in the series
    ///
    /// # Returns
    /// * `Ok(String)` with command output if stamping succeeds
    /// * `Err(IrieBookError)` if stamping fails
    fn stamp_metadata(&self, file_path: &Path, series: &str, index: u32) -> Result<String, IrieBookError>;

    /// Launch ebook-viewer to display an EPUB file
    ///
    /// Opens the EPUB in Calibre's ebook-viewer in a background process
    /// (non-blocking - doesn't wait for viewer to close)
    ///
    /// # Arguments
    /// * `epub_path` - Path to the EPUB file to view
    ///
    /// # Returns
    /// * `Ok(String)` with success message if viewer launches
    /// * `Err(IrieBookError)` if EPUB file not found or launch fails
    fn view_ebook(&self, epub_path: &Path) -> Result<String, IrieBookError>;
}

/// Trait for archive access (ZIP creation)
///
/// Implementations of this trait handle creation of ZIP archives containing
/// both EPUB and Kindle versions of the book.
pub trait ArchiveAccess: Send + Sync {
    /// Creates a ZIP archive containing EPUB and AZW3 files
    ///
    /// # Arguments
    /// * `input_epub` - Path to the EPUB file (AZW3 path derived from this)
    ///
    /// # Returns
    /// * `Ok(String)` with archive creation message if succeeds
    /// * `Err(IrieBookError)` if archive creation fails
    fn create_book_archive(&self, input_epub: &Path) -> Result<String, IrieBookError>;
}

/// Trait for Git access (version control operations)
///
/// Implementations of this trait handle git operations using gitoxide
pub trait GitAccess: Send + Sync {
    /// Clone repository from remote URL
    ///
    /// # Arguments
    /// * `url` - GitHub repository URL (HTTPS)
    /// * `path` - Local path where repository should be cloned
    /// * `token` - GitHub OAuth token for authentication
    ///
    /// # Returns
    /// * `Ok(())` if clone succeeds
    /// * `Err(IrieBookError)` if clone fails
    fn clone_repository(&self, url: &str, path: &Path, token: &str) -> Result<(), IrieBookError>;

    /// Get remote URL from existing repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(String)` with remote URL if found
    /// * `Err(IrieBookError)` if repository not found or no remote configured
    fn get_remote_url(&self, repo_path: &Path) -> Result<String, IrieBookError>;

    /// Check if directory is a git repository
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// * `true` if directory contains a valid git repository
    /// * `false` otherwise
    fn is_repository(&self, path: &Path) -> bool;

    /// Add all changes to staging
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(())` if changes staged successfully
    /// * `Err(IrieBookError)` if staging fails
    fn add_all(&self, repo_path: &Path) -> Result<(), IrieBookError>;

    /// Create commit with message
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `message` - Commit message
    ///
    /// # Returns
    /// * `Ok(String)` with commit hash if commit succeeds
    /// * `Err(IrieBookError)` if commit fails
    fn commit(&self, repo_path: &Path, message: &str) -> Result<String, IrieBookError>;

    /// Pull with rebase, auto-resolve conflicts with --ours
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(())` if pull succeeds
    /// * `Err(IrieBookError)` if pull fails
    fn pull_rebase_ours(&self, repo_path: &Path) -> Result<(), IrieBookError>;

    /// Push to remote
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `token` - GitHub OAuth token for authentication
    ///
    /// # Returns
    /// * `Ok(())` if push succeeds
    /// * `Err(IrieBookError)` if push fails
    fn push(&self, repo_path: &Path, token: &str) -> Result<(), IrieBookError>;

    /// Get simplified commit history
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `limit` - Maximum number of commits to retrieve
    ///
    /// # Returns
    /// * `Ok(Vec<GitCommit>)` with commit history
    /// * `Err(IrieBookError)` if operation fails
    fn get_log(&self, repo_path: &Path, limit: usize) -> Result<Vec<GitCommit>, IrieBookError>;

    /// Check repository status (clean, ahead, behind, etc.)
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(GitStatus)` with repository status
    /// * `Err(IrieBookError)` if operation fails
    fn get_status(&self, repo_path: &Path) -> Result<GitStatus, IrieBookError>;

    /// Check if there are uncommitted changes
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(true)` if there are uncommitted changes
    /// * `Ok(false)` if working directory is clean
    /// * `Err(IrieBookError)` if operation fails
    fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool, IrieBookError>;

    /// Get list of changed files in a commit
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `commit_hash` - Commit hash to analyze
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` with relative file paths of changed files
    /// * `Err(IrieBookError)` if operation fails
    fn get_changed_files(&self, repo_path: &Path, commit_hash: &str) -> Result<Vec<String>, IrieBookError>;

    /// Discard all local changes (both tracked and untracked)
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(())` if local changes discarded successfully
    /// * `Err(IrieBookError)` if operation fails
    fn discard_local_changes(&self, repo_path: &Path) -> Result<(), IrieBookError>;

    /// Prepare working directory for rebase (discard uncommitted changes + clean untracked)
    /// This is gentler than discard_local_changes - it preserves commits, only cleans working dir
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(())` if working directory prepared successfully
    /// * `Err(IrieBookError)` if operation fails
    fn prepare_for_rebase(&self, repo_path: &Path) -> Result<(), IrieBookError>;

    /// Check if any files in a folder have uncommitted changes
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `folder_path` - Path to the folder to check (checks all tracked files within)
    ///
    /// # Returns
    /// * `Ok(true)` if any files in the folder have uncommitted changes (modified, added, deleted, or untracked)
    /// * `Ok(false)` if all files are unchanged, or if not in a git repository, or if folder is outside repository
    /// * `Err(IrieBookError)` if operation fails
    fn get_folder_status(&self, repo_path: &Path, folder_path: &Path) -> Result<bool, IrieBookError>;

    /// Get all files with uncommitted changes (for bulk status checking)
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    /// * `Ok(Vec<PathBuf>)` with absolute paths to all files with uncommitted changes
    /// * `Err(IrieBookError)` if operation fails
    fn get_all_changed_files(&self, repo_path: &Path) -> Result<Vec<std::path::PathBuf>, IrieBookError>;
}

/// Trait for providing OAuth tokens (for testability)
///
/// Implementations provide access tokens for OAuth-based authentication.
/// This abstraction allows for easy mocking in tests.
#[async_trait::async_trait]
pub trait TokenProvider: Send + Sync {
    /// Get a valid access token, refreshing if necessary
    ///
    /// # Returns
    /// * `Ok(String)` with valid access token
    /// * `Err(IrieBookError)` if token cannot be obtained
    async fn get_valid_token(&self) -> Result<String, IrieBookError>;
}

/// Blanket implementation for Arc<T> where T: TokenProvider
#[async_trait::async_trait]
impl<T: TokenProvider + ?Sized> TokenProvider for std::sync::Arc<T> {
    async fn get_valid_token(&self) -> Result<String, IrieBookError> {
        (**self).get_valid_token().await
    }
}

/// Result of document sync operation
#[derive(Debug, Clone, PartialEq)]
pub enum SyncResult {
    /// Successfully synced from remote source
    Synced,
    /// Document not linked to any remote source
    NotLinked,
}

/// Trait for document synchronization (for testability)
///
/// Implementations handle syncing documents from remote sources.
/// This abstraction allows for easy mocking in tests.
#[async_trait::async_trait]
pub trait DocumentSyncer: Send + Sync {
    /// Sync a document from its linked remote source
    ///
    /// # Arguments
    /// * `book_path` - Path to the book's markdown file
    /// * `token` - Valid OAuth access token
    ///
    /// # Returns
    /// * `Ok(SyncResult)` with sync details
    /// * `Err(IrieBookError)` if sync fails
    async fn sync_document(&self, book_path: &Path, token: &str) -> Result<SyncResult, IrieBookError>;
}

/// Blanket implementation for Arc<T> where T: DocumentSyncer
#[async_trait::async_trait]
impl<T: DocumentSyncer + ?Sized> DocumentSyncer for std::sync::Arc<T> {
    async fn sync_document(&self, book_path: &Path, token: &str) -> Result<SyncResult, IrieBookError> {
        (**self).sync_document(book_path, token).await
    }
}

/// Trait for Google Docs access (listing and exporting documents)
///
/// Implementations of this trait handle interaction with Google Docs and Drive APIs
#[async_trait::async_trait]
pub trait GoogleDocsAccess: Send + Sync {
    /// List Google Docs documents accessible by the authenticated user
    ///
    /// # Arguments
    /// * `token` - Google OAuth token
    /// * `max_results` - Maximum number of documents to return
    ///
    /// # Returns
    /// * `Ok(Vec<GoogleDocInfo>)` with list of documents
    /// * `Err(IrieBookError)` if API call fails
    async fn list_documents(&self, token: &str, max_results: u32) -> Result<Vec<GoogleDocInfo>, IrieBookError>;

    /// Export a Google Doc as markdown
    ///
    /// # Arguments
    /// * `doc_id` - Google Docs document ID
    /// * `token` - Google OAuth token
    ///
    /// # Returns
    /// * `Ok(String)` with markdown content
    /// * `Err(IrieBookError)` if export fails
    async fn export_as_markdown(&self, doc_id: &str, token: &str) -> Result<String, IrieBookError>;
}

