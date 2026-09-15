//! MCP server (stdio, JSON-RPC 2.0) shared by the native binary and the npm
//! package. Each tool runs the matching command in-process with
//! `--json --no-prompt` through a capturing host, so results match the CLI
//! exactly. Hosts feed one message per line to [`handle`] and write back the
//! returned line, if any.

use crate::host::{Host, PromptKind};
use crate::storage::{IoError, Storage};
use serde_json::{json, Map, Value};

pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "tix is a ticket board stored as files: tix.yaml (schema) plus \
tickets/<ID>/ticket.md. Every tool accepts an optional `workspace` (absolute path of the board \
folder or any folder inside it); without it the server's default folder is used. Start with \
tix_help (no command) for the rules, JSON shapes and exit codes, and call tix_help with a command \
name before its first use. Call tix_help with command \"new\" inside a board to learn its fields \
and allowed values before tix_new. Results are the command's JSON output; failures come back as \
errors starting with `exit N` whose text names the broken rule (e.g. rule 3: required field \
'client' is missing or empty). Nothing is written when a write fails. Use full ids from results; \
prefixes of 4+ characters also work.";

pub const HELP: &str = "\
Run a local MCP server over stdio exposing tix commands as tools.

Usage: tix mcp [--workspace DIR]

  --workspace DIR  default board folder for tools that omit `workspace`
                   [default: the current directory]

Tools: tix_help, tix_init, tix_check, tix_new, tix_ls, tix_show, tix_mv, tix_set,
       tix_attach, tix_detach, tix_board, tix_path. Each runs the matching command
       with --json --no-prompt and returns its output, or an error naming the rule.

Setup:
  Claude Code:   claude mcp add tix -- tix mcp
                 claude mcp add tix -- npx -y @viewengine/tix mcp     (Node 18+)
  Other clients: {\"mcpServers\": {\"tix\": {\"command\": \"tix\", \"args\": [\"mcp\", \"--workspace\", \"/path/to/board\"]}}}
";

/// What `tix mcp <args>` asks for.
pub enum McpArgs {
    Serve { workspace: String },
    Help,
    Error(String),
}

/// Parses the arguments after `mcp`. `cwd` is the default workspace.
pub fn parse_args(args: &[String], cwd: &str) -> McpArgs {
    let mut workspace = cwd.to_string();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return McpArgs::Help,
            "--workspace" => match it.next() {
                Some(d) => workspace = d.replace('\\', "/"),
                None => return McpArgs::Error("--workspace needs a directory".into()),
            },
            other => return McpArgs::Error(format!("unexpected argument '{other}'\n\n{HELP}")),
        }
    }
    McpArgs::Serve { workspace }
}

/// Wraps a host: storage, clock and randomness pass through; output is
/// captured and prompts answer nothing (stdout belongs to the protocol).
struct Capture<'a, H: Host> {
    inner: &'a mut H,
    out: String,
    err: String,
}

impl<H: Host> Storage for Capture<'_, H> {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        self.inner.read(path)
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        self.inner.write(path, bytes)
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        self.inner.list_dir(path)
    }
    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        self.inner.mkdir_all(path)
    }
    fn exists(&self, path: &str) -> Result<bool, IoError> {
        self.inner.exists(path)
    }
}

impl<H: Host> Host for Capture<'_, H> {
    fn prompt(&mut self, _: &str, _: PromptKind, _: &[String]) -> Option<String> {
        None
    }
    fn stdout(&mut self, text: &str) {
        self.out.push_str(text);
    }
    fn stderr(&mut self, text: &str) {
        self.err.push_str(text);
    }
    fn now_unix(&mut self) -> u64 {
        self.inner.now_unix()
    }
    fn random_bytes(&mut self, n: usize) -> Vec<u8> {
        self.inner.random_bytes(n)
    }
}

fn workspace_prop() -> Value {
    json!({"type": "string", "description": "Absolute path of the board folder, or any folder inside it. Defaults to the server's workspace."})
}

fn id_prop() -> Value {
    json!({"type": "string", "description": "Ticket id or unique prefix of 4+ characters."})
}

fn field_value_schema() -> Value {
    json!({"anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}]})
}

fn schema(props: Value, required: &[&str]) -> Value {
    let mut p = props.as_object().cloned().unwrap_or_default();
    p.insert("workspace".into(), workspace_prop());
    json!({"type": "object", "properties": p, "required": required})
}

fn tool(name: &str, description: &str, input: Value, read_only: bool) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input,
        "annotations": {"readOnlyHint": read_only, "destructiveHint": name == "tix_detach"}
    })
}

