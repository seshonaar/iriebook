//! Pandoc access layer for EPUB conversion
//!
//! Provides access to the Pandoc command-line tool for converting
//! markdown files to EPUB format.

use crate::resource_access::traits::PandocAccess;
use crate::resource_access::{command, file};
use crate::utilities::error::IrieBookError;
use std::path::Path;
use std::process::Command;
use tracing::debug;

/// Concrete implementation of PandocAccess using the Pandoc command-line tool
pub struct PandocConverter;

impl PandocAccess for PandocConverter {
    fn convert_to_epub(
        &self,
        original_input: &Path,
        fixed_md: &Path,
        output_epub: &Path,
    ) -> Result<String, IrieBookError> {
        convert_to_epub_impl(original_input, fixed_md, output_epub)
    }
}

/// Implementation of EPUB conversion using Pandoc
///
/// Uses original_input to find metadata.yaml and cover.jpg in the root folder,
/// and fixed_md as the content to convert.
fn convert_to_epub_impl(
    original_input: &Path,
    fixed_md: &Path,
    output_epub: &Path,
) -> Result<String, IrieBookError> {
    let css_path = file::get_css_path().map_err(|e| IrieBookError::FileRead {
        path: "css".into(),
        source: std::io::Error::other(e),
    })?;

    // Use original input path to find metadata and cover (not the fixed.md path!)
    let metadata_path =
        file::get_book_folder_file(original_input, "metadata.yaml").map_err(|e| {
            IrieBookError::FileRead {
                path: "metadata.yaml".into(),
                source: std::io::Error::other(e),
            }
        })?;

    let cover_path = file::get_book_folder_file(original_input, "cover.jpg").map_err(|e| {
        IrieBookError::FileRead {
            path: "cover.jpg".into(),
            source: std::io::Error::other(e),
        }
    })?;

    // Verify all required files exist before running Pandoc
    if !Path::new(&css_path).exists() {
        return Err(IrieBookError::FileRead {
            path: css_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "CSS file not found"),
        });
    }

    if !metadata_path.exists() {
        return Err(IrieBookError::FileRead {
            path: metadata_path.to_string_lossy().to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "metadata.yaml file not found",
            ),
        });
    }

    if !cover_path.exists() {
        return Err(IrieBookError::FileRead {
            path: cover_path.to_string_lossy().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "cover.jpg file not found"),
        });
    }

    // Convert the fixed markdown content
    debug!(fixed_md = %fixed_md.display(), css = %css_path, "Starting EPUB conversion");
    let pandoc_output = Command::new("pandoc")
        .arg(fixed_md)
        .arg("-o")
        .arg(output_epub)
        .arg("--toc")
        .arg("-t")
        .arg("epub3")
        .arg("--css")
        .arg(css_path)
        .arg("--metadata-file")
        .arg(metadata_path)
        .arg("--epub-cover-image")
        .arg(cover_path)
        .arg("--standalone")
        .arg("--split-level=1")
        .output()
        .map_err(|e| IrieBookError::FileRead {
            path: "pandoc".into(),
            source: e,
        })?;

    Ok(command::format_output(pandoc_output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandoc_converter_implements_trait() {
        let converter = PandocConverter;
        // This test just verifies the trait is implemented correctly
        // Actual pandoc execution would require pandoc to be installed
        let _ = converter;
    }
}
