//! Configuration loading with cascade
//!
//! Loads config from: local config.json → global ~/.iriebook/config.json → defaults

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

const DEFAULT_EXCLUDED_WORDS: &[&str] = &[
    // Common Romanian words that's > 3 chars
    "într", "care", "este", "sunt", "dacă", "pentru"
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrieBookConfig {
    #[serde(default)]
    pub word_analysis: WordAnalysisConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordAnalysisConfig {
    #[serde(default = "default_excluded_words")]
    pub excluded_words: HashSet<String>,
}

impl Default for WordAnalysisConfig {
    fn default() -> Self {
        Self {
            excluded_words: default_excluded_words(),
        }
    }
}

fn default_excluded_words() -> HashSet<String> {
    DEFAULT_EXCLUDED_WORDS
        .iter()
        .map(|s| s.to_string())
        .collect()
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
            config.word_analysis.excluded_words.extend(global.word_analysis.excluded_words);
        }
    }

    // Merge local config
    let local_config = current_dir.join("config.json");
    if let Ok(local) = try_load_config(&local_config) {
        config.word_analysis.excluded_words.extend(local.word_analysis.excluded_words);
    }

    Ok(config)
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

        fs::write(&config_path, r#"{
            "word_analysis": {
                "excluded_words": ["custom1", "custom2"]
            }
        }"#)?;

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
