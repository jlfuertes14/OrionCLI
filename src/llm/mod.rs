pub mod provider;
pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod mistral;
pub mod ollama;

pub use provider::{LlmProvider, ChatRequest, Message, ToolCall, FunctionCall, StreamChunk};
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use mistral::MistralProvider;
pub use ollama::OllamaProvider;

use anyhow::{Result, anyhow};
use crate::config::Settings;

/// Instantiate the active LLM provider based on current Settings credentials
pub fn get_provider(settings: &Settings) -> Result<Box<dyn LlmProvider>> {
    let provider_name = settings.active_provider.to_lowercase();
    let api_key = settings.get_active_key();

    match provider_name.as_str() {
        "openai" => {
            let key = api_key.ok_or_else(|| anyhow!("Missing OPENAI_API_KEY environment variable"))?;
            let base_url = settings.get_active_base().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            Ok(Box::new(OpenAiProvider::new(key, base_url)))
        }
        "openrouter" => {
            let key = api_key.ok_or_else(|| anyhow!("Missing OPENROUTER_API_KEY environment variable"))?;
            let base_url = settings.get_active_base().unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
            Ok(Box::new(OpenAiProvider::new(key, base_url)))
        }
        "anthropic" => {
            let key = api_key.ok_or_else(|| anyhow!("Missing ANTHROPIC_API_KEY environment variable"))?;
            Ok(Box::new(AnthropicProvider::new(key)))
        }
        "gemini" => {
            let key = api_key.ok_or_else(|| anyhow!("Missing GEMINI_API_KEY environment variable"))?;
            Ok(Box::new(GeminiProvider::new(key)))
        }
        "mistral" => {
            let key = api_key.ok_or_else(|| anyhow!("Missing MISTRAL_API_KEY environment variable"))?;
            Ok(Box::new(MistralProvider::new(key)))
        }
        "ollama" => {
            let base_url = settings.get_active_base().unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(Box::new(OllamaProvider::new(base_url)))
        }
        _ => Err(anyhow!("Unsupported provider: {}", provider_name)),
    }
}
