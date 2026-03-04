use serde_json::Value;

pub const JSONRPC_VERSION: &str = "2.0";
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
pub const MCP_PROTOCOL_VERSION_ZED: &str = "2025-03-26";
pub const SUPPORTED_MCP_PROTOCOL_VERSIONS: [&str; 2] =
    [MCP_PROTOCOL_VERSION, MCP_PROTOCOL_VERSION_ZED];
pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;

pub fn update_plan_tool_schema() -> Value {
    serde_json::json!({
        "name": "update_plan",
        "description": "Updates the task plan. Provide an optional explanation and a list of plan items, each with a step and status. At most one step can be in_progress at a time.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "explanation": { "type": "string" },
                "plan": {
                    "type": "array",
                    "description": "The list of steps",
                    "items": {
                        "type": "object",
                        "properties": {
                            "step": { "type": "string" },
                            "status": {
                                "type": "string",
                                "description": "One of: pending, in_progress, completed",
                            },
                        },
                        "required": ["step", "status"],
                        "additionalProperties": false,
                    }
                }
            },
            "required": ["plan"],
            "additionalProperties": false,
        }
    })
}

pub fn ask_user_tool_schema() -> Value {
    serde_json::json!({
        "name": "ask_user",
        "description": "Ask the user a question and wait for an answer. This server currently supports an unsupported fallback response when no interactive backend is available.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "Prompt shown to the user",
                    "minLength": 1,
                    "pattern": ".*\\S.*",
                },
                "choices": {
                    "type": "array",
                    "description": "Optional list of suggested choices",
                    "items": { "type": "string" },
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Optional timeout in seconds",
                    "minimum": 1,
                }
            },
            "required": ["question"],
            "additionalProperties": false,
        }
    })
}

pub fn apply_patch_tool_schema() -> Value {
    serde_json::json!({
        "name": "apply_patch",
        "description": "Use the `apply_patch` tool to edit files.\nYour patch language is a stripped-down, file-oriented diff format designed to be easy to parse and safe to apply. You can think of it as a high-level envelope:\n\n*** Begin Patch\n[ one or more file sections ]\n*** End Patch\n\nWithin that envelope, you get a sequence of file operations.\nYou MUST include a header to specify the action you are taking.\nEach operation starts with one of three headers:\n\n*** Add File: <path> - create a new file. Every following line is a + line (the initial contents).\n*** Delete File: <path> - remove an existing file. Nothing follows.\n*** Update File: <path> - patch an existing file in place (optionally with a rename).\n\nMay be immediately followed by *** Move to: <new path> if you want to rename the file.\nThen one or more “hunks”, each introduced by @@ (optionally followed by a hunk header).\nWithin a hunk each line starts with:\n\nFor instructions on [context_before] and [context_after]:\n- By default, show 3 lines of code immediately above and 3 lines immediately below each change. If a change is within 3 lines of a previous change, do NOT duplicate the first change’s [context_after] lines in the second change’s [context_before] lines.\n- If 3 lines of context is insufficient to uniquely identify the snippet of code within the file, use the @@ operator to indicate the class or function to which the snippet belongs. For instance, we might have:\n@@ class BaseClass\n[3 lines of pre-context]\n- [old_code]\n+ [new_code]\n[3 lines of post-context]\n\n- If a code block is repeated so many times in a class or function such that even a single `@@` statement and 3 lines of context cannot uniquely identify the snippet of code, you can use multiple `@@` statements to jump to the right context. For instance:\n\n@@ class BaseClass\n@@ \t def method():\n[3 lines of pre-context]\n- [old_code]\n+ [new_code]\n[3 lines of post-context]\n\nThe full grammar definition is below:\nPatch := Begin { FileOp } End\nBegin := \"*** Begin Patch\" NEWLINE\nEnd := \"*** End Patch\" NEWLINE\nFileOp := AddFile | DeleteFile | UpdateFile\nAddFile := \"*** Add File: \" path NEWLINE { \"+\" line NEWLINE }\nDeleteFile := \"*** Delete File: \" path NEWLINE\nUpdateFile := \"*** Update File: \" path NEWLINE [ MoveTo ] { Hunk }\nMoveTo := \"*** Move to: \" newPath NEWLINE\nHunk := \"@@\" [ header ] NEWLINE { HunkLine } [ \"*** End of File\" NEWLINE ]\nHunkLine := (\" \" | \"-\" | \"+\") text NEWLINE\n\nA full patch can combine several operations:\n\n*** Begin Patch\n*** Add File: hello.txt\n+Hello world\n*** Update File: src/app.py\n*** Move to: src/main.py\n@@ def greet():\n-print(\"Hi\")\n+print(\"Hello, world!\")\n*** Delete File: obsolete.txt\n*** End Patch\n\nIt is important to remember:\n\n- You must include a header with your intended action (Add/Delete/Update)\n- You must prefix new lines with `+` even when creating a new file\n- File references can only be relative, NEVER ABSOLUTE.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "The entire contents of the apply_patch command",
                }
            },
            "required": ["input"],
            "additionalProperties": false,
        }
    })
}
