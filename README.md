gwe (Git Worktree Extension)
==============================

Windows‑native worktree helper written in Rust. "gwe" stands for **Git Worktree Extension**.

Git worktree を Windows で快適に扱うための CLI ツールです。Windows 11 / PowerShell 前提で使いやすくすることを目指しています。  
日本語の README は `README.ja.md` を参照してください。

> Status: 0.3.x. The CLI is ready for daily use.

> **Notice:**
> Until v0.2.0, this tool was based on `wtp` (Git Worktree Pro). Since v0.3.0, it has been rewritten as an original implementation, and the command name has been changed from `wtw` to `gwe`.


Features
--------

- **Windows‑first worktree helper**
  - Uses `git.exe` under the hood.
  - Supports Windows‑style paths and drive letters.
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
Expand-Archive -Path .\gwe-0.3.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\gwe

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

### Launch Tools & Create Worktrees

Open a worktree with your favorite editor or AI tool.
If the specified worktree does not exist, it will be created automatically.

```powershell
# Create/Open worktree from an existing branch
gwe cursor feature/auth

# Create a new branch and worktree
gwe cursor -b feature/new-feature

# Create a new branch tracking a remote branch
gwe claude --track origin/feature/remote -b feature/local

# Use a specific commit as base
gwe wind -b hotfix/urgent abc1234
```

**Available Commands:**

- **Editors**: `gwe cursor`, `gwe wind` (Windsurf), `gwe anti` (Antigravity)
- **AI CLI**: `gwe claude`, `gwe codex`, `gwe gemini` (opens in new terminal)
- **Generic**:
  - `gwe -e` (Uses `gwe.defaultEditor`)
  - `gwe -c` (Uses `gwe.defaultCli`)

By default, worktrees are placed under `../worktree` relative to the repo root.


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


### Configuration Management (`config`)

Manage `gwe` (and `git`) configuration values directly.

```powershell
# Get a value
gwe config get gwe.worktrees.dir

# Set a value
gwe config set gwe.worktrees.dir "../worktree"

# Unset a value
gwe config unset gwe.worktrees.dir
```

Configuration
-------------

GWE is configured via Git configuration variables (`gwe.*`). You can manage them with standard `git config` or the `gwe config` helper.

### Base directory

```powershell
# Set base directory for worktrees (relative to repo root, or absolute)
gwe config set gwe.worktrees.dir "../worktree"
```

- Relative paths are resolved from the Git repo root.
- Absolute paths are also supported.


### Hooks

You can define hooks to run after worktree creation using git config.

```powershell
# Copy a file pattern (glob) from main worktree to new worktree
gwe config add gwe.copy.include "*.env"

# Run a command after creation
gwe config add gwe.hook.postcreate "npm ci"
```


Exit Codes
----------

`gwe` uses structured exit codes to distinguish error types:

- `0`: success
- `1`: user errors (invalid arguments, unknown worktree, etc.)
- `2`: configuration errors
- `3`: Git command failures
- `10`: unexpected internal errors


License
-------

MIT License. See `LICENSE` file for details.
