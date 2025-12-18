GWE 仕様書
==========

1. 概要
-------

GWE (Git Worktree Extension) は、Git worktree を管理するための Windows ネイティブなヘルパー CLI です。Rust で実装されており、Windows 11、PowerShell、Bash、Zsh でのファーストクラスな体験を提供するように設計されています。

このドキュメントでは以下を規定します:

- コマンドラインインターフェース（グローバルオプションとサブコマンド）。
- 設定メカニズム（Git config）。
- GWE が Git および `git worktree` とどのように統合されるか。
- worktree 作成後のフックメカニズム。
- PowerShell、Bash、Zsh のためのシェル統合。
- ログ出力の挙動と終了コード。
- 自動テストスイートによって保証される動作仕様。

このドキュメントのすべての情報は、このリポジトリのソースコードとテストから直接導き出されています。推測や計画段階の動作は記述されていません。


2. 用語と概念
-------------

- **メイン worktree**  
  Git リポジトリのプライマリ worktree（`.git` ディレクトリに対応するもの）。`git worktree list --porcelain` では、最初の `worktree` エントリとなります。GWE はこのエントリを `is_main = true` としてマークします。

- **追加の worktree**  
  メイン worktree 以外の、`git worktree` によって管理される任意の worktree。

- **ベースディレクトリ (`base_dir`)**  
  GWE が worktree をデフォルトで管理するルートディレクトリです。Git config (`gwe.worktrees.dir`) で設定可能で、デフォルトはメインリポジトリルートからの相対パス `../worktree` です。

- **管理対象 worktree**  
  パスが設定された `base_dir` 下にある worktree（またはメイン worktree 自体）。`list`、`rm`、`cd` などの機能はこれらを特別に扱います。

- **表示名 (Display name)**  
  `gwe list` で表示され、コンパクトな識別子が有用な場所で使用される人間にとって読みやすい名前です。メイン worktree の場合は `"@"` です。`base_dir` 下の他の worktree の場合、表示名は `base_dir` から worktree ディレクトリへの相対パス（プラットフォームのパス区切り文字で結合）です。決定できない場合は、最後のパスコンポーネントまたはフルパス文字列がフォールバックとして使用されます。

- **worktree 名 (`cd` および `rm` 用)**  
  以下にマッチするユーザー指定のトークン:

  - `"@"`（メイン worktree）。
  - `"root"`（メイン worktree の大文字小文字を区別しないエイリアス）。
  - リポジトリ名（メインルートの最後のパスコンポーネント）、大文字小文字を区別しない。
  - ブランチ名（例: `feature/auth`）。
  - 表示名（例: Windows 上の `feature\auth`）。
  - worktree ディレクトリ名（最後のパスコンポーネント）。


3. アーキテクチャ概要
---------------------

クレートは主要な責務に対応するモジュールに構造化されています:

- `main`  
  OS エントリポイント。`gwe::run()` を呼び出し、エラーを終了コードにマッピングします。

- `lib`  
  `run()` 関数を公開し、Clap を介して CLI オプションをパースし、ロギングを初期化し、サブコマンドの実装にディスパッチします。

- `cli`  
  `clap::Parser` と `clap::Subcommand` を使用したグローバルオプションとサブコマンドの定義。これが CLI 表面の唯一の信頼できる情報源です。

- `config`  
  設定の読み込みと表現:

  - `config::loader`: git config からの読み込み。
  - `config::types`: 型付けされた設定 (`Config`, `Defaults`, `Hooks`, `Hook`) と実効パス解決 (`resolved_base_dir`)。

- `git`  
  Git との統合:

  - `git::rev`: メインルートと worktree ルートを発見する `RepoContext`。
  - `git::runner`: `git.exe` のラッパーである `GitRunner`（ロギングとエラータイプ付き）。
  - `git::worktree`: `git worktree list --porcelain` 出力の構造化された `WorktreeInfo` 値へのパース。

- `worktree`  
  worktree を操作するサブコマンドの実装:

  - `worktree::add`: `gwe add` の挙動（worktree 作成、パスマッピング、競合検出、作成後フック）。
  - `worktree::list`: `gwe list` の挙動（テーブルおよび JSON 出力）。
  - `worktree::rm`: `gwe rm` の挙動（worktree およびオプションのブランチ削除）。
  - `worktree::resolve`: `gwe cd` の挙動（名前解決）。
  - `worktree::common`: パス正規化、表示名、および「管理対象」チェックのための横断的なヘルパー。
  - `worktree::tool`: `gwe cursor`, `gwe wind`, `gwe anti` の挙動（外部ツール起動）。

- `hooks`  
  設定で定義された `copy`、`glob_copy`、`command` フックを実行する作成後フックエグゼキュータ（`HookExecutor`）。

- `shell`  
  シェル統合:

  - `shell::init`: シェルプロファイルの初期化（`gwe init`）。
  - `shell::config`: `gwe config` サブコマンド（git config ラッパー）。
  - `shell::pwsh`: PowerShell 関数および引数補完スクリプト。
  - `shell::bash`: Bash シェル関数スクリプト。
  - `shell::zsh`: Zsh シェル関数スクリプト。
  - `shell::cmd`: プレースホルダー（現在未使用）。

- `logging`  
  グローバルな verbosity フラグに基づく `tracing` サブスクライバの初期化。

- `error`  
  カテゴリと終了コードマッピングを持つコアアプリケーションエラータイプ `AppError`。

