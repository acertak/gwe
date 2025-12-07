use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli::ToolCommand;
use crate::config::Config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::{GitError, GitRunner};
use crate::git::worktree::{WorktreeInfo, list_worktrees};
use crate::hooks::executor::HookExecutor;
use crate::worktree::common;

struct AddSpec {
    path: PathBuf,
    branch: Option<String>,
    commitish: Option<String>,
    track: bool,
    display_name: String,
}

/// 指定された worktree が存在すればそのパスを返し、
/// 存在せず新規作成が必要なら作成してパスを返す。
/// 作成もしない場合は None を返す。
pub fn ensure_worktree(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
) -> Result<PathBuf> {
    let existing = list_worktrees(git)?;

    // 1. まずは既存の worktree を解決してみる (既存の resolve ロジック相当)
    // ただし resolve_worktree_path は Not Found エラーを返すので、ここでは簡易チェックを行うか、
    // あるいは一度 resolve を試みるのが良い。
    // しかし ToolCommand から target が None の場合 (@) は作成しないので、
    // 既存解決を優先する。

    // -b が指定されている場合は常に新規作成を試みる
    if cmd.branch.is_some() {
        return create_new_worktree(repo, git, config, cmd, &existing);
    }

    let target_name = cmd.target.clone().unwrap_or_else(|| "@".to_string());
    
    // 既存解決を試みる
    // target が指定されていない(None -> @)場合も既存解決(main)される
    if let Ok(path) = crate::worktree::resolve::resolve_worktree_path(repo, git, config, Some(target_name.clone())) {
        return Ok(path);
    }

    // 解決できなかった場合、かつ target が指定されているなら新規作成を試みる
    if let Some(target) = &cmd.target {
        if target == "@" || target == "root" {
            return Err(AppError::user("Main worktree not found").into());
        }
        // ここに来るのは、target が既存の worktree 名でもブランチ名でもない場合、
        // あるいは既存ブランチだが worktree 化されていない場合。
        // add <BRANCH> 相当として扱う
        return create_new_worktree(repo, git, config, cmd, &existing);
    }

    // target なし、branch なし、かつ @ も解決できない（ありえないが）場合はエラー
    Err(AppError::user("worktree not found").into())
}

fn create_new_worktree(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &ToolCommand,
    existing: &[WorktreeInfo],
) -> Result<PathBuf> {
    let spec = build_spec(repo, config, cmd, existing)?;

    ensure_parents_exist(&spec.path)?;
    run_git_add(git, &spec)?;

    let mut stdout = io::stdout().lock();
    let display_path = common::normalize_path(&spec.path);
    writeln!(
        stdout,
        "Created worktree '{}' at {}",
        spec.display_name,
        display_path.display()
    )?;

    let executor = HookExecutor::new(config, repo.main_root());
    executor.execute_post_create_hooks(&mut stdout, &spec.path)?;

    Ok(spec.path)
}

fn build_spec(
    repo: &RepoContext,
    config: &Config,
    cmd: &ToolCommand,
    existing: &[WorktreeInfo],
) -> Result<AddSpec> {
    let base_dir = config.resolved_base_dir(repo.main_root());

    let branch_flag = cmd
        .branch
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let track_flag = cmd
        .track
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let target_arg = cmd
        .target
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let (branch, commitish, track) = if let Some(track) = track_flag {
        let inferred_branch = branch_flag
            .map(|s| s.to_string())
            .or_else(|| infer_branch_from_track(track));
        if inferred_branch.is_none() {
            return Err(AppError::user(
                "--track requires a branch name (use --branch or specify remote/branch)",
            )
            .into());
        }
        (inferred_branch, Some(track.to_string()), true)
    } else if let Some(branch_name) = branch_flag {
        // -b <BRANCH> [COMMITISH] のパターン
        // target があれば commitish として扱う、なければ HEAD (None)
        (
            Some(branch_name.to_string()),
            target_arg.map(|s| s.to_string()),
            false,
        )
    } else if let Some(target) = target_arg {
        // add <BRANCH> のパターン (既存ブランチから作成)
        // target が BRANCH になる
        // この場合、新しいブランチは作成しない (branch = None)
        // commitish として target を使う
        (None, Some(target.to_string()), false)
    } else {
        // -b も target もない場合はエラー（既存解決で処理されているはずだがここに来たらエラー）
        return Err(AppError::user("branch or target is required for creation").into());
    };

    let identifier = branch
        .clone()
        .or_else(|| commitish.clone())
        .ok_or_else(|| AppError::user("unable to determine worktree name"))
        .map_err(anyhow::Error::from)?;

    let relative = branch_to_relative_path(&identifier);
    if relative.components().next().is_none() {
        return Err(AppError::user(format!(
            "worktree name resolves to an empty path: {}",
            identifier
        ))
        .into());
    }

    // Use the repository directory name as the first path component
    let repo_name = repo
        .main_root()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");

    let final_relative = PathBuf::from(repo_name).join(&relative);

    let path = base_dir.join(&final_relative);
    detect_conflicts(&path, branch.as_deref(), existing)?;

    let display_name = branch.clone().unwrap_or_else(|| identifier.clone());

    Ok(AddSpec {
        path,
        branch,
        commitish,
        track,
        display_name,
    })
}

