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
    // -x オプションが指定されている場合は複数ワークツリーを作成
    if let Some(count) = cmd.multiplier {
        return run_terminal_tool_multi(repo, git, config, cmd, tool_name, count, &cmd.args);
    }

    let target_path = create::ensure_worktree(repo, git, config, cmd)?;
    spawn_terminal(tool_name, &target_path, &cmd.args)
}

fn run_terminal_tool_multi(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
    tool_name: &str,
    count: u8,
    args: &[String],
) -> Result<()> {
    use crate::error::AppError;

    // -x は -b (新規ブランチ) と併用必須
    let base_branch = cmd.branch.as_ref().ok_or_else(|| {
        AppError::user("-x/--multiplier requires -b/--branch option")
    })?;

    // -x と target の併用はエラー
    if cmd.target.is_some() {
        return Err(AppError::user("-x/--multiplier cannot be used with target worktree").into());
    }

    // 複数ワークツリーを作成
    let paths = create::create_multiple_worktrees(
        repo,
        git,
        config,
        base_branch,
        count,
        cmd.track.as_deref(),
    )?;

    // ターミナル起動
    println!("\nLaunching {} terminals...", paths.len());
    let tools_with_paths: Vec<(String, PathBuf)> = paths.into_iter()
        .map(|p| (tool_name.to_string(), p))
        .collect();
    spawn_multiple_tools(&tools_with_paths, args)
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

pub fn run_multi_cli(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
) -> Result<()> {
    if config.multi_cli.is_empty() {
        return Err(anyhow!("No multi-CLI configured. Set it with 'gwe config add gwe.multiCli <NAME>'"));
    }

    let tools = &config.multi_cli;
    let count = tools.len() as u8;

    let tools_with_paths: Vec<(String, PathBuf)> = if let Some(base_branch) = &cmd.branch {
        // -b が指定されている場合は複数ワークツリーを作成
        use crate::error::AppError;

        if tools.len() > 5 {
            return Err(AppError::user(format!("Too many tools in multiCli (max 5 for split panes, found {})", tools.len())).into());
        }
        
        // target との併用はエラー (-x と同じ)
        if cmd.target.is_some() {
            return Err(AppError::user("'gwe cli' with -b/--branch cannot be used with target worktree").into());
        }

        let paths = create::create_multiple_worktrees(
            repo,
            git,
            config,
            base_branch,
            count,
            cmd.track.as_deref(),
        )?;

        tools.iter().zip(paths.into_iter())
            .map(|(t, p)| (t.clone(), p))
            .collect()
    } else {
        // それ以外（既存ワークツリー指定など）は単一ワークツリー
        let target_path = create::ensure_worktree(repo, git, config, cmd)?;
        tools.iter()
            .map(|tool| (tool.clone(), target_path.clone()))
            .collect()
    };

    println!("\nLaunching {} tools in split panes...", tools_with_paths.len());
    spawn_multiple_tools(&tools_with_paths, &cmd.args)
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
    #[cfg(target_os = "macos")]
    {
        let final_args = prepare_tool_args(tool, args);
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
        let final_args = prepare_tool_args(tool, args);
        // Ensure path uses backslashes for Windows
        let path_str = path.to_string_lossy().replace('/', "\\");

        let args_str = final_args.iter()
            .map(|arg| shell_quote_cmd(arg))
            .collect::<Vec<_>>()
            .join(" ");

        let cmd_str = if args_str.is_empty() {
            tool.to_string()
        } else {
            format!("{} {}", tool, args_str)
        };

        // We use `cmd /C start "Title" /D "Path" cmd /K "command"`
        // This opens a new window in the specified directory, runs the command, and keeps the window open.
        let status = Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg(format!("gwe - {}", tool)) // Window Title
            .arg("/D")
            .arg(path_str)
            .arg("cmd")
            .arg("/K")
            .arg(&cmd_str)
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
#[cfg(target_os = "macos")]
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
#[cfg(target_os = "windows")]
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

// ========================================
// 複数ツール起動
// ========================================

fn spawn_multiple_tools(tools_with_paths: &[(String, PathBuf)], args: &[String]) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Windows Terminal を試す
        if try_spawn_windows_terminal(tools_with_paths, args).is_ok() {
            return Ok(());
        }
        // フォールバック: 個別ウィンドウ
        spawn_multiple_windows(tools_with_paths, args)
    }

    #[cfg(target_os = "macos")]
    {
        // iTerm2 を試す
        if try_spawn_iterm_splits(tools_with_paths, args).is_ok() {
            return Ok(());
        }
        // フォールバック: 複数 Terminal.app ウィンドウ
        spawn_multiple_terminals_macos(tools_with_paths, args)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow!("Multi-terminal spawning is only supported on macOS and Windows"))
    }
}

