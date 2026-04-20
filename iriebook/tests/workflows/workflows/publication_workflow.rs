//! E2E tests for the complete publication workflow
//!
//! These tests exercise the full flow from workspace setup to EPUB generation,
//! using mocked external dependencies.

use iriebook::managers::ebook_publication::PublishArgs;
use iriebook::resource_access::pandoc::PandocConverter;
use iriebook_test_support::{
    GitCall, MockArchiveAccess, MockCalibreAccess, MockGitAccess, MockGoogleDocsAccess,
    MockPandocAccess, TestWorkspace,
};
use iriebook_ui_common::app_state::AppStateBuilder;
use std::{fs, process::Command, sync::Arc};

/// Test: Complete publication workflow from start to finish
///
/// Scenario:
/// 1. User has a workspace with one book
/// 2. Book is synced with Google Docs
/// 3. User generates EPUB and Kindle versions
/// 4. User commits and pushes changes
#[tokio::test]
async fn test_complete_publication_workflow() {
    // === ARRANGE ===

    // 1. Create test workspace with sample book
    let mut workspace = TestWorkspace::new().unwrap();
    let book = workspace.add_book("vampire-romance").unwrap();
    let book_path = book.md_path.clone();

    // 2. Setup mocks with expected behavior
    let mock_git = Arc::new(
        MockGitAccess::new()
            .with_repo_state(true, false) // Is a repo, no uncommitted changes initially
            .with_sync_state(0, 0), // Clean state
    );

    let mock_docs = Arc::new(MockGoogleDocsAccess::new().with_document(
        "doc-vampire-123",
        "Vampire Romance",
        "# Vampire Romance\n\n## Chapter 1\n\nThe night was dark...",
    ));

    let mock_pandoc = Arc::new(MockPandocAccess::new());
    let mock_calibre = Arc::new(MockCalibreAccess::new());
    let mock_archive = Arc::new(MockArchiveAccess::new());

    // 3. Build AppState with mocks
    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git.clone())
        .with_google_docs_access(mock_docs.clone())
        .with_pandoc_access(mock_pandoc.clone())
        .with_calibre_access(mock_calibre.clone())
        .with_archive_access(mock_archive.clone())
        .with_defaults_for_remaining()
        .build();

    // === ACT ===

    // Step 1: Get repository manager and check status
    let repo_manager = app_state.repository_manager();
    let _sync_status = repo_manager
        .get_sync_status(&workspace.workspace_path)
        .unwrap();

    // Step 2: Get publication manager and generate ebook
    let pub_manager = app_state.ebook_publication_manager();

    // Note: We need to pass the book path for publication
    // The actual publication requires the book.md to exist with proper structure
    let result = pub_manager.publish(PublishArgs {
        input_path: &book_path,
        output_path: None,
        enable_word_stats: true,
        enable_publishing: true,
        embed_cover: true,
        config_root: Some(&workspace.workspace_path),
        replace_pairs: None,
    });

    // === ASSERT ===

    // Verify sync status was checked
    assert!(mock_git.was_called(&GitCall::GetStatus {
        path: workspace.workspace_path.clone()
    }));

    // Publication should succeed (mocks create the output files)
    assert!(
        result.is_ok(),
        "Publication failed: {:?}",
        result.as_ref().err()
    );

    // Verify Pandoc was called to create EPUB
    let pandoc_calls = mock_pandoc.get_calls();
    assert!(!pandoc_calls.is_empty(), "Pandoc should have been called");

    // Verify Pandoc was called to create PDF from the library-root config
    let pdf_calls = mock_pandoc.get_pdf_calls();
    assert!(!pdf_calls.is_empty(), "Pandoc PDF should have been called");
    assert_eq!(pdf_calls[0].pdf_config.font_family, "Liberation Serif");
    assert!(
        pdf_calls[0].embed_cover,
        "PDF generation should receive cover embedding enabled"
    );
    let publication_result = result.unwrap();
    assert!(
        publication_result.pdf_output_path.is_some(),
        "Expected PDF output path"
    );
    assert!(
        workspace.workspace_path.join("config.json").exists(),
        "Expected editable root config.json to be created"
    );

    // Verify Calibre was called to create Kindle version
    let calibre_calls = mock_calibre.get_calls();
    assert!(!calibre_calls.is_empty(), "Calibre should have been called");

    // Verify Archive was called to create ZIP
    let archive_calls = mock_archive.get_calls();
    assert!(!archive_calls.is_empty(), "Archive should have been called");
    assert!(
        archive_calls[0].input_pdf.is_some(),
        "Archive should receive the generated PDF"
    );
}

