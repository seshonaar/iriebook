//! Pandoc access layer for EPUB conversion
//!
//! Provides access to the Pandoc command-line tool for converting
//! markdown files to EPUB format.

use crate::resource_access::config::PdfConfig;
use crate::resource_access::traits::PandocAccess;
use crate::resource_access::{command, file};
use crate::utilities::error::IrieBookError;
use image::GenericImageView;
use regex::Regex;
use std::fs;
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
        embed_cover: bool,
    ) -> Result<String, IrieBookError> {
        convert_to_epub_impl(
            original_input,
            fixed_md,
            output_epub,
            custom_metadata_path,
            embed_cover,
        )
    }

    fn convert_to_pdf(
        &self,
        original_input: &Path,
        fixed_md: &Path,
        output_pdf: &Path,
        metadata_path: &Path,
        pdf_config: &PdfConfig,
        embed_cover: bool,
    ) -> Result<String, IrieBookError> {
        convert_to_pdf_impl(
            original_input,
            fixed_md,
            output_pdf,
            metadata_path,
            pdf_config,
            embed_cover,
        )
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
    embed_cover: bool,
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

    let cover_path = resolve_cover_path(original_input, embed_cover)?;

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

    // Convert the fixed markdown content
    debug!(fixed_md = %fixed_md.display(), css = %css_path, "Starting EPUB conversion");
    let mut command = build_pandoc_command(
        fixed_md,
        output_epub,
        &css_path,
        &metadata_path,
        cover_path.as_deref(),
    );
    let pandoc_output = command.output().map_err(|e| IrieBookError::FileRead {
        path: "pandoc".into(),
        source: e,
    })?;

    let mut output = command::format_output(pandoc_output);

    if let Some((series_name, series_position)) =
        read_title_page_series_from_metadata(&metadata_path)?
    {
        let series_added =
            apply_title_page_series_to_epub(output_epub, &series_name, series_position)?;
        if series_added {
            output.push_str(" | injected series info on title page");
        }
    }

    if let Some(style) = read_title_page_style_from_metadata(&metadata_path)? {
        let styled = apply_title_page_style_to_epub(output_epub, &style)?;
        if styled {
            output.push_str(" | applied title-page style");
        }
    }

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

/// Implementation of PDF conversion using Pandoc and a configured PDF engine.
fn convert_to_pdf_impl(
    original_input: &Path,
    fixed_md: &Path,
    output_pdf: &Path,
    metadata_path: &Path,
    pdf_config: &PdfConfig,
    embed_cover: bool,
) -> Result<String, IrieBookError> {
    if !metadata_path.exists() {
        return Err(IrieBookError::FileRead {
            path: metadata_path.to_string_lossy().to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "metadata.yaml file not found",
            ),
        });
    }

    debug!(
        fixed_md = %fixed_md.display(),
        output_pdf = %output_pdf.display(),
        "Starting PDF conversion"
    );
    let cover_path = resolve_cover_path(original_input, embed_cover)?;
    let pdf_header_path = write_pdf_latex_header(output_pdf, cover_path.as_deref(), pdf_config)?;
    let pdf_filter_path = write_pdf_lua_filter(output_pdf)?;

    let mut command = build_pdf_pandoc_command(
        fixed_md,
        output_pdf,
        metadata_path,
        pdf_config,
        &pdf_header_path,
        &pdf_filter_path,
    );
    let pandoc_output = command.output().map_err(|e| IrieBookError::FileRead {
        path: "pandoc".into(),
        source: e,
    })?;

    let _ = std::fs::remove_file(&pdf_header_path);
    let _ = std::fs::remove_file(&pdf_filter_path);

    let output = command::format_output(pandoc_output);
    if output.starts_with("Error:") || output.starts_with("Command failed") {
        return Err(IrieBookError::Validation(format!(
            "PDF generation failed. Ensure Pandoc can run the configured PDF engine '{}' and the font '{}' is installed, or edit config.json pdf settings. Liberation Serif is available via the fonts-liberation package on Ubuntu. If fc-match still points to a manually installed variable font, remove that font and run fc-cache -r -f. {}",
            pdf_config.pdf_engine, pdf_config.font_family, output
        )));
    }

    Ok(output)
}

