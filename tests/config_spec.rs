mod common;
use common::TestRepo;
use predicates::prelude::*;

#[test]
fn config_set_get_unset() {
    let repo = TestRepo::new();

    // Set
    repo.command()
        .args(&["config", "set", "gwe.defaultbranch", "main"])
        .assert()
        .success();

    // Get
    repo.command()
        .args(&["config", "get", "gwe.defaultbranch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main"));

    // Unset
    repo.command()
        .args(&["config", "unset", "gwe.defaultbranch"])
        .assert()
        .success();

    // Get again (should be empty)
    repo.command()
        .args(&["config", "get", "gwe.defaultbranch"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn config_add_multiple_values() {
    let repo = TestRepo::new();

    // Add
    repo.command()
        .args(&["config", "add", "gwe.copy.include", "*.txt"])
        .assert()
        .success();

    repo.command()
        .args(&["config", "add", "gwe.copy.include", "*.md"])
        .assert()
        .success();

    // Get (should return both lines)
    repo.command()
        .args(&["config", "get", "gwe.copy.include"])
        .assert()
        .success()
        .stdout(predicate::str::contains("*.txt").and(predicate::str::contains("*.md")));
}
