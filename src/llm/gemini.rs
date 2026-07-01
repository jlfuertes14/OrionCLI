use reqwest::Client;
use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use crate::llm::provider::{LlmProvider, ChatRequest, ChatResponse, BoxedStream, StreamChunk, Message, ToolCall, FunctionCall};

pub struct GeminiProvider {
    client: Client,
    api_key: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        GeminiProvider {
            client: Client::new(),
            api_key,
        }
    }

    fn map_messages(&self, messages: &[Message]) -> (Option<String>, Vec<Value>) {
        let mut system = None;
        let mut contents = Vec::new();

        for m in messages {
            if m.role == "system" {
                system = Some(m.content.clone());
                continue;
            }

            // Gemini role must be either "user" or "model"
            let gemini_role = match m.role.as_str() {
                "assistant" => "model",
                "tool" => "user",
                _ => "user",
            };

            let mut parts = Vec::new();

            if m.role == "tool" {
                parts.push(json!({
                    "functionResponse": {
                        "name": m.name.clone().unwrap_or_default(),
                        "response": {
                            "output": m.content
                        }
                    }
                }));
            } else if let Some(ref tc) = m.tool_calls {
                if !m.content.is_empty() {
                    parts.push(json!({ "text": m.content }));
                }
                for call in tc {
                    let args_val: Value = serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
                    parts.push(json!({
                        "functionCall": {
                            "name": call.function.name,
                            "args": args_val,
                        }
                    }));
                }
            } else {
                parts.push(json!({ "text": m.content }));
            }

            contents.push(json!({
                "role": gemini_role,
                "parts": parts,
            }));
        }

        (system, contents)
    }

    fn map_tools(&self, tools: &[Value]) -> Value {
        let decls: Vec<Value> = tools.iter().map(|t| {
            let func = &t["function"];
            json!({
                "name": func["name"],
                "description": func["description"],
                "parameters": func["parameters"],
            })
        }).collect();

        json!([{ "functionDeclarations": decls }])
    }
}

#[async_trait::async_trait]
impl LlmProvider for GeminiProvider {
    async fn generate(&self, req: ChatRequest, model: &str) -> Result<ChatResponse> {
        let (system, contents) = self.map_messages(&req.messages);
        let mut body = json!({
            "contents": contents,
        });

        if let Some(sys) = system {
            body["systemInstruction"] = json!({
                "parts": [{ "text": sys }]
            });
        }

        if let Some(tools) = req.tools {
            if !tools.is_empty() {
                body["tools"] = self.map_tools(&tools);
            }
        }

        let model_clean = if model.contains('/') { model.to_string() } else { format!("models/{}", model) };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:generateContent?key={}",
            model_clean, self.api_key
        );

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await?;
            return Err(anyhow!("Gemini API error ({}): {}", status, err_txt));
        }

        let res_val: Value = resp.json().await?;
        let candidate = &res_val["candidates"][0]["content"];
        let role = candidate["role"].as_str().unwrap_or("model").to_string();

        let mut content = None;
        let mut tool_calls = Vec::new();

        if let Some(parts) = candidate["parts"].as_array() {
            for part in parts {
                if let Some(txt) = part["text"].as_str() {
                    content = Some(txt.to_string());
                } else if let Some(fc) = part["functionCall"].as_object() {
                    let name = fc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let default_args = json!({});
                    let args_val = fc.get("args").unwrap_or(&default_args);
                    let arguments = serde_json::to_string(args_val).unwrap_or_else(|_| "{}".to_string());
                    
                    tool_calls.push(ToolCall {
                        id: uuid::Uuid::new_v4().to_string(), // Gemini doesn't always supply a distinct ID; generate one
                        r#type: "function".to_string(),
                        function: FunctionCall { name, arguments },
                    });
                }
            }
        }

        let final_tool_calls = if tool_calls.is_empty() { None } else { Some(tool_calls) };

        Ok(ChatResponse {
            role: if role == "model" { "assistant".to_string() } else { role },
            content,
            tool_calls: final_tool_calls,
        })
    }

    async fn stream(&self, req: ChatRequest, model: &str) -> Result<BoxedStream> {
        let (system, contents) = self.map_messages(&req.messages);
        let mut body = json!({
            "contents": contents,
        });

        if let Some(sys) = system {
            body["systemInstruction"] = json!({
                "parts": [{ "text": sys }]
            });
        }

        if let Some(tools) = req.tools {
            if !tools.is_empty() {
                body["tools"] = self.map_tools(&tools);
            }
        }

        let model_clean = if model.contains('/') { model.to_string() } else { format!("models/{}", model) };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:streamGenerateContent?key={}",
            model_clean, self.api_key
        );

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await?;
            return Err(anyhow!("Gemini streaming API error ({}): {}", status, err_txt));
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
                            // Gemini returns streaming elements as a JSON Array structure.
                            // We can parse lines that start with SSE data indicator or try simple regex/brace balancing parsing.
                            // The stream format for beta is standard JSON array items:
                            // `[ { ... }, { ... } ]`
                            // A simple strategy is to extract braces or parse data blocks if it's SSE formatted.
                            // Let's do standard newline/brackets splitting to find candidate objects.
                            while let Some(line_end) = buffer.find('\n') {
                                let mut line = buffer[..line_end].trim().to_string();
                                buffer.drain(..=line_end);

                                if line.starts_with('[') {
                                    line = line[1..].to_string();
                                }
                                if line.ends_with(']') {
                                    line.pop();
                                }
                                if line.ends_with(',') {
                                    line.pop();
                                }
                                line = line.trim().to_string();

                                if !line.is_empty() {
                                    if let Ok(parsed) = serde_json::from_str::<Value>(&line) {
                                        if let Some(candidate) = parsed["candidates"].as_array().and_then(|a| a.first()) {
                                            if let Some(parts) = candidate["content"]["parts"].as_array() {
                                                for part in parts {
                                                    if let Some(txt) = part["text"].as_str() {
                                                        let _ = tx.send(Ok(StreamChunk::Content(txt.to_string()))).await;
                                                    }
                                                    if let Some(fc) = part["functionCall"].as_object() {
                                                        let name = fc.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                                                        let default_args = json!({});
                                                        let args_val = fc.get("args").unwrap_or(&default_args);
                                                        let args_str = serde_json::to_string(args_val).unwrap_or_default();
                                                        let _ = tx.send(Ok(StreamChunk::ToolCallChunk {
                                                            index: 0,
                                                            id: Some(uuid::Uuid::new_v4().to_string()),
                                                            name,
                                                            arguments: Some(args_str),
                                                        })).await;
                                                    }
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
