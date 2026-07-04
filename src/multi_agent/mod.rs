use crate::tools::{Tool, ToolContext};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

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
        let role = args["role"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'role' argument"))?;
        let task = args["task"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'task' argument"))?;

        // Resolve specialist prompt
        let sys_prompt = match role.to_lowercase().as_str() {
            "coder" => CODER_PROMPT,
            "researcher" => RESEARCHER_PROMPT,
            "reviewer" => REVIEWER_PROMPT,
            _ => return Ok(format!("Error: Unknown delegation role: {}", role)),
        };

        println!(
            "\n[Orion: Spawning sub-agent '{}' for task: '{}']",
            role, task
        );

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
                println!(
                    "[Orion: Sub-agent '{}' completed task successfully]\n",
                    role
                );
                Ok(format!(
                    "Sub-agent '{}' completed task. Summary:\n{}",
                    role, summary
                ))
            }
            Err(e) => {
                println!("[Orion: Sub-agent '{}' failed task]\n", role);
                Ok(format!("Sub-agent '{}' task failed: {}", role, e))
            }
        }
    }
}

pub struct ParallelAgentsTool;

#[async_trait::async_trait]
impl Tool for ParallelAgentsTool {
    fn name(&self) -> &str {
        "parallel_agents"
    }

    fn description(&self) -> &str {
        "Spawn multiple sub-agents to handle independent tasks concurrently. All sub-agents run in parallel."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "delegations": {
                    "type": "array",
                    "description": "List of sub-agent tasks to run in parallel.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "role": {
                                "type": "string",
                                "description": "The specialist role ('coder', 'researcher', 'reviewer')."
                            },
                            "task": {
                                "type": "string",
                                "description": "The instructions for this sub-agent."
                            }
                        },
                        "required": ["role", "task"]
                    }
                }
            },
            "required": ["delegations"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let delegations_val = args["delegations"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing or invalid 'delegations' array"))?;

        let mut handles = Vec::new();

        for item in delegations_val {
            let role = item["role"].as_str().unwrap_or("coder").to_string();
            let task = item["task"].as_str().unwrap_or("").to_string();
            let parent_settings = ctx.settings.clone();

            let sys_prompt = match role.to_lowercase().as_str() {
                "coder" => CODER_PROMPT,
                "researcher" => RESEARCHER_PROMPT,
                "reviewer" => REVIEWER_PROMPT,
                _ => CODER_PROMPT,
            };

            println!(
                "[Orion: Spawning concurrent sub-agent '{}' for task: '{}']",
                role, task
            );

            // Spawn inside a tokio task
            let handle = tokio::spawn(async move {
                let mut child_orch = crate::agent::AgentOrchestrator::new(parent_settings);
                if let Some(msg) = child_orch.history_mut().first_mut() {
                    msg.content = sys_prompt.to_string();
                }
                match child_orch.process_message_silent(&task).await {
                    Ok(summary) => {
                        format!("Sub-agent '{}' completed task. Summary:\n{}", role, summary)
                    }
                    Err(e) => format!("Sub-agent '{}' task failed: {}", role, e),
                }
            });

            handles.push(handle);
        }

        // Wait for all sub-agents to complete concurrently
        let results = futures::future::join_all(handles).await;

        let mut report = Vec::new();
        for res in results {
            match res {
                Ok(summary) => report.push(summary),
                Err(e) => report.push(format!("Sub-agent task join error: {}", e)),
            }
        }

        println!("[Orion: All concurrent sub-agents completed execution]");
        Ok(report.join("\n\n---\n\n"))
    }
}
