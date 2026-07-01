use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use crate::sandbox::validate_path;
use crate::tools::{Tool, ToolContext};

pub struct GrepSearchTool;

#[async_trait::async_trait]
impl Tool for GrepSearchTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a text query inside all text files recursively in a directory. Ignores binary files and common build directories."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The text pattern to search for (case-insensitive)."
                },
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the directory to search in."
                }
            },
            "required": ["query", "path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let query = args["query"].as_str().ok_or_else(|| anyhow!("Missing 'query' argument"))?;
        let path_str = args["path"].as_str().ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        
        let validated = validate_path(path_str, &ctx.settings)?;
        if !validated.is_dir() {
            return Ok(format!("Not a directory: {}", path_str));
        }

        let mut matches = Vec::new();
        self.search_recursive(&validated, query, &mut matches)?;

        if matches.is_empty() {
            Ok(format!("No matches found for '{}' in {}", query, path_str))
        } else {
            if matches.len() >= 50 {
                matches.truncate(50);
                matches.push("... (results capped at 50)".to_string());
            }
            Ok(matches.join("\n"))
        }
    }
}

impl GrepSearchTool {
    fn search_recursive(&self, dir: &Path, query: &str, matches: &mut Vec<String>) -> Result<()> {
        let ignore_dirs = [".git", "node_modules", "build", "venv", ".venv", "target", ".dart_tool", "__pycache__"];
        let text_extensions = [
            ".py", ".js", ".ts", ".tsx", ".jsx", ".md", ".txt", ".json",
            ".yaml", ".yml", ".toml", ".cfg", ".ini", ".html", ".css",
            ".dart", ".java", ".kt", ".swift", ".rs", ".go", ".c", ".cpp",
            ".h", ".hpp", ".rb", ".php", ".sh", ".bat", ".ps1",
        ];

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if ignore_dirs.contains(&name.as_str()) {
                    continue;
                }

                if path.is_dir() {
                    self.search_recursive(&path, query, matches)?;
                } else if path.is_file() {
                    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    let suffix = format!(".{}", extension);
                    if text_extensions.iter().any(|&ext| ext == suffix || ext == extension) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let query_lower = query.to_lowercase();
                            for (line_num, line) in content.lines().enumerate() {
                                if line.to_lowercase().contains(&query_lower) {
                                    let rel_path = path.to_string_lossy();
                                    matches.push(format!("{}:{}: {}", rel_path, line_num + 1, line.trim()));
                                    if matches.len() >= 50 {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
