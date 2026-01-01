use iriebook_ui_common::AppState;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::{Mutex, oneshot};

/// State management for Google Auth Flow cancellation
pub struct GoogleAuthState(pub Mutex<Option<oneshot::Sender<()>>>);

impl GoogleAuthState {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

impl Default for GoogleAuthState {
    fn default() -> Self {
        Self::new()
    }
}

/// State management for AppState (holds all managers)
/// Using RwLock<Option<Arc<AppState>>> to allow initialization after folder selection
pub struct AppStateHolder(Arc<RwLock<Option<Arc<AppState>>>>);

impl AppStateHolder {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }

    pub fn initialize(&self, workspace_path: PathBuf) {
        let mut state = self.0.write().unwrap();
        *state = Some(Arc::new(AppState::new(workspace_path)));
    }

    pub fn get(&self) -> Option<Arc<AppState>> {
        // Clone the Arc (cheap) - derefs Option<Arc<T>> and clones the Arc
        self.0.read().unwrap().as_ref().map(Arc::clone)
    }
}

impl Default for AppStateHolder {
    fn default() -> Self {
        Self::new()
    }
}
