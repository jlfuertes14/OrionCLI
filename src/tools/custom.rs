use super::{Tool, ToolContext};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomToolDef {
    pub name: String,
    pub description: String,
    pub schema: Value,
    pub command: String,
    pub requires_approval: Option<bool>,
}

pub struct CustomScriptTool {
    def: CustomToolDef,
}

impl CustomScriptTool {
    pub fn new(def: CustomToolDef) -> Self {
        Self { def }
    }
}

#[async_trait::async_trait]
impl Tool for CustomScriptTool {
    fn name(&self) -> &str {
        &self.def.name
    }

    fn description(&self) -> &str {
        &self.def.description
    }

    fn requires_approval(&self) -> bool {
        self.def.requires_approval.unwrap_or(true)
    }

    fn parameters_schema(&self) -> Value {
        self.def.schema.clone()
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let args_json = serde_json::to_string(&args)?;

        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };
        let flag = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        let mut child = Command::new(shell)
            .arg(flag)
            .arg(&self.def.command)
            .current_dir(&ctx.settings.workspace_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn custom tool process")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(args_json.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let err = String::from_utf8_lossy(&output.stderr).into_owned();
            let out = String::from_utf8_lossy(&output.stdout).into_owned();
            anyhow::bail!(
                "Command failed with code {:?}\nStdout: {}\nStderr: {}",
                output.status.code(),
                out,
                err
            )
        }
    }
}