/// The tool list (`tools/list`).
pub fn tools() -> Vec<Value> {
    vec![
        tool("tix_help", "Documentation for tix or one command: argument formats, output and JSON shapes, exit codes, examples. Call with no command first, then per command before first use; for \"new\" pass workspace to see that board's fields.",
            schema(json!({"command": {"type": "string", "description": "init, check, new, ls, show, mv, set, attach, detach, board or path. Omit for the overview."}}), &[]), true),
        tool("tix_init", "Create tix.yaml (default schema) and tickets/ in the workspace folder. Fails if a board already exists there or in a parent. Only run when the user asked for a new board.",
            schema(json!({}), &[]), false),
        tool("tix_check", "Validate tix.yaml and every ticket. Returns {\"ok\": bool, \"problems\": [{path, message}]}; problems are reported as an error result that includes that JSON. Only the first broken rule per ticket is listed, so re-run after fixing.",
            schema(json!({}), &[]), true),
        tool("tix_new", "Create a ticket. Pass schema fields in `fields` (learn them with tix_help command \"new\"). Returns the new ticket object including its id. Nothing is written if a required field is missing or a value is not allowed.",
            schema(json!({
                "title": {"type": "string", "description": "Ticket title."},
                "status": {"type": "string", "description": "Initial status; defaults to the first backlog status."},
                "fields": {"type": "object", "additionalProperties": field_value_schema(), "description": "Schema fields by name, e.g. {\"client\": \"acme\", \"type\": \"article\", \"tags\": [\"a\", \"b\"]}."}
            }), &["title"]), false),
        tool("tix_ls", "List tickets (optionally filtered) in board order: status order, then oldest first. Returns an array of ticket objects with \"valid\" and \"problem\".",
            schema(json!({"filters": {"type": "array", "items": {"type": "string"}, "description": "KEY:VALUE filters, all must match: \"status:backlog\", \"group:in_progress\", \"client:acme\"."}}), &[]), true),
        tool("tix_show", "Get one ticket: all fields, deliverables, \"body\" (the Markdown brief), \"valid\" and \"problem\".",
            schema(json!({"id": id_prop()}), &["id"]), true),
        tool("tix_mv", "Change a ticket's status (any status to any other). Returns the updated ticket. To fix status and fields together use tix_set with \"status\" in values.",
            schema(json!({"id": id_prop(), "status": {"type": "string", "description": "A status name from tix.yaml."}}), &["id", "status"]), false),
        tool("tix_set", "Change title, status and/or fields in one all-or-nothing write; fields not named stay unchanged. An empty string removes a field. Returns the updated ticket.",
            schema(json!({"id": id_prop(), "values": {"type": "object", "additionalProperties": field_value_schema(), "description": "Keys to change: \"title\", \"status\", or schema fields, e.g. {\"owner\": \"sam\", \"due\": \"2026-09-30\", \"notes\": \"\"}."}}), &["id", "values"]), false),
        tool("tix_attach", "Attach a deliverable reference (URL, path, or external id) to a ticket. Never copies or downloads anything. Fails if the ref is already attached. Returns the updated ticket.",
            schema(json!({"id": id_prop(), "ref": {"type": "string", "description": "Reference stored verbatim; ./ means relative to the ticket folder."}, "label": {"type": "string", "description": "Label; defaults to the last /-segment of ref."}}), &["id", "ref"]), false),
        tool("tix_detach", "Remove the single deliverable whose ref or label equals target. Fails if none or several match. Returns the updated ticket.",
            schema(json!({"id": id_prop(), "target": {"type": "string", "description": "Exact ref or label of the deliverable to remove."}}), &["id", "target"]), false),
        tool("tix_board", "Board view as columns: {\"columns\": [{\"name\", \"tickets\": [...]}]}, one per status (or per group with group=true), plus \"?\" for unknown statuses. For plain data prefer tix_ls.",
            schema(json!({"filters": {"type": "array", "items": {"type": "string"}}, "group": {"type": "boolean", "description": "Columns backlog, in_progress, completed instead of one per status."}}), &[]), true),
        tool("tix_path", "Path of a ticket's folder relative to the board root, e.g. {\"path\": \"tickets/01K5...\"}. Edit the brief in <board>/<path>/ticket.md below the frontmatter, then run tix_check.",
            schema(json!({"id": id_prop()}), &["id"]), true),
    ]
}

