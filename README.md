gwe (Git Worktree Extension)
==============================

Rust 製の Git worktree ヘルパーツールです。"gwe" は **Git Worktree Extension** の略です。

Git worktree を快適に扱うための CLI ツールです。Windows と macOS に対応しています。  
For the English version of the README, see `README.en.md`.

> Status: 1.2.0. 日常的な利用に十分な機能が揃っています。

> **注意:**
> v0.2.0 までは `wtp` (Git Worktree Pro) をベースにしていましたが、v0.3.0 からは `wtp` ベースを廃止し、完全なオリジナル実装となりました。これに伴い、コマンド名も `wtw` から `gwe` に変更されました。


特徴
----

- **Windows / macOS 対応の worktree ヘルパー**
  - 内部で `git` コマンドを使用します。
  - Windows スタイルのパスやドライブレター、Unix スタイルのパスをサポートします。
- **自動的な worktree レイアウト**
  - デフォルトで `feature/auth` のようなブランチ名を `../worktree/<repo_name>/feature/auth` にマッピングします。
  - Windows で禁止されている文字をブランチ名に含む場合、安全な文字に置換します（例: `feat:bad*name` → `feat_bad_name`）。
- **作成後のフック機能**
  - `copy` フック: `.env` などの gitignore されたファイルをメイン worktree からコピーします。
  - `command` フック: 依存関係のインストールやマイグレーションなどのセットアップコマンドを実行します。
- **リッチな `list` 出力と JSON サポート**
  - `PATH`, `BRANCH`, `HEAD`, `STATUS`, `UPSTREAM`, `ABS_PATH` を含む見やすいテーブル表示。
  - ツール連携や PowerShell 補完のための `gwe list --json`。
- **シェル統合 (PowerShell, Bash, Zsh)**
  - `gwe init` でシェルプロファイルに関数を追加し、`gwe cd` で実際にカレントディレクトリを変更できるようにします。
  - サブコマンドや `gwe cd` の worktree 名に対するタブ補完。


動作要件
--------

- **OS**:
  - Windows 11 (他のモダンな Windows バージョンでも動作する可能性があります)
  - macOS (Terminal.app / iTerm2 対応)
- **Git**: `git` コマンドが `PATH` に通っていること。
- **シェル**:
  - PowerShell 7+ (Windows、推奨)
  - Bash / Zsh (macOS / Linux)
  - Cmd は未サポート
- **Rust ツールチェーン** (ソースからビルドする場合のみ):
  - Rust stable
  - `cargo`


インストール
------------

### プレビルドバイナリのダウンロード (推奨)

リリースが公開されると、一般的な配布物は以下のようになります:

- `gwe-<version>-x86_64-pc-windows-msvc.zip` (Windows)
- `gwe-<version>-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `gwe-<version>-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)

各アーカイブには以下が含まれます:

- `gwe.exe` (Windows) / `gwe` (macOS)
- `README.md` (このファイル)
- `LICENSE`

#### Windows の場合

```powershell
# 1. リポジトリの "Releases" ページから ZIP をダウンロード
# 2. 任意の場所に解凍 (例: C:\tools\gwe)
Expand-Archive -Path .\gwe-1.2.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\gwe

# 3. そのディレクトリを PATH に追加 (一度だけ)
[System.Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\tools\gwe", "User")

# 4. 新しい PowerShell を開いて確認
gwe --help
```

#### macOS の場合

```bash
# 1. リポジトリの "Releases" ページから tar.gz をダウンロード
# 2. 任意の場所に解凍 (例: ~/tools/gwe)
mkdir -p ~/tools/gwe
tar -xzf gwe-1.2.0-aarch64-apple-darwin.tar.gz -C ~/tools/gwe

# 3. そのディレクトリを PATH に追加 (~/.zshrc または ~/.bashrc に追記)
echo 'export PATH="$HOME/tools/gwe:$PATH"' >> ~/.zshrc

# 4. 新しいターミナルを開いて確認
gwe --help
```

