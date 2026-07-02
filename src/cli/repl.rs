use std::path::PathBuf;
use rustyline::error::ReadlineError;
use rustyline::{Editor, Helper};
use rustyline::completion::Completer;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::history::DefaultHistory;
use std::borrow::Cow;
use colored::Colorize;
use crate::config::Settings;
use crate::cli::theme::{self, format_user_prompt};
use crate::cli::command_picker;

struct OrionHelper;

impl Helper for OrionHelper {}

impl Completer for OrionHelper {
    type Candidate = String;
}

impl Hinter for OrionHelper {
    type Hint = String;
}

impl Highlighter for OrionHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default && prompt == "❯ " {
            Cow::Owned(format!("{}", "❯ ".bold().truecolor(76, 175, 80)))
        } else {
            Cow::Borrowed(prompt)
        }
    }
}

impl Validator for OrionHelper {}

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

    pub async fn start(&mut self) -> rustyline::Result<()> {
        theme::print_logo(&self.settings.active_model, &self.settings.active_provider);

        let mut rl = Editor::<OrionHelper, DefaultHistory>::new()?;
        rl.set_helper(Some(OrionHelper));

        if let Some(ref path) = self.history_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = rl.load_history(path);
        }

        let mut orchestrator = crate::agent::AgentOrchestrator::new(self.settings.clone());
        let _ = orchestrator.initialize_mcp().await;

        // Print the hint once at startup
        println!(
            "  {} Type {} for commands, {} to quit\n",
            "tip:".truecolor(107, 114, 128),
            "/".bold().truecolor(99, 179, 237),
            "/exit".bold().truecolor(99, 179, 237),
        );

        loop {
            let readline = rl.readline(&format_user_prompt());
            match readline {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // If user typed exactly "/" (just a slash), open the picker
                    if trimmed == "/" {
                        // Move to next line so picker renders below the prompt
                        println!();
                        match command_picker::run_picker() {
                            Ok(Some(cmd)) => {
                                // Echo the selected command and execute it
                                println!(
                                    "{}",
                                    format!("❯ {}", cmd).truecolor(99, 179, 237)
                                );
                                let _ = rl.add_history_entry(&cmd);
                                let (should_exit, settings_changed) =
                                    self.handle_slash_command(&cmd, &mut orchestrator).await;
                                if settings_changed {
                                    orchestrator.update_settings(self.settings.clone());
                                }
                                if should_exit {
                                    break;
                                }
                            }
                            Ok(None) => {
                                // Cancelled — just continue
                            }
                            Err(e) => {
                                theme::print_error(&format!("Picker error: {}", e));
                            }
                        }
                        continue;
                    }

                    let _ = rl.add_history_entry(trimmed);

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
                Err(ReadlineError::Interrupted) => {
                    theme::print_info("Session interrupted. Goodbye!");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    theme::print_info("EOF received. Exiting.");
                    break;
                }
                Err(err) => {
                    theme::print_error(&format!("Readline error: {:?}", err));
                    break;
                }
            }
        }

        if let Some(ref path) = self.history_path {
            let _ = rl.save_history(path);
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
                                    Ok(name) => {
                                        theme::print_success(&format!(
                                            "Successfully added and loaded skill: {}",
                                            name.bold().truecolor(94, 234, 212)
                                        ));
                                        let _ = self.skill_registry.scan_skills_dir();
                                        if let Some(skill) = self.skill_registry.get(&name) {
                                            orchestrator.load_skill(skill);
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
