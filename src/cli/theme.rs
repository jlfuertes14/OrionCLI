use colored::*;

pub fn print_logo(active_model: &str, active_provider: &str, workspace_dir: &str) {
    let spaceship = [
        "⢀⣀⣤⣤⣀⣀⣀",
        "⣿⠉⠀⠀⣩⡿⠋⠛⢛⣷⣶⣤⣄⡀",
        "⣿⠀⢀⡾⠋⠀⠀⣠⠎⠁⠁⠰⠋⢙⠷⢦⣄",
        "⢿⡴⠋⠀⠀⣠⠞⠁⠀⠀⠀⠀⠀⠉⠀⠰⢋⡿⣦⣀",
        "⠸⣇⠀⢀⡾⠃⠀⢀⣴⣿⣛⠷⣦⡀⠀⠀⠈⠀⠊⣹⢷⣄",
        "⠀⢿⣴⠋⠀⠀⢀⣿⣽⣿⣿⣷⢻⡇⠀⠀⠀⠀⠀⠰⠋⣻⣦⡀",
        "⠀⠸⣷⠀⠀⠀⠈⣷⡸⢿⣿⣿⡾⠃⠀⢀⡀⠀⠀⠀⠘⠁⡼⢿⣟⠛⠛⠛⠛⠛⠷⠶⢦⣤⡀",
        "⠀⠀⢹⣶⢂⠀⠀⠈⠛⠻⠟⠋⠁⣴⣿⣿⣿⣳⣄⠀⠀⠈⣰⠋⣻⣷⠒⠶⠶⠶⠶⠶⠦⠬⢽⣦",
        "⠀⠀⠀⢻⣏⡀⠀⠀⠀⠀⠀⠀⢰⡯⣿⣿⣿⡏⣿⠀⠀⠀⢀⡼⠋⠹⣧⠀⠀⠀⠀⠀⠀⠀⠀⢹⡆",
        "⠀⠀⠀⠀⢻⣧⠄⣠⠄⠀⠀⠀⠀⠻⣟⣛⣛⣽⠏⠀⠀⣴⠟⠀⢀⣼⣿⣇⠀⠀⠀⠀⠀⠀⠀⠀⣿",
        "⠀⠀⠀⠀⠀⠹⣯⡥⢂⡀⠀⠀⠀⠀⠈⠉⠉⠀⠀⣠⣾⣁⠀⣠⠞⠉⠐⣿⣄⣀⣀⣀⣀⠀⠀⠀⢸⡇",
        "⠀⠀⠀⠀⠀⠀⠈⢿⣯⠴⢃⡄⠀⠀⠀⠀⠀⢀⡼⠋⢿⡝⠻⢷⣤⣀⣀⣿⠋⠉⠉⠉⠙⢷⣄⠀⠈⣷",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠙⢿⣯⡴⢃⠀⠀⢀⡼⠋⠀⢀⡼⢻⡄⠀⠈⠙⣿⡁⠀⠀⠀⠀⠀⠀⠙⢷⣄⣿",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢹⡿⢿⣦⣴⠋⠀⢀⡴⠋⠀⠈⢿⣆⠀⠀⠹⣿⠶⣤⡀⠀⠀⠀⠀⠀⠻⣿⡇",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⡇⠸⡏⠙⠻⠶⣯⣾⣗⣀⣠⡾⠋⠻⣷⢶⣿⣷⣮⡙⢷⣄⠀⠀⠀⠀⠈⠁",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢿⠀⣇⠀⠀⠀⠀⠈⠉⢻⡏⠀⠀⠀⢻⣄⠻⣯⡈⠳⣄⠙⢷⣄",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⡄⢻⠀⠀⠀⠀⠀⠀⢸⡇⠀⠀⠀⠀⠹⣦⡈⠛⢦⣘⢧⡀⠹⣧⡀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⣧⠘⡇⠀⠀⠀⠀⠀⠈⢷⣄⠀⠀⠀⠀⠈⠻⢶⣄⠉⠛⢷⡄⠘⢷⡄",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠙⢧⣷⡀⠀⠀⠀⠀⠀⠀⠙⢷⣄⠀⠀⠀⠀⠀⠙⠻⢦⣄⡀⠀⠈⢿⡄",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠙⠛⠳⠶⢦⣤⣤⣤⣤⣹⣷⣄⠀⠀⠀⠀⠀⠀⠉⠛⠷⣦⣌⣿⡄",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈",
    ];

    // Print spaceship with blue-to-orange gradient styling
    for (i, line) in spaceship.iter().enumerate() {
        if i < 13 {
            // Spaceship hull
            println!("{}", line.truecolor(33, 150, 243));
        } else {
            // Thrusters and fire
            println!("{}", line.truecolor(255, 112, 67));
        }
    }

    println!();

    // Print Title
    println!(
        "{}",
        " ██████╗ ██████╗ ██╗ ██████╗ ███╗   ██╗     ██████╗██╗     ██╗"
            .bold()
            .truecolor(33, 150, 243)
    );
    println!(
        "{}",
        "██╔═══██╗██╔══██╗██║██╔═══██╗████╗  ██║    ██╔════╝██║     ██║"
            .bold()
            .truecolor(33, 150, 243)
    );
    println!(
        "{}",
        "██║   ██║██████╔╝██║██║   ██║██╔██╗ ██║    ██║     ██║     ██║"
            .bold()
            .truecolor(33, 150, 243)
    );
    println!(
        "{}",
        "██║   ██║██╔══██╗██║██║   ██║██║╚██╗██║    ██║     ██║     ██║"
            .bold()
            .truecolor(33, 150, 243)
    );
    println!(
        "{}",
        "╚██████╔╝██║  ██║██║╚██████╔╝██║ ╚████║    ╚██████╗███████╗██║"
            .bold()
            .truecolor(33, 150, 243)
    );
    println!(
        "{}",
        " ╚═════╝ ╚═╝  ╚═╝╚═╝ ╚══════╝ ╚═╝  ╚═══╝     ╚═════╝╚══════╝╚═╝"
            .bold()
            .truecolor(33, 150, 243)
    );

    println!();
    println!(
        "{}",
        "  The Intelligent Rust Agent"
            .italic()
            .truecolor(200, 200, 200)
    );
    println!(
        "  {} {}",
        "Active Workspace:".bold().truecolor(129, 199, 132),
        workspace_dir.truecolor(200, 200, 200)
    );
    println!(
        "{}",
        "  ======================================================================"
            .truecolor(67, 160, 71)
    );
    println!(
        "  Type {} for list of commands, or ask me anything.",
        "/help".bold().green()
    );
    println!("  Active Model: {}", active_model.bold().cyan());
    println!("  Active Provider: {}\n", active_provider.bold().cyan());
}

