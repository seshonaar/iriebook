//! Pandoc access layer for EPUB conversion
//!
//! Provides access to the Pandoc command-line tool for converting
//! markdown files to EPUB format.

use crate::resource_access::traits::PandocAccess;
use crate::resource_access::{command, file};
use crate::utilities::error::IrieBookError;
use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;
use zip::write::FileOptions;

/// Concrete implementation of PandocAccess using the Pandoc command-line tool
pub struct PandocConverter;

impl PandocAccess for PandocConverter {
    fn convert_to_epub(
        &self,
        original_input: &Path,
        fixed_md: &Path,
        output_epub: &Path,
        custom_metadata_path: Option<&Path>,
    ) -> Result<String, IrieBookError> {
        convert_to_epub_impl(original_input, fixed_md, output_epub, custom_metadata_path)
    }
}

/// Implementation of EPUB conversion using Pandoc
///
/// Uses original_input to find metadata.yaml and cover.jpg in the root folder,
/// and fixed_md as the content to convert.
/// If custom_metadata_path is provided, it will be used instead of the book's metadata.yaml.
fn convert_to_epub_impl(
    original_input: &Path,
    fixed_md: &Path,
    output_epub: &Path,
    custom_metadata_path: Option<&Path>,
) -> Result<String, IrieBookError> {
    let css_path = file::get_css_path().map_err(|e| IrieBookError::FileRead {
        path: "css".into(),
        source: std::io::Error::other(e),
    })?;

    // Use custom metadata path if provided, otherwise find metadata.yaml in book folder
    let metadata_path = if let Some(custom_path) = custom_metadata_path {
        custom_path.to_path_buf()
    } else {
        file::get_book_folder_file(original_input, "metadata.yaml").map_err(|e| {
            IrieBookError::FileRead {
                path: "metadata.yaml".into(),
                source: std::io::Error::other(e),
            }
        })?
    };

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

    let mut output = command::format_output(pandoc_output);

    // When using custom metadata (rights removed), we also inject a custom
    // copyright page in markdown. Pandoc still places nav/toc before chapter
    // files, so we reorder EPUB spine to keep frontmatter flow consistent:
    // title -> copyright -> toc -> dedication -> body.
    if custom_metadata_path.is_some() {
        let reordered = reorder_epub_frontmatter_for_custom_copyright(output_epub)?;
        if reordered {
            output.push_str(" | adjusted EPUB spine order");
        }
    }

    Ok(output)
}

#[derive(Debug, Clone)]
struct EpubEntry {
    name: String,
    data: Vec<u8>,
    compression: zip::CompressionMethod,
    unix_mode: Option<u32>,
}

pub(crate) fn reorder_epub_frontmatter_for_custom_copyright(
    epub_path: &Path,
) -> Result<bool, IrieBookError> {
    let mut entries = read_epub_entries(epub_path)?;

    let Some(opf_index) = entries.iter().position(|e| e.name == "EPUB/content.opf") else {
        return Ok(false);
    };

    let Some(copyright_entry) = entries
        .iter()
        .find(|entry| {
            entry.name.starts_with("EPUB/text/")
                && entry.name.ends_with(".xhtml")
                && String::from_utf8_lossy(&entry.data).contains("copyright-page")
        })
        .map(|entry| entry.name.trim_start_matches("EPUB/").to_string())
    else {
        return Ok(false);
    };

    let opf_content = String::from_utf8(entries[opf_index].data.clone()).map_err(|e| {
        IrieBookError::FileRead {
            path: format!("{}::EPUB/content.opf", epub_path.display()),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        }
    })?;

    let Some(updated_opf) = reorder_spine_toc_after_href(&opf_content, &copyright_entry) else {
        return Ok(false);
    };

    entries[opf_index].data = updated_opf.into_bytes();
    write_epub_entries(epub_path, &entries)?;

    Ok(true)
}