fn resolve_cover_path(
    original_input: &Path,
    embed_cover: bool,
) -> Result<Option<PathBuf>, IrieBookError> {
    if !embed_cover {
        return Ok(None);
    }

    let cover_path = file::get_book_folder_file(original_input, "cover.jpg").map_err(|e| {
        IrieBookError::FileRead {
            path: "cover.jpg".into(),
            source: std::io::Error::other(e),
        }
    })?;

    if !cover_path.exists() {
        return Err(IrieBookError::FileRead {
            path: cover_path.to_string_lossy().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "cover.jpg file not found"),
        });
    }

    Ok(Some(cover_path))
}

fn build_pandoc_command(
    fixed_md: &Path,
    output_epub: &Path,
    css_path: &str,
    metadata_path: &Path,
    cover_path: Option<&Path>,
) -> Command {
    let mut command = Command::new("pandoc");
    command
        .arg(fixed_md)
        .arg("-o")
        .arg(output_epub)
        .arg("--toc")
        .arg("-t")
        .arg("epub3")
        .arg("--css")
        .arg(css_path)
        .arg("--metadata-file")
        .arg(metadata_path);

    if let Some(cover_path) = cover_path {
        command.arg("--epub-cover-image").arg(cover_path);
    }

    command.arg("--standalone").arg("--split-level=1");
    command
}

fn build_pdf_pandoc_command(
    fixed_md: &Path,
    output_pdf: &Path,
    metadata_path: &Path,
    pdf_config: &PdfConfig,
    pdf_header_path: &Path,
    pdf_filter_path: &Path,
) -> Command {
    let mut command = Command::new("pandoc");
    command
        .arg(fixed_md)
        .arg("-o")
        .arg(output_pdf)
        .arg("--toc")
        .arg("--pdf-engine")
        .arg(&pdf_config.pdf_engine)
        .arg("--metadata-file")
        .arg(metadata_path)
        .arg("--standalone")
        .arg("--include-in-header")
        .arg(pdf_header_path)
        .arg("--lua-filter")
        .arg(pdf_filter_path)
        .arg("-V")
        .arg(format!("mainfont={}", pdf_config.font_family))
        .arg("-V")
        .arg(format!("fontsize={}", pdf_config.font_size))
        .arg("-V")
        .arg(format!("linestretch={}", pdf_config.line_spacing))
        .arg("-V")
        .arg(format!("geometry:paperwidth={}", pdf_config.page_width))
        .arg("-V")
        .arg(format!("geometry:paperheight={}", pdf_config.page_height))
        .arg("-V")
        .arg(format!("geometry:inner={}", pdf_config.inner_margin))
        .arg("-V")
        .arg(format!("geometry:outer={}", pdf_config.outer_margin))
        .arg("-V")
        .arg(format!("geometry:top={}", pdf_config.top_margin))
        .arg("-V")
        .arg(format!("geometry:bottom={}", pdf_config.bottom_margin));

    if !pdf_config.justified {
        command.arg("-V").arg("documentclass=scrbook");
        command.arg("-V").arg("classoption=DIV=calc");
        command.arg("-V").arg("include-before=\\raggedright");
    }

    command
}

