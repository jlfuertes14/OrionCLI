use crate::tools::{Tool, ToolContext};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;

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

    async fn execute(&self, _args: Value, ctx: &ToolContext) -> Result<String> {
        let output = Command::new("git")
            .arg("status")
            .current_dir(&ctx.settings.workspace_dir)
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

    async fn execute(&self, _args: Value, ctx: &ToolContext) -> Result<String> {
        let output = Command::new("git")
            .arg("diff")
            .current_dir(&ctx.settings.workspace_dir)
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let commit_msg = args["message"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'message' argument"))?;

        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(commit_msg)
            .current_dir(&ctx.settings.workspace_dir)
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

pub struct GitExecuteTool;

#[async_trait::async_trait]
impl Tool for GitExecuteTool {
    fn name(&self) -> &str {
        "git_execute"
    }

    fn description(&self) -> &str {
        "Execute arbitrary Git commands (e.g., init, push, pull, branch, rebase). Always requires user approval."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "subcommand": {
                    "type": "string",
                    "description": "The git subcommand to execute (e.g., 'init', 'push', 'branch')."
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of arguments and flags for the subcommand (e.g., ['origin', 'main'], ['-b', 'feature'])."
                }
            },
            "required": ["subcommand", "args"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let subcommand = args["subcommand"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'subcommand' argument"))?;

        let mut cmd_args = Vec::new();
        if let Some(args_arr) = args["args"].as_array() {
            for arg in args_arr {
                if let Some(arg_str) = arg.as_str() {
                    cmd_args.push(arg_str.to_string());
                }
            }
        }

        let output = Command::new("git")
            .arg(subcommand)
            .args(&cmd_args)
            .current_dir(&ctx.settings.workspace_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        let out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(format!("Git command successful:\n{}", out))
        } else {
            Ok(format!("Error executing git command:\n{}\n{}", err, out))
        }
    }
}
