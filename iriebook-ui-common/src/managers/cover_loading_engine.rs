use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Status of cover loading operation
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoverStatus {
    /// Not started loading yet
    NotStarted,
    /// Currently loading thumbnail
    Loading,
    /// Thumbnail ready with data URL
    Ready {
        data_url: String,
        #[specta(type = u32)]
        width: u32,
        #[specta(type = u32)]
        height: u32,
    },
    /// Error during loading
    Error { message: String },
}

/// Trait for cover loading (stateful per-book engine)
pub trait CoverLoadingEngine: Send + Sync {
    /// Get current status and thumbnail data
    /// Returns cached data immediately if available or loading
    fn get_thumbnail(&self, cover_path: &Path) -> CoverStatus;
}

/// Callback type for cover loading completion notifications
pub type OnCoverLoaded = Arc<dyn Fn(String) + Send + Sync>;

/// Default cover loading engine that generates thumbnails
pub struct DefaultCoverLoadingEngine {
    cover_path: std::path::PathBuf,
    state: Arc<Mutex<EngineState>>,
    on_loaded: Option<OnCoverLoaded>,
}

#[derive(Clone)]
struct EngineState {
    status: CoverStatus,
}

impl DefaultCoverLoadingEngine {
    pub fn new(cover_path: &Path, on_loaded: Option<OnCoverLoaded>) -> Self {
        Self {
            cover_path: cover_path.to_path_buf(),
            state: Arc::new(Mutex::new(EngineState {
                status: CoverStatus::NotStarted,
            })),
            on_loaded,
        }
    }
}

impl CoverLoadingEngine for DefaultCoverLoadingEngine {
    fn get_thumbnail(&self, cover_path: &Path) -> CoverStatus {
        // Check if path matches this engine's path
        if cover_path != self.cover_path {
            return CoverStatus::Error {
                message: "Path mismatch".to_string(),
            };
        }

        let mut state = self.state.lock().unwrap();

        // Return cached status immediately if available
        if matches!(state.status, CoverStatus::Ready { .. } | CoverStatus::Loading) {
            return state.status.clone();
        }

        // Start loading on first call
        state.status = CoverStatus::Loading;
        drop(state); // Release lock before spawning

        // Spawn background thread for thumbnail generation
        // Using std::thread instead of tokio::spawn because this may be called
        // from a non-async context (synchronous Tauri commands)
        let cover_path = self.cover_path.clone();
        let state_clone = self.state.clone();
        let on_loaded = self.on_loaded.clone();

        std::thread::spawn(move || {
            let result = crate::image_loading::load_cover_as_data_url(&cover_path);

            let mut state = state_clone.lock().unwrap();
            match result {
                Ok(data) => {
                    state.status = CoverStatus::Ready {
                        data_url: data.data_url,
                        width: data.width,
                        height: data.height,
                    };
                }
                Err(e) => {
                    state.status = CoverStatus::Error {
                        message: e.to_string(),
                    };
                }
            }
            drop(state); // Release lock before callback

            // Notify callback that loading is complete
            if let Some(callback) = on_loaded {
                callback(cover_path.to_string_lossy().to_string());
            }
        });

        CoverStatus::Loading
    }
}

/// Mock cover loading engine for testing
pub struct MockCoverLoadingEngine {
    mock_status: Arc<Mutex<CoverStatus>>,
}

impl Default for MockCoverLoadingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCoverLoadingEngine {
    pub fn new() -> Self {
        Self {
            mock_status: Arc::new(Mutex::new(CoverStatus::NotStarted)),
        }
    }

    pub fn set_status(&self, status: CoverStatus) {
        *self.mock_status.lock().unwrap() = status;
    }
}

impl CoverLoadingEngine for MockCoverLoadingEngine {
    fn get_thumbnail(&self, _cover_path: &Path) -> CoverStatus {
        self.mock_status.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_engine_path_mismatch() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cover_path1 = temp_dir.path().join("cover1.jpg");
        let cover_path2 = temp_dir.path().join("cover2.jpg");

        let engine = DefaultCoverLoadingEngine::new(&cover_path1, None);
        let status = engine.get_thumbnail(&cover_path2);

        assert!(matches!(status, CoverStatus::Error { .. }));
    }

    #[test]
    fn test_mock_engine_returns_set_status() {
        let engine = MockCoverLoadingEngine::new();
        engine.set_status(CoverStatus::NotStarted);

        let status = engine.get_thumbnail(&std::path::Path::new("/any/path"));
        assert!(matches!(status, CoverStatus::NotStarted));

        engine.set_status(CoverStatus::Error {
            message: "test error".to_string(),
        });

        let status = engine.get_thumbnail(&std::path::Path::new("/any/path"));
        assert!(matches!(status, CoverStatus::Error { .. }));
    }

    #[test]
    fn test_mock_engine_returns_ready_status() {
        let engine = MockCoverLoadingEngine::new();
        engine.set_status(CoverStatus::Ready {
            data_url: "data:image/jpeg;base64,test".to_string(),
            width: 200,
            height: 300,
        });

        let status = engine.get_thumbnail(&std::path::Path::new("/any/path"));
        match status {
            CoverStatus::Ready {
                data_url,
                width,
                height,
            } => {
                assert_eq!(data_url, "data:image/jpeg;base64,test");
                assert_eq!(width, 200);
                assert_eq!(height, 300);
            }
            _ => panic!("Expected Ready status"),
        }
    }
}
