//! Diff source abstraction for accessing file content from different sources
//!
//! Provides trait-based abstraction for getting file content from:
//! - Direct filesystem paths (absolute file paths)
//! - Git revisions (HEAD, HEAD~N, commit hashes, branch names)
//!
//! The source type is automatically detected based on the source_id format.

use crate::utilities::error::IrieBookError;
use crate::utilities::types::DiffSourceId;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Trait for accessing file content from different sources
///
/// Implementations provide content from various sources (filesystem, git, etc.)
#[async_trait]
pub trait DiffSourceAccess: Send + Sync {
    /// Get file content from source
    ///
    /// # Arguments
    /// * `source_id` - Identifier (file path or git revision)
    /// * `relative_path` - Relative path to file within source (used for git revisions)
    ///
    /// # Returns
    /// * `Ok(String)` - File content as UTF-8
    /// * `Err(IrieBookError)` - If source not found or content cannot be read
    async fn get_content(
        &self,
        source_id: &DiffSourceId,
        relative_path: &Path,
    ) -> Result<String, IrieBookError>;

    /// Get list of changed files in a git revision
    ///
    /// # Arguments
    /// * `revision` - Git revision (commit hash, HEAD, HEAD~1, etc.)
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of relative file paths changed in the revision
    /// * `Err(IrieBookError)` - If revision not found or operation fails
    async fn get_changed_files(&self, revision: &str) -> Result<Vec<String>, IrieBookError>;

    /// Get list of files with uncommitted changes in working directory
    ///
    /// # Returns
    /// * `Ok(Vec<(String, String)>)` - List of (absolute_path, relative_path) tuples with uncommitted changes
    /// * `Err(IrieBookError)` - If operation fails or not a git repository
    async fn get_uncommitted_files(&self) -> Result<Vec<(String, String)>, IrieBookError>;
}

/// Diff source implementation that handles both filesystem and git sources
///
/// Automatically detects the source type based on the source_id format:
/// - If source_id looks like a git revision (HEAD, HEAD~N, commit hash), reads from git
/// - Otherwise treats it as a file path and reads from filesystem
///
/// This allows mixing different source types in a single comparison
/// (e.g., comparing working directory file to a git commit)
pub struct DiffSource {
    repo_path: PathBuf,
}

impl DiffSource {
    /// Create new DiffSource for a repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository root
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Detect if a source_id is a git revision
    ///
    /// Heuristics:
    /// - Starts with "HEAD" (HEAD, HEAD~1, etc.)
    /// - Contains "~" or "^" (relative refs)
    /// - Is a 7-40 character hex string (commit hash)
    fn is_git_revision(source_id: &str) -> bool {
        // HEAD references
        if source_id.starts_with("HEAD") {
            return true;
        }

        // Relative refs with ~ or ^
        if source_id.contains('~') || source_id.contains('^') {
            return true;
        }

        // Commit hash (7-40 hex characters)
        if source_id.len() >= 7
            && source_id.len() <= 40
            && source_id.chars().all(|c| c.is_ascii_hexdigit())
        {
            return true;
        }

        // Otherwise assume it's a file path
        false
    }