- `tests`  
  コンパイルされたバイナリを呼び出し、CLI の挙動を検証する統合テスト（`tests/*.rs`）。これらはクリティカルなフロー（add/list/rm/cd/config/shell‑init）のための実行可能な仕様として扱われます。


4. CLI 仕様
-----------

4.1 グローバルオプション
~~~~~~~~~~~~~~~~~~~~~~~~

トップレベル CLI は `cli::Cli` で定義され、`clap::Parser` を介してパースされます。

グローバルオプション（サブコマンドの前に利用可能）:

- `-v`, `--verbose` (カウントフラグ)  
  ログの verbosity を上げます。出現ごとに内部カウンタをインクリメントします:

  - `0` (デフォルト): ログレベル `WARN`。
  - `1`: ログレベル `DEBUG`。
  - `>= 2`: ログレベル `TRACE`。

- `--quiet`  
  ほとんどの診断出力を抑制し、ログレベルを `ERROR` に設定します。これは `--verbose` と競合します。

- `--repo <PATH>`  
  指定されたパスを Git worktree ルート発見の開始ディレクトリとして扱います。パスが相対パスの場合、カレントワーキングディレクトリに対して解決されます。パスがファイルを指している場合、その親ディレクトリが使用されます。結果のディレクトリが存在しない場合、エラーが返されます。

  `--repo` フラグは、リポジトリ外から `gwe` を実行し `list --json` が成功することを確認することで、統合テストで検証されています。


4.2 サブコマンド
~~~~~~~~~~~~~~~~

`Command` 列挙型は利用可能なサブコマンドを定義します:

- `add` (`AddCommand`)
- `list` (`ListCommand`)
- `rm` (`RmCommand`)
- `cd` (`CdCommand`)
- `init` (`InitCommand`)
- `shell-init` (`ShellInitCommand`)
- `config` (`ConfigCommand`)
- `cursor` (`ToolCommand`)
- `wind` (`ToolCommand`)
- `anti` (`ToolCommand`)

各サブコマンドについて以下に詳述します。


4.2.1 `gwe add`
^^^^^^^^^^^^^^^

**目的**  
設定されたベースディレクトリ下に新しい Git worktree を作成し、オプションでブランチの作成または追跡を行い、作成後フックを実行し、オプションでエディタで worktree を開きます。

**形式**

```text
gwe add [OPTIONS] [BRANCH_OR_COMMIT]
```

**オプション (AddCommand)**

- `BRANCH_OR_COMMIT` (位置引数、オプション)  
  - `--branch` が指定された場合、これは新しいブランチと worktree の開始点（commitish）として扱われます。
  - そうでない場合、これは必須であり、worktree の commitish として直接使用されます。

- `-b, --branch <BRANCH>`  
  worktree 用に作成する新しいブランチの名前。指定された場合:

  - 新しい worktree はブランチ名から派生したパスに作成されます。
  - 位置引数 `BRANCH_OR_COMMIT` が存在する場合、それが開始コミットとして使用されます。

- `--track <REMOTE/BRANCH>`  
  worktree 作成時に使用するリモート追跡ブランチ。この値は `git worktree add` に commitish として渡されます。ローカルブランチ名は、`--branch` が明示的に指定されない限り、remote/branch 文字列から推論されます。

- `-o, --open`  
  worktree 作成後、Cursor エディタで開きます。

**引数の検証**

挙動は `--branch`、`--track`、および `BRANCH_OR_COMMIT` の組み合わせによって決定されます:

1. `--track` が指定された場合:

   - ブランチ名は以下のいずれかです:
     - `--branch` で明示的に指定される、または
     - `REMOTE/BRANCH` の最初の `/` 以降の部分から推論される（例: `origin/feature/auth` → `feature/auth`）。
   - ブランチ名が決定できない場合、コマンドはユーザーエラーで失敗します:
     `"--track requires a branch name (use --branch or specify remote/branch)"`。

2. そうではなく `--branch` が指定された場合:

   - ブランチ名は `--branch` から取得されます。
   - `git worktree add` の commitish は、存在すれば `BRANCH_OR_COMMIT` から取得されます。そうでなければ `None` となり、Git は worktree ルートからのブランチ作成に独自のデフォルトを使用します。

3. それ以外（`--track` も `--branch` もない場合）:

   - `BRANCH_OR_COMMIT` は必須です。欠落または空白の場合、コマンドはユーザーエラーで失敗します: `"branch or commit is required"`。
   - commitish は指定された値に設定されます。
   - GWE によって新しいブランチは作成されません。`git worktree add` はパスと commitish のみで呼び出されます。

これらのエラーメッセージと終了コード 1（ユーザーエラー）はテストで検証されています。

**worktree パスの導出**

実効ベースディレクトリは `config.resolved_base_dir(main_root)` です。ここで `main_root` は `RepoContext` によって発見されたメインリポジトリルートです。これは以下を使用します:

- 存在するなら設定された `gwe.worktrees.dir` または `defaults.base_dir`。
- 存在しないならデフォルトの `../worktree`。
- 相対 `base_dir` は `main_root` に対して解決されます。絶対 `base_dir` はそのままです。

ベースディレクトリ内で、GWE はブランチまたはコミット識別子から相対パスを導出します:

- パスには最初のコンポーネントとしてリポジトリ名が含まれます:
  `base_dir/repo_name/branch_path`。
- リポジトリ名はメインリポジトリルートのディレクトリ名から導出されます。
- ブランチ/コミット識別子は `/` および `\` で分割されます。
- 各セグメントはサニタイズされます:
  - 空のセグメント、`"."`、または `".."` は `"_"` になります。
  - Windows で禁止されている文字 `<`、`>`、`:`、`"`、`|`、`?`、`*`、`\` は `"_"` に置換されます。
