gwe (Git Worktree Extension)
==============================

**gwe** は、Rust 製の Windows 向け Git worktree 支援ツールです。  
オリジナルの **wtp (Worktree Plus)** との高い互換性を保ちつつ、Windows 11 と PowerShell で快適に使えることを目標としています。  
名前の「gwe」は **Git Worktree Extension** の略です。

本プロジェクトは **wtp バージョン 2.3.4** をベースにしたフォークです。  
今後は wtp 本体との完全な互換性維持を目標とはせず、このリポジトリ独自の方向性で機能追加や仕様変更を行っていく予定です。  
そのため、**このリポジトリに含まれるすべてのファイルは、元の wtp から何らかの変更が加えられているものとみなしてください。**

> ステータス: 0.1 系の早期版です。日常的な利用には十分な機能がありますが、wtp との互換性はまだ移植途中の部分があります。


主な特徴
--------

- **Windows 向けに最適化された worktree ヘルパー**
  - 内部的に `git.exe` を呼び出して動作します。
  - Windows のパス表現やドライブレターを考慮した実装になっています。
- **wtp とほぼドロップイン互換**
  - `.wtp.yml` のフォーマット（`version`, `defaults.base_dir`, `hooks.post_create` など）をそのまま読み込みます。
  - `add`, `list`, `remove`, `cd` の挙動は wtp に極力合わせています。
- **自動的な worktree パスレイアウト**
  - 例えば `feature/auth` というブランチ名は、既定では `../worktree/feature/auth` にマップされます。
  - Windows で使えない文字を含むブランチ名はサニタイズされます（例: `feat:bad*name` → `feat_bad_name`）。
- **post_create hooks による自動セットアップ**
  - `copy` フックで、メイン worktree から `.env` のような gitignore されたファイルをコピーできます。
  - `command` フックで、依存関係のインストールや DB マイグレーションなどのコマンドを自動実行できます。
- **`list` のリッチな出力と JSON 対応**
  - `PATH`, `BRANCH`, `HEAD`, `STATUS`, `UPSTREAM`, `ABS_PATH` を含む表形式で一覧表示します。
  - `gwe list --json` で JSON 形式の一覧を出力でき、スクリプトや PowerShell 補完から利用できます。
- **シェル統合 (PowerShell, Bash, Zsh)**
  - `gwe init` でプロファイルに関数を追記し、`gwe cd` で実際にカレントディレクトリが移動するようになります。
  - `gwe` のサブコマンドや `gwe cd` の worktree 名に対するタブ補完を提供します。
- **開発支援ツール**
  - `gwe editor`: 指定した worktree をエディタで開きます。
  - `gwe ai`: 指定した worktree で AI ツールを起動します。
  - `gwe config`: `git config` をラップし、ツール設定を簡単に管理できます。


動作環境
--------

- **OS**: Windows 11（その他の Windows でも動作する可能性はありますが、公式には 11 を前提としています）
- **Git**: Git for Windows（`git.exe` が `PATH` に通っていること）
- **シェル**:
  - PowerShell 7+（推奨）
  - Git Bash (Bash) / Zsh
  - Cmd は未対応です。
- **Rust ツールチェーン**（ソースからビルドする場合のみ）
  - Rust stable
  - `cargo`


インストール
------------

### 1. 配布バイナリからインストール（推奨）

GitHub Releases などで配布する想定のアーカイブは次のような名前です:

- `gwe-<version>-x86_64-pc-windows-msvc.zip`

アーカイブには最低限、次のファイルを含めます:

- `gwe.exe`
- `README.md`（英語または日本語のどちらか / 本リポジトリでは英語版を想定）
- `LICENSE`

インストール例（PowerShell）:

```powershell
# 1. Releases ページから ZIP をダウンロード
# 2. 任意のディレクトリに展開（例）
Expand-Archive -Path .\gwe-0.2.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\gwe

# 3. 展開したディレクトリを PATH に追加（ユーザー環境変数）
[System.Environment]::SetEnvironmentVariable(
  "Path",
  $env:Path + ";C:\tools\gwe",
  "User"
)

# 4. 新しい PowerShell を開いて動作確認
gwe --help
```

> アーカイブ名や展開先はプロジェクト運用に合わせて適宜変更してください。


### 2. ソースコードからビルドしてインストール

このリポジトリをクローンし、`gwe` crate ディレクトリでビルドします:

```powershell
git clone <このリポジトリ>
cd gwe

# リリースビルド
cargo build --release

# そのままバイナリを実行
.\target\release\gwe.exe --help

# あるいは cargo install でインストール
cargo install --path .
gwe --help
```


クイックスタート
----------------

### 1. Git リポジトリを用意する

Git リポジトリ直下、または `--repo` でリポジトリを指し示した状態で `gwe` を実行します。  
`gwe` は `git rev-parse --show-toplevel` 相当の処理でリポジトリ root を自動検出します。

```powershell
# 既存の Git リポジトリ内で
cd C:\src\my-project
gwe list --json

# リポジトリの外から --repo で指定
gwe --repo C:\src\my-project list --json
```


### 2. シェル統合を有効化する（推奨）

`gwe.exe` が `PATH` に通っていれば、1 コマンドでシェル統合を有効化できます:

```powershell
# 現在のシェルの既定プロファイルを使用 (自動検出)
# 対応: pwsh, bash, zsh
gwe init

# シェル種別やプロファイルパスを明示的に指定する場合
gwe init --shell pwsh
gwe init --shell bash
gwe init --shell zsh
```

この処理により、次のような設定が行われます:

- プロファイルディレクトリ/ファイルが存在しない場合は作成。
- `# gwe shell integration` から始まるセクションを追記。
- 実際の `gwe.exe` を呼び出す `gwe` 関数が定義され、  
  最初の引数が `cd` でコマンドが成功した場合、出力されたパスに移動（`Set-Location` / `cd`）します。
- シェル補完（PowerShellの `Register-ArgumentCompleter` や Bash/Zsh の complete 関数）が登録されます。

設定後、新しいシェルセッションを開き、次のように試せます:

```powershell
gwe cd "@"
gwe cd <TAB>  # worktree 名が補完される
```

プロファイルを自分で管理したい場合は、スクリプトだけ出力して確認することもできます:

```powershell
gwe shell-init pwsh > gwe.ps1
# または
gwe shell-init bash > gwe.sh
gwe shell-init zsh > gwe.zsh
```


基本的な使い方
--------------

### worktree を追加する (`add`)

```powershell
# 既存のローカル / リモートブランチから worktree を作成
gwe add feature/auth

# 新規ブランチと worktree を同時に作成
gwe add -b feature/new-feature

# 特定のリモートブランチを追跡する新規ブランチ + worktree
gwe add --track origin/feature/remote-only

# 特定コミットから新規ブランチを切って worktree を作成
gwe add -b hotfix/urgent abc1234
```

- 既定では、worktree はリポジトリ root から見た `../worktree` 配下に作成されます。
- ブランチ名に `/` が含まれる場合、その区切りごとにディレクトリが切られます  
  （例: `feature/auth` → `../worktree/feature/auth`）。


### worktree を一覧表示する (`list`)

```powershell
# 表形式の一覧
gwe list

# 出力例（簡略化）
# PATH                      BRANCH           HEAD     STATUS  UPSTREAM            ABS_PATH
# ----                      ------           ----     ------  --------            --------
# @*                        main             c72c7800 clean   origin/main         C:\src\my-project
# feature/auth              feature/auth     def45678 dirty   origin/feature/auth C:\src\my-project\..\worktree\feature\auth

# ツールや補完から使いやすい JSON 形式
gwe list --json
```

`gwe list --json` の出力イメージ:

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


### worktree を削除する (`remove`)

```powershell
# 表示名 / ブランチ名 / ディレクトリ名で worktree を削除
gwe remove feature/auth

# dirty な worktree も強制削除
gwe remove --force feature/auth

# 対応するブランチも削除（マージ済みが前提）
gwe remove --with-branch feature/auth

# worktree 削除 + ブランチを強制削除
gwe remove --with-branch --force-branch feature/auth
```

- `.wtp.yml` の `base_dir` 管理下にある worktree のみ削除対象です。
- **現在の worktree** は削除できず、エラーが返されます。


### worktree 間を移動する (`cd`)

PowerShell 連携（`gwe init`）を有効にしている場合、次のように移動できます:

```powershell
# 名前やブランチ名で worktree に移動
gwe cd feature/auth

# メイン worktree に戻る
gwe cd @
gwe cd my-project   # リポジトリ名でも指定可能
```

存在しない worktree 名を指定した場合は、候補一覧とともに  
`Run 'gwe list' to see available worktrees.` というヒント付きのエラーメッセージが表示されます。


### エディタで開く (`editor`)

指定した worktree（または現在のディレクトリ）をエディタで開きます。

