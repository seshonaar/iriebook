//! Git operations using gitoxide
//!
//! This module provides git version control operations using the gitoxide library.
//! Phase 1 implementation - basic operations only, complex ones will be implemented in Phase 2.

use crate::resource_access::traits::GitAccess;
use crate::utilities::error::IrieBookError;
use crate::utilities::types::{GitCommit, GitStatus};
use anyhow::{Context, Result};
use gix::bstr::BString;
use gix::bstr::ByteSlice;
use gix::status::index_worktree::iter::Summary;
use std::path::Path;

/// Git client implementation using gitoxide
pub struct GitClient;

impl GitAccess for GitClient {
    fn clone_repository(&self, url: &str, path: &Path, token: &str) -> Result<(), IrieBookError> {
        // Inject token into URL for authentication
        // Format: https://oauth2:TOKEN@github.com/user/repo.git
        let auth_url = if let Some(stripped) = url.strip_prefix("https://") {
            // URL-encode the token to handle special characters
            let encoded_token = urlencoding::encode(token);
            format!("https://oauth2:{}@{}", encoded_token, stripped)
        } else {
            url.to_string()
        };

        // Prepare the clone operation
        let mut prepare = gix::prepare_clone(auth_url, path)
            .map_err(|e| IrieBookError::Git(format!("Failed to prepare clone: {}", e)))?;

        // Perform the fetch and the initial checkout
        let (mut checkout, _outcome) = prepare
            .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| IrieBookError::Git(format!("Failed to fetch: {}", e)))?;

