use std::process::Stdio;
use tokio::process::Command;
use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use crate::tools::{Tool, ToolContext};

pub struct RunCommandTool;

#[async_trait::async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the local system. Always requires approval."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute (e.g. 'npm install', 'cargo run', 'dir')."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        let cmd_str = args["command"].as_str().ok_or_else(|| anyhow!("Missing 'command' argument"))?;

        // Detect OS shell
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("powershell", "-Command")
        } else {
            ("sh", "-c")
        };

        // Run the command
        let child = Command::new(shell)
            .arg(shell_arg)
            .arg(cmd_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Capture output
        let output = child.wait_with_output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&format!("--- STDOUT ---\n{}\n", stdout));
        }
        if !stderr.is_empty() {
            result.push_str(&format!("--- STDERR ---\n{}\n", stderr));
        }
        
        result.push_str(&format!("Exit Code: {}", output.status.code().unwrap_or(-1)));

        Ok(result)
    }
}
