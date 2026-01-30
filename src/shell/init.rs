use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::shell::{bash, pwsh, zsh};

const LEGACY_MARKER: &str = "# gwe shell integration";
const MARKER_BEGIN: &str = "# gwe shell integration (begin)";
const MARKER_END: &str = "# gwe shell integration (end)";

fn default_home() -> Result<String> {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .context("failed to determine user home directory")
}

pub fn check_installed(profile_path: &Path) -> Result<bool> {
    let existing = fs::read_to_string(profile_path).unwrap_or_default();
    Ok(has_marker(&existing))
}

pub fn uninstall(profile_path: &Path) -> Result<bool> {
    let profile_display = profile_path.display().to_string();
    let existing = fs::read_to_string(profile_path).unwrap_or_default();
    let Some(updated) = remove_marker_block(&existing) else {
        return Ok(false);
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(profile_path)
        .with_context(|| format!("failed to open profile file: {}", profile_display))?;
    file.write_all(updated.as_bytes())?;
    Ok(true)
}

pub fn default_pwsh_profile() -> Result<PathBuf> {
    let home = default_home()?;
    Ok(PathBuf::from(home)
        .join("Documents")
        .join("PowerShell")
        .join("Microsoft.PowerShell_profile.ps1"))
}

pub fn default_bash_profile() -> Result<PathBuf> {
    let home = default_home()?;
    Ok(PathBuf::from(home).join(".bashrc"))
}

pub fn default_zsh_profile() -> Result<PathBuf> {
    let home = default_home()?;
    Ok(PathBuf::from(home).join(".zshrc"))
}

fn append_script(profile_path: &Path, script: &str) -> Result<()> {
    let profile_display = profile_path.display().to_string();

    if let Some(parent) = profile_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create profile directory: {}",
                parent.display()
            )
        })?;
    }

    let existing = fs::read_to_string(profile_path).unwrap_or_default();

    if has_marker(&existing) {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile_path)
        .with_context(|| format!("failed to open profile file: {}", profile_display))?;

    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }

    writeln!(file, "{}", MARKER_BEGIN)?;
    writeln!(file, "{}", script)?;
    writeln!(file, "{}", MARKER_END)?;

    Ok(())
}

pub fn init_pwsh(profile_path: &Path) -> Result<()> {
    append_script(profile_path, &pwsh::script())
}

pub fn init_bash(profile_path: &Path) -> Result<()> {
    append_script(profile_path, &bash::script())
}

pub fn init_zsh(profile_path: &Path) -> Result<()> {
    append_script(profile_path, &zsh::script())
}

fn has_marker(existing: &str) -> bool {
    existing.contains(MARKER_BEGIN) || existing.contains(LEGACY_MARKER)
}

fn remove_marker_block(existing: &str) -> Option<String> {
    if let Some(start) = existing.find(MARKER_BEGIN) {
        if let Some(end_offset) = existing[start..].find(MARKER_END) {
            let mut end = start + end_offset + MARKER_END.len();
            let remainder = &existing[end..];
            if remainder.starts_with("\r\n") {
                end += 2;
            } else if remainder.starts_with('\n') {
                end += 1;
            }
            let mut updated = String::new();
            updated.push_str(&existing[..start]);
            updated.push_str(&existing[end..]);
            return Some(updated);
        }
    }

    if let Some(start) = existing.find(LEGACY_MARKER) {
        let mut updated = existing[..start].to_string();
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        return Some(updated);
    }

    None
}