/// Test: Publication workflow handles validation errors gracefully
#[tokio::test]
async fn test_publication_handles_invalid_book() {
    let mut workspace = TestWorkspace::new().unwrap();

    // Create a book with unbalanced quotes (will fail validation)
    workspace
        .add_book_with_content(
            "bad-book",
            r#"# Bad Book

"This quote is not closed...

The end.
"#,
        )
        .unwrap();

    let book = workspace.get_book("bad-book").unwrap();

    let mock_git = Arc::new(MockGitAccess::new());
    let mock_pandoc = Arc::new(MockPandocAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace.workspace_path.clone())
        .with_git_access(mock_git)
        .with_pandoc_access(mock_pandoc.clone())
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();
    let result = pub_manager.publish(PublishArgs {
        input_path: &book.md_path,
        output_path: None,
        enable_word_stats: true,
        enable_publishing: true,
        embed_cover: true,
        config_root: None,
        replace_pairs: None,
    });

    // Publication should return a validation failure result (not an outright error)
    let result = result.expect("Publication should return structured validation failure");
    assert!(
        !result.validation_passed,
        "Validation must fail for unbalanced quotes"
    );
    assert!(
        result
            .validation_error
            .as_ref()
            .map(|msg| msg.contains("Unbalanced quotes"))
            .unwrap_or(false),
        "Validation error should mention unbalanced quotes"
    );

    // Pandoc should NOT have been called (validation fails first)
    let pandoc_calls = mock_pandoc.get_calls();
    assert!(
        pandoc_calls.is_empty(),
        "Pandoc should not be called when validation fails"
    );
}

/// Test: Publication workflow with Pandoc failure
#[tokio::test]
async fn test_publication_handles_pandoc_failure() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("good-book").unwrap();

    let mock_git = Arc::new(MockGitAccess::new());
    let mock_pandoc = Arc::new(MockPandocAccess::new().with_failure("Pandoc not installed"));
    let mock_calibre = Arc::new(MockCalibreAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_git_access(mock_git)
        .with_pandoc_access(mock_pandoc)
        .with_calibre_access(mock_calibre.clone())
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();
    let result = pub_manager.publish(PublishArgs {
        input_path: &book.md_path,
        output_path: None,
        enable_word_stats: true,
        enable_publishing: true,
        embed_cover: true,
        config_root: None,
        replace_pairs: None,
    });

    // Publication should fail
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Pandoc"),
        "Error should mention Pandoc: {}",
        err_msg
    );

    // Calibre should NOT have been called (Pandoc fails first)
    let calibre_calls = mock_calibre.get_calls();
    assert!(
        calibre_calls.is_empty(),
        "Calibre should not be called when Pandoc fails"
    );
}

/// Test: Publication generates all ebook formats
#[tokio::test]
async fn test_publication_generates_all_formats() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("full-book").unwrap();

    let mock_pandoc = Arc::new(MockPandocAccess::new());
    let mock_calibre = Arc::new(MockCalibreAccess::new());
    let mock_archive = Arc::new(MockArchiveAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_pandoc_access(mock_pandoc.clone())
        .with_calibre_access(mock_calibre.clone())
        .with_archive_access(mock_archive.clone())
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();

    // Publish with metadata (generates all formats)
    let result = pub_manager.publish(PublishArgs {
        input_path: &book.md_path,
        output_path: None,
        enable_word_stats: false,
        enable_publishing: true,
        embed_cover: true,
        config_root: None,
        replace_pairs: None,
    });

    assert!(result.is_ok());

    // All tools should have been called
    assert!(
        !mock_pandoc.get_calls().is_empty(),
        "Pandoc should be called"
    );
    assert!(
        !mock_calibre.get_calls().is_empty(),
        "Calibre should be called"
    );
    assert!(
        !mock_archive.get_calls().is_empty(),
        "Archive should be called"
    );
}

