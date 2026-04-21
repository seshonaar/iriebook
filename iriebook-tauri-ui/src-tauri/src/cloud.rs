//! Cloud-related commands: GitHub auth, Google auth, and Google Docs operations

use crate::state::{AppStateHolder, GoogleAuthState};
use iriebook::resource_access::{CredentialStore, GitHubAuthenticator, GoogleDocInfo, PollResult};
use iriebook_ui_common::{
    processing::DefaultBookProcessor, BatchGoogleDocsSyncProcessor, BookInfo, DeviceFlowInfo,
    GoogleDocsBatchSyncUpdateEvent, GoogleDocsProgressEvent,
};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tauri_specta::Event;
use tokio::sync::oneshot;

// ============= GITHUB AUTHENTICATION =============

#[tauri::command]
#[specta::specta]
pub async fn github_device_flow_start() -> Result<DeviceFlowInfo, String> {
    let authenticator = GitHubAuthenticator::new();
    let flow_data = authenticator
        .initiate_device_flow()
        .await
        .map_err(|e| e.to_string())?;

    Ok(DeviceFlowInfo {
        device_code: flow_data.device_code,
        user_code: flow_data.user_code,
        verification_uri: flow_data.verification_uri,
        expires_in: flow_data.expires_in,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn github_device_flow_poll(device_code: String) -> Result<String, String> {
    let authenticator = GitHubAuthenticator::new();

    loop {
        match authenticator
            .poll_for_token(&device_code)
            .await
            .map_err(|e| e.to_string())?
        {
            PollResult::Success(token) => return Ok(token),
            PollResult::Pending => {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
            PollResult::Expired => return Err("Device code expired".to_string()),
            PollResult::Denied => return Err("User denied authorization".to_string()),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn github_store_token(token: String) -> Result<(), String> {
    CredentialStore::store_github_token(&token).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn github_check_auth() -> Result<bool, String> {
    Ok(CredentialStore::has_github_token())
}

#[tauri::command]
#[specta::specta]
pub async fn github_logout() -> Result<(), String> {
    CredentialStore::delete_github_token().map_err(|e| e.to_string())
}

// ============= GOOGLE AUTHENTICATION =============

#[tauri::command]
#[specta::specta]
pub async fn google_auth_start(
    state: State<'_, GoogleAuthState>,
    app_state_holder: State<'_, AppStateHolder>,
) -> Result<(), String> {
    #[cfg(feature = "e2e-mocks")]
    {
        use tracing::warn;
        warn!("🚀 [E2E-TAURI] google_auth_start command called");
    }

    // Cancel any existing flow
    let mut lock = state.0.lock().await;
    if let Some(sender) = lock.take() {
        let _ = sender.send(());
    }

    let (tx, rx) = oneshot::channel();
    *lock = Some(tx);
    drop(lock);

    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    // Delegate to ui-common with browser callback
    let result =
        iriebook_ui_common::start_auth_flow(&app_state.google_authenticator(), rx, |url| {
            open::that(url).map_err(|e| format!("Failed to open browser: {}", e))
        })
        .await;

    // Clean up state
    let mut lock = state.0.lock().await;
    *lock = None;

    #[cfg(feature = "e2e-mocks")]
    {
        use tracing::warn;
        match &result {
            Ok(_) => warn!("🚀 [E2E-TAURI] google_auth_start completed successfully"),
            Err(e) => warn!("🚀 [E2E-TAURI] google_auth_start error: {}", e),
        }
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn google_auth_cancel(state: State<'_, GoogleAuthState>) -> Result<(), String> {
    let mut lock = state.0.lock().await;
    if let Some(sender) = lock.take() {
        let _ = sender.send(());
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn google_check_auth(
    app_state_holder: State<'_, AppStateHolder>,
) -> Result<bool, String> {
    #[cfg(feature = "e2e-mocks")]
    {
        use tracing::warn;
        warn!("🔑 [E2E-TAURI] google_check_auth command called");
    }

    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let result = iriebook_ui_common::check_authenticated(&app_state.google_authenticator()).await;

    #[cfg(feature = "e2e-mocks")]
    {
        use tracing::warn;
        warn!("🔑 [E2E-TAURI] google_check_auth returning: {:?}", result);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn google_logout() -> Result<(), String> {
    iriebook_ui_common::logout()
}

// ============= GOOGLE DOCS OPERATIONS =============

#[tauri::command]
#[specta::specta]
pub async fn google_list_docs(
    app_state_holder: State<'_, AppStateHolder>,
) -> Result<Vec<GoogleDocInfo>, String> {
    #[cfg(feature = "e2e-mocks")]
    {
        use tracing::warn;
        warn!("📋 [E2E-TAURI] google_list_docs command called");
    }

    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    let docs_client = app_state.google_docs_client();
    let result = iriebook_ui_common::list_documents(&app_state.google_authenticator(), &*docs_client, 50)
        .await;

    #[cfg(feature = "e2e-mocks")]
    {
        use tracing::warn;
        match &result {
            Ok(docs) => warn!("📋 [E2E-TAURI] google_list_docs returned {} docs", docs.len()),
            Err(e) => warn!("📋 [E2E-TAURI] google_list_docs error: {}", e),
        }
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn google_link_doc(
    app_state_holder: State<'_, AppStateHolder>,
    book_path: String,
    doc_id: String,
) -> Result<(), String> {
    let path = PathBuf::from(&book_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    iriebook_ui_common::link_document(&path, doc_id, &app_state.google_docs_manager())
}

#[tauri::command]
#[specta::specta]
pub async fn google_sync_doc(
    app_state_holder: State<'_, AppStateHolder>,
    book_path: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let path = PathBuf::from(&book_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    // Create progress callback that emits events to the UI
    let app_handle = app.clone();
    let progress_callback = move |msg: String| {
        let _ = GoogleDocsProgressEvent(msg).emit(&app_handle);
    };

    iriebook_ui_common::sync_document(
        &path,
        Some(app_state.workspace_path()),
        app_state_holder.publication_options(),
        &app_state.google_authenticator(),
        &app_state.google_docs_manager(),
        &iriebook_ui_common::processing::DefaultBookProcessor,
        Some(progress_callback),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn google_unlink_doc(
    app_state_holder: State<'_, AppStateHolder>,
    book_path: String,
) -> Result<(), String> {
    let path = PathBuf::from(&book_path);
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    iriebook_ui_common::unlink_document(&path, &app_state.google_docs_manager())
}

#[tauri::command]
#[specta::specta]
pub async fn google_sync_selected(
    app_state_holder: State<'_, AppStateHolder>,
    books: Vec<BookInfo>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let app_state = app_state_holder
        .get()
        .ok_or_else(|| "App state not initialized".to_string())?;

    BatchGoogleDocsSyncProcessor::new(app_state.workspace_path().to_path_buf()).sync_books(
        books,
        app_state_holder.publication_options(),
        app_state.google_authenticator(),
        app_state.google_docs_manager(),
        Arc::new(DefaultBookProcessor),
        move |event| {
            let _ = GoogleDocsBatchSyncUpdateEvent(event).emit(&app);
        },
    )
    .await
}