- 空でないサニタイズされたセグメントはパスコンポーネントとして結合されます。
- サニタイズの結果、相対パスが空になる場合、識別子全体を単一セグメントとしてサニタイズするフォールバックが使用されます。

最終的な worktree パスは `base_dir/repo_name/relative_path` です。

**競合検出**

worktree を作成する前に、GWE は以下を使用して競合をチェックします:

1. `git worktree list --porcelain` からの既存の worktree。
2. ファイルシステム。

チェック内容:

- ブランチ名が既知であり、既存の `WorktreeInfo` が同じブランチを持つ場合、コマンドはユーザーエラーで失敗します:

  ```text
  worktree for branch '<branch>' already exists: <existing_path>
  ```

- 既存の worktree のパス（正規化済み）がターゲットパス（正規化済み）と一致する場合、コマンドはユーザーエラーで失敗します:

  ```text
  worktree path already exists in git metadata: <path>
  ```

- ターゲットパスがファイルシステムに既に存在する場合、コマンドは以下で失敗します:

  ```text
  destination path already exists: <path>
  ```

- 導出された worktree 名が空のパスに解決される場合、コマンドは以下で失敗します:

  ```text
  worktree name resolves to an empty path: <identifier>
  ```

**Git 呼び出し**

GWE は `git worktree add` への引数を以下のように構築します:

- 常に: `["worktree", "add"]`。
- 追跡する場合: `"--track"` を追加。
- ブランチ名が設定されている場合: `"-b"` とブランチ名を追加。
- worktree パスを追加。
- commitish が設定されている場合: commitish を追加。

コマンドは `GitRunner::run` を介して実行されます。Git が非成功ステータスで終了した場合、GWE は:

- `stderr` を抽出し、トリムして:
  - 空でなければ、Git エラーメッセージとして表面化させます。
  - 空であれば、`"git worktree add failed without error output"` を表面化させます。

これらの失敗は Git エラーとして扱われ、終了コード 3 にマッピングされます。

**ユーザーに表示される出力**

成功時、`gwe add` は標準出力に 1 行を表示します:

```text
Created worktree '<display_name>' at <absolute_path>
```

その後、作成後フックを実行します（セクション 6 参照）。フックエグゼキュータは進行状況メッセージとフック固有の出力を表示します。

いずれかのフックが失敗した場合、`gwe add` は失敗します（エラーは伝播され `main` によって表示されます）。フックの失敗は内部エラーとして扱われ、終了コード 10 にマッピングされます。

`--open` が指定された場合、作成とフック実行の成功後に worktree が Cursor で開かれます。


4.2.2 `gwe list`
^^^^^^^^^^^^^^^^

**目的**  
現在のリポジトリに関連付けられた worktree を、人間が読みやすいテーブルまたはツールや補完に適した JSON として一覧表示します。

**形式**

```text
gwe list [--json]
```

**オプション (ListCommand)**

- `--json`  
  整形されたテーブルの代わりに JSON を出力します。

**データ収集**

`gwe list` は以下のステップを実行します:

1. `GitRunner` を介して `git worktree list --porcelain` を呼び出します。
2. 出力を `WorktreeInfo` エントリにパースします。各エントリには以下が含まれます:

   - `path` (絶対パス、正規化済み)。
   - `head` (Git によって報告された完全なコミットハッシュ)。
   - `branch` (オプションのブランチ名。detached HEAD では省略)。
   - `is_main` (パースされた最初のエントリはメインとしてフラグ付けされます)。
   - `is_detached` (`detached` 行が現れた場合 true)。
   - `locked` (`locked` 行からのオプションの理由)。
   - `prunable` (`prunable` 行からのオプションの理由)。

3. 実効ベースディレクトリを決定します: `config.resolved_base_dir(main_root)`。
4. `RepoContext::worktree_root()` から現在の worktree パスを決定します。
5. 各 worktree について、以下を含む `DisplayRow` を構築します:

   - `name` (表示名。セクション 2)。
   - `branch_display`:
     - 存在すればブランチ名。
     - そうでなければ `"detached"`。
   - `branch` (生のブランチ名。もしあれば)。
   - `head` (短縮されたコミットハッシュ。長い場合は最初の 8 文字)。
   - `status`: `"clean"` または `"dirty"`。以下によって決定:

     - worktree で `git status --short` を実行。
     - 空の出力を `"clean"`、それ以外を `"dirty"` として扱う。

   - `upstream`: オプションの上流参照。以下によって決定:

     - worktree で `git rev-parse --abbrev-ref --symbolic-full-name @{u}` を実行。
     - コマンドが成功した場合、トリムされた stdout を使用（空でない場合）。
     - コマンドが Git コマンドエラー（例: 上流が設定されていない）で失敗した場合、`None` として扱う。

   - `abs_path`: 正規化された絶対パス文字列。
   - `is_main`: 上記の通り。
   - `is_current`: worktree パスが現在の worktree パス（正規化後）と一致する場合 `true`。

これらの挙動は `tests/list_spec.rs` の統合テストによって検証されています。

**テーブル出力 (デフォルト)**

テーブルは動的にサイズ調整された列で表示されます。ヘッダーは:

```text
PATH  BRANCH  HEAD  STATUS  UPSTREAM  ABS_PATH
```

各行について:

