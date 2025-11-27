gwe (Git Worktree Extension)
==============================

Rust 製の Windows ネイティブな Git worktree ヘルパーツールです。"gwe" は **Git Worktree Extension** の略です。

Git worktree を Windows で快適に扱うための CLI ツールです。Windows 11 / PowerShell 前提で使いやすくすることを目指しています。

> Status: 0.3.x. 日常的な利用に十分な機能が揃っています。

> **注意:**
> v0.2.0 までは `wtp` (Git Worktree Pro) をベースにしていましたが、v0.3.0 からは `wtp` ベースを廃止し、完全なオリジナル実装となりました。これに伴い、コマンド名も `wtw` から `gwe` に変更されました。


特徴
----

- **Windows ファーストな worktree ヘルパー**
  - 内部で `git.exe` を使用します。
  - Windows スタイルのパスやドライブレターをサポートします。
- **自動的な worktree レイアウト**
  - デフォルトで `feature/auth` のようなブランチ名を `../worktree/feature/auth` にマッピングします。
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

- **OS**: Windows 11 (他のモダンな Windows バージョンでも動作する可能性がありますが、公式にはテストされていません)。
- **Git**: Git for Windows (`git.exe` が `PATH` に通っていること)。
- **シェル**:
  - PowerShell 7+ (推奨)。
  - Git Bash (Bash) / Zsh.
  - Cmd は未サポート。
- **Rust ツールチェーン** (ソースからビルドする場合のみ):
  - Rust stable
  - `cargo`


インストール
------------

### プレビルドバイナリのダウンロード (推奨)

リリースが公開されると、一般的な配布物は以下のようになります:

- `gwe-<version>-x86_64-pc-windows-msvc.zip`

各アーカイブには以下が含まれます:

- `gwe.exe`
- `README.md` (このファイル)
- `LICENSE`

インストール手順:

```powershell
# 1. リポジトリの "Releases" ページから ZIP をダウンロード
# 2. 任意の場所に解凍 (例: C:\tools\gwe)
Expand-Archive -Path .\gwe-0.3.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\gwe

# 3. そのディレクトリを PATH に追加 (一度だけ)
[System.Environment]::SetEnvironmentVariable(
  "Path",
  $env:Path + ";C:\tools\gwe",
  "User"
)

# 4. 新しい PowerShell を開いて確認
gwe --help
```

> NOTE: アーカイブ名や解凍先パスは一例です。実際のリリース/タグ名に合わせて調整してください。


### ソースからビルドしてインストール

このリポジトリをクローンし、`gwe` クレート内でビルドします:

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


クイックスタート
----------------

### 1. Git リポジトリの準備

Git リポジトリ内で (または `--repo` で指定して)、`gwe` はリポジトリルートを自動検出します:

```powershell
# 既存の Git リポジトリ内で
cd C:\src\my-project
gwe list --json

# またはリポジトリ外から
gwe --repo C:\src\my-project list --json
```


### 2. シェル統合の有効化 (任意ですが推奨)

`gwe.exe` が `PATH` にあれば、1つのコマンドで `gwe` 関数と補完をシェルプロファイルに追加できます:

```powershell
# 現在のシェルのデフォルトプロファイルを使用 (自動検出)
# サポート: pwsh, bash, zsh
gwe init

# シェルを明示的に指定する場合
gwe init --shell pwsh
gwe init --shell bash
gwe init --shell zsh
```

これが行うこと:

- 必要に応じてプロファイルディレクトリ/ファイルを作成します。
- `# gwe shell integration` で始まるセクションを追記します。
- 以下の機能を持つ `gwe` 関数を定義します:
  - 実際の `gwe.exe` を呼び出す。
  - 最初の引数が `cd` でコマンドが成功した場合、カレントディレクトリを表示されたパスに変更する。
- シェル補完を登録します (PowerShell の ArgumentCompleter, Bash/Zsh の complete 関数)。

`gwe init` 実行後、**新しい** シェルセッションを開いて以下を試してください:

```powershell
gwe cd @
gwe cd <TAB>  # worktree 名が補完されます
```

手動でプロファイルを管理したい場合は、スクリプトを出力して確認できます:

```powershell
gwe shell-init pwsh > gwe.ps1
# または
gwe shell-init bash > gwe.sh
gwe shell-init zsh > gwe.zsh
```


基本的な使い方
--------------

### worktree の作成 (`add`)

```powershell
# 既存のローカルまたはリモートブランチから worktree を作成
gwe add feature/auth

# 新しいブランチを作成して worktree を追加
gwe add -b feature/new-feature

# 特定のリモートブランチを追跡する新しいブランチを作成
gwe add --track origin/feature/remote-only

# 特定のコミットをベースに使用 (ブランチ名は -b で指定)
gwe add -b hotfix/urgent abc1234
```

- デフォルトでは、worktree はリポジトリルートからの相対パス `../worktree` 配下に配置されます。
- `/` を含むブランチ名はネストされたディレクトリになります (例: `feature/auth` → `../worktree/feature/auth`)。


### worktree の一覧表示 (`list`)

```powershell
# 見やすいテーブル形式
gwe list

# 出力例:
# PATH                      BRANCH           HEAD     STATUS  UPSTREAM       ABS_PATH
# ----                      ------           ----     ------  --------       --------
# @*                        main             c72c7800 clean   origin/main    C:\src\my-project
# feature/auth              feature/auth     def45678 dirty   origin/feature/auth C:\src\my-project\..\worktree\feature\auth

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


### worktree の削除 (`remove`)

```powershell
# worktree を削除 (表示名/ブランチ名/ディレクトリ名で指定)
gwe remove feature/auth

# worktree が dirty でも強制削除
gwe remove --force feature/auth

# worktree とそのブランチを削除 (マージ済みの場合のみ)
gwe remove --with-branch feature/auth

# worktree を削除し、ブランチも強制削除
gwe remove --with-branch --force-branch feature/auth
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

- `0`: 成功
- `1`: ユーザーエラー (無効な引数、不明な worktree など)
- `2`: 設定エラー
- `3`: Git コマンドの失敗
- `10`: 予期しない内部エラー


ライセンス
----------

MIT License. 詳細は `LICENSE` ファイルを参照してください。
