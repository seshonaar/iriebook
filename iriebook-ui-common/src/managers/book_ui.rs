use super::cover_loading_engine::{
    CoverLoadingEngine, CoverStatus, DefaultCoverLoadingEngine, MockCoverLoadingEngine,
    OnCoverLoaded,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Manager for book-related UI operations and state
///
/// Manages a collection of cover loading engines (one per book).
/// Refreshing the book list clears this collection.
pub struct BookUIManager {
    engines: Vec<(PathBuf, Arc<dyn CoverLoadingEngine>)>,
    use_mock_engine: bool,
    on_cover_loaded: Option<OnCoverLoaded>,
}

impl BookUIManager {
    pub fn new(use_mock_engine: bool) -> Self {
        Self {
            engines: Vec::new(),
            use_mock_engine,
            on_cover_loaded: None,
        }
    }

    /// Set callback to be invoked when any cover finishes loading
    pub fn set_on_cover_loaded<F>(&mut self, callback: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_cover_loaded = Some(Arc::new(callback));
    }

    /// Factory method: create appropriate engine implementation
    fn create_engine(&self, cover_path: &Path) -> Arc<dyn CoverLoadingEngine> {
        if self.use_mock_engine {
            Arc::new(MockCoverLoadingEngine::new())
        } else {
            Arc::new(DefaultCoverLoadingEngine::new(cover_path, self.on_cover_loaded.clone()))
        }
    }

    /// Get or create engine for a specific book
    pub fn get_or_create_engine(&mut self, book_path: &Path) -> Arc<dyn CoverLoadingEngine> {
        // Find existing engine
        if let Some((_, engine)) = self.engines.iter().find(|(p, _)| p == book_path) {
            return engine.clone();
        }

        // Create new engine
        let engine = self.create_engine(book_path);
        self.engines.push((book_path.to_path_buf(), engine.clone()));
        engine
    }

    /// Clear all engines (called when book list is refreshed)
    pub fn clear_engines(&mut self) {
        self.engines.clear();
    }

    /// Get thumbnail for a book's cover
    pub fn get_thumbnail(&mut self, book_path: &Path, cover_path: &Path) -> CoverStatus {
        let engine = self.get_or_create_engine(book_path);
        engine.get_thumbnail(cover_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creates_engine_on_first_request() {
        let mut manager = BookUIManager::new(true); // Use mock engine for test
        let book_path = PathBuf::from("/test/book.md");
        let cover_path = PathBuf::from("/test/cover.jpg");

        // First request should create engine and return NotStarted (mock default)
        let status = manager.get_thumbnail(&book_path, &cover_path);
        assert!(matches!(status, CoverStatus::NotStarted));

        // Engine should be stored
        assert_eq!(manager.engines.len(), 1);
    }

    #[test]
    fn test_manager_reuses_existing_engine() {
        let mut manager = BookUIManager::new(true); // Use mock engine for test
        let book_path = PathBuf::from("/test/book.md");
        let cover_path = PathBuf::from("/test/cover.jpg");

        // First request
        let status1 = manager.get_thumbnail(&book_path, &cover_path);
        let len1 = manager.engines.len();

        // Second request with same book
        let status2 = manager.get_thumbnail(&book_path, &cover_path);
        let len2 = manager.engines.len();

        // Mock engine returns NotStarted for new requests
        assert!(matches!(status1, CoverStatus::NotStarted));
        assert!(matches!(status2, CoverStatus::NotStarted));

        // Should not create another engine
        assert_eq!(len1, len2);
    }

    #[test]
    fn test_manager_clears_engines() {
        let mut manager = BookUIManager::new(true); // Use mock engine for test
        let book_path = PathBuf::from("/test/book.md");
        let cover_path = PathBuf::from("/test/cover.jpg");

        // Create an engine
        manager.get_thumbnail(&book_path, &cover_path);
        assert_eq!(manager.engines.len(), 1);

        // Clear engines
        manager.clear_engines();
        assert_eq!(manager.engines.len(), 0);
    }

    #[test]
    fn test_manager_uses_mock_engine() {
        let mut manager = BookUIManager::new(true);
        let book_path = PathBuf::from("/test/book.md");
        let cover_path = PathBuf::from("/test/cover.jpg");

        // Mock engine should return NotStarted by default
        let status = manager.get_thumbnail(&book_path, &cover_path);
        assert!(matches!(status, CoverStatus::NotStarted));
    }

    #[test]
    fn test_manager_creates_separate_engines() {
        let mut manager = BookUIManager::new(true); // Use mock engine for test
        let book_path1 = PathBuf::from("/test/book1.md");
        let book_path2 = PathBuf::from("/test/book2.md");
        let cover_path1 = PathBuf::from("/test/cover1.jpg");
        let cover_path2 = PathBuf::from("/test/cover2.jpg");

        // Create engines for different books
        manager.get_thumbnail(&book_path1, &cover_path1);
        manager.get_thumbnail(&book_path2, &cover_path2);

        // Should have two engines
        assert_eq!(manager.engines.len(), 2);
    }
}
