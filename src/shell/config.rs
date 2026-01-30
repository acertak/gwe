use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli::{ConfigAction, ConfigCommand};
use crate::config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;

pub fn run(repo: &RepoContext, cmd: ConfigCommand) -> Result<()> {
    let runner = GitRunner::new(repo.clone());
    
    match cmd.action {
        ConfigAction::List => list_config(repo, &runner),
        ConfigAction::Doctor => doctor_config(repo, &runner),
        ConfigAction::Get { key } => {
            // git config --get-all <key>
            let _ = match runner.run(["config", "--get-all", &key]) {
                Ok(output) => print!("{}", output.stdout),
                Err(_) => {
                    // Ignore error if key not found (git exits with 1)
                }
            };
            Ok(())
        }
        ConfigAction::Set { key, value, global } => {
            let value_str = value.join(" ");
            let mut args = vec!["config"];
            if global {
                args.push("--global");
            }
            args.push(&key);
            args.push(&value_str);
            runner.run(args)?;
            Ok(())
        }
        ConfigAction::Add { key, value, global } => {
            let value_str = value.join(" ");
            let mut args = vec!["config", "--add"];
            if global {
                args.push("--global");
            }
            args.push(&key);
            args.push(&value_str);
            runner.run(args)?;
            Ok(())
        }
        ConfigAction::Unset { key, global } => {
            let mut args = vec!["config", "--unset"];
            if global {
                args.push("--global");
            }
            args.push(&key);
            // Ignore error if key doesn't exist
            let _ = runner.run(args);
            Ok(())
        }
    }
}

fn list_config(repo: &RepoContext, runner: &GitRunner) -> Result<()> {
    let config = config::load_config(repo)?;
    let entries = read_gwe_entries(runner);
    let mut rows = Vec::new();

    let base_dir_value = config.defaults.base_dir.to_string_lossy().to_string();
    rows.push(ListRow::new(
        "gwe.worktrees.dir",
        base_dir_value,
        source_label(&entries, "gwe.worktrees.dir", Some("default")),
    ));

    rows.push(ListRow::new(
        "gwe.defaultBranch",
        config.default_branch.clone().unwrap_or_else(|| "-".to_string()),
        source_label(&entries, "gwe.defaultbranch", None),
    ));
    rows.push(ListRow::new(
        "gwe.defaultEditor",
        config.default_editor.clone().unwrap_or_else(|| "-".to_string()),
        source_label(&entries, "gwe.defaulteditor", None),
    ));
    rows.push(ListRow::new(
        "gwe.defaultCli",
        config.default_cli.clone().unwrap_or_else(|| "-".to_string()),
        source_label(&entries, "gwe.defaultcli", None),
    ));

    let multi_cli = if config.multi_cli.is_empty() {
        "-".to_string()
    } else {
        config.multi_cli.join(", ")
    };
    rows.push(ListRow::new(
        "gwe.multiCli",
        multi_cli,
        source_label(&entries, "gwe.multicli", None),
    ));

    rows.push(ListRow::new(
        "gwe.copy.include",
        join_values(&entries, "gwe.copy.include"),
        source_label(&entries, "gwe.copy.include", None),
    ));
    rows.push(ListRow::new(
        "gwe.hook.postcreate",
        join_values(&entries, "gwe.hook.postcreate"),
        source_label(&entries, "gwe.hook.postcreate", None),
    ));
    rows.push(ListRow::new(
        "gwe.lastWorktree",
        join_values(&entries, "gwe.lastworktree"),
        source_label(&entries, "gwe.lastworktree", None),
    ));

    let known = known_keys();
    for (key, values) in &entries {
        if known.contains(key) {
            continue;
        }
        rows.push(ListRow::new(
            key.as_str(),
            values.join(", "),
            "git".to_string(),
        ));
    }

    output_table(&rows)?;
    Ok(())
}

