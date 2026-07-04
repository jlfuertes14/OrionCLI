use crate::tools::{Tool, ToolContext};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

pub struct McpClient {
    tx: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    next_id: Arc<Mutex<i64>>,
    #[allow(dead_code)]
    pub name: String,
}

impl McpClient {
    pub async fn spawn(name: String, command: &str, args: &[String]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdout"))?;

        let pending = Arc::new(Mutex::new(HashMap::<i64, oneshot::Sender<Value>>::new()));
        let pending_clone = pending.clone();

        let (tx, mut rx) = mpsc::channel::<String>(32);
        tokio::spawn(async move {
            let mut writer = stdin;
            while let Some(line) = rx.recv().await {
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = writer.flush().await;
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(response) = serde_json::from_str::<Value>(&line) {
                    if let Some(id_val) = response.get("id").and_then(|v| v.as_i64()) {
                        let mut guard = pending_clone.lock().unwrap();
                        if let Some(tx_oneshot) = guard.remove(&id_val) {
                            let _ = tx_oneshot.send(response);
                        }
                    }
                }
            }
        });

        let client = McpClient {
            tx,
            pending,
            next_id: Arc::new(Mutex::new(1)),
            name,
        };

        client.initialize().await?;
        Ok(client)
    }

    async fn call_rpc(&self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut guard = self.next_id.lock().unwrap();
            let cur = *guard;
            *guard += 1;
            cur
        };

        let (tx_oneshot, rx_oneshot) = oneshot::channel();
        {
            let mut guard = self.pending.lock().unwrap();
            guard.insert(id, tx_oneshot);
        }

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let request_line = serde_json::to_string(&request)?;
        self.tx
            .send(request_line)
            .await
            .map_err(|_| anyhow!("Failed to send request to writer"))?;

        let response = rx_oneshot
            .await
            .map_err(|_| anyhow!("Reader task dropped channel"))?;
        if let Some(error) = response.get("error") {
            return Err(anyhow!("MCP error: {}", error));
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn initialize(&self) -> Result<()> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "OrionBot-CLI",
                "version": "0.1.0"
            }
        });
        self.call_rpc("initialize", params).await?;

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let notification_line = serde_json::to_string(&notification)?;
        let _ = self.tx.send(notification_line).await;

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<Value>> {
        let result = self.call_rpc("tools/list", json!({})).await?;
        if let Some(tools) = result.get("tools").and_then(|v| v.as_array()) {
            Ok(tools.clone())
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String> {
        let result = self
            .call_rpc(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": arguments
                }),
            )
            .await?;

        if let Some(content_array) = result.get("content").and_then(|v| v.as_array()) {
            let mut output = Vec::new();
            for item in content_array {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        output.push(text.to_string());
                    }
                }
            }
            Ok(output.join("\n"))
        } else {
            Err(anyhow!(
                "Invalid response content structure from MCP server"
            ))
        }
    }
}

pub struct McpToolProxy {
    pub client: Arc<McpClient>,
    pub name: String,
    pub desc: String,
    pub schema: Value,
}

#[async_trait::async_trait]
impl Tool for McpToolProxy {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.desc
    }

    fn requires_approval(&self) -> bool {
        true // always require approval for external MCP server executions for safety
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        self.client.call_tool(&self.name, args).await
    }
}
