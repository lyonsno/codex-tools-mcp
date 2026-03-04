use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[test]
fn prints_version() {
    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn applies_patch_and_creates_file() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Add File: hello.txt\n+hello world!\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure: {:?}",
        output
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("\"result\""),
        "expected responses in stdout: {stdout}"
    );

    let hello_path = dir.path().join("hello.txt");
    let contents = std::fs::read_to_string(&hello_path).expect("hello.txt created");
    assert_eq!(contents.trim(), "hello world!");
}

#[test]
fn applies_patch_in_explicit_workdir() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Add File: hello.txt\n+hello workdir!\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        let _ = stdin.write_all(input.as_bytes());
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let hello_path = work_dir.path().join("hello.txt");
    let contents = std::fs::read_to_string(&hello_path).expect("hello.txt created in --workdir");
    assert_eq!(contents.trim(), "hello workdir!");

    let wrong_path = launch_dir.path().join("hello.txt");
    assert!(
        !wrong_path.exists(),
        "hello.txt should not be created in process cwd"
    );
}

#[test]
fn restrict_to_workdir_still_allows_in_tree_patch_paths() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Add File: nested/ok.txt\n+safe write\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        !stdout.contains("\"isError\":true"),
        "did not expect apply_patch tool error: {stdout}"
    );

    let safe_path = work_dir.path().join("nested").join("ok.txt");
    let contents = std::fs::read_to_string(&safe_path).expect("ok.txt created in --workdir");
    assert_eq!(contents.trim(), "safe write");
}

#[test]
fn restrict_to_workdir_blocks_parent_directory_escape() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Add File: ../escape.txt\n+should be blocked\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("\"isError\":true"),
        "expected apply_patch rejection for parent directory escape: {stdout}"
    );

    let escaped_path = work_dir
        .path()
        .parent()
        .expect("work_dir has parent")
        .join("escape.txt");
    assert!(
        !escaped_path.exists(),
        "escape.txt should not be created outside --workdir"
    );
}

#[cfg(unix)]
#[test]
fn restrict_to_workdir_blocks_symlink_escape() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");
    let outside_dir = tempdir().expect("create outside temp dir");
    symlink(outside_dir.path(), work_dir.path().join("link")).expect("create symlink in workdir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Add File: link/escape.txt\n+should be blocked\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("\"isError\":true"),
        "expected apply_patch rejection for symlink escape: {stdout}"
    );

    let escaped_path = outside_dir.path().join("escape.txt");
    assert!(
        !escaped_path.exists(),
        "escape.txt should not be created via symlink outside --workdir"
    );
}

#[cfg(unix)]
#[test]
fn restrict_to_workdir_allows_delete_of_broken_symlink() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");
    let broken_link = work_dir.path().join("dead");
    symlink("missing-target", &broken_link).expect("create broken symlink in workdir");
    assert!(
        broken_link.exists() || std::fs::symlink_metadata(&broken_link).is_ok(),
        "broken symlink should exist as a directory entry"
    );

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Delete File: dead\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        !stdout.contains("\"isError\":true"),
        "expected delete of broken in-tree symlink to be allowed: {stdout}"
    );
    assert!(
        std::fs::symlink_metadata(&broken_link).is_err(),
        "broken symlink should be deleted"
    );
}

#[cfg(unix)]
#[test]
fn restrict_to_workdir_blocks_update_through_symlink_escape() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");
    let outside_dir = tempdir().expect("create outside temp dir");
    symlink(outside_dir.path(), work_dir.path().join("link")).expect("create symlink in workdir");
    let outside_file = outside_dir.path().join("target.txt");
    std::fs::write(&outside_file, "old value\n").expect("seed outside file");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Update File: link/target.txt\n@@\n-old value\n+new value\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("\"isError\":true"),
        "expected update through symlink escape to be rejected: {stdout}"
    );

    let outside_contents = std::fs::read_to_string(&outside_file).expect("read outside file");
    assert_eq!(
        outside_contents, "old value\n",
        "outside target should remain unchanged when restricted update is blocked"
    );
}

#[cfg(unix)]
#[test]
fn restrict_to_workdir_blocks_move_destination_through_symlink_escape() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");
    let outside_dir = tempdir().expect("create outside temp dir");
    let source = work_dir.path().join("source.txt");
    std::fs::write(&source, "hello\n").expect("seed source file");
    symlink(outside_dir.path(), work_dir.path().join("link")).expect("create symlink in workdir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Update File: source.txt\n*** Move to: link/moved.txt\n@@\n-hello\n+hello moved\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("\"isError\":true"),
        "expected move destination through symlink to be rejected: {stdout}"
    );

    let source_contents = std::fs::read_to_string(&source).expect("read source file");
    assert_eq!(
        source_contents, "hello\n",
        "source file should remain unchanged when restricted move is blocked"
    );
    assert!(
        !outside_dir.path().join("moved.txt").exists(),
        "move destination should not be created outside --workdir"
    );
}

#[cfg(unix)]
#[test]
fn restrict_to_workdir_allows_delete_of_nonbroken_symlink() {
    let launch_dir = tempdir().expect("create launch temp dir");
    let work_dir = tempdir().expect("create work temp dir");
    let outside_dir = tempdir().expect("create outside temp dir");
    let marker = outside_dir.path().join("marker.txt");
    std::fs::write(&marker, "outside").expect("create outside marker");

    let link = work_dir.path().join("link");
    symlink(outside_dir.path(), &link).expect("create symlink in workdir");
    assert!(
        std::fs::symlink_metadata(&link).is_ok(),
        "symlink should exist as a directory entry"
    );

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.arg("--workdir").arg(work_dir.path());
    cmd.arg("--restrict-to-workdir");
    cmd.current_dir(launch_dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_patch","arguments":{"input":"*** Begin Patch\n*** Delete File: link\n*** End Patch\n"}}}
"#;
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn server");

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }

    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "process exited with failure.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        !stdout.contains("\"isError\":true"),
        "expected delete of in-tree symlink entry to be allowed: {stdout}"
    );
    assert!(
        std::fs::symlink_metadata(&link).is_err(),
        "symlink entry should be deleted"
    );
    assert!(
        marker.exists(),
        "outside target should remain untouched when deleting symlink entry"
    );
}
