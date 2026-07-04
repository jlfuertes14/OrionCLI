use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{self, ClearType},
};
use std::io::{self, Write};

#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "/help",
        icon: "💡",
        description: "Show all available commands",
    },
    Command {
        name: "/model",
        icon: "⚡",
        description: "Switch AI provider and model  e.g. /model anthropic:claude-opus-4-5",
    },
    Command {
        name: "/skill",
        icon: "🛠 ",
        description: "Load/list skills: /skill load <name> or /skill list",
    },
    Command {
        name: "/vision",
        icon: "🖼 ",
        description: "Load local image: /vision <file_path>",
    },
    Command {
        name: "/screenshot",
        icon: "📸",
        description: "Capture desktop screenshot and load",
    },
    Command {
        name: "/session",
        icon: "💬",
        description:
            "List/resume/delete sessions: /session list, /session resume <id>, /session delete <id>",
    },
    Command {
        name: "/web",
        icon: "🌐",
        description: "Scrape a web page and inject into context: /web <url>",
    },
    Command {
        name: "/search",
        icon: "🔍",
        description: "Search the web and inject results: /search <query>",
    },
    Command {
        name: "/commit",
        icon: "📝",
        description: "Generate a commit message for staged changes",
    },
    Command {
        name: "/pr",
        icon: "🚀",
        description: "Generate a pull request description for unmerged changes",
    },
    Command {
        name: "/stats",
        icon: "📊",
        description: "View usage statistics and estimated costs",
    },
    Command {
        name: "/clear",
        icon: "🗑 ",
        description: "Clear the terminal screen",
    },
    Command {
        name: "/exit",
        icon: "✕ ",
        description: "Quit Orion",
    },
];

/// Run the fuzzy command picker. Returns the selected command string or None if cancelled.
pub fn run_picker() -> io::Result<Option<String>> {
    let mut query = String::new();
    let mut selected: usize = 0;
    let mut lines_rendered: u16 = 0;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = loop {
        let filtered = filter_commands(&query);
        // clamp selection
        if selected >= filtered.len() && !filtered.is_empty() {
            selected = filtered.len() - 1;
        }

        if lines_rendered > 0 {
            execute!(stdout, cursor::MoveUp(lines_rendered))?;
        }
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

        lines_rendered = render(&mut stdout, &query, &filtered, selected)?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
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
        } else {
            // Not a key event, wait for next event
            continue;
        }
    };

    // Cleanup: clear the picker area
    terminal::disable_raw_mode()?;
    if lines_rendered > 0 {
        execute!(stdout, cursor::MoveUp(lines_rendered))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
        stdout.flush()?;
    }

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
            cmd.name.to_lowercase().contains(&q) || cmd.description.to_lowercase().contains(&q)
        })
        .collect()
}

