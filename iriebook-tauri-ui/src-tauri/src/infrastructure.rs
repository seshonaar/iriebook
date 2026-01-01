//! Infrastructure commands: session management, dialogs, and system utilities

use iriebook_ui_common::session::{SessionData, save_session as save_session_impl};

// ============= SESSION MANAGEMENT =============

#[tauri::command]
#[specta::specta]
pub fn load_session() -> Result<Option<SessionData>, String> {
    iriebook_ui_common::session::load_session().map_err(|e| e.to_string())
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