        // Finalize the worktree (files on disk)
        let (_index, _outcome) = checkout
            .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| IrieBookError::Git(format!("Failed to checkout: {}", e)))?;

        Ok(())
    }

    fn get_remote_url(&self, repo_path: &Path) -> Result<String, IrieBookError> {
        let repo = gix::open(repo_path)
            .map_err(|e| IrieBookError::Git(format!("Failed to open repository: {}", e)))?;

        let remote = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .ok_or_else(|| IrieBookError::Git("No default remote configured".to_string()))?
            .map_err(|e| IrieBookError::Git(format!("Failed to find remote: {}", e)))?;

        let url = remote
            .url(gix::remote::Direction::Fetch)
            .ok_or_else(|| IrieBookError::Git("No fetch URL configured".to_string()))?;

        Ok(url.to_string())
    }

    fn is_repository(&self, path: &Path) -> bool {
        gix::discover(path).is_ok()
    }

    fn add_all(&self, repo_path: &std::path::Path) -> Result<(), IrieBookError> {
        // 1. Open the repo
        let repo = gix::open(repo_path)
            .context("Failed to open repository")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // 2. Get the index, or create a new empty one if it doesn't exist
        // We need an owned mutable index to modify it.
        let mut index = (*repo
            .index_or_empty()
            .context("Failed to load or create index")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?)
        .clone();

        // 3. Get Status - configure to show individual files (not collapsed directories)
        // This ensures .gitignore is properly respected for all files
        let status = repo
            .status(gix::progress::Discard)
            .context("Failed to initialize status")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?
            .untracked_files(gix::status::UntrackedFiles::Files);

        let outcomes = status
            .into_index_worktree_iter(Vec::<BString>::new())
            .context("Failed to get status iterator")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        for outcome in outcomes {
            // Fix E0282/E0412: Since we use anyhow, we don't need the explicit Error type
            let change = outcome
                .context("Error during status iteration")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            // In 0.77, change is a gix::status::index_worktree::Item
            // We get the relative path and the summary of what happened
            let path = change.rela_path().to_owned();
            let summary = change
                .summary()
                .context("Failed to compute summary for path")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            match summary {
                Summary::Removed => {
                    if let Some(idx) = index.entry_index_by_path_and_stage(
                        path.as_bstr(),
                        gix::index::entry::Stage::Unconflicted,
                    ) {
                        index.remove_entry_at_index(idx);
                    }
                }
                Summary::Added | Summary::Modified | Summary::TypeChange => {
                    use gix::index::entry::stat::Time;
                    use gix::index::entry::{Flags, Mode, Stat};

                    let path_str = path.to_string();
                    let full_path = repo_path.join(&path_str);

                    // Check if path exists and get metadata
                    let metadata = match std::fs::metadata(&full_path) {
                        Ok(m) => m,
                        Err(_) => continue, // File doesn't exist (maybe deleted), skip it
                    };

                    // Skip directories - status with UntrackedFiles::Files will enumerate
                    // individual files within directories, so we don't need to handle dirs
                    if metadata.is_dir() {
                        continue;
                    }

                    // Add the file
                    let content = std::fs::read(&full_path)
                        .context("Failed to read file content")
                        .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

                    let blob_id = repo
                        .write_blob(&content)
                        .context("Failed to write blob")
                        .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

                    // Manually construct Stat
                    let mtime = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
                    let ctime = metadata.created().unwrap_or(std::time::UNIX_EPOCH);
                    let size = metadata.len() as u32;

                    let stat = Stat {
                        mtime: Time::try_from(mtime).unwrap_or_default(),
                        ctime: Time::try_from(ctime).unwrap_or_default(),
                        dev: 0,
                        ino: 0,
                        uid: 0,
                        gid: 0,
                        size,
                    };

                    // Simplified mode handling
                    let mode = Mode::FILE;

                    // Remove existing entry if any
                    if let Some(idx) = index.entry_index_by_path_and_stage(
                        path.as_bstr(),
                        gix::index::entry::Stage::Unconflicted,
                    ) {
                        index.remove_entry_at_index(idx);
                    }

                    index.dangerously_push_entry(
                        stat,
                        blob_id.into(),
                        Flags::empty(),
                        mode,
                        path.as_bstr(),
                    );
                }
                _ => {} // Ignore Unchanged, IntentToAdd, etc.
            }
        }

        // Safety net: remove any index entries whose files no longer exist.
        // This covers rare cases where status iteration might miss deletions.
        let mut missing_indices = Vec::new();
        for (idx, entry) in index.entries().iter().enumerate() {
            let path = entry.path(&index);
            let full_path = repo_path.join(path.to_string());
            if !full_path.exists() {
                missing_indices.push(idx);
            }
        }

        for idx in missing_indices.into_iter().rev() {
            index.remove_entry_at_index(idx);
        }

        // Sort entries to restore invariant after dangerous pushes
        index.sort_entries();

        // 4. Write changes back to disk
        index
            .write(Default::default())
            .context("Failed to write index to disk")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        Ok(())
    }

    fn commit(&self, repo_path: &Path, message: &str) -> Result<String, IrieBookError> {
        let repo = gix::open(repo_path)
            .context("Failed to open repository")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // 1. Get the index
        let index = repo
            .index()
            .context("Failed to load index")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // 2. Create a tree from the index
        // We use the empty tree as a base and insert all entries from the index.
        let empty_tree = repo.empty_tree();
        let mut editor = empty_tree
            .edit()
            .context("Failed to create tree editor")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        for entry in index.entries() {
            let path = entry.path(&index);
            let mode = entry.mode;

            let kind = match mode {
                gix::index::entry::Mode::DIR => gix::object::tree::EntryKind::Tree,
                gix::index::entry::Mode::FILE => gix::object::tree::EntryKind::Blob,
                gix::index::entry::Mode::FILE_EXECUTABLE => {
                    gix::object::tree::EntryKind::BlobExecutable
                }
                gix::index::entry::Mode::SYMLINK => gix::object::tree::EntryKind::Link,
                gix::index::entry::Mode::COMMIT => gix::object::tree::EntryKind::Commit,
                _ => gix::object::tree::EntryKind::Blob,
            };

            editor
                .upsert(path, kind, entry.id)
                .context("Failed to add entry to tree")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;
        }

        let tree_id = editor
            .write()
            .context("Failed to write tree")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // 3. Commit
        let mut parents = Vec::new();
        let mut head_tree_id: Option<gix::ObjectId> = None;
        if let Ok(head_commit) = repo.head_commit() {
            if let Ok(tree) = head_commit.tree() {
                head_tree_id = Some(tree.id);
            }
            parents.push(head_commit.id);
        }

        // Prevent empty commits (same tree as HEAD)
        if head_tree_id.is_some_and(|id| id == tree_id) {
            return Err(IrieBookError::Git("Nothing to commit".to_string()));
        }

        let commit_id = repo
            .commit("HEAD", message, tree_id, parents)
            .context("Failed to commit")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        Ok(commit_id.to_string())
    }

    fn pull_rebase_ours(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        use std::process::Command;

        // Check if there's a stale rebase in progress and abort it
        let rebase_merge_path = repo_path.join(".git").join("rebase-merge");
        let rebase_apply_path = repo_path.join(".git").join("rebase-apply");

        if rebase_merge_path.exists() || rebase_apply_path.exists() {
            // Try to abort the stale rebase
            let abort_output = Command::new("git")
                .args(["rebase", "--abort"])
                .current_dir(repo_path)
                .output()
                .context("Failed to execute git rebase --abort")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            if !abort_output.status.success() {
                // If abort fails, forcefully remove the rebase directories
                if rebase_merge_path.exists() {
                    std::fs::remove_dir_all(&rebase_merge_path)
                        .context("Failed to remove stale rebase-merge directory")
                        .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;
                }
                if rebase_apply_path.exists() {
                    std::fs::remove_dir_all(&rebase_apply_path)
                        .context("Failed to remove stale rebase-apply directory")
                        .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;
                }
            }
        }

        // First, fetch from remote to get latest changes
        let fetch_output = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git fetch command")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !fetch_output.status.success() {
            let stderr = String::from_utf8_lossy(&fetch_output.stderr);
            return Err(IrieBookError::Git(format!("Fetch failed: {}", stderr)));
        }

        // Get current branch name
        let branch_output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(repo_path)
            .output()
            .context("Failed to get current branch")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !branch_output.status.success() {
            return Err(IrieBookError::Git(
                "Failed to determine current branch".to_string(),
            ));
        }

        let branch_name = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // If no branch (detached HEAD) or empty repo, nothing to rebase
        if branch_name.is_empty() {
            return Ok(());
        }

        // Check if remote branch exists
        let remote_branch = format!("origin/{}", branch_name);
        let check_remote = Command::new("git")
            .args(["rev-parse", "--verify", &remote_branch])
            .current_dir(repo_path)
            .output()
            .context("Failed to check remote branch")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // If remote branch doesn't exist (empty repo or first push), nothing to rebase
        if !check_remote.status.success() {
            return Ok(());
        }

        // Rebase current branch onto origin/<branch>
        // NOTE: During rebase, "theirs" means OUR local changes (what we're rebasing)
        // and "ours" means upstream changes (what we're rebasing onto)
        // So to keep our changes, we use -X theirs
        let rebase_output = Command::new("git")
            .args(["rebase", "-X", "theirs", &remote_branch])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git rebase command")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !rebase_output.status.success() {
            let stderr = String::from_utf8_lossy(&rebase_output.stderr);

            // If rebase fails, try to abort it to leave repo in clean state
            let _ = Command::new("git")
                .args(["rebase", "--abort"])
                .current_dir(repo_path)
                .output();

            return Err(IrieBookError::Git(format!("Rebase failed: {}", stderr)));
        }

        Ok(())
    }

    fn push(&self, repo_path: &Path, token: &str) -> Result<(), IrieBookError> {
        use std::process::Command;

        let output = if token.is_empty() {
            // No token - use for file:// or SSH remotes
            Command::new("git")
                .args(["push", "origin", "HEAD"])
                .current_dir(repo_path)
                .output()
                .context("Failed to execute git push command")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?
        } else {
            // With token - inject into HTTPS URL for authentication
            // Get the remote URL
            let remote_url = self.get_remote_url(repo_path)?;

            // Inject token into URL if it's HTTPS (with proper URL encoding)
            let auth_url = if let Some(stripped) = remote_url.strip_prefix("https://") {
                // Remove any existing authentication from the URL
                // (in case remote URL is already https://user:pass@github.com/...)
                let clean_url = if let Some(at_pos) = stripped.find('@') {
                    &stripped[at_pos + 1..]
                } else {
                    stripped
                };

                // URL-encode the token to handle special characters
                // GitHub tokens should be used as username with no password
                let encoded_token = urlencoding::encode(token);
                format!("https://{}@{}", encoded_token, clean_url)
            } else {
                // Not HTTPS, just use original URL (might be SSH or file://)
                remote_url
            };

            // Push using the authenticated URL
            Command::new("git")
                .args(["push", &auth_url, "HEAD"])
                .current_dir(repo_path)
                .output()
                .context("Failed to execute git push command")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(IrieBookError::Git(format!("Push failed: {}", stderr)));
        }

        Ok(())
    }

    fn get_log(&self, repo_path: &Path, limit: usize) -> Result<Vec<GitCommit>, IrieBookError> {
        let repo = gix::open(repo_path)
            .map_err(|e| IrieBookError::Git(format!("Failed to open repository: {}", e)))?;

        let head = repo
            .head_commit()
            .map_err(|e| IrieBookError::Git(format!("Failed to get HEAD commit: {}", e)))?;

        let mut commits = Vec::new();
        let mut current = Some(head);
        let mut count = 0;

        while let Some(commit) = current {
            if count >= limit {
                break;
            }

            // Get commit message (simple string conversion)
            let message_ref = commit
                .message_raw()
                .map_err(|e| IrieBookError::Git(format!("Invalid commit message: {}", e)))?;
            let message = String::from_utf8_lossy(message_ref.as_bytes()).to_string();

            // Get author information
            let author = commit
                .author()
                .map_err(|e| IrieBookError::Git(format!("Invalid commit author: {}", e)))?;

            let author_name = String::from_utf8_lossy(author.name.as_bytes()).to_string();

            commits.push(GitCommit {
                hash: commit.id.to_string(),
                message: message.trim().to_string(),
                author: author_name,
                timestamp: author
                    .time
                    .to_string()
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .to_string(),
            });

            // Get parent commit
            let parent_ids: Vec<_> = commit.parent_ids().collect();
            current = parent_ids.first().and_then(|id| {
                repo.find_object(*id)
                    .ok()
                    .and_then(|obj| obj.try_into_commit().ok())
            });

            count += 1;
        }

        Ok(commits)
    }

    fn get_status(&self, repo_path: &Path) -> Result<GitStatus, IrieBookError> {
        use std::process::Command;

        // Check for uncommitted changes
        let has_uncommitted = self.has_uncommitted_changes(repo_path)?;

        // Get current branch name
        let branch_output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(repo_path)
            .output()
            .context("Failed to get current branch")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !branch_output.status.success() {
            return Ok(GitStatus {
                ahead_by: 0,
                behind_by: 0,
                has_uncommitted,
            });
        }

        let branch_name = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // If no branch (detached HEAD or no commits), return zeros
        if branch_name.is_empty() {
            return Ok(GitStatus {
                ahead_by: 0,
                behind_by: 0,
                has_uncommitted,
            });
        }

        // Check if remote branch exists
        let remote_branch = format!("origin/{}", branch_name);
        let check_remote = Command::new("git")
            .args(["rev-parse", "--verify", &remote_branch])
            .current_dir(repo_path)
            .output()
            .context("Failed to check remote branch")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // If remote branch doesn't exist (e.g., empty remote), all local commits are ahead
        if !check_remote.status.success() {
            // Count total commits on current branch
            let count_output = Command::new("git")
                .args(["rev-list", "--count", "HEAD"])
                .current_dir(repo_path)
                .output()
                .context("Failed to count commits")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            let ahead_by = if count_output.status.success() {
                String::from_utf8_lossy(&count_output.stdout)
                    .trim()
                    .parse()
                    .unwrap_or(0)
            } else {
                0
            };

            return Ok(GitStatus {
                ahead_by,
                behind_by: 0,
                has_uncommitted,
            });
        }

        // Get ahead/behind counts using git rev-list
        let ahead_output = Command::new("git")
            .args(["rev-list", "--count", &format!("{}..HEAD", remote_branch)])
            .current_dir(repo_path)
            .output()
            .context("Failed to count ahead commits")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        let behind_output = Command::new("git")
            .args(["rev-list", "--count", &format!("HEAD..{}", remote_branch)])
            .current_dir(repo_path)
            .output()
            .context("Failed to count behind commits")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        let ahead_by = if ahead_output.status.success() {
            String::from_utf8_lossy(&ahead_output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        let behind_by = if behind_output.status.success() {
            String::from_utf8_lossy(&behind_output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        Ok(GitStatus {
            ahead_by,
            behind_by,
            has_uncommitted,
        })
    }

    fn has_uncommitted_changes(&self, repo_path: &Path) -> Result<bool, IrieBookError> {
        let repo = gix::open(repo_path)
            .context("Failed to open repository")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // Get status iterator to check for changes
        let status = repo
            .status(gix::progress::Discard)
            .context("Failed to initialize status")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        let outcomes = status
            .into_index_worktree_iter(Vec::<BString>::new())
            .context("Failed to get status iterator")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // Check if there are any changes (modified, added, removed, etc.)
        for outcome in outcomes {
            let change = outcome
                .context("Error during status iteration")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            let summary = change
                .summary()
                .context("Failed to compute summary for path")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            match summary {
                Summary::Added | Summary::Modified | Summary::Removed | Summary::TypeChange => {
                    // Found uncommitted changes
                    return Ok(true);
                }
                _ => {} // Ignore Unchanged, IntentToAdd, etc.
            }
        }

        // No changes found
        Ok(false)
    }

    fn get_folder_status(
        &self,
        repo_path: &Path,
        folder_path: &Path,
    ) -> Result<bool, IrieBookError> {
        // 1. Try to open repository
        let repo = match gix::open(repo_path) {
            Ok(r) => r,
            Err(_) => return Ok(false), // Not a git repo
        };

        // 2. Normalize folder_path to relative path from repo root
        let relative_folder = match folder_path.strip_prefix(repo_path) {
            Ok(rel) => rel,
            Err(_) => return Ok(false), // Folder outside repo
        };

        // 3. Get status iterator (same as has_uncommitted_changes)
        let status = repo
            .status(gix::progress::Discard)
            .context("Failed to initialize status")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        let outcomes = status
            .into_index_worktree_iter(Vec::<BString>::new())
            .context("Failed to get status iterator")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // 4. Check if any files within the folder have changes
        for outcome in outcomes {
            let change = outcome
                .context("Error during status iteration")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            let change_path = change.rela_path();

            // Convert to string for comparison
            let change_path_str = match change_path.to_str() {
                Ok(s) => s,
                Err(_) => continue, // Skip non-UTF8 paths
            };

            // Check if the changed file is within the target folder
            let change_path_buf = std::path::Path::new(change_path_str);
            if change_path_buf.starts_with(relative_folder) {
                let summary = change
                    .summary()
                    .context("Failed to compute summary")
                    .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

                match summary {
                    Summary::Added | Summary::Modified | Summary::Removed | Summary::TypeChange => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    fn get_all_changed_files(
        &self,
        repo_path: &Path,
    ) -> Result<Vec<std::path::PathBuf>, IrieBookError> {
        // Try to open repository
        let repo = match gix::open(repo_path) {
            Ok(r) => r,
            Err(e) => {
                return Err(IrieBookError::Git(format!(
                    "Failed to open repository: {}",
                    e
                )));
            }
        };

        // Get status iterator
        let status = repo
            .status(gix::progress::Discard)
            .context("Failed to initialize status")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        let outcomes = status
            .into_index_worktree_iter(Vec::<BString>::new())
            .context("Failed to get status iterator")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        // Collect all changed file paths
        let mut changed_files = Vec::new();
        for outcome in outcomes {
            let change = outcome
                .context("Error during status iteration")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            let summary = change
                .summary()
                .context("Failed to compute summary")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

            match summary {
                Summary::Added | Summary::Modified | Summary::Removed | Summary::TypeChange => {
                    let change_path = change.rela_path();
                    if let Ok(path_str) = change_path.to_str() {
                        let absolute_path = repo_path.join(path_str);
                        changed_files.push(absolute_path);
                    }
                }
                _ => {}
            }
        }

        Ok(changed_files)
    }

    fn get_changed_files(
        &self,
        repo_path: &Path,
        commit_hash: &str,
    ) -> Result<Vec<String>, IrieBookError> {
        use std::process::Command;

        // Use git diff-tree to get changed files
        // --no-commit-id: suppress commit ID output
        // --name-only: show only file names
        // -r: recurse into subdirectories
        // commit_hash: the commit to diff
        let output = Command::new("git")
            .args([
                "diff-tree",
                "--no-commit-id",
                "--name-only",
                "-r",
                commit_hash,
            ])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git diff-tree command")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

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
            .map(|line| line.to_string())
            .collect();

        Ok(files)
    }

    fn discard_local_changes(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        use std::process::Command;

        // First, reset all tracked files to HEAD
        let reset_output = Command::new("git")
            .args(["reset", "--hard", "HEAD"])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git reset command")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !reset_output.status.success() {
            let stderr = String::from_utf8_lossy(&reset_output.stderr);
            return Err(IrieBookError::Git(format!("Reset failed: {}", stderr)));
        }

        // Then, remove all untracked files and directories
        let clean_output = Command::new("git")
            .args(["clean", "-fd"])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git clean command")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !clean_output.status.success() {
            let stderr = String::from_utf8_lossy(&clean_output.stderr);
            return Err(IrieBookError::Git(format!("Clean failed: {}", stderr)));
        }

        Ok(())
    }

    fn prepare_for_rebase(&self, repo_path: &Path) -> Result<(), IrieBookError> {
        use std::process::Command;

        // Remove stale index.lock if it exists (from previous failed git operations)
        let index_lock_path = repo_path.join(".git").join("index.lock");
        if index_lock_path.exists() {
            std::fs::remove_file(&index_lock_path)
                .context("Failed to remove stale index.lock")
                .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;
        }

        // Reset working directory to HEAD (discards uncommitted changes, preserves commits)
        // This is more atomic than 'git checkout .' and less prone to leaving stale index.lock
        let reset_output = Command::new("git")
            .args(["reset", "--hard", "HEAD"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !reset_output.status.success() {
            let stderr = String::from_utf8_lossy(&reset_output.stderr);
            return Err(IrieBookError::Git(format!("Reset failed: {}", stderr)));
        }

        // Remove all untracked files and directories (prevents conflicts with incoming commits)
        let clean_output = Command::new("git")
            .args(["clean", "-fd"])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git clean command")
            .map_err(|e| IrieBookError::Git(format!("{:#}", e)))?;

        if !clean_output.status.success() {
            let stderr = String::from_utf8_lossy(&clean_output.stderr);
            return Err(IrieBookError::Git(format!("Clean failed: {}", stderr)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_access::file::OUTPUT_DIR_NAME;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn is_repository_returns_false_for_non_repo() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;
        assert!(!git_client.is_repository(temp_dir.path()));
    }

    #[test]
    fn is_repository_returns_true_for_valid_repo() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize a git repository
        gix::init(temp_dir.path()).unwrap();

        let git_client = GitClient;
        assert!(git_client.is_repository(temp_dir.path()));
    }

    #[test]
    fn get_log_on_empty_repo_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        gix::init(temp_dir.path()).unwrap();

        let git_client = GitClient;

        // Empty repo should fail to get log (no HEAD commit)
        let result = git_client.get_log(temp_dir.path(), 10);
        assert!(result.is_err());
    }

    #[test]
    fn clone_fails_with_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        let result = git_client.clone_repository(
            "https://github.com/test/repo.git",
            temp_dir.path(),
            "fake_token",
        );

        assert!(result.is_err());
        // We expect a network or git error now, not "Phase 2"
        assert!(!result.unwrap_err().to_string().contains("Phase 2"));
    }

    // Helper function to initialize a git repo using git CLI (creates proper index)
    fn init_git_repo_with_cli(path: &Path) {
        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .expect("Failed to init git repo");

        // Configure user for commits
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .expect("Failed to set git user.name");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .expect("Failed to set git user.email");
    }

    // Helper function to initialize a bare repository (acts as remote)
    fn init_bare_repo(path: &Path) {
        use std::process::Command;
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(path)
            .output()
            .expect("Failed to init bare repo");
    }

    // Helper function to add a remote to a repository
    fn add_remote(repo_path: &Path, remote_name: &str, remote_path: &Path) {
        use std::process::Command;
        let remote_url = format!("file://{}", remote_path.display());
        Command::new("git")
            .args(["remote", "add", remote_name, &remote_url])
            .current_dir(repo_path)
            .output()
            .expect("Failed to add remote");
    }

    // Helper function to verify commits exist in a bare repo
    fn get_commits_in_bare_repo(bare_path: &Path, branch: &str) -> Vec<String> {
        use std::process::Command;
        let output = Command::new("git")
            .args(["log", "--format=%H", branch])
            .current_dir(bare_path)
            .output()
            .expect("Failed to get log from bare repo");

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn add_all_adds_new_files() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create initial commit
        let file1 = temp_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Add new files
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");
        std::fs::write(&file2, "content2").unwrap();
        std::fs::write(&file3, "content3").unwrap();

        // Run add_all
        git_client.add_all(temp_dir.path()).unwrap();

        // After add_all but before commit, changes are staged but still "uncommitted"
        // So we commit and verify the files are in the tree
        git_client.commit(temp_dir.path(), "add new files").unwrap();

        // Verify by checking that we have no uncommitted changes
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn add_all_adds_modified_files() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "initial content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Modify the file
        std::fs::write(&test_file, "modified content").unwrap();

        // Should have uncommitted changes before add
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Run add_all
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "modify file").unwrap();

        // Should be clean after commit
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn add_all_handles_deleted_files() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Delete one file
        std::fs::remove_file(&file1).unwrap();

        // Should have uncommitted changes (deleted file)
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Run add_all to stage the deletion
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "delete file1").unwrap();

        // Should be clean now
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Verify file1 is gone but file2 remains
        assert!(!file1.exists());
        assert!(file2.exists());
    }

    #[test]
    fn add_all_handles_deleted_nested_file() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create directory structure with files
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let file1 = subdir.join("file1.txt");
        let file2 = subdir.join("file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();

        // Commit the files
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial with nested files")
            .unwrap();

        // Should be clean
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Delete one nested file
        std::fs::remove_file(&file1).unwrap();

        // Should detect the deletion
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Stage the deletion
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "delete nested file")
            .unwrap();

        // Should be clean now
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Verify file1 is gone but file2 remains
        assert!(!file1.exists());
        assert!(file2.exists());
    }

    #[test]
    fn add_all_handles_deleted_entire_directory() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create directory with multiple files
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let file1 = subdir.join("file1.txt");
        let file2 = subdir.join("file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();

        // Commit the files
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial with directory")
            .unwrap();

        // Delete entire directory
        std::fs::remove_dir_all(&subdir).unwrap();

        // Should detect the deletions
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Stage all deletions
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "delete entire directory")
            .unwrap();

        // Should be clean
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Verify directory is gone
        assert!(!subdir.exists());
    }

    #[test]
    fn add_all_handles_mixed_changes() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create initial files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Mixed changes:
        // - Modify file1
        // - Delete file2
        // - Add new file3
        std::fs::write(&file1, "modified1").unwrap();
        std::fs::remove_file(&file2).unwrap();
        let file3 = temp_dir.path().join("file3.txt");
        std::fs::write(&file3, "content3").unwrap();

        // All changes should be detected
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Stage all changes
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "mixed changes").unwrap();

        // Should be clean
        if git_client.has_uncommitted_changes(temp_dir.path()).unwrap() {
            use std::process::Command;

            let status_output = Command::new("git")
                .args(["status", "--short"])
                .current_dir(temp_dir.path())
                .output()
                .expect("Failed to run git status");

            let status = String::from_utf8_lossy(&status_output.stdout);
            panic!(
                "Repository not clean after mixed changes commit:\n{}",
                status
            );
        }
    }

    #[test]
    fn add_all_handles_directories() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create initial commit
        let file1 = temp_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Create a directory with files in it
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let file_in_dir = subdir.join("nested.txt");
        std::fs::write(&file_in_dir, "nested content").unwrap();

        // Add all should handle the directory and its contents
        let result = git_client.add_all(temp_dir.path());
        assert!(
            result.is_ok(),
            "add_all should handle directories: {:?}",
            result
        );

        // Commit to verify everything was staged
        git_client.commit(temp_dir.path(), "add directory").unwrap();

        // Should be clean
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn add_all_handles_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create nested directory structure
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = dir1.join("dir2");
        std::fs::create_dir_all(&dir2).unwrap();

        let file1 = dir1.join("file1.txt");
        let file2 = dir2.join("file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();

        // Add all should handle nested directories
        let result = git_client.add_all(temp_dir.path());
        assert!(
            result.is_ok(),
            "add_all should handle nested directories: {:?}",
            result
        );

        // Commit and verify
        git_client.commit(temp_dir.path(), "add nested").unwrap();
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn add_all_respects_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create .gitignore file
        let gitignore = temp_dir.path().join(".gitignore");
        std::fs::write(&gitignore, "*.log\n*.tmp\nsecrets/\n").unwrap();

        // Create files that should be tracked
        let tracked_file = temp_dir.path().join("tracked.txt");
        std::fs::write(&tracked_file, "track me").unwrap();

        // Create files that should be ignored
        let ignored_log = temp_dir.path().join("debug.log");
        let ignored_tmp = temp_dir.path().join("temp.tmp");
        std::fs::write(&ignored_log, "log content").unwrap();
        std::fs::write(&ignored_tmp, "tmp content").unwrap();

        // Create ignored directory with files
        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir(&secrets_dir).unwrap();
        let secret_file = secrets_dir.join("api_key.txt");
        std::fs::write(&secret_file, "SECRET_KEY").unwrap();

        // Run add_all
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial commit")
            .unwrap();

        // Verify using git CLI what was actually committed
        use std::process::Command;
        let ls_files = Command::new("git")
            .args(["ls-files"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let tracked_files = String::from_utf8_lossy(&ls_files.stdout);
        eprintln!("Tracked files:\n{}", tracked_files);

        // .gitignore and tracked.txt should be in the repo
        assert!(
            tracked_files.contains(".gitignore"),
            ".gitignore should be tracked"
        );
        assert!(
            tracked_files.contains("tracked.txt"),
            "tracked.txt should be tracked"
        );

        // Ignored files should NOT be in the repo
        assert!(
            !tracked_files.contains("debug.log"),
            "debug.log should be ignored"
        );
        assert!(
            !tracked_files.contains("temp.tmp"),
            "temp.tmp should be ignored"
        );
        assert!(
            !tracked_files.contains("secrets/"),
            "secrets/ should be ignored"
        );
        assert!(
            !tracked_files.contains("api_key.txt"),
            "api_key.txt should be ignored"
        );
    }

    #[test]
    fn add_all_respects_gitignore_nested() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create .gitignore like the user's case
        let gitignore = temp_dir.path().join(".gitignore");
        std::fs::write(&gitignore, format!("irie/\n{}/\n", OUTPUT_DIR_NAME)).unwrap();

        // Create nested structure like user's books
        let book_dir = temp_dir.path().join("My Book");
        std::fs::create_dir(&book_dir).unwrap();

        // Create irie/ folder inside book (should be ignored)
        let irie_dir = book_dir.join("irie");
        std::fs::create_dir(&irie_dir).unwrap();
        std::fs::write(irie_dir.join("fixed.md"), "fixed content").unwrap();
        std::fs::write(irie_dir.join("summary.md"), "summary").unwrap();

        // Create output folder inside book (should be ignored)
        let output_dir = book_dir.join(OUTPUT_DIR_NAME);
        std::fs::create_dir(&output_dir).unwrap();
        std::fs::write(output_dir.join("output.epub"), "epub data").unwrap();

        // Create tracked file in book dir
        std::fs::write(book_dir.join("manuscript.md"), "manuscript content").unwrap();

        // Run add_all
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial commit")
            .unwrap();

        // Verify using git CLI what was actually committed
        use std::process::Command;
        let ls_files = Command::new("git")
            .args(["ls-files"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let tracked_files = String::from_utf8_lossy(&ls_files.stdout);
        eprintln!("Tracked files:\n{}", tracked_files);

        // .gitignore and manuscript.md should be tracked
        assert!(
            tracked_files.contains(".gitignore"),
            ".gitignore should be tracked"
        );
        assert!(
            tracked_files.contains("manuscript.md"),
            "manuscript.md should be tracked"
        );

        // irie/ and output folders should NOT be tracked
        assert!(!tracked_files.contains("irie/"), "irie/ should be ignored");
        assert!(
            !tracked_files.contains(&format!("{}/", OUTPUT_DIR_NAME)),
            "{OUTPUT_DIR_NAME}/ should be ignored"
        );
        assert!(
            !tracked_files.contains("fixed.md"),
            "files in irie/ should be ignored"
        );
        assert!(
            !tracked_files.contains("output.epub"),
            "files in {OUTPUT_DIR_NAME}/ should be ignored"
        );
    }

    #[test]
    fn pull_rebase_ours_fetches_and_rebases() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir1 = TempDir::new().unwrap();
        let work_dir2 = TempDir::new().unwrap();
        let git_client = GitClient;

        // Create bare repo (acts as remote)
        init_bare_repo(bare_dir.path());

        // Clone to first working repo
        init_git_repo_with_cli(work_dir1.path());
        add_remote(work_dir1.path(), "origin", bare_dir.path());

        // Create initial commit and push
        let file1 = work_dir1.path().join("file1.txt");
        std::fs::write(&file1, "initial").unwrap();
        git_client.add_all(work_dir1.path()).unwrap();
        git_client
            .commit(work_dir1.path(), "initial commit")
            .unwrap();
        git_client.push(work_dir1.path(), "").unwrap();

        // Clone to second working repo
        init_git_repo_with_cli(work_dir2.path());
        add_remote(work_dir2.path(), "origin", bare_dir.path());

        // Fetch initial state to work_dir2
        use std::process::Command;
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(work_dir2.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["reset", "--hard", "origin/master"])
            .current_dir(work_dir2.path())
            .output()
            .unwrap();

        // Make change in work_dir1 and push
        std::fs::write(&file1, "updated from repo1").unwrap();
        git_client.add_all(work_dir1.path()).unwrap();
        git_client
            .commit(work_dir1.path(), "update from repo1")
            .unwrap();
        git_client.push(work_dir1.path(), "").unwrap();

        // Make different change in work_dir2 (creates potential conflict)
        let file2 = work_dir2.path().join("file2.txt");
        std::fs::write(&file2, "new file from repo2").unwrap();
        git_client.add_all(work_dir2.path()).unwrap();
        git_client
            .commit(work_dir2.path(), "add file2 from repo2")
            .unwrap();

        // Pull rebase should succeed (no actual conflict since different files)
        let result = git_client.pull_rebase_ours(work_dir2.path());
        assert!(result.is_ok(), "Pull rebase should succeed: {:?}", result);

        // Verify we have both changes
        let file1_content = std::fs::read_to_string(work_dir2.path().join("file1.txt")).unwrap();
        assert_eq!(file1_content, "updated from repo1");
        assert!(work_dir2.path().join("file2.txt").exists());
    }

    #[test]
    fn pull_rebase_ours_handles_conflicts_with_ours_strategy() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir1 = TempDir::new().unwrap();
        let work_dir2 = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup bare repo and two working repos
        init_bare_repo(bare_dir.path());

        init_git_repo_with_cli(work_dir1.path());
        add_remote(work_dir1.path(), "origin", bare_dir.path());

        init_git_repo_with_cli(work_dir2.path());
        add_remote(work_dir2.path(), "origin", bare_dir.path());

        // Create initial file and push from repo1
        let file1 = work_dir1.path().join("conflict.txt");
        std::fs::write(&file1, "initial content").unwrap();
        git_client.add_all(work_dir1.path()).unwrap();
        git_client.commit(work_dir1.path(), "initial").unwrap();
        git_client.push(work_dir1.path(), "").unwrap();

        // Fetch to repo2
        use std::process::Command;
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(work_dir2.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["reset", "--hard", "origin/master"])
            .current_dir(work_dir2.path())
            .output()
            .unwrap();

        // Make conflicting change in repo1 and push
        std::fs::write(&file1, "changed in repo1").unwrap();
        git_client.add_all(work_dir1.path()).unwrap();
        git_client
            .commit(work_dir1.path(), "change in repo1")
            .unwrap();
        git_client.push(work_dir1.path(), "").unwrap();

        // Make conflicting change in repo2
        let file2 = work_dir2.path().join("conflict.txt");
        std::fs::write(&file2, "changed in repo2 - OURS").unwrap();
        git_client.add_all(work_dir2.path()).unwrap();
        git_client
            .commit(work_dir2.path(), "change in repo2")
            .unwrap();

        // Pull rebase with ours strategy should keep our changes
        let result = git_client.pull_rebase_ours(work_dir2.path());
        assert!(
            result.is_ok(),
            "Pull rebase should handle conflicts with ours: {:?}",
            result
        );

        // Verify our changes are kept (ours strategy)
        let content = std::fs::read_to_string(file2).unwrap();
        assert_eq!(
            content, "changed in repo2 - OURS",
            "Should keep our changes with ours strategy"
        );
    }

    #[test]
    fn pull_rebase_ours_when_already_up_to_date() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Create and push a commit
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "initial").unwrap();
        git_client.push(work_dir.path(), "").unwrap();

        // Pull rebase when already up to date should succeed
        let result = git_client.pull_rebase_ours(work_dir.path());
        assert!(result.is_ok(), "Pull rebase when up to date should succeed");
    }

    #[test]
    fn pull_rebase_ours_from_empty_remote() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup empty bare repo
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Create local commit (but don't push yet)
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "initial").unwrap();

        // Pull rebase from empty remote should succeed (nothing to rebase)
        let result = git_client.pull_rebase_ours(work_dir.path());
        assert!(
            result.is_ok(),
            "Pull rebase from empty remote should succeed: {:?}",
            result
        );

        // Verify our commit still exists
        let log = git_client.get_log(work_dir.path(), 1).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "initial");
    }

    #[test]
    fn get_status_with_empty_remote() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup empty bare repo
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Create 2 local commits (remote is empty)
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "commit 1").unwrap();

        std::fs::write(&file1, "content2").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "commit 2").unwrap();

        // Get status - should show 2 commits ahead
        let status = git_client.get_status(work_dir.path()).unwrap();
        assert_eq!(
            status.ahead_by, 2,
            "Should have 2 commits ahead of empty remote"
        );
        assert_eq!(status.behind_by, 0);
        assert!(!status.has_uncommitted);
    }

    #[test]
    fn get_status_after_push() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Create and push commit
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "commit 1").unwrap();
        git_client.push(work_dir.path(), "").unwrap();

        // Get status - should be in sync
        let status = git_client.get_status(work_dir.path()).unwrap();
        assert_eq!(status.ahead_by, 0, "Should be in sync after push");
        assert_eq!(status.behind_by, 0);
        assert!(!status.has_uncommitted);
    }

    #[test]
    fn get_status_with_uncommitted_changes() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Create and push commit
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "commit 1").unwrap();
        git_client.push(work_dir.path(), "").unwrap();

        // Make uncommitted changes
        std::fs::write(&file1, "modified").unwrap();

        // Get status
        let status = git_client.get_status(work_dir.path()).unwrap();
        assert_eq!(status.ahead_by, 0);
        assert_eq!(status.behind_by, 0);
        assert!(status.has_uncommitted, "Should detect uncommitted changes");
    }

    #[test]
    fn commit_creates_commit_and_returns_hash() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and add a file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();

        // Create commit
        let commit_hash = git_client
            .commit(temp_dir.path(), "Test commit message")
            .unwrap();

        // Verify hash is returned (40 char SHA-1 hex string)
        assert_eq!(commit_hash.len(), 40);
        assert!(commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn commit_stores_correct_message() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit a file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();

        let message = "This is my test commit message";
        git_client.commit(temp_dir.path(), message).unwrap();

        // Verify message is stored correctly
        let log = git_client.get_log(temp_dir.path(), 1).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, message);
    }

    #[test]
    fn commit_creates_parent_child_relationship() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // First commit
        let file1 = temp_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        let hash1 = git_client.commit(temp_dir.path(), "First commit").unwrap();

        // Second commit
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file2, "content2").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        let hash2 = git_client.commit(temp_dir.path(), "Second commit").unwrap();

        // Verify they're different commits
        assert_ne!(hash1, hash2);

        // Verify log shows both commits in correct order
        let log = git_client.get_log(temp_dir.path(), 10).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].message, "Second commit");
        assert_eq!(log[1].message, "First commit");
    }

    #[test]
    fn commit_updates_head() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create first commit
        let file1 = temp_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        let hash1 = git_client.commit(temp_dir.path(), "First").unwrap();

        // Get log - HEAD should point to first commit
        let log1 = git_client.get_log(temp_dir.path(), 1).unwrap();
        assert_eq!(log1[0].hash, hash1);

        // Create second commit
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file2, "content2").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        let hash2 = git_client.commit(temp_dir.path(), "Second").unwrap();

        // Get log - HEAD should now point to second commit
        let log2 = git_client.get_log(temp_dir.path(), 1).unwrap();
        assert_eq!(log2[0].hash, hash2);
    }

    #[test]
    fn commit_with_no_changes_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // First commit with content
        let file1 = temp_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "First").unwrap();

        // Try to commit without any changes (nothing staged)
        let result = git_client.commit(temp_dir.path(), "Empty commit");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Nothing to commit")
        );
    }

    #[test]
    fn push_succeeds_to_local_bare_repo() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Create bare repo (acts as remote)
        init_bare_repo(bare_dir.path());

        // Create working repo
        init_git_repo_with_cli(work_dir.path());

        // Create a commit
        let file = work_dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        let commit_hash = git_client
            .commit(work_dir.path(), "initial commit")
            .unwrap();

        // Add bare repo as remote
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Push to bare repo (no token needed for file:// protocol)
        let result = git_client.push(work_dir.path(), "");
        assert!(result.is_ok());

        // Verify bare repo has the commit
        let commits = get_commits_in_bare_repo(bare_dir.path(), "master");
        assert!(commits.contains(&commit_hash));
    }

    #[test]
    fn push_multiple_commits_to_bare_repo() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup bare and working repos
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // Create first commit
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        let hash1 = git_client.commit(work_dir.path(), "first").unwrap();

        // Create second commit
        let file2 = work_dir.path().join("file2.txt");
        std::fs::write(&file2, "content2").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        let hash2 = git_client.commit(work_dir.path(), "second").unwrap();

        // Push both commits
        git_client.push(work_dir.path(), "").unwrap();

        // Verify both commits are in bare repo
        let commits = get_commits_in_bare_repo(bare_dir.path(), "master");
        assert!(commits.contains(&hash1));
        assert!(commits.contains(&hash2));
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn push_fails_without_remote() {
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Create working repo without remote
        init_git_repo_with_cli(work_dir.path());

        // Create a commit
        let file = work_dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        git_client.commit(work_dir.path(), "test").unwrap();

        // Push should fail - no remote configured
        let result = git_client.push(work_dir.path(), "");
        assert!(result.is_err());
    }

    #[test]
    fn push_incremental_changes() {
        let bare_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Setup
        init_bare_repo(bare_dir.path());
        init_git_repo_with_cli(work_dir.path());
        add_remote(work_dir.path(), "origin", bare_dir.path());

        // First commit and push
        let file1 = work_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        let hash1 = git_client.commit(work_dir.path(), "first").unwrap();
        git_client.push(work_dir.path(), "").unwrap();

        // Verify first commit is pushed
        let commits1 = get_commits_in_bare_repo(bare_dir.path(), "master");
        assert_eq!(commits1.len(), 1);

        // Second commit and push
        let file2 = work_dir.path().join("file2.txt");
        std::fs::write(&file2, "content2").unwrap();
        git_client.add_all(work_dir.path()).unwrap();
        let hash2 = git_client.commit(work_dir.path(), "second").unwrap();
        git_client.push(work_dir.path(), "").unwrap();

        // Verify both commits are now pushed
        let commits2 = get_commits_in_bare_repo(bare_dir.path(), "master");
        assert_eq!(commits2.len(), 2);
        assert!(commits2.contains(&hash1));
        assert!(commits2.contains(&hash2));
    }

    #[test]
    fn has_uncommitted_changes_detects_modified_file() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Initialize repository using CLI to get proper index
        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "initial content").unwrap();

        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial commit")
            .unwrap();

        // No uncommitted changes yet
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Modify the file
        std::fs::write(&test_file, "modified content").unwrap();

        // Should detect uncommitted changes
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn has_uncommitted_changes_detects_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Initialize repository with initial commit
        init_git_repo_with_cli(temp_dir.path());
        let initial_file = temp_dir.path().join("initial.txt");
        std::fs::write(&initial_file, "initial").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial commit")
            .unwrap();

        // Add a new untracked file
        let new_file = temp_dir.path().join("new.txt");
        std::fs::write(&new_file, "new content").unwrap();

        // Should detect uncommitted changes (new file)
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn has_uncommitted_changes_returns_false_for_clean_repo() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        // Initialize repository with a commit
        init_git_repo_with_cli(temp_dir.path());
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client
            .commit(temp_dir.path(), "initial commit")
            .unwrap();

        // Should be clean
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
    }

    #[test]
    fn get_folder_status_detects_modified_file_in_folder() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create two book folders
        std::fs::create_dir(temp_dir.path().join("book1")).unwrap();
        std::fs::create_dir(temp_dir.path().join("book2")).unwrap();

        // Create and commit files in both folders
        let file1 = temp_dir.path().join("book1/book.md");
        let file2 = temp_dir.path().join("book2/book.md");
        std::fs::write(&file1, "content").unwrap();
        std::fs::write(&file2, "content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Modify file in book1 folder
        std::fs::write(&file1, "modified").unwrap();

        // book1 folder should have changes
        assert!(
            git_client
                .get_folder_status(temp_dir.path(), &temp_dir.path().join("book1"))
                .unwrap()
        );
        // book2 folder should NOT have changes
        assert!(
            !git_client
                .get_folder_status(temp_dir.path(), &temp_dir.path().join("book2"))
                .unwrap()
        );
    }

    #[test]
    fn get_folder_status_detects_new_file_in_folder() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create folder
        std::fs::create_dir(temp_dir.path().join("book")).unwrap();

        // Add new file
        let new_file = temp_dir.path().join("book/book.md");
        std::fs::write(&new_file, "content").unwrap();

        // Folder should show changes
        assert!(
            git_client
                .get_folder_status(temp_dir.path(), &temp_dir.path().join("book"))
                .unwrap()
        );
    }

    #[test]
    fn get_folder_status_detects_any_file_change_in_folder() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create book folder with multiple files
        std::fs::create_dir(temp_dir.path().join("book")).unwrap();
        let book_md = temp_dir.path().join("book/book.md");
        let metadata = temp_dir.path().join("book/metadata.yaml");
        let cover = temp_dir.path().join("book/cover.jpg");

        std::fs::write(&book_md, "content").unwrap();
        std::fs::write(&metadata, "title: Test").unwrap();
        std::fs::write(&cover, "fake image").unwrap();

        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Modify metadata (not the .md file)
        std::fs::write(&metadata, "title: Updated").unwrap();

        // Folder should show changes even though .md file wasn't modified
        assert!(
            git_client
                .get_folder_status(temp_dir.path(), &temp_dir.path().join("book"))
                .unwrap()
        );
    }

    #[test]
    fn get_folder_status_returns_false_for_unchanged_folder() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit folder
        std::fs::create_dir(temp_dir.path().join("book")).unwrap();
        let file = temp_dir.path().join("book/book.md");
        std::fs::write(&file, "content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Folder should have no changes
        assert!(
            !git_client
                .get_folder_status(temp_dir.path(), &temp_dir.path().join("book"))
                .unwrap()
        );
    }

    #[test]
    fn get_folder_status_handles_non_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        std::fs::create_dir(temp_dir.path().join("book")).unwrap();
        let file = temp_dir.path().join("book/book.md");
        std::fs::write(&file, "content").unwrap();

        // Should not panic, should return false
        assert!(
            !git_client
                .get_folder_status(temp_dir.path(), &temp_dir.path().join("book"))
                .unwrap()
        );
    }

    #[test]
    fn get_folder_status_handles_folder_outside_repo() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        let outside_folder = PathBuf::from("/tmp/outside");

        // Should not panic, should return false
        assert!(
            !git_client
                .get_folder_status(temp_dir.path(), &outside_folder)
                .unwrap()
        );
    }

    #[test]
    fn discard_local_changes_removes_tracked_changes() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "initial content").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Modify the tracked file
        std::fs::write(&test_file, "modified content").unwrap();

        // Should have uncommitted changes
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Discard changes
        git_client.discard_local_changes(temp_dir.path()).unwrap();

        // Should be clean now
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // File should be back to original content
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "initial content");
    }

    #[test]
    fn discard_local_changes_removes_untracked_files() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial file
        let tracked_file = temp_dir.path().join("tracked.txt");
        std::fs::write(&tracked_file, "tracked").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Add untracked file
        let untracked_file = temp_dir.path().join("untracked.txt");
        std::fs::write(&untracked_file, "untracked content").unwrap();

        // Should have uncommitted changes
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
        assert!(untracked_file.exists());

        // Discard changes
        git_client.discard_local_changes(temp_dir.path()).unwrap();

        // Should be clean now
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Untracked file should be removed
        assert!(!untracked_file.exists());
    }

    #[test]
    fn discard_local_changes_removes_untracked_directories() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial file
        let tracked_file = temp_dir.path().join("tracked.txt");
        std::fs::write(&tracked_file, "tracked").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Add untracked directory with files
        let untracked_dir = temp_dir.path().join("untracked_dir");
        std::fs::create_dir(&untracked_dir).unwrap();
        std::fs::write(untracked_dir.join("file1.txt"), "content1").unwrap();
        std::fs::write(untracked_dir.join("file2.txt"), "content2").unwrap();

        // Should have uncommitted changes
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());
        assert!(untracked_dir.exists());

        // Discard changes
        git_client.discard_local_changes(temp_dir.path()).unwrap();

        // Should be clean now
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Untracked directory should be removed
        assert!(!untracked_dir.exists());
    }

    #[test]
    fn discard_local_changes_handles_mixed_changes() {
        let temp_dir = TempDir::new().unwrap();
        let git_client = GitClient;

        init_git_repo_with_cli(temp_dir.path());

        // Create and commit initial files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file1, "initial1").unwrap();
        std::fs::write(&file2, "initial2").unwrap();
        git_client.add_all(temp_dir.path()).unwrap();
        git_client.commit(temp_dir.path(), "initial").unwrap();

        // Make mixed changes:
        // - Modify tracked file1
        std::fs::write(&file1, "modified1").unwrap();
        // - Delete tracked file2
        std::fs::remove_file(&file2).unwrap();
        // - Add untracked file3
        let file3 = temp_dir.path().join("file3.txt");
        std::fs::write(&file3, "untracked").unwrap();

        // Should have uncommitted changes
        assert!(git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Discard all changes
        git_client.discard_local_changes(temp_dir.path()).unwrap();

        // Should be clean
        assert!(!git_client.has_uncommitted_changes(temp_dir.path()).unwrap());

        // Verify state: file1 restored, file2 restored, file3 removed
        assert_eq!(std::fs::read_to_string(&file1).unwrap(), "initial1");
        assert!(file2.exists());
        assert_eq!(std::fs::read_to_string(&file2).unwrap(), "initial2");
        assert!(!file3.exists());
    }
}