- `PATH` は表示名を含みます。
- worktree が現在の worktree である場合、アスタリスク `*` が表示名に追加されます（例: `feature\current*`）。
- `BRANCH` は `branch_display` を含みます。
- `HEAD` は短縮されたコミットハッシュを含みます。
- `STATUS` は `"clean"` または `"dirty"` を含みます。
- `UPSTREAM` は上流文字列、なければ `"-"` を含みます。
- `ABS_PATH` は正規化された絶対パス文字列を含みます。

**JSON 出力**

`--json` が指定された場合、`gwe list` は整形された JSON を出力します:

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

フィールド:

- `name`: 表示名（例: `"@"`, `"feature\\auth"`）。
- `branch`: オプションのブランチ名。
- `head`: 短縮コミットハッシュ（最大 8 文字）。
- `status`: `"clean"` または `"dirty"`。
- `upstream`: オプションの上流参照文字列。
- `path`: `name` と同じ（論理パス）。
- `abs_path`: 絶対ファイルシステムパス。
- `is_main`: これがメイン worktree かどうか。
- `is_current`: これが現在の worktree かどうか。


4.2.3 `gwe rm`
^^^^^^^^^^^^^^

**目的**  
管理対象 worktree を削除し、オプションで対応するブランチも削除します。

**形式**

```text
gwe rm [OPTIONS] <WORKTREE>
```

**オプション (RmCommand)**

- `WORKTREE` (位置引数、必須)  
  ターゲット worktree 識別子。解決は `gwe cd`（セクション 4.2.4 参照）と同じルールに従いますが、メイン worktree は決して削除できません。

- `-f, --force`  
  `git worktree remove` に `--force` を渡し、dirty な worktree の削除を許可します。

- `-b, --with-branch`  
  worktree 削除後、関連付けられたローカルブランチがあればそれも削除します。

- `--force-branch` (エイリアス: `--fb`)  
  `--with-branch` と共に使用された場合、`git branch` に `-d` の代わりに `-D` を渡し、マージされていない場合でもブランチを削除できるようにします。`--with-branch` なしで指定された場合、コマンドはユーザーエラーで失敗します:

  ```text
  --force-branch requires --with-branch
  ```

**ターゲット解決**

`gwe rm`:

1. `git worktree list --porcelain` から `WorktreeInfo` エントリを列挙します。
2. 実効 `base_dir` を計算します。
3. 以下をスキップします:
   - メイン worktree (`is_main == true`)。
   - "管理対象" でない worktree（パスが `base_dir` 下にない）。
4. 残りの各 worktree に対して、以下の順序でターゲット文字列のマッチを試みます:

   - ブランチ名 (`info.branch`)。
   - worktree ディレクトリ名（最後のパスコンポーネント）。
   - 表示名（`base_dir` 下の相対パス）。

マッチが見つかった場合、その worktree が削除対象となります。  
マッチが見つからない場合、以下の形式のメッセージと共にユーザーエラーが返されます:

```text
worktree '<target>' not found
Available worktrees: <name1>, <name2>, ...
Run 'gwe list' to see available worktrees.
```

利用可能な名前のリストには、管理対象 worktree の表示名が含まれます。

**現在の worktree 保護**

削除前に、GWE は以下を比較します:

- 現在の worktree の正規化されたパス（`RepoContext` から）。
- ターゲット worktree の正規化されたパス。

これらが等しい場合、削除はユーザーエラーで拒否されます:

```text
cannot remove the current worktree '<target>': <path>
```

この挙動と、現在の worktree が損なわれないままであるという保証は、テストで検証されています。

**Git 呼び出し**

worktree 削除:

- 引数: `["worktree", "remove", (オプション "--force"), <path>]`。
- 成功時: worktree ディレクトリは Git によって削除されます。
- 失敗時:

  - stderr が空でなければ、Git エラーメッセージとして表面化させます。
  - stderr が空であれば、`"git worktree remove failed for <path> without error output"` という形式のエラーが使用されます。

ブランチ削除（`--with-branch` かつブランチが利用可能な場合）:

- `--force-branch` に応じて `git branch -d <branch>` または `git branch -D <branch>` を使用します。
- 失敗時:

  - stderr が空でなければ、エラーメッセージとして直接使用されます。
  - そうでなければ、`"failed to remove branch '<branch>'"` のような汎用エラーが使用されます。

これらすべての失敗は Git エラーとして扱われ、終了コード 3 にマッピングされます。

**ユーザーに表示される出力**

worktree 削除成功時、GWE は以下を表示します:

```text
Removed worktree '<target>' at <absolute_path>
```

ブランチ削除が要求され成功した場合、以下も表示します:

```text
Removed branch '<branch>'
```


4.2.4 `gwe cd`
^^^^^^^^^^^^^^

**目的**  
worktree 識別子を絶対パスに解決します。シェル統合と組み合わせることで、シェルレベルでのディレクトリ変更が可能になります。

**形式**

```text
gwe cd <WORKTREE>
```

**オプション (CdCommand)**

- `WORKTREE` (位置引数、必須)  
  ターゲット worktree 識別子。欠落しているか空の名前に解決される場合、コマンドはユーザーエラーで失敗します:

  ```text
  worktree name is required
  ```

**名前のサニタイズ**

入力はまず以下によってサニタイズされます:

- 先頭/末尾の空白の削除。
- 末尾の `*` の削除（例: 現在の worktree がアスタリスクでマークされている `gwe list` からコピーされた値）。

サニタイズされた値が空の場合、コマンドは上記のエラーで失敗します。

**解決アルゴリズム**

`gwe cd`:

1. `git worktree list --porcelain` から `WorktreeInfo` エントリを列挙します。
2. 以下を計算します:
   - `config.resolved_base_dir(main_root)` として `base_dir`。
   - メインルートディレクトリ名から `repo_name`。

3. 各 worktree について、順に:

   - メイン worktree の場合 (`is_main == true`):

     - ターゲット文字列が以下の場合にマッチします:
       - `"@"`
       - `"root"` (大文字小文字を区別しない)
       - `repo_name` と等しい (大文字小文字を区別しない)
       - メインブランチ名と等しい (もしあれば)

     - マッチした場合、この worktree パスを即座に返します。

   - 非メイン worktree の場合:

     - `base_dir` 下で管理されていない worktree はスキップします。
     - ターゲット文字列が以下と等しい場合にマッチします:
       - ブランチ名 (`info.branch`)、または
       - 表示名、または
       - worktree ディレクトリ名（最後のパスコンポーネント）。

4. マッチが見つからない場合、以下の形式のメッセージと共にユーザーエラーが返されます:

```text
worktree '<target>' not found
Available worktrees: <names...>
Run 'gwe list' to see available worktrees.
```

利用可能な名前のリストには、`"@"`、メインブランチ名（もしあれば）、リポジトリ名、および管理対象 worktree の表示名が含まれます。

**出力**

成功時、`gwe cd` は解決された worktree の正規化された絶対パスを標準出力に表示し、改行を続けます。成功パスでは他に何も出力されません。

統合テストは以下をアサートします:

- `gwe cd @` がリポジトリルートに解決される。
- `gwe cd <display_name>` が `gwe add` 後に正しく解決される。
- 不明な worktree に対するエラーには、"Available worktrees" リストと `Run 'gwe list'` ヒントの両方が含まれる。


4.2.5 `gwe init`
^^^^^^^^^^^^^^^^

**目的**  
`gwe` 用の関数を追記することで、シェルプロファイルにシェル統合をインストールします。

**形式**

```text
gwe init [--shell <SHELL>] [PROFILE_PATH]
```

**オプション (InitCommand)**

- `--shell <SHELL>` (ValueEnum, デフォルト: `pwsh`)  
  シェル種別。サポートされている値:

  - `pwsh` (PowerShell; 完全サポート)。
  - `bash` (Bash; サポート)。
  - `zsh` (Zsh; サポート)。
  - `cmd` (Windows Command Prompt; 未サポート)。

- `PROFILE_PATH` (オプションの位置引数パス)  
  変更されるプロファイルファイルへのパス。省略された場合、GWE はシェル種別に基づいてデフォルトのプロファイルパスを計算します:

  - `pwsh`: `<HOME>\Documents\PowerShell\Microsoft.PowerShell_profile.ps1`
  - `bash`: `<HOME>/.bashrc`
  - `zsh`: `<HOME>/.zshrc`

  `HOME` は `USERPROFILE` または `HOME` 環境変数から決定されます。

**挙動**

- サポートされているシェル (`pwsh`, `bash`, `zsh`) の場合:

  - プロファイルディレクトリが存在することを確認し、必要なら作成します。
  - 既存のプロファイル内容（もしあれば）を読み込みます。
  - 内容にマーカー行 `# gwe shell integration` が既に含まれている場合、変更を行いません（冪等）。
  - そうでなければ、プロファイルファイルを追記モードで開き、オプションで改行を挿入し、以下を追記します:
    - マーカー行 `# gwe shell integration`。
    - 対応するシェルモジュールからのシェルスクリプト。

- `cmd` の場合:

  - 以下のメッセージと共にエラーを返します:
    `"shell 'cmd' is not supported yet"`。

  このエラーは汎用エラー（終了コード 10）として扱われます。


4.2.6 `gwe shell-init`
^^^^^^^^^^^^^^^^^^^^^^

**目的**  
シェル統合スクリプトをプロファイルファイルに書き込む代わりに標準出力に出力します。これにより手動検査や構成が可能になります。

**形式**

```text
gwe shell-init <SHELL>
```

**オプション (ShellInitCommand)**

- `shell` (ValueEnum; 必須)  
  シェル種別 (`pwsh`, `bash`, `zsh`, `cmd`)。

**挙動**

- サポートされているシェル (`pwsh`, `bash`, `zsh`) の場合:

  - 対応するシェルスクリプトの内容を標準出力に書き込み、出力をフラッシュします。

- `cmd` の場合:

  - 以下のメッセージと共にエラーを返します:
    `"shell 'cmd' is not supported yet"`。

統合テストは以下を確認します:

- `gwe shell-init pwsh` が `function gwe` と `Register-ArgumentCompleter` を含むスクリプトを出力する。
- `gwe shell-init cmd` が適切なエラーメッセージで失敗する。


4.2.7 `gwe config`
^^^^^^^^^^^^^^^^^^

**目的**  
Git 設定値の取得、設定、追加、または設定解除を行います。これは便利なインターフェースを持つ `git config` コマンドのラッパーです。

**形式**

```text
gwe config get <KEY>
gwe config set <KEY> <VALUE> [-g|--global]
gwe config add <KEY> <VALUE> [-g|--global]
gwe config unset <KEY> [-g|--global]
```

**サブコマンド**

- `get <KEY>`  
  `git config --get-all` を使用して指定されたキーのすべての値を取得します。キーが存在しない場合、何も出力しません（エラーなし）。

- `set <KEY> <VALUE> [-g|--global]`  
  指定されたキーを与えられた値に設定します。`-g` または `--global` が指定された場合、グローバル Git 設定ファイルを使用します。

