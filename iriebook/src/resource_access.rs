//! Resource Access layer - External resource abstraction
//!
//! Resource Access components encapsulate access to external resources:
//! - File I/O operations
//! - Configuration loading
//! - External tool execution (pandoc, calibre, zip)
//! - Git version control operations
//! - GitHub authentication
//! - Google OAuth authentication
//! - Google Docs API access
//! - Secure credential storage
//! - Diff source abstraction (filesystem and git revisions)
//!
//! Following the Righting Software Method, Resource Access:
//! - Abstracts "where to get data from"
//! - Hides implementation details of resource access
//! - Can be shared across Engines and Managers
//! - Makes testing easier through mocking/stubbing

pub mod traits;
pub mod file;
pub mod config;
pub mod command;
pub mod pandoc;
pub mod calibre;
pub mod archive;
pub mod git;
pub mod github_auth;
pub mod google_auth;
pub mod google_docs;
pub mod credential_storage;
pub mod embedded_config;
pub mod diff_source;

// Re-export commonly used types for convenience
pub use git::GitClient;
pub use github_auth::{GitHubAuthenticator, DeviceFlowData, PollResult};
pub use google_auth::{GoogleAuthenticator, AuthFlow, AuthCodeTokenResponse};
pub use google_docs::GoogleDocsClient;
pub use credential_storage::CredentialStore;
pub use traits::GoogleDocInfo;
pub use diff_source::{DiffSourceAccess, DiffSource};