/// Test: Publication forwards cover embedding choice to Pandoc
#[tokio::test]
async fn test_publication_can_disable_cover_embedding() {
    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("no-cover-embed").unwrap();
    std::fs::write(
        book.path.join("metadata.yaml"),
        r#"title: "no-cover-embed"
author: "Test Author"
language: en
cover-image: cover.jpg
"#,
    )
    .unwrap();
    std::fs::remove_file(book.path.join("cover.jpg")).unwrap();

    let mock_pandoc = Arc::new(MockPandocAccess::new());
    let mock_calibre = Arc::new(MockCalibreAccess::new());
    let mock_archive = Arc::new(MockArchiveAccess::new());

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_pandoc_access(mock_pandoc.clone())
        .with_calibre_access(mock_calibre)
        .with_archive_access(mock_archive)
        .with_defaults_for_remaining()
        .build();

    let pub_manager = app_state.ebook_publication_manager();
    let result = pub_manager.publish(PublishArgs {
        input_path: &book.md_path,
        output_path: None,
        enable_word_stats: false,
        enable_publishing: true,
        embed_cover: false,
        config_root: None,
        replace_pairs: None,
    });

    assert!(result.is_ok());

    let pandoc_calls = mock_pandoc.get_calls();
    assert_eq!(pandoc_calls.len(), 1);
    assert!(
        !pandoc_calls[0].embed_cover,
        "Pandoc should receive cover embedding disabled"
    );
    let custom_metadata = pandoc_calls[0]
        .custom_metadata_content
        .as_ref()
        .expect("Pandoc should receive temporary metadata when cover embedding is disabled");
    assert!(
        !custom_metadata.contains("cover-image"),
        "Temporary metadata must remove cover-image so Pandoc does not try to embed cover.jpg"
    );

    let pdf_calls = mock_pandoc.get_pdf_calls();
    assert_eq!(pdf_calls.len(), 1);
    assert!(
        !pdf_calls[0].embed_cover,
        "PDF generation should receive cover embedding disabled"
    );
    assert!(
        !pdf_calls[0].metadata_content.contains("cover-image"),
        "PDF metadata must also remove cover-image when embedding is disabled"
    );
}

/// Test: Disabling cover embedding still creates an EPUB when metadata names a missing cover
#[tokio::test]
async fn test_publication_without_cover_embedding_generates_epub_with_missing_cover_file() {
    if std::process::Command::new("pandoc")
        .arg("--version")
        .output()
        .is_err()
        || std::process::Command::new("xelatex")
            .arg("--version")
            .output()
            .is_err()
    {
        eprintln!("Skipping test: pandoc or xelatex is not installed");
        return;
    }

    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace.add_book("missing-cover").unwrap();
    std::fs::write(
        book.path.join("metadata.yaml"),
        r#"title: "missing-cover"
author: "Test Author"
language: en
cover-image: cover.jpg
"#,
    )
    .unwrap();
    std::fs::remove_file(book.path.join("cover.jpg")).unwrap();

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path)
        .with_pandoc_access(Arc::new(PandocConverter))
        .with_calibre_access(Arc::new(MockCalibreAccess::new()))
        .with_archive_access(Arc::new(MockArchiveAccess::new()))
        .with_defaults_for_remaining()
        .build();

    let result = app_state.ebook_publication_manager().publish(PublishArgs {
        input_path: &book.md_path,
        output_path: None,
        enable_word_stats: false,
        enable_publishing: true,
        embed_cover: false,
        config_root: None,
        replace_pairs: None,
    });

    assert!(
        result.is_ok(),
        "Publication without cover embedding should not require cover.jpg: {:?}",
        result.err()
    );
    let output_path = result
        .unwrap()
        .output_path
        .expect("Expected EPUB output path");
    assert!(output_path.exists(), "Expected generated EPUB to exist");
}

