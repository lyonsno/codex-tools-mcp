use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
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
fn initialize_uses_requested_newer_protocol_version() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let initialize_response = find_response_by_id(&messages, 1);
    let negotiated_version = initialize_response
        .get("result")
        .and_then(|result| result.get("protocolVersion"))
        .and_then(Value::as_str)
        .expect("initialize result should include protocolVersion");
    assert_eq!(
        negotiated_version, "2025-06-18",
        "server should negotiate the newer protocol when requested"
    );
}

#[test]
fn initialize_uses_requested_legacy_zed_protocol_version() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let initialize_response = find_response_by_id(&messages, 1);
    let negotiated_version = initialize_response
        .get("result")
        .and_then(|result| result.get("protocolVersion"))
        .and_then(Value::as_str)
        .expect("initialize result should include protocolVersion");
    assert_eq!(
        negotiated_version, "2025-03-26",
        "server should negotiate Zed legacy protocol when requested"
    );
}

#[test]
fn initialize_rejects_unsupported_protocol_version() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let initialize_response = find_response_by_id(&messages, 1);
    assert!(
        initialize_response.get("result").is_none(),
        "unsupported protocolVersion should not return initialize success result: {initialize_response:?}"
    );
    let error = initialize_response
        .get("error")
        .and_then(Value::as_object)
        .expect("unsupported protocolVersion should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "unsupported protocolVersion should return INVALID_PARAMS"
    );
}

#[test]
fn initialize_rejects_missing_protocol_version() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let initialize_response = find_response_by_id(&messages, 1);
    assert!(
        initialize_response.get("result").is_none(),
        "missing protocolVersion should not return initialize success result: {initialize_response:?}"
    );
    let error = initialize_response
        .get("error")
        .and_then(Value::as_object)
        .expect("missing protocolVersion should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "missing protocolVersion should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("protocolVersion string")),
        "missing protocolVersion should produce a clear validation message: {error:?}"
    );
}

#[test]
fn initialize_rejects_non_string_protocol_version() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":20250618,"clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let initialize_response = find_response_by_id(&messages, 1);
    assert!(
        initialize_response.get("result").is_none(),
        "non-string protocolVersion should not return initialize success result: {initialize_response:?}"
    );
    let error = initialize_response
        .get("error")
        .and_then(Value::as_object)
        .expect("non-string protocolVersion should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "non-string protocolVersion should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("protocolVersion string")),
        "non-string protocolVersion should produce a clear validation message: {error:?}"
    );
}

fn parse_jsonl_stdout(stdout: &[u8]) -> Vec<Value> {
    let text = String::from_utf8(stdout.to_vec()).expect("utf8 stdout");
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("valid json line from server"))
        .collect()
}

fn find_response_by_id(messages: &[Value], id: i64) -> &Value {
    messages
        .iter()
        .find(|message| message.get("id").and_then(Value::as_i64) == Some(id))
        .unwrap_or_else(|| panic!("response with id {id} not found: {messages:?}"))
}

