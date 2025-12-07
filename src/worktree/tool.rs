use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};

use crate::cli::ToolCommand;
use crate::config::Config;
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;
use crate::worktree::create;

pub fn run_tool_command(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
    tool_name: &str,
) -> Result<()> {
    let target_path = create::ensure_worktree(repo, git, config, cmd)?;
    run_tool(tool_name, &target_path, &cmd.args)
}

pub fn run_terminal_tool_command(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
    tool_name: &str,
) -> Result<()> {
    let target_path = create::ensure_worktree(repo, git, config, cmd)?;
    spawn_terminal(tool_name, &target_path, &cmd.args)
}

pub fn run_default_editor(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
) -> Result<()> {
    let editor = config.default_editor.as_deref().unwrap_or("cursor");
    
    if is_terminal_tool(editor) {
        run_terminal_tool_command(repo, git, config, cmd, editor)
    } else {
        run_tool_command(repo, git, config, cmd, editor)
    }
}

pub fn run_default_cli(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
) -> Result<()> {
    let cli = config.default_cli.as_deref().ok_or_else(|| anyhow!("No default CLI configured. Set it with 'gwe config set gwe.defaultCli <NAME>'"))?;
    
    // CLIツールは基本的にターミナルで実行されるべき
    run_terminal_tool_command(repo, git, config, cmd, cli)
}

fn is_terminal_tool(name: &str) -> bool {
    matches!(name, "claude" | "codex" | "gemini")
}

fn prepare_tool_args(tool: &str, args: &[String]) -> Vec<String> {
    if tool == "gemini" && !args.is_empty() {
        // gemini で引数があり、かつ -i/-p 等が指定されていない場合は -i (interactive) を付与
        if !args.iter().any(|a| a == "-i" || a == "--prompt-interactive" || a == "-p" || a == "--prompt") {
            let mut new_args = vec!["-i".to_string()];
            new_args.extend_from_slice(args);
            return new_args;
        }
    }
    args.to_vec()
}

fn run_tool(tool: &str, path: &Path, args: &[String]) -> Result<()> {
    let final_args = prepare_tool_args(tool, args);

    let mut command = if cfg!(windows) {
         let mut c = Command::new("cmd");
         c.arg("/C").arg(tool);
         c
    } else {
         Command::new(tool)
    };
    
    command.arg(path);
    command.args(&final_args);
    
    let status = command.status()
        .map_err(|e| anyhow!("Failed to execute tool '{}': {}", tool, e))?;
        
    if !status.success() {
        return Err(anyhow!("Tool '{}' exited with status {}", tool, status));
    }
    Ok(())
}

fn spawn_terminal(tool: &str, path: &Path, args: &[String]) -> Result<()> {
    let final_args = prepare_tool_args(tool, args);

    #[cfg(target_os = "macos")]
    {
        // Escape path for AppleScript
        let path_str = path.to_string_lossy().replace('"', "\\\"");
        
        let args_str = final_args.iter()
            .map(|arg| shell_quote_sh(arg))
            .collect::<Vec<_>>()
            .join(" ");

        let cmd = if args_str.is_empty() {
            tool.to_string()
        } else {
            format!("{} {}", tool, args_str)
        };
        // Escape cmd for AppleScript
        let cmd_escaped = cmd.replace('"', "\\\"");

        let script = format!(
            r#"tell application "Terminal"
                do script "cd \"{}\"; {}"
                activate
            end tell"#,
            path_str,
            cmd_escaped
        );

        let status = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status()
            .map_err(|e| anyhow!("Failed to execute osascript: {}", e))?;

        if !status.success() {
            return Err(anyhow!("Failed to spawn terminal"));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let args_str = args.iter()
            .map(|arg| shell_quote_cmd(arg))
            .collect::<Vec<_>>()
            .join(" ");

        let cmd_str = if args_str.is_empty() {
            tool.to_string()
        } else {
            format!("{} {}", tool, args_str)
        };
        
        // We use `cmd /C start "Title" cmd /K "cd /d PATH && command"`
        // This opens a new window, changes directory, runs the command, and keeps the window open.
        let status = Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg(format!("gwe - {}", tool)) // Window Title
            .arg("cmd")
            .arg("/K") // Keep window open
            .arg(format!("cd /d \"{}\" && {}", path.to_string_lossy(), cmd_str))
            .status()
            .map_err(|e| anyhow!("Failed to spawn terminal: {}", e))?;

        if !status.success() {
            return Err(anyhow!("Failed to spawn terminal"));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow!("Terminal spawning is only supported on macOS and Windows for now"))
    }
}

// Quote argument for POSIX shell (sh/bash/zsh)
// Wraps in single quotes and escapes single quotes inside
fn shell_quote_sh(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    // If it contains only safe chars, return as is
    if s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@')) {
        return s.to_string();
    }
    
    // 'string' -> 'string'
    // 'str'\''ing' -> 'str'\''ing'
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

// Quote argument for Windows CMD
// Wraps in double quotes and escapes double quotes inside
fn shell_quote_cmd(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    // If it contains only safe chars, return as is
    // Note: Windows paths may contain spaces but if they don't, quotes might be optional,
    // but quoting is safer generally if it might contain special shell chars.
    if s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '\\')) {
        return s.to_string();
    }

    // Windows CMD escaping is tricky, but generally wrapping in double quotes works.
    // Double quotes inside need to be escaped.
    // Usually replacing `"` with `""` works inside double quoted string in some contexts,
    // but passing to `cmd /c` is complex.
    // For simplicity here we replace `"` with `\"`.
    let escaped = s.replace('"', "\\\"");
    format!("\"{}\"", escaped)
}
