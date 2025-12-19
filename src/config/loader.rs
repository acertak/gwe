use std::path::PathBuf;

use anyhow::Result;

use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;

use super::types::{Config, Hook, CommandHook, GlobCopyHook};

pub fn load_config(repo: &RepoContext) -> Result<Config> {
    let mut config = Config::default();
    // Override/Augment with git config (gwe.*)
    load_from_git_config(repo, &mut config)?;

    Ok(config)
}

fn load_from_git_config(repo: &RepoContext, config: &mut Config) -> Result<()> {
    let runner = GitRunner::new(repo.clone());
    // Get all gwe.* config values
    // We accept failure (e.g. no config set) by checking status or empty output
    let output = match runner.run(["config", "--get-regexp", "^gwe\\."]) {
        Ok(out) => out,
        Err(_) => return Ok(()), // No config found or git failed, just ignore
    };

    for line in output.stdout.lines() {
        // line format: key value (value can contain spaces)
        // git config output separates key and value by space.
        // key cannot contain spaces.
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0];
        let value = parts[1];

        match key {
            "gwe.worktrees.dir" => {
                config.defaults.base_dir = PathBuf::from(value);
            }
            "gwe.defaultbranch" => {
                config.default_branch = Some(value.to_string());
            }
            "gwe.defaulteditor" => {
                config.default_editor = Some(value.to_string());
            }
            "gwe.defaultcli" => {
                config.default_cli = Some(value.to_string());
            }
            "gwe.multicli" => {
                // カンマまたは空白で分割し、トリミングして空でないもののみを追加
                for v in value.split(|c: char| c == ',' || c.is_whitespace()) {
                    let v = v.trim();
                    if !v.is_empty() {
                        config.multi_cli.push(v.to_string());
                    }
                }
            }
            "gwe.copy.include" => {
                config.hooks.post_create.push(Hook::GlobCopy(GlobCopyHook {
                    pattern: value.to_string(),
                }));
            }
            "gwe.copy.exclude" => {
                // カンマまたは空白で分割し、トリミングして空でないもののみを追加
                for v in value.split(|c: char| c == ',' || c.is_whitespace()) {
                    let v = v.trim();
                    if !v.is_empty() {
                        config.copy_exclude.push(v.to_string());
                    }
                }
            }
            "gwe.hook.postcreate" => {
                // Map to CommandHook
                config.hooks.post_create.push(Hook::Command(CommandHook {
                    command: value.to_string(),
                    env: Default::default(),
                    work_dir: None,
                }));
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn returns_default_when_config_missing() {
        let (_dir, repo) = temp_repo();
        let config = load_config(&repo).expect("load config");
        assert_eq!(config.version, types::DEFAULT_VERSION);
        assert_eq!(
            config.defaults.base_dir,
            PathBuf::from(types::DEFAULT_BASE_DIR)
        );
    }

    #[test]
    fn loads_from_git_config() {
        let (_dir, repo) = temp_repo();
        run_git(repo.worktree_root(), &["config", "gwe.defaultBranch", "main"]);

        let config = load_config(&repo).expect("load config");
        
        assert_eq!(config.default_branch, Some("main".to_string()));
    }

    fn temp_repo() -> (TempDir, RepoContext) {
        let dir = TempDir::new().expect("temp repo");
        init_git(dir.path());
        let repo = RepoContext::discover(Some(dir.path().to_path_buf())).expect("repo context");
        (dir, repo)
    }

    fn init_git(path: &Path) {
        run_git(path, &["init", "-q"]);
        run_git(path, &["config", "user.email", "gwe@example.com"]);
        run_git(path, &["config", "user.name", "gwe-test"]);
        // minimal commit so that rev-parse behaves identically to real repos
        fs::write(path.join("README.md"), "init").expect("write file");
        run_git(path, &["add", "README.md"]);
        run_git(path, &["commit", "-q", "-m", "init"]);
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.current_dir(dir);
        command.args(args);
        let status = command.status().expect("git status");
        assert!(status.success(), "git {:?} failed: {:?}", args, status);
    }
}
