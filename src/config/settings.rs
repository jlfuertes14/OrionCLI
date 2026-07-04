use crate::session::ContextLimits;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_tool_rounds: usize,
    pub auto_approve_reads: bool,
    #[serde(default)]
    pub max_budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandSandboxMode {
    Host,
    Docker,
}

impl Default for CommandSandboxMode {
    fn default() -> Self {
        Self::Host
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchProvider {
    Tavily,
    DuckDuckGo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub active_provider: String,
    pub active_model: String,
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(skip)]
    pub workspace_dir: PathBuf,
    pub allowed_dirs: Vec<String>,
    pub session: SessionConfig,
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    #[serde(default)]
    pub fallbacks: Vec<String>,
    #[serde(default)]
    pub command_sandbox: CommandSandboxMode,
    #[serde(default = "default_docker_image")]
    pub docker_image: Option<String>,
    #[serde(default)]
    pub context_limits: ContextLimits,
    #[serde(default)]
    pub search_provider: Option<SearchProvider>,
    #[serde(default)]
    pub tavily_api_key: Option<String>,
    #[serde(skip)]
    pub trace_enabled: bool,
}

fn default_docker_image() -> Option<String> {
    Some("ubuntu:latest".to_string())
}

impl Default for Settings {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                api_key: None,
                api_base: Some("http://localhost:11434".to_string()),
            },
        );
        providers.insert(
            "openrouter".to_string(),
            ProviderConfig {
                api_key: None,
                api_base: Some("https://openrouter.ai/api/v1".to_string()),
            },
        );

        Settings {
            active_provider: "openrouter".to_string(),
            active_model: "anthropic/claude-3.5-sonnet".to_string(),
            providers,
            workspace_dir: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            allowed_dirs: Vec::new(),
            session: SessionConfig {
                max_tool_rounds: 25,
                auto_approve_reads: true,
                max_budget_usd: None,
            },
            mcp_servers: None,
            fallbacks: Vec::new(),
            command_sandbox: CommandSandboxMode::Host,
            docker_image: default_docker_image(),
            context_limits: ContextLimits::default(),
            search_provider: None,
            tavily_api_key: None,
            trace_enabled: false,
        }
    }
}

impl Settings {
    /// Loads settings from:
    /// 1. `.env` file (if present)
    /// 2. TOML config file located in standard user config folder (e.g. `~/.orion/config.toml`)
    /// 3. Direct environment variables as overrides
    pub fn load() -> Result<Self> {
        let launch_dir =
            env::current_dir().context("Failed to detect current workspace directory")?;

        // Try loading .env first
        let _ = dotenvy::dotenv();

        let mut settings = Self::load_from_toml().unwrap_or_else(|_| Settings::default());
        settings.workspace_dir = launch_dir;

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
        settings.ensure_workspace_allowed();

        // Gather all provider api keys from environment
        let key_mappings = [
            ("openrouter", "OPENROUTER_API_KEY"),
            ("openai", "OPENAI_API_KEY"),
            ("anthropic", "ANTHROPIC_API_KEY"),
            ("gemini", "GEMINI_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("ollama", "OLLAMA_HOST"),
            ("cloudflare", "CLOUDFLARE_API_KEY"),
        ];

        for &(provider_name, env_var) in &key_mappings {
            if let Ok(val) = env::var(env_var) {
                let trimmed = val.trim();
                if trimmed.is_empty() || trimmed.starts_with("your_") || trimmed.ends_with("_here")
                {
                    continue;
                }
                let entry = settings
                    .providers
                    .entry(provider_name.to_string())
                    .or_insert(ProviderConfig {
                        api_key: None,
                        api_base: None,
                    });
                if provider_name == "ollama" {
                    entry.api_base = Some(trimmed.to_string());
                } else {
                    entry.api_key = Some(trimmed.to_string());
                }
            }
        }

        // Fallbacks for Cloudflare / Workers AI env vars
        if let Ok(val) = env::var("WORKERS_AI_API_KEY") {
            let trimmed = val.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("your_") && !trimmed.ends_with("_here") {
                let entry = settings
                    .providers
                    .entry("cloudflare".to_string())
                    .or_insert(ProviderConfig {
                        api_key: None,
                        api_base: None,
                    });
                if entry.api_key.is_none() {
                    entry.api_key = Some(trimmed.to_string());
                }
            }
        }

        if let Ok(val) = env::var("TAVILY_API_KEY") {
            let trimmed = val.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("your_") && !trimmed.ends_with("_here") {
                settings.tavily_api_key = Some(trimmed.to_string());
            }
        }

        if let Ok(account_id) =
            env::var("CLOUDFLARE_ACCOUNT_ID").or_else(|_| env::var("WORKERS_AI_ACCOUNT_ID"))
        {
            let trimmed = account_id.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("your_") && !trimmed.ends_with("_here") {
                let entry = settings
                    .providers
                    .entry("cloudflare".to_string())
                    .or_insert(ProviderConfig {
                        api_key: None,
                        api_base: None,
                    });
                if entry.api_base.is_none() {
                    entry.api_base = Some(format!(
                        "https://api.cloudflare.com/client/v4/accounts/{}/ai/v1",
                        trimmed
                    ));
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

    pub fn set_provider_key(&mut self, provider: &str, api_key: &str) -> Result<()> {
        let entry = self
            .providers
            .entry(provider.to_string())
            .or_insert(ProviderConfig {
                api_key: None,
                api_base: None,
            });
        entry.api_key = Some(api_key.to_string());
        Ok(())
    }

    fn ensure_workspace_allowed(&mut self) {
        let workspace = self.workspace_dir.to_string_lossy().to_string();
        let workspace_canonical = self
            .workspace_dir
            .canonicalize()
            .unwrap_or_else(|_| self.workspace_dir.clone());
        let already_allowed = self.allowed_dirs.iter().any(|dir| {
            let path = PathBuf::from(dir);
            path.canonicalize().unwrap_or(path) == workspace_canonical
        });

        if !already_allowed {
            self.allowed_dirs.push(workspace);
        }
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