fn render(
    stdout: &mut io::Stdout,
    query: &str,
    filtered: &[&Command],
    selected: usize,
) -> io::Result<u16> {
    let max_display = 10;
    let start_idx = if filtered.len() <= max_display {
        0
    } else if selected < max_display / 2 {
        0
    } else if selected + max_display / 2 >= filtered.len() {
        filtered.len() - max_display
    } else {
        selected - max_display / 2
    };
    let end_idx = std::cmp::min(start_idx + max_display, filtered.len());
    let display_items = if filtered.is_empty() {
        &[]
    } else {
        &filtered[start_idx..end_idx]
    };

    let lines_rendered = 4 + if display_items.is_empty() {
        1
    } else {
        display_items.len()
    } as u16;

    let max_name_len = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(10);
    let total_width = 56usize;

    // ── Top border ─────────────────────────────────────────────
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print(format!("╭{}╮\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    if filtered.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 107,
                g: 114,
                b: 128
            }),
            Print(format!(
                "│  {:<width$}│\r\n",
                "No matching commands",
                width = total_width - 2
            )),
            ResetColor,
        )?;
    } else {
        for (idx, cmd) in display_items.iter().enumerate() {
            let actual_idx = start_idx + idx;
            let is_selected = actual_idx == selected;

            let name_col = format!("{} {:<width$}", cmd.icon, cmd.name, width = max_name_len);
            let desc_col_max = total_width - max_name_len - 6;
            let desc = if cmd.description.chars().count() > desc_col_max {
                let truncated: String = cmd
                    .description
                    .chars()
                    .take(desc_col_max.saturating_sub(1))
                    .collect();
                format!("{}…", truncated)
            } else {
                cmd.description.to_string()
            };

            if is_selected {
                // Selected row: accent left border + highlighted background
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│"),
                    SetForegroundColor(Color::Rgb {
                        r: 99,
                        g: 179,
                        b: 237
                    }),
                    Print("▌"),
                    SetBackgroundColor(Color::Rgb {
                        r: 17,
                        g: 24,
                        b: 39
                    }),
                    SetForegroundColor(Color::Rgb {
                        r: 99,
                        g: 179,
                        b: 237
                    }),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" {:<width$}", name_col, width = max_name_len + 4)),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::Rgb {
                        r: 209,
                        g: 213,
                        b: 219
                    }),
                    Print(format!("{:<width$}", desc, width = desc_col_max)),
                    ResetColor,
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│\r\n"),
                    ResetColor,
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│ "),
                    SetForegroundColor(Color::Rgb {
                        r: 94,
                        g: 234,
                        b: 212
                    }),
                    Print(format!("{:<width$}", name_col, width = max_name_len + 4)),
                    SetForegroundColor(Color::Rgb {
                        r: 107,
                        g: 114,
                        b: 128
                    }),
                    Print(format!("{:<width$}", desc, width = desc_col_max)),
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│\r\n"),
                    ResetColor,
                )?;
            }
        }
    }

    // ── Bottom border ───────────────────────────────────────────
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
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
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print("│"),
        SetForegroundColor(Color::Rgb {
            r: 99,
            g: 179,
            b: 237
        }),
        Print(format!(" {:}", display_query)),
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
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
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print(format!("╰{}╯\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    stdout.flush()?;
    Ok(lines_rendered as u16)
}

pub fn prompt_for_input(prompt_text: &str) -> io::Result<Option<String>> {
    let mut input = String::new();
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = loop {
        execute!(
            stdout,
            Print("\r"),
            terminal::Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Rgb {
                r: 99,
                g: 179,
                b: 237
            }),
            Print(prompt_text),
            ResetColor,
            Print(&input)
        )?;
        stdout.flush()?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break None;
                }
                (KeyCode::Enter, _) => {
                    if !input.is_empty() {
                        break Some(input);
                    }
                }
                (KeyCode::Backspace, _) => {
                    input.pop();
                }
                (KeyCode::Char(c), _) => {
                    input.push(c);
                }
                _ => {}
            }
        }
    };

    terminal::disable_raw_mode()?;
    execute!(stdout, Print("\r\n"))?;
    stdout.flush()?;

    Ok(result)
}

pub fn run_provider_picker() -> io::Result<Option<String>> {
    let providers = vec![
        Command {
            name: "anthropic",
            description: "Anthropic (Claude series)",
            icon: "🧠",
        },
        Command {
            name: "openai",
            description: "OpenAI (GPT series)",
            icon: "🧠",
        },
        Command {
            name: "gemini",
            description: "Google Gemini",
            icon: "🧠",
        },
        Command {
            name: "openrouter",
            description: "OpenRouter (Router for various APIs)",
            icon: "🌐",
        },
        Command {
            name: "cloudflare",
            description: "Cloudflare Workers AI",
            icon: "☁️",
        },
        Command {
            name: "ollama",
            description: "Local Ollama instance",
            icon: "🦙",
        },
        Command {
            name: "groq",
            description: "Groq (Ultra-fast inference, Generous Free Tier)",
            icon: "⚡",
        },
        Command {
            name: "mistral",
            description: "Mistral AI (Has Free Tier)",
            icon: "🌪 ",
        },
    ];
    run_generic_picker("Select LLM Provider", &providers)
}

