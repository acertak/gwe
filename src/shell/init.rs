use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::shell::{bash, pwsh, zsh};

fn default_home() -> Result<String> {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .context("failed to determine user home directory")
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

    if existing.contains("# gwe shell integration") {
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

    writeln!(file, "# gwe shell integration")?;
    writeln!(file, "{}", script)?;

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


