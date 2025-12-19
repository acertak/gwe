mod common;

use common::TestRepo;
use predicates::prelude::*;
use std::path::Path;

#[test]
fn rm_with_branch_deletes_branch_and_worktree() {
    let repo = TestRepo::new();
    let branch = "feature/remove";
    let worktree_path = repo.worktree_path_for(branch);

    // Setup worktree using git directly since gwe add is not available
    repo.git(&["worktree", "add", "-b", branch, &worktree_path.to_string_lossy(), "HEAD"]);

    repo.command()
        .args(["rm", "--with-branch", branch])
        .assert()
        .success()
        .stdout(predicate::str::contains(worktree_path.to_string_lossy()));

    assert!(
        !worktree_path.exists(),
        "worktree directory should be removed"
    );
    assert!(
        !branch_exists(repo.path(), branch),
        "branch should be deleted when --with-branch is supplied"
    );
}

#[test]
fn rm_only_targets_current_base_dir() {
    let repo = TestRepo::new();
    let branch = "feature/legacy";
    let worktree_path = repo.worktree_path_for(branch);
    
    repo.git(&["worktree", "add", "-b", branch, &worktree_path.to_string_lossy(), "HEAD"]);

    // Change base_dir so that existing worktree falls outside managed scope
    repo.set_config("gwe.worktrees.dir", "alt-worktrees");

    repo.command()
        .args(["rm", branch])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));

    assert!(
        worktree_path.exists(),
        "worktree should remain because it is unmanaged under new base_dir"
    );
}

#[test]
fn rm_rejects_current_worktree() {
    let repo = TestRepo::new();
    let branch = "feature/current";
    let worktree_path = repo.worktree_path_for(branch);
    
    repo.git(&["worktree", "add", "-b", branch, &worktree_path.to_string_lossy(), "HEAD"]);

    repo.command_in(&worktree_path)
        .args(["rm", branch])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot remove the current worktree"));

    assert!(worktree_path.exists(), "current worktree must remain intact");
}

#[test]
fn rm_implies_force_branch() {
    let repo = TestRepo::new();
    let branch = "feature/unmerged";
    let worktree_path = repo.worktree_path_for(branch);
    
    // Setup unmerged branch
    repo.git(&["worktree", "add", "-b", branch, &worktree_path.to_string_lossy(), "HEAD"]);
    
    // Add a commit in the worktree
    std::fs::write(worktree_path.join("unmerged.txt"), "unmerged content").unwrap();
    repo.run_in_worktree(&worktree_path, &["add", "unmerged.txt"]);
    repo.run_in_worktree(&worktree_path, &["commit", "-m", "unmerged commit"]);

    // Now branch is unmerged. "git branch -d" should fail, but "gwe rm -b" should succeed.
    repo.command()
        .args(["rm", "-b", branch])
        .assert()
        .success();

    assert!(
        !branch_exists(repo.path(), branch),
        "branch should be deleted even if unmerged when -b is used"
    );
}

#[test]
fn rm_implies_force_worktree() {
    let repo = TestRepo::new();
    let branch = "feature/dirty";
    let worktree_path = repo.worktree_path_for(branch);
    
    // Setup worktree
    repo.git(&["worktree", "add", "-b", branch, &worktree_path.to_string_lossy(), "HEAD"]);
    
    // Make worktree dirty
    std::fs::write(worktree_path.join("dirty.txt"), "dirty content").unwrap();

    // "git worktree remove" without --force should fail, but "gwe rm" should succeed.
    repo.command()
        .args(["rm", branch])
        .assert()
        .success();

    assert!(
        !worktree_path.exists(),
        "worktree should be removed even if dirty"
    );
}

fn branch_exists(repo_path: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .current_dir(repo_path)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .expect("git show-ref")
        .success()
}

