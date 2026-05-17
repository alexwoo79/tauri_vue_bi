use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
    pub is_custom: bool,
    pub context_window: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub enable_thinking: bool,
}

pub struct LLMConfigManager {
    configs: HashMap<String, LLMConfig>,
    config_dir: PathBuf,
}

impl LLMConfigManager {
    pub fn new() -> Self {
        let config_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("LLM");

        fs::create_dir_all(&config_dir).ok();

        Self {
            configs: HashMap::new(),
            config_dir,
        }
    }

    pub fn load_from_file(&mut self) -> anyhow::Result<()> {
        let config_file = self.config_dir.join("llm_config.json");

        if !config_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&config_file)?;
        let configs: HashMap<String, LLMConfig> = serde_json::from_str(&content)?;

        self.configs = configs;
        Ok(())
    }

    pub fn save_to_file(&self) -> anyhow::Result<()> {
        let config_file = self.config_dir.join("llm_config.json");
        let content = serde_json::to_string_pretty(&self.configs)?;
        fs::write(&config_file, content)?;
        Ok(())
    }

    pub fn get_config(&self, provider: &str) -> Option<&LLMConfig> {
        self.configs.get(provider)
    }

    pub fn set_config(&mut self, provider: String, config: LLMConfig) {
        self.configs.insert(provider, config);
    }

    pub fn get_enabled_configs(&self) -> Vec<&LLMConfig> {
        self.configs.values().filter(|c| c.enabled).collect()
    }

    pub fn get_default_provider(&self) -> Option<String> {
        self.configs
            .iter()
            .find(|(_, c)| c.enabled)
            .map(|(k, _)| k.clone())
    }
}
