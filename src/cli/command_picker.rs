use std::io::{self, Write};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, SetBackgroundColor, Attribute, SetAttribute},
    terminal::{self, ClearType},
};

#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command { name: "/help",    icon: "💡", description: "Show all available commands" },
    Command { name: "/model",   icon: "⚡", description: "Switch AI provider and model  e.g. /model anthropic:claude-opus-4-5" },
    Command { name: "/clear",   icon: "🗑 ", description: "Clear the terminal screen" },
    Command { name: "/exit",    icon: "✕ ", description: "Quit Orion" },
];

/// Run the fuzzy command picker. Returns the selected command string or None if cancelled.
pub fn run_picker() -> io::Result<Option<String>> {
    let mut query = String::new();
    let mut selected: usize = 0;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = loop {
        let filtered = filter_commands(&query);
        // clamp selection
        if selected >= filtered.len() && !filtered.is_empty() {
            selected = filtered.len() - 1;
        }

        render(&mut stdout, &query, &filtered, selected)?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                // Cancel
                (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break None;
                }
                // Confirm selection
                (KeyCode::Enter, _) => {
                    if let Some(cmd) = filtered.get(selected) {
                        break Some(cmd.name.to_string());
                    } else {
                        // user typed a full custom command
                        let q = format!("/{}", query.trim_start_matches('/'));
                        break Some(q);
                    }
                }
                // Navigate down
                (KeyCode::Down, _) | (KeyCode::Tab, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + 1) % filtered.len();
                    }
                }
                // Navigate up
                (KeyCode::Up, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + filtered.len() - 1) % filtered.len();
                    }
                }
                // Backspace
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                    // If query is now empty, exit picker (user deleted the /)
                    if query.is_empty() {
                        break None;
                    }
                }
                // Typing
                (KeyCode::Char(c), _) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        }
    };

    // Cleanup: clear the picker area
    terminal::disable_raw_mode()?;
    clear_picker(&mut stdout, COMMANDS.len() + 4)?;

    Ok(result)
}

fn filter_commands(query: &str) -> Vec<&'static Command> {
    let q = query.trim_start_matches('/').to_lowercase();
    if q.is_empty() {
        return COMMANDS.iter().collect();
    }
    COMMANDS
        .iter()
        .filter(|cmd| {
            cmd.name.to_lowercase().contains(&q)
                || cmd.description.to_lowercase().contains(&q)
        })
        .collect()
}

fn render(
    stdout: &mut io::Stdout,
    query: &str,
    filtered: &[&Command],
    selected: usize,
) -> io::Result<()> {
    // Move cursor to start of picker block and clear downward
    execute!(stdout, cursor::SavePosition)?;
    execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

    let max_name_len = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(10);
    let total_width = 56usize;

    // ── Top border ─────────────────────────────────────────────
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
        Print(format!("╭{}╮\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    if filtered.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::Rgb { r: 107, g: 114, b: 128 }),
            Print(format!("│  {:<width$}│\r\n", "No matching commands", width = total_width - 2)),
            ResetColor,
        )?;
    } else {
        for (i, cmd) in filtered.iter().enumerate() {
            let is_selected = i == selected;

            let name_col = format!("{} {:<width$}", cmd.icon, cmd.name, width = max_name_len);
            let desc_col_max = total_width - max_name_len - 6;
            let desc = if cmd.description.len() > desc_col_max {
                format!("{}…", &cmd.description[..desc_col_max.saturating_sub(1)])
            } else {
                cmd.description.to_string()
            };

            if is_selected {
                // Selected row: accent left border + highlighted background
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
                    Print("│"),
                    SetForegroundColor(Color::Rgb { r: 99, g: 179, b: 237 }),
                    Print("▌"),
                    SetBackgroundColor(Color::Rgb { r: 17, g: 24, b: 39 }),
                    SetForegroundColor(Color::Rgb { r: 99, g: 179, b: 237 }),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" {:<width$}", name_col, width = max_name_len + 4)),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::Rgb { r: 209, g: 213, b: 219 }),
                    Print(format!("{:<width$}", desc, width = desc_col_max)),
                    ResetColor,
                    SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
                    Print("│\r\n"),
                    ResetColor,
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
                    Print("│ "),
                    SetForegroundColor(Color::Rgb { r: 94, g: 234, b: 212 }),
                    Print(format!("{:<width$}", name_col, width = max_name_len + 4)),
                    SetForegroundColor(Color::Rgb { r: 107, g: 114, b: 128 }),
                    Print(format!("{:<width$}", desc, width = desc_col_max)),
                    SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
                    Print("│\r\n"),
                    ResetColor,
                )?;
            }
        }
    }

    // ── Bottom border ───────────────────────────────────────────
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
        Print(format!("├{}┤\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    // ── Input row ───────────────────────────────────────────────
    let display_query = format!("/{}", query.trim_start_matches('/'));
    let hint = if filtered.len() == 1 {
        format!("↵ confirm  ↑↓ navigate  esc cancel")
    } else {
        format!("↑↓ navigate  ↵ select  esc cancel")
    };

    execute!(
        stdout,
        SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
        Print("│"),
        SetForegroundColor(Color::Rgb { r: 99, g: 179, b: 237 }),
        Print(format!(" {:}", display_query)),
        SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
        // fill the remainder of the row
        Print(format!(
            "{:>width$}",
            hint,
            width = total_width - display_query.len() - 1
        )),
        Print("│\r\n"),
        ResetColor,
    )?;

    // ── Close border ────────────────────────────────────────────
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb { r: 55, g: 65, b: 81 }),
        Print(format!("╰{}╯\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    stdout.flush()?;
    Ok(())
}

fn clear_picker(stdout: &mut io::Stdout, lines: usize) -> io::Result<()> {
    // Move up `lines` rows and clear from cursor down
    execute!(stdout, cursor::MoveUp(lines as u16))?;
    execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
    stdout.flush()?;
    Ok(())
}
