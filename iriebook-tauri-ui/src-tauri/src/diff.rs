//! Diff-related commands: viewing revision diffs, local changes, and processing previews

use crate::state::AppStateHolder;
use iriebook::utilities::types::DiffComparison;
use iriebook_ui_common::{GitOperationProgressEvent, RevisionDiff};
use std::path::PathBuf;
use tauri::State;
use tauri_specta::Event;

#[tauri::command]
#[specta::specta]
pub async fn git_get_revision_diffs(
    app: tauri::AppHandle,
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
    commit_hash: String,
) -> Result<Vec<RevisionDiff>, String> {
    let _path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let app_handle = app.clone();

    // Use diff_commands module - all filtering and orchestration logic is there
    iriebook_ui_common::get_revision_diffs(&commit_hash, &app_state.diff_manager(), move |msg| {
        let _ = GitOperationProgressEvent(msg).emit(&app_handle);
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn git_get_local_diffs(
    app: tauri::AppHandle,
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
) -> Result<Vec<RevisionDiff>, String> {
    let _path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let app_handle = app.clone();

    // Use diff_commands module - all filtering and orchestration logic is there
    iriebook_ui_common::get_local_diffs(&app_state.diff_manager(), move |msg| {
        let _ = GitOperationProgressEvent(msg).emit(&app_handle);
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_book_processing_diff(
    app_state_holder: State<'_, AppStateHolder>,
    book_path: String,
) -> Result<DiffComparison, String> {
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    iriebook_ui_common::get_book_processing_diff(&book_path, &app_state.diff_manager()).await
}