pub fn run_model_picker(provider: &str) -> io::Result<Option<String>> {
    let mut models = vec![];

    match provider {
        "cloudflare" => {
            models.push(Command {
                name: "@cf/meta/llama-3-8b-instruct",
                description: "Llama 3 8B Instruct",
                icon: "🦙",
            });
            models.push(Command {
                name: "@cf/meta/llama-3.1-8b-instruct",
                description: "Llama 3.1 8B Instruct",
                icon: "🦙",
            });
            models.push(Command {
                name: "@cf/qwen/qwen1.5-14b-chat",
                description: "Qwen 1.5 14B Chat",
                icon: "🧠",
            });
            models.push(Command {
                name: "@cf/deepseek-ai/deepseek-coder-6.7b-instruct",
                description: "Deepseek Coder 6.7B",
                icon: "💻",
            });
            models.push(Command {
                name: "@cf/zai-org/glm-5.2",
                description: "GLM 5.2 Chat Model",
                icon: "🧠",
            });
        }
        "anthropic" => {
            models.push(Command {
                name: "claude-fable-5",
                description: "Deepest reasoning model",
                icon: "🧠",
            });
            models.push(Command {
                name: "claude-mythos-5",
                description: "Restricted, ultra-secure tier",
                icon: "🛡 ",
            });
            models.push(Command {
                name: "claude-opus-4.8",
                description: "Stable enterprise flagship",
                icon: "🏭",
            });
            models.push(Command {
                name: "claude-sonnet-5",
                description: "Highly autonomous 'agentic' model",
                icon: "⚡",
            });
            models.push(Command {
                name: "claude-sonnet-4.6",
                description: "Excellent everyday production engine",
                icon: "🏭",
            });
            models.push(Command {
                name: "claude-haiku-4.5",
                description: "Fastest, most cost-effective",
                icon: "🏎 ",
            });
        }
        "openai" => {
            models.push(Command {
                name: "gpt-5.5-pro",
                description: "Premium, highly precise flagship",
                icon: "💎",
            });
            models.push(Command {
                name: "gpt-5.5",
                description: "Next-generation flagship",
                icon: "🧠",
            });
            models.push(Command {
                name: "gpt-5.4-pro",
                description: "Cost-efficient professional capability",
                icon: "💼",
            });
            models.push(Command {
                name: "gpt-5.4-mini",
                description: "Highly performant lightweight model",
                icon: "⚡",
            });
            models.push(Command {
                name: "gpt-5.4-nano",
                description: "Ultra-cheap rapid model",
                icon: "🏎 ",
            });
        }
        "gemini" => {
            models.push(Command {
                name: "gemini-3.5-flash",
                description: "Flagship default production engine",
                icon: "⚡",
            });
            models.push(Command {
                name: "gemini-3.1-pro-preview",
                description: "Highest-capability thinking model",
                icon: "🧠",
            });
            models.push(Command {
                name: "gemini-3.1-flash-lite",
                description: "Scaled workhorse",
                icon: "🐎",
            });
            models.push(Command {
                name: "gemini-2.5-pro",
                description: "Stable premium reasoning model",
                icon: "🏭",
            });
            models.push(Command {
                name: "gemini-2.5-flash",
                description: "Reliable stable runtime",
                icon: "🏭",
            });
        }
        "openrouter" => {
            models.push(Command {
                name: "anthropic/claude-3.5-sonnet",
                description: "Claude 3.5 Sonnet",
                icon: "⚡",
            });
            models.push(Command {
                name: "openai/gpt-4o",
                description: "GPT-4o",
                icon: "🧠",
            });
            models.push(Command {
                name: "google/gemini-pro-1.5",
                description: "Gemini Pro 1.5",
                icon: "🧠",
            });
            models.push(Command {
                name: "meta-llama/llama-3.1-405b-instruct",
                description: "Llama 3.1 405B",
                icon: "🦙",
            });
        }
        "ollama" => {
            models.push(Command {
                name: "llama3.2",
                description: "Llama 3.2",
                icon: "🦙",
            });
            models.push(Command {
                name: "llama3.1",
                description: "Llama 3.1",
                icon: "🦙",
            });
            models.push(Command {
                name: "phi3",
                description: "Phi-3",
                icon: "🧠",
            });
        }
        "groq" => {
            models.push(Command {
                name: "openai/gpt-oss-120b",
                description: "OpenAI GPT OSS 120B",
                icon: "🧠",
            });
            models.push(Command {
                name: "openai/gpt-oss-20b",
                description: "OpenAI GPT OSS 20B",
                icon: "⚡",
            });
            models.push(Command {
                name: "llama-3.3-70b-versatile",
                description: "Meta Llama 3.3 70B",
                icon: "🦙",
            });
            models.push(Command {
                name: "llama-3.1-8b-instant",
                description: "Meta Llama 3.1 8B",
                icon: "🦙",
            });
            models.push(Command {
                name: "whisper-large-v3",
                description: "Whisper Audio",
                icon: "🎙 ",
            });
        }
        "mistral" => {
            models.push(Command {
                name: "mistral-large-3",
                description: "Flagship general-purpose model",
                icon: "🧠",
            });
            models.push(Command {
                name: "mistral-medium-3.5",
                description: "Frontier-class multimodal model",
                icon: "⚖ ",
            });
            models.push(Command {
                name: "mistral-small-4",
                description: "Efficient hybrid model",
                icon: "⚡",
            });
            models.push(Command {
                name: "codestral-latest",
                description: "Specialized for code generation",
                icon: "💻",
            });
        }
        _ => {}
    }

    models.push(Command {
        name: "Custom...",
        description: "Type a custom model name",
        icon: "✏ ",
    });

    run_generic_picker(&format!("Select Model ({})", provider), &models)
}

