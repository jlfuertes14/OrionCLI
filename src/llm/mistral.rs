use crate::llm::openai::OpenAiProvider;
use crate::llm::provider::{BoxedStream, ChatRequest, ChatResponse, LlmProvider};
use anyhow::Result;

pub struct MistralProvider {
    inner: OpenAiProvider,
}

impl MistralProvider {
    pub fn new(api_key: String) -> Self {
        MistralProvider {
            inner: OpenAiProvider::new(api_key, "https://api.mistral.ai/v1".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MistralProvider {
    async fn generate(&self, req: ChatRequest, model: &str) -> Result<ChatResponse> {
        self.inner.generate(req, model).await
    }

    async fn stream(&self, req: ChatRequest, model: &str) -> Result<BoxedStream> {
        self.inner.stream(req, model).await
    }

    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }
}
