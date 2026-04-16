//! Mock implementations for publication-related traits (Pandoc, Calibre, Archive)
//!
//! These mocks simulate the external tools used for ebook generation.

use iriebook::resource_access::traits::{ArchiveAccess, CalibreAccess, PandocAccess};
use iriebook::utilities::error::IrieBookError;
use std::path::Path;
use std::sync::Mutex;

/// Records of Pandoc operations
#[derive(Debug, Clone, PartialEq)]
pub struct PandocCall {
    pub original_input: String,
    pub fixed_md: String,
    pub output_epub: String,
    pub custom_metadata_content: Option<String>,
    pub embed_cover: bool,
}

/// Mock PandocAccess implementation
pub struct MockPandocAccess {
    /// Whether operations should fail
    pub should_fail: bool,
    /// Error message when failing
    pub error_message: String,
    /// Simulated EPUB content (bytes)
    pub epub_content: Vec<u8>,
    /// Recorded calls
    calls: Mutex<Vec<PandocCall>>,
}

impl Default for MockPandocAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPandocAccess {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            error_message: "Mock Pandoc error".to_string(),
            epub_content: b"MOCK_EPUB_CONTENT".to_vec(),
            calls: Mutex::new(vec![]),
        }
    }

    pub fn with_failure(mut self, message: &str) -> Self {
        self.should_fail = true;
        self.error_message = message.to_string();
        self
    }

    pub fn get_calls(&self) -> Vec<PandocCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl PandocAccess for MockPandocAccess {
    fn convert_to_epub(
        &self,
        original_input: &Path,
        fixed_md: &Path,
        output_epub: &Path,
        custom_metadata_path: Option<&Path>,
        embed_cover: bool,
    ) -> Result<String, IrieBookError> {
        let custom_metadata_content =
            custom_metadata_path.and_then(|path| std::fs::read_to_string(path).ok());

        self.calls.lock().unwrap().push(PandocCall {
            original_input: original_input.to_string_lossy().to_string(),
            fixed_md: fixed_md.to_string_lossy().to_string(),
            output_epub: output_epub.to_string_lossy().to_string(),
            custom_metadata_content,
            embed_cover,
        });

        if self.should_fail {
            return Err(IrieBookError::Validation(format!(
                "Pandoc: {}",
                self.error_message
            )));
        }

        // Simulate writing EPUB file
        if let Some(parent) = output_epub.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(output_epub, &self.epub_content)
            .map_err(|e| IrieBookError::Validation(format!("Pandoc: {}", e)))?;

        Ok("EPUB created successfully".to_string())
    }
}

/// Records of Calibre operations
#[derive(Debug, Clone, PartialEq)]
pub enum CalibreCall {
    ConvertToKindle {
        input_md: String,
        input_epub: String,
    },
    StampMetadata {
        file_path: String,
        series: String,
        index: u32,
    },
    ViewEbook {
        epub_path: String,
    },
}

/// Mock CalibreAccess implementation
pub struct MockCalibreAccess {
    pub should_fail: bool,
    pub error_message: String,
    calls: Mutex<Vec<CalibreCall>>,
}