fn run_generic_picker(title: &str, items: &[Command]) -> io::Result<Option<String>> {
    let mut query = String::new();
    let mut selected: usize = 0;
    let mut lines_rendered: u16 = 0;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = loop {
        let q = query.to_lowercase();
        let filtered: Vec<&Command> = if q.is_empty() {
            items.iter().collect()
        } else {
            items
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&q) || c.description.to_lowercase().contains(&q)
                })
                .collect()
        };

        if selected >= filtered.len() && !filtered.is_empty() {
            selected = filtered.len() - 1;
        }

        if lines_rendered > 0 {
            execute!(stdout, cursor::MoveUp(lines_rendered))?;
        }
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

        // Inline generic render logic
        let max_display = 10;
        let start_idx = if filtered.len() <= max_display {
            0
        } else if selected < max_display / 2 {
            0
        } else if selected + max_display / 2 >= filtered.len() {
            filtered.len() - max_display
        } else {
            selected - max_display / 2
        };
        let end_idx = std::cmp::min(start_idx + max_display, filtered.len());
        let display_items = if filtered.is_empty() {
            &[]
        } else {
            &filtered[start_idx..end_idx]
        };

        let total_width = 56usize;
        lines_rendered = 6 + if display_items.is_empty() {
            1
        } else {
            display_items.len()
        } as u16;

        let max_name_len = items.iter().map(|c| c.name.len()).max().unwrap_or(10);

        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!("╭{}╮\r\n", "─".repeat(total_width))),
            Print("│ "),
            SetForegroundColor(Color::Rgb {
                r: 252,
                g: 211,
                b: 77
            }),
            SetAttribute(Attribute::Bold),
            Print(format!("{:<width$}", title, width = total_width - 2)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print("│\r\n"),
            Print(format!("├{}┤\r\n", "─".repeat(total_width))),
            ResetColor,
        )?;

        if filtered.is_empty() {
            execute!(
                stdout,
                SetForegroundColor(Color::Rgb {
                    r: 107,
                    g: 114,
                    b: 128
                }),
                Print(format!(
                    "│  {:<width$}│\r\n",
                    "No matches",
                    width = total_width - 2
                )),
                ResetColor,
            )?;
        } else {
            for (idx, cmd) in display_items.iter().enumerate() {
                let actual_idx = start_idx + idx;
                let is_selected = actual_idx == selected;

                let name_col = format!("{} {:<width$}", cmd.icon, cmd.name, width = max_name_len);
                let desc_col_max = total_width - max_name_len - 6;
                let desc = if cmd.description.chars().count() > desc_col_max {
                    let truncated: String = cmd
                        .description
                        .chars()
                        .take(desc_col_max.saturating_sub(1))
                        .collect();
                    format!("{}…", truncated)
                } else {
                    cmd.description.to_string()
                };

                if is_selected {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│"),
                        SetForegroundColor(Color::Rgb {
                            r: 99,
                            g: 179,
                            b: 237
                        }),
                        Print("▌"),
                        SetBackgroundColor(Color::Rgb {
                            r: 17,
                            g: 24,
                            b: 39
                        }),
                        SetForegroundColor(Color::Rgb {
                            r: 99,
                            g: 179,
                            b: 237
                        }),
                        SetAttribute(Attribute::Bold),
                        Print(format!(" {:<width$}", name_col, width = max_name_len + 4)),
                        SetAttribute(Attribute::Reset),
                        SetForegroundColor(Color::Rgb {
                            r: 209,
                            g: 213,
                            b: 219
                        }),
                        Print(format!("{:<width$}", desc, width = desc_col_max)),
                        ResetColor,
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│\r\n"),
                        ResetColor,
                    )?;
                } else {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│ "),
                        SetForegroundColor(Color::Rgb {
                            r: 94,
                            g: 234,
                            b: 212
                        }),
                        Print(format!("{:<width$}", name_col, width = max_name_len + 4)),
                        SetForegroundColor(Color::Rgb {
                            r: 107,
                            g: 114,
                            b: 128
                        }),
                        Print(format!("{:<width$}", desc, width = desc_col_max)),
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│\r\n"),
                        ResetColor,
                    )?;
                }
            }
        }

        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!("├{}┤\r\n", "─".repeat(total_width))),
            ResetColor,
        )?;

        let hint = "↑↓ navigate  ↵ select  esc cancel";
        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print("│"),
            SetForegroundColor(Color::Rgb {
                r: 99,
                g: 179,
                b: 237
            }),
            Print(format!(" {:}", query)),
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!(
                "{:>width$}",
                hint,
                width = total_width - query.len() - 1
            )),
            Print("│\r\n"),
            ResetColor,
        )?;

        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!("╰{}╯\r\n", "─".repeat(total_width))),
            ResetColor,
        )?;
        stdout.flush()?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break None;
                }
                (KeyCode::Enter, _) => {
                    if let Some(cmd) = filtered.get(selected) {
                        break Some(cmd.name.to_string());
                    } else {
                        break Some(query.clone());
                    }
                }
                (KeyCode::Down, _) | (KeyCode::Tab, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + 1) % filtered.len();
                    }
                }
                (KeyCode::Up, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + filtered.len() - 1) % filtered.len();
                    }
                }
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), _) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        } else {
            continue;
        }
    };

    terminal::disable_raw_mode()?;
    if lines_rendered > 0 {
        execute!(stdout, cursor::MoveUp(lines_rendered))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
        stdout.flush()?;
    }

    Ok(result)
}

