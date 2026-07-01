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
}

impl Repl {
    pub fn new(settings: Settings) -> Self {
        let history_path = dirs::home_dir().map(|h| h.join(".orion").join("history.txt"));
        Repl {
            settings,
            history_path,
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
                                    self.handle_slash_command(&cmd);
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
                        let (should_exit, settings_changed) = self.handle_slash_command(trimmed);
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
    fn handle_slash_command(&mut self, cmd: &str) -> (bool, bool) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
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