impl Default for MockCalibreAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCalibreAccess {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            error_message: "Mock Calibre error".to_string(),
            calls: Mutex::new(vec![]),
        }
    }

    pub fn with_failure(mut self, message: &str) -> Self {
        self.should_fail = true;
        self.error_message = message.to_string();
        self
    }

    pub fn get_calls(&self) -> Vec<CalibreCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl CalibreAccess for MockCalibreAccess {
    fn convert_to_kindle(
        &self,
        input_md: &Path,
        input_epub: &Path,
    ) -> Result<String, IrieBookError> {
        self.calls
            .lock()
            .unwrap()
            .push(CalibreCall::ConvertToKindle {
                input_md: input_md.to_string_lossy().to_string(),
                input_epub: input_epub.to_string_lossy().to_string(),
            });

        if self.should_fail {
            return Err(IrieBookError::Validation(format!(
                "Calibre: {}",
                self.error_message
            )));
        }

        // Simulate writing AZW3 file
        let azw3_path = input_epub.with_extension("azw3");
        if let Some(parent) = azw3_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&azw3_path, b"MOCK_AZW3_CONTENT")
            .map_err(|e| IrieBookError::Validation(format!("Calibre: {}", e)))?;

        Ok("Kindle file created".to_string())
    }

    fn stamp_metadata(
        &self,
        file_path: &Path,
        series: &str,
        index: u32,
    ) -> Result<String, IrieBookError> {
        self.calls.lock().unwrap().push(CalibreCall::StampMetadata {
            file_path: file_path.to_string_lossy().to_string(),
            series: series.to_string(),
            index,
        });

        if self.should_fail {
            return Err(IrieBookError::Validation(format!(
                "Calibre: {}",
                self.error_message
            )));
        }

        Ok("Metadata stamped".to_string())
    }

    fn view_ebook(&self, epub_path: &Path) -> Result<String, IrieBookError> {
        self.calls.lock().unwrap().push(CalibreCall::ViewEbook {
            epub_path: epub_path.to_string_lossy().to_string(),
        });

        if self.should_fail {
            return Err(IrieBookError::Validation(format!(
                "Calibre: {}",
                self.error_message
            )));
        }

        Ok("Viewer launched".to_string())
    }
}

/// Records of Archive operations
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveCall {
    pub input_epub: String,
}

/// Mock ArchiveAccess implementation
pub struct MockArchiveAccess {
    pub should_fail: bool,
    pub error_message: String,
    calls: Mutex<Vec<ArchiveCall>>,
}

impl Default for MockArchiveAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl MockArchiveAccess {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            error_message: "Mock Archive error".to_string(),
            calls: Mutex::new(vec![]),
        }
    }

    pub fn with_failure(mut self, message: &str) -> Self {
        self.should_fail = true;
        self.error_message = message.to_string();
        self
    }

    pub fn get_calls(&self) -> Vec<ArchiveCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl ArchiveAccess for MockArchiveAccess {
    fn create_book_archive(&self, input_epub: &Path) -> Result<String, IrieBookError> {
        self.calls.lock().unwrap().push(ArchiveCall {
            input_epub: input_epub.to_string_lossy().to_string(),
        });

        if self.should_fail {
            return Err(IrieBookError::Validation(format!(
                "Archive: {}",
                self.error_message
            )));
        }

        // Simulate creating ZIP archive
        let zip_path = input_epub.with_extension("zip");
        if let Some(parent) = zip_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&zip_path, b"MOCK_ZIP_CONTENT")
            .map_err(|e| IrieBookError::Validation(format!("Archive: {}", e)))?;

        Ok(format!("Archive created: {}", zip_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mock_pandoc_creates_epub() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.epub");

        let mock = MockPandocAccess::new();
        let result = mock.convert_to_epub(
            Path::new("/input/book.md"),
            Path::new("/tmp/fixed.md"),
            &output_path,
            None,
            true,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].embed_cover);
    }

    #[test]
    fn test_mock_calibre_creates_azw3() {
        let temp_dir = TempDir::new().unwrap();
        let epub_path = temp_dir.path().join("book.epub");
        std::fs::write(&epub_path, b"fake epub").unwrap();

        let mock = MockCalibreAccess::new();
        let result = mock.convert_to_kindle(Path::new("/input/book.md"), &epub_path);

        assert!(result.is_ok());

        let azw3_path = epub_path.with_extension("azw3");
        assert!(azw3_path.exists());
    }

    #[test]
    fn test_mock_archive_creates_zip() {
        let temp_dir = TempDir::new().unwrap();
        let epub_path = temp_dir.path().join("book.epub");
        std::fs::write(&epub_path, b"fake epub").unwrap();

        let mock = MockArchiveAccess::new();
        let result = mock.create_book_archive(&epub_path);

        assert!(result.is_ok());

        let zip_path = epub_path.with_extension("zip");
        assert!(zip_path.exists());
    }
}
