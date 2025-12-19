GWE Specification
=================

1. Overview
-----------

GWE (Git Worktree Extension) is a Windows‑native helper CLI for managing Git
worktrees. It is implemented in Rust and is designed to provide a first‑class
experience on Windows 11, PowerShell, Bash, and Zsh.

This document specifies:

- The command‑line interface (global options and subcommands).
- The configuration mechanism (Git config).
- How GWE integrates with Git and `git worktree`.
- The post‑create hook mechanism.
- Shell integration for PowerShell, Bash, and Zsh.
- Logging behavior and exit codes.
- Behavioral guarantees captured by the automated test suite.

All information in this document is derived directly from the source code and
tests in this repository. No speculative or planned behavior is described.


2. Terminology and Concepts
---------------------------

- **Main worktree**  
  The primary worktree of a Git repository (the one corresponding to the
  `.git` directory). In `git worktree list --porcelain`, it is the first
  `worktree` entry. GWE marks this entry as `is_main = true`.

- **Additional worktree**  
  Any worktree managed by `git worktree` other than the main worktree.

- **Base directory (`base_dir`)**  
  The root directory under which GWE manages worktrees by default. It is
  configured via git config (`gwe.worktrees.dir`), and defaults
  to `../worktree` relative to the main repository root.

- **Managed worktree**  
  A worktree whose path is under the configured `base_dir` (or the main
  worktree itself). Functions such as `list`, `rm`, and `cd` treat these
  specially.

- **Display name**  
  The human‑friendly name shown in `gwe list` and used in places where a
  compact identifier is useful. For the main worktree it is `"@"`. For other
  worktrees under `base_dir`, the display name is the relative path from
  `base_dir` to the worktree directory (joined using the platform's path
  separator). If that cannot be determined, the final path component or the
  full path string is used as a fallback.

- **Worktree name (for `cd` and `rm`)**  
  A user‑supplied token that can match:

  - `"@"` (main worktree).
  - `"root"` (case‑insensitive alias for the main worktree).
  - The repository name (the last path component of the main root), case‑insensitive.
  - A branch name (e.g. `feature/auth`).
  - A display name (e.g. `<repo_name>\feature\auth` on Windows).
  - The worktree directory name (final path component).


3. Architecture Overview
------------------------

The crate is structured into modules corresponding to major responsibilities:

- `main`  
  OS entrypoint. Calls `gwe::run()` and maps errors to exit codes.

- `lib`  
  Exposes the `run()` function, parses CLI options via Clap, initializes
  logging, and dispatches to subcommand implementations.

- `cli`  
  Definitions of global options and subcommands using `clap::Parser` and
  `clap::Subcommand`. This is the single source of truth for the CLI surface.

- `config`  
  Configuration loading and representation:

  - `config::loader`: Loading from git config.
  - `config::types`: strongly typed configuration (`Config`, `Defaults`,
    `Hooks`, `Hook`) and effective path resolution (`resolved_base_dir`).

- `git`  
  Integration with Git:

  - `git::rev`: `RepoContext` that discovers the main and worktree roots.
  - `git::runner`: `GitRunner` wrapper around `git.exe` with logging and
    error types.
  - `git::worktree`: parsing of `git worktree list --porcelain` output into
    structured `WorktreeInfo` values.

- `worktree`  
  Implementation of subcommands operating on worktrees:

  - `worktree::create`: worktree creation/reuse (path mapping,
    conflict detection, post‑create hooks).
  - `worktree::list`: `gwe list` behavior (table and JSON output).
  - `worktree::rm`: `gwe rm` behavior (worktree and optional branch removal).
  - `worktree::resolve`: `gwe cd` behavior (name resolution).
  - `worktree::common`: cross‑cutting helpers for path normalization,
    display names, and "managed" checks.
  - `worktree::tool`: `gwe add`, `gwe cursor`, `gwe wind`, `gwe anti`, `gwe claude`, `gwe codex`, `gwe gemini`, `gwe -e`, `gwe -c`, and `gwe cli` behavior (external tool launch / terminal spawning).

- `hooks`  
  The post‑create hooks executor (`HookExecutor`) which runs `copy`,
  `glob_copy`, and `command` hooks defined in configuration.

- `shell`  
  Shell integration:

  - `shell::init`: initialization of shell profiles (`gwe init`).
  - `shell::config`: `gwe config` subcommand (git config wrapper).
  - `shell::pwsh`: PowerShell function and argument completer script.
  - `shell::bash`: Bash shell function script.
  - `shell::zsh`: Zsh shell function script.
  - `shell::cmd`: placeholder (currently unused).

- `logging`  
  Initialization of the `tracing` subscriber based on global verbosity flags.

- `error`  
  Core application error type `AppError` with categories and exit code
  mapping.

- `tests`  
  Integration tests (`tests/*.rs`) which invoke the compiled binary and verify
  CLI behavior. These are treated as executable specification for critical
  flows (add/list/rm/cd/config/shell‑init).


4. CLI Specification
--------------------

4.1 Global Options
~~~~~~~~~~~~~~~~~~

The top‑level CLI is defined in `cli::Cli` and parsed via `clap::Parser`.

