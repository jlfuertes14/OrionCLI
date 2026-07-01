use std::pin::Pin;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;
use tokio_stream::Stream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub tools: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ChatResponse {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum StreamChunk {
    Content(String),
    ToolCallChunk {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    Error(String),
}

pub type BoxedStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait LlmProvider: Send + Sync {
    /// Non-streaming completion
    async fn generate(&self, req: ChatRequest, model: &str) -> Result<ChatResponse>;

    /// Streaming completion returning chunks of tokens or tool call deltas
    async fn stream(&self, req: ChatRequest, model: &str) -> Result<BoxedStream>;

    /// Indicates whether the provider has built-in tool calling support
    fn supports_tools(&self) -> bool;
}