fn clear_picker(stdout: &mut io::Stdout, lines: u16) -> io::Result<()> {
    execute!(stdout, cursor::MoveUp(lines))?;
    execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
    stdout.flush()?;
    Ok(())
}

pub fn run_skills_picker(
    registry: &mut crate::skills::SkillRegistry,
) -> io::Result<Option<String>> {
    let mut query = String::new();
    let mut selected: usize = 0;
    let mut lines_rendered: u16 = 0;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = loop {
        let list = registry.list();
        let total_skills = list.len();
        let filtered: Vec<&crate::skills::Skill> = list
            .into_iter()
            .filter(|s| {
                let q = query.trim_start_matches('/').to_lowercase();
                s.skill.name.to_lowercase().contains(&q)
                    || s.skill.description.to_lowercase().contains(&q)
            })
            .collect();

        if selected >= filtered.len() && !filtered.is_empty() {
            selected = filtered.len() - 1;
        }

        if lines_rendered > 0 {
            execute!(stdout, cursor::MoveUp(lines_rendered))?;
        }
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

        lines_rendered = render_skills(&mut stdout, &query, &filtered, selected, total_skills)?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break None;
                }
                (KeyCode::Enter, _) => {
                    if let Some(skill) = filtered.get(selected) {
                        break Some(skill.skill.name.clone());
                    } else {
                        break Some(query);
                    }
                }
                (KeyCode::Down, _) | (KeyCode::Tab, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + 1) % filtered.len();
                    }
                }
                (KeyCode::Up, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + filtered.len() - 1) % filtered.len();
                    }
                }
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), _) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        }
    };

    terminal::disable_raw_mode()?;
    if lines_rendered > 0 {
        execute!(stdout, cursor::MoveUp(lines_rendered))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
        stdout.flush()?;
    }

    Ok(result)
}

