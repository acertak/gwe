mod common;
use common::TestRepo;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn editor_command_launches_configured_editor() {
    let repo = TestRepo::new();
    let dummy_log = repo.path().join("editor_log.txt");
    let dummy_script = repo.path().join("dummy_editor.sh");

    // Create a dummy editor script
    // It writes all arguments to editor_log.txt
    let script_content = format!(
        "#!/bin/sh\necho \"$@\" > \"{}\"",
        dummy_log.display()
    );
    fs::write(&dummy_script, script_content).unwrap();
    
    // Make it executable
    let mut perms = fs::metadata(&dummy_script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dummy_script, perms).unwrap();

    // Create a worktree to edit
    repo.command()
        .args(&["add", "HEAD", "-b", "feature/edit"])
        .assert()
        .success();
    
    // Path logic: feature/edit -> repo_name/feature/edit
    let repo_name = repo.path().file_name().unwrap();
    let expected_suffix = std::path::PathBuf::from(repo_name).join("feature").join("edit");

    // Configure editor
    repo.command()
        .args(&["config", "set", "gwe.editor.default", dummy_script.to_str().unwrap()])
        .assert()
        .success();

    // Run editor command
    repo.command()
        .args(&["editor", "feature/edit"])
        .assert()
        .success();

    // Verify the script was called with correct path
    let output = fs::read_to_string(&dummy_log).expect("dummy log should exist");
    // The first argument should be the worktree path
    // Note: The path passed to editor might be absolute and may contain /private prefix on Mac.
    let suffix_str = expected_suffix.to_str().unwrap();
    assert!(output.trim().ends_with(suffix_str), 
        "Log content: '{}', expected suffix: '{}'", output.trim(), suffix_str);
}

#[test]
fn ai_command_launches_configured_tool_with_args() {
    let repo = TestRepo::new();
    let dummy_log = repo.path().join("ai_log.txt");
    let dummy_script = repo.path().join("dummy_ai.sh");

    // Create a dummy ai script
    // It writes args and pwd
    let script_content = format!(
        "#!/bin/sh\necho \"args: $@\" > \"{}\"\npwd >> \"{}\"",
        dummy_log.display(),
        dummy_log.display()
    );
    fs::write(&dummy_script, script_content).unwrap();
    
    // Make it executable
    let mut perms = fs::metadata(&dummy_script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dummy_script, perms).unwrap();

    // Create a worktree
    repo.command()
        .args(&["add", "HEAD", "-b", "feature/ai"])
        .assert()
        .success();
    
    let repo_name = repo.path().file_name().unwrap();

    // Run ai command with custom tool and args
    // Use --ai to override config
    repo.command()
        .args(&["ai", "feature/ai", "--ai", dummy_script.to_str().unwrap(), "--", "--ask", "fix this"])
        .assert()
        .success();

    // Verify
    let output = fs::read_to_string(&dummy_log).expect("dummy log should exist");
    
    // Check args: path should be first, followed by --ask fix this
    // Note: The output of echo "$@" separates args by space
    let expected_suffix = std::path::PathBuf::from(repo_name).join("feature").join("ai");
    assert!(output.contains(expected_suffix.to_str().unwrap()), "Should contain path suffix");
    assert!(output.contains("--ask fix this"), "Should contain extra args");
    
    // Check working directory (pwd)
    // Resolve symlinks for pwd check if necessary, but usually test env is simple.
    // We just check if the second line ends with feature/ai
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 2);
    let pwd = lines[1];
    // Use end_with because of potential /private/var vs /var issues on Mac
    assert!(pwd.ends_with("feature/ai"), "PWD should end with feature/ai, got: {}", pwd);
}