```powershell
# 現在の worktree を開く
gwe editor

# 指定した worktree を開く
gwe editor feature/auth
```

- 事前に `gwe.editor.default` の設定が必要です（後述の Config セクション参照）。


### AIツールを起動 (`ai`)

指定した worktree（または現在のディレクトリ）で AI ツール（Cursor等）を起動します。

```powershell
# 現在の worktree で起動
gwe ai

# 指定した worktree で起動
gwe ai feature/auth

# 引数を渡す
gwe ai -- -n
```

- 事前に `gwe.ai.default` の設定が必要です。


### 設定管理 (`config`)

`gwe`（および `git`）の設定値を直接管理します。

```powershell
# デフォルトエディタを設定 (global)
gwe config set --global gwe.editor.default "code"

# デフォルトAIツールを設定 (global)
gwe config set --global gwe.ai.default "cursor"

# 設定値の取得
gwe config get gwe.editor.default

# 設定値の削除
gwe config unset --global gwe.editor.default
```

設定ファイル: `.wtp.yml`
-------------------------

`gwe` はリポジトリ root の `.wtp.yml` を読み込み、**wtp と互換性のある形式**で解釈します。

### base_dir の設定

```yaml
version: "1.0"
defaults:
  # worktree のベースディレクトリ（リポジトリ root からの相対、または絶対パス）
  base_dir: "../worktree"
```

- 相対パスの `base_dir` は Git リポジトリ root を基準に解決されます。
- 絶対パスもサポートしており、異なるドライブを指すことも可能です。


### フック設定

```yaml
version: "1.0"
defaults:
  base_dir: "../worktree"

hooks:
  post_create:
    # メイン worktree から新規 worktree へ、gitignore されたファイルをコピー
    - type: copy
      from: ".env"     # メイン worktree からの相対パス
      to: ".env"       # 新規 worktree からの相対パス

    # 新規 worktree 上でセットアップコマンドを実行
    - type: command
      command: "npm ci"
      env:
        NODE_ENV: "development"

    - type: command
      command: "npm run db:setup"
      work_dir: "."
```

挙動のポイント:

- `from` は常に **メイン worktree** からの相対パスとして解釈されます。
- `to` は新規 worktree からの相対パスとして解釈されます。
- `command` フックは新規 worktree 内で実行され、`env` や `work_dir` で環境変数や作業ディレクトリを指定できます。
- いずれかのフックが失敗した場合、`gwe add` 全体が失敗として扱われます。

> **セキュリティ注意**: `command` フックは `.wtp.yml` に記述された任意のコマンドを実行します。  
> 信頼できるリポジトリでのみ有効化し、`gwe add` を実行する前にフック定義の内容を確認してください。


終了ステータス
--------------

`gwe` はエラーの種類ごとに終了コードを使い分けます:

- `0`: 正常終了
- `1`: ユーザーエラー（引数のミス、存在しない worktree など）
- `2`: 設定ファイルエラー（無効な `.wtp.yml` など）
- `3`: Git コマンドの失敗
- `10`: 想定外の内部エラー


wtp との互換性
--------------

`gwe` は、ベースとしている wtp 2.3.4 と **ある程度の互換性** を保ちつつも、  
今後は gwe 固有の拡張や仕様変更も行っていくことを想定しています:

- `.wtp.yml` の設定フォーマットを共有します。
- worktree のレイアウトや命名規則、`add/list/remove/cd` の基本挙動は wtp に近づけています。
- PowerShell の `gwe init` / `gwe shell-init pwsh` は、macOS/Linux 上の wtp の体験を Windows に持ち込むことを意図しています。

一方で、現時点では次のような差分・未対応もあります:

- `cmd` 向けの `shell-init` は未実装です。
- 一部の「helpful error」（詳細なエラーメッセージ）やリモートブランチ解決ロジックは、wtp ほどリッチではありません。
- wtp 固有の追加フラグ（`list --quiet` / `--compact` など）はまだ Rust 版では露出していません。

詳細な対応状況・ギャップについては次を参照してください:

- `docs/spec.md`


ライセンス
----------

GWE は、MIT License のもとで公開されている  
[satococoa/wtp](https://github.com/satococoa/wtp) をベースにしたプロジェクトです。

本リポジトリも同じく MIT License に従って配布されており、詳細な条文は同梱の `LICENSE` を参照してください。  
上流プロジェクト wtp のライセンスについては、wtp リポジトリに含まれる `LICENSE` を参照してください。


