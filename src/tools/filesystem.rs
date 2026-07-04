use crate::sandbox::validate_path;
use crate::tools::{Tool, ToolContext};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::fs;

pub struct ReadFileTool;

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file to read."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let validated = validate_path(path_str, &ctx.settings)?;

        if !validated.exists() {
            return Ok(format!("File not found: {}", path_str));
        }
        if !validated.is_file() {
            return Ok(format!("Not a file: {}", path_str));
        }

        let content = fs::read_to_string(&validated)?;
        if content.len() > 50_000 {
            let truncated = &content[..50_000];
            Ok(format!(
                "{}\n\n... [truncated, file is {} chars total]",
                truncated,
                content.len()
            ))
        } else {
            Ok(content)
        }
    }
}

pub struct WriteFileTool;

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates parent directories if needed."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file."
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file."
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'content' argument"))?;
        let validated = validate_path(path_str, &ctx.settings)?;

        if let Some(parent) = validated.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&validated, content)?;

        Ok(format!(
            "Successfully wrote {} chars to {}",
            content.len(),
            path_str
        ))
    }
}

pub struct ListDirectoryTool;

#[async_trait::async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        "List all files and subdirectories in a directory."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let validated = validate_path(path_str, &ctx.settings)?;

        if !validated.exists() {
            return Ok(format!("Directory not found: {}", path_str));
        }
        if !validated.is_dir() {
            return Ok(format!("Not a directory: {}", path_str));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&validated)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let prefix = if path.is_dir() { "📁" } else { "📄" };
            let size = if path.is_file() {
                let bytes = entry.metadata()?.len();
                if bytes < 1024 {
                    format!(" ({} B)", bytes)
                } else if bytes < 1024 * 1024 {
                    format!(" ({:.1} KB)", bytes as f64 / 1024.0)
                } else {
                    format!(" ({:.1} MB)", bytes as f64 / (1024.0 * 1024.0))
                }
            } else {
                String::new()
            };
            entries.push(format!("{} {}{}", prefix, name, size));
        }

        if entries.is_empty() {
            Ok(format!("Directory is empty: {}", path_str))
        } else {
            entries.sort();
            Ok(entries.join("\n"))
        }
    }
}

pub struct MoveFileTool;

#[async_trait::async_trait]
impl Tool for MoveFileTool {
    fn name(&self) -> &str {
        "move_file"
    }

    fn description(&self) -> &str {
        "Move or rename a file or directory."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source_path": {
                    "type": "string",
                    "description": "Path to the source file or directory"
                },
                "destination_path": {
                    "type": "string",
                    "description": "Path to the destination file or directory"
                }
            },
            "required": ["source_path", "destination_path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let src_str = args["source_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'source_path' argument"))?;
        let dst_str = args["destination_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'destination_path' argument"))?;
        let src = validate_path(src_str, &ctx.settings)?;
        let dst = validate_path(dst_str, &ctx.settings)?;

        if !src.exists() {
            return Ok(format!("Source path not found: {}", src_str));
        }

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&src, &dst)?;

        Ok(format!("Successfully moved '{}' to '{}'", src_str, dst_str))
    }
}

pub struct DeleteFileTool;

#[async_trait::async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file or empty directory permanently. Always requires approval."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file or directory to delete"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let validated = validate_path(path_str, &ctx.settings)?;

        if !validated.exists() {
            return Ok(format!("Path not found: {}", path_str));
        }

        if validated.is_file() {
            fs::remove_file(&validated)?;
            Ok(format!("Successfully deleted file: {}", path_str))
        } else if validated.is_dir() {
            fs::remove_dir(&validated)?;
            Ok(format!(
                "Successfully deleted empty directory: {}",
                path_str
            ))
        } else {
            Err(anyhow!("Unsupported item type for deletion"))
        }
    }
}
