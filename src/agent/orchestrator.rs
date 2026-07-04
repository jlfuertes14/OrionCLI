use crate::agent::middleware::{Middleware, PIIScrubber};
use crate::agent::prompts::SYSTEM_PROMPT;
use crate::cli::theme;
use crate::config::Settings;
use crate::llm::{self, ChatRequest, FunctionCall, Message, StreamChunk, ToolCall};
use crate::session::{PendingContext, SessionStore};
use crate::tools::{ToolContext, ToolRegistry};
use anyhow::{anyhow, Result};
use colored::Colorize;
use futures::StreamExt;
use serde_json::{json, Value};
use std::io::{self, Write};

pub struct AgentOrchestrator {
    settings: Settings,
    tool_registry: ToolRegistry,
    history: Vec<Message>,
    pub pending_images: Vec<crate::llm::provider::ImageContent>,
    pending_contexts: Vec<PendingContext>,
    middlewares: Vec<Box<dyn Middleware>>,
    session_store: Option<SessionStore>,
    session_id: Option<String>,
    session_cost_usd: f64,
}

impl Clone for AgentOrchestrator {
    fn clone(&self) -> Self {
        AgentOrchestrator {
            settings: self.settings.clone(),
            tool_registry: ToolRegistry::new(),
            history: self.history.clone(),
            pending_images: self.pending_images.clone(),
            pending_contexts: self.pending_contexts.clone(),
            middlewares: Vec::new(),
            session_store: None,
            session_id: self.session_id.clone(),
            session_cost_usd: self.session_cost_usd,
        }
    }
}

impl AgentOrchestrator {
    pub fn new(settings: Settings) -> Self {
        let mut history = Vec::new();
        let workspace_dir = settings.workspace_dir.to_string_lossy();
        history.push(Message {
            role: "system".to_string(),
            content: format!(
                "{}\n\nWORKSPACE CONTEXT:\n- The active workspace directory is: {}\n- Treat relative file paths and terminal commands as relative to this workspace.",
                SYSTEM_PROMPT,
                workspace_dir
            ),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            images: None,
        });

        AgentOrchestrator {
            settings,
            tool_registry: ToolRegistry::new(),
            history,
            pending_images: Vec::new(),
            pending_contexts: Vec::new(),
            middlewares: vec![Box::new(PIIScrubber)],
            session_store: None,
            session_id: None,
            session_cost_usd: 0.0,
        }
    }

    pub fn attach_session_store(&mut self, store: SessionStore, session_id: String) -> Result<()> {
        if store.get_session(&session_id)?.is_none() {
            return Err(anyhow!("Session not found: {}", session_id));
        }

        if let Some(messages) = store
            .load_messages(&session_id)
            .ok()
            .filter(|msgs| !msgs.is_empty())
        {
            self.history = messages;
        } else {
            for message in &self.history {
                store.append_message(&session_id, message)?;
            }
        }
        self.session_store = Some(store);
        self.session_id = Some(session_id);
        Ok(())
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn queue_context(&mut self, context: PendingContext) {
        self.pending_contexts.push(context);
    }

    /// Returns a mutable reference to the conversation history.
    pub fn history_mut(&mut self) -> &mut Vec<Message> {
        &mut self.history
    }

    fn push_history(&mut self, message: Message) -> Result<()> {
        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            store.append_message(session_id, &message)?;
        }
        self.history.push(message);
        Ok(())
    }

    fn build_user_content(&mut self, user_content: &str) -> String {
        if self.pending_contexts.is_empty() {
            return user_content.to_string();
        }

        let contexts = std::mem::take(&mut self.pending_contexts);
        let mut content = String::new();
        content.push_str(user_content);
        content.push_str("\n\n[ORION ATTACHED CONTEXT]\n");
        for context in contexts {
            content.push_str(&format!("\n## {}\n{}\n", context.label, context.content));
        }
        content
    }

