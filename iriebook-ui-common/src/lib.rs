//! Framework-agnostic UI common utilities for IrieBook
//!
//! This library contains shared functionality that can be used
//! across different UI frameworks (Slint, web, CLI, etc.)

pub mod actions;
pub mod analysis_cache;
pub mod app_state;
pub mod batch_processing;
pub mod book_scanner;
pub mod book_viewing;
pub mod collection_management;
pub mod default_paths;
pub mod diff_commands;
pub mod git_operations;
pub mod git_state;
pub mod google_auth_operations;
pub mod google_docs_operations;
pub mod image_loading;
pub mod metadata_utils;
pub mod processing;
pub mod session;
pub mod ui_state;

// Re-export commonly used types
pub use analysis_cache::{get_or_compute_analysis, AnalysisResponse};
pub use app_state::AppState;
pub use batch_processing::BatchProcessor;
pub use diff_commands::{get_book_processing_diff, get_local_diffs, get_revision_diffs, RevisionDiff};
pub use actions::{AppAction, LogEntryPayload, LogEntryType, ProcessingProgress};
pub use book_scanner::scan_for_books;
pub use book_viewing::view_book;
pub use collection_management::{
    add_book_with_rescan, change_book_with_rescan, check_for_duplicate, delete_book_with_rescan,
    AddBookResult, ChangeBookResult,
};
pub use default_paths::get_default_library_path;
pub use git_operations::{
    check_initialized, clone_repository, get_commit_history, get_remote_url, get_sync_status,
    save_repository, sync_repository,
};
pub use git_state::{DeviceFlowInfo, GitAuthStatus, GitCommit, GitSyncStatus};
pub use google_auth_operations::{
    check_authenticated, list_documents, logout, start_auth_flow,
};
pub use google_docs_operations::{link_document, sync_document, unlink_document};
pub use image_loading::{
    load_cover_as_data_url, load_cover_data, CoverImageData, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH,
};
pub use iriebook::resource_access::file::{load_metadata, save_metadata};
pub use iriebook::utilities::types::BookMetadata;
pub use metadata_utils::{collect_distinct_authors, collect_distinct_series, MetadataEditState};
pub use processing::{
    process_single_book, BookListChangedEvent, BookProcessingQueue, GitOperationProgressEvent,
    GoogleDocsProgressEvent, ProcessingEvent, ProcessingMessage, ProcessingUpdateEvent,
};
pub use session::{load_session, save_session, SessionData};
pub use ui_state::{
    BookInfo, BookPath, FolderPath, PublishEnabled, UiState, WordStatsEnabled,
};
