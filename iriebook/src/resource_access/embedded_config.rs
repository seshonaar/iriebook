//! Embedded configuration utilities
//!
//! Helper functions to decode and parse embedded OAuth credentials.

use crate::utilities::error::IrieBookError;
use base64::Engine;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub installed: InstalledConfig,
}

#[derive(Debug, Deserialize)]
pub struct InstalledConfig {
    pub client_id: String,
    pub project_id: Option<String>,
    pub auth_uri: Option<String>,
    pub token_uri: Option<String>,
    pub auth_provider_x509_cert_url: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
}

/// Decodes and parses configuration from a Base64 encoded JSON string.
///
/// The expected JSON structure is:
/// ```json
/// {
///   "installed": {
///     "client_id": "..."
///   }
/// }
/// ```
///
/// # Arguments
/// * `base64_config` - Base64 encoded JSON string
///
/// # Returns
/// * `Ok(InstalledConfig)` containing the configuration
/// * `Err(IrieBookError)` if decoding or parsing fails
pub fn decode_config(base64_config: &str) -> Result<InstalledConfig, IrieBookError> {
    if base64_config.is_empty() {
        return Err(IrieBookError::Validation(
            "Empty credentials string".to_string(),
        ));
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(base64_config)
        .map_err(|e| {
            IrieBookError::Validation(format!("Failed to decode embedded credentials: {}", e))
        })?;

    let config: AuthConfig = serde_json::from_slice(&decoded).map_err(|e| {
        IrieBookError::Validation(format!("Failed to parse embedded credentials: {}", e))
    })?;

    Ok(config.installed)
}