- `add <KEY> <VALUE> [-g|--global]`  
  `git config --add` を使用して指定されたキーに値を追加します（マルチバリューキー用）。リストのような設定エントリに便利です。

- `unset <KEY> [-g|--global]`  
  `git config --unset` を使用して指定されたキーを削除します。キーが存在しない場合は何もしません。

**一般的な設定キー**

- `gwe.defaultbranch`: デフォルトのブランチ名。
- `gwe.copy.include`: コピーするファイルパターンのためのマルチバリューキー（Glob copy フック）。
- `gwe.hook.postcreate`: 実行するコマンドのためのマルチバリューキー（Command フック）。


4.2.8 `gwe cursor` / `gwe wind` / `gwe anti`
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**目的**  
外部ツール（Cursor、Windsurf、Antigravity）を worktree で起動します。

**形式**

```text
gwe cursor [WORKTREE] [-- <ARGS>...]
gwe wind [WORKTREE] [-- <ARGS>...]
gwe anti [WORKTREE] [-- <ARGS>...]
```

**オプション (ToolCommand)**

- `WORKTREE` (位置引数、オプション)  
  ターゲット worktree 識別子。省略された場合、現在の worktree (`@`) を使用します。

- `-- <ARGS>...`  
  ツールに渡す追加引数。

**挙動**

1. `gwe cd` と同じアルゴリズムを使用して worktree パスを解決します。
2. 対応するツールを worktree パスを引数として起動します。
3. `-- <ARGS>...` からの追加引数を渡します。

**エラーハンドリング**

- ツールコマンドが失敗した場合、終了ステータスが報告されます。


5. 設定
-------

GWE は Git 設定変数（推奨）を使用して設定します。

5.1 Git 設定
~~~~~~~~~~~~

GWE は `gwe.*` 名前空間の git config 変数から設定を読み込みます。これらは `git config` またはヘルパーコマンド `gwe config` を使用して設定できます。

サポートされているキー:

- `gwe.worktrees.dir` (パス)
  管理対象 worktree のベースディレクトリ。デフォルトの `../worktree` を上書きします。

- `gwe.defaultbranch` (文字列)
  デフォルトのブランチ名。

- `gwe.copy.include` (マルチバリュー文字列)
  メイン worktree から新しい worktree にコピーするファイルの Glob パターン。各値が `glob_copy` フックを作成します。

- `gwe.hook.postcreate` (マルチバリュー文字列)
  worktree 作成後に実行するシェルコマンド。各値が `command` フックを作成します。


6. フック実行
-------------

フック実行は `HookExecutor` によって行われます。

`gwe add` 成功時、GWE は:

1. `HookExecutor` を構築します。
2. 以下で `execute_post_create_hooks` を呼び出します:

   - 標準出力をラップするミュータブルライター。
   - 新しく作成された worktree のパス。

3. `execute_post_create_hooks`:

   - `hooks.post_create` が空であれば即座に戻ります。
   - そうでなければ:
     - 開始メッセージを表示します:

       ```text
       Executing post-create hooks...
       ```

     - 各フックについて（1 ベースのインデックス）:

       - 表示します:

         ```text
         → Running hook <i> of <n>...
         ```

       - フック（`copy`、`glob_copy`、または `command`）を実行します。
       - フックが成功した場合、表示します:

         ```text
         ✓ Hook <i> completed
         ```

     - すべてのフックが成功した後、表示します:

       ```text
       ✓ All hooks executed successfully
       ```

4. `copy`、`glob_copy`、または `command` 実行中のエラーは、それ以降のフックを停止し、`gwe add` を失敗させます。

統合テストは以下を検証します:

- フックが worktree 作成後に実行されること。
- `copy` フックがメイン worktree から新しい worktree へファイルを正しくコピーすること。
- `glob_copy` フックがパターンに一致するファイルを正しくコピーすること。
- `command` フックが期待される出力を含むファイル（`hook.log`）を作成できること。
- フック実行の成功メッセージが `stdout` に存在すること。


7. Git と worktree の統合
-------------------------

7.1 リポジトリコンテキスト発見
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

`RepoContext::discover` は以下のように Git コンテキストを決定します:

1. 開始ディレクトリを決定します:

   - `--repo <PATH>` が提供された場合、それを解決します:
     - 相対パスはカレントディレクトリに対して解決されます。
     - パスがファイルを指している場合、その親ディレクトリが使用されます。
     - パス（またはその親）が存在しない場合、エラーが返されます。
   - そうでなければ、`std::env::current_dir()` を使用します。

2. 開始ディレクトリで `git rev-parse --show-toplevel` を実行して worktree ルートを取得します。これは正規化され、コマンドが失敗した場合はコンテキストエラーが返されます。

3. worktree ルートで `git rev-parse --git-common-dir` を実行して共通 Git ディレクトリを取得します。このパスは相対パスであれば worktree ルートに対して解決され、正規化されます。

4. 解決された共通ディレクトリが `.git` で終わる場合、その親ディレクトリがメインリポジトリルートとして使用されます。そうでなければ、正規化された共通ディレクトリが `main_root` として直接使用されます。

5. リポジトリ名 (`repo_name`) は `main_root` の最後のパスコンポーネントです。決定できない場合、`main_root` の表示文字列が使用されます。

`RepoContext` は以下を提供します:

- `worktree_root()`: 現在の worktree へのパス。
- `main_root()`: メインリポジトリルートへのパス。
- `repo_name()`: 導出されたリポジトリ名。
- `is_main_worktree()`: 現在の worktree がメインルートと等しいかどうか（正規化後）。


