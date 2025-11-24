gwe (Git Worktree Extension)
==============================

Windows‑native worktree helper compatible with **wtp** (Worktree Plus), written in Rust. “gwe” stands for **Git Worktree Extension**.

Git worktree を Windows で快適に扱うための CLI ツールです。`wtp` の `.wtp.yml` 設定と挙動にできるだけ追従しつつ、Windows 11 / PowerShell 前提で使いやすくすることを目指しています。  
日本語の README は `README.ja.md` を参照してください。

This project is **based on `wtp` version 2.3.4**.  
Going forward, we do **not** aim for strict compatibility with upstream `wtp`; instead, this repository will evolve with its own extensions and design choices.  
Because of that, **all files in this repository should be treated as potentially modified from the original `wtp` sources.**

> Status: early 0.1.x. The CLI is already useful for daily work, but full wtp compatibility is still in progress.


Features
--------

- **Windows‑first worktree helper**
  - Uses `git.exe` under the hood.
  - Supports Windows‑style paths and drive letters.
- **Almost drop‑in compatible with wtp**
  - Reads the same `.wtp.yml` format (version, `defaults.base_dir`, `hooks.post_create`, …).
  - `add`, `list`, `remove`, `cd` behave very close to wtp.
- **Automatic worktree layout**
  - Branch names like `feature/auth` are mapped to `../worktree/feature/auth` by default.
  - Windows‑forbidden characters in branch names are sanitized (e.g. `feat:bad*name` → `feat_bad_name`).
- **Post‑create hooks**
  - `copy` hooks to copy files (even gitignored ones like `.env`) from the main worktree.
  - `command` hooks to run bootstrap commands (install deps, run migrations, etc.).
- **Rich `list` output with JSON**
  - Human‑friendly table with `PATH`, `BRANCH`, `HEAD`, `STATUS`, `UPSTREAM`, `ABS_PATH`.
  - `gwe list --json` for tooling and PowerShell completion.
- **Shell integration (PowerShell, Bash, Zsh)**
  - `gwe init` appends a small function to your shell profile so that `gwe cd` actually changes the current directory.
  - Tab completion for subcommands and `gwe cd` worktree names.


Requirements
------------

- **OS**: Windows 11 (other modern Windows versions may work, but are not officially tested).
- **Git**: Git for Windows (with `git.exe` on `PATH`).
- **Shell**:
  - PowerShell 7+ (recommended).
  - Git Bash (Bash) / Zsh.
  - Cmd is not supported yet.
- **Rust toolchain** (only if you build from source):
  - Rust stable
  - `cargo`


Installation
------------

### Download prebuilt binary (recommended for most users)

Once you publish a release, the typical distribution looks like:

- `gwe-<version>-x86_64-pc-windows-msvc.zip`

Each archive should contain:

- `gwe.exe`
- `README.md` (this file)
- `LICENSE`

Install steps:

```powershell
# 1. Download the ZIP from this repository's “Releases” page
# 2. Extract it somewhere, for example:
Expand-Archive -Path .\gwe-0.2.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\gwe

# 3. Add that directory to your PATH (once)
[System.Environment]::SetEnvironmentVariable(
  "Path",
  $env:Path + ";C:\tools\gwe",
  "User"
)

# 4. Open a new PowerShell and verify
gwe --help
```

> NOTE: The exact archive name and destination path are just examples. Adjust them according to your release/tag naming.


### Build and install from source

Clone this repository and build inside the `gwe` crate:

```powershell
git clone <this repository>
cd gwe

# Build a release binary
cargo build --release

# Option 1: use the built binary directly
.\target\release\gwe.exe --help

# Option 2: install to ~/.cargo/bin
cargo install --path .
gwe --help
```


Quick Start
-----------

### 1. Prepare a Git repository

Inside a Git repository (or with `--repo` pointing to one), `gwe` auto‑detects the repo root:

```powershell
# In your existing Git repo
cd C:\src\my-project
gwe list --json

# Or from outside the repo
gwe --repo C:\src\my-project list --json
```


### 2. Enable Shell integration (optional but recommended)

If `gwe.exe` is on `PATH`, you can add the `gwe` function and completion to your shell profile with a single command:

```powershell
# Use the default profile for your current shell (auto-detected)
# Supported: pwsh, bash, zsh
gwe init

# Or specify the shell explicitly
gwe init --shell pwsh
gwe init --shell bash
gwe init --shell zsh
```

What this does:

- Creates the profile directory/file if needed.
- Appends a section starting with `# gwe shell integration`.
- Defines a `gwe` function that:
  - Calls the real `gwe.exe`.
  - If the first argument is `cd` and the command succeeds, changes the current directory to the printed path.
- Registers shell completion (ArgumentCompleter in PowerShell, complete function in Bash/Zsh).

After running `gwe init`, open a **new** shell session and try:

```powershell
gwe cd @
gwe cd <TAB>  # completes worktree names
```

If you prefer to manage your profile manually, you can also emit the script and inspect it:

```powershell
gwe shell-init pwsh > gwe.ps1
# or
gwe shell-init bash > gwe.sh
gwe shell-init zsh > gwe.zsh
```


Basic Usage
-----------

### Create a worktree (`add`)

```powershell
# Create a worktree from an existing local or remote branch
gwe add feature/auth

# Create a new branch and worktree
gwe add -b feature/new-feature

# Create a new branch tracking a specific remote branch
gwe add --track origin/feature/remote-only

# Use a specific commit as the base (branch name via -b)
gwe add -b hotfix/urgent abc1234
```

