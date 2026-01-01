//! Utility layer - Cross-cutting concerns
//!
//! Utilities provide common infrastructure used by all other components:
//! - Type definitions (NewType pattern for type safety)
//! - Error handling (custom error types with rich context)
//!
//! Following the Righting Software Method, Utilities:
//! - Are non-volatile (rarely change)
//! - Provide cross-cutting concerns
//! - Can be accessed from any layer
//! - Do not contain business logic

pub mod types;
pub mod error;
pub mod diff_trimmer;
