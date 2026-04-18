//! Archive access layer for creating ZIP archives
//!
//! Provides functionality to create ZIP archives containing both
//! EPUB and Kindle (AZW3) versions of the ebook.

use crate::resource_access::file;
use crate::resource_access::traits::ArchiveAccess;
use crate::utilities::error::IrieBookError;
use std::fs::File;
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::FileOptions;

/// Concrete implementation of ArchiveAccess for creating ZIP files
pub struct ZipArchiver;

impl ArchiveAccess for ZipArchiver {
    fn create_book_archive(
        &self,
        input_epub: &Path,
        input_pdf: Option<&Path>,
    ) -> Result<String, IrieBookError> {
        create_archive_impl(input_epub, input_pdf)
    }
}

/// Implementation of ZIP archive creation
fn create_archive_impl(
    input_epub: &Path,
    input_pdf: Option<&Path>,
) -> Result<String, IrieBookError> {
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

    add_file_to_archive(&mut zip, &zip_path, input_epub, options)?;
    add_file_to_archive(&mut zip, &zip_path, &azw3_path, options)?;

    if let Some(pdf_path) = input_pdf.filter(|path| path.exists()) {
        add_file_to_archive(&mut zip, &zip_path, pdf_path, options)?;
    }

    zip.finish().map_err(|e| IrieBookError::FileWrite {
        path: zip_path.display().to_string(),
        source: std::io::Error::other(e),
    })?;

    Ok(format!("📦 Archive created: {}", zip_path.display()))
}

fn add_file_to_archive(
    zip: &mut ZipWriter<File>,
    zip_path: &Path,
    input_path: &Path,
    options: FileOptions<()>,
) -> Result<(), IrieBookError> {
    let filename = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| IrieBookError::OutputPathError {
            input: input_path.display().to_string(),
        })?;

    zip.start_file(filename, options)
        .map_err(|e| IrieBookError::FileWrite {
            path: zip_path.display().to_string(),
            source: std::io::Error::other(e),
        })?;

    let mut input_file = File::open(input_path).map_err(|e| IrieBookError::FileRead {
        path: input_path.display().to_string(),
        source: e,
    })?;

    let mut buffer = Vec::new();
    input_file
        .read_to_end(&mut buffer)
        .map_err(|e| IrieBookError::FileRead {
            path: input_path.display().to_string(),
            source: e,
        })?;

    zip.write_all(&buffer)
        .map_err(|e| IrieBookError::FileWrite {
            path: zip_path.display().to_string(),
            source: e,
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn zip_archiver_implements_trait() {
        let archiver = ZipArchiver;
        // This test just verifies the trait is implemented correctly
        let _ = archiver;
    }

    #[test]
    fn archive_includes_pdf_when_present() {
        let temp_dir = TempDir::new().unwrap();
        let epub_path = temp_dir.path().join("book.epub");
        let azw3_path = temp_dir.path().join("book.azw3");
        let pdf_path = temp_dir.path().join("book.pdf");
        std::fs::write(&epub_path, b"epub").unwrap();
        std::fs::write(&azw3_path, b"azw3").unwrap();
        std::fs::write(&pdf_path, b"pdf").unwrap();

        create_archive_impl(&epub_path, Some(&pdf_path)).unwrap();

        let zip_path = temp_dir.path().join("book.zip");
        let zip_file = File::open(zip_path).unwrap();
        let mut zip = zip::ZipArchive::new(zip_file).unwrap();

        assert!(zip.by_name("book.epub").is_ok());
        assert!(zip.by_name("book.azw3").is_ok());
        assert!(zip.by_name("book.pdf").is_ok());
    }
}