// ========================================
// Windows Terminal 分割ペイン
// ========================================

#[cfg(target_os = "windows")]
fn is_windows_terminal_available() -> bool {
    // wt.exe --version はGUIヘルプを表示するため、where コマンドで存在確認
    Command::new("where")
        .arg("wt.exe")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn build_tool_command_str(tool: &str, args: &[String]) -> String {
    let final_args = prepare_tool_args(tool, args);
    if final_args.is_empty() {
        tool.to_string()
    } else {
        let args_str = final_args
            .iter()
            .map(|a| shell_quote_cmd(a))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {}", tool, args_str)
    }
}

#[cfg(target_os = "windows")]
fn build_wt_pane_args(cmd_str: &str, path: &Path) -> Vec<String> {
    let path_str = path.to_string_lossy().replace('/', "\\");
    vec![
        "-d".to_string(),
        path_str,
        "cmd".to_string(),
        "/K".to_string(),
        cmd_str.to_string(),
    ]
}

#[cfg(target_os = "windows")]
fn try_spawn_windows_terminal(tools_with_paths: &[(String, PathBuf)], args: &[String]) -> Result<()> {
    if !is_windows_terminal_available() {
        return Err(anyhow!("wt.exe not found"));
    }

    let mut wt_args: Vec<String> = Vec::new();

    // レイアウト仕様:
    // 2分割: [1][2] 垂直（左右）
    // 3分割: [1][2] / [3][空] 2x2グリッド
    // 4分割: [1][2] / [3][4] 2x2グリッド
    // 5分割: [1][2][3] / [4][5][空] 2x3グリッド

    match tools_with_paths.len() {
        1 => {
            // 1つだけ
            let (tool, path) = &tools_with_paths[0];
            let cmd_str = build_tool_command_str(tool, args);
            wt_args.extend(build_wt_pane_args(&cmd_str, path));
        }
        2 => {
            // [1][2] 垂直分割
            let (t1, p1) = &tools_with_paths[0];
            let (t2, p2) = &tools_with_paths[1];
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t1, args), p1));
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-V".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t2, args), p2));
        }
        3 => {
            // [1][2] / [3][空] 2x2グリッド
            let (t1, p1) = &tools_with_paths[0];
            let (t2, p2) = &tools_with_paths[1];
            let (t3, p3) = &tools_with_paths[2];
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t1, args), p1));
            // 右に分割 [1][2]
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-V".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t2, args), p2));
            // 左に戻って下に分割 [3]
            wt_args.extend([";".to_string(), "move-focus".to_string(), "left".to_string()]);
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-H".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t3, args), p3));
        }
        4 => {
            // [1][2] / [3][4] 2x2グリッド
            let (t1, p1) = &tools_with_paths[0];
            let (t2, p2) = &tools_with_paths[1];
            let (t3, p3) = &tools_with_paths[2];
            let (t4, p4) = &tools_with_paths[3];
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t1, args), p1));
            // 右に分割 [1][2]
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-V".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t2, args), p2));
            // 左に戻って下に分割 [3]
            wt_args.extend([";".to_string(), "move-focus".to_string(), "left".to_string()]);
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-H".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t3, args), p3));
            // 右に移動して下に分割 [4]
            wt_args.extend([";".to_string(), "move-focus".to_string(), "right".to_string()]);
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-H".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t4, args), p4));
        }
        5 => {
            // [1][2][3] / [4][5][空] 2x3グリッド
            let (t1, p1) = &tools_with_paths[0];
            let (t2, p2) = &tools_with_paths[1];
            let (t3, p3) = &tools_with_paths[2];
            let (t4, p4) = &tools_with_paths[3];
            let (t5, p5) = &tools_with_paths[4];
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t1, args), p1));
            // 右に分割 [1][2]
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-V".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t2, args), p2));
            // さらに右に分割 [1][2][3]
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-V".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t3, args), p3));
            // 左端に戻って下に分割 [4]
            wt_args.extend([";".to_string(), "move-focus".to_string(), "left".to_string()]);
            wt_args.extend([";".to_string(), "move-focus".to_string(), "left".to_string()]);
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-H".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t4, args), p4));
            // 右に移動して下に分割 [5]
            wt_args.extend([";".to_string(), "move-focus".to_string(), "right".to_string()]);
            wt_args.extend([";".to_string(), "split-pane".to_string(), "-H".to_string()]);
            wt_args.extend(build_wt_pane_args(&build_tool_command_str(t5, args), p5));
        }
        _ => {
            return Err(anyhow!("Unsupported number of panes: {}", tools_with_paths.len()));
        }
    }

    let status = Command::new("wt.exe")
        .args(&wt_args)
        .status()
        .map_err(|e| anyhow!("Failed to spawn Windows Terminal: {}", e))?;

    if !status.success() {
        return Err(anyhow!("Windows Terminal exited with error"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn spawn_multiple_windows(tools_with_paths: &[(String, PathBuf)], args: &[String]) -> Result<()> {
    for (tool, path) in tools_with_paths {
        spawn_terminal(tool, path, args)?;
    }
    Ok(())
}

// ========================================
// macOS iTerm2 分割ペイン
// ========================================

#[cfg(target_os = "macos")]
fn is_iterm_available() -> bool {
    std::path::Path::new("/Applications/iTerm.app").exists()
}

#[cfg(target_os = "macos")]
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn build_terminal_command_str(tool: &str, path: &Path, args: &[String]) -> String {
    let final_args = prepare_tool_args(tool, args);
    let args_str = final_args
        .iter()
        .map(|a| shell_quote_sh(a))
        .collect::<Vec<_>>()
        .join(" ");

    let cmd = if args_str.is_empty() {
        tool.to_string()
    } else {
        format!("{} {}", tool, args_str)
    };

    format!("cd {}; {}", shell_quote_sh(&path.to_string_lossy()), cmd)
}

#[cfg(target_os = "macos")]
fn try_spawn_iterm_splits(tools_with_paths: &[(String, PathBuf)], args: &[String]) -> Result<()> {
    if !is_iterm_available() {
        return Err(anyhow!("iTerm not found"));
    }

    // レイアウト仕様:
    // 2分割: [1][2] 垂直（左右）
    // 3分割: [1][2] / [3][空] 2x2グリッド
    // 4分割: [1][2] / [3][4] 2x2グリッド
    // 5分割: [1][2][3] / [4][5][空] 2x3グリッド

    let cmds: Vec<String> = tools_with_paths
        .iter()
        .map(|(t, p)| build_terminal_command_str(t, p, args))
        .collect();

    let script = match tools_with_paths.len() {
        1 => {
            format!(
                r#"tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "{}"
    end tell
    activate
end tell"#,
                escape_applescript(&cmds[0])
            )
        }
        2 => {
            // [1][2] 垂直分割
            format!(
                r#"tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "{}"
        set s2 to (split vertically with default profile)
        tell s2
            write text "{}"
        end tell
    end tell
    activate
end tell"#,
                escape_applescript(&cmds[0]),
                escape_applescript(&cmds[1])
            )
        }
        3 => {
            // [1][2] / [3][空] 2x2グリッド
            format!(
                r#"tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "{}"
        set s2 to (split vertically with default profile)
        tell s2
            write text "{}"
        end tell
        set s3 to (split horizontally with default profile)
        tell s3
            write text "{}"
        end tell
    end tell
    activate
end tell"#,
                escape_applescript(&cmds[0]),
                escape_applescript(&cmds[1]),
                escape_applescript(&cmds[2])
            )
        }
        4 => {
            // [1][2] / [3][4] 2x2グリッド
            format!(
                r#"tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "{}"
        set s2 to (split vertically with default profile)
        tell s2
            write text "{}"
            set s4 to (split horizontally with default profile)
            tell s4
                write text "{}"
            end tell
        end tell
        set s3 to (split horizontally with default profile)
        tell s3
            write text "{}"
        end tell
    end tell
    activate
end tell"#,
                escape_applescript(&cmds[0]),
                escape_applescript(&cmds[1]),
                escape_applescript(&cmds[3]),
                escape_applescript(&cmds[2])
            )
        }
        5 => {
            // [1][2][3] / [4][5][空] 2x3グリッド
            format!(
                r#"tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "{}"
        set s2 to (split vertically with default profile)
        tell s2
            write text "{}"
            set s3 to (split vertically with default profile)
            tell s3
                write text "{}"
            end tell
            set s5 to (split horizontally with default profile)
            tell s5
                write text "{}"
            end tell
        end tell
        set s4 to (split horizontally with default profile)
        tell s4
            write text "{}"
        end tell
    end tell
    activate
end tell"#,
                escape_applescript(&cmds[0]),
                escape_applescript(&cmds[1]),
                escape_applescript(&cmds[2]),
                escape_applescript(&cmds[4]),
                escape_applescript(&cmds[3])
            )
        }
        _ => {
            return Err(anyhow!("Unsupported number of panes: {}", tools_with_paths.len()));
        }
    };

    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|e| anyhow!("Failed to execute osascript: {}", e))?;

    if !status.success() {
        return Err(anyhow!("osascript failed"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn_multiple_terminals_macos(tools_with_paths: &[(String, PathBuf)], args: &[String]) -> Result<()> {
    for (tool, path) in tools_with_paths {
        spawn_terminal(tool, path, args)?;
    }
    Ok(())
}
