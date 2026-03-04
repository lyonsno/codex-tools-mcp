use codex_apply_patch::apply_patch as run_apply_patch;
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Component, Path, PathBuf};

use crate::tools::{
    apply_patch_tool_schema, ask_user_tool_schema, update_plan_tool_schema, INVALID_PARAMS,
    INVALID_REQUEST, JSONRPC_VERSION, MCP_PROTOCOL_VERSION, METHOD_NOT_FOUND, PARSE_ERROR,
};

pub struct ServerConfig {
    pub restrict_root: Option<PathBuf>,
}

pub fn run_server(config: ServerConfig) -> io::Result<()> {
    let stdin = io::stdin();
    for line_result in stdin.lock().lines() {
        let line = match line_result {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                debug!("stdin: {line}");
                line
            }
            Err(err) => {
                error!("failed to read stdin: {err}");
                break;
            }
        };

        let message: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                warn!("Malformed JSON from client: {err}");
                if let Err(send_err) =
                    send_error(None, PARSE_ERROR, format!("Malformed JSON: {err}"))
                {
                    error!("failed to send parse error response: {send_err}");
                }
                continue;
            }
        };

        if !message.is_object() {
            warn!("Received non-object message");
            if let Err(err) = send_error(None, INVALID_REQUEST, "Request must be a JSON object") {
                error!("failed to send invalid request error: {err}");
            }
            continue;
        }

        if let Err(err) = handle_message(message, &config) {
            error!("internal error while processing message: {err}");
        }
    }

    Ok(())
}

fn handle_message(message: Value, config: &ServerConfig) -> io::Result<()> {
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing method"))?;
    let request_id = message.get("id").cloned();
    let params = message.get("params").cloned();

    debug!("dispatching method: {method}");

    match method {
        "initialize" => handle_initialize(request_id, params),
        "tools/list" => handle_tools_list(request_id),
        "tools/call" => handle_tools_call(request_id, params, config),
        "ping" => handle_ping(request_id),
        _ => send_error(
            request_id,
            METHOD_NOT_FOUND,
            format!("Unknown method: {method}"),
        ),
    }
}

fn handle_initialize(request_id: Option<Value>, params: Option<Value>) -> io::Result<()> {
    if request_id.is_none() {
        return send_error(None, INVALID_REQUEST, "initialize must include an id");
    }

    if !params.as_ref().is_some_and(Value::is_object) {
        return send_error(
            request_id,
            INVALID_PARAMS,
            "initialize params must be object",
        );
    }

    let result = json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "serverInfo": {
            "name": "codex-tools-mcp",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": {
            "tools": json!({}),
        }
    });
    send_result(request_id.clone(), result)?;

    debug!("sent initialize response");

    send_json(json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": "notifications/initialized",
        "params": Value::Null,
    }))
}

fn handle_tools_list(request_id: Option<Value>) -> io::Result<()> {
    if request_id.is_none() {
        return send_error(None, INVALID_REQUEST, "tools/list must include an id");
    }

    let tools = vec![
        update_plan_tool_schema(),
        ask_user_tool_schema(),
        apply_patch_tool_schema(),
    ];
    debug!("advertising {} tools", tools.len());
    let result = json!({ "tools": tools });
    send_result(request_id, result)
}

fn handle_tools_call(
    request_id: Option<Value>,
    params: Option<Value>,
    config: &ServerConfig,
) -> io::Result<()> {
    if request_id.is_none() {
        return send_error(None, INVALID_REQUEST, "tools/call must include an id");
    }

    let params_obj = match params {
        Some(Value::Object(map)) => map,
        _ => {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "tools/call params must be object",
            )
        }
    };

    match params_obj.get("name").and_then(Value::as_str) {
        Some("update_plan") => {
            info!("received update_plan call");
            let result = json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Plan updated",
                    }
                ]
            });
            send_result(request_id, result)
        }
        Some("ask_user") => handle_ask_user_tool(request_id, &params_obj),
        Some("apply_patch") => handle_apply_patch_tool(request_id, &params_obj, config),
        Some(other) => {
            warn!("unknown tool requested: {other}");
            send_error(
                request_id,
                METHOD_NOT_FOUND,
                format!("Unknown tool: {other}"),
            )
        }
        None => send_error(request_id, INVALID_PARAMS, "tools/call params missing name"),
    }
}