- By default, worktrees are placed under `../worktree` relative to the repo root.
- Branch names with `/` become nested directories (e.g. `feature/auth` → `../worktree/feature/auth`).


### List worktrees (`list`)

```powershell
# Human-friendly table
gwe list

# Example output:
# PATH                      BRANCH           HEAD     STATUS  UPSTREAM       ABS_PATH
# ----                      ------           ----     ------  --------       --------
# @*                        main             c72c7800 clean   origin/main    C:\src\my-project
# feature/auth              feature/auth     def45678 dirty   origin/feature/auth C:\src\my-project\..\worktree\feature\auth

# JSON for tooling or completion
gwe list --json
```

The JSON output roughly looks like this:

```json
[
  {
    "name": "@",
    "branch": "main",
    "head": "c72c7800",
    "status": "clean",
    "upstream": "origin/main",
    "path": "@",
    "abs_path": "C:\\src\\my-project",
    "is_main": true,
    "is_current": true
  }
]
```


### Remove a worktree (`remove`)

```powershell
# Remove a worktree (by display name/branch/directory)
gwe remove feature/auth

# Force removal even if the worktree is dirty
gwe remove --force feature/auth

# Remove worktree and its branch (only if merged)
gwe remove --with-branch feature/auth

# Remove worktree and force-delete the branch
gwe remove --with-branch --force-branch feature/auth
```

Only worktrees managed under `base_dir` are removed; others are left untouched.
You cannot remove the **current** worktree (an error is returned instead).


### Navigate between worktrees (`cd`)

With PowerShell integration enabled (`gwe init`), you can jump between worktrees:

```powershell
# Change to a worktree by its name or branch
gwe cd feature/auth

# Change back to the main worktree
gwe cd @
gwe cd my-project   # repo name also works
```

If `gwe` cannot find the requested worktree, it prints a helpful error with a list of available names and suggests running `gwe list`.


### Open in editor (`editor`)

Open the specified worktree (or current directory) in your preferred editor.

```powershell
# Open current worktree
gwe editor

# Open specific worktree
gwe editor feature/auth
```

- Requires `gwe.editor.default` configuration (see Config section below).


### Launch AI tool (`ai`)

Open the specified worktree (or current directory) in your preferred AI tool.

```powershell
# Open current worktree
gwe ai

# Open specific worktree
gwe ai feature/auth

# Pass arguments
gwe ai -- -n
```

- Requires `gwe.ai.default` configuration.


### Configuration Management (`config`)

Manage `gwe` (and `git`) configuration values directly.

```powershell
# Set default editor (global)
gwe config set --global gwe.editor.default "code"

# Set default AI tool (global)
gwe config set --global gwe.ai.default "cursor"

# Get a value
gwe config get gwe.editor.default

# Unset a value
gwe config unset --global gwe.editor.default
```

Configuration: .wtp.yml
-----------------------

`gwe` reads `.wtp.yml` at the repository root and is designed to be compatible with wtp’s configuration format.

### Base directory

```yaml
version: "1.0"
defaults:
  # Base directory for worktrees (relative to repo root, or absolute)
  base_dir: "../worktree"
```

- Relative `base_dir` is resolved from the Git repo root.
- Absolute paths are also supported, even on different drives.


### Hooks

```yaml
version: "1.0"
defaults:
  base_dir: "../worktree"

hooks:
  post_create:
    # Copy gitignored files from main worktree to the new worktree
    - type: copy
      from: ".env"     # relative to the main worktree
      to: ".env"       # relative to the new worktree

    # Run setup commands in the new worktree
    - type: command
      command: "npm ci"
      env:
        NODE_ENV: "development"

    - type: command
      command: "npm run db:setup"
      work_dir: "."
```

Behavior:

- `from` paths are always resolved relative to the **main** worktree.
- `to` paths are resolved relative to the newly created worktree.
- `command` hooks run inside the new worktree, with optional `env` and `work_dir`.
- If any hook fails, the whole `gwe add` command fails.

> **Security note**: `command` hooks execute arbitrary commands defined in `.wtp.yml`.  
> Only enable and run hooks for repositories you trust, and review the hook definitions before using `gwe add`.


Exit Codes
----------

`gwe` uses structured exit codes to distinguish error types:

- `0`: success
- `1`: user errors (invalid arguments, unknown worktree, etc.)
- `2`: configuration errors (invalid `.wtp.yml`)
- `3`: Git command failures
- `10`: unexpected internal errors


Compatibility with wtp
----------------------

While `gwe` starts from `wtp` 2.3.4 and keeps **a good level of compatibility** with that version,  
the long‑term direction is to allow `gwe` to grow its own features and behavior:

- `.wtp.yml` configuration is shared.
- Worktree layout, naming, and most of the `add/list/remove/cd` behavior match closely.
- PowerShell shell integration (`gwe init` / `gwe shell-init pwsh`) mirrors the wtp experience on macOS/Linux.

However, there are still known gaps:

- `shell-init` for `cmd` is not implemented yet.
- Some detailed “helpful error” messages and remote branch resolution logic are less sophisticated than wtp.
- Additional flags specific to wtp (e.g. `list --quiet` / `--compact`) are not currently exposed.

For a detailed, up‑to‑date mapping, see:

- `docs/spec.md`


License
-------

GWE は、MIT License のもとで公開されている [satococoa/wtp](https://github.com/satococoa/wtp) をベースにしたプロジェクトです。
このリポジトリ自体も MIT License で配布されており、詳細な条文は同梱の `LICENSE` を参照してください。  
上流プロジェクト wtp のライセンスについては、wtp リポジトリに含まれる `LICENSE` を参照してください。



