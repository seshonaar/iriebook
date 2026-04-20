//! Diff Manager - Orchestrates diff workflow
//!
//! Coordinates DiffSourceAccess and DifferEngine to produce diff results.
//! Follows Righting Software Method - zero business logic, pure orchestration.

use crate::engines::traits::DifferEngine;
use crate::resource_access::diff_source::DiffSourceAccess;
use crate::utilities::diff_trimmer::{ContextConfig, trim_segments_with_context};
use crate::utilities::error::IrieBookError;
use crate::utilities::types::{
    DiffComparison, DiffRequest, DiffSourceId, DisplayName, SegmentType, WordChangeStats,
};
use std::path::Path;
use std::sync::Arc;

/// Manager for orchestrating diff operations
///
/// Coordinates fetching content from sources and computing diffs.
/// Follows the Manager pattern: orchestrates workflow, delegates logic to engines.
#[derive(Clone)]
pub struct DiffManager {
    source_access: Arc<dyn DiffSourceAccess>,
    differ: Arc<dyn DifferEngine>,
}

impl DiffManager {
    /// Create new DiffManager with injected dependencies
    ///
    /// # Arguments
    /// * `source_access` - Trait object for accessing file sources
    /// * `differ` - Trait object for computing diffs
    pub fn new(source_access: Arc<dyn DiffSourceAccess>, differ: Arc<dyn DifferEngine>) -> Self {
        Self {
            source_access,
            differ,
        }
    }

    /// Compare two sources and produce diff result
    ///
    /// # Arguments
    /// * `request` - DiffRequest with source identifiers and display names
    ///
    /// # Returns
    /// * `Ok(DiffComparison)` - Complete diff with metadata
    /// * `Err(IrieBookError)` - If source access or diffing fails
    ///
    /// # Workflow
    /// 1. Get content from left source
    /// 2. Get content from right source
    /// 3. Compute diff using differ engine
    /// 4. Return structured result with display names
    pub async fn compare(&self, request: &DiffRequest) -> Result<DiffComparison, IrieBookError> {
        let relative_path = Path::new(&request.relative_path);

        // Stage 1: Get content from left source
        let left_content = self
            .source_access
            .get_content(&request.left_source, relative_path)
            .await?;

        // Stage 2: Get content from right source
        let right_content = self
            .source_access
            .get_content(&request.right_source, relative_path)
            .await?;

        // Stage 3: Compute diff (offload to blocking thread)
        let differ = self.differ.clone();
        // Clone strings to move into closure
        let left = left_content.clone();
        let right = right_content.clone();

        let diff = tokio::task::spawn_blocking(move || differ.diff(&left, &right))
            .await
            .map_err(|e| IrieBookError::Diff(format!("Join error: {}", e)))??;

        // Stage 4: Return structured result
        Ok(DiffComparison {
            left_display_name: request.left_display_name.clone(),
            right_display_name: request.right_display_name.clone(),
            diff,
        })
    }

    /// Compares two sources and returns a diff with trimmed context
    ///
    /// Unlike `compare()`, this trims large unchanged blocks to only keep
    /// context around changes. Useful for comparing large files like books.
    ///
    /// # Arguments
    /// * `request` - DiffRequest with source identifiers and display names
    /// * `context_config` - Configuration for how much context to keep
    ///
    /// # Returns
    /// * `Ok(DiffComparison)` - Complete diff with trimmed segments
    /// * `Err(IrieBookError)` - If source access or diffing fails
    ///
    /// # Workflow
    /// 1. Get full diff using compare()
    /// 2. Trim unchanged segments to context windows
    /// 3. Recalculate stats based on trimmed segments
    /// 4. Return optimized result
    pub async fn compare_with_context(
        &self,
        request: &DiffRequest,
        context_config: ContextConfig,
    ) -> Result<DiffComparison, IrieBookError> {
        // Get the full diff (unchanged)
        let mut comparison = self.compare(request).await?;

        // Trim segments to context windows
        comparison.diff.segments =
            trim_segments_with_context(comparison.diff.segments, context_config);

        // Update stats to reflect trimmed segments
        let mut stats = WordChangeStats {
            added: 0,
            removed: 0,
            unchanged: 0,
        };

        for segment in &comparison.diff.segments {
            match segment.segment_type {
                SegmentType::Added => stats.added += 1,
                SegmentType::Removed => stats.removed += 1,
                SegmentType::Unchanged => stats.unchanged += 1,
            }
        }

        comparison.diff.stats = stats;

        Ok(comparison)
    }

