//! Git repository commands: local git operations (clone, sync, save, status, log)

use crate::state::AppStateHolder;
use iriebook_ui_common::{
    BookListChangedEvent, GitCommit, GitOperationProgressEvent, GitSyncStatus,
};
use std::path::PathBuf;
use tauri::State;
use tauri_specta::Event;

#[tauri::command]
#[specta::specta]
pub async fn git_check_initialized(workspace_path: String) -> Result<bool, String> {
    let path = PathBuf::from(workspace_path);
    iriebook_ui_common::check_initialized(&path)
}

#[tauri::command]
#[specta::specta]
pub async fn git_clone_repository(
    app: tauri::AppHandle,
    app_state_holder: State<'_, AppStateHolder>,
    github_url: String,
    workspace_path: String,
) -> Result<(), String> {
    let path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let _ = GitOperationProgressEvent("Cloning repository...".to_string()).emit(&app);

    let result =
        iriebook_ui_common::clone_repository(&github_url, &path, &app_state.repository_manager())
            .await;

    let _ = GitOperationProgressEvent("Clone complete".to_string()).emit(&app);

    // Signal UI to refresh book list after clone
    if result.is_ok() {
        let _ = BookListChangedEvent {}.emit(&app);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn git_sync(
    app: tauri::AppHandle,
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
) -> Result<String, String> {
    let path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let _ = GitOperationProgressEvent("Syncing...".to_string()).emit(&app);

    let result = iriebook_ui_common::sync_repository(&path, &app_state.repository_manager()).await;

    let _ = GitOperationProgressEvent("Sync complete".to_string()).emit(&app);

    // Signal UI to refresh book list after sync (could have pulled new/deleted books)
    if result.is_ok() {
        let _ = BookListChangedEvent {}.emit(&app);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn git_save(
    app: tauri::AppHandle,
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
    message: String,
) -> Result<String, String> {
    let path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let _ = GitOperationProgressEvent("Saving...".to_string()).emit(&app);

    let result =
        iriebook_ui_common::save_repository(&path, &message, &app_state.repository_manager()).await;

    let _ = GitOperationProgressEvent("Save complete".to_string()).emit(&app);

    // Signal UI to refresh book list after save (could have saved new books)
    if result.is_ok() {
        let _ = BookListChangedEvent {}.emit(&app);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn git_reset_local_changes(
    app: tauri::AppHandle,
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
) -> Result<String, String> {
    let path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let _ = GitOperationProgressEvent("Resetting local changes...".to_string()).emit(&app);

    let result = iriebook_ui_common::reset_repository(&path, &app_state.repository_manager()).await;

    let _ = GitOperationProgressEvent("Reset complete".to_string()).emit(&app);

    if result.is_ok() {
        let _ = BookListChangedEvent {}.emit(&app);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn git_get_log(
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
    limit: u32,
) -> Result<Vec<GitCommit>, String> {
    let path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    iriebook_ui_common::get_commit_history(&path, &app_state.repository_manager(), limit as usize)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn git_get_status(
    app_state_holder: State<'_, AppStateHolder>,
    workspace_path: String,
) -> Result<GitSyncStatus, String> {
    let path = PathBuf::from(workspace_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    iriebook_ui_common::get_sync_status(&path, &app_state.repository_manager()).await
}

#[tauri::command]
#[specta::specta]
pub async fn git_get_remote_url(workspace_path: String) -> Result<String, String> {
    let path = PathBuf::from(workspace_path);
    iriebook_ui_common::get_remote_url(&path)
}