#[allow(dead_code)]
pub fn format_assistant(text: &str) -> String {
    format!("{} {}", "Orion:".bold().truecolor(33, 150, 243), text)
}

#[allow(dead_code)]
pub fn format_system(text: &str) -> String {
    format!("{} {}", "System:".bold().truecolor(158, 158, 158), text)
}

pub fn format_user_prompt() -> String {
    "❯ ".to_string()
}

pub fn print_info(text: &str) {
    println!("{} {}", "ℹ".blue().bold(), text.blue());
}

pub fn print_warning(text: &str) {
    println!("{} {}", "⚠".yellow().bold(), text.yellow());
}

pub fn print_error(text: &str) {
    println!("{} {}", "✖".red().bold(), text.red());
}

pub fn print_success(text: &str) {
    println!("{} {}", "✔".green().bold(), text.green());
}

pub fn generate_color_diff(old: &str, new: &str) -> String {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    let mut result = String::new();

    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };

            let line = format!("{} {}", sign, change.value());
            let colored_line = match change.tag() {
                ChangeTag::Delete => line.red().to_string(),
                ChangeTag::Insert => line.green().to_string(),
                ChangeTag::Equal => line.truecolor(100, 100, 100).to_string(),
            };

            result.push_str(&colored_line);
        }
    }

    result
}
