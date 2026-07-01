use std::process::Stdio;
use tokio::process::Command;
use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use crate::tools::{Tool, ToolContext};

pub struct GitStatusTool;

#[async_trait::async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Show the working tree status of the git repository."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<String> {
        let output = Command::new("git")
            .arg("status")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        let out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(out)
        } else {
            Ok(format!("Error checking git status:\n{}", err))
        }
    }
}

pub struct GitDiffTool;

#[async_trait::async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Show changes in files between the working directory and the commit index."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<String> {
        let output = Command::new("git")
            .arg("diff")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        let out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if out.is_empty() {
                Ok("No changes detected.".to_string())
            } else {
                Ok(out)
            }
        } else {
            Ok(format!("Error checking git diff:\n{}", err))
        }
    }
}

pub struct GitCommitTool;

#[async_trait::async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn description(&self) -> &str {
        "Commit staged changes to the git repository. Staged changed files are automatically committed. Always requires approval."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Commit message explaining what changes were committed."
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        let commit_msg = args["message"].as_str().ok_or_else(|| anyhow!("Missing 'message' argument"))?;

        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(commit_msg)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        let out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(format!("Git commit successful:\n{}", out))
        } else {
            Ok(format!("Error committing staged files:\n{}", err))
        }
    }
}
