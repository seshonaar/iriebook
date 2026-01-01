//! Archive access layer for creating ZIP archives
//!
//! Provides functionality to create ZIP archives containing both
//! EPUB and Kindle (AZW3) versions of the ebook.

use crate::utilities::error::IrieBookError;
use crate::resource_access::file;
use crate::resource_access::traits::ArchiveAccess;
use std::fs::File;
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::ZipWriter;

/// Concrete implementation of ArchiveAccess for creating ZIP files
pub struct ZipArchiver;

impl ArchiveAccess for ZipArchiver {
    fn create_book_archive(&self, input_epub: &Path) -> Result<String, IrieBookError> {
        create_archive_impl(input_epub)
    }
}

/// Implementation of ZIP archive creation
fn create_archive_impl(input_epub: &Path) -> Result<String, IrieBookError> {
    let azw3_path = PathBuf::from(file::change_extension(input_epub, "azw3"));
    let zip_path = PathBuf::from(file::change_extension(input_epub, "zip"));

    let zip_file = File::create(&zip_path).map_err(|e| IrieBookError::FileWrite {
        path: zip_path.display().to_string(),
        source: e,
    })?;

    let mut zip = ZipWriter::new(zip_file);

    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // Add EPUB to archive
    let epub_filename = input_epub
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| IrieBookError::OutputPathError {
            input: input_epub.display().to_string(),
        })?;

    zip.start_file(epub_filename, options)
        .map_err(|e| IrieBookError::FileWrite {
            path: zip_path.display().to_string(),
            source: std::io::Error::other(e),
        })?;

    let mut epub_file = File::open(input_epub).map_err(|e| IrieBookError::FileRead {
        path: input_epub.display().to_string(),
        source: e,
    })?;

    let mut epub_buffer = Vec::new();
    epub_file
        .read_to_end(&mut epub_buffer)
        .map_err(|e| IrieBookError::FileRead {
            path: input_epub.display().to_string(),
            source: e,
        })?;

    zip.write_all(&epub_buffer)
        .map_err(|e| IrieBookError::FileWrite {
            path: zip_path.display().to_string(),
            source: e,
        })?;

    // Add AZW3 to archive
    let azw3_filename = azw3_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| IrieBookError::OutputPathError {
            input: azw3_path.display().to_string(),
        })?;

    zip.start_file(azw3_filename, options)
        .map_err(|e| IrieBookError::FileWrite {
            path: zip_path.display().to_string(),
            source: std::io::Error::other(e),
        })?;

    let mut azw3_file = File::open(&azw3_path).map_err(|e| IrieBookError::FileRead {
        path: azw3_path.display().to_string(),
        source: e,
    })?;

    let mut azw3_buffer = Vec::new();
    azw3_file
        .read_to_end(&mut azw3_buffer)
        .map_err(|e| IrieBookError::FileRead {
            path: azw3_path.display().to_string(),
            source: e,
        })?;

    zip.write_all(&azw3_buffer)
        .map_err(|e| IrieBookError::FileWrite {
            path: zip_path.display().to_string(),
            source: e,
        })?;

    zip.finish().map_err(|e| IrieBookError::FileWrite {
        path: zip_path.display().to_string(),
        source: std::io::Error::other(e),
    })?;

    Ok(format!("📦 Archive created: {}", zip_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_archiver_implements_trait() {
        let archiver = ZipArchiver;
        // This test just verifies the trait is implemented correctly
        let _ = archiver;
    }
}