    /// Read content from git revision
    fn read_from_git(
        &self,
        source_id: &DiffSourceId,
        relative_path: &Path,
    ) -> Result<String, IrieBookError> {
        let revision = &source_id.0;

        // Open repository using gitoxide
        let repo = gix::open(&self.repo_path)
            .map_err(|e| IrieBookError::Git(format!("Failed to open repository: {}", e)))?;

        // Resolve revision to commit ID (handles HEAD, HEAD~1, hashes, branch names)
        let commit_id = repo
            .rev_parse_single(revision.as_str())
            .map_err(|e| IrieBookError::GitRevisionNotFound(format!("'{}': {}", revision, e)))?;

        let commit = repo
            .find_object(commit_id)
            .map_err(|e| IrieBookError::Git(format!("Failed to find commit: {}", e)))?
            .try_into_commit()
            .map_err(|_| IrieBookError::Git("Not a commit".to_string()))?;

        // Get tree from commit
        let tree_id = commit
            .tree_id()
            .map_err(|e| IrieBookError::Git(format!("Failed to get tree: {}", e)))?;

        let tree = repo
            .find_object(tree_id)
            .map_err(|e| IrieBookError::Git(format!("Failed to find tree: {}", e)))?
            .try_into_tree()
            .map_err(|_| IrieBookError::Git("Not a tree".to_string()))?;

        // Find file in tree using relative path
        let entry = tree
            .lookup_entry_by_path(relative_path)
            .map_err(|e| IrieBookError::Git(format!("Failed to lookup path: {}", e)))?
            .ok_or_else(|| IrieBookError::FileNotFoundInRevision {
                file: relative_path.display().to_string(),
                revision: revision.clone(),
            })?;

        // Get blob content
        let blob = repo
            .find_object(entry.oid())
            .map_err(|e| IrieBookError::Git(format!("Failed to find blob: {}", e)))?
            .try_into_blob()
            .map_err(|_| IrieBookError::Git("Not a blob".to_string()))?;

        // Convert to UTF-8 string (with proper error handling)
        String::from_utf8(blob.data.to_vec()).map_err(|_| IrieBookError::InvalidUtf8InGitBlob)
    }
}