7.2 Git コマンド実行
~~~~~~~~~~~~~~~~~~~~

`GitRunner` は `git.exe` への呼び出しをカプセル化します:

- すべてのコマンドは、以下と共に `DEBUG` 以上でログ記録されます:

  - ワーキングディレクトリパス。
  - `format_command` によって整形された完全なコマンド文字列。

- `GitRunner::run`:

  - コマンドを実行し、成功時に `GitOutput` を返します。
  - コマンドが非ゼロステータスで終了した場合、終了ステータスとキャプチャされた `stdout` および `stderr` を含む `GitError::CommandFailed` を返します。

- `GitRunner::run_in`:

  - `run` と同じですが、明示的なワーキングディレクトリを取ります。

- `GitRunner::run_with_status` / `run_with_status_in`:

  - Git が失敗した場合でも `GitOutput` を返し、呼び出し元が終了コードと出力を自分で検査して解釈できるようにします。

`GitOutput` は以下を公開します:

- `command`: 整形されたコマンド文字列。
- `status`: `ExitStatus`。
- `stdout`: キャプチャされた標準出力。
- `stderr`: キャプチャされた標準エラー。

`GitError` バリアント:

- `Spawn`  
  コマンドを開始できませんでした（例: `git` が見つからない）。ワーキングディレクトリ、コマンド文字列、および基礎となる `io::Error` を含みます。

- `CommandFailed`  
  `git` が非ゼロステータスで終了しました。ステータスとキャプチャされた `stdout`/`stderr` を含みます。

- `InvalidUtf8`  
  `git` からの出力を UTF‑8 としてデコードできませんでした。

`GitRunner` からのエラーは一般的に Git アプリケーションエラーとしてラップされ、`main` によって終了コード 3 にマッピングされます。


7.3 worktree パース
~~~~~~~~~~~~~~~~~~~

`git::worktree::list_worktrees`:

- `GitRunner` を介して `git worktree list --porcelain` を実行します。
- 行を以下のように `WorktreeInfo` にパースします:

  - `worktree <path>`: 新しいエントリを開始し、`path` を `<path>` の正規化バージョンに設定します。
  - `HEAD <hash>`: 完全なコミットハッシュを設定します。
  - `branch refs/heads/<branch>`: `branch` を `<branch>` に設定します。
  - `detached`: `is_detached = true` を設定し、`branch` が使用されるのを防ぎます。
  - `locked <reason>` または `locked`: それに応じて `locked` を設定します。
  - `prunable <reason>` または `prunable`: それに応じて `prunable` を設定します。
  - 空行はエントリを区切ります。

- パース後、リストの最初のエントリは `is_main = true` としてマークされます。

この挙動は `git::worktree` のユニットテストによってカバーされています。


8. シェル統合
-------------

8.1 PowerShell
~~~~~~~~~~~~~~

`gwe shell-init pwsh` によって出力され、`gwe init` によって追記される PowerShell スクリプトには以下が含まれます:

- 以下を行うヘルパー関数 `Get-GweExePath`:

  - まず `Get-Command` を使用して `gwe.exe` を探します。
  - `Get-Command gwe -CommandType Application` にフォールバックします。
  - 実行可能ファイルが見つからない場合はエラーをスローします。

- 以下を行う `gwe` 関数:

  - 引数を実際の `gwe.exe` に転送します。
  - `stdout` と終了コードをキャプチャします。
  - 終了コードがゼロで、最初の引数が `cd` の場合:

    - 出力の最後の行を読み込み、トリムし、空でなければ、そのパスに `Set-Location` を呼び出します。

  - そうでなければ、出力（もしあれば）をコンソールに書き込みます。
  - `$global:LASTEXITCODE` を `gwe.exe` からの終了コードに設定します。

- `Register-ArgumentCompleter` を介して登録される引数補完:

  - 最初の引数（サブコマンド）を補完する場合、以下を提案します:
    `add`, `list`, `remove`, `cd`, `shell-init`。
  - サブコマンドが `cd` の場合:
    - `gwe list --json` を呼び出します。
    - JSON を `.name` フィールドを持つオブジェクトにパースします。
    - 各 `name` を補完候補として提案します。
    - `"@"` という名前を、PowerShell のパース問題を避けるために `'@'` とクォートして提案する特別扱いをします。

これらの挙動は `shell::pwsh` のユニットテストと `tests/shell_spec.rs` の統合テストによってアサートされています。


8.2 Bash
~~~~~~~~

`gwe shell-init bash` によって出力され、`gwe init --shell bash` によって追記される Bash スクリプトには以下が含まれます:

- 以下を行う `gwe` 関数:

  - 最初の引数が `cd` の場合:
    - 残りの引数で `command gwe cd` を呼び出します。
    - 終了コードをキャプチャします。
    - 成功した場合、ディレクトリを出力先に変更します。
    - 失敗した場合、終了コードを返します。

  - 他のすべてのコマンドについては、実際の `gwe` 実行可能ファイルにパススルーします。


8.3 Zsh
~~~~~~~

`gwe shell-init zsh` によって出力され、`gwe init --shell zsh` によって追記される Zsh スクリプトは、構文が互換であるため Bash スクリプトと同一です。


9. ログとエラーハンドリング
---------------------------

9.1 ログ
~~~~~~~~

`logging::init` は `GlobalOptions` に基づいて `tracing_subscriber` を設定します:

