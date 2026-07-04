use crate::config::settings::CommandSandboxMode;
use crate::tools::{Tool, ToolContext};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, timeout, Duration};

pub struct RunCommandTool;

#[async_trait::async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the local system. Always requires approval."
    }

    fn requires_approval(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute (e.g. 'npm install', 'cargo run', 'dir')."
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Optional command timeout in seconds. Defaults to 300."
                },
                "background": {
                    "type": "boolean",
                    "description": "Run long-lived commands such as dev servers in the background and return immediately."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let cmd_str = args["command"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'command' argument"))?;
        let timeout_seconds = args["timeout_seconds"]
            .as_u64()
            .filter(|seconds| *seconds > 0)
            .unwrap_or(300);
        let background = args["background"]
            .as_bool()
            .unwrap_or_else(|| is_likely_dev_server(cmd_str));

        if ctx.settings.command_sandbox == CommandSandboxMode::Docker {
            return run_docker_command(cmd_str, timeout_seconds, background, ctx).await;
        }

        // Detect OS shell
        let (shell, shell_args) = if cfg!(target_os = "windows") {
            (
                "powershell",
                vec!["-NoProfile", "-NonInteractive", "-Command"],
            )
        } else {
            ("sh", vec!["-c"])
        };

        if background {
            let log_dir = ctx.settings.workspace_dir.join(".orion").join("logs");
            std::fs::create_dir_all(&log_dir)?;

            let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            let stdout_path = log_dir.join(format!("command-{}-stdout.log", timestamp));
            let stderr_path = log_dir.join(format!("command-{}-stderr.log", timestamp));
            let stdout_file = std::fs::File::create(&stdout_path)?;
            let stderr_file = std::fs::File::create(&stderr_path)?;

            let mut command = Command::new(shell);
            command
                .args(shell_args)
                .arg(cmd_str)
                .current_dir(&ctx.settings.workspace_dir)
                .env("NPM_CONFIG_YES", "true")
                .stdin(Stdio::null())
                .stdout(Stdio::from(stdout_file))
                .stderr(Stdio::from(stderr_file));

            let mut child = command.spawn()?;
            let pid = child.id().unwrap_or(0);
            sleep(Duration::from_secs(2)).await;

            if let Some(status) = child.try_wait()? {
                let stdout = std::fs::read_to_string(&stdout_path).unwrap_or_default();
                let stderr = std::fs::read_to_string(&stderr_path).unwrap_or_default();
                let mut result = String::new();
                result.push_str("Background command exited quickly.\n");
                append_command_output(&mut result, &stdout, &stderr);
                result.push_str(&format!("Exit Code: {}", status.code().unwrap_or(-1)));
                return Ok(result);
            }

            return Ok(format!(
                "Started background command (pid {}).\nstdout: {}\nstderr: {}\nThe process is still running.",
                pid,
                stdout_path.display(),
                stderr_path.display()
            ));
        }

        // Run the command
        let mut command = Command::new(shell);
        command
            .args(shell_args)
            .arg(cmd_str)
            .current_dir(&ctx.settings.workspace_dir)
            .env("NPM_CONFIG_YES", "true")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        command.kill_on_drop(true);
        let child = command.spawn()?;

        // Capture output
        let output = match timeout(
            Duration::from_secs(timeout_seconds),
            child.wait_with_output(),
        )
        .await
        {
            Ok(output) => output?,
            Err(_) => {
                return Ok(format!(
                    "Command timed out after {} seconds and was stopped. The command may have been waiting for interactive input. Retry with non-interactive flags such as -y/--yes when using npm/npx scaffolding commands.",
                    timeout_seconds
                ));
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = String::new();
        append_command_output(&mut result, &stdout, &stderr);
        result.push_str(&format!(
            "Exit Code: {}",
            output.status.code().unwrap_or(-1)
        ));

        Ok(result)
    }
}

async fn run_docker_command(
    cmd_str: &str,
    timeout_seconds: u64,
    background: bool,
    ctx: &ToolContext,
) -> Result<String> {
    if background {
        return Ok("Docker sandbox mode does not support background commands yet. Switch to /sandbox host for dev servers.".to_string());
    }

    let docker_check = Command::new("docker")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let docker_check = match docker_check {
        Ok(child) => child.wait_with_output().await?,
        Err(e) => {
            return Ok(format!(
                "Docker sandbox is enabled, but Docker could not be started: {}",
                e
            ));
        }
    };
    if !docker_check.status.success() {
        return Ok(format!(
            "Docker sandbox is enabled, but Docker is unavailable:\n{}",
            String::from_utf8_lossy(&docker_check.stderr)
        ));
    }

    let image = ctx
        .settings
        .docker_image
        .clone()
        .unwrap_or_else(|| "ubuntu:latest".to_string());
    let workspace = ctx.settings.workspace_dir.to_string_lossy().to_string();
    let shell_command = if cfg!(target_os = "windows") {
        cmd_str.to_string()
    } else {
        cmd_str.to_string()
    };

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{}:/workspace", workspace))
        .arg("-w")
        .arg("/workspace")
        .arg(&image)
        .arg("sh")
        .arg("-lc")
        .arg(shell_command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command.kill_on_drop(true);
    let child = command.spawn()?;
    let output = match timeout(
        Duration::from_secs(timeout_seconds),
        child.wait_with_output(),
    )
    .await
    {
        Ok(output) => output?,
        Err(_) => {
            return Ok(format!(
                "Docker sandbox command timed out after {} seconds and was stopped.",
                timeout_seconds
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let mut result = format!("Docker sandbox image: {}\n", image);
    append_command_output(&mut result, &stdout, &stderr);
    result.push_str(&format!(
        "Exit Code: {}",
        output.status.code().unwrap_or(-1)
    ));
    Ok(result)
}

fn is_likely_dev_server(command: &str) -> bool {
    let normalized = command.to_lowercase();
    let dev_server_markers = [
        "npm run dev",
        "npm start",
        " run dev",
        "pnpm dev",
        "pnpm start",
        "yarn dev",
        "yarn start",
        "vite --host",
        "vite --open",
        "next dev",
        "astro dev",
        "remix dev",
    ];

    dev_server_markers
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn append_command_output(result: &mut String, stdout: &str, stderr: &str) {
    if !stdout.is_empty() {
        result.push_str("STDOUT\n");
        result.push_str(stdout.trim_end());
        result.push_str("\n\n");
    }
    if !stderr.is_empty() {
        result.push_str("STDERR\n");
        result.push_str(stderr.trim_end());
        result.push_str("\n\n");
    }
}