> NOTE: アーカイブ名や解凍先パスは一例です。実際のリリース/タグ名に合わせて調整してください。


### ソースからビルドしてインストール

このリポジトリをクローンし、`gwe` クレート内でビルドします:

#### Windows の場合

```powershell
git clone <this repository>
cd gwe

# リリースビルド
cargo build --release

# オプション 1: ビルドされたバイナリを直接使用
.\target\release\gwe.exe --help

# オプション 2: ~/.cargo/bin にインストール
cargo install --path .
gwe --help
```

#### macOS の場合

```bash
git clone <this repository>
cd gwe

# リリースビルド
cargo build --release

# オプション 1: ビルドされたバイナリを直接使用
./target/release/gwe --help

# オプション 2: ~/.cargo/bin にインストール
cargo install --path .
gwe --help
```


クイックスタート
----------------

### 1. Git リポジトリの準備

Git リポジトリ内で (または `--repo` で指定して)、`gwe` はリポジトリルートを自動検出します:

まずは `gwe list --json` を実行して、リポジトリ検出（および `--repo` 指定）が正しく動作することを確認します。`list` は worktree を作成・削除せず一覧を取得するだけなので、最初の疎通確認として安全です（`--json` は出力が安定していて機械処理にも向いています）。

#### Windows の場合

```powershell
# 既存の Git リポジトリ内で
cd C:\src\my-project
gwe list --json

# またはリポジトリ外から
gwe --repo C:\src\my-project list --json
```

#### macOS の場合

```bash
# 既存の Git リポジトリ内で
cd ~/src/my-project
gwe list --json

# またはリポジトリ外から
gwe --repo ~/src/my-project list --json
```


### 2. シェル統合の有効化 (任意ですが推奨)

`gwe` が `PATH` にあれば、1つのコマンドで `gwe` 関数と補完をシェルプロファイルに追加できます:

#### Windows (PowerShell) の場合

```powershell
# PowerShell プロファイルに追加
gwe init --shell pwsh

# 新しい PowerShell を開いて確認
gwe cd @
gwe cd <TAB>  # worktree 名が補完されます
```

#### macOS (Zsh / Bash) の場合

```bash
# Zsh の場合 (~/.zshrc に追加)
gwe init --shell zsh

# Bash の場合 (~/.bashrc に追加)
gwe init --shell bash

# 新しいターミナルを開いて確認
gwe cd @
gwe cd <TAB>  # worktree 名が補完されます
```

これが行うこと:

- 必要に応じてプロファイルディレクトリ/ファイルを作成します。
- `# gwe shell integration` で始まるセクションを追記します。
- 以下の機能を持つ `gwe` 関数を定義します:
  - 実際の `gwe` バイナリを呼び出す。
  - 最初の引数が `cd` でコマンドが成功した場合、カレントディレクトリを表示されたパスに変更する。
- シェル補完を登録します (PowerShell の ArgumentCompleter, Bash/Zsh の complete 関数)。
- （現在の実装）グローバル git config に `gwe.defaultEditor=cursor` と `gwe.defaultCli=claude` を自動設定します（変更したい場合は `gwe config set -g ...` を使用してください）。

手動でプロファイルを管理したい場合は、スクリプトを出力して確認できます:

```bash
gwe shell-init pwsh > gwe.ps1   # PowerShell
gwe shell-init bash > gwe.sh    # Bash
gwe shell-init zsh > gwe.zsh    # Zsh
```


基本的な使い方
--------------

### ツール起動・Worktree 作成

エディタや AI ツールを指定して worktree を開きます。
指定された worktree が存在しない場合は、新規作成されます。

```powershell
# 既存のローカルまたはリモートブランチから worktree を作成・開く
gwe cursor feature/auth

# 新しいブランチを作成して worktree を追加・開く
gwe cursor -b feature/new-feature

# 特定のリモートブランチを追跡する新しいブランチを作成
gwe claude --track origin/feature/remote-only -b feature/local

# 特定のコミットをベースに使用
gwe wind -b hotfix/urgent abc1234

# 複数の worktree を作成し、分割ペインで起動
gwe claude -x 3 -b feature/parallel
# → feature/parallel-1, feature/parallel-2, feature/parallel-3 を作成
```