fn text_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn str_arg(args: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("argument '{key}' must be a string")),
    }
}

fn req_arg(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    str_arg(args, key)?.ok_or_else(|| format!("missing required argument '{key}'"))
}

fn cli_value(key: &str, v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                i.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("'{key}' list items must be strings"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.join(",")),
        _ => Err(format!(
            "value of '{key}' must be a string or a list of strings"
        )),
    }
}

fn string_list(args: &Map<String, Value>, key: &str) -> Result<Vec<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|i| {
                i.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("'{key}' must be a list of strings"))
            })
            .collect(),
        Some(_) => Err(format!("argument '{key}' must be a list of strings")),
    }
}

/// Maps a tool call to `tix` argv (without `--json`/`--no-prompt`) and whether to request JSON.
fn tool_argv(name: &str, args: &Map<String, Value>) -> Result<(Vec<String>, bool), String> {
    let s = |x: &str| x.to_string();
    Ok(match name {
        "tix_help" => {
            let mut v = str_arg(args, "command")?.into_iter().collect::<Vec<_>>();
            v.push(s("--help"));
            (v, false)
        }
        "tix_init" => (vec![s("init")], true),
        "tix_check" => (vec![s("check")], true),
        "tix_new" => {
            let mut v = vec![s("new"), s("--title"), req_arg(args, "title")?];
            if let Some(st) = str_arg(args, "status")? {
                v.extend([s("--status"), st]);
            }
            if let Some(fields) = args.get("fields").and_then(Value::as_object) {
                for (k, val) in fields {
                    v.extend([format!("--{k}"), cli_value(k, val)?]);
                }
            }
            (v, true)
        }
        "tix_ls" => {
            let mut v = vec![s("ls")];
            v.extend(string_list(args, "filters")?);
            (v, true)
        }
        "tix_show" => (vec![s("show"), req_arg(args, "id")?], true),
        "tix_mv" => (
            vec![s("mv"), req_arg(args, "id")?, req_arg(args, "status")?],
            true,
        ),
        "tix_set" => {
            let mut v = vec![s("set"), req_arg(args, "id")?];
            let values = args
                .get("values")
                .and_then(Value::as_object)
                .ok_or("missing required argument 'values' (an object)")?;
            for (k, val) in values {
                v.push(format!("{k}={}", cli_value(k, val)?));
            }
            (v, true)
        }
        "tix_attach" => {
            let mut v = vec![s("attach"), req_arg(args, "id")?, req_arg(args, "ref")?];
            if let Some(l) = str_arg(args, "label")? {
                v.extend([s("--label"), l]);
            }
            (v, true)
        }
        "tix_detach" => (
            vec![s("detach"), req_arg(args, "id")?, req_arg(args, "target")?],
            true,
        ),
        "tix_board" => {
            let mut v = vec![s("board")];
            v.extend(string_list(args, "filters")?);
            if args.get("group").and_then(Value::as_bool).unwrap_or(false) {
                v.push(s("--group"));
            }
            (v, true)
        }
        "tix_path" => (vec![s("path"), req_arg(args, "id")?], true),
        _ => unreachable!("checked by caller"),
    })
}

fn call_tool<H: Host>(
    host: &mut H,
    default_dir: &str,
    name: &str,
    args: &Map<String, Value>,
) -> Value {
    let (mut argv, json_out) = match tool_argv(name, args) {
        Ok(x) => x,
        Err(msg) => return text_result(format!("invalid arguments: {msg}"), true),
    };
    if json_out {
        argv.push("--json".into());
    }
    argv.push("--no-prompt".into());
    let dir = match str_arg(args, "workspace") {
        Ok(Some(w)) => w.replace('\\', "/"),
        Ok(None) => default_dir.to_string(),
        Err(msg) => return text_result(format!("invalid arguments: {msg}"), true),
    };
    let mut cap = Capture {
        inner: host,
        out: String::new(),
        err: String::new(),
    };
    let code = crate::cli::run(&mut cap, &argv, &dir);
    if code == 0 {
        let mut text = cap.out;
        if !cap.err.is_empty() {
            text.push_str(&format!("\nwarnings:\n{}", cap.err));
        }
        text_result(text, false)
    } else {
        let meaning = match code {
            1 => "validation error",
            2 => "usage or workspace error",
            3 => "I/O error",
            _ => "error",
        };
        let mut text = format!("exit {code} ({meaning}): {}", cap.err.trim_end());
        if !cap.out.trim().is_empty() {
            text.push_str(&format!("\n{}", cap.out.trim_end()));
        }
        text_result(text, true)
    }
}

