//! Framework-agnostic UI common utilities for IrieBook
//!
//! This library contains shared functionality that can be used
//! across different UI frameworks (Slint, web, CLI, etc.)

pub mod actions;
pub mod analysis_cache;
pub mod app_state;
pub mod batch_google_docs_sync;
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
pub mod managers;
pub mod metadata_utils;
pub mod processing;
pub mod session;
pub mod ui_state;

// Re-export commonly used types
pub use actions::{AppAction, LogEntryPayload, LogEntryType, ProcessingProgress};
pub use analysis_cache::{AnalysisResponse, get_or_compute_analysis};
pub use app_state::AppState;
pub use batch_google_docs_sync::BatchGoogleDocsSyncProcessor;
pub use batch_processing::BatchProcessor;
pub use book_scanner::scan_for_books;
pub use book_viewing::{BookOutputFormat, BookOutputLink, get_available_book_outputs};
pub use collection_management::{
    AddBookResult, ChangeBookResult, add_book_with_rescan, change_book_with_rescan,
    check_for_duplicate, delete_book_with_rescan,
};
pub use default_paths::get_default_library_path;
pub use diff_commands::{
    RevisionDiff, get_book_processing_diff, get_local_diffs, get_revision_diffs,
};
pub use git_operations::{
    check_initialized, clone_repository, get_commit_history, get_remote_url, get_sync_status,
    reset_repository, save_repository, sync_repository,
};
pub use git_state::{DeviceFlowInfo, GitAuthStatus, GitCommit, GitSyncStatus};
pub use google_auth_operations::{check_authenticated, list_documents, logout, start_auth_flow};
pub use google_docs_operations::{link_document, sync_document, unlink_document};
pub use image_loading::{
    CoverImageData, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH, load_cover_as_data_url, load_cover_data,
};
pub use iriebook::resource_access::file::{load_metadata, replace_cover_image, save_metadata};
pub use iriebook::utilities::types::{BookMetadata, ReplacePair};
pub use managers::{
    BookUIManager, CoverLoadingEngine, CoverStatus, DefaultCoverLoadingEngine,
    MockCoverLoadingEngine, OnCoverLoaded,
};
pub use metadata_utils::{MetadataEditState, collect_distinct_authors, collect_distinct_series};
pub use processing::{
    BookListChangedEvent, BookProcessingQueue, CoverReloadEvent, GitOperationProgressEvent,
    GoogleDocsBatchSyncEvent, GoogleDocsBatchSyncUpdateEvent, GoogleDocsProgressEvent,
    ProcessingEvent, ProcessingMessage, ProcessingUpdateEvent, process_single_book,
};
pub use session::{SessionData, load_session, save_session};
pub use ui_state::{BookInfo, BookPath, FolderPath, PublishEnabled, UiState, WordStatsEnabled};