fn doctor_config(repo: &RepoContext, runner: &GitRunner) -> Result<()> {
    let config = config::load_config(repo)?;
    let mut issues = Vec::new();

    let base_dir = config.resolved_base_dir(repo.main_root());
    if !base_dir.exists() {
        issues.push(format!(
            "worktrees dir does not exist: {}",
            base_dir.display()
        ));
    }

    if let Some(branch) = &config.default_branch {
        if runner
            .run(["rev-parse", "--verify", &format!("refs/heads/{}", branch)])
            .is_err()
        {
            issues.push(format!("default branch not found: {}", branch));
        }
    }

    if let Some(editor) = &config.default_editor {
        if !command_exists(editor) {
            issues.push(format!("default editor not found on PATH: {}", editor));
        }
    }

    if let Some(cli) = &config.default_cli {
        if !command_exists(cli) {
            issues.push(format!("default CLI not found on PATH: {}", cli));
        }
    }

    for tool in &config.multi_cli {
        if !command_exists(tool) {
            issues.push(format!("multiCli tool not found on PATH: {}", tool));
        }
    }

    if issues.is_empty() {
        println!("Config OK");
        return Ok(());
    }

    eprintln!("Config issues found:");
    for issue in &issues {
        eprintln!("- {}", issue);
    }

    Err(AppError::user(format!(
        "config doctor found {} issue(s)",
        issues.len()
    ))
    .into())
}

fn read_gwe_entries(runner: &GitRunner) -> BTreeMap<String, Vec<String>> {
    let mut entries: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let output = match runner.run(["config", "--get-regexp", "^gwe\\."]) {
        Ok(output) => output.stdout,
        Err(_) => return entries,
    };

    for line in output.lines() {
        let mut parts = line.splitn(2, ' ');
        let Some(key) = parts.next() else { continue };
        let value = parts.next().unwrap_or("").to_string();
        let key = key.to_ascii_lowercase();
        entries.entry(key).or_default().push(value);
    }

    entries
}

fn join_values(entries: &BTreeMap<String, Vec<String>>, key: &str) -> String {
    entries
        .get(&key.to_ascii_lowercase())
        .map(|values| values.join(", "))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string())
}

fn source_label(entries: &BTreeMap<String, Vec<String>>, key: &str, default: Option<&str>) -> String {
    if entries.contains_key(&key.to_ascii_lowercase()) {
        "git".to_string()
    } else {
        default.unwrap_or("unset").to_string()
    }
}

fn known_keys() -> BTreeSet<String> {
    [
        "gwe.worktrees.dir",
        "gwe.defaultbranch",
        "gwe.defaulteditor",
        "gwe.defaultcli",
        "gwe.multicli",
        "gwe.copy.include",
        "gwe.hook.postcreate",
        "gwe.lastworktree",
    ]
    .into_iter()
    .map(|value| value.to_string())
    .collect()
}

fn output_table(rows: &[ListRow]) -> Result<()> {
    let mut key_width = "KEY".len();
    let mut value_width = "VALUE".len();
    let mut source_width = "SOURCE".len();

    for row in rows {
        key_width = key_width.max(row.key.len());
        value_width = value_width.max(row.value.len());
        source_width = source_width.max(row.source.len());
    }

    println!(
        "{:<key_width$} {:<value_width$} {:<source_width$}",
        "KEY", "VALUE", "SOURCE"
    );
    println!(
        "{:-<key_width$} {:-<value_width$} {:-<source_width$}",
        "", "", ""
    );

    for row in rows {
        println!(
            "{:<key_width$} {:<value_width$} {:<source_width$}",
            row.key, row.value, row.source
        );
    }

    Ok(())
}

#[derive(Debug)]
struct ListRow {
    key: String,
    value: String,
    source: String,
}

impl ListRow {
    fn new(key: &str, value: String, source: String) -> Self {
        Self {
            key: key.to_string(),
            value,
            source,
        }
    }
}

fn command_exists(name: &str) -> bool {
    if name.contains(std::path::MAIN_SEPARATOR) || Path::new(name).is_absolute() {
        return Path::new(name).exists();
    }

    let path_var = env::var_os("PATH").unwrap_or_default();
    for dir in env::split_paths(&path_var) {
        if candidate_paths(&dir, name).iter().any(|path| path.exists()) {
            return true;
        }
    }

    false
}

#[cfg(windows)]
fn candidate_paths(dir: &Path, name: &str) -> Vec<PathBuf> {
    if Path::new(name).extension().is_some() {
        return vec![dir.join(name)];
    }

    let pathext = env::var_os("PATHEXT").unwrap_or_else(|| ".EXE;.BAT;.CMD;.COM".into());
    let mut paths = Vec::new();
    for ext in pathext.to_string_lossy().split(';') {
        if ext.is_empty() {
            continue;
        }
        paths.push(dir.join(format!("{}{}", name, ext)));
    }
    paths
}

#[cfg(not(windows))]
fn candidate_paths(dir: &Path, name: &str) -> Vec<PathBuf> {
    vec![dir.join(name)]
}
