use crate::cli::command_picker;
use crate::cli::theme::{self, format_user_prompt};
use crate::config::Settings;
use anyhow::Result;
use colored::Colorize;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use std::path::PathBuf;

pub struct Repl {
    settings: Settings,
    resume_session_id: Option<String>,
    history_path: Option<PathBuf>,
    skill_registry: crate::skills::SkillRegistry,
}

impl Repl {
    pub fn new(settings: Settings, resume_session_id: Option<String>) -> Self {
        let history_path = dirs::home_dir().map(|h| h.join(".orion").join("history.txt"));
        Repl {
            settings,
            resume_session_id,
            history_path,
            skill_registry: crate::skills::SkillRegistry::load_defaults(),
        }
    }

    async fn read_line_custom(&mut self, prompt: &str) -> Result<Option<String>> {
        use crossterm::style::Print;
        use crossterm::terminal::{self, ClearType};
        use crossterm::{event, execute};
        use std::io::{self, Write};

        let mut line = String::new();
        let mut history_list = Vec::new();

        if let Some(ref path) = self.history_path {
            if let Ok(content) = std::fs::read_to_string(path) {
                history_list = content.lines().map(|s| s.to_string()).collect();
            }
        }
        let mut history_index = history_list.len();

        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();

        execute!(stdout, Print(prompt))?;
        stdout.flush()?;

        let result = loop {
            if let Event::Key(key) = event::read()? {
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Enter, _) => {
                        execute!(stdout, Print("\r\n"))?;
                        stdout.flush()?;
                        break Some(line);
                    }
                    (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        if line.is_empty() {
                            execute!(stdout, Print("\r\n"))?;
                            stdout.flush()?;
                            break None;
                        } else {
                            line.clear();
                            execute!(
                                stdout,
                                Print("\r"),
                                terminal::Clear(ClearType::CurrentLine),
                                Print(prompt)
                            )?;
                            stdout.flush()?;
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        if !line.is_empty() {
                            line.pop();
                            execute!(
                                stdout,
                                Print("\r"),
                                terminal::Clear(ClearType::CurrentLine),
                                Print(prompt),
                                Print(&line)
                            )?;
                            stdout.flush()?;
                        }
                    }
                    (KeyCode::Up, _) => {
                        if history_index > 0 {
                            history_index -= 1;
                            if let Some(hist) = history_list.get(history_index) {
                                line = hist.clone();
                                execute!(
                                    stdout,
                                    Print("\r"),
                                    terminal::Clear(ClearType::CurrentLine),
                                    Print(prompt),
                                    Print(&line)
                                )?;
                                stdout.flush()?;
                            }
                        }
                    }
                    (KeyCode::Down, _) => {
                        if history_index + 1 < history_list.len() {
                            history_index += 1;
                            if let Some(hist) = history_list.get(history_index) {
                                line = hist.clone();
                            }
                        } else {
                            history_index = history_list.len();
                            line.clear();
                        }
                        execute!(
                            stdout,
                            Print("\r"),
                            terminal::Clear(ClearType::CurrentLine),
                            Print(prompt),
                            Print(&line)
                        )?;
                        stdout.flush()?;
                    }
                    (KeyCode::Char(c), _) => {
                        if line.is_empty() && c == '/' {
                            terminal::disable_raw_mode()?;
                            println!();
                            if let Some(selected) = command_picker::run_picker()? {
                                if selected == "/skill" {
                                    let _ = self.skill_registry.scan_skills_dir();
                                    if let Some(skill_selected) =
                                        command_picker::run_skills_picker(&mut self.skill_registry)?
                                    {
                                        terminal::enable_raw_mode()?;
                                        line = format!("/skill load {}", skill_selected);
                                        execute!(
                                            stdout,
                                            Print("\r"),
                                            terminal::Clear(ClearType::CurrentLine),
                                            Print(prompt),
                                            Print(&line),
                                            Print("\r\n")
                                        )?;
                                        stdout.flush()?;
                                        break Some(line);
                                    } else {
                                        terminal::enable_raw_mode()?;
                                        line.clear();
                                        execute!(
                                            stdout,
                                            Print("\r"),
                                            terminal::Clear(ClearType::CurrentLine),
                                            Print(prompt)
                                        )?;
                                        stdout.flush()?;
                                    }
                                } else {
                                    terminal::enable_raw_mode()?;
                                    let needs_args = ["/search", "/web", "/vision", "/model"]
                                        .contains(&selected.as_str());

                                    if needs_args {
                                        line = format!("{} ", selected);
                                        execute!(
                                            stdout,
                                            Print("\r"),
                                            terminal::Clear(ClearType::CurrentLine),
                                            Print(prompt),
                                            Print(&line)
                                        )?;
                                        stdout.flush()?;
                                    } else {
                                        execute!(
                                            stdout,
                                            Print("\r"),
                                            terminal::Clear(ClearType::CurrentLine),
                                            Print(prompt),
                                            Print(&selected),
                                            Print("\r\n")
                                        )?;
                                        stdout.flush()?;
                                        break Some(selected);
                                    }
                                }
                            } else {
                                terminal::enable_raw_mode()?;
                                execute!(
                                    stdout,
                                    Print("\r"),
                                    terminal::Clear(ClearType::CurrentLine),
                                    Print(prompt)
                                )?;
                                stdout.flush()?;
                            }
                        } else if line.is_empty() && c == '.' {
                            terminal::disable_raw_mode()?;
                            println!();
                            let _ = self.skill_registry.scan_skills_dir();
                            if let Some(selected) =
                                command_picker::run_skills_picker(&mut self.skill_registry)?
                            {
                                terminal::enable_raw_mode()?;
                                line = format!("/skill load {}", selected);
                                execute!(
                                    stdout,
                                    Print("\r"),
                                    terminal::Clear(ClearType::CurrentLine),
                                    Print(prompt),
                                    Print(&line),
                                    Print("\r\n")
                                )?;
                                stdout.flush()?;
                                break Some(line);
                            } else {
                                terminal::enable_raw_mode()?;
                                execute!(
                                    stdout,
                                    Print("\r"),
                                    terminal::Clear(ClearType::CurrentLine),
                                    Print(prompt)
                                )?;
                                stdout.flush()?;
                            }
                        } else {
                            line.push(c);
                            if line == "/skill load " {
                                terminal::disable_raw_mode()?;
                                println!();
                                let _ = self.skill_registry.scan_skills_dir();
                                if let Some(selected) =
                                    command_picker::run_skills_picker(&mut self.skill_registry)?
                                {
                                    terminal::enable_raw_mode()?;
                                    line = format!("/skill load {}", selected);
                                    execute!(
                                        stdout,
                                        Print("\r"),
                                        terminal::Clear(ClearType::CurrentLine),
                                        Print(prompt),
                                        Print(&line),
                                        Print("\r\n")
                                    )?;
                                    stdout.flush()?;
                                    break Some(line);
                                } else {
                                    terminal::enable_raw_mode()?;
                                    execute!(
                                        stdout,
                                        Print("\r"),
                                        terminal::Clear(ClearType::CurrentLine),
                                        Print(prompt),
                                        Print(&line)
                                    )?;
                                    stdout.flush()?;
                                }
                            } else {
                                execute!(stdout, Print(c))?;
                                stdout.flush()?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        };

        terminal::disable_raw_mode()?;

        if let Some(ref text) = result {
            if !text.trim().is_empty() {
                if let Some(ref path) = self.history_path {
                    let mut history_entries = history_list.clone();
                    history_entries.push(text.clone());
                    if history_entries.len() > 1000 {
                        history_entries.remove(0);
                    }
                    let _ = std::fs::write(path, history_entries.join("\n"));
                }
            }
        }

        Ok(result)
    }

    pub async fn start(&mut self) -> Result<()> {
        let workspace = self.settings.workspace_dir.to_string_lossy().to_string();
        theme::print_logo(
            &self.settings.active_model,
            &self.settings.active_provider,
            &workspace,
        );

        if let Some(ref path) = self.history_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut orchestrator = crate::agent::AgentOrchestrator::new(self.settings.clone());
        let store = crate::session::SessionStore::open_default()?;
        let session_id = if let Some(resume_id) = self.resume_session_id.clone() {
            if store.get_session(&resume_id)?.is_none() {
                anyhow::bail!("Session not found: {}", resume_id);
            }
            resume_id
        } else {
            store
                .create_session(&self.settings, Some("Interactive session"))?
                .id
        };
        orchestrator.attach_session_store(store, session_id.clone())?;
        let _ = orchestrator.initialize_mcp().await;
        theme::print_info(&format!("Session: {}", session_id));

        println!(
            "  {} Type {} for commands, {} to quit\n",
            "tip:".truecolor(107, 114, 128),
            "/".bold().truecolor(99, 179, 237),
            "/exit".bold().truecolor(99, 179, 237),
        );

        loop {
            let prompt = format_user_prompt();
            match self.read_line_custom(&prompt).await {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if trimmed.starts_with('/') {
                        let (should_exit, settings_changed) =
                            self.handle_slash_command(trimmed, &mut orchestrator).await;
                        if settings_changed {
                            orchestrator.update_settings(self.settings.clone());
                        }
                        if should_exit {
                            break;
                        }
                    } else {
                        if let Err(e) = orchestrator.process_message(trimmed).await {
                            theme::print_error(&format!("Orchestrator error: {}", e));
                        }
                    }
                }
                Ok(None) => {
                    theme::print_info("Session interrupted. Goodbye!");
                    break;
                }
                Err(err) => {
                    theme::print_error(&format!("Terminal error: {:?}", err));
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handles slash commands. Returns (should_exit, settings_changed).
    async fn handle_slash_command(
        &mut self,
        cmd: &str,
        orchestrator: &mut crate::agent::AgentOrchestrator,
    ) -> (bool, bool) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return (false, false);
        }
        let command = parts[0];
        let mut settings_changed = false;

        match command {
            "/exit" | "/quit" => {
                theme::print_info("Exiting OrionBot. Have a great day!");
                return (true, false);
            }
            "/help" => {
                println!("\n{}", "Available Commands:".bold().truecolor(99, 179, 237));
                for c in command_picker::COMMANDS {
                    println!(
                        "  {}  {:<14} {}",
                        c.icon,
                        c.name.bold().truecolor(94, 234, 212),
                        c.description.truecolor(107, 114, 128),
                    );
                }
                println!();
            }
            "/clear" => {
                let _ = std::process::Command::new("cmd")
                    .args(&["/c", "cls"])
                    .status()
                    .or_else(|_| std::process::Command::new("clear").status());
                // Reprint logo after clear
                let workspace = self.settings.workspace_dir.to_string_lossy().to_string();
                theme::print_logo(
                    &self.settings.active_model,
                    &self.settings.active_provider,
                    &workspace,
                );
            }
            "/model" => {
                let (provider, model) = if parts.len() == 1 {
                    // Interactive flow
                    if let Ok(Some(selected_provider)) = command_picker::run_provider_picker() {
                        let provider = selected_provider;
                        if let Ok(Some(mut selected_model)) =
                            command_picker::run_model_picker(&provider)
                        {
                            if selected_model == "Custom..." {
                                if let Ok(Some(custom_model)) =
                                    command_picker::prompt_for_input("Enter custom model name: ")
                                {
                                    selected_model = custom_model;
                                } else {
                                    return (false, false);
                                }
                            }
                            (provider, selected_model)
                        } else {
                            return (false, false);
                        }
                    } else {
                        return (false, false);
                    }
                } else {
                    let full_model = parts[1];
                    if let Some(idx) = full_model.find(':') {
                        (
                            full_model[..idx].to_string(),
                            full_model[idx + 1..].to_string(),
                        )
                    } else {
                        theme::print_error("Invalid model format. Use provider:model (e.g. anthropic:claude-3-opus-latest)");
                        return (false, false);
                    }
                };

                // Check API key
                let mut needs_key = false;
                if provider != "ollama" {
                    if let Some(config) = self.settings.providers.get(&provider) {
                        if config.api_key.is_none() || config.api_key.as_ref().unwrap().is_empty() {
                            needs_key = true;
                        }
                    } else {
                        needs_key = true;
                    }
                }

                if needs_key {
                    let prompt_text = format!("Enter API Key for {}: ", provider);
                    if let Ok(Some(api_key)) = command_picker::prompt_for_input(&prompt_text) {
                        if let Err(e) = self.settings.set_provider_key(&provider, &api_key) {
                            theme::print_error(&format!("Failed to securely store API key: {}", e));
                        }
                    } else {
                        theme::print_warning("API key not provided. Model switch cancelled.");
                        return (false, false);
                    }
                }

                if provider == "cloudflare" {
                    let has_base = self
                        .settings
                        .providers
                        .get("cloudflare")
                        .and_then(|c| c.api_base.as_ref())
                        .map(|b| !b.is_empty())
                        .unwrap_or(false);

                    if !has_base
                        && std::env::var("CLOUDFLARE_ACCOUNT_ID").is_err()
                        && std::env::var("WORKERS_AI_ACCOUNT_ID").is_err()
                    {
                        let prompt_text = "Enter Cloudflare Account ID: ";
                        if let Ok(Some(account_id)) = command_picker::prompt_for_input(prompt_text)
                        {
                            let config = self
                                .settings
                                .providers
                                .entry("cloudflare".to_string())
                                .or_insert_with(|| crate::config::settings::ProviderConfig {
                                    api_key: None,
                                    api_base: None,
                                });
                            config.api_base = Some(format!(
                                "https://api.cloudflare.com/client/v4/accounts/{}/ai/v1",
                                account_id.trim()
                            ));
                        } else {
                            theme::print_warning(
                                "Cloudflare Account ID not provided. Model switch cancelled.",
                            );
                            return (false, false);
                        }
                    }
                }

                self.settings.active_provider = provider.clone();
                self.settings.active_model = model.clone();

                if let Err(e) = self.settings.save() {
                    theme::print_error(&format!("Failed to save new model setting: {}", e));
                } else {
                    settings_changed = true;
                    theme::print_success(&format!(
                        "Switched to {} · {}",
                        provider.bold().truecolor(94, 234, 212),
                        model.bold().truecolor(94, 234, 212),
                    ));
                }
            }
            "/skill" => {
                if parts.len() < 2 {
                    theme::print_warning(
                        "Usage: /skill load <name>, /skill list, or /skill add <owner/repo>",
                    );
                    use colored::Colorize;
                    println!(
                        "  {} Type {} at the prompt to open the interactive skill picker.",
                        "Tip:".bold().truecolor(252, 211, 77),
                        ".".bold().white()
                    );
                } else {
                    match parts[1] {
                        "list" => {
                            let _ = self.skill_registry.scan_skills_dir();
                            let list = self.skill_registry.list();
                            if list.is_empty() {
                                println!("\nNo skills found in ~/.orion/skills/\n");
                            } else {
                                println!(
                                    "\n{}",
                                    "Available Skills:".bold().truecolor(99, 179, 237)
                                );
                                for s in list {
                                    println!(
                                        "  - {} (v{}) : {}",
                                        s.skill.name.bold().truecolor(94, 234, 212),
                                        s.skill.version,
                                        s.skill.description.truecolor(107, 114, 128)
                                    );
                                }
                                println!();
                            }
                        }
                        "load" => {
                            if parts.len() < 3 {
                                theme::print_warning("Usage: /skill load <name>");
                            } else {
                                let name = parts[2];
                                let _ = self.skill_registry.scan_skills_dir();
                                if let Some(skill) = self.skill_registry.get(name) {
                                    orchestrator.load_skill(skill);
                                    theme::print_success(&format!(
                                        "Loaded skill: {} (v{})",
                                        skill.skill.name.bold().truecolor(94, 234, 212),
                                        skill.skill.version
                                    ));
                                } else {
                                    theme::print_error(&format!("Skill not found: {}", name));
                                }
                            }
                        }
                        "add" => {
                            if parts.len() < 3 {
                                theme::print_warning("Usage: /skill add <owner/repo>");
                            } else {
                                let repo = parts[2];
                                theme::print_info(&format!(
                                    "Downloading skill from repository '{}'...",
                                    repo
                                ));
                                match crate::skills::download_skill(repo).await {
                                    Ok(names) => {
                                        theme::print_success(&format!(
                                            "Successfully downloaded {} skill(s):",
                                            names.len()
                                        ));
                                        let _ = self.skill_registry.scan_skills_dir();
                                        for name in names {
                                            println!("  - {}", name.bold().truecolor(94, 234, 212));
                                            if let Some(skill) = self.skill_registry.get(&name) {
                                                orchestrator.load_skill(skill);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        theme::print_error(&format!("Failed to add skill: {}", e));
                                    }
                                }
                            }
                        }
                        _ => {
                            theme::print_warning("Usage: /skill load <name>, /skill list, or /skill add <owner/repo>");
                            use colored::Colorize;
                            println!(
                                "  {} Type {} at the prompt to open the interactive skill picker.",
                                "Tip:".bold().truecolor(252, 211, 77),
                                ".".bold().white()
                            );
                        }
                    }
                }
            }
            "/session" => {
                if let Ok(store) = crate::session::SessionStore::open_default() {
                    let subcmd = parts.get(1).map(|s| *s);
                    match subcmd {
                        Some("list") => {
                            if let Ok(list) = store.list_sessions(25) {
                                if list.is_empty() {
                                    println!("\nNo saved sessions found.\n");
                                } else {
                                    println!(
                                        "\n{}",
                                        "Recent Sessions:".bold().truecolor(99, 179, 237)
                                    );
                                    for meta in list {
                                        println!(
                                            "  {}  {}  {}:{}  {}",
                                            meta.id.cyan(),
                                            meta.updated_at.format("%Y-%m-%d %H:%M"),
                                            meta.provider,
                                            meta.model,
                                            meta.title
                                        );
                                    }
                                    println!();
                                }
                            }
                        }
                        Some("resume") => {
                            if parts.len() < 3 {
                                theme::print_warning("Usage: /session resume <session_id>");
                            } else {
                                let session_id = parts[2].to_string();
                                match store.get_session(&session_id) {
                                    Ok(Some(meta)) => {
                                        match orchestrator
                                            .attach_session_store(store, session_id.clone())
                                        {
                                            Ok(_) => {
                                                self.settings.active_provider = meta.provider;
                                                self.settings.active_model = meta.model;
                                                let _ = self.settings.save();
                                                settings_changed = true;
                                                theme::print_success(&format!(
                                                    "Resumed session: {}",
                                                    session_id
                                                ));
                                            }
                                            Err(e) => {
                                                theme::print_error(&format!(
                                                    "Failed to resume session: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        theme::print_error(&format!(
                                            "Session not found: {}",
                                            session_id
                                        ));
                                    }
                                    Err(e) => {
                                        theme::print_error(&format!("Database error: {}", e));
                                    }
                                }
                            }
                        }
                        Some("delete") => {
                            if parts.len() < 3 {
                                theme::print_warning("Usage: /session delete <session_id>");
                            } else {
                                let session_id = parts[2].to_string();
                                match store.delete_session(&session_id) {
                                    Ok(_) => {
                                        theme::print_success(&format!(
                                            "Deleted session: {}",
                                            session_id
                                        ));
                                        if Some(&session_id)
                                            == orchestrator
                                                .session_id()
                                                .map(|s| s.to_string())
                                                .as_ref()
                                        {
                                            theme::print_info("You deleted your active session. Please restart Orion or run /session to pick a new one.");
                                        }
                                    }
                                    Err(e) => theme::print_error(&format!(
                                        "Failed to delete session: {}",
                                        e
                                    )),
                                }
                            }
                        }
                        _ => {
                            // Interactive flow
                            if let Ok(list) = store.list_sessions(25) {
                                if list.is_empty() {
                                    theme::print_warning("No saved sessions found.");
                                } else {
                                    use crate::cli::command_picker::{
                                        run_dynamic_picker, DynamicCommand,
                                    };
                                    let items: Vec<DynamicCommand> = list
                                        .iter()
                                        .map(|meta| DynamicCommand {
                                            name: meta.title.clone(),
                                            description: format!(
                                                "{} · {}:{}",
                                                if meta.id.len() > 8 {
                                                    &meta.id[..8]
                                                } else {
                                                    &meta.id
                                                },
                                                meta.provider,
                                                meta.model
                                            ),
                                            icon: "💬".to_string(),
                                        })
                                        .collect();
                                    if let Ok(Some(selected_title)) =
                                        run_dynamic_picker("Select Session to Resume", &items)
                                    {
                                        if let Some(target) =
                                            list.iter().find(|s| s.title == selected_title)
                                        {
                                            match orchestrator
                                                .attach_session_store(store, target.id.clone())
                                            {
                                                Ok(_) => {
                                                    self.settings.active_provider =
                                                        target.provider.clone();
                                                    self.settings.active_model =
                                                        target.model.clone();
                                                    let _ = self.settings.save();
                                                    settings_changed = true;
                                                    theme::print_success(&format!(
                                                        "Resumed session: {}",
                                                        target.id
                                                    ));
                                                }
                                                Err(e) => {
                                                    theme::print_error(&format!(
                                                        "Failed to resume session: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    theme::print_error("Failed to open session store.");
                }
            }
            "/vision" => {
                if parts.len() < 2 {
                    theme::print_warning("Usage: /vision <file_path>");
                } else {
                    let path_str = parts[1];
                    let path = std::path::Path::new(path_str);
                    if !path.exists() {
                        theme::print_error(&format!("File does not exist: {}", path_str));
                    } else {
                        match std::fs::read(path) {
                            Ok(bytes) => {
                                let ext = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("png")
                                    .to_lowercase();
                                let media_type = match ext.as_str() {
                                    "jpg" | "jpeg" => "image/jpeg",
                                    "gif" => "image/gif",
                                    "webp" => "image/webp",
                                    _ => "image/png",
                                };
                                let base64_data = base64_encode(&bytes);
                                orchestrator.pending_images.push(
                                    crate::llm::provider::ImageContent {
                                        media_type: media_type.to_string(),
                                        data: base64_data,
                                    },
                                );
                                theme::print_success(&format!(
                                    "Loaded image: {} (will be sent with your next message)",
                                    path.file_name()
                                        .and_then(|f| f.to_str())
                                        .unwrap_or(path_str)
                                ));
                            }
                            Err(e) => {
                                theme::print_error(&format!("Failed to read image file: {}", e));
                            }
                        }
                    }
                }
            }
            "/screenshot" => {
                theme::print_info("Capturing screenshot...");
                let script = r#"Add-Type -AssemblyName System.Windows.Forms; $bmp = New-Object System.Drawing.Bitmap([System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width, [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height); $graphics = [System.Drawing.Graphics]::FromImage($bmp); $graphics.CopyFromScreen(0,0,0,0, $bmp.Size); $bmp.Save('orion_screenshot.png', [System.Drawing.Imaging.ImageFormat]::Png); $graphics.Dispose(); $bmp.Dispose();"#;
                let output = std::process::Command::new("powershell")
                    .args(&["-Command", script])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        let path = std::path::Path::new("orion_screenshot.png");
                        if path.exists() {
                            match std::fs::read(path) {
                                Ok(bytes) => {
                                    let base64_data = base64_encode(&bytes);
                                    orchestrator.pending_images.push(
                                        crate::llm::provider::ImageContent {
                                            media_type: "image/png".to_string(),
                                            data: base64_data,
                                        },
                                    );
                                    let _ = std::fs::remove_file(path);
                                    theme::print_success("Screenshot captured and loaded (will be sent with your next message)");
                                }
                                Err(e) => {
                                    theme::print_error(&format!(
                                        "Failed to read captured screenshot: {}",
                                        e
                                    ));
                                }
                            }
                        } else {
                            theme::print_error(
                                "Screenshot file was not saved successfully by PowerShell.",
                            );
                        }
                    }
                    Ok(out) => {
                        let err_msg = String::from_utf8_lossy(&out.stderr);
                        theme::print_error(&format!("Failed to capture screenshot: {}", err_msg));
                    }
                    Err(e) => {
                        theme::print_error(&format!("Failed to run screenshot script: {}", e));
                    }
                }
            }
            "/web" => {
                if parts.len() < 2 {
                    theme::print_warning("Usage: /web <url>");
                } else {
                    let url = parts[1];
                    theme::print_info(&format!("Scraping web page: {}...", url));
                    use crate::tools::Tool;
                    let tool = crate::tools::browser::BrowserReadTool;
                    let ctx = crate::tools::ToolContext {
                        settings: self.settings.clone(),
                    };
                    match tool.execute(serde_json::json!({ "url": url }), &ctx).await {
                        Ok(content) => {
                            orchestrator.queue_context(crate::session::PendingContext {
                                label: format!("URL: {}", url),
                                content: content.clone(),
                            });
                            theme::print_success(&format!(
                                "Loaded {} chars from {} (will be sent with your next message)",
                                content.len(),
                                url
                            ));
                        }
                        Err(e) => theme::print_error(&format!("Failed to read web page: {}", e)),
                    }
                }
            }
            "/search" => {
                if parts.len() < 2 {
                    theme::print_warning("Usage: /search <query>");
                } else {
                    let query = parts[1..].join(" ");
                    theme::print_info(&format!("Searching web for: {}...", query));
                    use crate::tools::Tool;
                    let tool = crate::tools::browser::WebSearchTool;
                    let ctx = crate::tools::ToolContext {
                        settings: self.settings.clone(),
                    };
                    match tool
                        .execute(serde_json::json!({ "query": query }), &ctx)
                        .await
                    {
                        Ok(content) => {
                            orchestrator.queue_context(crate::session::PendingContext {
                                label: format!("Search: {}", query),
                                content: content.clone(),
                            });
                            theme::print_success("Search results loaded. Generating answer...");
                            let _ = orchestrator.process_message(&query).await;
                        }
                        Err(e) => theme::print_error(&format!("Failed to search web: {}", e)),
                    }
                }
            }
            "/commit" => {
                theme::print_info("Gathering staged changes...");
                let output = std::process::Command::new("git")
                    .args(&["diff", "--cached"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => {
                        let mut diff = String::from_utf8_lossy(&out.stdout).to_string();
                        if diff.trim().is_empty() {
                            let un_staged =
                                std::process::Command::new("git").args(&["diff"]).output();
                            if let Ok(un_out) = un_staged {
                                diff = String::from_utf8_lossy(&un_out.stdout).to_string();
                            }
                        }
                        if diff.trim().is_empty() {
                            theme::print_warning("No changes found to commit.");
                        } else {
                            theme::print_success("Changes gathered. Generating commit message...");
                            let prompt = format!("Write a concise, conventional git commit message for the following diff. Only output the commit message, no markdown code blocks or explanations.\n\n{}", diff);
                            let _ = orchestrator.process_message(&prompt).await;
                        }
                    }
                    _ => theme::print_error("Failed to run git diff. Are you in a git repository?"),
                }
            }
            "/pr" => {
                theme::print_info("Gathering unmerged changes...");
                let output = std::process::Command::new("git")
                    .args(&["diff", "master...HEAD"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => {
                        let diff = String::from_utf8_lossy(&out.stdout).to_string();
                        if diff.trim().is_empty() {
                            theme::print_warning("No changes found compared to master branch.");
                        } else {
                            theme::print_success("Changes gathered. Generating PR description...");
                            let prompt = format!("Write a detailed GitHub Pull Request description for the following diff. Include a title, summary, and bullet points of changes.\n\n{}", diff);
                            let _ = orchestrator.process_message(&prompt).await;
                        }
                    }
                    _ => theme::print_error(
                        "Failed to run git diff against master. Are you in a git repository?",
                    ),
                }
            }
            "/stats" => {
                if let Ok(store) = crate::session::SessionStore::open_default() {
                    theme::print_info("Usage Statistics");
                    println!(
                        "{}",
                        "================================".truecolor(107, 114, 128)
                    );

                    if let Some(id) = orchestrator.session_id() {
                        if let Ok((i, o, c)) = store.get_session_stats(id) {
                            println!("Current Session:");
                            println!("  Input Tokens : {}", i);
                            println!("  Output Tokens: {}", o);
                            println!("  Total Cost   : ${:.4}", c);
                        }
                    }

                    if let Ok((i, o, c)) = store.get_all_time_stats() {
                        println!("\nAll-Time:");
                        println!("  Input Tokens : {}", i);
                        println!("  Output Tokens: {}", o);
                        println!("  Total Cost   : ${:.4}", c);
                    }
                    println!(
                        "{}",
                        "================================".truecolor(107, 114, 128)
                    );
                } else {
                    theme::print_error("Failed to open session store.");
                }
            }
            _ => {
                theme::print_warning(&format!(
                    "Unknown command: {}  —  type {} or press {} for the command picker.",
                    command.bold(),
                    "/help".bold().truecolor(99, 179, 237),
                    "/".bold().truecolor(99, 179, 237),
                ));
            }
        }

        (false, settings_changed)
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut chunks = bytes.chunks_exact(3);
    while let Some(chunk) = chunks.next() {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        result.push(CHARSET[(n & 63) as usize] as char);
    }
    let remainder = chunks.remainder();
    if remainder.len() == 1 {
        let n = (remainder[0] as u32) << 16;
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push('=');
        result.push('=');
    } else if remainder.len() == 2 {
        let n = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        result.push('=');
    }
    result
}