Global options (available before any subcommand):

- `-v`, `--verbose` (counting flag)  
  Increases log verbosity. Each occurrence increments an internal counter:

  - `0` (default): log level `WARN`.
  - `1`: log level `DEBUG`.
  - `>= 2`: log level `TRACE`.

- `--quiet`  
  Suppresses most diagnostic output and sets the log level to `ERROR`. This
  conflicts with `--verbose`.

- `--repo <PATH>`  
  Treats the given path as the starting directory for discovering the Git
  worktree root. If the path is relative, it is resolved against the current
  working directory. If the path points to a file, its parent directory is
  used. If the resulting directory does not exist, an error is returned.

  The `--repo` flag is validated in integration tests by running `gwe` from
  outside the repository and checking that `list --json` still succeeds.


4.2 Subcommands
~~~~~~~~~~~~~~~

The `Command` enum defines the available subcommands:

- `add` (`ToolCommand`)
- `list` (`ListCommand`)
- `rm` (`RmCommand`)
- `cd` (`CdCommand`)
- `init` (`InitCommand`)
- `shell-init` (`ShellInitCommand`)
- `config` (`ConfigCommand`)
- `cursor` (`ToolCommand`)
- `wind` (`ToolCommand`)
- `anti` (`ToolCommand`)
- `claude` (`ToolCommand`)
- `codex` (`ToolCommand`)
- `gemini` (`ToolCommand`)
- `-e` (`ToolCommand`) (launch default editor)
- `-c` (`ToolCommand`) (launch default CLI)
- `cli` (`ToolCommand`)

Each subcommand is documented below.


4.2.1 `gwe add`
^^^^^^^^^^^^^^^

**Purpose**  
Create (or reuse) a Git worktree under the configured base directory. If a new
worktree is created, post‑create hooks are executed. On success, prints the
resolved absolute path of the worktree to standard output.
**Synopsis**

```text
gwe add [WORKTREE] [OPTIONS]
```

**Options (ToolCommand)**

- `WORKTREE` (positional, optional)  
  - If `--branch` is specified, this is treated as the starting point
    (commitish) for the new branch and worktree (optional).
  - Otherwise, this is required and is used as a branch name or commitish.

- `-b, --branch <BRANCH>`  
  Name of the new branch to create for the worktree. When provided:

  - The new worktree is created at the path derived from the branch name.
  - The positional `BRANCH_OR_COMMIT` argument, if present, is used as the
    starting commit.

- `--track <REMOTE/BRANCH>`  
  Remote tracking branch to use when creating the worktree. This value is
  passed as the commitish to `git worktree add`. The local branch name is
  inferred from the remote/branch string unless `--branch` is explicitly
  supplied.

**Argument validation**

Behavior is determined from the combination of `--branch`, `--track`, and
`BRANCH_OR_COMMIT`:

1. If `--track` is supplied:

   - The branch name is either:
     - Provided explicitly via `--branch`, or
     - Inferred from the part after the first `/` in `REMOTE/BRANCH`
       (e.g. `origin/feature/auth` → `feature/auth`).
   - If no branch name can be determined, the command fails with a user error:
     `"--track requires a branch name (use --branch or specify remote/branch)"`.

2. Else if `--branch` is supplied:

   - The branch name is taken from `--branch`.
   - The commitish for `git worktree add` is taken from `WORKTREE`
     if present; otherwise it is `None`, and Git will use its own defaults
     for branch creation from the worktree root.

3. Else (no `--track` and no `--branch`):

   - `WORKTREE` is required. If it is missing or blank, the command
     fails with a user error: `"branch or commit is required"`.
   - The commitish is set to the provided value.
   - No new branch is created by GWE; `git worktree add` is invoked with only
     the path and commitish.

These error messages and exit code 1 (user error) are verified in tests.

**Worktree path derivation**

The effective base directory is `config.resolved_base_dir(main_root)`, where
`main_root` is the main repository root discovered by `RepoContext`. This
uses:

- The configured `gwe.worktrees.dir` or `defaults.base_dir` when present.
- The default `../worktree` when absent.
- Relative `base_dir` resolved against `main_root`; absolute `base_dir`
  left as‑is.

Within the base directory, GWE derives a relative path from the branch or
commit identifier:

- The path includes the repository name as the first component:
  `base_dir/repo_name/branch_path`.
- The repository name is derived from the main repository root directory name.
- The branch/commit identifier is split on `/` and `\`.
- Each segment is sanitized:
  - Empty segments, `"."`, or `".."` become `"_"`.
  - Windows‑forbidden characters `<`, `>`, `:`, `"`, `|`, `?`, `*`, and `\`
    are replaced with `_`.
- Non‑empty sanitized segments are joined as path components.
- If sanitization results in an empty relative path, a fallback is used by
  sanitizing the entire identifier as a single segment.

The final worktree path is `base_dir/repo_name/relative_path`.

**Conflict detection**

Before creating the worktree, GWE checks for conflicts using:

1. Existing worktrees from `git worktree list --porcelain`.
2. The filesystem.

Checks:

- If a branch name is known and any existing `WorktreeInfo` has the same
  branch, the command fails with a user error:

  ```text
  worktree for branch '<branch>' already exists: <existing_path>
  ```

- If any existing worktree's path (normalized) matches the target path
  (normalized), the command fails with a user error:

  ```text
  worktree path already exists in git metadata: <path>
  ```

- If the target path already exists in the filesystem, the command fails with:

  ```text
  destination path already exists: <path>
  ```

- If the derived worktree name resolves to an empty path, the command fails
  with:

  ```text
  worktree name resolves to an empty path: <identifier>
  ```

**Git invocation**

GWE constructs arguments to `git worktree add` as follows:

- Always: `["worktree", "add"]`.
- If tracking: appends `"--track"`.
- If a branch name is set: appends `"-b"` and the branch name.
- Appends the worktree path.
- If a commitish is set: appends the commitish.

The command is executed via `GitRunner::run`. If Git exits with a non‑success
status, GWE:

- Extracts `stderr`, trims it, and:
  - If non‑empty, surfaces it as a Git error message.
  - If empty, surfaces `"git worktree add failed without error output"`.

These failures are treated as Git errors and mapped to exit code 3.

**User‑visible output**

When a new worktree is created, GWE prints a progress line:

```text
Created worktree '<display_name>' at <absolute_path>
```

After that, it executes post‑create hooks (see section 6). The hooks executor
prints progress messages and hook‑specific output.

If any hook fails, `gwe add` fails (the error is propagated and printed by
`main`). Hook failures are treated as internal errors and mapped to exit code
10.

Finally, a successful `gwe add` prints the **normalized absolute path** of the
resolved worktree as a single line. If an existing worktree is resolved, it
prints only this single line (no creation message, no hooks).


4.2.2 `gwe list`
^^^^^^^^^^^^^^^^

**Purpose**  
List worktrees associated with the current repository, either in a
human‑readable table or as JSON suitable for tooling and completion.

**Synopsis**

```text
gwe list [--json]
```

**Options (ListCommand)**

- `--json`  
  Output JSON instead of a formatted table.

**Data collection**

`gwe list` performs the following steps:

1. Calls `git worktree list --porcelain` via `GitRunner`.
2. Parses the output into `WorktreeInfo` entries. Each entry contains:

   - `path` (absolute, canonicalized path).
   - `head` (full commit hash as reported by Git).
   - `branch` (optional branch name; omitted in detached HEAD).
   - `is_main` (the first parsed entry is flagged as main).
   - `is_detached` (true if the `detached` line appears).
   - `locked` (optional reason from a `locked` line).
   - `prunable` (optional reason from a `prunable` line).

3. Determines the effective base directory: `config.resolved_base_dir(main_root)`.
4. Determines the current worktree path from `RepoContext::worktree_root()`.
5. For each worktree, builds a `DisplayRow` with:

   - `name` (display name; section 2).
   - `branch_display`:
     - Branch name if present.
     - `"detached"` otherwise.
   - `branch` (the raw branch name, if any).
   - `head` (shortened commit hash, first 8 characters when longer).
   - `status`: `"clean"` or `"dirty"`, determined by:

     - Running `git status --short` in the worktree.
     - Treating an empty output as `"clean"`, otherwise `"dirty"`.

   - `upstream`: optional upstream reference, determined by:

     - Running `git rev-parse --abbrev-ref --symbolic-full-name @{u}` in the
       worktree.
     - If the command succeeds, using its trimmed stdout (if non‑empty).
     - If the command fails due to a Git command error (e.g. no upstream
       configured), treating it as `None`.

   - `abs_path`: normalized absolute path string.
   - `is_main`: as above.
   - `is_current`: `true` if the worktree path matches the current worktree
     path (after normalization).

These behaviors are validated by the integration tests in `tests/list_spec.rs`.

**Table output (default)**

The table is printed with dynamically sized columns. The headers are:

```text
PATH  BRANCH  HEAD  STATUS  UPSTREAM  ABS_PATH
```

For each row:

- `PATH` contains the display name.
- If the worktree is the current worktree, an asterisk `*` is appended to the
  display name (e.g. `feature\current*`).
- `BRANCH` contains `branch_display`.
- `HEAD` contains the shortened commit hash.
- `STATUS` contains `"clean"` or `"dirty"`.
- `UPSTREAM` contains the upstream string or `"-"` if none.
- `ABS_PATH` contains the normalized absolute path string.

**JSON output**

When `--json` is provided, `gwe list` emits pretty‑printed JSON:

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

Fields:

- `name`: display name (e.g. `"@"`, `"<repo_name>\\feature\\auth"`).
- `branch`: optional branch name.
- `head`: short commit hash (up to 8 characters).
- `status`: `"clean"` or `"dirty"`.
- `upstream`: optional upstream reference string.
- `path`: same as `name` (logical path).
- `abs_path`: absolute filesystem path.
- `is_main`: whether this is the main worktree.
- `is_current`: whether this is the current worktree.


4.2.3 `gwe rm`
^^^^^^^^^^^^^^

**Purpose**  
Remove a managed worktree, and optionally remove its corresponding branch.

**Synopsis**

