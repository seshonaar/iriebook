//! CLI Client - Presentation layer for the Fixit application
//!
//! This module handles all user interaction:
//! - Parsing command-line arguments
//! - Displaying results to the user
//! - Formatting output (emojis, colors, verbose modes)
//! - Handling exit codes for validation failures
//!
//! Following the Righting Software Method, this Client layer contains
//! ZERO business logic - it only presents data from the Manager.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::managers::ebook_publication::PublicationResult;

/// CLI arguments for the Fixit application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Input markdown file to process
    pub input: PathBuf,

    /// Custom output path (default: INPUT-fixed.md)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show detailed processing info
    #[arg(short, long)]
    pub verbose: bool,

    /// Enable word statistics analysis
    #[arg(long)]
    pub word_stats: bool,

    /// Enable ebook publication (write output files)
    #[arg(long)]
    pub publish: bool,
}

/// Format and display results to the user (Client presentation layer)
pub fn display_results(result: &PublicationResult, verbose: bool) {
    // Verbose: show bytes read
    if verbose {
        println!("✅ Read {} bytes", result.bytes_read);
    }

    // Verbose: show validation status
    if verbose && result.validation_passed {
        println!("✅ Validation passed - no quotation marks, quotes are balanced");
    }

    // If validation failed, we'll handle it after this function returns
    if !result.validation_passed {
        return;
    }

    // Show conversion stats
    println!(
        "✅ Converted {} quotes and {} apostrophes",
        result.quotes_converted, result.apostrophes_converted
    );

    // Show whitespace trimming stats
    println!("✅ Trimmed whitespace:");
    println!("   {} spaces collapsed", result.spaces_collapsed);
    println!("   {} tabs converted", result.tabs_converted);
    println!("   {} blank lines removed", result.blank_lines_removed);
    println!("   {} lines trimmed", result.lines_trimmed);

    // Show word analysis if enabled
    match &result.word_analysis {
        Some(analysis) => {
            println!("📊 Word Analysis:");
            println!("   {} total words", analysis.total_words);
            println!("   {} unique words", analysis.unique_words);
            println!("   {} stopwords excluded", analysis.excluded_count);

            if !analysis.top_words.is_empty() {
                println!("   Top words:");
                for (i, (word, count)) in analysis.top_words.iter().take(10).enumerate() {
                    println!("      {}. {} ({})", i + 1, word, count);
                }
            } else {
                println!("   (No content words found)");
            }
        }
        None => {
            // Word analysis disabled - skip this section
        }
    }

    // Show completion message
    if result.output_path.is_none() {
        println!("\n📊 Word Analysis Complete (no files written)");
        println!("   (use --publish to generate output files)");
    } else if let Some(output_path) = &result.output_path {
        println!("\n✅ Success!");
        println!("   Output: {}", output_path.display());
        if let Some(pdf_output_path) = &result.pdf_output_path {
            println!("   PDF: {}", pdf_output_path.display());
        }
    }
}

/// Format summary for writing to file
pub fn format_summary(result: &PublicationResult) -> String {
    let mut summary = String::new();

    if !result.validation_passed
        && let Some(error) = &result.validation_error
    {
        summary.push_str("❌ Validation failed!\n\n");
        summary.push_str(error);
        return summary;
    }

    summary.push_str(&format!(
        "✅ Converted {} quotes and {} apostrophes\n",
        result.quotes_converted, result.apostrophes_converted
    ));
    summary.push_str("✅ Trimmed whitespace:\n");
    summary.push_str(&format!(
        "   {} spaces collapsed\n",
        result.spaces_collapsed
    ));
    summary.push_str(&format!("   {} tabs converted\n", result.tabs_converted));
    summary.push_str(&format!(
        "   {} blank lines removed\n",
        result.blank_lines_removed
    ));
    summary.push_str(&format!("   {} lines trimmed\n", result.lines_trimmed));

    // Include word analysis if enabled
    match &result.word_analysis {
        Some(analysis) => {
            summary.push_str("📊 Word Analysis:\n");
            summary.push_str(&format!("   {} total words\n", analysis.total_words));
            summary.push_str(&format!("   {} unique words\n", analysis.unique_words));
            summary.push_str(&format!(
                "   {} stopwords excluded\n",
                analysis.excluded_count
            ));

            if !analysis.top_words.is_empty() {
                summary.push_str("   Top words:\n");
                for (i, (word, count)) in analysis.top_words.iter().take(10).enumerate() {
                    summary.push_str(&format!("      {}. {} ({})\n", i + 1, word, count));
                }
            } else {
                summary.push_str("   (No content words found)\n");
            }
        }
        None => {
            // Word analysis disabled - skip this section
        }
    }

    if result.output_path.is_none() {
        summary.push_str("\n📊 Word Analysis Complete (no files written)\n");
    } else if let Some(output_path) = &result.output_path {
        summary.push_str("\n✅ Success!\n");
        summary.push_str(&format!("   Output: {}\n", output_path.display()));
        if let Some(pdf_output_path) = &result.pdf_output_path {
            summary.push_str(&format!("   PDF: {}\n", pdf_output_path.display()));
        }
    }

    summary
}

/// Handle validation failure by displaying error and exiting
pub fn handle_validation_failure(result: &PublicationResult) -> Result<()> {
    if !result.validation_passed {
        if let Some(error) = &result.validation_error {
            eprintln!("❌ Validation failed!\n");
            eprintln!("{}", error);
            eprintln!("\nPlease fix these issues in the original file and try again.");
        }
        std::process::exit(1);
    }
    Ok(())
}
