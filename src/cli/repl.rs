use std::path::PathBuf;
use anyhow::Result;
use colored::Colorize;
use crate::config::Settings;
use crate::cli::theme::{self, format_user_prompt};
use crate::cli::command_picker;
use crossterm::event::{Event, KeyCode, KeyModifiers};

pub struct Repl {
    settings: Settings,
    history_path: Option<PathBuf>,
    skill_registry: crate::skills::SkillRegistry,
}

impl Repl {
    pub fn new(settings: Settings) -> Self {
        let history_path = dirs::home_dir().map(|h| h.join(".orion").join("history.txt"));
        Repl {
            settings,
            history_path,
            skill_registry: crate::skills::SkillRegistry::load_defaults(),
        }
    }

    async fn read_line_custom(&mut self, prompt: &str) -> Result<Option<String>> {
        use crossterm::style::Print;
        use crossterm::terminal::{self, ClearType};
        use crossterm::{execute, event};
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
                        if (line.is_empty() || line.starts_with("/skill load")) && c == '/' {
                            terminal::disable_raw_mode()?;
                            println!();
                            if let Some(selected) = command_picker::run_picker()? {
                                terminal::enable_raw_mode()?;
                                line = selected;
                                execute!(
                                    stdout,
                                    Print("\r"),
                                    terminal::Clear(ClearType::CurrentLine),
                                    Print(prompt),
                                    Print(&line)
                                )?;
                                stdout.flush()?;
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
                            if let Some(selected) = command_picker::run_skills_picker(&mut self.skill_registry)? {
                                terminal::enable_raw_mode()?;
                                line = format!("/skill load {}", selected);
                                execute!(
                                    stdout,
                                    Print("\r"),
                                    terminal::Clear(ClearType::CurrentLine),
                                    Print(prompt),
                                    Print(&line)
                                )?;
                                stdout.flush()?;
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
                                if let Some(selected) = command_picker::run_skills_picker(&mut self.skill_registry)? {
                                    terminal::enable_raw_mode()?;
                                    line = format!("/skill load {}", selected);
                                    execute!(
                                        stdout,
                                        Print("\r"),
                                        terminal::Clear(ClearType::CurrentLine),
                                        Print(prompt),
                                        Print(&line)
                                    )?;
                                    stdout.flush()?;
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
        theme::print_logo(&self.settings.active_model, &self.settings.active_provider);

        if let Some(ref path) = self.history_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut orchestrator = crate::agent::AgentOrchestrator::new(self.settings.clone());
        let _ = orchestrator.initialize_mcp().await;

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
                        let (should_exit, settings_changed) = self.handle_slash_command(trimmed, &mut orchestrator).await;
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
    async fn handle_slash_command(&mut self, cmd: &str, orchestrator: &mut crate::agent::AgentOrchestrator) -> (bool, bool) {
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
                    .or_else(|_| {
                        std::process::Command::new("clear").status()
                    });
                // Reprint logo after clear
                theme::print_logo(&self.settings.active_model, &self.settings.active_provider);
            }
            "/model" => {
                if parts.len() == 1 {
                    println!("\n{}", "Current LLM Model Settings:".bold().truecolor(99, 179, 237));
                    println!("  Provider: {}", self.settings.active_provider.bold().truecolor(94, 234, 212));
                    println!("  Model:    {}\n", self.settings.active_model.bold().truecolor(94, 234, 212));
                } else {
                    let full_model = parts[1];
                    if let Some(idx) = full_model.find(':') {
                        let provider = &full_model[..idx];
                        let model = &full_model[idx + 1..];
                        self.settings.active_provider = provider.to_string();
                        self.settings.active_model = model.to_string();
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
                    } else {
                        theme::print_warning("Invalid format. Use: /model <provider>:<model>");
                        theme::print_warning("Example: /model anthropic:claude-opus-4-5");
                    }
                }
            }
            "/skill" => {
                if parts.len() < 2 {
                    theme::print_warning("Usage: /skill load <name> or /skill list");
                } else {
                    match parts[1] {
                        "list" => {
                            let _ = self.skill_registry.scan_skills_dir();
                            let list = self.skill_registry.list();
                            if list.is_empty() {
                                println!("\nNo skills found in ~/.orion/skills/\n");
                            } else {
                                println!("\n{}", "Available Skills:".bold().truecolor(99, 179, 237));
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
                                theme::print_info(&format!("Downloading skill from repository '{}'...", repo));
                                match crate::skills::download_skill(repo).await {
                                    Ok(names) => {
                                        theme::print_success(&format!(
                                            "Successfully downloaded {} skill(s):",
                                            names.len()
                                        ));
                                        let _ = self.skill_registry.scan_skills_dir();
                                        for name in names {
                                            println!(
                                                "  - {}",
                                                name.bold().truecolor(94, 234, 212)
                                            );
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
                        }
                    }
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
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
                                let media_type = match ext.as_str() {
                                    "jpg" | "jpeg" => "image/jpeg",
                                    "gif" => "image/gif",
                                    "webp" => "image/webp",
                                    _ => "image/png",
                                };
                                let base64_data = base64_encode(&bytes);
                                orchestrator.pending_images.push(crate::llm::provider::ImageContent {
                                    media_type: media_type.to_string(),
                                    data: base64_data,
                                });
                                theme::print_success(&format!(
                                    "Loaded image: {} (will be sent with your next message)",
                                    path.file_name().and_then(|f| f.to_str()).unwrap_or(path_str)
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
                                    orchestrator.pending_images.push(crate::llm::provider::ImageContent {
                                        media_type: "image/png".to_string(),
                                        data: base64_data,
                                    });
                                    let _ = std::fs::remove_file(path);
                                    theme::print_success("Screenshot captured and loaded (will be sent with your next message)");
                                }
                                Err(e) => {
                                    theme::print_error(&format!("Failed to read captured screenshot: {}", e));
                                }
                            }
                        } else {
                            theme::print_error("Screenshot file was not saved successfully by PowerShell.");
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
