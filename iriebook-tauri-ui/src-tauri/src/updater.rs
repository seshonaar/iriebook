//! Application update functionality using tauri-plugin-updater

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use specta::Type;
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;

/// Event emitted during update progress
#[derive(Clone, Serialize, Type, Event)]
pub struct UpdateProgressEvent(pub UpdateProgress);

/// Update progress status
#[derive(Clone, Serialize, Type)]
#[serde(tag = "type")]
pub enum UpdateProgress {
    Checking,
    NoUpdate,
    UpdateAvailable { version: String },
    Downloading { percent: u32 },
    Installing,
    Done,
    Error { message: String },
}

/// Check for updates and install if available
#[tauri::command]
#[specta::specta]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<(), String> {
    // Emit checking status
    let _ = UpdateProgressEvent(UpdateProgress::Checking).emit(&app);

    // Check for updates
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            let msg = e.to_string();
            let _ = UpdateProgressEvent(UpdateProgress::Error { message: msg.clone() }).emit(&app);
            return Err(msg);
        }
    };

    let update = match updater.check().await {
        Ok(u) => u,
        Err(e) => {
            let msg = e.to_string();
            let _ = UpdateProgressEvent(UpdateProgress::Error { message: msg.clone() }).emit(&app);
            return Err(msg);
        }
    };

    match update {
        Some(update) => {
            let version = update.version.clone();
            let _ = UpdateProgressEvent(UpdateProgress::UpdateAvailable { version }).emit(&app);

            // Clone app handle for the callbacks
            let app_download = app.clone();
            let app_finish = app.clone();

            // Track cumulative downloaded bytes (callback gives us chunk size, not total)
            let downloaded_bytes = Arc::new(AtomicU64::new(0));
            let downloaded_bytes_clone = Arc::clone(&downloaded_bytes);

            // Download and install
            if let Err(e) = update
                .download_and_install(
                    move |chunk_length, total| {
                        let downloaded = downloaded_bytes_clone
                            .fetch_add(chunk_length as u64, Ordering::Relaxed)
                            + chunk_length as u64;
                        let percent = total
                            .map(|t| ((downloaded as f64 / t as f64) * 100.0) as u32)
                            .unwrap_or(0);
                        let _ = UpdateProgressEvent(UpdateProgress::Downloading { percent })
                            .emit(&app_download);
                    },
                    move || {
                        let _ = UpdateProgressEvent(UpdateProgress::Installing).emit(&app_finish);
                    },
                )
                .await
            {
                let msg = e.to_string();
                let _ = UpdateProgressEvent(UpdateProgress::Error { message: msg.clone() }).emit(&app);
                return Err(msg);
            }

            let _ = UpdateProgressEvent(UpdateProgress::Done).emit(&app);

            // Restart the app
            app.restart();
        }
        None => {
            let _ = UpdateProgressEvent(UpdateProgress::NoUpdate).emit(&app);
        }
    }

    Ok(())
}