#[test]
fn tools_list_advertises_ask_user_schema_without_allow_free_form() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let tools_list_response = find_response_by_id(&messages, 2);
    let tools = tools_list_response
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .expect("tools/list result should include tools array");

    let ask_user = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("ask_user"))
        .expect("ask_user should be advertised in tools/list");

    let input_schema = ask_user
        .get("inputSchema")
        .expect("ask_user should include inputSchema");
    assert_eq!(
        input_schema.get("type").and_then(Value::as_str),
        Some("object"),
        "ask_user input schema type should be object"
    );
    assert_eq!(
        input_schema
            .get("additionalProperties")
            .and_then(Value::as_bool),
        Some(false),
        "ask_user input schema should reject additional properties"
    );
    let schema_properties = input_schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("ask_user should include inputSchema.properties");
    let required_fields = input_schema
        .get("required")
        .and_then(Value::as_array)
        .expect("ask_user should include inputSchema.required");

    assert!(
        schema_properties.contains_key("question"),
        "ask_user schema should require question"
    );
    let question_schema = schema_properties
        .get("question")
        .and_then(Value::as_object)
        .expect("ask_user schema should define question object");
    assert_eq!(
        question_schema.get("type").and_then(Value::as_str),
        Some("string"),
        "ask_user question type should be string"
    );
    assert_eq!(
        question_schema.get("minLength").and_then(Value::as_i64),
        Some(1),
        "ask_user question should set minLength=1 so empty strings fail schema validation"
    );
    assert_eq!(
        question_schema.get("pattern").and_then(Value::as_str),
        Some(".*\\S.*"),
        "ask_user question should require at least one non-whitespace character"
    );
    assert!(
        required_fields
            .iter()
            .any(|field| field.as_str() == Some("question")),
        "ask_user schema should list question as required"
    );
    assert!(
        schema_properties.contains_key("choices"),
        "ask_user schema should include optional choices"
    );
    let choices_schema = schema_properties
        .get("choices")
        .and_then(Value::as_object)
        .expect("ask_user schema should define choices object");
    assert_eq!(
        choices_schema.get("type").and_then(Value::as_str),
        Some("array"),
        "ask_user choices type should be array"
    );
    assert_eq!(
        choices_schema
            .get("items")
            .and_then(Value::as_object)
            .and_then(|items| items.get("type"))
            .and_then(Value::as_str),
        Some("string"),
        "ask_user choices items should be strings"
    );
    assert!(
        !required_fields
            .iter()
            .any(|field| field.as_str() == Some("choices")),
        "ask_user choices should stay optional"
    );
    assert!(
        schema_properties.contains_key("timeout_seconds"),
        "ask_user schema should include optional timeout_seconds"
    );
    let timeout_schema = schema_properties
        .get("timeout_seconds")
        .and_then(Value::as_object)
        .expect("ask_user schema should define timeout_seconds object");
    assert_eq!(
        timeout_schema.get("type").and_then(Value::as_str),
        Some("integer"),
        "ask_user timeout_seconds type should be integer"
    );
    assert_eq!(
        timeout_schema.get("minimum").and_then(Value::as_i64),
        Some(1),
        "ask_user timeout_seconds should require minimum of 1"
    );
    assert!(
        !required_fields
            .iter()
            .any(|field| field.as_str() == Some("timeout_seconds")),
        "ask_user timeout_seconds should stay optional"
    );
    assert!(
        !schema_properties.contains_key("allow_free_form"),
        "ask_user schema should not expose allow_free_form because free-form is always enabled"
    );
}

#[test]
fn ask_user_call_without_popup_returns_structured_unsupported_result() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.env("CODEX_TOOLS_MCP_ASK_USER_BACKEND", "none");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Which branch should I use?","choices":["main","dev"]}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_some(),
        "ask_user should return a tool result object, not a JSON-RPC method error: {ask_user_response:?}"
    );
    assert!(
        ask_user_response.get("error").is_none(),
        "ask_user should not be treated as unknown tool when popup backend is unavailable: {ask_user_response:?}"
    );

    let result_obj = ask_user_response
        .get("result")
        .and_then(Value::as_object)
        .expect("ask_user result should be an object");
    assert_eq!(
        result_obj.get("isError").and_then(Value::as_bool),
        Some(true),
        "ask_user unsupported fallback should mark isError=true"
    );

    let classification = result_obj
        .get("error")
        .and_then(Value::as_object)
        .expect("ask_user result should include structured error classification");
    assert_eq!(
        classification.get("kind").and_then(Value::as_str),
        Some("unsupported"),
        "ask_user unsupported fallback should classify error kind as unsupported"
    );
    assert_eq!(
        classification.get("tool").and_then(Value::as_str),
        Some("ask_user"),
        "ask_user unsupported fallback should identify the tool"
    );
    assert_eq!(
        classification.get("backend").and_then(Value::as_str),
        Some("none"),
        "ask_user unsupported fallback should report the forced backend"
    );
    assert_eq!(
        classification.get("retryable").and_then(Value::as_bool),
        Some(false),
        "ask_user unsupported fallback should mark retryable=false"
    );

    let text_blocks = result_obj
        .get("content")
        .and_then(Value::as_array)
        .expect("ask_user result should include content array");
    assert!(
        text_blocks
            .iter()
            .all(|block| block.get("type").and_then(Value::as_str) == Some("text")),
        "ask_user fallback content blocks should all be text"
    );
    let combined_text = text_blocks
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        combined_text.contains("unsupported"),
        "ask_user fallback should clearly report unsupported status when popup is unavailable: {combined_text}"
    );
}

