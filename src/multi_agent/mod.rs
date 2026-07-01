use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use crate::tools::{Tool, ToolContext};

pub const CODER_PROMPT: &str = r#"You are Orion Coder, a specialist software engineering sub-agent.
Your goal is to inspect codebases, write files, refactor scripts, and fix compile bugs.
Focus strictly on implementation correctness, code quality, and styling guidelines.
"#;

pub const RESEARCHER_PROMPT: &str = r#"You are Orion Researcher, a specialist information retrieval sub-agent.
Your goal is to lookup files, perform code searches, and retrieve documentation from the web.
Analyze details carefully and compile descriptive reports for other agents.
"#;

pub const REVIEWER_PROMPT: &str = r#"You are Orion Reviewer, a specialist code review sub-agent.
Your goal is to review code changes, look for errors, check safety boundaries, and perform linting checks.
Provide helpful critical critiques without writing full file revisions yourself unless requested.
"#;

pub struct DelegateTaskTool;

#[async_trait::async_trait]
impl Tool for DelegateTaskTool {
    fn name(&self) -> &str {
        "delegate_task"
    }

    fn description(&self) -> &str {
        "Spawn a specialist sub-agent to handle a specific task (e.g. coder, researcher, reviewer)."
    }

    fn requires_approval(&self) -> bool {
        false // Delegation itself is safe; the child agent's write operations will trigger their own approvals.
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "role": {
                    "type": "string",
                    "description": "The specialist role to delegate to ('coder', 'researcher', 'reviewer')."
                },
                "task": {
                    "type": "string",
                    "description": "The prompt instruction explaining what the sub-agent needs to accomplish."
                }
            },
            "required": ["role", "task"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let role = args["role"].as_str().ok_or_else(|| anyhow!("Missing 'role' argument"))?;
        let task = args["task"].as_str().ok_or_else(|| anyhow!("Missing 'task' argument"))?;

        // Resolve specialist prompt
        let sys_prompt = match role.to_lowercase().as_str() {
            "coder" => CODER_PROMPT,
            "researcher" => RESEARCHER_PROMPT,
            "reviewer" => REVIEWER_PROMPT,
            _ => return Ok(format!("Error: Unknown delegation role: {}", role)),
        };

        println!("\n[Orion: Spawning sub-agent '{}' for task: '{}']", role, task);

        // Build child settings with the same settings
        let child_settings = ctx.settings.clone();
        
        // Spawn sub-agent orchestrator
        // To construct a child with a custom prompt, we initialize it and rewrite its system prompt history
        let mut child_orch = crate::agent::AgentOrchestrator::new(child_settings);
        
        // Set child's specialized system prompt on index 0
        if let Some(msg) = child_orch.history_mut().first_mut() {
            msg.content = sys_prompt.to_string();
        }

        // Process message and return output
        // Redirect child stdout so it doesn't pollute the main terminal directly, or capture its tokens
        // For simplicity, we process it and let the child stream or run cleanly.
        // Let's run child orchestrator to execute the delegated task
        match child_orch.process_message_silent(task).await {
            Ok(summary) => {
                println!("[Orion: Sub-agent '{}' completed task successfully]\n", role);
                Ok(format!("Sub-agent '{}' completed task. Summary:\n{}", role, summary))
            }
            Err(e) => {
                println!("[Orion: Sub-agent '{}' failed task]\n", role);
                Ok(format!("Sub-agent '{}' task failed: {}", role, e))
            }
        }
    }
}