    async fn stream_with_fallback(
        &self,
        req: ChatRequest,
    ) -> Result<(
        Box<dyn crate::llm::LlmProvider>,
        String,
        String,
        crate::llm::provider::BoxedStream,
    )> {
        let mut attempts = Vec::new();
        attempts.push((
            self.settings.active_provider.clone(),
            self.settings.active_model.clone(),
        ));
        for fallback in &self.settings.fallbacks {
            if let Some((provider, model)) = parse_provider_model(fallback) {
                attempts.push((provider, model));
            }
        }

        let mut last_err: Option<anyhow::Error> = None;
        for (idx, (provider_name, model)) in attempts.iter().enumerate() {
            let mut settings = self.settings.clone();
            settings.active_provider = provider_name.clone();
            settings.active_model = model.clone();
            let provider = match llm::get_provider(&settings) {
                Ok(provider) => provider,
                Err(err) => {
                    last_err = Some(err);
                    continue;
                }
            };

            match provider.stream(req.clone(), model).await {
                Ok(stream) => {
                    if idx > 0 {
                        theme::print_warning(&format!(
                            "Provider fallback activated: using {}:{}",
                            provider_name, model
                        ));
                    }
                    return Ok((provider, provider_name.clone(), model.clone(), stream));
                }
                Err(err) => {
                    let class = crate::llm::classify_provider_error(&err);
                    if !class.is_retryable() {
                        return Err(err);
                    }
                    last_err = Some(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("No provider attempts were available")))
    }

    fn record_usage(
        &mut self,
        input_text: &str,
        output_text: &str,
        provider: &str,
        model: &str,
    ) -> Result<()> {
        let input_tokens = estimate_tokens(input_text);
        let output_tokens = estimate_tokens(output_text);
        let cost = estimate_cost_usd(provider, model, input_tokens, output_tokens);
        self.session_cost_usd += cost;
        println!(
            "{} {} input / {} output tokens, ${:.4} response, ${:.4} session",
            "usage:".truecolor(107, 114, 128),
            input_tokens,
            output_tokens,
            cost,
            self.session_cost_usd
        );

        if let (Some(store), Some(id)) = (&self.session_store, &self.session_id) {
            let _ = store.update_session_stats(id, input_tokens, output_tokens, cost);
        }

        if let Some(max_budget) = self.settings.session.max_budget_usd {
            if self.session_cost_usd > max_budget {
                theme::print_warning(&format!(
                    "Session budget exceeded (${:.4}/${:.4}). Further model calls will be refused until you raise or clear the cap.",
                    self.session_cost_usd, max_budget
                ));
            }
        }
        Ok(())
    }

    fn ensure_budget_available(&self) -> Result<()> {
        if let Some(max_budget) = self.settings.session.max_budget_usd {
            if self.session_cost_usd >= max_budget {
                return Err(anyhow!(
                    "Session budget exhausted (${:.4}/${:.4}). Update session.max_budget_usd to continue.",
                    self.session_cost_usd,
                    max_budget
                ));
            }
        }
        Ok(())
    }

    pub fn load_skill(&mut self, skill: &crate::skills::Skill) {
        if let Some(system_msg) = self.history.first_mut() {
            if system_msg.role == "system" {
                system_msg.content.push_str("\n\n=== Skill Loaded: ");
                system_msg.content.push_str(&skill.skill.name);
                system_msg.content.push_str(" ===\n");
                system_msg.content.push_str(&skill.prompt.inject);
                system_msg.content.push_str("\n");
            }
        }
    }

    pub async fn initialize_mcp(&mut self) -> Result<()> {
        if let Some(ref servers) = self.settings.mcp_servers {
            for server in servers {
                match crate::mcp::McpClient::spawn(
                    server.name.clone(),
                    &server.command,
                    &server.args,
                )
                .await
                {
                    Ok(client) => {
                        let client_shared = std::sync::Arc::new(client);
                        match client_shared.list_tools().await {
                            Ok(tools) => {
                                for t_val in tools {
                                    if let Some(name) = t_val.get("name").and_then(|v| v.as_str()) {
                                        let desc = t_val
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let schema = t_val.get("inputSchema").cloned().unwrap_or(
                                            serde_json::json!({
                                                "type": "object",
                                                "properties": {}
                                            }),
                                        );

                                        let proxy = crate::mcp::McpToolProxy {
                                            client: client_shared.clone(),
                                            name: name.to_string(),
                                            desc,
                                            schema,
                                        };
                                        self.tool_registry.register(proxy);
                                        println!(
                                            "[MCP: Registered tool '{}' from server '{}']",
                                            name, server.name
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                println!(
                                    "[MCP: Failed to list tools from server '{}': {}]",
                                    server.name, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("[MCP: Failed to spawn server '{}': {}]", server.name, e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Process a user message through the loop silently (no stdout token printing) and returns the final text response.
    pub async fn process_message_silent(&mut self, user_content: &str) -> Result<String> {
        let images = if self.pending_images.is_empty() {
            None
        } else {
            let imgs = self.pending_images.clone();
            self.pending_images.clear();
            Some(imgs)
        };

        self.ensure_budget_available()?;

        let short_title = {
            let trimmed = user_content.trim();
            if trimmed.len() > 40 {
                let mut end_idx = 37;
                while !trimmed.is_char_boundary(end_idx) && end_idx > 0 {
                    end_idx -= 1;
                }
                format!("{}...", &trimmed[..end_idx])
            } else {
                trimmed.to_string()
            }
        };

        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            if let Ok(Some(meta)) = store.get_session(session_id) {
                if meta.title == "Interactive session" || meta.title == "New session" {
                    let _ = store.update_session_title(session_id, &short_title);
                }
            }
        }

        let user_content = self.build_user_content(user_content);

        self.push_history(Message {
            role: "user".to_string(),
            content: user_content.clone(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            images,
        })?;

        let tool_schemas = self.tool_registry.get_openai_schemas();
        let max_rounds = self.settings.session.max_tool_rounds;

        for _round in 0..max_rounds {
            let req = ChatRequest {
                messages: self.history.clone(),
                tools: Some(tool_schemas.clone()),
            };

            let (_provider, provider_name, model, mut stream) =
                self.stream_with_fallback(req).await?;
            let mut assistant_content = String::new();
            let mut pending_calls: std::collections::BTreeMap<usize, (String, String, String)> =
                std::collections::BTreeMap::new();

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(StreamChunk::Content(txt)) => {
                        assistant_content.push_str(&txt);
                    }
                    Ok(StreamChunk::ToolCallChunk {
                        index,
                        id,
                        name,
                        arguments,
                    }) => {
                        let entry = pending_calls.entry(index).or_insert((
                            String::new(),
                            String::new(),
                            String::new(),
                        ));
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

                self.push_history(Message {
                    role: "assistant".to_string(),
                    content: assistant_content,
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                    images: None,
                })?;

                for call in tool_calls {
                    let tool_name = &call.function.name;
                    let arguments: Value =
                        serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));
                    let tool = self
                        .tool_registry
                        .get(tool_name)
                        .ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?;

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

                    self.push_history(Message {
                        role: "tool".to_string(),
                        content: result_str,
                        tool_calls: None,
                        tool_call_id: Some(call.id.clone()),
                        name: Some(tool_name.clone()),
                        images: None,
                    })?;
                }
                continue;
            }

            self.record_usage(&user_content, &assistant_content, &provider_name, &model)?;
            self.push_history(Message {
                role: "assistant".to_string(),
                content: assistant_content.clone(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            })?;
            return Ok(assistant_content);
        }

        Err(anyhow!("Exceeded maximum orchestrator loop rounds"))
    }

    fn estimate_tokens(messages: &[Message]) -> usize {
        let mut total = 0;
        for msg in messages {
            total += msg.content.len() / 4;
            if let Some(calls) = &msg.tool_calls {
                for c in calls {
                    total += c.function.arguments.len() / 4;
                }
            }
            if let Some(imgs) = &msg.images {
                total += imgs.len() * 1000; // rough approximation for image cost
            }
        }
        total += messages.len() * 10;
        total
    }

    fn prune_history_if_needed(&mut self) {
        let max_tokens = 60_000; // Safe threshold for context windows

        while Self::estimate_tokens(&self.history) > max_tokens && self.history.len() > 3 {
            // Keep the system prompt at index 0. Remove the oldest message (index 1).
            self.history.remove(1);

            // If the new oldest message is a "tool" role (which means it's a dangling tool result), remove it too.
            // If the message before it had tool calls that were removed, the LLM will error if it sees a stray tool result.
            while self.history.len() > 1 && self.history[1].role == "tool" {
                self.history.remove(1);
            }
        }
    }

    /// Process a user message through the orchestrator tool execution loop.
    /// Streams tokens as they arrive, handles tool calls, prompts for approvals, and loops until final response.
    pub async fn process_message(&mut self, user_content: &str) -> Result<()> {
        let mut consecutive_tool_errors = 0;
        let images = if self.pending_images.is_empty() {
            None
        } else {
            let imgs = self.pending_images.clone();
            self.pending_images.clear();
            Some(imgs)
        };

        self.ensure_budget_available()?;

        let short_title = {
            let trimmed = user_content.trim();
            if trimmed.len() > 40 {
                let mut end_idx = 37;
                while !trimmed.is_char_boundary(end_idx) && end_idx > 0 {
                    end_idx -= 1;
                }
                format!("{}...", &trimmed[..end_idx])
            } else {
                trimmed.to_string()
            }
        };

        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            if let Ok(Some(meta)) = store.get_session(session_id) {
                if meta.title == "Interactive session" || meta.title == "New session" {
                    let _ = store.update_session_title(session_id, &short_title);
                }
            }
        }

        let user_content = self.build_user_content(user_content);

        self.push_history(Message {
            role: "user".to_string(),
            content: user_content.clone(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            images,
        })?;

        let tool_schemas = self.tool_registry.get_openai_schemas();
        let max_rounds = self.settings.session.max_tool_rounds;

        for _round in 0..max_rounds {
            self.prune_history_if_needed();

            let mut req = ChatRequest {
                messages: self.history.clone(),
                tools: Some(tool_schemas.clone()),
            };

            for middleware in &self.middlewares {
                if let Err(e) = middleware.process_request(&mut req) {
                    eprintln!("Middleware error: {}", e);
                }
            }

            if self.settings.trace_enabled {
                if let Some(mut trace_dir) = dirs::home_dir() {
                    trace_dir.push(".orion");
                    trace_dir.push("traces");
                    let _ = std::fs::create_dir_all(&trace_dir);
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let trace_file = trace_dir.join(format!("{}_request.json", timestamp));
                    if let Ok(json) = serde_json::to_string_pretty(&req) {
                        let _ = std::fs::write(trace_file, json);
                    }
                }
            }

            print!("{}", "Orion: ".bold().truecolor(33, 150, 243));
            io::stdout().flush()?;

            let (_provider, provider_name, model, mut stream) =
                self.stream_with_fallback(req).await?;
            let mut assistant_content = String::new();
            let mut line_buffer = String::new();
            let mut md_renderer = crate::cli::MarkdownRenderer::new();

            // Accumulator for tool calls streamed in chunks
            // index -> (id, name, arguments)
            let mut pending_calls: std::collections::BTreeMap<usize, (String, String, String)> =
                std::collections::BTreeMap::new();

            loop {
                let chunk_res = tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        println!();
                        theme::print_warning("Generation cancelled. Returning to prompt.");
                        return Ok(());
                    }
                    chunk = stream.next() => chunk,
                };
                let Some(chunk_res) = chunk_res else {
                    break;
                };
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
                    Ok(StreamChunk::ToolCallChunk {
                        index,
                        id,
                        name,
                        arguments,
                    }) => {
                        let entry = pending_calls.entry(index).or_insert((
                            String::new(),
                            String::new(),
                            String::new(),
                        ));
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
                self.push_history(Message {
                    role: "assistant".to_string(),
                    content: assistant_content,
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                    images: None,
                })?;

                // Execute all tool calls
                for call in tool_calls {
                    let tool_name = &call.function.name;
                    let arguments: Value =
                        serde_json::from_str(&call.function.arguments).unwrap_or(json!({}));

                    let tool = self
                        .tool_registry
                        .get(tool_name)
                        .ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?;

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
                            Ok(out) => {
                                consecutive_tool_errors = 0;
                                out
                            }
                            Err(e) => {
                                consecutive_tool_errors += 1;
                                format!("Error executing tool: {}", e)
                            }
                        }
                    } else {
                        consecutive_tool_errors += 1;
                        "Error: User denied tool execution.".to_string()
                    };

                    if consecutive_tool_errors >= 3 {
                        theme::print_warning("Circuit breaker triggered: Too many consecutive tool errors. Aborting loop.");
                        return Err(anyhow!("Infinite loop circuit breaker triggered: Too many consecutive tool errors."));
                    }

                    // Feed tool result back to the messages history
                    self.push_history(Message {
                        role: "tool".to_string(),
                        content: result_str,
                        tool_calls: None,
                        tool_call_id: Some(call.id.clone()),
                        name: Some(tool_name.clone()),
                        images: None,
                    })?;
                }

                // Continue to next round (let the LLM process the tool execution results)
                continue;
            }

            // No tool calls requested: final assistant reply finished
            self.record_usage(&user_content, &assistant_content, &provider_name, &model)?;
            self.push_history(Message {
                role: "assistant".to_string(),
                content: assistant_content,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            })?;
            break;
        }

        Ok(())
    }

    /// Prompt user on stdin for approval of a dangerous tool execution.
    fn prompt_user_approval(&self, name: &str, args: &Value) -> Result<bool> {
        println!(
            "{}",
            "=========================================================".yellow()
        );
        println!(
            "⚠️  Orion requests execution approval for tool: {}",
            name.bold().yellow()
        );
        println!("{}", self.format_approval_summary(name, args)?.cyan());
        println!(
            "{}",
            "=========================================================".yellow()
        );

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

    fn format_approval_summary(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "write_file" => {
                let path_str = args["path"].as_str().unwrap_or("<missing path>");
                let new_content = args["content"].as_str().unwrap_or("");

                let validated_path = crate::sandbox::validate_path(path_str, &self.settings)
                    .unwrap_or_else(|_| std::path::PathBuf::from(path_str));

                if validated_path.exists() && validated_path.is_file() {
                    let old_content = std::fs::read_to_string(&validated_path).unwrap_or_default();
                    let diff = crate::cli::theme::generate_color_diff(&old_content, new_content);
                    Ok(format!(
                        "Update file\n  Path: {}\n\nChanges:\n{}",
                        path_str,
                        indent_block(&diff, "  ")
                    ))
                } else {
                    let preview = preview_text(new_content, 8, 600);
                    Ok(format!(
                        "Create file\n  Path: {}\n  Size: {} chars\n\nPreview:\n{}",
                        path_str,
                        new_content.chars().count(),
                        indent_block(&preview, "  ")
                    ))
                }
            }
            "run_command" => {
                let command = args["command"].as_str().unwrap_or("<missing command>");
                let timeout = args["timeout_seconds"].as_u64().unwrap_or(300);
                let background = args["background"]
                    .as_bool()
                    .unwrap_or_else(|| is_likely_dev_server_command(command));
                Ok(format!(
                    "Run command\n  Workspace: {}\n  Command: {}\n  Timeout: {}s\n  Background: {}",
                    self.settings.workspace_dir.display(),
                    command,
                    timeout,
                    if background { "yes" } else { "no" }
                ))
            }
            "move_file" => Ok(format!(
                "Move file\n  From: {}\n  To: {}",
                args["source_path"].as_str().unwrap_or("<missing source>"),
                args["destination_path"]
                    .as_str()
                    .unwrap_or("<missing destination>")
            )),
            "delete_file" => Ok(format!(
                "Delete path\n  Path: {}",
                args["path"].as_str().unwrap_or("<missing path>")
            )),
            "git_commit" => Ok(format!(
                "Create git commit\n  Message: {}",
                args["message"].as_str().unwrap_or("<missing message>")
            )),
            _ => Ok(format!(
                "Arguments:\n{}",
                serde_json::to_string_pretty(args)?
            )),
        }
    }

    /// Update settings dynamically (e.g. after model / provider changes mid-session)
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
        if let (Some(store), Some(session_id)) = (&self.session_store, &self.session_id) {
            let _ = store.update_session_model(
                session_id,
                &self.settings.active_provider,
                &self.settings.active_model,
            );
        }
    }
}

fn parse_provider_model(value: &str) -> Option<(String, String)> {
    value
        .split_once(':')
        .map(|(provider, model)| (provider.trim().to_string(), model.trim().to_string()))
        .filter(|(provider, model)| !provider.is_empty() && !model.is_empty())
}

fn estimate_tokens(text: &str) -> usize {
    (text.chars().count().max(1) + 3) / 4
}

fn estimate_cost_usd(
    provider: &str,
    model: &str,
    input_tokens: usize,
    output_tokens: usize,
) -> f64 {
    let key = format!("{}:{}", provider.to_lowercase(), model.to_lowercase());
    let (input_per_m, output_per_m) = if key.contains("gpt-4o") {
        (2.50, 10.00)
    } else if key.contains("claude-3.5") || key.contains("sonnet") {
        (3.00, 15.00)
    } else if key.contains("gemini") {
        (0.35, 1.05)
    } else {
        (0.0, 0.0)
    };
    (input_tokens as f64 / 1_000_000.0 * input_per_m)
        + (output_tokens as f64 / 1_000_000.0 * output_per_m)
}

fn is_likely_dev_server_command(command: &str) -> bool {
    let normalized = command.to_lowercase();
    [
        "npm run dev",
        "npm start",
        "pnpm dev",
        "pnpm start",
        "yarn dev",
        "yarn start",
        "vite --host",
        "vite --open",
        "next dev",
        "astro dev",
        "remix dev",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn preview_text(text: &str, max_lines: usize, max_chars: usize) -> String {
    let mut preview = String::new();
    let mut line_count = 0;

    for line in text.lines() {
        if line_count >= max_lines || preview.chars().count() >= max_chars {
            break;
        }

        if !preview.is_empty() {
            preview.push('\n');
        }

        let remaining = max_chars.saturating_sub(preview.chars().count());
        let clipped: String = line.chars().take(remaining).collect();
        preview.push_str(&clipped);
        line_count += 1;
    }

    if text.lines().count() > line_count || text.chars().count() > preview.chars().count() {
        preview.push_str("\n...");
    }

    preview
}

fn indent_block(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            images: None,
        }];
        // 5 chars / 4 = 1 token + 10 base tokens = 11 tokens
        assert_eq!(AgentOrchestrator::estimate_tokens(&messages), 11);
    }

    #[test]
    fn test_prune_history_if_needed() {
        let settings = Settings::default();
        let mut orchestrator = AgentOrchestrator::new(settings);

        // Let's manually populate history
        orchestrator.history = vec![
            Message {
                role: "system".to_string(),
                content: "System Prompt".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            },
            Message {
                role: "user".to_string(),
                // Generate a very long message. max_tokens is 60k.
                // Let's make it have ~280k characters so token count estimation is ~70k.
                content: "a".repeat(280_000),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            },
            Message {
                role: "assistant".to_string(),
                content: "Sure".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            },
            Message {
                role: "tool".to_string(),
                content: "Tool result".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            },
            Message {
                role: "user".to_string(),
                content: "Final User Message".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                images: None,
            },
        ];

        // Pruning should remove the massive user message (index 1)
        // And then because index 2 becomes index 1, and the one after is "tool",
        // it might keep going unless we reach safe limits.
        // Let's verify pruning triggers.
        orchestrator.prune_history_if_needed();

        // System prompt (index 0) must be preserved.
        assert_eq!(orchestrator.history[0].role, "system");

        // Total tokens should now be below 60_000
        assert!(AgentOrchestrator::estimate_tokens(&orchestrator.history) <= 60_000);
    }
}