- `--quiet` が設定されている場合:

  - 最大ログレベルは `ERROR`。

- そうではなく `--verbose` が:

  - `0`: 最大レベルは `WARN`。
  - `1`: 最大レベルは `DEBUG`。
  - `>= 2`: 最大レベルは `TRACE`。

ログはタイムスタンプやターゲット情報なしで標準エラー (`stderr`) に書き込まれます。統合テストは、`--verbose` を使用すると `"Executing git command"` などのデバッグ行が出力されることを検証します。


9.2 エラータイプと終了コード
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

GWE は 4 つのバリアントを持つ構造化されたエラータイプ `AppError` を使用します:

- `User(String)`  
  引数の不足や無効な組み合わせなどのユーザーミス用。

- `Config(String)`  
  設定に関連する問題用。

- `Git(String)`  
  基礎となる Git コマンドの失敗用。

- `Internal(String)`  
  予期しない内部エラー用。

各バリアントは終了コードにマッピングされます:

- `User` → `1`
- `Config` → `2`
- `Git` → `3`
- `Internal` → `10`

統合テストは以下をアサートします:

- `add` 中の Git 失敗は終了コード 3 を生成する。

`main` 関数はエラーを以下のように処理します:

1. `gwe::run()` を呼び出します:

   - `Ok(ExitCode)` の場合: その終了コードを返します（現在は常に成功）。

2. `Err(error)` の場合:

   - まず `AppError` への直接ダウンキャストを試みます。
   - 成功した場合:
     - エラーメッセージを表示します。
     - 対応する `AppError::exit_code()` で終了します。

   - そうでなければ、エラー原因チェーンを反復し:

     - `AppError` 原因が見つかった場合:
       - そのメッセージと終了コードを使用します。

     - `GitError` 原因が見つかった場合:
       - Git エラーとして扱います:
         - 終了コード: 3。
         - メッセージ: `GitError` の `Display` 出力。

   - 上記のいずれも見つからない場合:

     - トップレベルエラーの `Display` 出力をメッセージとして使用します。
     - 終了コード 10 を使用します。

すべての失敗ケースにおいて、選択されたメッセージは `stderr` に表示されます。


10. テスト戦略と動作保証
------------------------

`tests/` 内のテストは実行可能な仕様として機能します。主な保証には以下が含まれます:

- **リポジトリ発見と `--repo`**  
  `--repo` が提供された場合、`gwe list --json` はリポジトリの内外両方で機能し、少なくともメイン worktree エントリを返します。

- **設定ハンドリング**  
  - 設定されていない場合、Git config のデフォルト値が使用されます。

- **`add` の挙動**  
  - 設定された `base_dir` 下に、ブランチ名から派生しリポジトリ名をパスコンポーネントとして含むパスで新しい worktree を作成します。
  - `--branch`/`--track` が使用されない場合、ブランチまたはコミット引数を要求します。
  - ブランチの競合を検出し、明確なメッセージで報告します。
  - `--track` 引数の要件を強制します。
  - 作成後フックを実行し、その効果（コピーされたファイルやコマンド生成ファイル）を観察します。
  - 作成後の Cursor 起動のための `--open` をサポートします。

- **`cd` の挙動**  
  - `gwe cd @` はリポジトリルートに解決されます。
  - `gwe cd <display_name>` は適切な worktree パスに解決されます。
  - 不明な worktree は、"Available worktrees" リストと `Run 'gwe list'` ヒントを含む「見つかりません」エラーを生成します。

- **`list` の挙動**  
  - `list --json` は `name = "@"` および `branch = "main"` を持つメイン worktree を含みます。
  - `list` は dirty な worktree をマークし、設定されている場合は上流ブランチを表示します。
  - `list` は `PATH` 列で現在の worktree をアスタリスクでマークします。
  - `list --json` は `is_main` および `is_current` フラグを正しく反映します。

- **`rm` の挙動**  
  - `rm --with-branch --force-branch` は worktree ディレクトリとそのブランチの両方を削除します。
  - `rm` は現在設定されている `base_dir` 下の worktree にのみ影響します。`base_dir` を変更すると、既存の worktree が管理対象外となり、削除から保護される可能性があります。
  - 現在の worktree を削除しようとすると、明確なエラーで失敗し、ディレクトリはそのまま残ります。
  - `--with-branch` なしの `--force-branch` は拒否されます。

- **`config` の挙動**
  - `config set` / `config get` / `config unset` は正しく動作します。
  - `config add` は同じキーに対して複数の値を許可します。

- **`cursor` / `wind` / `anti` の挙動**
  - 対応するツールを worktree パスで起動します。
  - `--` による追加引数をサポートします。

- **シェル統合**  
  - `shell-init pwsh` はラッパー関数と引数補完の両方を含むスクリプトを出力します。
  - `shell-init bash` および `shell-init zsh` はラッパー関数を出力します。
  - `shell-init cmd` は明示的にサポートされておらず、ドキュメント化されたエラーメッセージで失敗します。

- **ヘルプとバージョン**  
  - `gwe --help` は使用法、説明を表示し、`shell-init`、`config`、`cursor`、`wind`、`anti` を含むサブコマンドを一覧表示します。
  - `gwe --version` は `CARGO_PKG_VERSION` で定義されたパッケージバージョンを表示します。

- **Verbosity**  
  - `--verbose` を使用すると、Git コマンド実行メッセージを含むデバッグログが出力されます。

GWE への将来の変更は、新しい仕様を反映するためにテストが意図的に更新されない限り、これらの挙動を維持する必要があります。
