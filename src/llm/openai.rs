use crate::llm::provider::{
    BoxedStream, ChatRequest, ChatResponse, FunctionCall, LlmProvider, Message, StreamChunk,
    ToolCall,
};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: String) -> Self {
        OpenAiProvider {
            client: Client::new(),
            api_key,
            base_url,
        }
    }
    fn is_cloudflare(&self) -> bool {
        self.base_url.contains("api.cloudflare.com")
    }

    fn map_messages(&self, messages: &[Message]) -> Vec<Value> {
        let is_cf = self.is_cloudflare();
        messages
            .iter()
            .map(|m| {
                let role = if is_cf {
                    match m.role.as_str() {
                        "system" => "user".to_string(),
                        "tool" => "user".to_string(),
                        other => other.to_string(),
                    }
                } else {
                    m.role.clone()
                };

                let content = if is_cf && m.role == "tool" {
                    format!(
                        "[TOOL RESULT: {}]\n{}",
                        m.name.as_deref().unwrap_or(""),
                        m.content
                    )
                } else {
                    m.content.clone()
                };

                let mut val = if let Some(ref imgs) = m.images {
                    let mut content_parts = vec![json!({
                        "type": "text",
                        "text": content,
                    })];
                    for img in imgs {
                        content_parts.push(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", img.media_type, img.data)
                            }
                        }));
                    }
                    json!({
                        "role": role,
                        "content": content_parts,
                    })
                } else {
                    let content_str = if is_cf {
                        let mut txt = content;
                        if let Some(ref tc) = m.tool_calls {
                            for call in tc {
                                txt.push_str(&format!(
                                    "\n\n[TOOL CALL: {}]\nArguments: {}",
                                    call.function.name, call.function.arguments
                                ));
                            }
                        }
                        txt
                    } else {
                        content
                    };

                    json!({
                        "role": role,
                        "content": content_str,
                    })
                };

                if !is_cf {
                    if let Some(ref tc) = m.tool_calls {
                        val["tool_calls"] = json!(tc);
                    }
                    if let Some(ref t_id) = m.tool_call_id {
                        val["tool_call_id"] = json!(t_id);
                    }
                    if let Some(ref name) = m.name {
                        val["name"] = json!(name);
                    }
                }
                val
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    async fn generate(&self, req: ChatRequest, model: &str) -> Result<ChatResponse> {
        let mapped = self.map_messages(&req.messages);
        let mut body = json!({
            "model": model,
            "messages": mapped,
        });

        if self.is_cloudflare() {
            body["max_tokens"] = json!(2048);
        } else {
            if let Some(tools) = req.tools {
                if !tools.is_empty() {
                    body["tools"] = json!(tools);
                    body["tool_choice"] = json!("auto");
                }
            }
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await?;
            return Err(anyhow!("OpenAI API error ({}): {}", status, err_txt));
        }

        let res_val: Value = resp.json().await?;
        let choice = &res_val["choices"][0]["message"];
        let role = choice["role"].as_str().unwrap_or("assistant").to_string();
        let content = choice["content"].as_str().map(|s| s.to_string());

        let mut tool_calls = None;
        if let Some(tc_array) = choice["tool_calls"].as_array() {
            let mut calls = Vec::new();
            for item in tc_array {
                calls.push(ToolCall {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    r#type: item["type"].as_str().unwrap_or("function").to_string(),
                    function: FunctionCall {
                        name: item["function"]["name"].as_str().unwrap_or("").to_string(),
                        arguments: item["function"]["arguments"]
                            .as_str()
                            .unwrap_or("{}")
                            .to_string(),
                    },
                });
            }
            if !calls.is_empty() {
                tool_calls = Some(calls);
            }
        }

        Ok(ChatResponse {
            role,
            content,
            tool_calls,
        })
    }

    async fn stream(&self, req: ChatRequest, model: &str) -> Result<BoxedStream> {
        let mapped = self.map_messages(&req.messages);
        let mut body = json!({
            "model": model,
            "messages": mapped,
            "stream": true,
        });

        if self.is_cloudflare() {
            body["max_tokens"] = json!(2048);
        } else {
            if let Some(tools) = req.tools {
                if !tools.is_empty() {
                    body["tools"] = json!(tools);
                    body["tool_choice"] = json!("auto");
                }
            }
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await?;
            return Err(anyhow!("OpenAI streaming error ({}): {}", status, err_txt));
        }

        let mut stream = resp.bytes_stream();
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut buffer = String::new();
            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(bytes) => {
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            buffer.push_str(text);
                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer[..line_end].trim().to_string();
                                buffer.drain(..=line_end);

                                if line.starts_with("data: ") {
                                    let data_str = &line[6..];
                                    if data_str == "[DONE]" {
                                        break;
                                    }
                                    if let Ok(parsed) = serde_json::from_str::<Value>(data_str) {
                                        if let Some(choice) =
                                            parsed["choices"].as_array().and_then(|a| a.first())
                                        {
                                            // Handle content delta
                                            if let Some(content) =
                                                choice["delta"]["content"].as_str()
                                            {
                                                let _ = tx
                                                    .send(Ok(StreamChunk::Content(
                                                        content.to_string(),
                                                    )))
                                                    .await;
                                            }
                                            // Handle tool call delta
                                            if let Some(tc_array) =
                                                choice["delta"]["tool_calls"].as_array()
                                            {
                                                for item in tc_array {
                                                    let index = item["index"].as_u64().unwrap_or(0)
                                                        as usize;
                                                    let id =
                                                        item["id"].as_str().map(|s| s.to_string());
                                                    let name = item["function"]["name"]
                                                        .as_str()
                                                        .map(|s| s.to_string());
                                                    let arguments = item["function"]["arguments"]
                                                        .as_str()
                                                        .map(|s| s.to_string());

                                                    let _ = tx
                                                        .send(Ok(StreamChunk::ToolCallChunk {
                                                            index,
                                                            id,
                                                            name,
                                                            arguments,
                                                        }))
                                                        .await;
                                                }
                                            }
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
