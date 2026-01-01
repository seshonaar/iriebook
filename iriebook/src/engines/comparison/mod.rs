//! Comparison engine module
//!
//! Provides word-level diff computation between text sources.

pub mod differ;

// Re-export for convenience
pub use differ::Differ;
