use std::path::PathBuf;

use anyhow::{Context, Result};

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
    ) -> Result<Self> {
        let api_key = std::env::var("ANTLET_API_KEY")
            .context("missing ANTLET_API_KEY environment variable")?;
        let api_base = api_base_arg
            .or_else(|| std::env::var("ANTLET_API_BASE").ok())
            .unwrap_or_else(|| "https://api.minimaxi.com/v1".to_string());
        let model = model_arg
            .or_else(|| std::env::var("ANTLET_MODEL").ok())
            .unwrap_or_else(|| "MiniMax-M2.5".to_string());

        let data_dir = std::env::var("ANTLET_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".antlet"))
            })
            .unwrap_or_else(|| PathBuf::from(".antlet"));

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
            data_dir,
            profile_dir,
        })
    }
}
