mod agent;
mod cli;
mod config;
mod llm;
mod mcp;
mod multi_agent;
mod sandbox;
mod session;
mod skills;
mod tools;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use cli::Repl;
use colored::Colorize;
use config::Settings;

#[derive(Parser, Debug)]
#[command(name = "orion")]
#[command(author = "JL <jl@orion.bot>")]
#[command(version = "0.1.0")]
#[command(about = "OrionBot CLI - High-performance agentic coding assistant in Rust", long_about = None)]
struct Cli {
    /// Optional one-shot instruction prompt to run without launching REPL
    #[arg(index = 1)]
    prompt: Option<String>,

    /// Override the default LLM model (e.g. anthropic:claude-3-5-sonnet)
    #[arg(short, long)]
    model: Option<String>,

    /// Resume a saved session by ID
    #[arg(long)]
    resume: Option<String>,

    /// Enable diagnostic tracing (logs payloads to .orion/traces/)
    #[arg(long)]
    trace: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show or set environment and config options
    Config {
        /// Print active configurations
        #[arg(short, long)]
        show: bool,
    },
    /// Manage skills
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Manage saved chat sessions
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Generate shell completion scripts
    Completions { shell: CompletionShell },
    /// Execute a single prompt and exit (Headless mode)
    Execute {
        /// The instruction to execute
        prompt: Option<String>,
        /// Read the instruction from stdin
        #[arg(long)]
        stdin: bool,
    },
}

#[derive(Subcommand, Debug)]
enum SkillAction {
    /// Add a skill from a GitHub repository
    Add { repo: String },
    /// List installed skills
    List,
}

#[derive(Subcommand, Debug)]
enum SessionAction {
    /// List recent sessions
    List,
    /// Show a session transcript
    Show { session_id: String },
}

#[derive(Clone, Debug, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Powershell,
}

impl From<CompletionShell> for Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Powershell => Shell::PowerShell,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args = Cli::parse();

    // Load active settings (env + toml config)
    let mut settings = Settings::load()?;

    if let Some(ref m_override) = args.model {
        if let Some(idx) = m_override.find(':') {
            settings.active_provider = m_override[..idx].to_string();
            settings.active_model = m_override[idx + 1..].to_string();
        } else {
            settings.active_model = m_override.to_string();
        }
    }

    settings.trace_enabled = args.trace;

    // Process CLI commands if provided
    if let Some(command) = args.command {
        match command {
            Commands::Config { show } => {
                if show {
                    println!("Active settings configuration:");
                    println!("{}", toml::to_string_pretty(&settings)?);
                } else {
                    println!("Use 'orion config --show' to view configurations.");
                }
                return Ok(());
            }
            Commands::Skill { action } => {
                match action {
                    SkillAction::Add { repo } => {
                        println!("Downloading skill from {}...", repo);
                        match skills::download_skill(&repo).await {
                            Ok(skill_names) => {
                                for name in skill_names {
                                    println!("{} Successfully added skill: {}", "✓".green(), name);
                                }
                            }
                            Err(e) => {
                                eprintln!("{} Failed to add skill: {}", "✗".red(), e);
                            }
                        }
                    }
                    SkillAction::List => {
                        let registry = skills::SkillRegistry::load_defaults();
                        let list = registry.list();
                        if list.is_empty() {
                            println!("No skills installed.");
                        } else {
                            println!("Installed skills:");
                            for s in list {
                                println!("  - {} (v{})", s.skill.name.cyan(), s.skill.version);
                                println!("    {}", s.skill.description.bright_black());
                            }
                        }
                    }
                }
                return Ok(());
            }
            Commands::Sessions { action } => {
                let store = session::SessionStore::open_default()?;
                match action {
                    SessionAction::List => {
                        for meta in store.list_sessions(25)? {
                            println!(
                                "{}  {}  {}:{}  {}",
                                meta.id.cyan(),
                                meta.updated_at.format("%Y-%m-%d %H:%M"),
                                meta.provider,
                                meta.model,
                                meta.title
                            );
                        }
                    }
                    SessionAction::Show { session_id } => {
                        let Some(meta) = store.get_session(&session_id)? else {
                            eprintln!("Session not found: {}", session_id);
                            return Ok(());
                        };
                        println!(
                            "{} {}  {}:{}  {}",
                            "Session".bold(),
                            meta.id.cyan(),
                            meta.provider,
                            meta.model,
                            meta.title
                        );
                        for msg in store.load_stored_messages(&session_id)? {
                            println!("\n[{} #{}]\n{}", msg.role.bold(), msg.seq, msg.content);
                        }
                    }
                }
                return Ok(());
            }
            Commands::Completions { shell } => {
                let mut cmd = Cli::command();
                let shell: Shell = shell.into();
                generate(shell, &mut cmd, "orion", &mut std::io::stdout());
                return Ok(());
            }
            Commands::Execute { prompt, stdin } => {
                let mut instruction = String::new();
                if stdin {
                    use std::io::Read;
                    std::io::stdin().read_to_string(&mut instruction)?;
                } else if let Some(p) = prompt {
                    instruction = p;
                } else {
                    anyhow::bail!("Must provide a prompt or use --stdin");
                }

                let settings_snapshot = settings.clone();
                let mut orchestrator = agent::AgentOrchestrator::new(settings);
                let store = session::SessionStore::open_default()?;
                let session_id = if let Some(resume_id) = args.resume {
                    resume_id
                } else {
                    store
                        .create_session(&settings_snapshot, Some(&instruction))?
                        .id
                };
                orchestrator.attach_session_store(store, session_id)?;
                orchestrator.process_message(&instruction).await?;
                return Ok(());
            }
        }
    }

    // Execute one-shot query or launch interactive REPL
    if let Some(prompt) = args.prompt {
        let settings_snapshot = settings.clone();
        let mut orchestrator = agent::AgentOrchestrator::new(settings);
        let store = session::SessionStore::open_default()?;
        let session_id = if let Some(resume_id) = args.resume {
            resume_id
        } else {
            store.create_session(&settings_snapshot, Some(&prompt))?.id
        };
        orchestrator.attach_session_store(store, session_id)?;
        orchestrator.process_message(&prompt).await?;
    } else {
        // Launch REPL
        let mut repl = Repl::new(settings, args.resume);
        if let Err(e) = repl.start().await {
            eprintln!("REPL error: {:?}", e);
        }
    }

    Ok(())
}
