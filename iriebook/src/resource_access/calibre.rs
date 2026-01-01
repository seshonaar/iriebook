//! Calibre access layer for Kindle conversion and metadata stamping
//!
//! Provides access to Calibre command-line tools (ebook-convert, ebook-meta)
//! for converting EPUB to Kindle format and stamping series metadata.

use crate::utilities::error::IrieBookError;
use crate::resource_access::{file, command};
use crate::resource_access::traits::CalibreAccess;
use crate::utilities::types::BookMetadata;
use std::path::Path;
use std::process::Command;

/// Concrete implementation of CalibreAccess using Calibre command-line tools
pub struct CalibreConverter;

impl CalibreAccess for CalibreConverter {
    fn convert_to_kindle(&self, input_md: &Path, input_epub: &Path) -> Result<String, IrieBookError> {
        convert_to_kindle_impl(input_md, input_epub)
    }

    fn stamp_metadata(&self, file_path: &Path, series: &str, index: u32) -> Result<String, IrieBookError> {
        stamp_metadata_impl(file_path, series, index)
    }

    fn view_ebook(&self, epub_path: &Path) -> Result<String, IrieBookError> {
        view_ebook_impl(epub_path)
    }
}

/// Implementation of Kindle conversion using ebook-convert
fn convert_to_kindle_impl(input_md: &Path, input_epub: &Path) -> Result<String, IrieBookError> {
    let output_kindle = file::change_extension(input_epub, "azw3");

    // Read and parse metadata from book root folder
    let metadata_path =
        file::get_book_folder_file(input_md, "metadata.yaml").map_err(|e| {
            IrieBookError::FileRead {
                path: "metadata.yaml".into(),
                source: std::io::Error::other(e),
            }
        })?;

    let metadata_content =
        file::read_file(&metadata_path).map_err(|e| IrieBookError::FileRead {
            path: metadata_path.display().to_string(),
            source: std::io::Error::other(e),
        })?;

    // Strip YAML frontmatter delimiters
    let yaml_content = metadata_content
        .lines()
        .filter(|line| *line != "---")
        .collect::<Vec<_>>()
        .join("\n");

    let metadata: BookMetadata =
        serde_yaml::from_str(&yaml_content).map_err(|e| IrieBookError::FileRead {
            path: "metadata.yaml".into(),
            source: std::io::Error::other(e.to_string()),
        })?;

    // Build ebook-convert command
    let output = Command::new("ebook-convert")
        .arg(input_epub)
        .arg(&output_kindle)
        .arg("--output-profile")
        .arg("kindle_pw")
        .arg("--title")
        .arg(&metadata.title)
        .arg("--authors")
        .arg(&metadata.author)
        .output()
        .map_err(|e| IrieBookError::FileRead {
            path: "ebook-convert".into(),
            source: e,
        })?;

    let convert_output = command::format_output(output);

    // Stamp series metadata if present
    let full_output = match (&metadata.belongs_to_collection, metadata.group_position) {
        (Some(series), Some(index)) => {
            let meta_output = stamp_metadata_impl(Path::new(&output_kindle), series, index)?;
            format!("{}\n{}", convert_output, meta_output)
        }
        _ => convert_output,
    };

    Ok(full_output)
}

/// Implementation of metadata stamping using ebook-meta
fn stamp_metadata_impl(file_path: &Path, series: &str, index: u32) -> Result<String, IrieBookError> {
    let output = Command::new("ebook-meta")
        .arg(file_path)
        .arg("--series")
        .arg(series)
        .arg("--index")
        .arg(index.to_string())
        .output()
        .map_err(|e| IrieBookError::FileRead {
            path: "ebook-meta".into(),
            source: e,
        })?;

    Ok(command::format_output(output))
}

/// Implementation of ebook viewing using ebook-viewer
fn view_ebook_impl(epub_path: &Path) -> Result<String, IrieBookError> {
    // Verify EPUB exists
    if !epub_path.exists() {
        return Err(IrieBookError::FileRead {
            path: epub_path.display().to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "EPUB file not found",
            ),
        });
    }

    // Launch ebook-viewer in background (don't wait)
    Command::new("ebook-viewer")
        .arg(epub_path)
        .spawn()
        .map_err(|e| IrieBookError::FileRead {
            path: "ebook-viewer".into(),
            source: e,
        })?;

    Ok(format!("Launched ebook-viewer for: {}", epub_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibre_converter_implements_trait() {
        let converter = CalibreConverter;
        // This test just verifies the trait is implemented correctly
        // Actual calibre execution would require calibre to be installed
        let _ = converter;
    }
}
