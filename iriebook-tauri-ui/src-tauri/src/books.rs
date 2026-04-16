//! Book-related commands: scanning, covers, metadata, processing, and CRUD operations

use crate::state::AppStateHolder;
use iriebook_ui_common::ui_state::{BookInfo, PublishEnabled, WordStatsEnabled};
use iriebook_ui_common::{
    AddBookResult, AnalysisResponse, BatchProcessor, BookListChangedEvent, BookMetadata,
    ChangeBookResult, CoverReloadEvent, CoverStatus, ProcessingUpdateEvent, add_book_with_rescan,
    book_scanner, change_book_with_rescan, check_for_duplicate, collect_distinct_authors,
    collect_distinct_series, delete_book_with_rescan, get_or_compute_analysis, load_metadata,
    save_metadata,
};
use std::path::PathBuf;
use tauri::State;
use tauri_specta::Event;

// ============= SCANNING =============

#[tauri::command]
#[specta::specta]
pub fn scan_books(
    app: tauri::AppHandle,
    path: String,
    app_state_holder: State<AppStateHolder>,
) -> Result<Vec<BookInfo>, String> {
    let path_buf = PathBuf::from(path);

    // Initialize AppState with this workspace path
    app_state_holder.initialize(path_buf.clone());

    // Set up cover loaded callback to emit events
    if let Some(app_state) = app_state_holder.get() {
        let app_handle = app.clone();
        let book_ui_manager = app_state.book_ui_manager();
        let mut manager = book_ui_manager.lock().unwrap();
        manager.set_on_cover_loaded(move |book_path| {
            let _ = CoverReloadEvent { book_path }.emit(&app_handle);
        });
    }

    book_scanner::scan_for_books(&path_buf).map_err(|e| e.to_string())
}

// ============= COVER IMAGES =============

#[tauri::command]
#[specta::specta]
pub fn load_cover_image(
    cover_path: Option<String>,
    app_state_holder: State<AppStateHolder>,
) -> Result<CoverStatus, String> {
    let cover_path = cover_path.ok_or_else(|| "No cover path provided".to_string())?;
    let path = PathBuf::from(&cover_path);

    if !path.exists() {
        return Ok(CoverStatus::Error {
            message: "Cover image not found".to_string(),
        });
    }

    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let manager = app_state.book_ui_manager();
    let mut book_ui_manager = manager.lock().unwrap();
    let status = book_ui_manager.get_thumbnail(&path, &path);

    Ok(status)
}

// ============= METADATA =============

#[tauri::command]
#[specta::specta]
pub fn load_book_metadata(book_path: String) -> Result<BookMetadata, String> {
    let path = PathBuf::from(book_path);
    load_metadata(&path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Metadata not found".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn save_book_metadata(book_path: String, metadata: BookMetadata) -> Result<(), String> {
    let path = PathBuf::from(book_path);
    save_metadata(&path, &metadata).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn replace_cover_image(
    app: tauri::AppHandle,
    book_path: String,
    new_cover_path: String,
    app_state_holder: State<AppStateHolder>,
) -> Result<(), String> {
    let book = PathBuf::from(book_path);
    let source = PathBuf::from(new_cover_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;
    let manager = app_state.book_ui_manager();
    let mut book_ui_manager = manager.lock().unwrap();

    book_ui_manager.replace_cover_image(&book, &source)?;
    let _ = BookListChangedEvent {}.emit(&app);
    Ok(())
}

// ============= BOOK VIEWING =============

#[tauri::command]
#[specta::specta]
pub fn view_book(book_path: String, app_state_holder: State<AppStateHolder>) -> Result<(), String> {
    let path = PathBuf::from(book_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    iriebook_ui_common::view_book(
        &path,
        &app_state.ebook_publication_manager(),
        &app_state.calibre_access(),
    )
    .map_err(|e| e.to_string())
}

// ============= PROCESSING =============

#[tauri::command]
#[specta::specta]
pub async fn start_processing(
    app: tauri::AppHandle,
    books: Vec<BookInfo>,
    publish_enabled: bool,
    word_stats_enabled: bool,
    embed_cover: bool,
) -> Result<(), String> {
    // Use BatchProcessor from ui-common - all orchestration logic is there
    BatchProcessor::process_books(
        books,
        PublishEnabled::new(publish_enabled),
        WordStatsEnabled::new(word_stats_enabled),
        embed_cover,
        move |event| {
            let _ = ProcessingUpdateEvent(event).emit(&app);
        },
    )
    .await
}

// ============= COLLECTION CRUD =============

#[tauri::command]
#[specta::specta]
pub fn add_book(workspace_root: String, source_md: String) -> Result<AddBookResult, String> {
    let workspace = PathBuf::from(workspace_root);
    let source = PathBuf::from(source_md);

    add_book_with_rescan(&workspace, &source).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn check_duplicate(
    workspace_root: String,
    md_filename: String,
) -> Result<Option<String>, String> {
    let workspace = PathBuf::from(workspace_root);

    check_for_duplicate(&workspace, &md_filename).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn delete_book(book_path: String, workspace_root: String) -> Result<Vec<BookInfo>, String> {
    let book = PathBuf::from(book_path);
    let workspace = PathBuf::from(workspace_root);

    delete_book_with_rescan(&book, &workspace).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn change_book_file(
    book_path: String,
    new_source: String,
    workspace_root: String,
) -> Result<ChangeBookResult, String> {
    let book = PathBuf::from(book_path);
    let source = PathBuf::from(new_source);
    let workspace = PathBuf::from(workspace_root);

    change_book_with_rescan(&book, &source, &workspace).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_book_analysis(
    book_path: String,
    force_refresh: bool,
) -> Result<AnalysisResponse, String> {
    let path = PathBuf::from(book_path);

    get_or_compute_analysis(&path, force_refresh)
}

#[tauri::command]
#[specta::specta]
pub fn get_autocomplete_authors(books: Vec<BookInfo>) -> Result<Vec<String>, String> {
    Ok(collect_distinct_authors(&books))
}

#[tauri::command]
#[specta::specta]
pub fn get_autocomplete_series(books: Vec<BookInfo>) -> Result<Vec<String>, String> {
    Ok(collect_distinct_series(&books))
}
