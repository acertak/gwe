mod common;
use common::TestRepo;
use std::fs;

#[test]
fn glob_copy_hook_copies_files() {
    let repo = TestRepo::new();

    // Setup files to copy
    fs::write(repo.path().join(".env.example"), "SECRET=example").unwrap();
    fs::write(repo.path().join("config.json"), "{}").unwrap();

    // Configure glob copy
    repo.command()
        .args(&["config", "add", "gwe.copy.include", ".env.*"])
        .assert()
        .success();
    
    repo.command()
        .args(&["config", "add", "gwe.copy.include", "*.json"])
        .assert()
        .success();

    // Create new worktree
    repo.command()
        .args(&["add", "HEAD", "--branch", "feature/new"])
        .assert()
        .success();

    // Check if worktree is created with repo name as prefix
    // repo path name -> "feature/new"
    let repo_name = repo.path().file_name().unwrap();
    let wt_dir = repo.worktrees_dir().join(repo_name).join("feature").join("new");
    assert!(wt_dir.exists(), "Worktree directory should exist at {}", wt_dir.display());

    // Check if files are copied
    assert!(wt_dir.join(".env.example").exists(), ".env.example should be copied");
    assert!(wt_dir.join("config.json").exists(), "config.json should be copied");

    // ignore.txt should NOT be copied (not matched)
    assert!(!wt_dir.join("ignore.txt").exists(), "ignore.txt should NOT be copied (only .env.* and *.json)");
}
