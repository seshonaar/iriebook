use iriebook::engines::analysis::word_analyzer::WordAnalyzer;
use iriebook::engines::comparison::differ::Differ;
use iriebook::engines::text_processing::markdown_transform::MarkdownTransformer;
use iriebook::engines::text_processing::quote_fixer::QuoteFixer;
use iriebook::engines::text_processing::whitespace_trimmer::WhitespaceTrimmer;
use iriebook::engines::validation::validator::Validator;
use iriebook::managers::{
    diff_manager::DiffManager, ebook_publication::EbookPublicationManager,
    google_docs_sync::GoogleDocsSyncManager, repository_manager::RepositoryManager,
};
use iriebook::resource_access::traits::CalibreAccess;
use iriebook::resource_access::{
    archive::ZipArchiver, calibre::CalibreConverter, diff_source::DiffSource, git::GitClient,
    github_auth::GitHubAuthenticator, google_auth::GoogleAuthenticator,
    google_docs::GoogleDocsClient, pandoc::PandocConverter,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Centralized application state holding all managers
///
/// AppState is the single source of truth for all manager instances.
/// It caches managers to avoid repeated instantiation and ensures
/// consistent dependencies across the application.
///
/// Following the Volatility-Based Design principle:
/// - UI layer (Tauri commands) should NEVER instantiate managers
/// - All manager access goes through AppState getters
/// - This makes the UI layer thin and replaceable
pub struct AppState {
    workspace_path: PathBuf,
    repository_manager: Arc<RepositoryManager>,
    google_docs_manager: Arc<GoogleDocsSyncManager>,
    diff_manager: Arc<DiffManager>,
    github_authenticator: Arc<GitHubAuthenticator>,
    google_authenticator: Arc<GoogleAuthenticator>,
    google_docs_client: Arc<GoogleDocsClient>,
    ebook_publication_manager: Arc<EbookPublicationManager>,
}

impl AppState {
    /// Create new AppState with all managers initialized
    ///
    /// This is the ONLY place where managers should be constructed.
    /// All dependencies are assembled here and cached for reuse.
    pub fn new(workspace_path: PathBuf) -> Self {
        // Repository Manager
        let git_client = Arc::new(GitClient);
        let repository_manager = Arc::new(RepositoryManager::new(git_client));

        // Google Docs Manager
        let docs_client = Arc::new(GoogleDocsClient::new());
        let google_docs_manager = Arc::new(GoogleDocsSyncManager::new(docs_client.clone()));

        // Diff Manager
        let diff_source = Arc::new(DiffSource::new(workspace_path.clone()));
        let differ = Arc::new(Differ);
        let diff_manager = Arc::new(DiffManager::new(diff_source, differ));

        // Authenticators
        let github_authenticator = Arc::new(GitHubAuthenticator::new());
        let google_authenticator = Arc::new(GoogleAuthenticator::new());

        // Ebook Publication Manager
        let ebook_publication_manager = Arc::new(EbookPublicationManager::new(
            Arc::new(Validator),
            Arc::new(QuoteFixer),
            Arc::new(WhitespaceTrimmer),
            Arc::new(WordAnalyzer),
            Arc::new(MarkdownTransformer),
            Arc::new(PandocConverter),
            Arc::new(CalibreConverter),
            Arc::new(ZipArchiver),
        ));

        Self {
            workspace_path,
            repository_manager,
            google_docs_manager,
            diff_manager,
            github_authenticator,
            google_authenticator,
            google_docs_client: docs_client,
            ebook_publication_manager,
        }
    }

    /// Get repository manager for git operations
    pub fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.repository_manager.clone()
    }

    /// Get Google Docs sync manager
    pub fn google_docs_manager(&self) -> Arc<GoogleDocsSyncManager> {
        self.google_docs_manager.clone()
    }

    /// Get diff manager for revision analysis
    pub fn diff_manager(&self) -> Arc<DiffManager> {
        self.diff_manager.clone()
    }

    /// Get GitHub authenticator
    pub fn github_authenticator(&self) -> Arc<GitHubAuthenticator> {
        self.github_authenticator.clone()
    }

    /// Get Google authenticator
    pub fn google_authenticator(&self) -> Arc<GoogleAuthenticator> {
        self.google_authenticator.clone()
    }

    /// Get Google Docs client
    pub fn google_docs_client(&self) -> Arc<GoogleDocsClient> {
        self.google_docs_client.clone()
    }

    /// Get workspace path
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    /// Get ebook publication manager for EPUB generation
    pub fn ebook_publication_manager(&self) -> Arc<EbookPublicationManager> {
        self.ebook_publication_manager.clone()
    }

    /// Get Calibre access for viewing ebooks
    pub fn calibre_access(&self) -> Arc<dyn CalibreAccess> {
        Arc::new(CalibreConverter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_app_state_initializes_all_managers() {
        let workspace_path = PathBuf::from("/test/workspace");
        let app_state = AppState::new(workspace_path.clone());

        // Verify workspace path is set correctly
        assert_eq!(app_state.workspace_path(), workspace_path.as_path());

        // Verify all managers are initialized (not panicking)
        let _repo_manager = app_state.repository_manager();
        let _google_manager = app_state.google_docs_manager();
        let _diff_manager = app_state.diff_manager();
        let _github_auth = app_state.github_authenticator();
        let _google_auth = app_state.google_authenticator();
        let _docs_client = app_state.google_docs_client();
    }

    #[test]
    fn test_app_state_getters_return_same_instance() {
        let workspace_path = PathBuf::from("/test/workspace");
        let app_state = AppState::new(workspace_path);

        // Get managers multiple times
        let repo1 = app_state.repository_manager();
        let repo2 = app_state.repository_manager();

        let google1 = app_state.google_docs_manager();
        let google2 = app_state.google_docs_manager();

        let diff1 = app_state.diff_manager();
        let diff2 = app_state.diff_manager();

        // Verify they point to the same instance (Arc::ptr_eq checks pointer equality)
        assert!(Arc::ptr_eq(&repo1, &repo2));
        assert!(Arc::ptr_eq(&google1, &google2));
        assert!(Arc::ptr_eq(&diff1, &diff2));
    }

    #[test]
    fn test_app_state_workspace_path() {
        let workspace_path = PathBuf::from("/my/custom/workspace");
        let app_state = AppState::new(workspace_path.clone());

        assert_eq!(app_state.workspace_path(), workspace_path.as_path());
    }
}
