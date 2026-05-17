use std::path::{Path, PathBuf};
use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    #[serde(rename = "ANTLET_API_KEY", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "ANTLET_API_BASE", skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
    #[serde(rename = "ANTLET_MODEL", skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(rename = "TAVILY_API_KEY", skip_serializing_if = "Option::is_none")]
    pub tavily_api_key: Option<String>,
}

impl ConfigFile {
    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(config_path)
            .context("failed to read config file")?;
        toml::from_str(&content).context("failed to parse config file")
    }

    pub fn save(&self, config_path: &Path) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .context("failed to serialize config")?;
        fs::write(config_path, content)
            .context("failed to write config file")?;
        Ok(())
    }
}

pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("config.toml")
}

pub fn ensure_config(data_dir: &Path) -> Result<ConfigFile> {
    let path = config_path(data_dir);
    if !path.exists() {
        let default_config = ConfigFile::default();
        default_config.save(&path)?;
        eprintln!("Created config file: {}", path.display());
    }
    ConfigFile::load(&path)
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
    pub workspace: PathBuf,
    pub max_steps: usize,
    pub session: String,
    pub data_dir: PathBuf,
    pub profile_dir: PathBuf,
}

impl AppConfig {
    pub fn from_parts(
        workspace: PathBuf,
        max_steps: usize,
        session: String,
        api_base_arg: Option<String>,
        model_arg: Option<String>,
        data_dir: &Path,
    ) -> Result<Self> {
        let api_key = std::env::var("ANTLET_API_KEY")
            .or_else(|_| {
                load_config(data_dir)
                    .ok()
                    .and_then(|c| c.api_key)
                    .ok_or_else(|| anyhow::anyhow!("missing ANTLET_API_KEY. Set it via environment or in ~/.antlet/config.toml"))
            })?;

        let api_base = api_base_arg
            .or_else(|| std::env::var("ANTLET_API_BASE").ok())
            .or_else(|| {
                load_config(data_dir)
                    .ok()
                    .and_then(|c| c.api_base)
            })
            .unwrap_or_else(|| "https://api.minimaxi.com/v1".to_string());

        let model = model_arg
            .or_else(|| std::env::var("ANTLET_MODEL").ok())
            .or_else(|| {
                load_config(data_dir)
                    .ok()
                    .and_then(|c| c.model)
            })
            .unwrap_or_else(|| "MiniMax-M2.5".to_string());

        let profile_dir = std::env::var("ANTLET_PROFILE_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("profile"));

        Ok(Self {
            api_key,
            api_base,
            model,
            workspace,
            max_steps,
            session,
            data_dir: data_dir.to_path_buf(),
            profile_dir,
        })
    }
}

fn load_config(data_dir: &Path) -> Result<ConfigFile> {
    let path = config_path(data_dir);
    if !path.exists() {
        return Ok(ConfigFile::default());
    }
    ConfigFile::load(&path)
}

pub fn get_tavily_api_key(data_dir: &Path) -> Option<String> {
    if let Ok(env_val) = std::env::var("TAVILY_API_KEY") {
        return Some(env_val);
    }
    if let Ok(config) = load_config(data_dir) {
        return config.tavily_api_key;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_save_load() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let config = ConfigFile {
            api_key: Some("test-key".to_string()),
            api_base: Some("https://test.com".to_string()),
            model: Some("test-model".to_string()),
            tavily_api_key: Some("tavily-key".to_string()),
        };

        config.save(&path).unwrap();
        let loaded = ConfigFile::load(&path).unwrap();

        assert_eq!(loaded.api_key, Some("test-key".to_string()));
        assert_eq!(loaded.api_base, Some("https://test.com".to_string()));
        assert_eq!(loaded.model, Some("test-model".to_string()));
        assert_eq!(loaded.tavily_api_key, Some("tavily-key".to_string()));
    }
}