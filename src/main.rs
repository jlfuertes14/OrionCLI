mod config;
mod cli;
mod llm;
mod tools;
mod agent;
mod sandbox;
mod multi_agent;

use clap::{Parser, Subcommand};
use anyhow::Result;
use config::Settings;
use cli::Repl;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args = Cli::parse();

    // Load active settings (env + toml config)
    let mut settings = Settings::load()?;

    // Override model if CLI arg is present
    if let Some(ref m_override) = args.model {
        if let Some(idx) = m_override.find(':') {
            settings.active_provider = m_override[..idx].to_string();
            settings.active_model = m_override[idx + 1..].to_string();
        } else {
            settings.active_model = m_override.to_string();
        }
    }

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
        }
    }

    // Execute one-shot query or launch interactive REPL
    if let Some(prompt) = args.prompt {
        let mut orchestrator = agent::AgentOrchestrator::new(settings);
        orchestrator.process_message(&prompt).await?;
    } else {
        // Launch REPL
        let mut repl = Repl::new(settings);
        if let Err(e) = repl.start().await {
            eprintln!("REPL error: {:?}", e);
        }
    }

    Ok(())
}
