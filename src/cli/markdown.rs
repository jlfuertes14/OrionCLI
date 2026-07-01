use colored::Colorize;

pub struct MarkdownRenderer {
    in_code_block: bool,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        MarkdownRenderer {
            in_code_block: false,
        }
    }

    /// Processes a line of markdown and returns a colored/formatted ANSI string.
    pub fn render_line(&mut self, line: &str) -> String {
        let trimmed = line.trim();

        // Handle code block toggles
        if trimmed.starts_with("```") {
            self.in_code_block = !self.in_code_block;
            if self.in_code_block {
                return format!(" {}{}", "┌── CODE ──────────────────────────────────────────".truecolor(100, 100, 100), "\n");
            } else {
                return format!(" {}{}", "└──────────────────────────────────────────────────".truecolor(100, 100, 100), "\n");
            }
        }

        // If we are inside a code block, print in gray/cyan with indentation
        if self.in_code_block {
            return format!(" {} {}\n", "│".truecolor(100, 100, 100), line.cyan());
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
