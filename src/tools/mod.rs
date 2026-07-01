use std::collections::HashMap;
use serde_json::Value;
use anyhow::Result;
use crate::config::Settings;

pub mod filesystem;
pub mod terminal;
pub mod grep;
pub mod git;
pub mod browser;

pub struct ToolContext {
    pub settings: Settings,
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Tool identifier name (e.g. "read_file")
    fn name(&self) -> &str;

    /// Description explaining what the tool does to the LLM
    fn description(&self) -> &str;

    /// True if the tool requires explicit user confirmation before running
    fn requires_approval(&self) -> bool;

    /// OpenAI compatible schema for function parameters
    fn parameters_schema(&self) -> Value;

    /// Runs the tool with the parsed JSON arguments
    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = ToolRegistry {
            tools: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Filesystem
        self.register(filesystem::ReadFileTool);
        self.register(filesystem::WriteFileTool);
        self.register(filesystem::ListDirectoryTool);
        self.register(filesystem::MoveFileTool);
        self.register(filesystem::DeleteFileTool);

        // Terminal
        self.register(terminal::RunCommandTool);

        // Search
        self.register(grep::GrepSearchTool);

        // Git
        self.register(git::GitStatusTool);
        self.register(git::GitDiffTool);
        self.register(git::GitCommitTool);

        // Delegation
        self.register(crate::multi_agent::DelegateTaskTool);

        // Browser
        self.register(browser::BrowserReadTool);
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    /// List all tools
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<&dyn Tool> {
        self.tools.values().map(|b| b.as_ref()).collect()
    }

    /// Serializes registered tool definitions into OpenAI API tools array
    pub fn get_openai_schemas(&self) -> Vec<Value> {
        self.tools.values().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.parameters_schema(),
                }
            })
        }).collect()
    }
}