fn handle_ask_user_tool(
    request_id: Option<Value>,
    params_obj: &serde_json::Map<String, Value>,
) -> io::Result<()> {
    let arguments = match params_obj.get("arguments") {
        Some(Value::Object(arguments)) => arguments,
        Some(_) => {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "ask_user arguments must be an object",
            )
        }
        None => return send_error(request_id, INVALID_PARAMS, "ask_user requires arguments"),
    };

    const ALLOWED_ARGUMENT_KEYS: [&str; 3] = ["question", "choices", "timeout_seconds"];
    if let Some(unknown_key) = arguments
        .keys()
        .find(|key| !ALLOWED_ARGUMENT_KEYS.contains(&key.as_str()))
    {
        return send_error(
            request_id,
            INVALID_PARAMS,
            format!("ask_user received unknown argument: {unknown_key}"),
        );
    }

    let question = match arguments.get("question").and_then(Value::as_str) {
        Some(question) if !question.trim().is_empty() => question,
        Some(_) => {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "ask_user question must be a non-empty string",
            )
        }
        None => {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "ask_user question must be provided as a string",
            )
        }
    };

    if let Some(choices) = arguments.get("choices") {
        let choices = match choices.as_array() {
            Some(choices) => choices,
            None => {
                return send_error(
                    request_id,
                    INVALID_PARAMS,
                    "ask_user choices must be an array of strings",
                )
            }
        };

        let all_strings = choices.iter().all(|choice| choice.as_str().is_some());
        if !all_strings {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "ask_user choices must be an array of strings",
            );
        }
    }

    if let Some(timeout_seconds) = arguments.get("timeout_seconds") {
        let is_positive_integer = timeout_seconds.as_i64().is_some_and(|value| value > 0);
        if !is_positive_integer {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "ask_user timeout_seconds must be a positive integer",
            );
        }
    }

    let requested_backend = std::env::var("CODEX_TOOLS_MCP_ASK_USER_BACKEND")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "none".to_string());

    info!(
        "ask_user requested but interactive backend is unsupported (backend={})",
        requested_backend
    );

    let result = json!({
        "content": [
            {
                "type": "text",
                "text": format!(
                    "ask_user unsupported: interactive popup backend is unavailable in this build. question={}",
                    question
                ),
            }
        ],
        "isError": true,
        "error": {
            "kind": "unsupported",
            "tool": "ask_user",
            "backend": requested_backend,
            "retryable": false,
        }
    });
    send_result(request_id, result)
}

fn handle_apply_patch_tool(
    request_id: Option<Value>,
    params_obj: &serde_json::Map<String, Value>,
    config: &ServerConfig,
) -> io::Result<()> {
    let arguments = match params_obj.get("arguments") {
        Some(Value::Object(arguments)) => arguments,
        Some(_) => {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "apply_patch arguments must be an object",
            )
        }
        None => return send_error(request_id, INVALID_PARAMS, "apply_patch requires arguments"),
    };

    let patch = match arguments.get("input").and_then(Value::as_str) {
        Some(patch) => patch.to_string(),
        None => {
            return send_error(
                request_id,
                INVALID_PARAMS,
                "apply_patch input must be provided as a string",
            )
        }
    };

    if let Some(root) = &config.restrict_root {
        if let Err(err) = validate_patch_paths_within_root(&patch, root) {
            warn!("apply_patch blocked by --restrict-to-workdir: {err}");
            return send_apply_patch_error(request_id, err);
        }
    }

    info!("running apply_patch ({} bytes)", patch.len());

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    let apply_result = run_apply_patch(&patch, &mut stdout_buf, &mut stderr_buf);
    let stdout_text = String::from_utf8_lossy(&stdout_buf).to_string();
    let stderr_text = String::from_utf8_lossy(&stderr_buf).to_string();

    match apply_result {
        Ok(()) => {
            let mut blocks = Vec::new();
            if !stdout_text.trim().is_empty() {
                blocks.push(json!({
                    "type": "text",
                    "text": stdout_text.trim_end_matches('\n'),
                }));
            }
            if !stderr_text.trim().is_empty() {
                blocks.push(json!({
                    "type": "text",
                    "text": format!("stderr:\n{}", stderr_text.trim_end_matches('\n')),
                }));
            }
            if blocks.is_empty() {
                blocks.push(json!({
                    "type": "text",
                    "text": "Patch applied",
                }));
            }

            info!("apply_patch completed successfully");
            let result = json!({ "content": blocks });
            send_result(request_id, result)
        }
        Err(err) => {
            warn!("apply_patch failed: {err}");
            let mut blocks = Vec::new();
            let error_message = err.to_string();
            blocks.push(json!({
                "type": "text",
                "text": format!("apply_patch failed: {error_message}"),
            }));

            if !stdout_text.trim().is_empty() {
                blocks.push(json!({
                    "type": "text",
                    "text": stdout_text.trim_end_matches('\n'),
                }));
            }
            if !stderr_text.trim().is_empty() {
                blocks.push(json!({
                    "type": "text",
                    "text": format!("stderr:\n{}", stderr_text.trim_end_matches('\n')),
                }));
            }

            let result = json!({
                "content": blocks,
                "isError": true,
            });
            send_result(request_id, result)
        }
    }
}