```text
gwe rm [OPTIONS] <WORKTREE>
```

**Options (RmCommand)**

- `WORKTREE` (positional, required)  
  Target worktree identifier. Resolution follows the same rules as for
  `gwe cd` (see section 4.2.4), except that the main worktree is never
  removable.

- `-b, --with-branch`  
  After removing the worktree, also remove its local branch if one is
  associated.

**Target resolution**

`gwe rm`:

1. Enumerates `WorktreeInfo` entries from `git worktree list --porcelain`.
2. Computes the effective `base_dir`.
3. Skips:
   - The main worktree (`is_main == true`).
   - Any worktree that is not "managed" (its path is not under `base_dir`).
4. Attempts to match the target string against each remaining worktree in
   this order:

   - The branch name (`info.branch`).
   - The worktree directory name (final path component).
   - The display name (relative path under `base_dir`).

If a match is found, that worktree is the removal target.  
If no match is found, a user error is returned with a message of the form:

```text
worktree '<target>' not found
Available worktrees: <name1>, <name2>, ...
Run 'gwe list' to see available worktrees.
```

The list of available names includes display names of managed worktrees.

**Current worktree protection**

Before removal, GWE compares:

- The normalized path of the current worktree (from `RepoContext`), and
- The normalized path of the target worktree.

If they are equal, removal is rejected with a user error:

```text
cannot remove the current worktree '<target>': <path>
```

This behavior, and the guarantee that the current worktree remains intact, is
verified in tests.

**Git invocation**

Worktree removal:

- Arguments: `["worktree", "remove", "--force", <path>]` (always uses `--force`).
- On success: the worktree directory is removed by Git.
- On failure:

  - If stderr is non‑empty, it is surfaced as a Git error message.
  - If stderr is empty, an error of the form
    `"git worktree remove failed for <path> without error output"` is used.

Branch removal (when `--with-branch` and a branch is available):

- Uses `git branch -D <branch>` (forced; can delete unmerged branches).
- On failure:

  - If stderr is non‑empty, it is used directly as the error message.
  - Otherwise, a generic error like `"failed to remove branch '<branch>'"`
    is used.

All such failures are treated as Git errors and mapped to exit code 3.

**User‑visible output**

On successful worktree removal, GWE prints:

```text
Removed worktree '<target>' at <absolute_path>
```

If branch removal is requested and succeeds, it also prints:

```text
Removed branch '<branch>'
```


4.2.4 `gwe cd`
^^^^^^^^^^^^^^

**Purpose**  
Resolve a worktree identifier to an absolute path. In conjunction with the
shell integration, this enables shell‑level directory changes.

**Synopsis**

```text
gwe cd <WORKTREE>
```

**Options (CdCommand)**

- `WORKTREE` (positional, required)  
  Target worktree identifier. If missing or resolves to an empty name, the
  command fails with a user error:

  ```text
  worktree name is required
  ```

**Name sanitization**

The input is first sanitized by:

- Trimming leading/trailing whitespace.
- Removing any trailing `*` (e.g. a copied value from `gwe list` where the
  current worktree is marked with an asterisk).

If the sanitized value is empty, the command fails with the error above.

**Resolution algorithm**

`gwe cd`:

1. Enumerates `WorktreeInfo` entries from `git worktree list --porcelain`.
2. Computes:
   - `base_dir` as `config.resolved_base_dir(main_root)`, and
   - `repo_name` from the main root directory name.

3. For each worktree, in order:

   - If it is the main worktree (`is_main == true`):

     - Matches if the target string is:
       - `"@"`
       - `"root"` (case‑insensitive)
       - Equal to `repo_name` (case‑insensitive)
       - Equal to the main branch name (if any)

     - If matched, returns this worktree path immediately.

   - For non‑main worktrees:

     - Skips any worktree that is not managed under `base_dir`.
     - Matches if the target string equals:
       - The branch name (`info.branch`), or
       - The display name, or
       - The worktree directory name (final path component).

4. If no match is found, a user error is returned with a message of the form:

```text
worktree '<target>' not found
Available worktrees: <names...>
Run 'gwe list' to see available worktrees.
```

The list of available names includes `"@"`, the main branch name (if any),
the repository name, and the display names of managed worktrees.

**Output**

On success, `gwe cd` prints the normalized absolute path of the resolved
worktree to standard output, followed by a newline. No other output is
emitted in the success path.

Integration tests assert that:

- `gwe cd @` resolves to the repository root.
- `gwe cd <display_name>` resolves correctly after `gwe add`.
- Errors for unknown worktrees include both the "Available worktrees" list
  and the `Run 'gwe list'` hint.


4.2.5 `gwe init`
^^^^^^^^^^^^^^^^

**Purpose**  
Install shell integration into a shell profile by appending a function
for `gwe`.

**Synopsis**

```text
gwe init [--shell <SHELL>] [PROFILE_PATH]
```

**Options (InitCommand)**

- `--shell <SHELL>` (ValueEnum, default: `pwsh`)  
  Shell kind. Supported values:

  - `pwsh` (PowerShell; fully supported).
  - `bash` (Bash; supported).
  - `zsh` (Zsh; supported).
  - `cmd` (Windows Command Prompt; not supported).

