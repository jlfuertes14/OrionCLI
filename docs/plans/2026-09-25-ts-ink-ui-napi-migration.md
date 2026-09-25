# Migration Plan: TypeScript (React Ink) CLI UI + Rust Native Core via NAPI-RS

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate Orion CLI's terminal user interface to a modern, reactive TypeScript + React Ink frontend while keeping all heavy file, git, search, and system execution tools in high-performance Rust via NAPI-RS native bindings.

**Architecture:** 
- **Rust Core (`crates/orion` as `cdylib`):** Implements `napi-rs` bindings to expose file management, git operations, ripgrep search, diff calculation, and tool execution to Node.js in-process with zero IPC latency.
- **TypeScript UI (`packages/cli` or root TS app):** Uses React Ink for declarative terminal UI rendering, handling user prompts, streaming LLM tokens, slash command pickers, interactive diff approvals, and keyboard shortcuts.
- **Bridge:** NAPI-RS auto-generates TypeScript type definitions (`.d.ts`) directly from Rust structs and functions.

**Tech Stack:**
- Rust (Tokio, NAPI-RS, Serde, Similar)
- TypeScript / Node.js (React Ink, Commander, Chalk, Zod)
- Package Manager / Builder: `npm` + `@napi-rs/cli` + `tsx` / `tsup`

---

### Task 1: Initialize Workspace & Configure Cargo for NAPI-RS

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/napi/mod.rs`
- Modify: `package.json`

**Step 1: Update Cargo.toml to support `cdylib` and NAPI-RS dependencies**
Add `[lib]` section with `crate-type = ["cdylib", "rlib"]`, and add `napi` and `napi-derive` dependencies with async/tokio features.

**Step 2: Add NAPI-RS and TypeScript dev dependencies to `package.json`**
Add `@napi-rs/cli`, `typescript`, `@types/node`, `tsx`, `tsup`, `ink`, `react`, `@types/react`, `commander`, `chalk`.

**Step 3: Create `src/lib.rs` exposing modules and NAPI entry point**
Expose `tools`, `config`, `llm`, etc., and define a basic NAPI test function:
```rust
#[napi]
pub fn orion_core_version() -> String {
    "0.1.0".to_string()
}
```

**Step 4: Run build test with `npx @napi-rs/cli build`**
Verify that `index.node` and `index.d.ts` are generated without errors.

---

### Task 2: Expose Core Rust Tools via NAPI-RS Bridge

**Files:**
- Create: `src/napi/tools.rs`
- Modify: `src/napi/mod.rs`
- Create: `tests/napi_tools.test.ts`

**Step 1: Implement NAPI functions for Filesystem operations**
- `napi_list_files(path: String, max_depth: Option<u32>) -> napi::Result<Vec<FileInfo>>`
- `napi_read_file(path: String, offset: Option<u32>, limit: Option<u32>) -> napi::Result<String>`
- `napi_write_file(path: String, content: String) -> napi::Result<()>`
- `napi_edit_file(path: String, old_content: String, new_content: String) -> napi::Result<FileEditResult>`

**Step 2: Implement NAPI functions for Git and Ripgrep**
- `napi_git_status() -> napi::Result<String>`
- `napi_git_diff() -> napi::Result<String>`
- `napi_grep_search(query: String, path: Option<String>) -> napi::Result<Vec<GrepMatch>>`
- `napi_compute_diff(old_text: String, new_text: String) -> napi::Result<Vec<DiffLine>>`

**Step 3: Implement Generic Tool Dispatcher**
- `napi_execute_tool(tool_name: String, args_json: String) -> napi::Result<String>`
Allows any registered tool in `ToolRegistry` to be invoked by name with JSON string arguments asynchronously.

**Step 4: Write TypeScript smoke test**
Run a test using `tsx` that imports `./index.js` and calls `orion_core_version()`, `napi_list_files(".")`, and `napi_git_status()`.

---

### Task 3: Build TypeScript Ink Terminal UI Foundation

**Files:**
- Create: `src/ui/cli.tsx`
- Create: `src/ui/theme.ts`
- Create: `src/ui/components/Header.tsx`
- Create: `src/ui/components/PromptInput.tsx`
- Create: `src/ui/components/StatusBadge.tsx`

**Step 1: Setup Terminal Theme and Vercel-style Aesthetics**
Implement clean dark theme palette, tabular numbers, Unicode symbols, and status indicators in `src/ui/theme.ts`.

**Step 2: Create `PromptInput` Component**
Support interactive typing, slash commands trigger (`/`), multiline input, history navigation with Arrow Up/Down, and `Ctrl+C` handling.

**Step 3: Create `Header` and `StatusBadge` Components**
Render model indicator (e.g. `claude-3-5-sonnet`), working directory, git branch, and session token counters.

---

### Task 4: Port Slash Command Picker & Tool Approval Dialogs to React Ink

**Files:**
- Create: `src/ui/components/CommandPicker.tsx`
- Create: `src/ui/components/ToolApproval.tsx`
- Create: `src/ui/components/DiffViewer.tsx`

**Step 1: Build `CommandPicker` Component**
Replaces the 52KB `command_picker.rs`:
- Filterable command list (`/model`, `/config`, `/clear`, `/history`, `/diff`, `/commit`, `/exit`).
- Keyboard arrow navigation and Enter selection.

**Step 2: Build `ToolApproval` and `DiffViewer` Components**
- When the agent performs file edits or command execution that require confirmation, render an interactive card.
- Displays colored diffs (green additions, red deletions) using lines computed by Rust's `napi_compute_diff`.
- Options: `[Y] Approve`, `[N] Reject`, `[E] Explain`.

---

### Task 5: Build Reactive REPL Chat Loop & Stream Rendering

**Files:**
- Create: `src/ui/components/Repl.tsx`
- Create: `src/ui/components/MessageList.tsx`
- Create: `src/ui/components/StreamingMessage.tsx`
- Create: `src/ui/agent/loop.ts`

**Step 1: Build `StreamingMessage` with live token output**
Stream assistant tokens into an Ink `Text` box without flickering, with syntax highlighting for code blocks.

**Step 2: Connect Agent Loop with Native Rust Tools**
When LLM responds with a tool call (e.g. `read_file` or `edit_file`):
1. Show pending status spinner.
2. If approval needed, prompt via `ToolApproval`.
3. Call native Rust tool via `napi_execute_tool`.
4. Render output and feed result back to the conversation loop.

---

### Task 6: Packaging, Distribution & Verification

**Files:**
- Modify: `package.json`
- Create: `bin/orion.js`
- Test: Full build and interactive execution

**Step 1: Setup build scripts in `package.json`**
```json
{
  "scripts": {
    "build:napi": "napi build --platform --release",
    "build:ui": "tsup src/ui/cli.tsx --format cjs --out-dir dist",
    "build": "npm run build:napi && npm run build:ui",
    "dev": "tsx src/ui/cli.tsx",
    "start": "node bin/orion.js"
  }
}
```

**Step 2: End-to-End Verification**
- Run `npm run dev`
- Test `/help`, `/model`, file listing via native Rust backend, and prompt execution.
- Validate startup time and memory footprint.
