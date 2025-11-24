mod common;

use common::TestRepo;
use predicates::prelude::*;

#[test]
fn shell_init_pwsh_emits_wrapper_function() {
    TestRepo::new()
        .command()
        .args(["shell-init", "pwsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("function gwe"))
        .stdout(predicate::str::contains("Register-ArgumentCompleter"));
}

#[test]
fn shell_init_cmd_is_not_supported_yet() {
    TestRepo::new()
        .command()
        .args(["shell-init", "cmd"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("shell 'cmd' is not supported yet"));
}

#[test]
fn shell_init_bash_emits_wrapper_function() {
    TestRepo::new()
        .command()
        .args(["shell-init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gwe() {"))
        .stdout(predicate::str::contains("cd \"$dest\""));
}

#[test]
fn shell_init_zsh_emits_wrapper_function() {
    TestRepo::new()
        .command()
        .args(["shell-init", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gwe() {"))
        .stdout(predicate::str::contains("cd \"$dest\""));
}

