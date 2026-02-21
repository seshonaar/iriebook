//! IrieBook - Ebook publication pipeline
//!
//! Main entry point for the CLI application

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use iriebook::client::cli::{display_results, format_summary, handle_validation_failure, Args};
use iriebook::engines::analysis::word_analyzer::WordAnalyzer;
use iriebook::engines::text_processing::markdown_transform::MarkdownTransformer;
use iriebook::engines::text_processing::quote_fixer::QuoteFixer;
use iriebook::engines::text_processing::whitespace_trimmer::WhitespaceTrimmer;
use iriebook::engines::text_processing::word_replacement::WordReplacer;
use iriebook::engines::validation::validator::Validator;
use iriebook::managers::ebook_publication::{EbookPublicationManager, PublishArgs};
use iriebook::resource_access::archive::ZipArchiver;
use iriebook::resource_access::calibre::CalibreConverter;
use iriebook::resource_access::file;
use iriebook::resource_access::pandoc::PandocConverter;

fn main() -> Result<()> {
    // Initialize tracing subscriber for CLI
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("iriebook=info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    // Validate that at least one action is requested
    if !args.publish && !args.word_stats {
        eprintln!("Error: No action specified");
        eprintln!();
        eprintln!("Use --publish to generate ebook files (EPUB, Kindle)");
        eprintln!("Use --word-stats to analyze word frequency");
        eprintln!("Use both together for complete processing");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  iriebook --publish input.md");
        eprintln!("  iriebook --word-stats input.md");
        eprintln!("  iriebook --publish --word-stats input.md");
        std::process::exit(1);
    }

    if args.verbose {
        println!("🔧 IrieBook - Wife's Ebook Publication Pipeline");
        println!("📖 Input: {}", args.input.display());
    }

    // Create manager with Engine and Resource Access dependencies
    let manager = EbookPublicationManager::new(
        Arc::new(Validator),
        Arc::new(QuoteFixer),
        Arc::new(WhitespaceTrimmer),
        Arc::new(WordAnalyzer),
        Arc::new(MarkdownTransformer),
        Arc::new(WordReplacer::new()),
        Arc::new(PandocConverter),
        Arc::new(CalibreConverter),
        Arc::new(ZipArchiver),
    );

    // Execute publication pipeline
    let result = manager.publish(PublishArgs {
        input_path: &args.input,
        output_path: args.output.as_deref(),
        enable_word_stats: args.word_stats,
        enable_publishing: args.publish,
        replace_pairs: None,
    })?;

    // Display results (Client's responsibility)
    display_results(&result, args.verbose);

    // Handle validation failure (exit with error code if needed)
    handle_validation_failure(&result)?;

    // Write summary file if we have an output path
    if let Some(summary_path) = &result.summary_path {
        let summary = format_summary(&result);
        file::write_file(summary_path, &summary)?;
    }

    Ok(())
}
