use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_tool_rounds: usize,
    pub auto_approve_reads: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub active_provider: String,
    pub active_model: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub allowed_dirs: Vec<String>,
    pub session: SessionConfig,
}

impl Default for Settings {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("ollama".to_string(), ProviderConfig {
            api_key: None,
            api_base: Some("http://localhost:11434".to_string()),
        });
        providers.insert("openrouter".to_string(), ProviderConfig {
            api_key: None,
            api_base: Some("https://openrouter.ai/api/v1".to_string()),
        });

        Settings {
            active_provider: "openrouter".to_string(),
            active_model: "anthropic/claude-3.5-sonnet".to_string(),
            providers,
            allowed_dirs: Vec::new(),
            session: SessionConfig {
                max_tool_rounds: 25,
                auto_approve_reads: true,
            },
        }
    }
}

impl Settings {
    /// Loads settings from:
    /// 1. `.env` file (if present)
    /// 2. TOML config file located in standard user config folder (e.g. `~/.orion/config.toml`)
    /// 3. Direct environment variables as overrides
    pub fn load() -> Result<Self> {
        // Try loading .env first
        let _ = dotenvy::dotenv();

        let mut settings = Self::load_from_toml().unwrap_or_else(|_| Settings::default());

        // Override settings with environment variables if present
        if let Ok(prov) = env::var("ORION_PROVIDER") {
            settings.active_provider = prov.to_string();
        }
        if let Ok(model) = env::var("ORION_MODEL") {
            settings.active_model = model.to_string();
        }
        if let Ok(dirs) = env::var("ALLOWED_DIRS") {
            settings.allowed_dirs = dirs
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Gather all provider api keys from environment
        let key_mappings = [
            ("openrouter", "OPENROUTER_API_KEY"),
            ("openai", "OPENAI_API_KEY"),
            ("anthropic", "ANTHROPIC_API_KEY"),
            ("gemini", "GEMINI_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("ollama", "OLLAMA_HOST"),
        ];

        for &(provider_name, env_var) in &key_mappings {
            if let Ok(val) = env::var(env_var) {
                let entry = settings.providers.entry(provider_name.to_string()).or_insert(ProviderConfig {
                    api_key: None,
                    api_base: None,
                });
                if provider_name == "ollama" {
                    entry.api_base = Some(val);
                } else {
                    entry.api_key = Some(val);
                }
            }
        }

        Ok(settings)
    }

    /// Retrieve the API key for the active provider
    pub fn get_active_key(&self) -> Option<String> {
        self.providers
            .get(&self.active_provider)
            .and_then(|c| c.api_key.clone())
    }

    /// Retrieve the base URL for the active provider if custom
    pub fn get_active_base(&self) -> Option<String> {
        self.providers
            .get(&self.active_provider)
            .and_then(|c| c.api_base.clone())
    }

    fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".orion").join("config.toml"))
    }

    fn load_from_toml() -> Result<Self> {
        let path = Self::config_path().context("Failed to get home directory")?;
        if !path.exists() {
            return Ok(Settings::default());
        }
        let content = fs::read_to_string(&path)?;
        let parsed: Settings = toml::from_str(&content)?;
        Ok(parsed)
    }

    /// Save the current settings back to TOML file
    pub fn save(&self) -> Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let serialized = toml::to_string_pretty(self)?;
            fs::write(path, serialized)?;
        }
        Ok(())
    }
}