    /// Get all changed files in a git revision with their diffs
    ///
    /// # Arguments
    /// * `revision` - Git revision (commit hash, HEAD, etc.)
    /// * `filter_extension` - Optional file extension filter (e.g., Some(".md"))
    ///
    /// # Returns
    /// * `Ok(Vec<(String, DiffComparison)>)` - List of (file_path, diff) tuples
    /// * `Err(IrieBookError)` - If revision not found or operation fails
    ///
    /// # Business Logic
    /// This method abstracts the complete workflow for viewing revision changes:
    /// 1. Get list of changed files in the revision
    /// 2. Filter by extension if requested
    /// 3. Compute diff for each file (revision vs parent)
    /// 4. Return collection ready for UI display
    pub async fn get_revision_changes(
        &self,
        revision: &str,
        filter_extension: Option<&str>,
    ) -> Result<Vec<(String, DiffComparison)>, IrieBookError> {
        // Stage 1: Get list of changed files using source_access
        let changed_files = self.source_access.get_changed_files(revision).await?;

        // Stage 2: Filter by extension if provided
        let files_to_diff: Vec<String> = if let Some(ext) = filter_extension {
            changed_files
                .into_iter()
                .filter(|f| f.ends_with(ext))
                .collect()
        } else {
            changed_files
        };

        // Stage 3: For each file, compute diff against parent with context trimming
        let mut results = Vec::new();
        let short_hash = if revision.len() >= 7 {
            &revision[..7]
        } else {
            revision
        };

        // Use context-aware comparison for book files (20 words default)
        let context_config = ContextConfig::default();

        for file_path in files_to_diff {
            let request = DiffRequest {
                left_source: DiffSourceId(format!("{}~1", revision)),
                left_display_name: DisplayName(format!("Previous ({})", short_hash)),
                right_source: DiffSourceId(revision.to_string()),
                right_display_name: DisplayName(format!("Current ({})", short_hash)),
                relative_path: file_path.clone(),
            };

            // Use trimmed comparison to reduce payload size
            let comparison = self.compare_with_context(&request, context_config).await?;
            results.push((file_path, comparison));
        }

        Ok(results)
    }

