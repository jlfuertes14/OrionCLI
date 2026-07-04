use crate::llm::provider::{
    BoxedStream, ChatRequest, ChatResponse, FunctionCall, LlmProvider, Message, StreamChunk,
    ToolCall,
};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        AnthropicProvider {
            client: Client::new(),
            api_key,
        }
    }

    fn map_messages(&self, messages: &[Message]) -> (Option<String>, Vec<Value>) {
        let mut system = None;
        let mut mapped = Vec::new();

        for m in messages {
            if m.role == "system" {
                system = Some(m.content.clone());
                continue;
            }

            let mut val = json!({
                "role": m.role,
            });

            if m.role == "tool" {
                // Translate standard tool response back to Anthropic's block format
                val["role"] = json!("user");
                val["content"] = json!([{
                    "type": "tool_result",
                    "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                    "content": m.content.clone(),
                }]);
            } else if let Some(ref tc) = m.tool_calls {
                // Assistant message containing tool use requests
                let mut content_blocks = Vec::new();
                if !m.content.is_empty() {
                    content_blocks.push(json!({
                        "type": "text",
                        "text": m.content,
                    }));
                }
                for call in tc {
                    // Try parsing arguments as JSON value for Anthropic's input block
                    let input_val: Value =
                        serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
                    content_blocks.push(json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.function.name,
                        "input": input_val,
                    }));
                }
                val["content"] = json!(content_blocks);
            } else {
                if let Some(ref imgs) = m.images {
                    let mut content_blocks = vec![json!({
                        "type": "text",
                        "text": m.content,
                    })];
                    for img in imgs {
                        content_blocks.push(json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": img.media_type,
                                "data": img.data,
                            }
                        }));
                    }
                    val["content"] = json!(content_blocks);
                } else {
                    val["content"] = json!(m.content);
                }
            }

            mapped.push(val);
        }

        (system, mapped)
    }

    fn map_tools(&self, tools: &[Value]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                let func = &t["function"];
                json!({
                    "name": func["name"],
                    "description": func["description"],
                    "input_schema": func["parameters"],
                })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn generate(&self, req: ChatRequest, model: &str) -> Result<ChatResponse> {
        let (system, messages) = self.map_messages(&req.messages);
        let mut body = json!({
            "model": model,
            "max_tokens": 4096,
            "messages": messages,
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }

        if let Some(tools) = req.tools {
            if !tools.is_empty() {
                body["tools"] = json!(self.map_tools(&tools));
            }
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await?;
            return Err(anyhow!("Anthropic API error ({}): {}", status, err_txt));
        }

        let res_val: Value = resp.json().await?;
        let role = res_val["role"].as_str().unwrap_or("assistant").to_string();

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        if let Some(blocks) = res_val["content"].as_array() {
            for block in blocks {
                match block["type"].as_str().unwrap_or("") {
                    "text" => {
                        if let Some(txt) = block["text"].as_str() {
                            content.push_str(txt);
                        }
                    }
                    "tool_use" => {
                        let id = block["id"].as_str().unwrap_or("").to_string();
                        let name = block["name"].as_str().unwrap_or("").to_string();
                        let input = &block["input"];
                        let arguments =
                            serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string());
                        tool_calls.push(ToolCall {
                            id,
                            r#type: "function".to_string(),
                            function: FunctionCall { name, arguments },
                        });
                    }
                    _ => {}
                }
            }
        }

        let final_content = if content.is_empty() {
            None
        } else {
            Some(content)
        };
        let final_tool_calls = if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        };

        Ok(ChatResponse {
            role,
            content: final_content,
            tool_calls: final_tool_calls,
        })
    }

    async fn stream(&self, req: ChatRequest, model: &str) -> Result<BoxedStream> {
        let (system, messages) = self.map_messages(&req.messages);
        let mut body = json!({
            "model": model,
            "max_tokens": 4096,
            "messages": messages,
            "stream": true,
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }

        if let Some(tools) = req.tools {
            if !tools.is_empty() {
                body["tools"] = json!(self.map_tools(&tools));
            }
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await?;
            return Err(anyhow!(
                "Anthropic streaming error ({}): {}",
                status,
                err_txt
            ));
        }

        let mut stream = resp.bytes_stream();
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut event_name = String::new();

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(bytes) => {
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            buffer.push_str(text);
                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer[..line_end].trim().to_string();
                                buffer.drain(..=line_end);

                                if line.starts_with("event: ") {
                                    event_name = line[7..].to_string();
                                } else if line.starts_with("data: ") {
                                    let data_str = &line[6..];
                                    if let Ok(data) = serde_json::from_str::<Value>(data_str) {
                                        match event_name.as_str() {
                                            "content_block_delta" => {
                                                let index =
                                                    data["index"].as_u64().unwrap_or(0) as usize;
                                                let delta = &data["delta"];
                                                match delta["type"].as_str().unwrap_or("") {
                                                    "text_delta" => {
                                                        if let Some(txt) = delta["text"].as_str() {
                                                            let _ = tx
                                                                .send(Ok(StreamChunk::Content(
                                                                    txt.to_string(),
                                                                )))
                                                                .await;
                                                        }
                                                    }
                                                    "input_json_delta" => {
                                                        if let Some(json_txt) =
                                                            delta["partial_json"].as_str()
                                                        {
                                                            let _ = tx
                                                                .send(Ok(
                                                                    StreamChunk::ToolCallChunk {
                                                                        index,
                                                                        id: None,
                                                                        name: None,
                                                                        arguments: Some(
                                                                            json_txt.to_string(),
                                                                        ),
                                                                    },
                                                                ))
                                                                .await;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            "content_block_start" => {
                                                let index =
                                                    data["index"].as_u64().unwrap_or(0) as usize;
                                                let block = &data["content_block"];
                                                if block["type"].as_str().unwrap_or("")
                                                    == "tool_use"
                                                {
                                                    let id =
                                                        block["id"].as_str().map(|s| s.to_string());
                                                    let name = block["name"]
                                                        .as_str()
                                                        .map(|s| s.to_string());
                                                    let _ = tx
                                                        .send(Ok(StreamChunk::ToolCallChunk {
                                                            index,
                                                            id,
                                                            name,
                                                            arguments: None,
                                                        }))
                                                        .await;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow!(e))).await;
                        break;
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
