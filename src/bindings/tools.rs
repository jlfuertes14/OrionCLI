use crate::config::Settings;
use crate::tools::{ToolContext, ToolRegistry};
use napi_derive::napi;
use serde_json::{json, Value};
use similar::{ChangeTag, TextDiff};

fn get_context() -> ToolContext {
    let settings = Settings::load().unwrap_or_default();
    ToolContext { settings }
}

#[napi(object)]
pub struct DiffLine {
    pub tag: String,
    pub text: String,
}

#[napi]
pub fn napi_compute_diff(old_text: String, new_text: String) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(&old_text, &new_text);
    let mut lines = Vec::new();
    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let tag = match change.tag() {
                ChangeTag::Delete => "delete",
                ChangeTag::Insert => "insert",
                ChangeTag::Equal => "equal",
            };
            lines.push(DiffLine {
                tag: tag.to_string(),
                text: change.value().to_string(),
            });
        }
    }
    lines
}

#[napi]
pub fn napi_get_tool_schemas() -> napi::Result<String> {
    let registry = ToolRegistry::new();
    let schemas = registry.get_openai_schemas();
    serde_json::to_string(&schemas)
        .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
}

#[napi]
pub async fn napi_execute_tool(name: String, args_json: String) -> napi::Result<String> {
    let registry = ToolRegistry::new();
    let tool = registry
        .get(&name)
        .ok_or_else(|| napi::Error::from_reason(format!("Tool '{}' not found", name)))?;

    let args: Value = if args_json.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&args_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid JSON arguments: {}", e)))?
    };

    let ctx = get_context();
    let result = tool
        .execute(args, &ctx)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Tool execution error: {}", e)))?;

    Ok(result)
}

#[napi]
pub async fn napi_read_file(path: String) -> napi::Result<String> {
    napi_execute_tool("read_file".into(), json!({ "path": path }).to_string()).await
}

#[napi]
pub async fn napi_write_file(path: String, content: String) -> napi::Result<String> {
    napi_execute_tool(
        "write_file".into(),
        json!({ "path": path, "content": content }).to_string(),
    )
    .await
}

#[napi]
pub async fn napi_list_directory(path: String) -> napi::Result<String> {
    napi_execute_tool("list_directory".into(), json!({ "path": path }).to_string()).await
}

#[napi]
pub async fn napi_git_status() -> napi::Result<String> {
    napi_execute_tool("git_status".into(), "{}".into()).await
}

#[napi]
pub async fn napi_git_diff() -> napi::Result<String> {
    napi_execute_tool("git_diff".into(), "{}".into()).await
}

#[napi]
pub async fn napi_grep_search(query: String, path: Option<String>) -> napi::Result<String> {
    let mut args = json!({ "query": query });
    if let Some(p) = path {
        args["path"] = json!(p);
    }
    napi_execute_tool("grep".into(), args.to_string()).await
}

#[napi]
pub async fn napi_run_command(command: String) -> napi::Result<String> {
    napi_execute_tool(
        "run_command".into(),
        json!({ "command": command }).to_string(),
    )
    .await
}

#[napi(object)]
pub struct SessionItem {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub updated_at: String,
}

#[napi]
pub fn napi_list_sessions(limit: Option<u32>) -> napi::Result<Vec<SessionItem>> {
    let store = crate::session::SessionStore::open_default()
        .map_err(|e| napi::Error::from_reason(format!("Session DB error: {}", e)))?;
    let list = store
        .list_sessions(limit.unwrap_or(25) as usize)
        .map_err(|e| napi::Error::from_reason(format!("List sessions error: {}", e)))?;
    Ok(list
        .into_iter()
        .map(|s| SessionItem {
            id: s.id,
            title: s.title,
            provider: s.provider,
            model: s.model,
            updated_at: s.updated_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect())
}

#[napi]
pub fn napi_delete_session(session_id: String) -> napi::Result<bool> {
    let store = crate::session::SessionStore::open_default()
        .map_err(|e| napi::Error::from_reason(format!("Session DB error: {}", e)))?;
    store
        .delete_session(&session_id)
        .map_err(|e| napi::Error::from_reason(format!("Delete error: {}", e)))?;
    Ok(true)
}
