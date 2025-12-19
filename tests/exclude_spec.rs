mod common;
use common::TestRepo;
use std::fs;

#[test]
fn copy_exclude_honors_patterns() {
    let repo = TestRepo::new();

    // Setup files
    fs::write(repo.path().join(".env.test"), "TEST=1").unwrap();
    fs::write(repo.path().join(".env.prod"), "PROD=1").unwrap();
    
    fs::create_dir_all(repo.path().join("node_modules")).unwrap();
    fs::write(repo.path().join("node_modules/package.json"), "{}").unwrap();
    fs::write(repo.path().join("important.json"), "{}").unwrap();

    // Configure glob copy and exclude
    repo.command()
        .args(&["config", "add", "gwe.copy.include", ".env.*"])
        .assert()
        .success();
    
    repo.command()
        .args(&["config", "add", "gwe.copy.include", "**/*.json"])
        .assert()
        .success();

    repo.command()
        .args(&["config", "add", "gwe.copy.exclude", ".env.prod"])
        .assert()
        .success();

    repo.command()
        .args(&["config", "add", "gwe.copy.exclude", "node_modules/**"])
        .assert()
        .success();

    // Create new worktree
    repo.command()
        .args(&["add", "HEAD", "--branch", "feature/exclude-test"])
        .assert()
        .success();

    let repo_name = repo.path().file_name().unwrap();
    let wt_dir = repo.worktrees_dir().join(repo_name).join("feature").join("exclude-test");

    // Check inclusion
    assert!(wt_dir.join(".env.test").exists(), ".env.test should be copied");
    assert!(wt_dir.join("important.json").exists(), "important.json should be copied");

    // Check exclusion
    assert!(!wt_dir.join(".env.prod").exists(), ".env.prod should be EXCLUDED");
    assert!(!wt_dir.join("node_modules").exists(), "node_modules should be EXCLUDED");
    assert!(!wt_dir.join("node_modules/package.json").exists(), "node_modules/package.json should be EXCLUDED");
}

#[test]
fn copy_exclude_dir_name_excludes_contents() {
    let repo = TestRepo::new();

    // Setup directory with contents
    let legacy_dir = repo.path().join("legacy-src");
    fs::create_dir_all(&legacy_dir).unwrap();
    fs::write(legacy_dir.join("old.txt"), "old content").unwrap();
    fs::create_dir_all(legacy_dir.join("sub")).unwrap();
    fs::write(legacy_dir.join("sub/very_old.txt"), "very old").unwrap();
    
    fs::write(repo.path().join("keep.txt"), "keep me").unwrap();

    // Configure glob copy and exclude with JUST the directory name
    repo.command()
        .args(&["config", "add", "gwe.copy.include", "*.txt"])
        .assert()
        .success();

    repo.command()
        .args(&["config", "add", "gwe.copy.include", "legacy-src/**/*"])
        .assert()
        .success();

    repo.command()
        .args(&["config", "add", "gwe.copy.exclude", "legacy-src"])
        .assert()
        .success();

    // Create new worktree
    repo.command()
        .args(&["add", "HEAD", "--branch", "feature/dir-exclude"])
        .assert()
        .success();

    let repo_name = repo.path().file_name().unwrap();
    let wt_dir = repo.worktrees_dir().join(repo_name).join("feature").join("dir-exclude");

    // Check inclusion
    assert!(wt_dir.join("keep.txt").exists(), "keep.txt should be copied");

    // Check exclusion - the whole directory and its contents should be gone
    assert!(!wt_dir.join("legacy-src").exists(), "legacy-src directory itself should be EXCLUDED");
    assert!(!wt_dir.join("legacy-src/old.txt").exists(), "contents of legacy-src should be EXCLUDED");
    assert!(!wt_dir.join("legacy-src/sub/very_old.txt").exists(), "nested contents should be EXCLUDED");
}

