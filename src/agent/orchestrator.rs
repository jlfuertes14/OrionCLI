use std::io::{self, Write};
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use colored::Colorize;
use futures::StreamExt;
use crate::config::Settings;
use crate::llm::{self, ChatRequest, Message, ToolCall, FunctionCall, StreamChunk};
use crate::tools::{ToolRegistry, ToolContext};
use crate::agent::prompts::SYSTEM_PROMPT;
use crate::cli::theme;

pub struct AgentOrchestrator {
    settings: Settings,
    tool_registry: ToolRegistry,
    history: Vec<Message>,
}

impl AgentOrchestrator {
    pub fn new(settings: Settings) -> Self {
        let mut history = Vec::new();
        history.push(Message {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        AgentOrchestrator {
            settings,
            tool_registry: ToolRegistry::new(),
            history,
        }
    }

    /// Returns a mutable reference to the conversation history.
    pub fn history_mut(&mut self) -> &mut Vec<Message> {
        &mut self.history
    }

    /// Process a user message through the loop silently (no stdout token printing) and returns the final text response.
    pub async fn process_message_silent(&mut self, user_content: &str) -> Result<String> {
        self.history.push(Message {
            role: "user".to_string(),
            content: user_content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        let provider = llm::get_provider(&self.settings)?;
        let tool_schemas = self.tool_registry.get_openai_schemas();
        let max_rounds = self.settings.session.max_tool_rounds;

        for _round in 0..max_rounds {
            let req = ChatRequest {
                messages: self.history.clone(),
                tools: Some(tool_schemas.clone()),
            };

            let mut stream = provider.stream(req, &self.settings.active_model).await?;
            let mut assistant_content = String::new();
            let mut pending_calls: std::collections::BTreeMap<usize, (String, String, String)> = std::collections::BTreeMap::new();

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(StreamChunk::Content(txt)) => {
                        assistant_content.push_str(&txt);
                    }
                    Ok(StreamChunk::ToolCallChunk { index, id, name, arguments }) => {
                        let entry = pending_calls.entry(index).or_insert((String::new(), String::new(), String::new()));
                        if let Some(ref val_id) = id {
                            entry.0.push_str(val_id);
                        }
                        if let Some(ref val_name) = name {
                            entry.1.push_str(val_name);
                        }
                        if let Some(ref val_args) = arguments {
                            entry.2.push_str(val_args);
                        }
                    }
                    Ok(StreamChunk::Error(err)) => {
                        return Err(anyhow!("Stream chunk error: {}", err));
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            if !pending_calls.is_empty() {
                let mut tool_calls = Vec::new();
                for (_idx, (id, name, args)) in pending_calls {
                    tool_calls.push(ToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name,
                            arguments: args,
                        },
                    });
                }

                self.history.push(Message {
                    role: "assistant".to_string(),
                    content: assistant_content,
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                });

                for call in tool_calls {
                    let tool_name = &call.function.name;
                    let arguments: Value = serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
                    let tool = self.tool_registry.get(tool_name).ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?;

                    let approved = if tool.requires_approval() {
                        self.prompt_user_approval(tool_name, &arguments)?
                    } else {
                        true
                    };

                    let result_str = if approved {
                        theme::print_info(&format!("Running tool '{}'...", tool_name));
                        let ctx = ToolContext {
                            settings: self.settings.clone(),
                        };
                        match tool.execute(arguments, &ctx).await {
                            Ok(out) => out,
                            Err(e) => format!("Error executing tool: {}", e),
                        }
                    } else {
                        "Error: User denied tool execution.".to_string()
                    };

                    self.history.push(Message {
                        role: "tool".to_string(),
                        content: result_str,
                        tool_calls: None,
                        tool_call_id: Some(call.id.clone()),
                        name: Some(tool_name.clone()),
                    });
                }
                continue;
            }

            self.history.push(Message {
                role: "assistant".to_string(),
                content: assistant_content.clone(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });
            return Ok(assistant_content);
        }

        Err(anyhow!("Exceeded maximum orchestrator loop rounds"))
    }

    /// Process a user message through the orchestrator tool execution loop.
    /// Streams tokens as they arrive, handles tool calls, prompts for approvals, and loops until final response.
    pub async fn process_message(&mut self, user_content: &str) -> Result<()> {
        self.history.push(Message {
            role: "user".to_string(),
            content: user_content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        let provider = llm::get_provider(&self.settings)?;
        let tool_schemas = self.tool_registry.get_openai_schemas();
        let max_rounds = self.settings.session.max_tool_rounds;

        for _round in 0..max_rounds {
            let req = ChatRequest {
                messages: self.history.clone(),
                tools: Some(tool_schemas.clone()),
            };

            print!("{}", "Orion: ".bold().truecolor(33, 150, 243));
            io::stdout().flush()?;

            let mut stream = provider.stream(req, &self.settings.active_model).await?;
            let mut assistant_content = String::new();
            let mut line_buffer = String::new();
            let mut md_renderer = crate::cli::MarkdownRenderer::new();
            
            // Accumulator for tool calls streamed in chunks
            // index -> (id, name, arguments)
            let mut pending_calls: std::collections::BTreeMap<usize, (String, String, String)> = std::collections::BTreeMap::new();

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(StreamChunk::Content(txt)) => {
                        assistant_content.push_str(&txt);
                        line_buffer.push_str(&txt);
                        
                        while let Some(idx) = line_buffer.find('\n') {
                            let line = line_buffer[..idx].to_string();
                            line_buffer.drain(..=idx);
                            let rendered = md_renderer.render_line(&line);
                            print!("{}", rendered);
                            io::stdout().flush()?;
                        }
                    }
                    Ok(StreamChunk::ToolCallChunk { index, id, name, arguments }) => {
                        let entry = pending_calls.entry(index).or_insert((String::new(), String::new(), String::new()));
                        if let Some(ref val_id) = id {
                            entry.0.push_str(val_id);
                        }
                        if let Some(ref val_name) = name {
                            entry.1.push_str(val_name);
                        }
                        if let Some(ref val_args) = arguments {
                            entry.2.push_str(val_args);
                        }
                    }
                    Ok(StreamChunk::Error(err)) => {
                        println!();
                        theme::print_error(&format!("Stream error: {}", err));
                        return Err(anyhow!("Stream chunk error: {}", err));
                    }
                    Err(e) => {
                        println!();
                        theme::print_error(&format!("Connection error: {}", e));
                        return Err(e);
                    }
                }
            }
            if !line_buffer.is_empty() {
                let rendered = md_renderer.render_line(&line_buffer);
                print!("{}", rendered);
                io::stdout().flush()?;
            }
            println!();

            // Check if tool calls were requested by LLM
            if !pending_calls.is_empty() {
                let mut tool_calls = Vec::new();
                for (_idx, (id, name, args)) in pending_calls {
                    tool_calls.push(ToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name,
                            arguments: args,
                        },
                    });
                }

                // Add assistant tool calls to message history
                self.history.push(Message {
                    role: "assistant".to_string(),
                    content: assistant_content,
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                });

                // Execute all tool calls
                for call in tool_calls {
                    let tool_name = &call.function.name;
                    let arguments: Value = serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));

                    let tool = self.tool_registry.get(tool_name).ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?;

                    // Handle approval check
                    let approved = if tool.requires_approval() {
                        self.prompt_user_approval(tool_name, &arguments)?
                    } else {
                        true
                    };

                    let result_str = if approved {
                        theme::print_info(&format!("Running tool '{}'...", tool_name));
                        let ctx = ToolContext {
                            settings: self.settings.clone(),
                        };
                        match tool.execute(arguments, &ctx).await {
                            Ok(out) => out,
                            Err(e) => format!("Error executing tool: {}", e),
                        }
                    } else {
                        "Error: User denied tool execution.".to_string()
                    };

                    // Feed tool result back to the messages history
                    self.history.push(Message {
                        role: "tool".to_string(),
                        content: result_str,
                        tool_calls: None,
                        tool_call_id: Some(call.id.clone()),
                        name: Some(tool_name.clone()),
                    });
                }

                // Continue to next round (let the LLM process the tool execution results)
                continue;
            }

            // No tool calls requested: final assistant reply finished
            self.history.push(Message {
                role: "assistant".to_string(),
                content: assistant_content,
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });
            break;
        }

        Ok(())
    }

    /// Prompt user on stdin for approval of a dangerous tool execution.
    fn prompt_user_approval(&self, name: &str, args: &Value) -> Result<bool> {
        println!("{}", "=========================================================".yellow());
        println!("⚠️  Orion requests execution approval for tool: {}", name.bold().yellow());
        println!("Arguments: {}", serde_json::to_string_pretty(args)?.cyan());
        println!("{}", "=========================================================".yellow());
        
        loop {
            print!("Approve execution? [y/N]: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let trimmed = input.trim().to_lowercase();
            
            if trimmed == "y" || trimmed == "yes" {
                return Ok(true);
            } else if trimmed == "n" || trimmed == "no" || trimmed.is_empty() {
                return Ok(false);
            } else {
                println!("Please type 'y' or 'n'.");
            }
        }
    }

    /// Update settings dynamically (e.g. after model / provider changes mid-session)
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }
}
