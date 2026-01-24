//! Infrastructure commands: session management, dialogs, and system utilities

use crate::state::AppStateHolder;
use iriebook_ui_common::session::{SessionData, save_session as save_session_impl};
#[cfg(feature = "e2e-mocks")]
use std::path::PathBuf;
use tauri::State;

// ============= SESSION MANAGEMENT =============

#[tauri::command]
#[specta::specta]
pub fn load_session() -> Result<Option<SessionData>, String> {
    // In e2e-mocks mode, check for IRIEBOOK_WORKSPACE env var
    // If set, return a fake session with that workspace path
    #[cfg(feature = "e2e-mocks")]
    {
        eprintln!("[E2E] load_session: checking for IRIEBOOK_WORKSPACE env var");
        match std::env::var("IRIEBOOK_WORKSPACE") {
            Ok(workspace) => {
                eprintln!("[E2E] load_session: found IRIEBOOK_WORKSPACE={}", workspace);
                return Ok(Some(SessionData {
                    folder_path: workspace.into(),
                    selected_book_paths: vec![],
                    current_book_mode: true,
                }));
            }
            Err(e) => {
                eprintln!("[E2E] load_session: IRIEBOOK_WORKSPACE not found: {:?}", e);
            }
        }
        eprintln!("[E2E] load_session: returning None (no env var)");
        Ok(None)
    }

    #[cfg(not(feature = "e2e-mocks"))]
    {
        iriebook_ui_common::session::load_session().map_err(|e| e.to_string())
    }
}

#[tauri::command]
#[specta::specta]
pub fn save_session(session: SessionData) -> Result<(), String> {
    save_session_impl(&session).map_err(|e| e.to_string())
}

// ============= FILE/FOLDER DIALOGS =============

#[tauri::command]
#[specta::specta]
pub fn select_folder() -> Result<Option<String>, String> {
    match rfd::FileDialog::new().pick_folder() {
        Some(folder) => Ok(Some(
            folder.to_str().ok_or("Invalid folder path")?.to_string(),
        )),
        None => Ok(None),
    }
}

#[tauri::command]
#[specta::specta]
pub fn select_file(
    title: String,
    filters: Vec<(String, Vec<String>)>,
) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title(&title);

    // Add file filters
    for (name, extensions) in filters {
        dialog = dialog.add_filter(&name, &extensions);
    }

    match dialog.pick_file() {
        Some(file) => Ok(Some(file.to_str().ok_or("Invalid file path")?.to_string())),
        None => Ok(None),
    }
}

// ============= SYSTEM UTILITIES =============

#[tauri::command]
#[specta::specta]
pub fn open_folder(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("Failed to open folder: {}", e))
}

#[tauri::command]
#[specta::specta]
pub async fn open_browser(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open browser: {}", e))
}

// ============= TEST SUPPORT (E2E MOCK INITIALIZER) =============

#[cfg(feature = "e2e-mocks")]
#[tauri::command]
#[specta::specta]
pub fn init_mock_state(
    state: State<'_, AppStateHolder>,
    workspace_path: String,
) -> Result<(), String> {
    state.initialize_with_mocks(PathBuf::from(workspace_path));
    Ok(())
}

#[cfg(not(feature = "e2e-mocks"))]
#[tauri::command]
#[specta::specta]
pub fn init_mock_state(
    _state: State<'_, AppStateHolder>,
    _workspace_path: String,
) -> Result<(), String> {
    Err("e2e-mocks feature disabled".into())
}