fn render_skills(
    stdout: &mut io::Stdout,
    query: &str,
    filtered: &[&crate::skills::Skill],
    selected: usize,
    total_skills: usize,
) -> io::Result<u16> {
    let max_display = 10;
    let start_idx = if filtered.len() <= max_display {
        0
    } else if selected < max_display / 2 {
        0
    } else if selected + max_display / 2 >= filtered.len() {
        filtered.len() - max_display
    } else {
        selected - max_display / 2
    };
    let end_idx = std::cmp::min(start_idx + max_display, filtered.len());
    let display_items = if filtered.is_empty() {
        &[]
    } else {
        &filtered[start_idx..end_idx]
    };

    let lines_rendered = 4 + if display_items.is_empty() {
        1
    } else {
        display_items.len()
    } as u16;

    let max_name_len = 16usize;
    let total_width = 56usize;

    let header_text = if query.trim_start_matches('/').is_empty() {
        format!(" Skills ({}) ", total_skills)
    } else {
        format!(" Skills ({}/{}) ", filtered.len(), total_skills)
    };
    let header_len = header_text.len();

    // Top border
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print("╭─"),
        SetForegroundColor(Color::Rgb {
            r: 99,
            g: 179,
            b: 237
        }),
        Print(header_text),
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print(format!(
            "{}╮\r\n",
            "─".repeat(total_width.saturating_sub(header_len + 2))
        )),
        ResetColor,
    )?;

    if filtered.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 107,
                g: 114,
                b: 128
            }),
            Print(format!(
                "│  {:<width$}│\r\n",
                "No matching skills",
                width = total_width - 2
            )),
            ResetColor,
        )?;
    } else {
        for (idx, skill) in display_items.iter().enumerate() {
            let actual_idx = start_idx + idx;
            let is_selected = actual_idx == selected;
            let icon = "🛠 ";
            let name_col = format!(
                "{} {:<width$}",
                icon,
                skill.skill.name,
                width = max_name_len
            );
            let desc_col_max = total_width - max_name_len - 6;
            let desc = if skill.skill.description.chars().count() > desc_col_max {
                let truncated: String = skill
                    .skill
                    .description
                    .chars()
                    .take(desc_col_max.saturating_sub(1))
                    .collect();
                format!("{}…", truncated)
            } else {
                skill.skill.description.to_string()
            };

            if is_selected {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│"),
                    SetForegroundColor(Color::Rgb {
                        r: 99,
                        g: 179,
                        b: 237
                    }),
                    Print("▌"),
                    SetBackgroundColor(Color::Rgb {
                        r: 17,
                        g: 24,
                        b: 39
                    }),
                    SetForegroundColor(Color::Rgb {
                        r: 99,
                        g: 179,
                        b: 237
                    }),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" {:<width$}", name_col, width = max_name_len + 4)),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::Rgb {
                        r: 209,
                        g: 213,
                        b: 219
                    }),
                    Print(format!("{:<width$}", desc, width = desc_col_max)),
                    ResetColor,
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│\r\n"),
                    ResetColor,
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│ "),
                    SetForegroundColor(Color::Rgb {
                        r: 94,
                        g: 234,
                        b: 212
                    }),
                    Print(format!("{:<width$}", name_col, width = max_name_len + 4)),
                    SetForegroundColor(Color::Rgb {
                        r: 107,
                        g: 114,
                        b: 128
                    }),
                    Print(format!("{:<width$}", desc, width = desc_col_max)),
                    SetForegroundColor(Color::Rgb {
                        r: 55,
                        g: 65,
                        b: 81
                    }),
                    Print("│\r\n"),
                    ResetColor,
                )?;
            }
        }
    }

    // Bottom border
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print(format!("├{}┤\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    // Input row
    let display_query = format!("Skill: {}", query);
    let hint = "↑↓ navigate  ↵ load  esc cancel";
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print("│"),
        SetForegroundColor(Color::Rgb {
            r: 99,
            g: 179,
            b: 237
        }),
        Print(format!(" {:}", display_query)),
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print(format!(
            "{:>width$}",
            hint,
            width = total_width - display_query.len() - 1
        )),
        Print("│\r\n"),
        ResetColor,
    )?;

    // Close border
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 55,
            g: 65,
            b: 81
        }),
        Print(format!("╰{}╯\r\n", "─".repeat(total_width))),
        ResetColor,
    )?;

    stdout.flush()?;
    Ok(lines_rendered as u16)
}

