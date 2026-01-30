use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::Config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;
use crate::git::worktree::{WorktreeInfo, list_worktrees};
use crate::worktree::common;

pub fn run(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    target: Option<String>,
) -> Result<()> {
    let resolved = resolve_worktree_path(repo, git, config, target)?;
    
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", resolved.display())?;
    Ok(())
}

fn render_relative_path(path: &Path, repo_root: &Path) -> String {
    let relative = relative_path(repo_root, path);
    relative.to_string_lossy().to_string()
}

fn relative_path(base: &Path, target: &Path) -> PathBuf {
    let base = common::normalize_path(base);
    let target = common::normalize_path(target);

    let base_components: Vec<_> = base.components().collect();
    let target_components: Vec<_> = target.components().collect();
    let mut common_len = 0;

    for (left, right) in base_components.iter().zip(target_components.iter()) {
        if left == right {
            common_len += 1;
        } else {
            break;
        }
    }

    if common_len == 0 {
        return target;
    }

    let mut relative = PathBuf::new();
    for _ in common_len..base_components.len() {
        relative.push("..");
    }
    for component in target_components.iter().skip(common_len) {
        relative.push(component.as_os_str());
    }

    if relative.as_os_str().is_empty() {
        relative.push(".");
    }

    relative
}

#[derive(Debug)]
struct SelectionItem {
    name: String,
    branch: String,
    abs_path: PathBuf,
    is_current: bool,
}

pub fn resolve_worktree_path(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    target: Option<String>,
) -> Result<PathBuf> {
    let worktrees = list_worktrees(git)?;
    let base_dir = common::normalize_path(&config.resolved_base_dir(repo.main_root()));
    let repo_name = repo.repo_name().to_string();
    let current_worktree = common::normalize_path(repo.worktree_root());
    let repo_root = common::normalize_path(repo.main_root());

    let raw_target = match target {
        Some(value) => value,
        None => select_worktree(&worktrees, &base_dir, &current_worktree, &repo_root)?,
    };

    let mut target = sanitize_target(&raw_target);
    if target.is_empty() {
        return Err(AppError::user("worktree name is required").into());
    }

    if target == "-" {
        target = load_last_worktree(git)?;
    }

    let resolved = resolve_path(&worktrees, &base_dir, &repo_name, &target)
        .ok_or_else(|| worktree_not_found(&target, &worktrees, &base_dir, &repo_name))
        .map_err(anyhow::Error::from)?;

    store_last_worktree(git, &worktrees, &base_dir, &current_worktree);

    Ok(common::normalize_path(&resolved))
}

fn sanitize_target(target: &str) -> String {
    target.trim().trim_end_matches('*').to_string()
}

fn select_worktree(
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    current_worktree: &Path,
    repo_root: &Path,
) -> Result<String> {
    let items = build_selection_items(worktrees, base_dir, current_worktree);
    if items.is_empty() {
        return Err(AppError::user("no available worktrees").into());
    }

    let mut name_width = "PATH".len();
    let mut branch_width = "BRANCH".len();
    let mut rendered_names = Vec::with_capacity(items.len());

    for item in &items {
        let mut name = item.name.clone();
        if item.is_current {
            name.push('*');
        }
        name_width = name_width.max(name.len());
        branch_width = branch_width.max(item.branch.len());
        rendered_names.push(name);
    }

    let mut stderr = io::stderr().lock();
    writeln!(stderr, "Select worktree:")?;
    for (idx, item) in items.iter().enumerate() {
        let name = &rendered_names[idx];
        let display_path = render_relative_path(&item.abs_path, repo_root);
        writeln!(
            stderr,
            "  {:>2}) {:<name_width$} {:<branch_width$} {}",
            idx + 1,
            name,
            item.branch,
            display_path,
        )?;
    }
    write!(stderr, "> ")?;
    stderr.flush()?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(AppError::internal_from)?;

    let input = input.trim();
    if input.is_empty() {
        return Err(AppError::user("worktree selection cancelled").into());
    }

    if let Ok(index) = input.parse::<usize>() {
        if index == 0 || index > items.len() {
            return Err(AppError::user("invalid selection").into());
        }
        return Ok(items[index - 1].name.clone());
    }

    Ok(input.to_string())
}

