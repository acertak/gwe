use anyhow::Result;
use crate::cli::{ConfigAction, ConfigCommand};
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;

pub fn run(repo: &RepoContext, cmd: ConfigCommand) -> Result<()> {
    let runner = GitRunner::new(repo.clone());
    
    match cmd.action {
        ConfigAction::Get { key } => {
            // git config --get-all <key>
            match runner.run(["config", "--get-all", &key]) {
                Ok(output) => print!("{}", output.stdout),
                Err(_) => {
                    // Ignore error if key not found (git exits with 1)
                }
            }
        }
        ConfigAction::Set { key, value, global } => {
            let value_str = value.join(" ");
            let mut args = vec!["config"];
            if global {
                args.push("--global");
            }
            args.push(&key);
            args.push(&value_str);
            runner.run(args)?;
            eprintln!("Set '{}' = '{}'", key, value_str);
        }
        ConfigAction::Add { key, value, global } => {
            let value_str = value.join(" ");
            let mut args = vec!["config", "--add"];
            if global {
                args.push("--global");
            }
            args.push(&key);
            args.push(&value_str);
            runner.run(args)?;
            eprintln!("Added '{}' = '{}'", key, value_str);
        }
        ConfigAction::Unset { key, global } => {
            let mut args = vec!["config", "--unset"];
            if global {
                args.push("--global");
            }
            args.push(&key);
            // Ignore error if key doesn't exist
            if runner.run(args).is_ok() {
                eprintln!("Unset '{}'", key);
            }
        }
    }
    
    Ok(())
}
