use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::config::Settings;
use crate::llm::provider::Message;
use crate::sandbox::validate_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub workspace: String,
    pub provider: String,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub seq: i64,
    pub role: String,
    pub content: String,
    pub message_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingContext {
    pub label: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLimits {
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: usize,
    #[serde(default = "default_max_total_bytes")]
    pub max_total_bytes: usize,
}

impl Default for ContextLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: default_max_file_bytes(),
            max_total_bytes: default_max_total_bytes(),
        }
    }
}

pub fn default_max_file_bytes() -> usize {
    100 * 1024
}

pub fn default_max_total_bytes() -> usize {
    500 * 1024
}

pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    pub fn open_default() -> Result<Self> {
        let path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Failed to resolve home directory"))?
            .join(".orion")
            .join("history")
            .join("orion.db");
        Self::open(path)
    }

    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                workspace TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                message_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session_seq
            ON messages(session_id, seq);
            "#,
        )?;

        // Safe migrations for new stats columns
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN total_input_tokens INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN total_output_tokens INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN total_cost_usd REAL DEFAULT 0.0",
            [],
        );

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow!("Session store lock poisoned"))
    }

    pub fn create_session(&self, settings: &Settings, title: Option<&str>) -> Result<SessionMeta> {
        let conn = self.conn()?;
        let now = Utc::now();
        let meta = SessionMeta {
            id: Uuid::new_v4().to_string(),
            title: title
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("New session")
                .to_string(),
            workspace: settings.workspace_dir.to_string_lossy().to_string(),
            provider: settings.active_provider.clone(),
            model: settings.active_model.clone(),
            created_at: now,
            updated_at: now,
        };

        conn.execute(
            "INSERT INTO sessions (id, title, workspace, provider, model, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                meta.id,
                meta.title,
                meta.workspace,
                meta.provider,
                meta.model,
                meta.created_at.to_rfc3339(),
                meta.updated_at.to_rfc3339()
            ],
        )?;

        Ok(meta)
    }

    pub fn get_session(&self, id: &str) -> Result<Option<SessionMeta>> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT id, title, workspace, provider, model, created_at, updated_at
                 FROM sessions WHERE id = ?1",
            params![id],
            row_to_session_meta,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<SessionMeta>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, workspace, provider, model, created_at, updated_at
             FROM sessions ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_session_meta)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn update_session_model(&self, id: &str, provider: &str, model: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE sessions SET provider = ?2, model = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, provider, model, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn update_session_title(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, title, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_session_stats(
        &self,
        id: &str,
        input: usize,
        output: usize,
        cost: f64,
    ) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE sessions SET 
                total_input_tokens = total_input_tokens + ?2,
                total_output_tokens = total_output_tokens + ?3,
                total_cost_usd = total_cost_usd + ?4,
                updated_at = ?5
             WHERE id = ?1",
            params![
                id,
                input as i64,
                output as i64,
                cost,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get_session_stats(&self, id: &str) -> Result<(usize, usize, f64)> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT total_input_tokens, total_output_tokens, total_cost_usd FROM sessions WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        ).map_err(Into::into)
    }

    pub fn get_all_time_stats(&self) -> Result<(usize, usize, f64)> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(total_input_tokens), 0), COALESCE(SUM(total_output_tokens), 0), COALESCE(SUM(total_cost_usd), 0.0) FROM sessions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        ).map_err(Into::into)
    }

    pub fn append_message(&self, session_id: &str, message: &Message) -> Result<()> {
        let conn = self.conn()?;
        let seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), -1) + 1 FROM messages WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let now = Utc::now();
        let message_json = serde_json::to_string(message)?;
        conn.execute(
            "INSERT INTO messages (session_id, seq, role, content, message_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id,
                seq,
                message.role,
                message.content,
                message_json,
                now.to_rfc3339()
            ],
        )?;
        conn.execute(
            "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
            params![session_id, now.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn load_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT message_json FROM messages WHERE session_id = ?1 ORDER BY seq ASC")?;
        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(serde_json::from_str(&row?)?);
        }
        Ok(messages)
    }

    pub fn load_stored_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, seq, role, content, message_json, created_at
             FROM messages WHERE session_id = ?1 ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            let created_at: String = row.get(6)?;
            let message_json: String = row.get(5)?;
            Ok(StoredMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                seq: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                message_json: serde_json::from_str(&message_json).unwrap_or(Value::Null),
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

fn row_to_session_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionMeta> {
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    Ok(SessionMeta {
        id: row.get(0)?,
        title: row.get(1)?,
        workspace: row.get(2)?,
        provider: row.get(3)?,
        model: row.get(4)?,
        created_at: parse_rfc3339_or_now(&created_at),
        updated_at: parse_rfc3339_or_now(&updated_at),
    })
}

fn parse_rfc3339_or_now(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub fn gather_path_context(path_str: &str, settings: &Settings) -> Result<PendingContext> {
    let root = validate_path(path_str, settings)?;
    let limits = &settings.context_limits;
    let mut total_bytes = 0usize;
    let mut included = Vec::new();
    let mut skipped = Vec::new();

    if root.is_file() {
        ingest_file(
            &root,
            settings,
            limits,
            &mut total_bytes,
            &mut included,
            &mut skipped,
        )?;
    } else if root.is_dir() {
        ingest_dir(
            &root,
            settings,
            limits,
            &mut total_bytes,
            &mut included,
            &mut skipped,
        )?;
    } else {
        return Err(anyhow!("Path does not exist: {}", path_str));
    }

    let mut content = String::new();
    content.push_str(&format!("Context loaded from: {}\n\n", path_str));
    for (path, text) in included {
        content.push_str(&format!(
            "--- FILE: {} ---\n",
            display_workspace_path(&path, settings)
        ));
        content.push_str(&text);
        if !text.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
    }

    if !skipped.is_empty() {
        content.push_str("--- SKIPPED ---\n");
        for item in skipped {
            content.push_str(&format!("- {}\n", item));
        }
    }

    Ok(PendingContext {
        label: format!("files: {}", path_str),
        content,
    })
}

pub fn gather_git_context(settings: &Settings) -> Result<PendingContext> {
    let status = run_git(&settings.workspace_dir, &["status", "--short"])?;
    let diff = run_git(&settings.workspace_dir, &["diff"])?;
    Ok(PendingContext {
        label: "git diff".to_string(),
        content: format!(
            "Git context for {}\n\n--- git status --short ---\n{}\n\n--- git diff ---\n{}\n",
            settings.workspace_dir.display(),
            if status.trim().is_empty() {
                "(clean)"
            } else {
                status.trim_end()
            },
            if diff.trim().is_empty() {
                "(no diff)"
            } else {
                diff.trim_end()
            }
        ),
    })
}

fn ingest_dir(
    dir: &Path,
    settings: &Settings,
    limits: &ContextLimits,
    total_bytes: &mut usize,
    included: &mut Vec<(PathBuf, String)>,
    skipped: &mut Vec<String>,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_name(&name) {
            skipped.push(format!(
                "{} (ignored)",
                display_workspace_path(&path, settings)
            ));
            continue;
        }
        if path.is_dir() {
            ingest_dir(&path, settings, limits, total_bytes, included, skipped)?;
        } else if path.is_file() {
            ingest_file(&path, settings, limits, total_bytes, included, skipped)?;
        }
        if *total_bytes >= limits.max_total_bytes {
            break;
        }
    }
    Ok(())
}

fn ingest_file(
    path: &Path,
    settings: &Settings,
    limits: &ContextLimits,
    total_bytes: &mut usize,
    included: &mut Vec<(PathBuf, String)>,
    skipped: &mut Vec<String>,
) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let len = metadata.len() as usize;
    let display = display_workspace_path(path, settings);
    if len > limits.max_file_bytes {
        skipped.push(format!(
            "{} ({} bytes exceeds per-file limit)",
            display, len
        ));
        return Ok(());
    }
    if *total_bytes + len > limits.max_total_bytes {
        skipped.push(format!("{} (total context limit reached)", display));
        return Ok(());
    }

    let bytes = fs::read(path)?;
    if looks_binary(&bytes) {
        skipped.push(format!("{} (binary)", display));
        return Ok(());
    }
    let text = String::from_utf8(bytes).context("Failed to decode file as UTF-8")?;
    *total_bytes += text.len();
    included.push((path.to_path_buf(), text));
    Ok(())
}

fn should_skip_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "target_cli" | "node_modules" | ".orion" | ".DS_Store"
    ) || name.ends_with(".exe")
        || name.ends_with(".png")
        || name.ends_with(".jpg")
        || name.ends_with(".jpeg")
        || name.ends_with(".gif")
        || name.ends_with(".webp")
        || name.ends_with(".zip")
        || name.ends_with(".tar")
        || name.ends_with(".gz")
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|b| *b == 0)
}

fn display_workspace_path(path: &Path, settings: &Settings) -> String {
    path.strip_prefix(&settings.workspace_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn run_git(workspace: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Ok(format!(
            "git {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_loads_messages() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let store = SessionStore::open(dir.path().join("orion.db"))?;
        let settings = Settings::default();
        let meta = store.create_session(&settings, Some("test"))?;
        let msg = Message {
            role: "user".to_string(),
            content: "hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            images: None,
        };
        store.append_message(&meta.id, &msg)?;
        let loaded = store.load_messages(&meta.id)?;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "hello");
        Ok(())
    }
}