#[test]
fn ask_user_call_with_non_string_choice_returns_invalid_params_error() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Pick one","choices":["main",42]}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_none(),
        "invalid ask_user request should not return tool result: {ask_user_response:?}"
    );
    let error = ask_user_response
        .get("error")
        .and_then(Value::as_object)
        .expect("invalid ask_user request should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "invalid ask_user request should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("choices must be an array of strings")),
        "invalid ask_user choices should produce a clear validation message: {error:?}"
    );
}

#[test]
fn ask_user_call_with_unknown_argument_returns_invalid_params_error() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Pick one","allow_free_form":true}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_none(),
        "ask_user should reject unknown arguments with INVALID_PARAMS: {ask_user_response:?}"
    );
    let error = ask_user_response
        .get("error")
        .and_then(Value::as_object)
        .expect("invalid ask_user request should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "unknown ask_user arguments should return INVALID_PARAMS"
    );
}

#[test]
fn ask_user_call_without_question_returns_invalid_params_error() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"choices":["main","dev"]}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_none(),
        "missing ask_user question should return INVALID_PARAMS: {ask_user_response:?}"
    );
    let error = ask_user_response
        .get("error")
        .and_then(Value::as_object)
        .expect("invalid ask_user request should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "missing ask_user question should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("question must be provided as a string")),
        "missing ask_user question should produce clear validation message: {error:?}"
    );
}

#[test]
fn ask_user_call_with_blank_question_returns_invalid_params_error() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"   ","choices":["main","dev"]}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_none(),
        "blank ask_user question should return INVALID_PARAMS: {ask_user_response:?}"
    );
    let error = ask_user_response
        .get("error")
        .and_then(Value::as_object)
        .expect("invalid ask_user request should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "blank ask_user question should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("question must be a non-empty string")),
        "blank ask_user question should produce clear validation message: {error:?}"
    );
}

#[test]
fn ask_user_call_with_non_array_choices_returns_invalid_params_error() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Pick one","choices":"main"}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_none(),
        "non-array ask_user choices should return INVALID_PARAMS: {ask_user_response:?}"
    );
    let error = ask_user_response
        .get("error")
        .and_then(Value::as_object)
        .expect("invalid ask_user request should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "non-array ask_user choices should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("choices must be an array of strings")),
        "non-array ask_user choices should produce clear validation message: {error:?}"
    );
}

#[test]
fn ask_user_call_with_non_positive_timeout_returns_invalid_params_error() {
    let dir = tempdir().expect("create temp dir");

    let mut cmd = Command::cargo_bin("codex-tools-mcp").expect("binary exists");
    cmd.arg("--log-level").arg("error");
    cmd.current_dir(dir.path());

    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Pick one","timeout_seconds":0}}}
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

    let messages = parse_jsonl_stdout(&output.stdout);
    let ask_user_response = find_response_by_id(&messages, 2);

    assert!(
        ask_user_response.get("result").is_none(),
        "non-positive ask_user timeout should return INVALID_PARAMS: {ask_user_response:?}"
    );
    let error = ask_user_response
        .get("error")
        .and_then(Value::as_object)
        .expect("invalid ask_user request should return JSON-RPC error object");
    assert_eq!(
        error.get("code").and_then(Value::as_i64),
        Some(-32602),
        "non-positive ask_user timeout should return INVALID_PARAMS"
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("timeout_seconds must be a positive integer")),
        "non-positive ask_user timeout should produce clear validation message: {error:?}"
    );
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
