use colored::Colorize;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

pub struct MarkdownRenderer {
    in_code_block: bool,
    highlighter: Option<HighlightLines<'static>>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        MarkdownRenderer {
            in_code_block: false,
            highlighter: None,
        }
    }

    /// Processes a line of markdown and returns a colored/formatted ANSI string.
    pub fn render_line(&mut self, line: &str) -> String {
        let trimmed = line.trim();

        // Handle code block toggles
        if trimmed.starts_with("```") {
            if self.in_code_block {
                self.in_code_block = false;
                self.highlighter = None;
                return format!(
                    " {}{}",
                    "└──────────────────────────────────────────────────".truecolor(100, 100, 100),
                    "\n"
                );
            } else {
                self.in_code_block = true;
                let lang = trimmed[3..].trim();
                let ps = get_syntax_set();
                let ts = get_theme_set();

                let syntax = ps
                    .find_syntax_by_token(lang)
                    .unwrap_or_else(|| ps.find_syntax_plain_text());

                // base16-ocean.dark is a nice built-in theme
                let theme = &ts.themes["base16-ocean.dark"];
                self.highlighter = Some(HighlightLines::new(syntax, theme));

                let lang_display = if lang.is_empty() { "CODE" } else { lang };
                return format!(
                    " {}{}",
                    format!(
                        "┌── {} ──────────────────────────────────────────",
                        lang_display
                    )
                    .truecolor(100, 100, 100),
                    "\n"
                );
            }
        }

        // If we are inside a code block, format with syntect
        if self.in_code_block {
            let highlighted = if let Some(h) = &mut self.highlighter {
                let line_with_nl = format!("{}\n", line);
                let ranges: Vec<(Style, &str)> = h
                    .highlight_line(&line_with_nl, get_syntax_set())
                    .unwrap_or_default();
                as_24_bit_terminal_escaped(&ranges[..], false)
            } else {
                format!("{}\n", line.cyan())
            };
            return format!(" {} {}", "│".truecolor(100, 100, 100), highlighted);
        }

        // Handle Headers
        if trimmed.starts_with("# ") {
            let header = &trimmed[2..];
            return format!("\n{}\n", header.bold().blue().underline());
        }
        if trimmed.starts_with("## ") {
            let header = &trimmed[3..];
            return format!("\n{}\n", header.bold().cyan());
        }
        if trimmed.starts_with("### ") {
            let header = &trimmed[4..];
            return format!("\n{}\n", header.bold().yellow());
        }

        // Handle Lists
        let mut processed_line = if trimmed.starts_with("- ") {
            format!("  {} {}", "•".truecolor(76, 175, 80), &trimmed[2..])
        } else if trimmed.starts_with("* ") {
            format!("  {} {}", "•".truecolor(76, 175, 80), &trimmed[2..])
        } else {
            line.to_string()
        };

        // Parse inline bold (**text**)
        processed_line = self.format_inline_bold(&processed_line);

        // Parse inline code (`code`)
        processed_line = self.format_inline_code(&processed_line);

        format!("{}\n", processed_line)
    }

    fn format_inline_bold(&self, text: &str) -> String {
        let mut result = String::new();
        let parts = text.split("**");
        let mut is_bold = false;

        for part in parts {
            if is_bold {
                result.push_str(&part.bold().to_string());
            } else {
                result.push_str(part);
            }
            is_bold = !is_bold;
        }

        result
    }

    fn format_inline_code(&self, text: &str) -> String {
        let mut result = String::new();
        let parts = text.split('`');
        let mut is_code = false;

        for part in parts {
            if is_code {
                result.push_str(&part.truecolor(244, 143, 177).to_string()); // Light pink/magenta for inline code
            } else {
                result.push_str(part);
            }
            is_code = !is_code;
        }

        result
    }
}
