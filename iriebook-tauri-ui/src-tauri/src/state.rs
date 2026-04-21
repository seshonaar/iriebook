use iriebook::utilities::types::PublicationOptions;
#[cfg(feature = "e2e-mocks")]
use iriebook_ui_common::app_state::AppStateBuilder;
use iriebook_ui_common::AppState;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::{oneshot, Mutex};

#[cfg(feature = "e2e-mocks")]
use iriebook_test_support::{
    MockArchiveAccess, MockCalibreAccess, MockGoogleDocsAccess, MockPandocAccess,
};

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
pub struct AppStateHolder {
    app_state: Arc<RwLock<Option<Arc<AppState>>>>,
    publication_options: Arc<RwLock<PublicationOptions>>,
}

impl AppStateHolder {
    pub fn new() -> Self {
        Self {
            app_state: Arc::new(RwLock::new(None)),
            publication_options: Arc::new(RwLock::new(PublicationOptions::default())),
        }
    }

    pub fn initialize(&self, workspace_path: PathBuf) {
        let mut state = self.app_state.write().unwrap();
        *state = Some(Arc::new(AppState::new(workspace_path)));
    }

    #[cfg(feature = "e2e-mocks")]
    pub fn initialize_with_mocks(&self, workspace_path: PathBuf) {
        // Use real GitClient for e2e tests - the test creates a real git repo
        // Only mock the external tools (Pandoc, Calibre, Archive) and remote services (Google Docs)
        let app_state = AppStateBuilder::new()
            .workspace_path(workspace_path)
            .with_google_docs_access(Arc::new(MockGoogleDocsAccess::new()))
            .with_pandoc_access(Arc::new(MockPandocAccess::new()))
            .with_calibre_access(Arc::new(MockCalibreAccess::new()))
            .with_archive_access(Arc::new(MockArchiveAccess::new()))
            .with_defaults_for_remaining() // This will use real GitClient
            .build();

        let mut state = self.app_state.write().unwrap();
        *state = Some(Arc::new(app_state));
    }

    pub fn get(&self) -> Option<Arc<AppState>> {
        // Clone the Arc (cheap) - derefs Option<Arc<T>> and clones the Arc
        self.app_state.read().unwrap().as_ref().map(Arc::clone)
    }

    pub fn publication_options(&self) -> PublicationOptions {
        *self.publication_options.read().unwrap()
    }

    pub fn set_publication_options(&self, options: PublicationOptions) {
        *self.publication_options.write().unwrap() = options.normalized();
    }
}

impl Default for AppStateHolder {
    fn default() -> Self {
        Self::new()
    }
}
