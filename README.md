# Orion

[![npm version](https://img.shields.io/npm/v/@jlfuertes14/orion-agent-cli.svg)](https://www.npmjs.com/package/@jlfuertes14/orion-agent-cli)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Orion is a fast, terminal-native agentic coding assistant written in Rust. It can chat with multiple LLM providers, inspect and edit files, search code, run shell commands, read web pages, manage Git workflows, store sessions, load reusable skills, and run custom tools from your local filesystem.

This README is both a quick start and a practical user guide. Start with the first-run tutorial if you are new to Orion, then use the command reference sections as your day-to-day docs.

## Table Of Contents

- [What Orion Does](#what-orion-does)
- [Installation](#installation)
- [First Run Tutorial](#first-run-tutorial)
- [Configuration](#configuration)
- [Using The Interactive REPL](#using-the-interactive-repl)
- [Headless And Scripted Usage](#headless-and-scripted-usage)
- [Sessions](#sessions)
- [Skills](#skills)
- [Custom Tools](#custom-tools)
- [Web, Search, Vision, And Screenshots](#web-search-vision-and-screenshots)
- [Safety And Sandboxing](#safety-and-sandboxing)
- [Shell Completions](#shell-completions)
- [Release And Publishing](#release-and-publishing)
- [Troubleshooting](#troubleshooting)
- [Developer Notes](#developer-notes)

## What Orion Does

Orion is built around an agent loop:

1. You give Orion an instruction.
2. Orion talks to the configured model provider.
3. The model may request tools such as file reads, file writes, grep, Git commands, browser reads, web search, or terminal commands.
4. Orion executes approved tools, feeds results back to the model, and continues until the task is done or a safety limit is reached.

Core capabilities:

- Interactive REPL with slash commands, command picker, model picker, sessions, and Markdown-style terminal output.
- One-shot execution for automation and CI.
- Multi-provider LLM support for OpenRouter, OpenAI, Anthropic, Gemini, Mistral, Ollama, and Cloudflare Workers AI style endpoints.
- Built-in file, terminal, grep, Git, browser, web-search, YouTube, and multi-agent delegation tools.
- Optional custom tools loaded from `~/.orion/tools`.
- Skills loaded from `~/.orion/skills` and compatible `~/.agents/skills` directories.
- SQLite-backed session history under `~/.orion`.
- Context limits, tool-round limits, read approvals, budget settings, and optional Docker command sandboxing.

## Installation

### Install From NPM

The recommended end-user install is the NPM package. It installs a small JavaScript launcher plus the matching native binary package for your platform.

```bash
npm install -g @jlfuertes14/orion-agent-cli
orion --help
```

Supported NPM binary targets:

- macOS x64
- macOS arm64
- Linux x64 glibc
- Linux arm64 glibc
- Windows x64

### Build From Source

Use this path if you are developing Orion locally.

```bash
git clone https://github.com/jlfuertes14/OrionBot.git
cd OrionBot
cargo build --release
```

Run the local binary:

```bash
./target/release/orion --help
```

On Windows:

```powershell
.\target\release\orion.exe --help
```

### Install With Cargo

If the crate is available in your configured Cargo registry:

```bash
cargo install orion
```

## First Run Tutorial

This walkthrough gets you from a fresh install to a useful coding session.

### 1. Choose A Provider

Orion defaults to OpenRouter with the model `anthropic/claude-3.5-sonnet`. You can use environment variables, a `.env` file in your current project, or `~/.orion/config.toml`.

OpenRouter example:

```bash
export OPENROUTER_API_KEY="your-key"
export ORION_PROVIDER="openrouter"
export ORION_MODEL="anthropic/claude-3.5-sonnet"
```

PowerShell:

```powershell
$env:OPENROUTER_API_KEY = "your-key"
$env:ORION_PROVIDER = "openrouter"
$env:ORION_MODEL = "anthropic/claude-3.5-sonnet"
```

OpenAI example:

```bash
export OPENAI_API_KEY="your-key"
export ORION_PROVIDER="openai"
export ORION_MODEL="gpt-4o"
```

Anthropic example:

```bash
export ANTHROPIC_API_KEY="your-key"
export ORION_PROVIDER="anthropic"
export ORION_MODEL="claude-3-5-sonnet-latest"
```

Local Ollama example:

```bash
ollama serve
ollama pull llama3.1
export ORION_PROVIDER="ollama"
export ORION_MODEL="llama3.1"
export OLLAMA_HOST="http://localhost:11434"
```

### 2. Start Orion In Your Project

Run Orion from the repository or folder you want it to work in:

```bash
cd path/to/your/project
orion
```

The current directory becomes the workspace. Orion automatically adds that workspace to `allowed_dirs` for file access.

### 3. Ask For A Small Read-Only Task

Try a safe first prompt:

```text
Summarize this repository. Mention the entry points, main modules, and likely test commands.
```

Orion can inspect files, search the tree, and explain what it finds.

### 4. Ask For A Focused Edit

Example:

```text
Add input validation to the config loader and run the relevant tests.
```

For best results, be specific about:

- The file or behavior to change.
- The expected result.
- Whether Orion should run tests, formatters, or build commands.
- Whether it may commit changes.

### 5. Review Changes

You can ask Orion:

```text
Show me the files you changed and summarize the verification you ran.
```

Or use Git yourself:

```bash
git status --short
git diff
```

## Configuration

Orion reads configuration in this order:

1. `.env` in the current project, if present.
2. `~/.orion/config.toml`, if present.
3. Direct environment variables, which override TOML values.

Show active settings:

```bash
orion config --show
```

### Important Environment Variables

Provider and model:

```bash
ORION_PROVIDER=openrouter
ORION_MODEL=anthropic/claude-3.5-sonnet
```

Provider keys and endpoints:

```bash
OPENROUTER_API_KEY=...
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
GEMINI_API_KEY=...
MISTRAL_API_KEY=...
OLLAMA_HOST=http://localhost:11434
CLOUDFLARE_API_KEY=...
CLOUDFLARE_ACCOUNT_ID=...
WORKERS_AI_API_KEY=...
WORKERS_AI_ACCOUNT_ID=...
```

Workspace access:

```bash
ALLOWED_DIRS=/path/to/project,/path/to/another/project
```

Web search:

```bash
TAVILY_API_KEY=...
```

If `TAVILY_API_KEY` is not configured, Orion can fall back to DuckDuckGo Lite for basic search.

### Example `~/.orion/config.toml`

```toml
active_provider = "openrouter"
active_model = "anthropic/claude-3.5-sonnet"
allowed_dirs = ["/Users/me/code/my-project"]
fallbacks = ["anthropic:claude-3-5-sonnet-latest", "openai:gpt-4o"]
command_sandbox = "host"
docker_image = "ubuntu:latest"
search_provider = "duckduckgo"

[session]
max_tool_rounds = 25
auto_approve_reads = true
max_budget_usd = 5.00

[providers.openrouter]
api_key = "your-openrouter-key"
api_base = "https://openrouter.ai/api/v1"

[providers.openai]
api_key = "your-openai-key"

[providers.ollama]
api_base = "http://localhost:11434"

[context_limits]
max_file_bytes = 102400
max_total_bytes = 512000
```

Only include keys you actually want stored on disk. For shared machines, prefer environment variables.

### Switching Models

Use a one-off model override:

```bash
orion --model openai:gpt-4o "Review this repository for risky code paths"
```

Inside the REPL:

```text
/model
/model anthropic:claude-3-5-sonnet-latest
/model openrouter:anthropic/claude-3.5-sonnet
```

The slash command can open the interactive provider/model picker, or accept `provider:model` directly.

## Using The Interactive REPL

Start it with:

```bash
orion
```

Useful REPL commands:

| Command | Purpose |
| --- | --- |
| `/help` | Show available slash commands. |
| `/model` | Pick or change the active provider/model. |
| `/skill list` | List installed skills. |
| `/skill load <name>` | Inject a skill into the current session. |
| `/skill add <owner/repo>` | Download a skill from a GitHub repository. |
| `/vision <file_path>` | Attach a local image to the next message. |
| `/screenshot` | Capture the primary screen on Windows and attach it to the next message. |
| `/session list` | List recent sessions. |
| `/session resume <id>` | Resume an existing session. |
| `/session delete <id>` | Delete a saved session. |
| `/web <url>` | Read a web page and add it to context. |
| `/search <query>` | Search the web and answer with loaded search context. |
| `/commit` | Generate a commit message from staged or unstaged diff. |
| `/pr` | Generate pull request text from the current branch diff. |
| `/stats` | Show current session stats. |
| `/clear` | Clear the terminal and redraw the Orion header. |
| `/exit` or `/quit` | Exit Orion. |

Tip: type `/` at an empty prompt to open the interactive command picker.

## Headless And Scripted Usage

Run a single prompt and exit:

```bash
orion execute "Audit src/main.rs and suggest the smallest safe improvement."
```

Read the prompt from standard input:

```bash
cat prompt.txt | orion execute --stdin
```

Use the positional prompt form:

```bash
orion "Explain the build and release process for this repository."
```

Resume a previous session from the command line:

```bash
orion --resume <session-id>
orion --resume <session-id> execute "Continue the refactor and run cargo check."
```

Enable diagnostic tracing:

```bash
orion --trace execute "Reproduce the failing test and explain the payload flow."
```

Tracing logs provider payloads under `.orion/traces/` in the workspace. Do not enable it when prompts may contain secrets unless you are comfortable storing those payloads locally.

## Sessions

Orion stores session metadata and messages in a SQLite database under `~/.orion`.

List sessions:

```bash
orion sessions list
```

Show a transcript:

```bash
orion sessions show <session-id>
```

Inside the REPL:

```text
/session list
/session resume <session-id>
/session delete <session-id>
```

Sessions preserve conversation history and provider/model metadata. They are useful for long-running work, but starting a fresh session is often better for unrelated tasks.

## Skills

Skills are reusable instruction packs. They do not add native code by themselves; they inject specialized guidance into the current agent session.

Orion loads skills from:

- `~/.orion/skills/*.toml`
- `~/.agents/skills/*/SKILL.md` when compatible frontmatter is present

List installed skills:

```bash
orion skill list
```

Add skills from GitHub:

```bash
orion skill add owner/repo
```

The downloader looks for:

- `SKILL.md` at the repository root
- `skills/<skill-name>/SKILL.md`

### Local TOML Skill Example

Create `~/.orion/skills/rust-reviewer.toml`:

```toml
[skill]
name = "rust-reviewer"
description = "Review Rust changes for correctness, safety, and maintainability"
version = "1.0"

[prompt]
inject = """
You are a senior Rust reviewer.
Prioritize correctness, ownership, error handling, async behavior, and tests.
When reviewing, list concrete findings first with file and line references.
"""
```

Load it inside Orion:

```text
/skill load rust-reviewer
```

## Custom Tools

Custom tools let you expose local scripts to the model. Put JSON tool definitions in:

```text
~/.orion/tools/
```

Each `.json` file describes one tool. Orion sends the tool arguments to the command over `stdin` as JSON and captures `stdout` as the tool result.

Example `~/.orion/tools/count_lines.json`:

```json
{
  "name": "count_lines",
  "description": "Count lines in a file relative to the current workspace.",
  "requires_approval": true,
  "command": "python scripts/count_lines.py",
  "schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path relative to the workspace."
      }
    },
    "required": ["path"]
  }
}
```

Example script:

```python
import json
import pathlib
import sys

args = json.load(sys.stdin)
path = pathlib.Path(args["path"])
print(len(path.read_text(encoding="utf-8").splitlines()))
```

Notes:

- Commands run from the active workspace directory.
- `requires_approval` defaults to `true`.
- Prefer small, deterministic tools with explicit schemas.
- Treat custom tools as trusted local code.

## Web, Search, Vision, And Screenshots

Read a page into context:

```text
/web https://docs.rs/tokio/latest/tokio/
```

Search the web:

```text
/search latest Rust 2024 edition migration notes
```

Attach an image to the next message:

```text
/vision ./design/mockup.png
```

Capture a screenshot on Windows:

```text
/screenshot
```

The screenshot command currently uses PowerShell and Windows Forms, so it is Windows-oriented. On other platforms, use `/vision` with a saved screenshot file.

## Safety And Sandboxing

Orion is powerful because it can execute tools. That also means you should run it with the same care you would use for a human pair programmer who can type commands in your terminal.

Safety controls:

- `allowed_dirs` limits filesystem access. The launch workspace is automatically allowed.
- `session.max_tool_rounds` limits how long an autonomous loop can continue.
- `session.auto_approve_reads` can allow read-only file access without repeated prompts.
- `session.max_budget_usd` can cap estimated model spend.
- `command_sandbox = "docker"` can run terminal commands in Docker instead of on the host.
- Some tools require explicit approval before they run.

Docker sandbox example:

```toml
command_sandbox = "docker"
docker_image = "ubuntu:latest"
```

Docker mode is useful for risky shell commands, but it has limitations. Background commands and host-specific tooling may not work the same way inside the container.

## Shell Completions

Generate completions:

```bash
orion completions bash
orion completions zsh
orion completions powershell
```

Example for PowerShell:

```powershell
orion completions powershell > orion.ps1
. .\orion.ps1
```

## Release And Publishing

The repository includes a GitHub Actions workflow at `.github/workflows/publish.yml`.

Current release flow:

1. Push a tag matching `v*`, for example `v0.1.0`.
2. GitHub Actions builds native binaries for Windows, Linux, and macOS targets.
3. The workflow downloads the artifacts into the expected target layout.
4. `cargo npm generate` creates NPM package folders.
5. `cargo npm publish` publishes the main NPM package and platform packages.

One-time setup:

1. Create an NPM automation token.
2. Add it to the GitHub repository as `NPM_TOKEN`.

Release example:

```bash
git status --short
git add .
git commit -m "feat: release v0.1.0"
git push origin main
git tag v0.1.0
git push origin v0.1.0
```

Note: the workflow name mentions crates.io, but the current workflow only publishes NPM packages. Add a `cargo publish` job and `CARGO_REGISTRY_TOKEN` if crates.io publishing is needed.

## Troubleshooting

### `Unsupported platform`

The NPM launcher selects a native package from `process.platform` and `process.arch`. If you see this error, your OS/CPU pair is not one of the published targets. Build from source with Cargo.

### No provider key found

Set the matching environment variable or add the provider to `~/.orion/config.toml`.

```bash
export OPENROUTER_API_KEY="..."
export ORION_PROVIDER="openrouter"
```

Then verify:

```bash
orion config --show
```

### Ollama does not respond

Check that Ollama is running and the model exists:

```bash
ollama serve
ollama list
```

Set:

```bash
export ORION_PROVIDER="ollama"
export ORION_MODEL="llama3.1"
export OLLAMA_HOST="http://localhost:11434"
```

### Web search is weak or unavailable

Set `TAVILY_API_KEY` for richer search results. Without it, Orion can use DuckDuckGo Lite, which is free but less structured and may be rate-limited.

### The agent is taking too many actions

Lower the tool-round limit:

```toml
[session]
max_tool_rounds = 10
```

Or ask for a plan first:

```text
Inspect the issue and propose a plan. Do not edit files yet.
```

### The model changed files I did not expect

Use Git to inspect and restore selectively:

```bash
git status --short
git diff
git restore path/to/file
```

Do not run broad destructive commands unless you are sure you want to discard work.

## Developer Notes

Common local commands:

```bash
cargo fmt
cargo check
cargo test
cargo build --release
```

Project structure:

- `src/main.rs` - CLI entry point and subcommands.
- `src/cli/` - interactive REPL, command picker, theme, and Markdown rendering.
- `src/agent/` - orchestration loop and middleware.
- `src/llm/` - provider implementations.
- `src/tools/` - filesystem, terminal, grep, Git, browser, and custom tools.
- `src/session/` - SQLite-backed session storage and context helpers.
- `src/skills/` - skill loading and GitHub skill downloader.
- `src/mcp/` - MCP configuration surface.
- `npm/` - generated NPM package output, ignored by Git.

Before committing:

```bash
cargo fmt
cargo check
git status --short
```

## License

Orion is released under the [MIT License](LICENSE).