**利用可能なツールコマンド:**

- **エディタ**: `gwe cursor`, `gwe wind` (Windsurf), `gwe anti` (Antigravity)
- **AI CLI**: `gwe claude`, `gwe codex`, `gwe gemini` (新しいターミナルで起動)
- **汎用**:
  - `gwe -e` (`gwe config set gwe.defaultEditor ...`で設定されたエディタ)
  - `gwe -c` (`gwe config set gwe.defaultCli ...`で設定された CLI)
  - `gwe cli` (`gwe config set gwe.multiCli claude,codex` のようにカンマ区切りで複数の CLI を分割ペインで起動。`-b`指定時はツールごとに個別の worktree を作成)

デフォルトでは、worktree はリポジトリルートからの相対パス `../worktree` 配下に作成され、さらにリポジトリ名が 1 階層目になります（例: `../worktree/my-project/feature/auth`）。

### worktree の一覧表示 (`list`)

```powershell
# 見やすいテーブル形式
gwe list

# 出力例:
# PATH                      BRANCH           HEAD     STATUS  UPSTREAM       ABS_PATH
# ----                      ------           ----     ------  --------       --------
# @*                        main             c72c7800 clean   origin/main    C:\src\my-project
# my-project\feature\auth   feature/auth     def45678 dirty   origin/feature/auth C:\src\my-project\..\worktree\my-project\feature\auth

# ツールや補完用の JSON 出力
gwe list --json
```

JSON 出力は概ね以下のようになります:

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


### worktree の削除 (`rm`)

```powershell
# worktree を削除 (表示名/ブランチ名/ディレクトリ名で指定)
# dirty（未コミットの変更あり）でも強制的に削除されます。
gwe rm feature/auth

# worktree とそのブランチを一緒に削除 (未マージでも強制的に削除されます)
gwe rm -b feature/auth
```

`base_dir` 管理下の worktree のみが削除対象です。それ以外は変更されません。
**現在の** worktree は削除できません (エラーが返されます)。


### worktree 間の移動 (`cd`)

シェル統合が有効 (`gwe init`) であれば、worktree 間を移動できます:

```powershell
# 名前またはブランチ名で worktree に移動
gwe cd feature/auth

# メイン worktree に戻る
gwe cd @
gwe cd my-project   # リポジトリ名でも可
```

`gwe` が指定された worktree を見つけられない場合、利用可能な名前のリストと共にヘルプを表示し、`gwe list` の実行を提案します。


### 設定管理 (`config`)

`gwe` (および `git`) の設定値を直接管理します。

```powershell
# 設定値を取得
gwe config get gwe.worktrees.dir

# 設定値を設定
gwe config set gwe.worktrees.dir "../worktree"

# グローバル設定に設定 (-g)
gwe config set -g gwe.defaultEditor "cursor"

# 設定値を削除
gwe config unset gwe.worktrees.dir
```

設定
----

GWE は Git 設定変数 (`gwe.*`) を使用して設定します。標準の `git config` または `gwe config` ヘルパーで管理できます。

### ベースディレクトリ

```powershell
# worktree のベースディレクトリを設定 (リポジトリルートからの相対パス、または絶対パス)
gwe config set gwe.worktrees.dir "../worktree"
```

- 相対パスは Git リポジトリルートから解決されます。
- 絶対パスもサポートされています。


### フック

worktree 作成後に実行するフックを git config で定義できます。

```powershell
# メイン worktree から新しい worktree へファイルパターン (glob) をコピー
gwe config add gwe.copy.include "*.env"

# 作成後にコマンドを実行
gwe config add gwe.hook.postcreate "npm ci"
```


終了コード
----------

`gwe` はエラータイプを区別するために構造化された終了コードを使用します:

