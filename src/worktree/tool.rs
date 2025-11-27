use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};

use crate::cli::ToolCommand;
use crate::config::Config;
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;
use crate::worktree::resolve;

pub fn run_tool_command(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
    tool_name: &str,
) -> Result<()> {
    let target = cmd.target.clone().or_else(|| Some("@".to_string()));
    let target_path = resolve::resolve_worktree_path(repo, git, config, target)?;

    run_tool(tool_name, &target_path, &cmd.args)
}

// For 'gwe add --open'
pub fn launch_editor(config: &Config, path: &Path) -> Result<()> {
    // Default to "cursor" for newly created worktrees
    run_tool("cursor", path, &[])
}

fn run_tool(tool: &str, path: &Path, args: &[String]) -> Result<()> {
    let mut command = if cfg!(windows) {
         let mut c = Command::new("cmd");
         c.arg("/C").arg(tool);
         c
    } else {
         Command::new(tool)
    };
    
    command.arg(path);
    command.args(args);
    
    let status = command.status()
        .map_err(|e| anyhow!("Failed to execute tool '{}': {}", tool, e))?;
        
    if !status.success() {
        return Err(anyhow!("Tool '{}' exited with status {}", tool, status));
    }
    Ok(())
}