fn infer_branch_from_track(track: &str) -> Option<String> {
    track
        .split_once('/')
        .map(|(_, branch)| branch.to_string())
        .filter(|branch| !branch.is_empty())
}

fn branch_to_relative_path(name: &str) -> PathBuf {
    let mut result = PathBuf::new();
    for segment in name.split(['/', '\\']) {
        let sanitized = sanitize_segment(segment);
        if !sanitized.is_empty() {
            result.push(sanitized);
        }
    }
    if result.as_os_str().is_empty() {
        result.push(sanitize_segment(name));
    }
    result
}

fn sanitize_segment(segment: &str) -> String {
    if segment.is_empty() || segment == "." || segment == ".." {
        return "_".to_string();
    }

    let invalid_chars: HashSet<char> = ['<', '>', ':', '"', '|', '?', '*', '\\']
        .into_iter()
        .collect();

    segment
        .chars()
        .map(|ch| if invalid_chars.contains(&ch) { '_' } else { ch })
        .collect()
}

fn detect_conflicts(path: &Path, branch: Option<&str>, existing: &[WorktreeInfo]) -> Result<()> {
    if let Some(branch_name) = branch {
        if let Some(conflict) = existing
            .iter()
            .find(|wt| wt.branch.as_deref() == Some(branch_name))
        {
            return Err(AppError::user(format!(
                "worktree for branch '{}' already exists: {}",
                branch_name,
                conflict.path.display()
            ))
            .into());
        }
    }

    let target_normalized = common::normalize_path(path);

    if existing
        .iter()
        .map(|wt| common::normalize_path(&wt.path))
        .any(|existing_path| existing_path == target_normalized)
    {
        return Err(AppError::user(format!(
            "worktree path already exists in git metadata: {}",
            path.display()
        ))
        .into());
    }

    if path.exists() {
        return Err(AppError::user(format!(
            "destination path already exists: {}",
            path.display()
        ))
        .into());
    }

    Ok(())
}

fn ensure_parents_exist(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn run_git_add(git: &GitRunner, spec: &AddSpec) -> Result<()> {
    let mut args: Vec<OsString> = Vec::new();
    args.push("worktree".into());
    args.push("add".into());

    if spec.track {
        args.push("--track".into());
    }

    if let Some(branch) = &spec.branch {
        args.push("-b".into());
        args.push(branch.clone().into());
    }

    args.push(spec.path.to_string_lossy().into_owned().into());

    if let Some(commitish) = &spec.commitish {
        args.push(commitish.clone().into());
    }

    match git.run(args) {
        Ok(_) => Ok(()),
        Err(GitError::CommandFailed { stderr, .. }) => {
            let message = stderr.trim();
            if message.is_empty() {
                Err(AppError::git("git worktree add failed without error output").into())
            } else {
                Err(AppError::git(message.to_string()).into())
            }
        }
        Err(err) => Err(AppError::git(err.to_string()).into()),
    }
}
