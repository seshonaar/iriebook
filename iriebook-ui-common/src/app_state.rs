use crate::managers::BookUIManager;
use iriebook::engines::analysis::word_analyzer::WordAnalyzer;
use iriebook::engines::comparison::differ::Differ;
use iriebook::engines::text_processing::markdown_transform::MarkdownTransformer;
use iriebook::engines::text_processing::quote_fixer::QuoteFixer;
use iriebook::engines::text_processing::whitespace_trimmer::WhitespaceTrimmer;
use iriebook::engines::traits::{
    DifferEngine, MarkdownTransformEngine, QuoteFixerEngine, ValidatorEngine,
    WhitespaceTrimmerEngine, WordAnalyzerEngine,
};
use iriebook::engines::validation::validator::Validator;
use iriebook::managers::{
    diff_manager::DiffManager, ebook_publication::EbookPublicationManager,
    google_docs_sync::GoogleDocsSyncManager, repository_manager::RepositoryManager,
};
use iriebook::resource_access::diff_source::DiffSourceAccess;
use iriebook::resource_access::traits::{
    ArchiveAccess, CalibreAccess, GitAccess, GoogleDocsAccess, PandocAccess,
};
use iriebook::resource_access::{
    archive::ZipArchiver, calibre::CalibreConverter, diff_source::DiffSource, git::GitClient,
    github_auth::GitHubAuthenticator, google_auth::GoogleAuthenticator,
    google_docs::GoogleDocsClient, pandoc::PandocConverter,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Centralized application state holding all managers
///
/// AppState is single source of truth for all manager instances.
/// It caches managers to avoid repeated instantiation and ensures
/// consistent dependencies across application.
///
/// Following Volatility-Based Design principle:
/// - UI layer (Tauri commands) should NEVER instantiate managers
/// - All manager access goes through AppState getters
/// - This makes UI layer thin and replaceable
pub struct AppState {
    workspace_path: PathBuf,
    repository_manager: Arc<RepositoryManager>,
    google_docs_manager: Arc<GoogleDocsSyncManager>,
    diff_manager: Arc<DiffManager>,
    github_authenticator: Arc<GitHubAuthenticator>,
    google_authenticator: Arc<GoogleAuthenticator>,
    google_docs_client: Arc<dyn GoogleDocsAccess>,
    ebook_publication_manager: Arc<EbookPublicationManager>,
    book_ui_manager: Arc<Mutex<BookUIManager>>,
    calibre_access: Arc<dyn CalibreAccess>,
}

/// Builder for creating AppState with custom dependencies
///
/// Use this builder for E2E tests to inject mock implementations
/// of resource access traits. In production, use `AppState::new()`
/// which uses real implementations.
///
/// # Example
/// ```ignore
/// let mock_git = Arc::new(MockGitAccess::new());
/// let app_state = AppStateBuilder::new()
///     .workspace_path(PathBuf::from("/test"))
///     .with_git_access(mock_git)
///     .with_defaults_for_remaining()
///     .build();
/// ```
pub struct AppStateBuilder {
    workspace_path: Option<PathBuf>,
    // Resource Access traits
    git_access: Option<Arc<dyn GitAccess>>,
    google_docs_access: Option<Arc<dyn GoogleDocsAccess>>,
    pandoc_access: Option<Arc<dyn PandocAccess>>,
    calibre_access: Option<Arc<dyn CalibreAccess>>,
    archive_access: Option<Arc<dyn ArchiveAccess>>,
    diff_source: Option<Arc<dyn DiffSourceAccess>>,
    // Engine traits
    validator: Option<Arc<dyn ValidatorEngine>>,
    quote_fixer: Option<Arc<dyn QuoteFixerEngine>>,
    whitespace_trimmer: Option<Arc<dyn WhitespaceTrimmerEngine>>,
    word_analyzer: Option<Arc<dyn WordAnalyzerEngine>>,
    markdown_transformer: Option<Arc<dyn MarkdownTransformEngine>>,
    differ: Option<Arc<dyn DifferEngine>>,
    // Authenticators (not trait-based yet, but included for completeness)
    github_authenticator: Option<Arc<GitHubAuthenticator>>,
    google_authenticator: Option<Arc<GoogleAuthenticator>>,
    // UI-specific
    use_mock_book_ui: bool,
}

impl Default for AppStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppStateBuilder {
    /// Create a new builder with no dependencies set
    pub fn new() -> Self {
        Self {
            workspace_path: None,
            git_access: None,
            google_docs_access: None,
            pandoc_access: None,
            calibre_access: None,
            archive_access: None,
            diff_source: None,
            validator: None,
            quote_fixer: None,
            whitespace_trimmer: None,
            word_analyzer: None,
            markdown_transformer: None,
            differ: None,
            github_authenticator: None,
            google_authenticator: None,
            use_mock_book_ui: false,
        }
    }

    /// Set the workspace path (required)
    pub fn workspace_path(mut self, path: PathBuf) -> Self {
        self.workspace_path = Some(path);
        self
    }

    // --- Resource Access trait setters ---

    /// Inject a custom GitAccess implementation
    pub fn with_git_access(mut self, git: Arc<dyn GitAccess>) -> Self {
        self.git_access = Some(git);
        self
    }

    /// Inject a custom GoogleDocsAccess implementation
    pub fn with_google_docs_access(mut self, docs: Arc<dyn GoogleDocsAccess>) -> Self {
        self.google_docs_access = Some(docs);
        self
    }

    /// Inject a custom PandocAccess implementation
    pub fn with_pandoc_access(mut self, pandoc: Arc<dyn PandocAccess>) -> Self {
        self.pandoc_access = Some(pandoc);
        self
    }

    /// Inject a custom CalibreAccess implementation
    pub fn with_calibre_access(mut self, calibre: Arc<dyn CalibreAccess>) -> Self {
        self.calibre_access = Some(calibre);
        self
    }

    /// Inject a custom ArchiveAccess implementation
    pub fn with_archive_access(mut self, archive: Arc<dyn ArchiveAccess>) -> Self {
        self.archive_access = Some(archive);
        self
    }

    /// Inject a custom DiffSourceAccess implementation
    pub fn with_diff_source(mut self, diff_source: Arc<dyn DiffSourceAccess>) -> Self {
        self.diff_source = Some(diff_source);
        self
    }

    // --- Engine trait setters ---

    /// Inject a custom ValidatorEngine implementation
    pub fn with_validator(mut self, validator: Arc<dyn ValidatorEngine>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Inject a custom QuoteFixerEngine implementation
    pub fn with_quote_fixer(mut self, fixer: Arc<dyn QuoteFixerEngine>) -> Self {
        self.quote_fixer = Some(fixer);
        self
    }

    /// Inject a custom WhitespaceTrimmerEngine implementation
    pub fn with_whitespace_trimmer(mut self, trimmer: Arc<dyn WhitespaceTrimmerEngine>) -> Self {
        self.whitespace_trimmer = Some(trimmer);
        self
    }

    /// Inject a custom WordAnalyzerEngine implementation
    pub fn with_word_analyzer(mut self, analyzer: Arc<dyn WordAnalyzerEngine>) -> Self {
        self.word_analyzer = Some(analyzer);
        self
    }

    /// Inject a custom MarkdownTransformEngine implementation
    pub fn with_markdown_transformer(
        mut self,
        transformer: Arc<dyn MarkdownTransformEngine>,
    ) -> Self {
        self.markdown_transformer = Some(transformer);
        self
    }

    /// Inject a custom DifferEngine implementation
    pub fn with_differ(mut self, differ: Arc<dyn DifferEngine>) -> Self {
        self.differ = Some(differ);
        self
    }

    // --- Authenticator setters ---

    /// Inject a custom GitHubAuthenticator
    pub fn with_github_authenticator(mut self, auth: Arc<GitHubAuthenticator>) -> Self {
        self.github_authenticator = Some(auth);
        self
    }

    /// Inject a custom GoogleAuthenticator
    pub fn with_google_authenticator(mut self, auth: Arc<GoogleAuthenticator>) -> Self {
        self.google_authenticator = Some(auth);
        self
    }

    /// Use mock BookUIManager (for tests)
    pub fn with_mock_book_ui(mut self) -> Self {
        self.use_mock_book_ui = true;
        self
    }

    /// Fill in any unset dependencies with real implementations
    ///
    /// Call this after setting your mock dependencies to fill in the rest
    /// with production implementations.
    pub fn with_defaults_for_remaining(mut self) -> Self {
        let workspace = self
            .workspace_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        if self.git_access.is_none() {
            self.git_access = Some(Arc::new(GitClient));
        }
        if self.google_docs_access.is_none() {
            self.google_docs_access = Some(Arc::new(GoogleDocsClient::new()));
        }
        if self.pandoc_access.is_none() {
            self.pandoc_access = Some(Arc::new(PandocConverter));
        }
        if self.calibre_access.is_none() {
            self.calibre_access = Some(Arc::new(CalibreConverter));
        }
        if self.archive_access.is_none() {
            self.archive_access = Some(Arc::new(ZipArchiver));
        }
        if self.diff_source.is_none() {
            self.diff_source = Some(Arc::new(DiffSource::new(workspace)));
        }
        if self.validator.is_none() {
            self.validator = Some(Arc::new(Validator));
        }
        if self.quote_fixer.is_none() {
            self.quote_fixer = Some(Arc::new(QuoteFixer));
        }
        if self.whitespace_trimmer.is_none() {
            self.whitespace_trimmer = Some(Arc::new(WhitespaceTrimmer));
        }
        if self.word_analyzer.is_none() {
            self.word_analyzer = Some(Arc::new(WordAnalyzer));
        }
        if self.markdown_transformer.is_none() {
            self.markdown_transformer = Some(Arc::new(MarkdownTransformer));
        }
        if self.differ.is_none() {
            self.differ = Some(Arc::new(Differ));
        }
        if self.github_authenticator.is_none() {
            self.github_authenticator = Some(Arc::new(GitHubAuthenticator::new()));
        }
        if self.google_authenticator.is_none() {
            self.google_authenticator = Some(Arc::new(GoogleAuthenticator::new()));
        }
        self
    }

    /// Build the AppState with all configured dependencies
    ///
    /// # Panics
    /// Panics if workspace_path is not set or if any required dependency is missing.
    /// Use `with_defaults_for_remaining()` before calling `build()` to fill in missing deps.
    pub fn build(self) -> AppState {
        let workspace_path = self
            .workspace_path
            .expect("workspace_path is required - call .workspace_path() first");

        // Resource Access
        let git_access = self
            .git_access
            .expect("git_access is required - call .with_git_access() or .with_defaults_for_remaining()");
        let google_docs_access = self
            .google_docs_access
            .expect("google_docs_access is required");
        let pandoc_access = self.pandoc_access.expect("pandoc_access is required");
        let calibre_access = self.calibre_access.expect("calibre_access is required");
        let archive_access = self.archive_access.expect("archive_access is required");
        let diff_source = self.diff_source.expect("diff_source is required");

        // Engines
        let validator = self.validator.expect("validator is required");
        let quote_fixer = self.quote_fixer.expect("quote_fixer is required");
        let whitespace_trimmer = self
            .whitespace_trimmer
            .expect("whitespace_trimmer is required");
        let word_analyzer = self.word_analyzer.expect("word_analyzer is required");
        let markdown_transformer = self
            .markdown_transformer
            .expect("markdown_transformer is required");
        let differ = self.differ.expect("differ is required");

        // Authenticators
        let github_authenticator = self
            .github_authenticator
            .expect("github_authenticator is required");
        let google_authenticator = self
            .google_authenticator
            .expect("google_authenticator is required");

        // Build managers with injected dependencies
        let repository_manager = Arc::new(RepositoryManager::new(git_access));
        let google_docs_manager =
            Arc::new(GoogleDocsSyncManager::new(google_docs_access.clone()));
        let diff_manager = Arc::new(DiffManager::new(diff_source, differ));
        let ebook_publication_manager = Arc::new(EbookPublicationManager::new(
            validator,
            quote_fixer,
            whitespace_trimmer,
            word_analyzer,
            markdown_transformer,
            pandoc_access,
            calibre_access.clone(),
            archive_access,
        ));
        let book_ui_manager = Arc::new(Mutex::new(BookUIManager::new(self.use_mock_book_ui)));

        AppState {
            workspace_path,
            repository_manager,
            google_docs_manager,
            diff_manager,
            github_authenticator,
            google_authenticator,
            google_docs_client: google_docs_access,
            ebook_publication_manager,
            book_ui_manager,
            calibre_access,
        }
    }
}

impl AppState {
    /// Create new AppState with all managers initialized using production implementations
    ///
    /// This is the ONLY place where managers should be constructed in production.
    /// All dependencies are assembled here and cached for reuse.
    ///
    /// For testing with mock dependencies, use `AppStateBuilder` instead.
    pub fn new(workspace_path: PathBuf) -> Self {
        AppStateBuilder::new()
            .workspace_path(workspace_path)
            .with_defaults_for_remaining()
            .build()
    }

    /// Create a builder for custom dependency injection (useful for testing)
    pub fn builder() -> AppStateBuilder {
        AppStateBuilder::new()
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
    pub fn google_docs_client(&self) -> Arc<dyn GoogleDocsAccess> {
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
        self.calibre_access.clone()
    }

    /// Get book UI manager for cover loading and book operations
    pub fn book_ui_manager(&self) -> Arc<Mutex<BookUIManager>> {
        self.book_ui_manager.clone()
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

    #[test]
    fn test_builder_with_defaults() {
        let workspace_path = PathBuf::from("/test/workspace");
        let app_state = AppStateBuilder::new()
            .workspace_path(workspace_path.clone())
            .with_defaults_for_remaining()
            .build();

        assert_eq!(app_state.workspace_path(), workspace_path.as_path());
        // Verify managers are created
        let _repo = app_state.repository_manager();
        let _docs = app_state.google_docs_manager();
        let _diff = app_state.diff_manager();
        let _pub = app_state.ebook_publication_manager();
    }

    #[test]
    fn test_builder_creates_same_result_as_new() {
        let workspace_path = PathBuf::from("/test/workspace");

        // Both methods should create working AppState instances
        let via_new = AppState::new(workspace_path.clone());
        let via_builder = AppStateBuilder::new()
            .workspace_path(workspace_path.clone())
            .with_defaults_for_remaining()
            .build();

        // Verify both have same workspace path
        assert_eq!(via_new.workspace_path(), via_builder.workspace_path());
    }

    #[test]
    fn test_builder_static_method() {
        let workspace_path = PathBuf::from("/test/workspace");
        let app_state = AppState::builder()
            .workspace_path(workspace_path.clone())
            .with_defaults_for_remaining()
            .build();

        assert_eq!(app_state.workspace_path(), workspace_path.as_path());
    }

    #[test]
    #[should_panic(expected = "workspace_path is required")]
    fn test_builder_panics_without_workspace() {
        AppStateBuilder::new().with_defaults_for_remaining().build();
    }

    #[test]
    #[should_panic(expected = "git_access is required")]
    fn test_builder_panics_without_dependencies() {
        AppStateBuilder::new()
            .workspace_path(PathBuf::from("/test"))
            .build();
    }
}