| コード | 意味 |
|--------|------|
| `0` | 成功 |
| `1` | ユーザーエラー (無効な引数、不明な worktree など) |
| `2` | 設定エラー |
| `3` | Git コマンドの失敗 |
| `10` | 予期しない内部エラー |


よく使うパターン
----------------

### 新機能の開発を始める

```bash
# 新しいブランチで worktree を作成し、Cursor で開く
gwe cursor -b feature/awesome-feature

# または Claude Code で開く
gwe claude -b feature/awesome-feature
```

### リモートブランチをローカルで作業する

```bash
# リモートブランチを追跡するローカルブランチを作成
gwe cursor --track origin/feature/someone-else -b feature/someone-else
```

### 複数の worktree で並行作業

```bash
# 3つの worktree を作成し、分割ペインで Claude を起動
gwe claude -x 3 -b feature/parallel-work
# → feature/parallel-work-1, feature/parallel-work-2, feature/parallel-work-3 が作成される
```

### worktree 間の移動

```bash
# 現在の worktree を確認
gwe list

# 別の worktree に移動
gwe cd feature/awesome-feature

# メイン worktree に戻る
gwe cd @
```

### 作業完了後のクリーンアップ

```bash
# メイン worktree に移動してから削除
gwe cd @

# worktree のみ削除（ブランチは残す。dirty でも削除されます）
gwe rm feature/awesome-feature

# worktree とブランチを一緒に削除 (未マージでも削除されます)
gwe rm -b feature/awesome-feature
```

### プロジェクト初期設定（推奨）

```bash
# .env ファイルを新規 worktree にコピーする設定
gwe config add gwe.copy.include "*.env"
gwe config add gwe.copy.include ".env.*"

# worktree 作成後に依存関係をインストール
gwe config add gwe.hook.postcreate "npm ci"

# デフォルトエディタを設定（gwe -e で使用）
gwe config set gwe.defaultEditor "cursor"

# デフォルト CLI を設定（gwe -c で使用）
gwe config set gwe.defaultCli "claude"
```

### 緊急のホットフィックス

```bash
# 特定のコミットから hotfix ブランチを作成
gwe cursor abc1234 -b hotfix/critical-bug

# 作業が終わったら削除 (ブランチも削除)
gwe cd @
gwe rm -b hotfix/critical-bug
```


コマンドリファレンス
--------------------

### グローバルオプション

| オプション | 説明 |
|-----------|------|
| `-v, --verbose` | 詳細ログ出力（stderr） |
| `--quiet` | 標準出力を最小限に（エラーのみ） |
| `--repo <PATH>` | 任意のディレクトリを Git リポジトリ root として扱う |

### ツールコマンドのオプション

| オプション | 説明 |
|-----------|------|
| `-b, --branch <BRANCH>` | 新規ブランチ名（指定時は常に新規作成） |
| `--track <REMOTE/BRANCH>` | 追跡する remote/branch |
| `-x, --multiplier <COUNT>` | 並列 worktree 作成（1-5、分割ペインで起動） |
| `-- <ARGS>...` | ツールに渡す引数 |

### 設定キー一覧

| キー | 説明 | 例 |
|------|------|-----|
| `gwe.worktrees.dir` | worktree のベースディレクトリ | `../worktree` |
| `gwe.defaultBranch` | デフォルトブランチ | `main` |
| `gwe.defaultEditor` | デフォルトエディタ (`-e`) | `cursor` |
| `gwe.defaultCli` | デフォルト CLI ツール (`-c`) | `claude` |
| `gwe.multiCli` | `gwe cli` で起動するツール一覧 | `claude, codex, gemini` |
| `gwe.copy.include` | コピーするファイルパターン | `*.env` |
| `gwe.copy.exclude` | 除外するファイルパターン | `node_modules/**` |
| `gwe.hook.postcreate` | 作成後に実行するコマンド | `npm ci` |


ライセンス
----------

MIT License. 詳細は `LICENSE` ファイルを参照してください。
