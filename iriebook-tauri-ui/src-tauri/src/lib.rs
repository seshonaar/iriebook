//! Tauri application entry point
//!
//! This module is the thin Tauri layer that wires up commands and events.
//! All business logic is delegated to iriebook-ui-common or iriebook crates.

mod books;
mod cloud;
mod diff;
mod git;
mod infrastructure;
mod state;
mod updater;

use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the tracing subscriber for logging
///
/// Sets up dual output:
/// - Console (stderr) for development
/// - JSON file logging with daily rotation for production debugging
fn init_tracing() {
    // Determine log directory - use platform-specific data directory
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("iriebook")
        .join("logs");

    // Create log directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Warning: Failed to create log directory: {}", e);
    }

    // Set up file appender with daily rotation
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "iriebook.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Store guard in a Box::leak to keep it alive for the program's lifetime
    // This is intentional - the guard must live as long as the application
    Box::leak(Box::new(_guard));

    // Set up filter - default to info for iriebook crates
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("iriebook=info,iriebook_ui_common=info"));

    // Build the subscriber with both console and file layers
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_writer(non_blocking).json())
        .init();

    tracing::info!("Logging initialized - log directory: {}", log_dir.display());
}

use iriebook_ui_common::{
    BookListChangedEvent, CoverReloadEvent, GitOperationProgressEvent,
    GoogleDocsBatchSyncUpdateEvent, GoogleDocsProgressEvent, ProcessingUpdateEvent,
};
use state::{AppStateHolder, GoogleAuthState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging first
    init_tracing();

    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            // Infrastructure: session, dialogs, utilities
            infrastructure::load_session,
            infrastructure::save_session,
            infrastructure::init_mock_state,
            infrastructure::select_folder,
            infrastructure::select_file,
            infrastructure::open_folder,
            infrastructure::open_browser,
            // Books: scanning, covers, metadata, processing, CRUD
            books::scan_books,
            books::load_cover_image,
            books::load_book_metadata,
            books::save_book_metadata,
            books::get_autocomplete_authors,
            books::get_autocomplete_series,
            books::replace_cover_image,
            books::view_book,
            books::start_processing,
            books::add_book,
            books::check_duplicate,
            books::delete_book,
            books::change_book_file,
            books::get_book_analysis,
            // Cloud: GitHub auth
            cloud::github_device_flow_start,
            cloud::github_device_flow_poll,
            cloud::github_store_token,
            cloud::github_check_auth,
            cloud::github_logout,
            // Cloud: Google auth
            cloud::google_auth_start,
            cloud::google_auth_cancel,
            cloud::google_check_auth,
            cloud::google_logout,
            // Cloud: Google Docs
            cloud::google_list_docs,
            cloud::google_link_doc,
            cloud::google_sync_doc,
            cloud::google_sync_selected,
            cloud::google_unlink_doc,
            // Git: repository operations
            git::git_check_initialized,
            git::git_clone_repository,
            git::git_sync,
            git::git_save,
            git::git_reset_local_changes,
            git::git_get_log,
            git::git_get_status,
            git::git_get_remote_url,
            // Diff: viewing changes
            diff::git_get_revision_diffs,
            diff::git_get_local_diffs,
            diff::get_book_processing_diff,
            // Updater
            updater::check_for_updates,
        ])
        .events(tauri_specta::collect_events![
            ProcessingUpdateEvent,
            GitOperationProgressEvent,
            GoogleDocsProgressEvent,
            GoogleDocsBatchSyncUpdateEvent,
            BookListChangedEvent,
            CoverReloadEvent,
            updater::UpdateProgressEvent
        ]);

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(GoogleAuthState::new())
        .manage(AppStateHolder::new())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
