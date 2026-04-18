//! Configuration loading with cascade
//!
//! Loads config from: local config.json → global ~/.iriebook/config.json → defaults

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

const DEFAULT_EXCLUDED_WORDS: &[&str] = &[
    // Common Romanian words that's > 3 chars
    "într", "care", "este", "sunt", "dacă", "pentru",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrieBookConfig {
    #[serde(default)]
    pub word_analysis: WordAnalysisConfig,
    #[serde(default)]
    pub pdf: PdfConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordAnalysisConfig {
    #[serde(default = "default_excluded_words")]
    pub excluded_words: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PdfConfig {
    #[serde(default = "default_pdf_enabled")]
    pub enabled: bool,
    #[serde(default = "default_pdf_page_width")]
    pub page_width: String,
    #[serde(default = "default_pdf_page_height")]
    pub page_height: String,
    #[serde(default = "default_pdf_font_family")]
    pub font_family: String,
    #[serde(default = "default_pdf_font_size")]
    pub font_size: String,
    #[serde(default = "default_pdf_line_spacing")]
    pub line_spacing: f64,
    #[serde(default = "default_pdf_inner_margin")]
    pub inner_margin: String,
    #[serde(default = "default_pdf_outer_margin")]
    pub outer_margin: String,
    #[serde(default = "default_pdf_top_margin")]
    pub top_margin: String,
    #[serde(default = "default_pdf_bottom_margin")]
    pub bottom_margin: String,
    #[serde(default = "default_pdf_justified")]
    pub justified: bool,
    #[serde(default = "default_pdf_engine")]
    pub pdf_engine: String,
}

impl Default for WordAnalysisConfig {
    fn default() -> Self {
        Self {
            excluded_words: default_excluded_words(),
        }
    }
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            enabled: default_pdf_enabled(),
            page_width: default_pdf_page_width(),
            page_height: default_pdf_page_height(),
            font_family: default_pdf_font_family(),
            font_size: default_pdf_font_size(),
            line_spacing: default_pdf_line_spacing(),
            inner_margin: default_pdf_inner_margin(),
            outer_margin: default_pdf_outer_margin(),
            top_margin: default_pdf_top_margin(),
            bottom_margin: default_pdf_bottom_margin(),
            justified: default_pdf_justified(),
            pdf_engine: default_pdf_engine(),
        }
    }
}

fn default_excluded_words() -> HashSet<String> {
    DEFAULT_EXCLUDED_WORDS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_pdf_enabled() -> bool {
    true
}

fn default_pdf_page_width() -> String {
    "5.5in".to_string()
}

fn default_pdf_page_height() -> String {
    "8.5in".to_string()
}

fn default_pdf_font_family() -> String {
    "Liberation Serif".to_string()
}

fn default_pdf_font_size() -> String {
    "11pt".to_string()
}

fn default_pdf_line_spacing() -> f64 {
    1.2
}

fn default_pdf_inner_margin() -> String {
    "2.2cm".to_string()
}

fn default_pdf_outer_margin() -> String {
    "1.8cm".to_string()
}

fn default_pdf_top_margin() -> String {
    "1.8cm".to_string()
}

fn default_pdf_bottom_margin() -> String {
    "1.8cm".to_string()
}

fn default_pdf_justified() -> bool {
    true
}

fn default_pdf_engine() -> String {
    "xelatex".to_string()
}

/// Load configuration with merging: defaults + global + local
///
/// All three levels are merged together:
/// - Defaults provide base Romanian stopwords
/// - Global config adds user-wide customizations
/// - Local config adds project-specific customizations
pub fn load_config(current_dir: &Path) -> Result<IrieBookConfig> {
    // Start with defaults
    let mut config = IrieBookConfig::default();

    // Merge global config (~/.iriebook/config.json)
    if let Some(home) = dirs::home_dir() {
        let global_config = home.join(".iriebook/config.json");
        if let Ok(global) = try_load_config(&global_config) {
            config
                .word_analysis
                .excluded_words
                .extend(global.word_analysis.excluded_words);
            config.pdf = global.pdf;
        }
    }

    // Merge local config
    let local_config = current_dir.join("config.json");
    if let Ok(local) = try_load_config(&local_config) {
        config
            .word_analysis
            .excluded_words
            .extend(local.word_analysis.excluded_words);
        config.pdf = local.pdf;
    }

    Ok(config)
}

/// Create or update the editable library-root config with missing default sections.
///
/// Existing valid settings are preserved. Invalid JSON is never overwritten.
pub fn ensure_config_defaults(config_root: &Path) -> Result<()> {
    let config_path = config_root.join("config.json");
    let default_value = serde_json::to_value(IrieBookConfig::default())
        .context("Failed to serialize default config")?;

    if !config_path.exists() {
        let content = serde_json::to_string_pretty(&default_value)
            .context("Failed to serialize default config")?;
        std::fs::write(&config_path, format!("{}\n", content))
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
    let mut current: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    if !current.is_object() {
        anyhow::bail!(
            "Config file root must be a JSON object: {}",
            config_path.display()
        );
    }
    if !default_value.is_object() {
        anyhow::bail!("Default config root must be a JSON object");
    }

    let changed = merge_missing_defaults(&mut current, &default_value);

    if changed {
        let next_content =
            serde_json::to_string_pretty(&current).context("Failed to serialize merged config")?;
        std::fs::write(&config_path, format!("{}\n", next_content))
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
    }

    Ok(())
}

fn merge_missing_defaults(current: &mut serde_json::Value, defaults: &serde_json::Value) -> bool {
    let (Some(current_object), Some(default_object)) =
        (current.as_object_mut(), defaults.as_object())
    else {
        return false;
    };

    let mut changed = false;
    for (key, default_value) in default_object {
        match current_object.get_mut(key) {
            Some(current_value) if current_value.is_object() && default_value.is_object() => {
                changed |= merge_missing_defaults(current_value, default_value);
            }
            Some(_) => {}
            None => {
                current_object.insert(key.clone(), default_value.clone());
                changed = true;
            }
        }
    }

    changed
}

fn try_load_config(path: &Path) -> Result<IrieBookConfig> {
    if !path.exists() {
        anyhow::bail!("Config file does not exist");
    }

    let content = std::fs::read_to_string(path)?;
    let config: IrieBookConfig = serde_json::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn loads_default_config_when_no_files_exist() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = load_config(temp_dir.path())?;

        assert!(config.word_analysis.excluded_words.contains("care"));
        assert!(config.word_analysis.excluded_words.contains("este"));
        assert!(config.word_analysis.excluded_words.contains("sunt"));

        Ok(())
    }

    #[test]
    fn loads_local_config_and_merges_with_defaults() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        fs::write(
            &config_path,
            r#"{
            "word_analysis": {
                "excluded_words": ["custom1", "custom2"]
            }
        }"#,
        )?;

        let config = load_config(temp_dir.path())?;

        // Local words should be present
        assert!(config.word_analysis.excluded_words.contains("custom1"));
        assert!(config.word_analysis.excluded_words.contains("custom2"));

        // Defaults should still be present (merged, not replaced)
        assert!(config.word_analysis.excluded_words.contains("care"));
        assert!(config.word_analysis.excluded_words.contains("este"));
        assert!(config.word_analysis.excluded_words.contains("pentru"));

        Ok(())
    }

    #[test]
    fn invalid_json_falls_back_to_defaults() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        // Write malformed JSON
        fs::write(&config_path, "{ invalid json }")?;

        // Should fall back to defaults without crashing
        let config = load_config(temp_dir.path())?;
        assert!(config.word_analysis.excluded_words.contains("care"));

        Ok(())
    }

    #[test]
    fn missing_config_is_not_error() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let result = load_config(temp_dir.path());
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn default_config_has_romanian_stopwords() {
        let config = IrieBookConfig::default();

        assert!(config.word_analysis.excluded_words.contains("într"));
        assert!(config.word_analysis.excluded_words.contains("care"));
        assert!(config.word_analysis.excluded_words.contains("este"));
        assert!(config.word_analysis.excluded_words.contains("sunt"));
        assert!(config.word_analysis.excluded_words.contains("dacă"));
        assert!(config.word_analysis.excluded_words.contains("pentru"));
    }

    #[test]
    fn default_config_has_pdf_print_defaults() {
        let config = IrieBookConfig::default();

        assert!(config.pdf.enabled);
        assert_eq!(config.pdf.page_width, "5.5in");
        assert_eq!(config.pdf.page_height, "8.5in");
        assert_eq!(config.pdf.font_family, "Liberation Serif");
        assert_eq!(config.pdf.font_size, "11pt");
        assert_eq!(config.pdf.line_spacing, 1.2);
        assert_eq!(config.pdf.inner_margin, "2.2cm");
        assert_eq!(config.pdf.outer_margin, "1.8cm");
        assert_eq!(config.pdf.top_margin, "1.8cm");
        assert_eq!(config.pdf.bottom_margin, "1.8cm");
        assert!(config.pdf.justified);
        assert_eq!(config.pdf.pdf_engine, "xelatex");
    }

    #[test]
    fn ensure_config_defaults_creates_missing_config() -> Result<()> {
        let temp_dir = TempDir::new()?;

        ensure_config_defaults(temp_dir.path())?;

        let config_path = temp_dir.path().join("config.json");
        assert!(config_path.exists());

        let config = try_load_config(&config_path)?;
        assert_eq!(config.pdf, PdfConfig::default());
        assert!(config.word_analysis.excluded_words.contains("care"));

        Ok(())
    }

    #[test]
    fn ensure_config_defaults_adds_missing_pdf_section_without_removing_words() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{
              "word_analysis": {
                "excluded_words": ["custom"]
              }
            }"#,
        )?;

        ensure_config_defaults(temp_dir.path())?;

        let content = fs::read_to_string(&config_path)?;
        let value: serde_json::Value = serde_json::from_str(&content)?;
        assert!(value.get("pdf").is_some());

        let config = try_load_config(&config_path)?;
        assert!(config.word_analysis.excluded_words.contains("custom"));
        assert_eq!(config.pdf, PdfConfig::default());

        Ok(())
    }

    #[test]
    fn ensure_config_defaults_adds_missing_pdf_fields() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{
              "pdf": {
                "font_family": "Liberation Serif"
              }
            }"#,
        )?;

        ensure_config_defaults(temp_dir.path())?;

        let config = try_load_config(&config_path)?;
        assert_eq!(config.pdf.font_family, "Liberation Serif");
        assert_eq!(config.pdf.page_width, "5.5in");
        assert_eq!(config.pdf.inner_margin, "2.2cm");

        Ok(())
    }

    #[test]
    fn ensure_config_defaults_does_not_overwrite_invalid_json() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{ invalid json }")?;

        assert!(ensure_config_defaults(temp_dir.path()).is_err());
        assert_eq!(fs::read_to_string(&config_path)?, "{ invalid json }");

        Ok(())
    }

    #[test]
    fn default_excluded_words_function_returns_correct_set() {
        let excluded = default_excluded_words();

        // Check prepositions
        assert!(excluded.contains("într"));
        assert!(excluded.contains("pentru"));

        // Check pronouns
        assert!(excluded.contains("care"));

        // Check verbs
        assert!(excluded.contains("este"));
        assert!(excluded.contains("sunt"));

        // Check conjunctions
        assert!(excluded.contains("dacă"));
    }
}
