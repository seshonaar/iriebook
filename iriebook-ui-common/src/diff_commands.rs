use iriebook::managers::diff_manager::DiffManager;
pub use iriebook::utilities::types::RevisionDiff;

/// Get diffs for all markdown files changed in a revision
///
/// Uses `DiffManager::get_revision_changes()` which already has filtering built-in.
/// This is the proper way to get revision diffs - NO DUPLICATION of logic!
///
/// Following Volatility-Based Design:
/// - The UI layer should NOT reimplement manager functionality
/// - Progress callbacks abstract away framework-specific events
/// - This delegates to the existing manager method
///
/// # Arguments
/// * `commit_hash` - The git commit hash to analyze
/// * `diff_manager` - The diff manager instance
/// * `progress_callback` - Callback for progress updates
///
/// # Returns
/// * `Ok(Vec<RevisionDiff>)` with diffs for all changed markdown files
/// * `Err(String)` with error message
pub async fn get_revision_diffs<F>(
    commit_hash: &str,
    diff_manager: &DiffManager,
    progress_callback: F,
) -> Result<Vec<RevisionDiff>, String>
where
    F: Fn(String) + Send + 'static,
{
    progress_callback("Scanning for changed files...".to_string());

    // Use existing DiffManager method - NO DUPLICATION!
    // This method already:
    // 1. Gets list of changed files
    // 2. Filters by extension (.md)
    // 3. Computes diffs with context trimming
    let changes = diff_manager
        .get_revision_changes(commit_hash, Some(".md"))
        .await
        .map_err(|e| e.to_string())?;

    progress_callback(format!("Comparing {} files...", changes.len()));

    // Convert from Vec<(String, DiffComparison)> to Vec<RevisionDiff>
    let diffs: Vec<RevisionDiff> = changes
        .into_iter()
        .map(|(file_path, comparison)| RevisionDiff {
            file_path,
            comparison,
        })
        .collect();

    progress_callback("Comparison complete".to_string());
    Ok(diffs)
}

/// Get diffs for all uncommitted markdown files (working directory vs HEAD)
///
/// Uses `DiffManager::get_local_changes()` which gets uncommitted changes.
/// This is the proper way to get local diffs - NO DUPLICATION of logic!
///
/// Following Volatility-Based Design:
/// - The UI layer should NOT reimplement manager functionality
/// - Progress callbacks abstract away framework-specific events
/// - This delegates to the existing manager method
///
/// # Arguments
/// * `diff_manager` - The diff manager instance
/// * `progress_callback` - Callback for progress updates
///
/// # Returns
/// * `Ok(Vec<RevisionDiff>)` with diffs for all uncommitted markdown files
/// * `Err(String)` with error message
pub async fn get_local_diffs<F>(
    diff_manager: &DiffManager,
    progress_callback: F,
) -> Result<Vec<RevisionDiff>, String>
where
    F: Fn(String) + Send + 'static,
{
    progress_callback("Scanning for uncommitted files...".to_string());

    // Use existing DiffManager method - NO DUPLICATION!
    // This method already:
    // 1. Gets list of uncommitted files
    // 2. Filters by extension (.md)
    // 3. Computes diffs with context trimming (HEAD vs working directory)
    let changes = diff_manager
        .get_local_changes(Some(".md"))
        .await
        .map_err(|e| e.to_string())?;

    progress_callback(format!("Comparing {} files...", changes.len()));

    // Convert from Vec<(String, DiffComparison)> to Vec<RevisionDiff>
    let diffs: Vec<RevisionDiff> = changes
        .into_iter()
        .map(|(file_path, comparison)| RevisionDiff {
            file_path,
            comparison,
        })
        .collect();

    progress_callback("Comparison complete".to_string());
    Ok(diffs)
}

/// Get diff between original book and processed fixed.md
///
/// Compares the original manuscript with the processed version in irie/fixed.md.
/// Returns error if fixed.md doesn't exist yet.
///
/// # Arguments
/// * `book_path` - Absolute path to the original book file
/// * `diff_manager` - The diff manager instance
///
/// # Returns
/// * `Ok(DiffComparison)` with the comparison
/// * `Err(String)` if fixed.md doesn't exist or comparison fails
pub async fn get_book_processing_diff(
    book_path: &str,
    diff_manager: &DiffManager,
) -> Result<iriebook::utilities::types::DiffComparison, String> {
    use std::path::Path;
    use iriebook::resource_access::file;
    use iriebook::utilities::types::{DiffRequest, DiffSourceId, DisplayName};

    let original_path = Path::new(book_path);

    // Generate path to fixed.md using existing utility
    let fixed_path = file::generate_output_path(original_path)
        .map_err(|e| format!("Failed to generate fixed.md path: {}", e))?;

    // Check if fixed.md exists
    if !fixed_path.exists() {
        return Err("No processed version available. Please process this book first.".to_string());
    }

    // Create diff request
    let request = DiffRequest {
        left_source: DiffSourceId(book_path.to_string()),
        left_display_name: DisplayName("Original".to_string()),
        right_source: DiffSourceId(fixed_path.display().to_string()),
        right_display_name: DisplayName("Processed".to_string()),
        relative_path: String::new(), // Not used for file-to-file comparison
    };

    // Use DiffManager with context trimming (same as git diffs)
    let context_config = iriebook::utilities::diff_trimmer::ContextConfig::default();
    diff_manager
        .compare_with_context(&request, context_config)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    // Note: Testing get_revision_diffs would require a mock DiffManager
    // For now, we're ensuring the module compiles correctly

    #[test]
    fn test_module_compiles() {
        // This test ensures the module compiles without errors
        assert!(true);
    }
}