fn build_selection_items(
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    current_worktree: &Path,
) -> Vec<SelectionItem> {
    let mut items = Vec::new();

    for info in worktrees {
        if !info.is_main && !common::is_managed(info, base_dir) {
            continue;
        }

        let abs_path = common::normalize_path(&info.path);
        let is_current = abs_path == current_worktree;
        let name = common::display_name(info, base_dir);
        let branch = info
            .branch
            .clone()
            .unwrap_or_else(|| "detached".to_string());

        items.push(SelectionItem {
            name,
            branch,
            abs_path,
            is_current,
        });
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn load_last_worktree(git: &GitRunner) -> Result<String> {
    match git.run(["config", "--get", "gwe.lastWorktree"]) {
        Ok(output) => {
            let value = output.stdout().trim();
            if value.is_empty() {
                Err(AppError::user("previous worktree not found").into())
            } else {
                Ok(value.to_string())
            }
        }
        Err(_) => Err(AppError::user("previous worktree not found").into()),
    }
}

fn store_last_worktree(
    git: &GitRunner,
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    current_worktree: &Path,
) {
    let name = worktrees.iter().find_map(|info| {
        let abs_path = common::normalize_path(&info.path);
        if abs_path == current_worktree {
            Some(common::display_name(info, base_dir))
        } else {
            None
        }
    });

    if let Some(value) = name {
        let _ = git.run(["config", "gwe.lastWorktree", &value]);
    }
}

fn resolve_path(
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    repo_name: &str,
    target: &str,
) -> Option<PathBuf> {
    for info in worktrees {
        let path = info.path.clone();

        if matches_main(info, repo_name, target) {
            return Some(path);
        }

        if !common::is_managed(info, base_dir) {
            continue;
        }

        if matches_branch(info, target) {
            return Some(path);
        }

        if matches_display_name(info, base_dir, target) {
            return Some(path);
        }

        if matches_directory_name(info, target) {
            return Some(path);
        }
    }

    None
}

fn matches_main(info: &WorktreeInfo, repo_name: &str, target: &str) -> bool {
    if !info.is_main {
        return false;
    }

    if target == "@" || target.eq_ignore_ascii_case("root") {
        return true;
    }

    if target.eq_ignore_ascii_case(repo_name) {
        return true;
    }

    if let Some(branch) = &info.branch {
        if branch == target {
            return true;
        }
    }

    false
}

fn matches_branch(info: &WorktreeInfo, target: &str) -> bool {
    info.branch
        .as_ref()
        .map(|branch| branch == target)
        .unwrap_or(false)
}

fn matches_display_name(info: &WorktreeInfo, base_dir: &Path, target: &str) -> bool {
    if info.is_main {
        return false;
    }

    let display_name = common::display_name(info, base_dir);
    display_name == target
}

fn matches_directory_name(info: &WorktreeInfo, target: &str) -> bool {
    info.path
        .file_name()
        .map(|name| name.to_string_lossy() == target)
        .unwrap_or(false)
}

fn worktree_not_found(
    target: &str,
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    repo_name: &str,
) -> AppError {
    let mut available = Vec::new();
    for info in worktrees {
        if !common::is_managed(info, base_dir) {
            continue;
        }
        available.push(common::display_name(info, base_dir));
    }

    if let Some(main) = worktrees.iter().find(|info| info.is_main) {
        available.insert(0, "@".to_string());
        if let Some(branch) = &main.branch {
            if !available.iter().any(|name| name == branch) {
                available.push(branch.clone());
            }
        }
        if !available
            .iter()
            .any(|name| name.eq_ignore_ascii_case(repo_name))
        {
            available.push(repo_name.to_string());
        }
    }

    available.sort();
    available.dedup();

    let suggestion = if available.is_empty() {
        String::from("Run 'gwe list' to see available worktrees.")
    } else {
        format!(
            "Available worktrees: {}\nRun 'gwe list' to see available worktrees.",
            available.join(", ")
        )
    };

    AppError::user(format!("worktree '{}' not found\n{}", target, suggestion))
}

#[cfg(test)]
mod tests {
    use crate::git::worktree::WorktreeInfo;
    use crate::worktree::common;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    #[test]
    fn sanitize_target_trims_whitespace_and_wildcards() {
        assert_eq!(super::sanitize_target("  feature/*  "), "feature/");
        assert_eq!(super::sanitize_target("@ "), "@");
    }

    #[test]
    fn resolve_path_matches_main_aliases_and_branch_names() {
        let fixture = Fixture::new();
        let worktrees = fixture.worktrees.clone();
        let base_dir = fixture.base_dir.clone();
        let repo = fixture.repo_name.clone();

        let resolved_main = super::resolve_path(&worktrees, &base_dir, &repo, "@").unwrap();
        assert_eq!(common::normalize_path(&resolved_main), fixture.main_path);

        let resolved_repo = super::resolve_path(&worktrees, &base_dir, &repo, &repo).unwrap();
        assert_eq!(common::normalize_path(&resolved_repo), fixture.main_path);

        let resolved_branch =
            super::resolve_path(&worktrees, &base_dir, &repo, "feature/auth").unwrap();
        assert_eq!(common::normalize_path(&resolved_branch), fixture.feature_path);
    }

    #[test]
    fn resolve_path_matches_display_names() {
        let fixture = Fixture::new();
        let worktrees = fixture.worktrees.clone();
        let base_dir = fixture.base_dir.clone();

        let resolved =
            super::resolve_path(&worktrees, &base_dir, &fixture.repo_name, &fixture.feature_display)
                .unwrap();
        assert_eq!(common::normalize_path(&resolved), fixture.feature_path);
    }

    #[test]
    fn worktree_not_found_lists_available_options() {
        let fixture = Fixture::new();
        let err =
            super::worktree_not_found("ghost", &fixture.worktrees, &fixture.base_dir, "repo");
        let message = format!("{err}");
        assert!(
            message.contains("Available worktrees"),
            "expected suggestions, got: {message}"
        );
        assert!(message.contains("Run 'gwe list'"));
        assert!(message.contains("@"));
    }

    #[derive(Debug)]
    struct Fixture {
        #[allow(dead_code)]
        temp: TempDir,
        base_dir: PathBuf,
        repo_name: String,
        worktrees: Vec<WorktreeInfo>,
        main_path: PathBuf,
        feature_path: PathBuf,
        feature_display: String,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = TempDir::new().expect("temp dir");
            let repo_root = temp.path().join("repo");
            fs::create_dir_all(&repo_root).expect("repo root");
            let base_dir = repo_root.join("worktree");
            fs::create_dir_all(&base_dir).expect("base dir");
            let feature_dir = base_dir.join("feature").join("auth");
            fs::create_dir_all(&feature_dir).expect("feature dir");
            let bugfix_dir = base_dir.join("bugfix").join("one");
            fs::create_dir_all(&bugfix_dir).expect("bugfix dir");

            let main = make_info(&repo_root, Some("main"), true);
            let feature = make_info(&feature_dir, Some("feature/auth"), false);
            let bugfix = make_info(&bugfix_dir, Some("bugfix/one"), false);

            let feature_display = common::display_name(&feature, &base_dir);

            Self {
                temp,
                base_dir,
                repo_name: "repo".to_string(),
                worktrees: vec![main.clone(), feature.clone(), bugfix],
                main_path: common::normalize_path(&main.path),
                feature_path: common::normalize_path(&feature.path),
                feature_display,
            }
        }
    }

    fn make_info(path: &Path, branch: Option<&str>, is_main: bool) -> WorktreeInfo {
        WorktreeInfo {
            path: path.to_path_buf(),
            head: "0123456789abcdef".to_string(),
            branch: branch.map(|b| b.to_string()),
            is_main,
            is_detached: branch.is_none(),
            locked: None,
            prunable: None,
        }
    }
}