#[derive(Clone)]
pub struct DynamicCommand {
    pub name: String,
    pub description: String,
    pub icon: String,
}

pub fn run_dynamic_picker(title: &str, items: &[DynamicCommand]) -> io::Result<Option<String>> {
    let mut query = String::new();
    let mut selected: usize = 0;
    let mut lines_rendered: u16 = 0;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = loop {
        let q = query.to_lowercase();
        let filtered: Vec<&DynamicCommand> = if q.is_empty() {
            items.iter().collect()
        } else {
            items
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&q) || c.description.to_lowercase().contains(&q)
                })
                .collect()
        };

        if selected >= filtered.len() && !filtered.is_empty() {
            selected = filtered.len() - 1;
        }

        if lines_rendered > 0 {
            execute!(stdout, cursor::MoveUp(lines_rendered))?;
        }
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

        // Inline generic render logic
        let max_display = 10;
        let start_idx = if filtered.len() <= max_display {
            0
        } else if selected < max_display / 2 {
            0
        } else if selected + max_display / 2 >= filtered.len() {
            filtered.len() - max_display
        } else {
            selected - max_display / 2
        };
        let end_idx = std::cmp::min(start_idx + max_display, filtered.len());
        let display_items = if filtered.is_empty() {
            &[]
        } else {
            &filtered[start_idx..end_idx]
        };

        let total_width = 60usize;
        lines_rendered = 6 + if display_items.is_empty() {
            1
        } else {
            display_items.len()
        } as u16;

        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!("╭{}╮\r\n", "─".repeat(total_width))),
            Print("│ "),
            SetForegroundColor(Color::Rgb {
                r: 252,
                g: 211,
                b: 77
            }),
            SetAttribute(Attribute::Bold),
            Print(format!("{:<width$}", title, width = total_width - 2)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print("│\r\n"),
            Print(format!("├{}┤\r\n", "─".repeat(total_width))),
            ResetColor,
        )?;

        if display_items.is_empty() {
            execute!(
                stdout,
                SetForegroundColor(Color::Rgb {
                    r: 55,
                    g: 65,
                    b: 81
                }),
                Print("│ "),
                SetForegroundColor(Color::Rgb {
                    r: 156,
                    g: 163,
                    b: 175
                }),
                Print(format!(
                    "{:<width$}",
                    "No items match query",
                    width = total_width - 2
                )),
                SetForegroundColor(Color::Rgb {
                    r: 55,
                    g: 65,
                    b: 81
                }),
                Print("│\r\n"),
                ResetColor,
            )?;
        } else {
            let mut max_name_len = 0;
            for c in display_items {
                let name_len = c.name.chars().count();
                if name_len > max_name_len {
                    max_name_len = name_len;
                }
            }
            if max_name_len > 25 {
                max_name_len = 25;
            }

            let desc_col_max = total_width - max_name_len - 8;

            for (idx, c) in display_items.iter().enumerate() {
                let is_selected = start_idx + idx == selected;
                let name_truncated = if c.name.chars().count() > 25 {
                    format!("{}...", c.name.chars().take(22).collect::<String>())
                } else {
                    c.name.clone()
                };
                let name_col = format!("{} {}", c.icon, name_truncated);
                let desc = if c.description.chars().count() > desc_col_max {
                    format!(
                        "{}...",
                        c.description
                            .chars()
                            .take(desc_col_max - 3)
                            .collect::<String>()
                    )
                } else {
                    c.description.clone()
                };

                if is_selected {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Rgb {
                            r: 99,
                            g: 179,
                            b: 237
                        }),
                        Print("▌"),
                        SetBackgroundColor(Color::Rgb {
                            r: 17,
                            g: 24,
                            b: 39
                        }),
                        SetForegroundColor(Color::Rgb {
                            r: 99,
                            g: 179,
                            b: 237
                        }),
                        SetAttribute(Attribute::Bold),
                        Print(format!(" {:<width$}", name_col, width = max_name_len + 4)),
                        SetAttribute(Attribute::Reset),
                        SetForegroundColor(Color::Rgb {
                            r: 209,
                            g: 213,
                            b: 219
                        }),
                        Print(format!("{:<width$}", desc, width = desc_col_max)),
                        ResetColor,
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│\r\n"),
                        ResetColor,
                    )?;
                } else {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│ "),
                        SetForegroundColor(Color::Rgb {
                            r: 94,
                            g: 234,
                            b: 212
                        }),
                        Print(format!("{:<width$}", name_col, width = max_name_len + 4)),
                        SetForegroundColor(Color::Rgb {
                            r: 107,
                            g: 114,
                            b: 128
                        }),
                        Print(format!("{:<width$}", desc, width = desc_col_max)),
                        SetForegroundColor(Color::Rgb {
                            r: 55,
                            g: 65,
                            b: 81
                        }),
                        Print("│\r\n"),
                        ResetColor,
                    )?;
                }
            }
        }

        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!("├{}┤\r\n", "─".repeat(total_width))),
            ResetColor,
        )?;

        let hint = "↑↓ navigate  ↵ select  esc cancel";
        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print("│"),
            SetForegroundColor(Color::Rgb {
                r: 99,
                g: 179,
                b: 237
            }),
            Print(format!(" {:}", query)),
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!(
                "{:>width$}",
                hint,
                width = total_width - query.len() - 1
            )),
            Print("│\r\n"),
            ResetColor,
        )?;

        execute!(
            stdout,
            SetForegroundColor(Color::Rgb {
                r: 55,
                g: 65,
                b: 81
            }),
            Print(format!("╰{}╯\r\n", "─".repeat(total_width))),
            ResetColor,
        )?;
        stdout.flush()?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    break None;
                }
                (KeyCode::Enter, _) => {
                    if let Some(cmd) = filtered.get(selected) {
                        break Some(cmd.name.clone());
                    } else {
                        break Some(query.clone());
                    }
                }
                (KeyCode::Down, _) | (KeyCode::Tab, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + 1) % filtered.len();
                    }
                }
                (KeyCode::Up, _) => {
                    if !filtered.is_empty() {
                        selected = (selected + filtered.len() - 1) % filtered.len();
                    }
                }
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), _) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        } else {
            continue;
        }
    };

    terminal::disable_raw_mode()?;
    if lines_rendered > 0 {
        execute!(stdout, cursor::MoveUp(lines_rendered))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
        stdout.flush()?;
    }

    Ok(result)
}