fn validate_patch_paths_within_root(patch: &str, root: &Path) -> Result<(), String> {
    const FILE_PREFIXES: [(&str, PatchPathKind); 4] = [
        ("*** Add File: ", PatchPathKind::WriteLike),
        ("*** Update File: ", PatchPathKind::WriteLike),
        ("*** Delete File: ", PatchPathKind::Delete),
        ("*** Move to: ", PatchPathKind::WriteLike),
    ];

    for (idx, line) in patch.lines().enumerate() {
        for (prefix, kind) in FILE_PREFIXES {
            if let Some(path_text) = line.strip_prefix(prefix) {
                validate_patch_path(path_text, root, kind)
                    .map_err(|err| format!("{err} (line {})", idx + 1))?;
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum PatchPathKind {
    WriteLike,
    Delete,
}

fn validate_patch_path(path_text: &str, root: &Path, kind: PatchPathKind) -> Result<(), String> {
    if path_text.is_empty() {
        return Err("Patch path cannot be empty".to_string());
    }

    let path = Path::new(path_text);
    if path.is_absolute() {
        return Err(format!("Absolute patch paths are not allowed: {path_text}"));
    }

    let normalized = normalize_relative_path(path)
        .ok_or_else(|| format!("Patch path escapes workdir: {path_text}"))?;
    if normalized.as_os_str().is_empty() {
        return Err(format!(
            "Patch path cannot resolve to workdir root: {path_text}"
        ));
    }

    let allow_terminal_symlink_delete = matches!(kind, PatchPathKind::Delete);
    ensure_path_stays_within_root(root, &normalized, allow_terminal_symlink_delete)
        .map_err(|err| format!("{err}: {path_text}"))?;

    Ok(())
}

fn normalize_relative_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(seg) => normalized.push(seg),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(normalized)
}

fn ensure_path_stays_within_root(
    root: &Path,
    relative: &Path,
    allow_terminal_symlink_delete: bool,
) -> Result<(), String> {
    let mut cursor = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        let segment = match component {
            Component::Normal(seg) => seg,
            _ => return Err("Patch path contains unsupported component".to_string()),
        };

        cursor.push(segment);

        match fs::symlink_metadata(&cursor) {
            Ok(metadata) => match fs::canonicalize(&cursor) {
                Ok(canonical) => {
                    if allow_terminal_symlink_delete && is_last && metadata.file_type().is_symlink()
                    {
                        // Deleting the symlink entry itself is safe, regardless of where it points.
                        continue;
                    }
                    if !canonical.starts_with(root) {
                        return Err("Patch path escapes workdir via symlink".to_string());
                    }
                    cursor = canonical;
                }
                Err(err)
                    if allow_terminal_symlink_delete
                        && is_last
                        && err.kind() == io::ErrorKind::NotFound
                        && metadata.file_type().is_symlink() =>
                {
                    // Allow deleting a broken symlink that is inside workdir.
                }
                Err(err) => return Err(format!("Failed to resolve patch path: {err}")),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Remaining components do not exist yet, so lexical relative joining is enough.
            }
            Err(err) => return Err(format!("Failed to inspect patch path: {err}")),
        }
    }

    if !cursor.starts_with(root) {
        return Err("Patch path escapes workdir".to_string());
    }

    Ok(())
}

fn send_apply_patch_error(request_id: Option<Value>, message: impl Into<String>) -> io::Result<()> {
    let result = json!({
        "content": [
            {
                "type": "text",
                "text": format!("apply_patch failed: {}", message.into()),
            }
        ],
        "isError": true,
    });
    send_result(request_id, result)
}

fn handle_ping(request_id: Option<Value>) -> io::Result<()> {
    if request_id.is_none() {
        return send_error(None, INVALID_REQUEST, "ping must include an id");
    }

    send_result(request_id, json!({}))
}

fn send_result(request_id: Option<Value>, result: Value) -> io::Result<()> {
    let mut message = serde_json::Map::new();
    message.insert(
        "jsonrpc".to_string(),
        Value::String(JSONRPC_VERSION.to_string()),
    );
    message.insert("result".to_string(), result);

    match request_id {
        Some(id) => {
            message.insert("id".to_string(), id);
        }
        None => {
            message.insert("id".to_string(), Value::Null);
        }
    }

    send_json(Value::Object(message))
}

fn send_error(request_id: Option<Value>, code: i64, message: impl Into<String>) -> io::Result<()> {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "jsonrpc".to_string(),
        Value::String(JSONRPC_VERSION.to_string()),
    );
    payload.insert(
        "error".to_string(),
        json!({
            "code": code,
            "message": message.into(),
        }),
    );

    match request_id {
        Some(id) => {
            payload.insert("id".to_string(), id);
        }
        None => {
            payload.insert("id".to_string(), Value::Null);
        }
    }

    send_json(Value::Object(payload))
}

fn send_json(value: Value) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &value)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}
