//! IrieBook - Ebook publication pipeline
//!
//! This library converts straight double quotes to curly quotes,
//! cleans up whitespace, and generates professional ebooks.
//!
//! # Architecture
//!
//! Following the Righting Software Method, this codebase is organized by
//! volatility rather than by function:
//!
//! - **Client**: Presentation layer (CLI)
//! - **Managers**: Workflow orchestration
//! - **Engines**: Business logic implementations (organized by volatility domain)
//!   - text_processing: Quote fixing, whitespace, markdown (change together)
//!   - analysis: Word frequency analysis
//!   - validation: Quote validation
//! - **Resource Access**: External resource abstraction (files, config, external tools)
//! - **Utilities**: Cross-cutting concerns (types, errors)

pub mod client;
pub mod managers;
pub mod engines;
pub mod resource_access;
pub mod utilities;
