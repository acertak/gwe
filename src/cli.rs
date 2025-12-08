use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "gwe",
    version,
    about = "Windows-native worktree helper"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalOptions {
    /// 詳細ログ（stderr に出力）
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, conflicts_with = "quiet")]
    pub verbose: u8,
    /// 標準出力を最小限に（エラーのみ）
    #[arg(long = "quiet", action = ArgAction::SetTrue)]
    pub quiet: bool,
    /// 任意のディレクトリを Git リポジトリ root として扱う
    #[arg(long = "repo", value_name = "PATH")]
    pub repo: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// 登録済み worktree を一覧表示
    List(ListCommand),
    /// worktree を削除
    Rm(RmCommand),
    /// 指定 worktree の絶対パスを出力
    Cd(CdCommand),
    /// シェル統合をプロファイルにインストール
    Init(InitCommand),
    /// シェル初期化スクリプトを出力
    #[command(name = "shell-init")]
    ShellInit(ShellInitCommand),
    /// 設定の確認・変更
    Config(ConfigCommand),
    /// Cursor を起動
    Cursor(ToolCommand),
    /// Windsurf を起動
    Wind(ToolCommand),
    /// Antigravity を起動
    Anti(ToolCommand),
    /// Claude を新しいターミナルで起動
    Claude(ToolCommand),
    /// Codex を新しいターミナルで起動
    Codex(ToolCommand),
    /// Gemini を新しいターミナルで起動
    Gemini(ToolCommand),
    /// デフォルトエディタを起動 (-e)
    #[command(short_flag = 'e')]
    Edit(ToolCommand),
    /// デフォルトCLIを起動 (-c)
    #[command(short_flag = 'c')]
    RunCli(ToolCommand),
}

#[derive(Args, Debug, Clone)]
pub struct ToolCommand {
    /// 対象 worktree (または新規作成時のターゲット)
    #[arg(value_name = "WORKTREE")]
    pub target: Option<String>,
    
    /// 新規ブランチ名 (指定された場合、新規 worktree を作成して開く)
    #[arg(short = 'b', long = "branch", value_name = "BRANCH")]
    pub branch: Option<String>,

    /// 追跡する remote/branch (新規作成時用)
    #[arg(long = "track", value_name = "REMOTE/BRANCH")]
    pub track: Option<String>,

    /// 並列 worktree 作成数 (1-5、分割ペインで起動)
    #[arg(short = 'x', long = "multiplier", value_name = "COUNT", value_parser = clap::value_parser!(u8).range(1..=5))]
    pub multiplier: Option<u8>,

    /// ツールに渡す引数
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// 設定値を取得
    Get {
        key: String,
    },
    /// 設定値を設定
    Set {
        key: String,
        value: String,
        #[arg(long)]
        global: bool,
    },
    /// 値を追加（マルチバリュー用）
    Add {
        key: String,
        value: String,
        #[arg(long)]
        global: bool,
    },
    /// 設定値を削除
    Unset {
        key: String,
        #[arg(long)]
        global: bool,
    },
}

#[derive(Args, Debug, Clone, Copy)]
pub struct ListCommand {
    /// JSON 形式で出力
    #[arg(long = "json")]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RmCommand {
    /// 削除対象の worktree
    #[arg(value_name = "WORKTREE")]
    pub target: Option<String>,
    /// 強制削除
    #[arg(short = 'f', long = "force")]
    pub force: bool,
    /// 対応ブランチも削除
    #[arg(short = 'b', long = "with-branch")]
    pub with_branch: bool,
    /// ブランチが別の worktree にチェックアウトされていても削除
    #[arg(long = "force-branch", visible_alias = "fb")]
    pub force_branch: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CdCommand {
    /// 対象 worktree
    #[arg(value_name = "WORKTREE")]
    pub target: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ShellInitCommand {
    /// シェル種別（pwsh/cmd/bash）
    #[arg(value_enum)]
    pub shell: ShellKind,
}

#[derive(Args, Debug, Clone)]
pub struct InitCommand {
    /// シェル種別（省略時は pwsh）
    #[arg(long = "shell", value_enum, default_value_t = ShellKind::Pwsh)]
    pub shell: ShellKind,
    /// シェルのプロファイルファイルパス（例: $PROFILE）。省略時は既定の PowerShell プロファイルを使用
    #[arg(value_name = "PROFILE_PATH")]
    pub profile: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ShellKind {
    Pwsh,
    Cmd,
    Bash,
    Zsh,
}

impl ShellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShellKind::Pwsh => "pwsh",
            ShellKind::Cmd => "cmd",
            ShellKind::Bash => "bash",
            ShellKind::Zsh => "zsh",
        }
    }
}
