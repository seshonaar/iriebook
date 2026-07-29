//! Book output discovery.

use anyhow::{Context, Result};
use iriebook::resource_access::file;
use iriebook::utilities::types::PdfPageProfile;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum BookOutputFormat {
    Epub,
    Pdf,
    Azw3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BookOutputLink {
    pub format: BookOutputFormat,
    pub path: String,
    pub display_name: String,
}

pub fn get_available_book_outputs(book_path: &Path) -> Result<Vec<BookOutputLink>> {
    let epub_path = file::get_output_file_name(book_path).context(
        "Failed to get output paths. Ensure metadata.yaml exists with title and author fields.",
    )?;

    let pdf_path = epub_path.with_extension("pdf");
    let azw3_path = epub_path.with_extension("azw3");

    let mut outputs = Vec::new();

    if epub_path.exists() {
        outputs.push(BookOutputLink {
            format: BookOutputFormat::Epub,
            path: epub_path.to_string_lossy().into_owned(),
            display_name: "epub".to_string(),
        });
    }

    if pdf_path.exists() {
        outputs.push(BookOutputLink {
            format: BookOutputFormat::Pdf,
            path: pdf_path.to_string_lossy().into_owned(),
            display_name: "pdf".to_string(),
        });
    }

    for profile in PdfPageProfile::ALL {
        let profile_name = file::pdf_profile_folder_name(profile);
        let profile_pdf_path = epub_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(profile_name)
            .join(epub_path.file_name().unwrap_or_default())
            .with_extension("pdf");

        if profile_pdf_path.exists() {
            outputs.push(BookOutputLink {
                format: BookOutputFormat::Pdf,
                path: profile_pdf_path.to_string_lossy().into_owned(),
                display_name: pdf_profile_display_name(book_path, profile),
            });
        }
    }

    if azw3_path.exists() {
        outputs.push(BookOutputLink {
            format: BookOutputFormat::Azw3,
            path: azw3_path.to_string_lossy().into_owned(),
            display_name: "azw3".to_string(),
        });
    }

    Ok(outputs)
}

fn pdf_profile_display_name(book_path: &Path, profile: PdfPageProfile) -> String {
    let profile_document = file::get_pdf_profile_file(book_path, profile, "profile.json")
        .ok()
        .and_then(|profile_path| fs::read_to_string(profile_path).ok())
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|value| {
            value
                .get("label")
                .and_then(|label| label.as_str())
                .map(str::to_string)
        });

    profile_document
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| profile.label().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_available_book_outputs_requires_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();

        let result = get_available_book_outputs(&book_path);
        assert!(
            result.is_err(),
            "Expected error when metadata.yaml is missing"
        );
    }

    #[test]
    fn test_get_available_book_outputs_only_returns_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();
        fs::write(
            temp_dir.path().join("metadata.yaml"),
            "title: Test Book\nauthor: Test Author\n",
        )
        .unwrap();

        let epub_path = file::get_output_file_name(&book_path).unwrap();
        fs::write(&epub_path, "epub").unwrap();
        fs::write(epub_path.with_extension("pdf"), "pdf").unwrap();

        let outputs = get_available_book_outputs(&book_path).unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].format, BookOutputFormat::Epub);
        assert_eq!(outputs[0].display_name, "epub");
        assert_eq!(outputs[1].format, BookOutputFormat::Pdf);
        assert_eq!(outputs[1].display_name, "pdf");
    }

    #[test]
    fn test_get_available_book_outputs_returns_profile_pdf_files() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();
        fs::write(
            temp_dir.path().join("metadata.yaml"),
            "title: Test Book\nauthor: Test Author\n",
        )
        .unwrap();

        let epub_path = file::get_output_file_name(&book_path).unwrap();
        let output_dir = epub_path.parent().unwrap();
        let bookbite_pdf_path = output_dir
            .join("bookbite")
            .join(epub_path.file_name().unwrap())
            .with_extension("pdf");
        let draft_pdf_path = output_dir
            .join("draft_2_digital")
            .join(epub_path.file_name().unwrap())
            .with_extension("pdf");
        fs::create_dir_all(bookbite_pdf_path.parent().unwrap()).unwrap();
        fs::create_dir_all(draft_pdf_path.parent().unwrap()).unwrap();
        fs::write(&bookbite_pdf_path, "bookbite pdf").unwrap();
        fs::write(&draft_pdf_path, "draft pdf").unwrap();

        let outputs = get_available_book_outputs(&book_path).unwrap();

        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].format, BookOutputFormat::Pdf);
        assert_eq!(outputs[0].display_name, "Draft2Digital");
        assert_eq!(outputs[0].path, draft_pdf_path.to_string_lossy());
        assert_eq!(outputs[1].format, BookOutputFormat::Pdf);
        assert_eq!(outputs[1].display_name, "Bookbite");
        assert_eq!(outputs[1].path, bookbite_pdf_path.to_string_lossy());
    }

    #[test]
    fn test_get_available_book_outputs_uses_profile_json_label() {
        let temp_dir = TempDir::new().unwrap();
        let book_path = temp_dir.path().join("test.md");
        fs::write(&book_path, "# Test Book").unwrap();
        fs::write(
            temp_dir.path().join("metadata.yaml"),
            "title: Test Book\nauthor: Test Author\n",
        )
        .unwrap();

        let epub_path = file::get_output_file_name(&book_path).unwrap();
        let bookbite_pdf_path = epub_path
            .parent()
            .unwrap()
            .join("bookbite")
            .join(epub_path.file_name().unwrap())
            .with_extension("pdf");
        let bookbite_profile_path = temp_dir.path().join("profiles/pdf/bookbite/profile.json");
        fs::create_dir_all(bookbite_pdf_path.parent().unwrap()).unwrap();
        fs::create_dir_all(bookbite_profile_path.parent().unwrap()).unwrap();
        fs::write(&bookbite_pdf_path, "bookbite pdf").unwrap();
        fs::write(
            bookbite_profile_path,
            r#"{
  "id": "bookbite",
  "label": "Bookbite print",
  "page": { "width": "148mm", "height": "210mm" },
  "render": { "pdf_engine": "xelatex" },
  "copyright": { "type": "standard" }
}"#,
        )
        .unwrap();

        let outputs = get_available_book_outputs(&book_path).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].format, BookOutputFormat::Pdf);
        assert_eq!(outputs[0].display_name, "Bookbite print");
        assert_eq!(outputs[0].path, bookbite_pdf_path.to_string_lossy());
    }
}
