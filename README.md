# Orion 🌌

[![npm version](https://img.shields.io/npm/v/orion-agent-cli.svg)](https://www.npmjs.com/package/orion-agent-cli)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Orion** is a high-performance, autonomous, agentic coding assistant built in Rust. It runs entirely on your command line and acts as an intelligent pair programmer capable of analyzing code, managing files, reading web pages, running terminal commands, and executing tasks autonomously.

---

## Key Features

### 🌌 High-Performance Core
* **Ultra-low latency & footprint:** Built in Rust with asynchronous execution (`tokio`).
* **Multi-Provider Fallback:** Native support for Anthropic, OpenAI, Gemini, OpenRouter, Mistral, and local models (Ollama/LlamaEdge), with automatic, configurable fallbacks if an API provider goes down.

### 🛡️ Core Safety Harness (Enterprise Grade)
* **Context Sliding Window:** Automatically estimates tokens and prunes the oldest conversation messages when approaching the 60,000 token limit—preserving the system prompt and preventing context-overflow crashes.
* **Failure Circuit Breaker:** Detects recursive error patterns and automatically aborts execution if tools fail consecutively 3 times in a row, saving your token budget.
* **Command Sandboxing:** Optionally execute all terminal commands inside a Docker container sandbox instead of your host machine to protect your system.

### 🌐 Browser & Web Tools
* **Interactive Setup Search:** Detects search settings on first run. Supports **Tavily Search API** (rich structured context) and automatically falls back to **DuckDuckGo Lite** (free, zero keys) with helpful setup warnings.
* **Browser Reader:** Extracts clean, readable Markdown from arbitrary webpage URLs directly into the context window.

### 💼 Session & State Management
* **SQLite Session Database:** Tracks all histories, metadata, and token budgets inside a central `~/.orion/orion.db` store.
* **Cost tracking:** Enforces strict dollar budgets per session to prevent accidental runaway agent costs.

### ⚡ CLI Layers & Middleware
* **Interactive REPL:** Rich terminal environment with autocomplete, command history, syntax highlighting, and theme support.
* **Headless execution:** Run single commands or pipe inputs via standard input for CI/CD usage:
  ```bash
  orion execute "Verify the tests in src/main.rs"
  cat prompt.txt | orion execute --stdin
  ```
* **Outgoing Middleware:** Integrated `PIIScrubber` middleware automatically intercepts and redacts emails and phone numbers before shipping payloads to LLM providers.

### 🔌 Dynamic Extensibility (Plugins)
* Create custom tool integrations entirely from the filesystem. Simply place a `.json` schema accompanied by a script in `~/.orion/tools/`. Orion will dynamically register the tool schema and pass the LLM's arguments to your script via `stdin` as JSON.

---

## Quick Start

### Installation

#### Via NPM (No dependencies required)
```bash
npm install -g orion-agent-cli
```
*Note: This downloads the pre-compiled native machine-code binary for your OS (Windows, Linux, macOS) out of the box.*

#### Via Cargo (Rust toolchain)
```bash
cargo install orion
```

---

## Configuration

Orion stores all global data under `~/.orion/` (`C:\Users\Username\.orion\` on Windows).

To configure settings, edit `~/.orion/config.toml` or set environment variables:
```toml
active_provider = "anthropic"
active_model = "claude-3-5-sonnet"

[providers.anthropic]
api_key = "your-api-key"
```

---

## Commands

* `orion` - Launch interactive REPL.
* `orion execute "<prompt>"` - Headless single execution.
* `orion config --show` - View active configurations.
* `orion sessions list` - List recent sessions.
* `orion sessions show <id>` - View session transcript.

---

## License

Orion is released under the [MIT License](LICENSE).
