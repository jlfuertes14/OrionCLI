use rusqlite::{params, Connection, Result};

fn main() -> Result<()> {
    let path = dirs::home_dir()
        .expect("No home dir")
        .join(".orion")
        .join("history")
        .join("orion.db");

    if !path.exists() {
        println!("No db found at {:?}", path);
        return Ok(());
    }

    let conn = Connection::open(path)?;

    // Get all sessions with generic titles
    let mut stmt = conn.prepare(
        "SELECT id FROM sessions WHERE title = 'Interactive session' OR title = 'New session'",
    )?;
    let session_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();

    println!("Found {} sessions to update.", session_ids.len());

    for session_id in session_ids {
        // Find the first user message for this session
        let mut msg_stmt = conn.prepare("SELECT content FROM messages WHERE session_id = ?1 AND role = 'user' ORDER BY seq ASC LIMIT 1")?;

        let content: Option<String> = msg_stmt
            .query_row(params![session_id], |row| row.get(0))
            .ok();

        if let Some(content) = content {
            let mut new_title = content.trim().to_string();
            if new_title.len() > 40 {
                let mut end_idx = 37;
                while !new_title.is_char_boundary(end_idx) && end_idx > 0 {
                    end_idx -= 1;
                }
                new_title = format!("{}...", &new_title[..end_idx]);
            }

            // Replace newlines with spaces for a single-line title
            new_title = new_title.replace('\n', " ").replace('\r', "");

            conn.execute(
                "UPDATE sessions SET title = ?2 WHERE id = ?1",
                params![session_id, new_title],
            )?;
            println!("Updated {} -> {}", session_id, new_title);
        }
    }

    println!("Done updating session titles.");
    Ok(())
}