- `PROFILE_PATH` (optional positional path)  
  Path to the profile file to be modified. If omitted, GWE computes a default
  profile path based on the shell kind:

  - `pwsh`: `<HOME>\Documents\PowerShell\Microsoft.PowerShell_profile.ps1`
  - `bash`: `<HOME>/.bashrc`
  - `zsh`: `<HOME>/.zshrc`

  The `HOME` is determined from `USERPROFILE` or `HOME` environment variable.

**Behavior**

- (Current implementation) Attempts to set the following global git config
  defaults (errors are ignored; existing values are overwritten):
  - `gwe.defaultEditor = cursor`
  - `gwe.defaultCli = claude`
  - Prints a short hint to `stderr`.

- For supported shells (`pwsh`, `bash`, `zsh`):

  - Ensures the profile directory exists, creating it if necessary.
  - Reads the existing profile content (if any).
  - If the content already contains the marker line `# gwe shell integration`,
    it performs no changes (idempotent).
  - Otherwise, opens the profile file in append mode, optionally inserts a
    newline, and appends:
    - The marker line `# gwe shell integration`.
    - The shell script from the corresponding shell module.

- For `cmd`:

  - Returns an error with the message:
    `"shell 'cmd' is not supported yet"`.

  This error is treated as a generic error (exit code 10).


4.2.6 `gwe shell-init`
^^^^^^^^^^^^^^^^^^^^^^

**Purpose**  
Emit the shell integration script to standard output instead of writing it to
the profile file. This allows manual inspection or composition.

**Synopsis**

```text
gwe shell-init <SHELL>
```

**Options (ShellInitCommand)**

- `shell` (ValueEnum; required)  
  Shell kind (`pwsh`, `bash`, `zsh`, `cmd`).

**Behavior**

- For supported shells (`pwsh`, `bash`, `zsh`):

  - Writes the content of the corresponding shell script to standard output
    and flushes the output.

- For `cmd`:

  - Returns an error with the message:
    `"shell 'cmd' is not supported yet"`.

Integration tests check that:

- `gwe shell-init pwsh` prints a script containing `function gwe` and
  `Register-ArgumentCompleter`.
- `gwe shell-init cmd` fails with the appropriate error message.


4.2.7 `gwe config`
^^^^^^^^^^^^^^^^^^

**Purpose**  
Get, set, add, or unset Git configuration values. This is a wrapper around
`git config` commands with a convenient interface.

**Synopsis**

```text
gwe config get <KEY>
gwe config set <KEY> <VALUE...> [-g|--global]
gwe config add <KEY> <VALUE...> [-g|--global]
gwe config unset <KEY> [-g|--global]
```

**Subcommands**

- `get <KEY>`  
  Retrieves all values for the specified key using `git config --get-all`.
  If the key does not exist, outputs nothing (no error).

- `set <KEY> <VALUE> [-g|--global]`  
  Sets the specified key to the given value. If `-g` or `--global` is provided,
  uses the global Git configuration file.

- `add <KEY> <VALUE> [-g|--global]`  
  Adds a value to the specified key (for multi-value keys) using
  `git config --add`. Useful for list-like configuration entries.

- `unset <KEY> [-g|--global]`  
  Removes the specified key using `git config --unset`. Silent if the key
  does not exist.

**Common configuration keys**

- `gwe.defaultbranch`: Default branch name.
- `gwe.defaulteditor`: Default editor to launch with `gwe -e` (defaults to `cursor` if unset).
- `gwe.defaultcli`: Default CLI tool to launch with `gwe -c` (required for `gwe -c`).
- `gwe.multicli`: List of tools to launch with `gwe cli` (comma- or whitespace-separated).
- `gwe.copy.include`: Multi-value key for file patterns to copy (Glob copy hooks).
- `gwe.copy.exclude`: Multi-value key for exclude patterns when copying (glob-style; `.git` is always excluded).
- `gwe.hook.postcreate`: Multi-value key for commands to run (Command hooks).


4.2.8 Tool launch (`gwe cursor` / `gwe wind` / `gwe anti` / `gwe claude` / `gwe codex` / `gwe gemini` / `gwe -e` / `gwe -c`)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Purpose**  
Launch an external tool (editor or AI CLI) in a worktree. If the specified
worktree does not exist, it will be created (and post-create hooks will run).

**Synopsis**

```text
gwe cursor [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe wind [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe anti [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe claude [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe codex [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe gemini [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe -e [WORKTREE] [OPTIONS] [-- <ARGS>...]
gwe -c [WORKTREE] [OPTIONS] [-- <ARGS>...]
```

**Options (ToolCommand)**

- `WORKTREE` (positional, optional)  
  Target worktree identifier. If omitted, uses the current worktree (`@`).

- `-b, --branch <BRANCH>`  
  New branch name (always creates a new worktree when set).

- `--track <REMOTE/BRANCH>`  
  Remote branch to track when creating a worktree (`git worktree add --track`).
  The local branch name is inferred from `REMOTE/BRANCH` unless `--branch` is
  explicitly supplied.

