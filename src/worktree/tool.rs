use std::process::Command;
use anyhow::{Result, anyhow};
use crate::cli::{EditorCommand, AiCommand};
use crate::config::Config;
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;
use crate::worktree::resolve;

pub fn open_editor(repo: &RepoContext, git: &GitRunner, config: &Config, cmd: &EditorCommand) -> Result<()> {
    let target_path = resolve::resolve_worktree_path(repo, git, config, cmd.target.clone())?;
    let editor = cmd.editor.clone().or_else(|| config.editor.default.clone())
        .ok_or_else(|| anyhow!("No editor configured. Set gwe.editor.default or use --editor"))?;

    run_tool(&editor, &target_path, &[])
}

pub fn launch_editor(config: &Config, path: &std::path::Path) -> Result<()> {
    let editor = config.editor.default.clone()
        .ok_or_else(|| anyhow!("No editor configured. Set gwe.editor.default"))?;
    run_tool(&editor, path, &[])
}

pub fn run_ai(repo: &RepoContext, git: &GitRunner, config: &Config, cmd: &AiCommand) -> Result<()> {
    let target_path = resolve::resolve_worktree_path(repo, git, config, cmd.target.clone())?;
    let tool = cmd.ai.clone().or_else(|| config.ai.default.clone())
        .ok_or_else(|| anyhow!("No AI tool configured. Set gwe.ai.default or use --ai"))?;

    run_tool(&tool, &target_path, &cmd.args)
}

fn run_tool(tool: &str, path: &std::path::Path, args: &[String]) -> Result<()> {
    let mut command = if cfg!(windows) {
         let mut c = Command::new("cmd");
         c.arg("/C").arg(tool);
         c
    } else {
         // Try direct execution if it's a simple command name, otherwise shell
         // But determining "simple" is hard. gtr uses "eval $EDITOR_CMD" or similar.
         // Let's use direct execution for now assuming it's in PATH.
         Command::new(tool)
    };
    
    // Many editors (code, cursor) accept path as argument.
    command.arg(path);
    command.args(args);
    
    command.current_dir(path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow!("Tool {} exited with status {}", tool, status));
    }
    Ok(())
}