fn response(id: Value, result: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

fn error(id: Value, code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}).to_string()
}

/// Handles one JSON-RPC message; returns the response line, or `None` for notifications.
pub fn handle<H: Host>(host: &mut H, default_dir: &str, line: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Some(error(Value::Null, -32700, "parse error")),
    };
    let Some(obj) = msg.as_object() else {
        return Some(error(Value::Null, -32600, "invalid request"));
    };
    let method = obj.get("method").and_then(Value::as_str).unwrap_or("");
    let Some(id) = obj.get("id").cloned() else {
        return None; // notification (e.g. notifications/initialized)
    };
    let empty = Map::new();
    let params = obj
        .get("params")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    Some(match method {
        "initialize" => {
            let asked = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("");
            let version = PROTOCOL_VERSIONS
                .iter()
                .find(|v| **v == asked)
                .copied()
                .unwrap_or(PROTOCOL_VERSIONS[0]);
            response(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "tix", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": INSTRUCTIONS
                }),
            )
        }
        "ping" => response(id, json!({})),
        "tools/list" => response(id, json!({"tools": tools()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            if !tools().iter().any(|t| t["name"] == name) {
                return Some(error(id, -32602, &format!("unknown tool '{name}'")));
            }
            let args = params
                .get("arguments")
                .and_then(Value::as_object)
                .unwrap_or(&empty);
            response(id, call_tool(host, default_dir, name, args))
        }
        _ => error(id, -32601, &format!("method not found: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::MemHost;

    fn ask(h: &mut MemHost, line: Value) -> Value {
        serde_json::from_str(&handle(h, "/ws", &line.to_string()).expect("response")).unwrap()
    }

    #[test]
    fn mcp_initialize_negotiates_version_and_ignores_notifications() {
        let mut h = MemHost::new();
        let r = ask(
            &mut h,
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}}),
        );
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(r["result"]["serverInfo"]["name"], "tix");
        let r = ask(
            &mut h,
            json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {"protocolVersion": "1999-01-01"}}),
        );
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
        assert!(handle(
            &mut h,
            "/ws",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
        )
        .is_none());
    }

    #[test]
    fn mcp_errors_for_bad_json_unknown_method_and_tool() {
        let mut h = MemHost::new();
        let r: Value = serde_json::from_str(&handle(&mut h, "/ws", "{not json").unwrap()).unwrap();
        assert_eq!(r["error"]["code"], -32700);
        assert_eq!(
            ask(&mut h, json!({"jsonrpc": "2.0", "id": 1, "method": "nope"}))["error"]["code"],
            -32601
        );
        let r = ask(
            &mut h,
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "tix_nope"}}),
        );
        assert_eq!(r["error"]["code"], -32602);
    }

    #[test]
    fn mcp_tool_calls_run_commands_and_capture_output() {
        let mut h = MemHost::new();
        let call = |h: &mut MemHost, name: &str, args: Value| {
            let r = ask(
                h,
                json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": name, "arguments": args}}),
            );
            (
                r["result"]["isError"].as_bool().unwrap(),
                r["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            )
        };
        assert!(!call(&mut h, "tix_init", json!({})).0);
        let (err, text) = call(
            &mut h,
            "tix_new",
            json!({"title": "x", "fields": {"client": "acme", "type": "article"}}),
        );
        assert!(!err, "{text}");
        assert!(
            h.stdout.is_empty(),
            "tool output must not reach the host's stdout"
        );
        let (err, text) = call(&mut h, "tix_new", json!({"title": "x"}));
        assert!(
            err && text.starts_with("exit 1 (validation error): rule 3"),
            "{text}"
        );
        let (err, text) = call(&mut h, "tix_show", json!({}));
        assert!(
            err && text.contains("missing required argument 'id'"),
            "{text}"
        );
        assert!(h.prompts.is_empty());
    }
}
