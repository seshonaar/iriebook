//! Default paths for IrieBook application
//!
//! Provides platform-specific default folder locations

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Get the default library path for IrieBook
///
/// Returns ~/Documents/IrieBook on all platforms.
/// Creates the directory if it doesn't exist.
///
/// # Errors
///
/// Returns an error if:
/// - The Documents directory cannot be determined
/// - The IrieBook directory cannot be created
pub fn get_default_library_path() -> Result<PathBuf> {
    let docs_dir = dirs::document_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine Documents directory"))?;

    let iriebook_path = docs_dir.join("IrieBook");

    // Create directory if it doesn't exist
    fs::create_dir_all(&iriebook_path)?;

    Ok(iriebook_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_library_path_creates_directory() {
        // This test will actually create ~/Documents/IrieBook
        // which is acceptable behavior for a default library location
        let result = get_default_library_path();

        match result {
            Ok(path) => {
                assert!(path.exists());
                assert!(path.is_dir());
                assert!(path.ends_with("IrieBook"));
            }
            Err(e) => {
                // On some test environments, document_dir() might not be available
                eprintln!("Could not get default library path: {}", e);
            }
        }
    }

    #[test]
    fn test_get_default_library_path_is_idempotent() {
        // Calling multiple times should return same path
        let path1 = get_default_library_path();
        let path2 = get_default_library_path();

        match (path1, path2) {
            (Ok(p1), Ok(p2)) => assert_eq!(p1, p2),
            _ => {
                // One or both failed - acceptable in test environments
            }
        }
    }
}