    /// Get all uncommitted files with their diffs (working directory vs HEAD)
    ///
    /// # Arguments
    /// * `filter_extension` - Optional file extension filter (e.g., Some(".md"))
    ///
    /// # Returns
    /// * `Ok(Vec<(String, DiffComparison)>)` - List of (file_path, diff) tuples
    /// * `Err(IrieBookError)` - If operation fails or not a git repository
    ///
    /// # Business Logic
    /// This method abstracts the complete workflow for viewing local uncommitted changes:
    /// 1. Get list of uncommitted files
    /// 2. Filter by extension if requested
    /// 3. Compute diff for each file (HEAD vs working directory)
    /// 4. Return collection ready for UI display
    pub async fn get_local_changes(
        &self,
        filter_extension: Option<&str>,
    ) -> Result<Vec<(String, DiffComparison)>, IrieBookError> {
        // Stage 1: Get list of uncommitted files as (absolute_path, relative_path) tuples
        let changed_files = self.source_access.get_uncommitted_files().await?;

        // Stage 2: Filter by extension if provided
        let files_to_diff: Vec<(String, String)> = if let Some(ext) = filter_extension {
            changed_files
                .into_iter()
                .filter(|(_, rel_path)| rel_path.ends_with(ext))
                .collect()
        } else {
            changed_files
        };

        // Stage 3: For each file, compute diff HEAD vs working directory
        let mut results = Vec::new();
        let context_config = ContextConfig::default();

        for (abs_path, rel_path) in files_to_diff {
            let request = DiffRequest {
                left_source: DiffSourceId("HEAD".to_string()),
                left_display_name: DisplayName("HEAD".to_string()),
                right_source: DiffSourceId(abs_path), // Absolute path for file reading
                right_display_name: DisplayName("Working Directory".to_string()),
                relative_path: rel_path.clone(), // Relative path for git lookup
            };

            // Use trimmed comparison to reduce payload size
            let comparison = self.compare_with_context(&request, context_config).await?;
            results.push((rel_path, comparison)); // Return relative path for UI
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::comparison::differ::Differ;
    use crate::resource_access::diff_source::DiffSource;
    use crate::utilities::types::{DiffSourceId, DisplayName};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Helper to create test files
    fn create_test_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let file_path = dir.join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    // Helper to init git repo
    fn init_git_repo_with_file(dir: &std::path::Path, file_name: &str, content: &str) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("Failed to init git");

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

        create_test_file(dir, file_name, content);

        Command::new("git")
            .args(["add", file_name])
            .current_dir(dir)
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[tokio::test]
    async fn diff_manager_preserves_display_names() {
        let temp = TempDir::new().unwrap();
        let left_path = create_test_file(temp.path(), "left.txt", "hello");
        let right_path = create_test_file(temp.path(), "right.txt", "world");

        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId(left_path.display().to_string()),
            left_display_name: DisplayName("Left Side".to_string()),
            right_source: DiffSourceId(right_path.display().to_string()),
            right_display_name: DisplayName("Right Side".to_string()),
            relative_path: String::new(),
        };

        let result = manager.compare(&request).await.unwrap();

        assert_eq!(result.left_display_name.0, "Left Side");
        assert_eq!(result.right_display_name.0, "Right Side");
    }

    #[tokio::test]
    async fn diff_manager_file_to_file() {
        let temp = TempDir::new().unwrap();
        let left_path = create_test_file(temp.path(), "left.txt", "original text");
        let right_path = create_test_file(temp.path(), "right.txt", "modified text");

        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId(left_path.display().to_string()),
            left_display_name: DisplayName("Original".to_string()),
            right_source: DiffSourceId(right_path.display().to_string()),
            right_display_name: DisplayName("Modified".to_string()),
            relative_path: String::new(),
        };

        let result = manager.compare(&request).await.unwrap();

        // Should detect changes
        assert!(result.diff.stats.added > 0 || result.diff.stats.removed > 0);
        assert!(result.diff.segments.len() > 0);
    }

    #[tokio::test]
    async fn diff_manager_git_revision_to_git_revision() {
        let temp = TempDir::new().unwrap();

        // Create git history with two commits
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

        // First commit
        create_test_file(temp.path(), "test.txt", "old content");
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "First commit"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Second commit with changed content
        fs::write(temp.path().join("test.txt"), "new content").unwrap();
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Second commit"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId("HEAD~1".to_string()),
            left_display_name: DisplayName("Previous Commit".to_string()),
            right_source: DiffSourceId("HEAD".to_string()),
            right_display_name: DisplayName("Current Commit".to_string()),
            relative_path: "test.txt".to_string(),
        };

        let result = manager.compare(&request).await.unwrap();