- `-x, --multiplier <COUNT>` (1-5)  
  **Effective only for terminal tools** (`claude`/`codex`/`gemini`).
  Creates multiple worktrees and launches them in split panes.
  - Requires `-b/--branch`.
  - Cannot be used together with `WORKTREE`.

- `-- <ARGS>...`  
  Additional arguments to pass to the tool.

**Resolution/creation (high level)**

- If `-b/--branch` or `--track` is specified, GWE always attempts to create a new worktree.
- Otherwise, it resolves `WORKTREE` (or `@`) using the same rules as `gwe cd`;
  if not found, it treats `WORKTREE` as a branch/commitish and attempts to create a new worktree.

**Tool-specific behavior**

- `cursor` / `wind` / `anti`: launched as an external process with the worktree path as an argument.
- `claude` / `codex` / `gemini`: spawned in a new terminal window.
  - For `gemini`, if extra args are provided and neither `-i/--prompt-interactive` nor `-p/--prompt` is present, `-i` is automatically prepended.
- `-e`: launches the tool configured in `gwe.defaultEditor` (defaults to `cursor` if unset).
- `-c`: launches the tool configured in `gwe.defaultCli` (errors if unset).

**Split panes**

- Windows: uses Windows Terminal (`wt.exe`) when available; otherwise falls back to separate terminal windows.
- macOS: uses iTerm2 split panes when available; otherwise falls back to multiple Terminal.app windows.


4.2.9 `gwe cli`
^^^^^^^^^^^^^^^

**Purpose**  
Launch multiple AI CLI tools configured in `gwe.multicli` in split panes.

**Synopsis**

```text
gwe cli [WORKTREE] [-- <ARGS>...]
```

**Options (ToolCommand)**

Same as above.

**Behavior**

1. Reads the tool list from `gwe.multicli` (case-insensitive; often configured as `gwe.multiCli`).
2. Resolves or creates the worktree path:
   - If `-b/--branch` is specified, creates multiple worktrees (one per tool; max 5 for split panes), using the same naming convention as `-x` (`<branch>-1`, `<branch>-2`, ...).
   - Otherwise, uses a single worktree for all tools.
3. Launches all tools in split panes (Windows Terminal or iTerm2) within their respective target worktrees.


5. Configuration
----------------

GWE is configured via Git configuration variables (recommended).

5.1 Git Configuration
~~~~~~~~~~~~~~~~~~~~~

GWE reads configuration from git config variables in the `gwe.*` namespace.
These can be set using `git config` or the helper command `gwe config`.

Supported keys:

- `gwe.worktrees.dir` (path)
  Base directory for managed worktrees. Overrides the default `../worktree`.

- `gwe.defaultbranch` (string)
  Default branch name.

- `gwe.defaulteditor` (string)
  Default editor tool to launch via `gwe -e`.

- `gwe.defaultcli` (string)
  Default CLI tool to launch via `gwe -c`.

- `gwe.copy.include` (multi-value string)
  Glob patterns for files to copy from the main worktree to new worktrees.
  Each value creates a `glob_copy` hook.

- `gwe.copy.exclude` (multi-value string)
  Exclude patterns applied when copying files (glob-style; `.git` is always excluded).

- `gwe.hook.postcreate` (multi-value string)
  Shell commands to execute after worktree creation. Each value creates a
  `command` hook.

- `gwe.multicli` (multi-value string)
  AI CLI tools to be launched by `gwe cli` in split panes.


6. Hook Execution
-----------------

Hook execution is performed by `HookExecutor`.

On `gwe add` success, GWE:

1. Constructs a `HookExecutor`.
2. Calls `execute_post_create_hooks` with:

   - A mutable writer wrapping standard output.
   - The path of the newly created worktree.

3. `execute_post_create_hooks`:

   - Returns immediately if `hooks.post_create` is empty.
   - Otherwise:
     - Prints an introductory message:

       ```text
       Executing post-create hooks...
       ```

     - For each hook (1‑based index):

       - Prints:

         ```text
         → Running hook <i> of <n>...
         ```

       - Executes the hook (`copy`, `glob_copy`, or `command`).
       - If the hook succeeds, prints:

         ```text
         ✓ Hook <i> completed
         ```

     - After all hooks succeed, prints:

       ```text
       ✓ All hooks executed successfully
       ```

4. Any error during `copy`, `glob_copy`, or `command` execution stops further
   hooks and causes `gwe add` to fail.

Integration tests verify that:

- Hooks are executed after worktree creation.
- A `copy` hook correctly copies a file from the main worktree into the new
  worktree.
- A `glob_copy` hook correctly copies files matching the pattern.
- A `command` hook can create a file (`hook.log`) whose contents include
  the expected output.
- The success messages for hook execution are present in `stdout`.


7. Git and Worktree Integration
-------------------------------

7.1 Repository Context Discovery
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

`RepoContext::discover` determines the Git context as follows:

1. Determine the starting directory:

   - If `--repo <PATH>` was provided, resolve it:
     - Relative paths are resolved against the current directory.
     - If the path refers to a file, its parent directory is used.
     - If the path (or its parent) does not exist, an error is returned.
   - Otherwise, use `std::env::current_dir()`.