fn reorder_spine_toc_after_href(opf_content: &str, target_href: &str) -> Option<String> {
    let item_tag_re = Regex::new(r#"<item\b[^>]*>"#).ok()?;

    let mut nav_id: Option<String> = None;
    let mut target_id: Option<String> = None;

    for m in item_tag_re.find_iter(opf_content) {
        let tag = m.as_str();
        let id = extract_attr(tag, "id")?;

        if let Some(properties) = extract_attr(tag, "properties")
            && properties.split_whitespace().any(|p| p == "nav")
        {
            nav_id = Some(id.clone());
        }

        if let Some(href) = extract_attr(tag, "href")
            && href == target_href
        {
            target_id = Some(id.clone());
        }
    }

    let nav_id = nav_id?;
    let target_id = target_id?;

    let spine_re = Regex::new(r#"(?s)(<spine\b[^>]*>)(.*?)(</spine>)"#).ok()?;
    let captures = spine_re.captures(opf_content)?;

    let full_match = captures.get(0)?;
    let spine_open = captures.get(1)?.as_str();
    let spine_inner = captures.get(2)?.as_str();
    let spine_close = captures.get(3)?.as_str();

    let mut lines: Vec<String> = spine_inner.lines().map(ToString::to_string).collect();
    let nav_marker = format!("idref=\"{}\"", nav_id);
    let target_marker = format!("idref=\"{}\"", target_id);

    let nav_index = lines
        .iter()
        .position(|line| line.contains("<itemref") && line.contains(&nav_marker))?;
    let target_index = lines
        .iter()
        .position(|line| line.contains("<itemref") && line.contains(&target_marker))?;

    if nav_index == target_index + 1 {
        return None;
    }

    let nav_line = lines.remove(nav_index);
    let mut insert_after = target_index;
    if nav_index < target_index {
        insert_after -= 1;
    }
    lines.insert(insert_after + 1, nav_line);

    let rebuilt_spine = format!("{}{}{}", spine_open, lines.join("\n"), spine_close);

    let mut updated = String::new();
    updated.push_str(&opf_content[..full_match.start()]);
    updated.push_str(&rebuilt_spine);
    updated.push_str(&opf_content[full_match.end()..]);

    Some(updated)
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let attr_re = Regex::new(&format!(r#"\b{}=\"([^\"]+)\""#, attr)).ok()?;
    attr_re
        .captures(tag)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn read_epub_entries(epub_path: &Path) -> Result<Vec<EpubEntry>, IrieBookError> {
    let file = File::open(epub_path).map_err(|e| IrieBookError::FileRead {
        path: epub_path.display().to_string(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| IrieBookError::FileRead {
        path: epub_path.display().to_string(),
        source: std::io::Error::other(e),
    })?;

    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| IrieBookError::FileRead {
                path: epub_path.display().to_string(),
                source: std::io::Error::other(e),
            })?;

        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|e| IrieBookError::FileRead {
                path: format!("{}::{}", epub_path.display(), entry.name()),
                source: e,
            })?;

        entries.push(EpubEntry {
            name: entry.name().to_string(),
            data,
            compression: entry.compression(),
            unix_mode: entry.unix_mode(),
        });
    }

    Ok(entries)
}

fn write_epub_entries(epub_path: &Path, entries: &[EpubEntry]) -> Result<(), IrieBookError> {
    let temp_path = PathBuf::from(format!("{}.tmp", epub_path.display()));

    let temp_file = File::create(&temp_path).map_err(|e| IrieBookError::FileWrite {
        path: temp_path.display().to_string(),
        source: e,
    })?;

    let mut writer = zip::ZipWriter::new(temp_file);

    for entry in entries {
        let mut options = FileOptions::<()>::default().compression_method(entry.compression);
        if let Some(mode) = entry.unix_mode {
            options = options.unix_permissions(mode);
        }

        writer
            .start_file(&entry.name, options)
            .map_err(|e| IrieBookError::FileWrite {
                path: format!("{}::{}", temp_path.display(), entry.name),
                source: std::io::Error::other(e),
            })?;

        writer
            .write_all(&entry.data)
            .map_err(|e| IrieBookError::FileWrite {
                path: format!("{}::{}", temp_path.display(), entry.name),
                source: e,
            })?;
    }

    writer.finish().map_err(|e| IrieBookError::FileWrite {
        path: temp_path.display().to_string(),
        source: std::io::Error::other(e),
    })?;

    std::fs::rename(&temp_path, epub_path).map_err(|e| IrieBookError::FileWrite {
        path: epub_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    #[test]
    fn pandoc_converter_implements_trait() {
        let converter = PandocConverter;
        // This test just verifies the trait is implemented correctly
        // Actual pandoc execution would require pandoc to be installed
        let _ = converter;
    }

    #[test]
    fn reorders_toc_after_copyright_page_in_epub_spine() {
        let temp_dir = TempDir::new().unwrap();
        let epub_path = temp_dir.path().join("book.epub");

        let file = std::fs::File::create(&epub_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();

        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav" />
    <item id="ch001_xhtml" href="text/ch001.xhtml" media-type="application/xhtml+xml" />
    <item id="ch002_xhtml" href="text/ch002.xhtml" media-type="application/xhtml+xml" />
  </manifest>
  <spine>
    <itemref idref="nav" />
    <itemref idref="ch001_xhtml" />
    <itemref idref="ch002_xhtml" />
  </spine>
</package>
"#;

        let copyright_page =
            r#"<html><body><section class="copyright-page"></section></body></html>"#;
        let dedication_page =
            r#"<html><body><section class="dedication-page"></section></body></html>"#;

        zip.start_file("EPUB/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();
        zip.start_file("EPUB/text/ch001.xhtml", options).unwrap();
        zip.write_all(copyright_page.as_bytes()).unwrap();
        zip.start_file("EPUB/text/ch002.xhtml", options).unwrap();
        zip.write_all(dedication_page.as_bytes()).unwrap();
        zip.finish().unwrap();

        let changed = reorder_epub_frontmatter_for_custom_copyright(&epub_path).unwrap();
        assert!(changed, "Expected EPUB spine to be reordered");

        let file = std::fs::File::open(&epub_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut updated_opf = String::new();
        archive
            .by_name("EPUB/content.opf")
            .unwrap()
            .read_to_string(&mut updated_opf)
            .unwrap();

        let nav_idx = updated_opf.find("<itemref idref=\"nav\" />").unwrap();
        let copyright_idx = updated_opf
            .find("<itemref idref=\"ch001_xhtml\" />")
            .unwrap();
        assert!(
            nav_idx > copyright_idx,
            "Expected nav itemref to come after copyright itemref"
        );
    }
}