        // Should detect changes between HEAD~1 and HEAD
        assert!(result.diff.stats.added > 0 || result.diff.stats.removed > 0);
    }

    #[tokio::test]
    async fn diff_manager_errors_on_missing_file() {
        let temp = TempDir::new().unwrap();
        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId("/nonexistent/left.txt".to_string()),
            left_display_name: DisplayName("Left".to_string()),
            right_source: DiffSourceId("/nonexistent/right.txt".to_string()),
            right_display_name: DisplayName("Right".to_string()),
            relative_path: String::new(),
        };

        let result = manager.compare(&request).await;

        assert!(matches!(result, Err(IrieBookError::FileRead { .. })));
    }

    #[tokio::test]
    async fn diff_manager_identical_files_no_changes() {
        let temp = TempDir::new().unwrap();
        let content = "identical content";
        let left_path = create_test_file(temp.path(), "left.txt", content);
        let right_path = create_test_file(temp.path(), "right.txt", content);

        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId(left_path.display().to_string()),
            left_display_name: DisplayName("Left".to_string()),
            right_source: DiffSourceId(right_path.display().to_string()),
            right_display_name: DisplayName("Right".to_string()),
            relative_path: String::new(),
        };

        let result = manager.compare(&request).await.unwrap();

        // Should have no additions or removals
        assert_eq!(result.diff.stats.added, 0);
        assert_eq!(result.diff.stats.removed, 0);
        assert!(result.diff.stats.unchanged > 0);
    }

    #[tokio::test]
    async fn diff_manager_respects_relative_path_for_git() {
        let temp = TempDir::new().unwrap();
        init_git_repo_with_file(temp.path(), "manuscript.md", "chapter one");

        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId("HEAD".to_string()),
            left_display_name: DisplayName("HEAD".to_string()),
            right_source: DiffSourceId("HEAD".to_string()),
            right_display_name: DisplayName("HEAD (again)".to_string()),
            relative_path: "manuscript.md".to_string(),
        };

        let result = manager.compare(&request).await.unwrap();

        // Same commit, same file - should be identical
        assert_eq!(result.diff.stats.added, 0);
        assert_eq!(result.diff.stats.removed, 0);
    }

    #[tokio::test]
    async fn diff_manager_git_to_file() {
        // CRITICAL TEST: Compare committed version (HEAD) to working directory (uncommitted)
        // This is the MOST common use case for diff view!
        let temp = TempDir::new().unwrap();

        // Create git repo with committed file
        init_git_repo_with_file(temp.path(), "manuscript.md", "Chapter one\n\nOld content");

        // Modify file in working directory (uncommitted changes)
        fs::write(
            temp.path().join("manuscript.md"),
            "Chapter one\n\nNew content with edits",
        )
        .unwrap();

        // DiffSource can handle BOTH git revisions and file paths
        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        let request = DiffRequest {
            left_source: DiffSourceId("HEAD".to_string()), // Git revision
            left_display_name: DisplayName("Last Commit".to_string()),
            right_source: DiffSourceId(temp.path().join("manuscript.md").display().to_string()), // File path
            right_display_name: DisplayName("Working Directory".to_string()),
            relative_path: "manuscript.md".to_string(),
        };

        let result = manager.compare(&request).await.unwrap();

        // Should detect changes between committed and uncommitted versions
        assert!(result.diff.stats.added > 0 || result.diff.stats.removed > 0);
        assert!(result.diff.segments.len() > 0);

        // Display names should be preserved
        assert_eq!(result.left_display_name.0, "Last Commit");
        assert_eq!(result.right_display_name.0, "Working Directory");
    }

    #[tokio::test]
    async fn diff_manager_get_local_changes() {
        let temp = TempDir::new().unwrap();

        // Create git repo with committed files
        init_git_repo_with_file(temp.path(), "committed.md", "Original content");

        // Add another file and commit it
        create_test_file(temp.path(), "another.md", "Another file");
        Command::new("git")
            .args(["add", "another.md"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Add another file"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Modify the first file (uncommitted change)
        fs::write(
            temp.path().join("committed.md"),
            "Modified content with local changes",
        )
        .unwrap();

        let source = Arc::new(DiffSource::new(temp.path().to_path_buf()));
        let differ = Arc::new(Differ);
        let manager = DiffManager::new(source, differ);

        // Get local changes - should find the modified .md file
        let changes = manager.get_local_changes(Some(".md")).await.unwrap();

        // Should find committed.md with uncommitted changes
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].0, "committed.md");

        // Verify the diff shows changes
        let comparison = &changes[0].1;
        assert!(comparison.diff.stats.added > 0 || comparison.diff.stats.removed > 0);
        assert_eq!(comparison.left_display_name.0, "HEAD");
        assert_eq!(comparison.right_display_name.0, "Working Directory");
    }
}
