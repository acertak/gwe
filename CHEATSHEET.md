# GWE チートシート

gwe (Git Worktree Extension) v0.3.0 のクイックリファレンス

---

## グローバルオプション

| オプション | 説明 |
|-----------|------|
| `-v, --verbose` | 詳細ログ出力（stderr） |
| `--quiet` | 標準出力を最小限に（エラーのみ） |
| `--repo <PATH>` | 任意のディレクトリを Git リポジトリ root として扱う |

---

## コマンド一覧

### ツール起動・Worktree 作成

ツールを指定して worktree を開き、コマンドを実行します。
指定された worktree が存在しない場合は新規作成されます。

```bash
# エディタ系
gwe cursor <WORKTREE>      # Cursor を起動
gwe wind <WORKTREE>        # Windsurf を起動
gwe anti <WORKTREE>        # Antigravity を起動

# AI ツール系 (新しいターミナルで起動)
gwe claude <WORKTREE>      # Claude を起動
gwe codex <WORKTREE>       # Codex を起動
gwe gemini <WORKTREE>      # Gemini を起動

# 汎用起動
gwe -e <WORKTREE>          # デフォルトエディタを起動 (gwe.defaultEditor)
gwe -c <WORKTREE>          # デフォルトCLIを起動 (gwe.defaultCli)
```

**Worktree 作成オプション**:

各ツールコマンドで共通のオプションを使用できます。

```bash
# 既存ブランチまたはコミットから作成・開く
gwe cursor feature/auth

# 新規ブランチを作成して開く
gwe cursor -b feature/new-idea

# リモートブランチを追跡して開く
gwe cursor --track origin/feature/remote -b feature/local

# コミットをベースに新規ブランチを作成して開く
gwe cursor abc1234 -b hotfix/fix-bug
```

| オプション | 説明 |
|-----------|------|
| `-b, --branch <BRANCH>` | 新規ブランチ名 (指定時は常に新規作成) |
| `--track <REMOTE/BRANCH>` | 追跡する remote/branch |
| `-- <ARGS>...` | ツールに渡す引数 |

---

### `gwe list` - Worktree 一覧

```bash
gwe list         # テーブル形式で表示
gwe list --json  # JSON 形式で出力
```

| オプション | 説明 |
|-----------|------|
| `--json` | JSON 形式で出力 |

**出力カラム**: `PATH`, `BRANCH`, `HEAD`, `STATUS`, `UPSTREAM`, `ABS_PATH`

---

### `gwe rm` - Worktree の削除

```bash
gwe rm <WORKTREE>                 # worktree を削除
gwe rm -f <WORKTREE>              # 強制削除（dirty でも）
gwe rm -b <WORKTREE>              # ブランチも一緒に削除
gwe rm -b --force-branch <WT>    # ブランチを強制削除
```

| オプション | 説明 |
|-----------|------|
| `-f, --force` | 強制削除 |
| `-b, --with-branch` | 対応ブランチも削除 |
| `--force-branch` (alias: `--fb`) | ブランチを強制削除 |

---

### `gwe cd` - Worktree 間移動

```bash
gwe cd <WORKTREE>  # 指定 worktree に移動
gwe cd @           # メイン worktree に移動
gwe cd my-project  # リポジトリ名で移動
```

> **Note**: `gwe init` でシェル統合が必要

---

### `gwe init` - シェル統合

```bash
gwe init              # 自動検出でプロファイルに追加
gwe init --shell pwsh # PowerShell
gwe init --shell bash # Bash
gwe init --shell zsh  # Zsh
```

| オプション | 説明 |
|-----------|------|
| `--shell <SHELL>` | シェル種別（pwsh/bash/zsh） |
| `<PROFILE_PATH>` | プロファイルファイルパス（省略可） |

---

### `gwe shell-init` - シェルスクリプト出力

```bash
gwe shell-init pwsh > gwe.ps1
gwe shell-init bash > gwe.sh
gwe shell-init zsh  > gwe.zsh
```

---

### `gwe config` - 設定管理

```bash
gwe config get <KEY>           # 値を取得
gwe config set <KEY> <VALUE>   # 値を設定
gwe config add <KEY> <VALUE>   # 値を追加（マルチバリュー）
gwe config unset <KEY>         # 値を削除
gwe config set --global <KEY> <VALUE>  # グローバル設定
```

---

## 設定キー

| キー | 説明 | 例 |
|------|------|-----|
| `gwe.worktrees.dir` | worktree のベースディレクトリ | `../worktree` |
| `gwe.defaultBranch` | デフォルトブランチ | `main` |
| `gwe.defaultEditor` | デフォルトエディタ (`-e`) | `cursor` |
| `gwe.defaultCli`    | デフォルトCLIツール (`-c`) | `claude` |
| `gwe.copy.include` | コピーするファイルパターン | `*.env` |
| `gwe.hook.postcreate` | 作成後に実行するコマンド | `npm ci` |

```bash
# 設定例
gwe config set gwe.worktrees.dir "../worktree"
gwe config set gwe.defaultEditor "windsurf"
gwe config set gwe.defaultCli "claude"
gwe config add gwe.copy.include "*.env"
gwe config add gwe.copy.include ".env.*"
gwe config add gwe.hook.postcreate "npm ci"
```

---

## 終了コード

| コード | 意味 |
|--------|------|
| `0` | 成功 |
| `1` | ユーザーエラー（無効な引数、不明な worktree） |
| `2` | 設定エラー |
| `3` | Git コマンドの失敗 |
| `10` | 予期しない内部エラー |

---

## よく使うパターン

```bash
# 新機能の作業開始
gwe add -b feature/new-feature
gwe cd feature/new-feature

# 作業完了後に削除
gwe cd @
gwe rm -b feature/new-feature

# 全 worktree を確認
gwe list

# エディタで開く
gwe cursor feature/auth
```