#[async_trait]
impl DiffSourceAccess for DiffSource {
    async fn get_content(
        &self,
        source_id: &DiffSourceId,
        relative_path: &Path,
    ) -> Result<String, IrieBookError> {
        if Self::is_git_revision(&source_id.0) {
            // Read from git - wrap blocking gix operations
            let repo_path = self.repo_path.clone();
            let source_id = source_id.clone();
            let relative_path = relative_path.to_path_buf();

            // We need to clone specific fields or the whole struct if it's cheap (PathBuf is fine)
            // But we can't pass &self to spawn_blocking. 
            // We'll create a temporary DiffSource or just call logic that doesn't need &self if possible.
            // Actually read_from_git uses self.repo_path.
            tokio::task::spawn_blocking(move || {
                let source = DiffSource::new(repo_path);
                source.read_from_git(&source_id, &relative_path)
            })
            .await
            .map_err(|e| IrieBookError::Git(format!("Join error: {}", e)))?
        } else {
            // Read from filesystem - async
            let path = PathBuf::from(&source_id.0);

            // Read file with proper error handling
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| IrieBookError::FileRead {
                    path: source_id.0.clone(),
                    source: e,
                })
                // Strip UTF-8 BOM if present
                .map(|content| {
                    const BOM: char = '\u{FEFF}';
                    if content.starts_with(BOM) {
                        content.strip_prefix(BOM).unwrap_or(&content).to_string()
                    } else {
                        content
                    }
                })
        }
    }

    async fn get_changed_files(&self, revision: &str) -> Result<Vec<String>, IrieBookError> {
        use tokio::process::Command;

        // Use git diff-tree to get changed files
        let output = Command::new("git")
            .args([
                "-c",
                "core.quotePath=false",
                "diff-tree",
                "--no-commit-id",
                "--name-only",
                "-r",
                revision,
            ])
            .current_dir(&self.repo_path)
            .output()
            .await
            .map_err(|e| IrieBookError::Git(format!("Failed to execute git diff-tree: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(IrieBookError::Git(format!(
                "Failed to get changed files: {}",
                stderr
            )));
        }

        // Parse the output - one file path per line
        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_matches(|c: char| c == '\"').to_string())
            .collect();

        Ok(files)
    }

    async fn get_uncommitted_files(&self) -> Result<Vec<(String, String)>, IrieBookError> {
        use crate::resource_access::git::GitClient;
        use crate::resource_access::traits::GitAccess;

        let git_client = GitClient;

        // Get all changed files (uncommitted) - returns absolute paths
        let changed_files = git_client.get_all_changed_files(&self.repo_path)?;

        // Convert to (absolute_path, relative_path) tuples
        let file_paths: Vec<(String, String)> = changed_files
            .into_iter()
            .filter_map(|abs_path| {
                let abs_str = abs_path.to_string_lossy().to_string();
                abs_path
                    .strip_prefix(&self.repo_path)
                    .ok()
                    .map(|rel| {
                        let rel_str = rel.to_string_lossy().to_string();
                        (abs_str, rel_str)
                    })
            })
            .collect();

        Ok(file_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Helper to create a test file
    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    // Helper to initialize a git repository with a commit
    fn init_git_repo_with_file(dir: &Path, file_name: &str, content: &str) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .output()
            .expect("Failed to set git email");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .expect("Failed to set git name");

        create_test_file(dir, file_name, content);

        Command::new("git")
            .args(["add", file_name])
            .current_dir(dir)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir)
            .output()
            .expect("Failed to git commit");
    }

    // Helper to create multiple commits
    fn create_git_history(dir: &Path, commits: Vec<(&str, &str)>) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .unwrap();

        for (file_name, content) in commits {
            fs::write(dir.join(file_name), content).unwrap();
            Command::new("git")
                .args(["add", file_name])
                .current_dir(dir)
                .output()
                .unwrap();
            Command::new("git")
                .args(["commit", "-m", &format!("Commit {}", content)])
                .current_dir(dir)
                .output()
                .unwrap();
        }
    }

    // Filesystem tests
    #[tokio::test]
    async fn diff_source_reads_existing_file() {
        let temp = TempDir::new().unwrap();
        let file_path = create_test_file(temp.path(), "test.txt", "hello world");

        let source = DiffSource::new(temp.path().to_path_buf());
        let id = DiffSourceId(file_path.display().to_string());
        let content = source.get_content(&id, Path::new("")).await.unwrap();

        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn diff_source_errors_on_missing_file() {
        let source = DiffSource::new(PathBuf::from("/tmp"));
        let id = DiffSourceId("/nonexistent/file.txt".to_string());
        let result = source.get_content(&id, Path::new("")).await;

        assert!(matches!(result, Err(IrieBookError::FileRead { .. })));
    }

    #[tokio::test]
    async fn diff_source_reads_multiline_content() {
        let temp = TempDir::new().unwrap();
        let content = "Line one\nLine two\nLine three";
        let file_path = create_test_file(temp.path(), "multiline.txt", content);

        let source = DiffSource::new(temp.path().to_path_buf());
        let id = DiffSourceId(file_path.display().to_string());
        let result = source.get_content(&id, Path::new("")).await.unwrap();

        assert_eq!(result, content);
    }

    // Git revision tests
    #[tokio::test]
    async fn diff_source_reads_from_head() {
        let temp = TempDir::new().unwrap();
        init_git_repo_with_file(temp.path(), "test.txt", "content from HEAD");

        let source = DiffSource::new(temp.path().to_path_buf());
        let id = DiffSourceId("HEAD".to_string());
        let content = source.get_content(&id, Path::new("test.txt")).await.unwrap();

        assert_eq!(content, "content from HEAD");
    }

    #[tokio::test]
    async fn diff_source_supports_relative_refs() {
        let temp = TempDir::new().unwrap();
        create_git_history(temp.path(), vec![("test.txt", "v1"), ("test.txt", "v2")]);

        let source = DiffSource::new(temp.path().to_path_buf());

        // HEAD should be v2
        let id_head = DiffSourceId("HEAD".to_string());
        let content_head = source.get_content(&id_head, Path::new("test.txt")).await.unwrap();
        assert_eq!(content_head, "v2");

        // HEAD~1 should be v1
        let id_prev = DiffSourceId("HEAD~1".to_string());
        let content_prev = source.get_content(&id_prev, Path::new("test.txt")).await.unwrap();
        assert_eq!(content_prev, "v1");
    }

    #[tokio::test]
    async fn diff_source_errors_on_invalid_revision() {
        let temp = TempDir::new().unwrap();
        init_git_repo_with_file(temp.path(), "test.txt", "content");

        let source = DiffSource::new(temp.path().to_path_buf());
        // Use HEAD~999 which looks like a git revision but doesn't exist
        let id = DiffSourceId("HEAD~999".to_string());
        let result = source.get_content(&id, Path::new("test.txt")).await;

        assert!(matches!(result, Err(IrieBookError::GitRevisionNotFound(_))));
    }

    #[tokio::test]
    async fn diff_source_errors_on_file_not_found_in_revision() {
        let temp = TempDir::new().unwrap();
        init_git_repo_with_file(temp.path(), "existing.txt", "content");

        let source = DiffSource::new(temp.path().to_path_buf());
        let id = DiffSourceId("HEAD".to_string());
        let result = source.get_content(&id, Path::new("nonexistent.txt")).await;

        assert!(matches!(
            result,
            Err(IrieBookError::FileNotFoundInRevision { .. })
        ));
    }

    #[tokio::test]
    async fn diff_source_reads_multiline_git_content() {
        let temp = TempDir::new().unwrap();
        let content = "Line one\nLine two\nLine three";
        init_git_repo_with_file(temp.path(), "multiline.txt", content);

        let source = DiffSource::new(temp.path().to_path_buf());
        let id = DiffSourceId("HEAD".to_string());
        let result = source.get_content(&id, Path::new("multiline.txt")).await.unwrap();

        assert_eq!(result, content);
    }

    #[tokio::test]
    async fn diff_source_handles_subdirectories() {
        let temp = TempDir::new().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Create subdirectory and file
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("nested.txt"), "nested content").unwrap();

        Command::new("git")
            .args(["add", "subdir/nested.txt"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Add nested file"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let source = DiffSource::new(temp.path().to_path_buf());
        let id = DiffSourceId("HEAD".to_string());
        let content = source
            .get_content(&id, Path::new("subdir/nested.txt"))
            .await
            .unwrap();

        assert_eq!(content, "nested content");
    }

    // Detection tests
    #[test]
    fn diff_source_detects_head_revision() {
        assert!(DiffSource::is_git_revision("HEAD"));
        assert!(DiffSource::is_git_revision("HEAD~1"));
        assert!(DiffSource::is_git_revision("HEAD~10"));
        assert!(DiffSource::is_git_revision("HEAD^"));
    }

    #[test]
    fn diff_source_detects_commit_hashes() {
        assert!(DiffSource::is_git_revision("abc1234")); // short hash
        assert!(DiffSource::is_git_revision(
            "2d07002a1b3c4d5e6f7890abcdef12345678"
        )); // full hash
        assert!(DiffSource::is_git_revision("DEADBEEF")); // uppercase hex
    }

    #[test]
    fn diff_source_detects_relative_refs() {
        assert!(DiffSource::is_git_revision("main~3"));
        assert!(DiffSource::is_git_revision("feature^2"));
    }

    #[test]
    fn diff_source_treats_paths_as_files() {
        assert!(!DiffSource::is_git_revision("/tmp/test.txt"));
        assert!(!DiffSource::is_git_revision("./relative/path.md"));
        assert!(!DiffSource::is_git_revision("C:\\Windows\\file.txt"));
        assert!(!DiffSource::is_git_revision("manuscript.md"));
    }

    // Mixed source test (THE CRITICAL ONE)
    #[tokio::test]
    async fn diff_source_handles_mixed_sources() {
        let temp = TempDir::new().unwrap();

        // Create git repo with committed file
        init_git_repo_with_file(temp.path(), "test.txt", "committed content");

        // Modify file in working directory (uncommitted)
        fs::write(temp.path().join("test.txt"), "modified content").unwrap();

        let source = DiffSource::new(temp.path().to_path_buf());

        // Read from git (HEAD)
        let git_id = DiffSourceId("HEAD".to_string());
        let git_content = source.get_content(&git_id, Path::new("test.txt")).await.unwrap();
        assert_eq!(git_content, "committed content");

        // Read from file (working directory)
        let file_id = DiffSourceId(temp.path().join("test.txt").display().to_string());
        let file_content = source.get_content(&file_id, Path::new("")).await.unwrap();
        assert_eq!(file_content, "modified content");
    }

    #[tokio::test]
    async fn diff_source_handles_utf8_filenames() {
        let temp = TempDir::new().unwrap();
        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Create file with UTF-8 name
        let filename = "test_ă.txt";
        create_test_file(temp.path(), filename, "content");

        Command::new("git")
            .args(["add", filename])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Make a change
        fs::write(temp.path().join(filename), "changed content").unwrap();
        Command::new("git")
            .args(["add", filename])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "changed"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let source = DiffSource::new(temp.path().to_path_buf());
        let changed = source.get_changed_files("HEAD").await.unwrap();

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], filename);
    }
}