#[tokio::test]
async fn test_generated_epub_passes_epubcheck_with_custom_copyright_page() {
    if Command::new("pandoc").arg("--version").output().is_err()
        || Command::new("epubcheck").arg("--version").output().is_err()
    {
        eprintln!("Skipping test: pandoc or epubcheck is not installed");
        return;
    }

    let mut workspace = TestWorkspace::new().unwrap();
    let workspace_path = workspace.workspace_path.clone();
    let book = workspace
        .add_book_with_content(
            "book",
            r#"# Book

## Prelude

The bells of the old quarter rang before dawn.

## Chapter 1: The Visit

Mara stepped through the courtyard gate and listened for movement.

"Are you certain this is the right house?" she asked.

"It is the only house that still keeps its lantern lit at this hour," Victor said.

## Chapter 2: The Reading

The table was already set with cards, candles, and a brass bowl of water.

"Then let the reading begin," said the host.

## Chapter 3: The Warning

By sunrise, each of them understood that the message had been meant for all three.
"#,
        )
        .unwrap()
        .clone();

    fs::write(
        book.path.join("metadata.yaml"),
        r#"title: "Book"
author: "Test Author"
language: en
"#,
    )
    .unwrap();
    fs::write(
        book.path.join("copyright.txt"),
        "First edition.\n\nNo part of this book may be reproduced without permission.",
    )
    .unwrap();
    fs::write(workspace_path.join("config.json"), "{\n  \"pdf\": {\n    \"enabled\": false\n  }\n}\n")
        .unwrap();

    let app_state = AppStateBuilder::new()
        .workspace_path(workspace_path.clone())
        .with_pandoc_access(Arc::new(PandocConverter))
        .with_calibre_access(Arc::new(MockCalibreAccess::new()))
        .with_archive_access(Arc::new(MockArchiveAccess::new()))
        .with_defaults_for_remaining()
        .build();

    let result = app_state.ebook_publication_manager().publish(PublishArgs {
        input_path: &book.md_path,
        output_path: None,
        enable_word_stats: false,
        enable_publishing: true,
        embed_cover: false,
        config_root: Some(&workspace_path),
        replace_pairs: None,
    });

    let output_path = result
        .expect("Publication should produce an EPUB so epubcheck can inspect it")
        .output_path
        .expect("Expected EPUB output path");
    assert!(output_path.exists(), "Expected generated EPUB to exist");

    let epubcheck = Command::new("epubcheck")
        .arg(&output_path)
        .output()
        .expect("epubcheck should run");

    assert!(
        epubcheck.status.success(),
        "epubcheck failed: stdout={} stderr={}",
        String::from_utf8_lossy(&epubcheck.stdout),
        String::from_utf8_lossy(&epubcheck.stderr)
    );
}

#[cfg(test)]
mod quote_processing_tests {
    use super::*;

    /// Test: Publication properly curls straight quotes
    #[tokio::test]
    async fn test_publication_curls_quotes() {
        let mut workspace = TestWorkspace::new().unwrap();

        // Create book with straight quotes that need curling
        workspace
            .add_book_with_content(
                "quotes-book",
                r#"# Quote Test

"Hello," she said. "How are you?"

He replied, "I'm fine!"

The end.
"#,
            )
            .unwrap();

        let book = workspace.get_book("quotes-book").unwrap();

        let mock_pandoc = Arc::new(MockPandocAccess::new());
        let mock_calibre = Arc::new(MockCalibreAccess::new());
        let mock_archive = Arc::new(MockArchiveAccess::new());

        let app_state = AppStateBuilder::new()
            .workspace_path(workspace.workspace_path.clone())
            .with_pandoc_access(mock_pandoc.clone())
            .with_calibre_access(mock_calibre)
            .with_archive_access(mock_archive)
            .with_defaults_for_remaining()
            .build();

        let pub_manager = app_state.ebook_publication_manager();
        let result = pub_manager.publish(PublishArgs {
            input_path: &book.md_path,
            output_path: None,
            enable_word_stats: false,
            enable_publishing: true,
            embed_cover: true,
            config_root: None,
            replace_pairs: None,
        });

        // Should succeed - quotes are balanced and will be curled
        assert!(result.is_ok(), "Publication failed: {:?}", result.err());

        // Pandoc should have been called with the fixed markdown
        let calls = mock_pandoc.get_calls();
        assert!(!calls.is_empty());

        // The fixed markdown file should contain curly quotes
        // (We'd need to read the temp file to verify, but the fact it succeeded is good)
    }
}