2. Run `git rev-parse --show-toplevel` in the starting directory to obtain
   the worktree root. This is canonicalized, and if the command fails, a
   contextual error is returned.

3. Run `git rev-parse --git-common-dir` in the worktree root to obtain the
   common Git directory. This path is resolved against the worktree root if
   it is relative, and canonicalized.

4. If the resolved common directory ends with `.git`, its parent directory is
   used as the main repository root. Otherwise, the canonical common
   directory is used directly as `main_root`.

5. The repository name (`repo_name`) is the final path component of
   `main_root`; if that cannot be determined, `main_root`'s display string
   is used.

`RepoContext` provides:

- `worktree_root()`: path to the current worktree.
- `main_root()`: path to the main repository root.
- `repo_name()`: the derived repository name.
- `is_main_worktree()`: whether current worktree equals main root (after
  canonicalization).


7.2 Git Command Execution
~~~~~~~~~~~~~~~~~~~~~~~~~

`GitRunner` encapsulates calls to `git.exe`:

- All commands are logged at `DEBUG` or higher with:

  - The working directory path.
  - The full command string as formatted by `format_command`.

- `GitRunner::run`:

  - Runs a command and returns a `GitOutput` on success.
  - If the command exits with a non‑zero status, returns `GitError::CommandFailed`
    with the exit status and both `stdout` and `stderr` captured.

- `GitRunner::run_in`:

  - Same as `run` but takes an explicit working directory.

- `GitRunner::run_with_status` / `run_with_status_in`:

  - Return `GitOutput` even when Git fails, allowing callers to inspect and
    interpret the exit code and output themselves.

`GitOutput` exposes:

- `command`: formatted command string.
- `status`: `ExitStatus`.
- `stdout`: captured standard output.
- `stderr`: captured standard error.

`GitError` variants:

- `Spawn`  
  Command could not be started (e.g. `git` not found). Includes the working
  directory, command string, and underlying `io::Error`.

- `CommandFailed`  
  `git` exited with a non‑zero status. Includes the status and captured
  `stdout`/`stderr`.

- `InvalidUtf8`  
  Output from `git` could not be decoded as UTF‑8.

Errors from `GitRunner` are generally wrapped as Git application errors and
mapped to exit code 3 by `main`.


7.3 Worktree Parsing
~~~~~~~~~~~~~~~~~~~~

`git::worktree::list_worktrees`:

- Executes `git worktree list --porcelain` via `GitRunner`.
- Parses lines into `WorktreeInfo` as follows:

  - `worktree <path>`: starts a new entry and sets `path` to the canonicalized
    version of `<path>`.
  - `HEAD <hash>`: sets the full commit hash.
  - `branch refs/heads/<branch>`: sets `branch` to `<branch>`.
  - `detached`: sets `is_detached = true` and prevents `branch` from being
    used.
  - `locked <reason>` or `locked`: sets `locked` accordingly.
  - `prunable <reason>` or `prunable`: sets `prunable` accordingly.
  - Blank lines delimit entries.

- After parsing, the first entry in the list is marked `is_main = true`.

This behavior is covered by unit tests in `git::worktree`.


8. Shell Integration
--------------------

8.1 PowerShell
~~~~~~~~~~~~~~

The PowerShell script emitted by `gwe shell-init pwsh` and appended by
`gwe init` contains:

- A helper function `Get-GweExePath` that:

  - First looks for `gwe.exe` using `Get-Command`.
  - Falls back to `Get-Command gwe -CommandType Application`.
  - Throws an error if no executable is found.

- A `gwe` function that:

  - Forwards arguments to the actual `gwe.exe`.
  - Captures `stdout` and the exit code.
  - If the exit code is zero and the first argument is `cd`:

    - Reads the last line of the output, trims it, and, if non‑empty,
      calls `Set-Location` to that path.

  - Otherwise, writes the output (if any) to the console.
  - Sets `$global:LASTEXITCODE` to the exit code from `gwe.exe`.

- An argument completer registered via `Register-ArgumentCompleter`:

  - When completing the first argument (the subcommand), suggests:
    `add`, `list`, `rm`, `cd`, `init`, `shell-init`, `config`, `cursor`, `wind`, `anti`, `claude`, `codex`, `gemini`, `cli`, `-e`, `-c`.
  - When the subcommand is `cd` / `rm` / tool commands (`cursor`/`wind`/`anti`/`claude`/`codex`/`gemini`/`cli`/`-e`/`-c`), it:
    - Invokes `gwe list --json`.
    - Parses the JSON into objects with a `.name` field.
    - Suggests each `name` as a completion candidate.
    - Special‑cases the `"@"` name by offering it quoted as `'@'` to avoid
      PowerShell parsing issues (but excludes `"@"` for `rm`).

These behaviors are asserted by unit tests in `shell::pwsh` and integration
tests in `tests/shell_spec.rs`.


8.2 Bash
~~~~~~~~

The Bash script emitted by `gwe shell-init bash` and appended by
`gwe init --shell bash` contains:

- A `gwe` function that:

  - If the first argument is `cd`:
    - Calls `command gwe cd` with the remaining arguments.
    - Captures the exit code.
    - If successful, changes directory to the output.
    - If failed, returns the exit code.

  - For all other commands, passes through to the real `gwe` executable.


8.3 Zsh
~~~~~~~

The Zsh script emitted by `gwe shell-init zsh` and appended by
`gwe init --shell zsh` is identical to the Bash script, as the syntax is
compatible.


9. Logging and Error Handling
-----------------------------

9.1 Logging
~~~~~~~~~~~

`logging::init` configures a `tracing_subscriber` based on `GlobalOptions`:

- If `--quiet` is set:

  - Maximum log level is `ERROR`.

- Else if `--verbose` is:

  - `0`: maximum level is `WARN`.
  - `1`: maximum level is `DEBUG`.
  - `>= 2`: maximum level is `TRACE`.

Logs are written to standard error (`stderr`), without timestamps or target
information. Integration tests verify that using `--verbose` results in
debug lines such as `"Executing git command"` being emitted.


9.2 Error Types and Exit Codes
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

GWE uses a structured error type `AppError` with four variants:

- `User(String)`  
  For user mistakes such as missing arguments or invalid combinations.

- `Config(String)`  
  For problems related to configuration.

- `Git(String)`  
  For failures of underlying Git commands.

- `Internal(String)`  
  For unexpected internal errors.

Each variant maps to an exit code:

- `User` → `1`
- `Config` → `2`
- `Git` → `3`
- `Internal` → `10`

Integration tests assert that:

- Git failures during `add` yield exit code 3.

The `main` function handles errors as follows:

1. Calls `gwe::run()`:

   - On `Ok(ExitCode)`: returns that exit code (currently always success).

2. On `Err(error)`:

   - First attempts to downcast directly to `AppError`.
   - If successful:
     - Prints the error message.
     - Exits with the corresponding `AppError::exit_code()`.

   - If not, iterates over the error cause chain and:

     - If an `AppError` cause is found:
       - Uses its message and exit code.

     - Else if a `GitError` cause is found:
       - Treats it as a Git error:
         - Exit code: 3.
         - Message: `GitError`'s `Display` output.

   - If none of the above are found:

     - Uses the top‑level error's `Display` output as the message.
     - Uses exit code 10.

In all failure cases, the selected message is printed to `stderr`.


10. Testing Strategy and Behavioral Guarantees
---------------------------------------------

The tests in `tests/` serve as an executable specification. Key guarantees
include:

- **Repository discovery and `--repo`**  
  `gwe list --json` works both inside and outside a repository when `--repo`
  is provided, returning at least the main worktree entry.

- **Configuration handling**  
  - Git config defaults are used when not set.

- **`add` behavior**  
  - Creates new worktrees under the configured `base_dir` with paths derived
    from branch names, including the repository name as a path component.
  - Requires a branch or commit argument when no `--branch`/`--track` is used.
  - Detects branch conflicts and reports them with clear messages.
  - Enforces `--track` argument requirements.
  - Runs post‑create hooks and observes their effects (copied files and
    command‑generated files).
  - Prints the resolved worktree path to stdout on success.

- **`cd` behavior**  
  - `gwe cd @` resolves to the repository root.
  - `gwe cd <display_name>` resolves to the appropriate worktree path.
  - Unknown worktrees produce "not found" errors including:
    - An "Available worktrees" list.
    - A "Run 'gwe list'" hint.

- **`list` behavior**  
  - `list --json` includes the main worktree with `name = "@"` and
    `branch = "main"`.
  - `list` marks dirty worktrees and shows upstream branches when configured.
  - `list` marks the current worktree with an asterisk in the `PATH` column.
  - `list --json` correctly reflects `is_main` and `is_current` flags.

- **`rm` behavior**  
  - `rm --with-branch` deletes both the worktree directory and its branch
    (branch deletion is forced even if unmerged).
  - `rm` removes worktrees even if they are dirty (force removal).
  - `rm` only affects worktrees under the currently configured `base_dir`;
    changing `base_dir` can make existing worktrees unmanaged and thus
    protected from removal.
  - Attempting to remove the current worktree fails with a clear error and
    leaves the directory intact.

- **`config` behavior**
  - `config set` / `config get` / `config unset` work correctly.
  - `config add` allows multiple values for the same key.

- **`cursor` / `wind` / `anti` behavior**
  - Launches the corresponding tool with the worktree path.
  - Supports additional arguments via `--`.

- **Shell integration**  
  - `shell-init pwsh` emits a script containing both the wrapper function and
    the argument completer.
  - `shell-init bash` and `shell-init zsh` emit wrapper functions.
  - `shell-init cmd` is explicitly not supported and fails with the
    documented error message.

- **Help and version**  
  - `gwe --help` prints usage, descriptions, and lists subcommands including
    `shell-init`, `config`, `cursor`, `wind`, and `anti`.
  - `gwe --version` prints the package version as defined by `CARGO_PKG_VERSION`.

- **Verbosity**  
  - Using `--verbose` results in debug logging that includes Git command
    execution messages.

Any future changes to GWE should preserve these behaviors unless the tests
are deliberately updated to reflect new specifications.
