pub mod anthropic;
pub mod gemini;
pub mod mistral;
pub mod ollama;
pub mod openai;
pub mod provider;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use mistral::MistralProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use provider::{ChatRequest, FunctionCall, LlmProvider, Message, StreamChunk, ToolCall};

use crate::config::Settings;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    RateLimited,
    Unavailable,
    Auth,
    BadRequest,
    Network,
    Unknown,
}

impl ProviderErrorKind {
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            ProviderErrorKind::RateLimited
                | ProviderErrorKind::Unavailable
                | ProviderErrorKind::Network
                | ProviderErrorKind::Unknown
        )
    }
}

pub fn classify_provider_error(error: &anyhow::Error) -> ProviderErrorKind {
    let text = error.to_string().to_lowercase();
    if text.contains("429") || text.contains("rate limit") || text.contains("too many requests") {
        ProviderErrorKind::RateLimited
    } else if text.contains("503")
        || text.contains("502")
        || text.contains("504")
        || text.contains("500")
        || text.contains("unavailable")
        || text.contains("temporarily")
    {
        ProviderErrorKind::Unavailable
    } else if text.contains("401")
        || text.contains("403")
        || text.contains("unauthorized")
        || text.contains("forbidden")
        || text.contains("invalid api key")
    {
        ProviderErrorKind::Auth
    } else if text.contains("400")
        || text.contains("bad request")
        || text.contains("invalid request")
    {
        ProviderErrorKind::BadRequest
    } else if text.contains("connection")
        || text.contains("dns")
        || text.contains("timeout")
        || text.contains("network")
    {
        ProviderErrorKind::Network
    } else {
        ProviderErrorKind::Unknown
    }
}

/// Instantiate the active LLM provider based on current Settings credentials
pub fn get_provider(settings: &Settings) -> Result<Box<dyn LlmProvider>> {
    let provider_name = settings.active_provider.to_lowercase();
    let api_key = settings.get_active_key();

    match provider_name.as_str() {
        "openai" => {
            let key =
                api_key.ok_or_else(|| anyhow!("Missing OPENAI_API_KEY environment variable"))?;
            let base_url = settings
                .get_active_base()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            Ok(Box::new(OpenAiProvider::new(key, base_url)))
        }
        "openrouter" => {
            let key = api_key
                .ok_or_else(|| anyhow!("Missing OPENROUTER_API_KEY environment variable"))?;
            let base_url = settings
                .get_active_base()
                .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
            Ok(Box::new(OpenAiProvider::new(key, base_url)))
        }
        "anthropic" => {
            let key =
                api_key.ok_or_else(|| anyhow!("Missing ANTHROPIC_API_KEY environment variable"))?;
            Ok(Box::new(AnthropicProvider::new(key)))
        }
        "gemini" => {
            let key =
                api_key.ok_or_else(|| anyhow!("Missing GEMINI_API_KEY environment variable"))?;
            Ok(Box::new(GeminiProvider::new(key)))
        }
        "mistral" => {
            let key =
                api_key.ok_or_else(|| anyhow!("Missing MISTRAL_API_KEY environment variable"))?;
            Ok(Box::new(MistralProvider::new(key)))
        }
        "ollama" => {
            let base_url = settings
                .get_active_base()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(Box::new(OllamaProvider::new(base_url)))
        }
        "cloudflare" => {
            let key = api_key
                .or_else(|| std::env::var("WORKERS_AI_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow!("Missing CLOUDFLARE_API_KEY or WORKERS_AI_API_KEY environment variable")
                })?;

            let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID")
                .or_else(|_| std::env::var("WORKERS_AI_ACCOUNT_ID"))
                .ok()
                .or_else(|| {
                    settings.get_active_base().and_then(|base| {
                        let parts: Vec<&str> = base.split("/accounts/").collect();
                        if parts.len() > 1 {
                            parts[1].split('/').next().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                });

            let account = account_id.ok_or_else(|| anyhow!(
                "Missing CLOUDFLARE_ACCOUNT_ID or WORKERS_AI_ACCOUNT_ID. Please set it in your .env file or configuration."
            ))?;

            let base_url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/ai/v1",
                account
            );
            Ok(Box::new(OpenAiProvider::new(key, base_url)))
        }
        _ => Err(anyhow!("Unsupported provider: {}", provider_name)),
    }
}