fn write_pdf_latex_header(
    output_pdf: &Path,
    cover_path: Option<&Path>,
    pdf_config: &PdfConfig,
) -> Result<PathBuf, IrieBookError> {
    let header_path = output_pdf.with_extension("pdf-style.tex");
    if let Some(parent) = header_path.parent() {
        fs::create_dir_all(parent).map_err(|e| IrieBookError::FileWrite {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    let cover_command = match cover_path {
        Some(path) => build_pdf_cover_command(path, pdf_config)?,
        None => String::new(),
    };

    let include = format!(
        r#"\usepackage{{graphicx}}
\usepackage{{geometry}}
\usepackage{{titlesec}}
\usepackage{{fontspec}}
\usepackage{{etoolbox}}
\usepackage{{indentfirst}}

\setlength{{\parindent}}{{1.5em}}
\setlength{{\parskip}}{{0pt}}
\pagestyle{{empty}}
\raggedbottom

\newfontfamily\iriesymbolfont{{DejaVu Sans}}

\titleformat{{\section}}
  {{\normalfont\Large\bfseries\centering\MakeUppercase}}
  {{}}
  {{0pt}}
  {{}}
\titlespacing*{{\section}}{{0pt}}{{0pt}}{{4em}}

\newif\ifirieMainMatterStarted
\newif\ifirieInToc
\pretocmd{{\section}}{{%
  \ifirieInToc
  \else
    \ifirieMainMatterStarted
      \clearpage
      \vspace*{{0.15\textheight}}
    \else
      \clearpage
      \global\irieMainMatterStartedtrue
      \pagestyle{{plain}}
      \thispagestyle{{plain}}
      \vspace*{{0.15\textheight}}
    \fi
  \fi
}}{{}}{{}}

\newenvironment{{irieSubtitle}}
  {{\begin{{center}}\vspace{{-3.2em}}\itshape\large}}
  {{\par\end{{center}}\vspace{{2em}}}}

\newcommand{{\irieSceneBreak}}{{%
  \par\vspace{{1em}}%
  \begin{{center}}{{\iriesymbolfont\Large ❦}}\end{{center}}%
  \vspace{{1em}}\par%
}}

\newcommand{{\irieBlankPage}}{{%
  \thispagestyle{{empty}}%
  \null%
  \newpage%
}}

\newcommand{{\irieDedicationOpening}}{{%
  \clearpage%
  \ifodd\value{{page}}%
    \irieBlankPage%
  \fi%
  \irieBlankPage%
}}

\newenvironment{{irieDedication}}
  {{\irieDedicationOpening\thispagestyle{{empty}}\vspace*{{\fill}}\begin{{center}}\itshape\large}}
  {{\end{{center}}\vspace*{{\fill}}\clearpage\irieBlankPage}}

\newenvironment{{irieCopyright}}
  {{\clearpage\thispagestyle{{empty}}\begingroup\setlength{{\parindent}}{{0pt}}\small\vspace*{{4em}}}}
  {{\par\endgroup\clearpage}}

\newenvironment{{irieCopyrightDisclaimer}}
  {{\par\small\setlength{{\parindent}}{{0pt}}}}
  {{\par}}

\makeatletter
\let\irieOriginalTableOfContents\tableofcontents
\renewcommand{{\tableofcontents}}{{%
  \clearpage
  \begingroup
  \irieInToctrue
  \renewcommand{{\contentsname}}{{\@title}}
  \pagestyle{{empty}}
  \thispagestyle{{empty}}
  \irieOriginalTableOfContents
  \clearpage
  \endgroup
  \irieInTocfalse
}}

\renewcommand{{\maketitle}}{{%
  \begin{{titlepage}}
  \thispagestyle{{empty}}
  \pagestyle{{empty}}
  \centering
  \vspace*{{5em}}
  {{\huge\bfseries\MakeUppercase{{\@title}}\par}}
  \vspace{{2.6em}}
  {{\large\MakeUppercase{{\@author}}\par}}
  \vfill
  \end{{titlepage}}
  \setcounter{{page}}{{1}}
  \pagestyle{{empty}}
}}
\makeatother

{}
"#,
        cover_command
    );

    fs::write(&header_path, include).map_err(|e| IrieBookError::FileWrite {
        path: header_path.display().to_string(),
        source: e,
    })?;

    Ok(header_path)
}

fn build_pdf_cover_command(
    cover_path: &Path,
    pdf_config: &PdfConfig,
) -> Result<String, IrieBookError> {
    let (width, height) = image::ImageReader::open(cover_path)
        .map_err(|e| IrieBookError::FileRead {
            path: cover_path.display().to_string(),
            source: e,
        })?
        .with_guessed_format()
        .map_err(|e| IrieBookError::FileRead {
            path: cover_path.display().to_string(),
            source: e,
        })?
        .decode()
        .map_err(|e| IrieBookError::FileRead {
            path: cover_path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?
        .dimensions();

    let page_width = parse_pdf_dimension_inches(&pdf_config.page_width).unwrap_or(5.5);
    let page_height = parse_pdf_dimension_inches(&pdf_config.page_height).unwrap_or(8.5);
    let page_ratio = page_width / page_height;
    let image_ratio = f64::from(width) / f64::from(height);

    let image_options = if image_ratio > page_ratio {
        let target_width = f64::from(height) * page_ratio;
        let trim = ((f64::from(width) - target_width) / 2.0).max(0.0);
        format!(
            "height=\\paperheight,trim={:.2}bp 0bp {:.2}bp 0bp,clip",
            trim, trim
        )
    } else {
        let target_height = f64::from(width) / page_ratio;
        let trim = ((f64::from(height) - target_height) / 2.0).max(0.0);
        format!(
            "width=\\paperwidth,trim=0bp {:.2}bp 0bp {:.2}bp,clip",
            trim, trim
        )
    };

    Ok(format!(
        r#"\newcommand{{\irieCoverPage}}{{%
  \clearpage
  \thispagestyle{{empty}}
  \newgeometry{{margin=0pt}}
  \noindent\makebox[\paperwidth][c]{{\includegraphics[{}]{{{}}}}}
  \restoregeometry
  \clearpage
  \setcounter{{page}}{{1}}
  \pagestyle{{empty}}
}}
\AtBeginDocument{{\irieCoverPage}}
"#,
        image_options,
        escape_latex_path(cover_path)
    ))
}

fn parse_pdf_dimension_inches(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let split_at =
        trimmed.find(|character: char| !(character.is_ascii_digit() || character == '.'))?;
    let (number, unit) = trimmed.split_at(split_at);
    let number = number.parse::<f64>().ok()?;
    match unit.trim().to_ascii_lowercase().as_str() {
        "in" => Some(number),
        "cm" => Some(number / 2.54),
        "mm" => Some(number / 25.4),
        "pt" => Some(number / 72.0),
        _ => None,
    }
}

fn write_pdf_lua_filter(output_pdf: &Path) -> Result<PathBuf, IrieBookError> {
    let filter_path = output_pdf.with_extension("pdf-style.lua");
    if let Some(parent) = filter_path.parent() {
        fs::create_dir_all(parent).map_err(|e| IrieBookError::FileWrite {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    fs::write(&filter_path, PDF_STYLE_LUA_FILTER).map_err(|e| IrieBookError::FileWrite {
        path: filter_path.display().to_string(),
        source: e,
    })?;

    Ok(filter_path)
}

const PDF_STYLE_LUA_FILTER: &str = r#"
local function has_class(el, class)
  for _, value in ipairs(el.classes or {}) do
    if value == class then
      return true
    end
  end
  return false
end

function Header(el)
  if has_class(el, 'dedication-page') or has_class(el, 'copyright-page') then
    return {}
  end
  return nil
end

function Div(el)
  if has_class(el, 'scene-break') then
    return pandoc.RawBlock('latex', '\\irieSceneBreak{}')
  end

  if has_class(el, 'dedication') then
    local blocks = { pandoc.RawBlock('latex', '\\begin{irieDedication}') }
    for _, block in ipairs(el.content) do
      table.insert(blocks, block)
    end
    table.insert(blocks, pandoc.RawBlock('latex', '\\end{irieDedication}'))
    return blocks
  end

  if has_class(el, 'copyright') then
    local blocks = { pandoc.RawBlock('latex', '\\begin{irieCopyright}') }
    for _, block in ipairs(el.content) do
      table.insert(blocks, block)
    end
    table.insert(blocks, pandoc.RawBlock('latex', '\\end{irieCopyright}'))
    return blocks
  end

  if has_class(el, 'copyright-disclaimer') then
    local blocks = { pandoc.RawBlock('latex', '\\begin{irieCopyrightDisclaimer}') }
    for _, block in ipairs(el.content) do
      table.insert(blocks, block)
    end
    table.insert(blocks, pandoc.RawBlock('latex', '\\end{irieCopyrightDisclaimer}'))
    return blocks
  end

  return nil
end

function Blocks(blocks)
  local next_blocks = {}
  local index = 1

  while index <= #blocks do
    local block = blocks[index]
    local next_block = blocks[index + 1]
    local close_block = blocks[index + 2]

    if block.t == 'RawBlock'
      and block.format:match('html')
      and block.text:match([=[<p%s+class=["']subtitle["']]=])
      and next_block ~= nil
      and close_block ~= nil
      and close_block.t == 'RawBlock'
      and close_block.format:match('html')
      and close_block.text:match('</p>') then
      table.insert(next_blocks, pandoc.RawBlock('latex', '\\begin{irieSubtitle}'))
      table.insert(next_blocks, next_block)
      table.insert(next_blocks, pandoc.RawBlock('latex', '\\end{irieSubtitle}'))
      index = index + 3
    else
      table.insert(next_blocks, block)
      index = index + 1
    end
  end

  return next_blocks
end
"#;

fn escape_latex_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('%', "\\%")
        .replace('#', "\\#")
        .replace('&', "\\&")
}

fn read_title_page_style_from_metadata(
    metadata_path: &Path,
) -> Result<Option<String>, IrieBookError> {
    let yaml_value = read_metadata_yaml_value(metadata_path)?;

    let style = yaml_value
        .as_mapping()
        .and_then(|mapping| mapping.get(serde_yaml::Value::String("title-page-style".to_string())))
        .and_then(|value| value.as_str())
        .and_then(normalize_title_page_style)
        .map(ToString::to_string);

    Ok(style)
}

fn read_title_page_series_from_metadata(
    metadata_path: &Path,
) -> Result<Option<(String, Option<u32>)>, IrieBookError> {
    let yaml_value = read_metadata_yaml_value(metadata_path)?;

    let Some(mapping) = yaml_value.as_mapping() else {
        return Ok(None);
    };

    let series_name = mapping
        .get(serde_yaml::Value::String(
            "belongs-to-collection".to_string(),
        ))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let series_position = mapping
        .get(serde_yaml::Value::String("group-position".to_string()))
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok());

    Ok(series_name.map(|name| (name, series_position)))
}

fn read_metadata_yaml_value(metadata_path: &Path) -> Result<serde_yaml::Value, IrieBookError> {
    let metadata_text = fs::read_to_string(metadata_path).map_err(|e| IrieBookError::FileRead {
        path: metadata_path.display().to_string(),
        source: e,
    })?;

    let cleaned_yaml = metadata_text
        .lines()
        .filter(|line| *line != "---")
        .collect::<Vec<_>>()
        .join("\n");

    serde_yaml::from_str(&cleaned_yaml).map_err(|e| IrieBookError::FileRead {
        path: metadata_path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })
}

fn normalize_title_page_style(style: &str) -> Option<&'static str> {
    match style.trim().to_ascii_lowercase().as_str() {
        "classic" => Some("classic"),
        "minimal" => Some("minimal"),
        "ornate" => Some("ornate"),
        _ => None,
    }
}

fn apply_title_page_style_to_epub(epub_path: &Path, style: &str) -> Result<bool, IrieBookError> {
    let mut entries = read_epub_entries(epub_path)?;

    let Some(title_page_index) = entries
        .iter()
        .position(|entry| entry.name == "EPUB/text/title_page.xhtml")
    else {
        return Ok(false);
    };

    let title_page = String::from_utf8(entries[title_page_index].data.clone()).map_err(|e| {
        IrieBookError::FileRead {
            path: format!("{}::EPUB/text/title_page.xhtml", epub_path.display()),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        }
    })?;

    let Some(updated_title_page) = inject_title_page_style_class(&title_page, style) else {
        return Ok(false);
    };

    entries[title_page_index].data = updated_title_page.into_bytes();
    write_epub_entries(epub_path, &entries)?;
    Ok(true)
}

fn apply_title_page_series_to_epub(
    epub_path: &Path,
    series_name: &str,
    series_position: Option<u32>,
) -> Result<bool, IrieBookError> {
    let mut entries = read_epub_entries(epub_path)?;

    let Some(title_page_index) = entries
        .iter()
        .position(|entry| entry.name == "EPUB/text/title_page.xhtml")
    else {
        return Ok(false);
    };

    let title_page = String::from_utf8(entries[title_page_index].data.clone()).map_err(|e| {
        IrieBookError::FileRead {
            path: format!("{}::EPUB/text/title_page.xhtml", epub_path.display()),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        }
    })?;

    let Some(updated_title_page) =
        inject_title_page_series(&title_page, series_name, series_position)
    else {
        return Ok(false);
    };

    entries[title_page_index].data = updated_title_page.into_bytes();
    write_epub_entries(epub_path, &entries)?;
    Ok(true)
}

fn inject_title_page_style_class(title_page_xhtml: &str, style: &str) -> Option<String> {
    let section_re =
        Regex::new(r#"(<section\b[^>]*epub:type="titlepage"[^>]*\bclass=")([^"]*)(")"#).ok()?;
    let captures = section_re.captures(title_page_xhtml)?;
    let full_match = captures.get(0)?;
    let prefix = captures.get(1)?.as_str();
    let classes = captures.get(2)?.as_str();
    let suffix = captures.get(3)?.as_str();

    let mut class_tokens: Vec<String> = classes
        .split_whitespace()
        .filter(|token| !token.starts_with("title-style-"))
        .map(ToString::to_string)
        .collect();
    class_tokens.push(format!("title-style-{}", style));

    let replacement = format!("{}{}{}", prefix, class_tokens.join(" "), suffix);

    let mut updated = String::new();
    updated.push_str(&title_page_xhtml[..full_match.start()]);
    updated.push_str(&replacement);
    updated.push_str(&title_page_xhtml[full_match.end()..]);

    Some(updated)
}

fn inject_title_page_series(
    title_page_xhtml: &str,
    series_name: &str,
    series_position: Option<u32>,
) -> Option<String> {
    let title_re = Regex::new(r#"(<h1\b[^>]*\bclass="[^"]*\btitle\b[^"]*"[^>]*>.*?</h1>)"#).ok()?;
    let title_match = title_re.find(title_page_xhtml)?;
    let title_block = &title_page_xhtml[title_match.start()..title_match.end()];

    let top_stars = build_top_star_group(series_position);

    let mut top_block = String::new();
    top_block.push_str(&format!(
        "\n  <p class=\"titlepage-top-stars\">{}</p>",
        top_stars
    ));

    let mut series_block = String::new();
    series_block.push_str(&format!(
        "\n  <p class=\"titlepage-series\">{}</p>",
        series_name
    ));

    let mut updated = String::new();
    updated.push_str(&title_page_xhtml[..title_match.start()]);
    updated.push_str(&top_block);
    updated.push_str(title_block);
    updated.push_str(&series_block);
    updated.push_str(&title_page_xhtml[title_match.end()..]);

    add_class_to_titlepage_section(&updated, "titlepage-has-series-stars")
}

fn build_top_star_group(series_position: Option<u32>) -> String {
    let count = series_position
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(1);

    (0..count).map(|_| "✦").collect::<Vec<_>>().join(" ")
}

fn add_class_to_titlepage_section(title_page_xhtml: &str, class_name: &str) -> Option<String> {
    let section_re =
        Regex::new(r#"(<section\b[^>]*epub:type="titlepage"[^>]*\bclass=")([^"]*)(")"#).ok()?;
    let captures = section_re.captures(title_page_xhtml)?;
    let full_match = captures.get(0)?;
    let prefix = captures.get(1)?.as_str();
    let classes = captures.get(2)?.as_str();
    let suffix = captures.get(3)?.as_str();

    let mut class_tokens: Vec<String> = classes
        .split_whitespace()
        .map(ToString::to_string)
        .collect();
    if !class_tokens.iter().any(|token| token == class_name) {
        class_tokens.push(class_name.to_string());
    }

    let replacement = format!("{}{}{}", prefix, class_tokens.join(" "), suffix);

    let mut updated = String::new();
    updated.push_str(&title_page_xhtml[..full_match.start()]);
    updated.push_str(&replacement);
    updated.push_str(&title_page_xhtml[full_match.end()..]);

    Some(updated)
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
    use zip::ZipWriter;
    use zip::write::FileOptions;

    #[test]
    fn pandoc_converter_implements_trait() {
        let converter = PandocConverter;
        // This test just verifies the trait is implemented correctly
        // Actual pandoc execution would require pandoc to be installed
        let _ = converter;
    }

    #[test]
    fn build_pandoc_command_includes_cover_arg_when_cover_path_is_present() {
        let command = build_pandoc_command(
            Path::new("fixed.md"),
            Path::new("book.epub"),
            "default.css",
            Path::new("metadata.yaml"),
            Some(Path::new("cover.jpg")),
        );

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.contains(&"--epub-cover-image".to_string()));
        assert!(args.contains(&"cover.jpg".to_string()));
    }

    #[test]
    fn build_pandoc_command_omits_cover_arg_when_cover_path_is_absent() {
        let command = build_pandoc_command(
            Path::new("fixed.md"),
            Path::new("book.epub"),
            "default.css",
            Path::new("metadata.yaml"),
            None,
        );

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(!args.contains(&"--epub-cover-image".to_string()));
        assert!(!args.contains(&"cover.jpg".to_string()));
    }

    #[test]
    fn build_pdf_pandoc_command_uses_print_settings() {
        let config = PdfConfig::default();
        let command = build_pdf_pandoc_command(
            Path::new("fixed.md"),
            Path::new("book.pdf"),
            Path::new("metadata.yaml"),
            &config,
            Path::new("pdf-style.tex"),
            Path::new("pdf-style.lua"),
        );

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.contains(&"fixed.md".to_string()));
        assert!(args.contains(&"book.pdf".to_string()));
        assert!(args.contains(&"--pdf-engine".to_string()));
        assert!(args.contains(&"xelatex".to_string()));
        assert!(args.contains(&"--metadata-file".to_string()));
        assert!(args.contains(&"metadata.yaml".to_string()));
        assert!(args.contains(&"mainfont=Liberation Serif".to_string()));
        assert!(args.contains(&"fontsize=11pt".to_string()));
        assert!(args.contains(&"linestretch=1.2".to_string()));
        assert!(args.contains(&"geometry:paperwidth=5.5in".to_string()));
        assert!(args.contains(&"geometry:paperheight=8.5in".to_string()));
        assert!(args.contains(&"geometry:inner=2.2cm".to_string()));
        assert!(args.contains(&"geometry:outer=1.8cm".to_string()));
        assert!(args.contains(&"geometry:top=1.8cm".to_string()));
        assert!(args.contains(&"geometry:bottom=1.8cm".to_string()));
        assert!(args.contains(&"--include-in-header".to_string()));
        assert!(args.contains(&"pdf-style.tex".to_string()));
        assert!(args.contains(&"--lua-filter".to_string()));
        assert!(args.contains(&"pdf-style.lua".to_string()));
        assert!(!args.contains(&"--include-before-body".to_string()));
    }

    #[test]
    fn write_pdf_latex_header_references_first_page_cover() {
        let temp_dir = TempDir::new().unwrap();
        let output_pdf = temp_dir.path().join("book.pdf");
        let cover_path = temp_dir.path().join("cover.jpg");
        let image = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(800, 1200);
        image.save(&cover_path).unwrap();

        let include_path =
            write_pdf_latex_header(&output_pdf, Some(&cover_path), &PdfConfig::default()).unwrap();
        let include = std::fs::read_to_string(include_path).unwrap();

        assert!(include.contains("\\AtBeginDocument{\\irieCoverPage}"));
        assert!(include.contains("\\includegraphics"));
        assert!(include.contains("cover.jpg"));
        assert!(include.contains("\\thispagestyle{empty}"));
        assert!(include.contains("\\newgeometry{margin=0pt}"));
        assert!(include.contains("clip"));
    }

    #[test]
    fn write_pdf_latex_header_contains_epub_style_mirror() {
        let temp_dir = TempDir::new().unwrap();
        let output_pdf = temp_dir.path().join("book.pdf");

        let include_path =
            write_pdf_latex_header(&output_pdf, None, &PdfConfig::default()).unwrap();
        let include = std::fs::read_to_string(include_path).unwrap();

        assert!(include.contains("\\newenvironment{irieSubtitle}"));
        assert!(include.contains("\\newcommand{\\irieSceneBreak}"));
        assert!(include.contains("\\newenvironment{irieDedication}"));
        assert!(include.contains("\\newenvironment{irieCopyright}"));
        assert!(include.contains("\\newcommand{\\irieBlankPage}"));
        assert!(include.contains("\\newcommand{\\irieDedicationOpening}"));
        assert!(include.contains("\\usepackage{indentfirst}"));
        assert!(include.contains("\\renewcommand{\\tableofcontents}"));
        assert!(include.contains("\\renewcommand{\\contentsname}{\\@title}"));
        assert!(include.contains("\\ifirieMainMatterStarted"));
        assert!(include.contains("\\renewcommand{\\maketitle}"));
        assert!(!include.contains("\\AtBeginDocument{\\irieCoverPage}"));
    }

    #[test]
    fn write_pdf_latex_header_wraps_dedication_in_blank_pages() {
        let temp_dir = TempDir::new().unwrap();
        let output_pdf = temp_dir.path().join("book.pdf");

        let include_path =
            write_pdf_latex_header(&output_pdf, None, &PdfConfig::default()).unwrap();
        let include = std::fs::read_to_string(include_path).unwrap();

        assert!(include.contains("\\ifodd\\value{page}"));
        assert!(include.contains("\\irieDedicationOpening"));
        assert!(include.contains("\\irieBlankPage"));
        assert!(include.contains("\\clearpage\\irieBlankPage"));
    }

    #[test]
    fn write_pdf_lua_filter_maps_epub_semantics() {
        let temp_dir = TempDir::new().unwrap();
        let output_pdf = temp_dir.path().join("book.pdf");

        let filter_path = write_pdf_lua_filter(&output_pdf).unwrap();
        let filter = std::fs::read_to_string(filter_path).unwrap();

        assert!(filter.contains("irieSubtitle"));
        assert!(filter.contains("irieSceneBreak"));
        assert!(filter.contains("irieDedication"));
        assert!(filter.contains("irieCopyright"));
    }

    #[test]
    fn resolve_cover_path_allows_missing_cover_when_embedding_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("book.md");
        std::fs::write(&input_path, "# Book").unwrap();

        let cover_path = resolve_cover_path(&input_path, false).unwrap();

        assert!(cover_path.is_none());
    }

    #[test]
    fn resolve_cover_path_requires_cover_when_embedding_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("book.md");
        std::fs::write(&input_path, "# Book").unwrap();

        let result = resolve_cover_path(&input_path, true);

        assert!(result.is_err());
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

    #[test]
    fn reads_title_page_style_from_metadata_with_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let metadata_path = temp_dir.path().join("metadata.yaml");
        std::fs::write(
            &metadata_path,
            "---\ntitle: Test\nauthor: Jane\ntitle-page-style: ornate\n---\n",
        )
        .unwrap();

        let style = read_title_page_style_from_metadata(&metadata_path).unwrap();
        assert_eq!(style.as_deref(), Some("ornate"));
    }

    #[test]
    fn reads_title_page_series_from_metadata_with_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let metadata_path = temp_dir.path().join("metadata.yaml");
        std::fs::write(
            &metadata_path,
            "---\ntitle: Test\nauthor: Jane\nbelongs-to-collection: Saga\ngroup-position: 3\n---\n",
        )
        .unwrap();

        let series = read_title_page_series_from_metadata(&metadata_path).unwrap();
        assert_eq!(series, Some(("Saga".to_string(), Some(3))));
    }

    #[test]
    fn applies_title_page_style_class_in_epub() {
        let temp_dir = TempDir::new().unwrap();
        let epub_path = temp_dir.path().join("book.epub");

        let file = std::fs::File::create(&epub_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();

        let title_page = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<body epub:type="frontmatter">
<section epub:type="titlepage" class="titlepage">
  <h1 class="title">Book</h1>
  <p class="author">Author</p>
</section>
</body>
</html>
"#;

        zip.start_file("EPUB/text/title_page.xhtml", options)
            .unwrap();
        zip.write_all(title_page.as_bytes()).unwrap();
        zip.finish().unwrap();

        let changed = apply_title_page_style_to_epub(&epub_path, "ornate").unwrap();
        assert!(changed, "Expected title page style class to be applied");

        let file = std::fs::File::open(&epub_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut updated_title_page = String::new();
        archive
            .by_name("EPUB/text/title_page.xhtml")
            .unwrap()
            .read_to_string(&mut updated_title_page)
            .unwrap();

        assert!(
            updated_title_page.contains("class=\"titlepage title-style-ornate\""),
            "Expected style class on title page section"
        );
    }

    #[test]
    fn applies_series_info_in_epub_title_page() {
        let temp_dir = TempDir::new().unwrap();
        let epub_path = temp_dir.path().join("book.epub");

        let file = std::fs::File::create(&epub_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();

        let title_page = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<body epub:type="frontmatter">
<section epub:type="titlepage" class="titlepage">
  <h1 class="title">Book</h1>
  <p class="author">Author</p>
</section>
</body>
</html>
"#;

        zip.start_file("EPUB/text/title_page.xhtml", options)
            .unwrap();
        zip.write_all(title_page.as_bytes()).unwrap();
        zip.finish().unwrap();

        let changed = apply_title_page_series_to_epub(&epub_path, "Saga", Some(3)).unwrap();
        assert!(changed, "Expected title page series info to be injected");

        let file = std::fs::File::open(&epub_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut updated_title_page = String::new();
        archive
            .by_name("EPUB/text/title_page.xhtml")
            .unwrap()
            .read_to_string(&mut updated_title_page)
            .unwrap();

        assert!(
            updated_title_page.contains("<p class=\"titlepage-top-stars\">✦ ✦ ✦</p>"),
            "Expected top stars to match series index"
        );
        assert!(
            updated_title_page.contains("class=\"titlepage titlepage-has-series-stars\""),
            "Expected marker class when series stars are injected"
        );
        assert!(
            updated_title_page.contains("<p class=\"titlepage-series\">Saga</p>"),
            "Expected series name on title page"
        );
        assert!(
            !updated_title_page.contains("titlepage-series-index"),
            "Series index should not be rendered when stars are present"
        );
    }
}
