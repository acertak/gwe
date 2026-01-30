use clap::Parser;

pub mod cli;
pub mod config;
pub mod error;
pub mod git;
pub mod hooks;
pub mod logging;
pub mod shell;
pub mod worktree;

use anyhow::{Result, anyhow};
use std::io::{self, Write};
use std::process::ExitCode;

pub fn run() -> Result<ExitCode> {
    let cli = cli::Cli::parse();
    let globals = cli.global.clone();
    logging::init(&globals)?;

    match cli.command {
        cli::Command::List(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::list::run(
                &repo,
                &git,
                &config,
                worktree::list::ListOptions {
                    json: cmd.json,
                    completion: cmd.completion,
                },
            )?;
        }
        cli::Command::Rm(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::rm::run(&repo, &git, &config, &cmd)?;
        }
        cli::Command::Cd(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::resolve::run(&repo, &git, &config, cmd.target)?;
        }
        cli::Command::Config(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            shell::config::run(&repo, cmd)?;
        }
        cli::Command::Add(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_add_command(&repo, &git, &config, &cmd)?;
        }
        cli::Command::Cursor(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_tool_command(&repo, &git, &config, &cmd, "cursor")?;
        }
        cli::Command::Wind(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_tool_command(&repo, &git, &config, &cmd, "windsurf")?;
        }
        cli::Command::Anti(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_tool_command(&repo, &git, &config, &cmd, "antigravity")?;
        }
        cli::Command::Claude(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_terminal_tool_command(&repo, &git, &config, &cmd, "claude")?;
        }
        cli::Command::Codex(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_terminal_tool_command(&repo, &git, &config, &cmd, "codex")?;
        }
        cli::Command::Gemini(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_terminal_tool_command(&repo, &git, &config, &cmd, "gemini")?;
        }
        cli::Command::Edit(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_default_editor(&repo, &git, &config, &cmd)?;
        }
        cli::Command::RunCli(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_default_cli(&repo, &git, &config, &cmd)?;
        }
        cli::Command::Cli(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::tool::run_multi_cli(&repo, &git, &config, &cmd)?;
        }
        cli::Command::Init(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let profile = match cmd.shell {
                cli::ShellKind::Pwsh => match &cmd.profile {
                    Some(path) => path.clone(),
                    None => shell::init::default_pwsh_profile()?,
                },
                cli::ShellKind::Bash => match &cmd.profile {
                    Some(path) => path.clone(),
                    None => shell::init::default_bash_profile()?,
                },
                cli::ShellKind::Zsh => match &cmd.profile {
                    Some(path) => path.clone(),
                    None => shell::init::default_zsh_profile()?,
                },
                cli::ShellKind::Cmd => {
                    return Err(anyhow!("shell 'cmd' is not supported yet"));
                }
            };

            if cmd.check {
                let installed = shell::init::check_installed(&profile)?;
                if installed {
                    println!("Shell integration is installed: {}", profile.display());
                } else {
                    return Err(anyhow!("Shell integration is not installed: {}", profile.display()));
                }
                return Ok(ExitCode::SUCCESS);
            }

            if cmd.uninstall {
                let removed = shell::init::uninstall(&profile)?;
                if removed {
                    println!("Shell integration removed: {}", profile.display());
                } else {
                    println!("Shell integration not found: {}", profile.display());
                }
                return Ok(ExitCode::SUCCESS);
            }

            // Set default configuration automatically (install only)
            let runner = git::runner::GitRunner::new(repo.clone());
            let _ = runner.run(["config", "--global", "gwe.defaultEditor", "cursor"]);
            let _ = runner.run(["config", "--global", "gwe.defaultCli", "claude"]);
            eprintln!("Set default configuration: editor=cursor, cli=claude");
            eprintln!("To change defaults, edit global gitconfig or run: gwe config set --global gwe.defaultEditor <EDITOR>");

            match cmd.shell {
                cli::ShellKind::Pwsh => shell::init::init_pwsh(&profile)?,
                cli::ShellKind::Bash => shell::init::init_bash(&profile)?,
                cli::ShellKind::Zsh => shell::init::init_zsh(&profile)?,
                cli::ShellKind::Cmd => {
                    return Err(anyhow!("shell 'cmd' is not supported yet"));
                }
            }
        }
        cli::Command::ShellInit(cmd) => match cmd.shell {
            cli::ShellKind::Pwsh => {
                print!("{}", shell::pwsh::script());
                io::stdout().flush()?;
            }
            cli::ShellKind::Bash => {
                print!("{}", shell::bash::script());
                io::stdout().flush()?;
            }
            cli::ShellKind::Zsh => {
                print!("{}", shell::zsh::script());
                io::stdout().flush()?;
            }
            cli::ShellKind::Cmd => {
                return Err(anyhow!("shell 'cmd' is not supported yet"));
            }
        },
    }
    Ok(ExitCode::SUCCESS)
}
