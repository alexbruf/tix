//! `tix mcp`: a local MCP server over stdio. Each tool runs the matching tix
//! command in-process with `--json --no-prompt` and returns its stdout, or its
//! stderr and exit code as a tool error, so behaviour matches the CLI exactly.

use crate::host::{normalize_dir, StdHost};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct TixServer {
    default_dir: String,
}

#[derive(Deserialize, JsonSchema, Default)]
pub struct WorkspaceArgs {
    /// Absolute path of the board folder, or any folder inside it. Defaults to the server's working directory.
    pub workspace: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HelpArgs {
    /// Command to explain (init, check, new, ls, show, mv, set, attach, detach, board, path). Omit for the overview.
    pub command: Option<String>,
    /// Board folder; matters for `new`, whose help lists that board's fields.
    pub workspace: Option<String>,
}

/// A field value: a string, or a list of strings for `list` fields.
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FieldValue {
    One(String),
    Many(Vec<String>),
}

impl FieldValue {
    fn cli(&self) -> String {
        match self {
            FieldValue::One(s) => s.clone(),
            FieldValue::Many(v) => v.join(","),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct NewArgs {
    pub workspace: Option<String>,
    /// Ticket title (required).
    pub title: String,
    /// Initial status; defaults to the first backlog status.
    pub status: Option<String>,
    /// Schema fields by name, e.g. {"client": "acme", "type": "article", "tags": ["a", "b"]}. See tix_help command "new".
    #[serde(default)]
    pub fields: BTreeMap<String, FieldValue>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FilterArgs {
    pub workspace: Option<String>,
    /// KEY:VALUE filters, all must match: "status:backlog", "group:in_progress", "client:acme".
    #[serde(default)]
    pub filters: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct BoardArgs {
    pub workspace: Option<String>,
    #[serde(default)]
    pub filters: Vec<String>,
    /// Columns backlog, in_progress, completed instead of one per status.
    #[serde(default)]
    pub group: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct IdArgs {
    pub workspace: Option<String>,
    /// Ticket id or unique prefix of 4+ characters.
    pub id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct MoveArgs {
    pub workspace: Option<String>,
    pub id: String,
    /// A status name from tix.yaml.
    pub status: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SetArgs {
    pub workspace: Option<String>,
    pub id: String,
    /// Keys to change: "title", "status", or schema fields. An empty string removes a field.
    /// Example: {"owner": "sam", "due": "2026-09-30", "tags": ["a", "b"], "notes": ""}.
    pub values: BTreeMap<String, FieldValue>,
}

#[derive(Deserialize, JsonSchema)]
pub struct AttachArgs {
    pub workspace: Option<String>,
    pub id: String,
    /// Reference stored verbatim: URL, path (./ means relative to the ticket folder), or an id in another tool.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Label; defaults to the last /-segment of ref.
    pub label: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct DetachArgs {
    pub workspace: Option<String>,
    pub id: String,
    /// Exact ref or label of the deliverable to remove.
    pub target: String,
}

impl TixServer {
    fn dir(&self, workspace: &Option<String>) -> String {
        workspace
            .as_deref()
            .map(normalize_dir)
            .unwrap_or_else(|| self.default_dir.clone())
    }

    /// Runs `tix <args> [--json] --no-prompt` in `dir` and maps the outcome to a tool result.
    fn exec(&self, dir: String, mut args: Vec<String>, json: bool) -> CallToolResult {
        if json {
            args.push("--json".into());
        }
        args.push("--no-prompt".into());
        let mut host = StdHost {
            capture: true,
            ..Default::default()
        };
        let code = tix_io::run(&mut host, &args, &dir);
        if code == 0 {
            let mut text = host.out;
            if !host.err.is_empty() {
                text.push_str(&format!("\nwarnings:\n{}", host.err));
            }
            CallToolResult::success(vec![ContentBlock::text(text)])
        } else {
            let meaning = match code {
                1 => "validation error",
                2 => "usage or workspace error",
                3 => "I/O error",
                _ => "error",
            };
            let mut text = format!("exit {code} ({meaning}): {}", host.err.trim_end());
            if !host.out.trim().is_empty() {
                text.push_str(&format!("\n{}", host.out.trim_end()));
            }
            CallToolResult::error(vec![ContentBlock::text(text)])
        }
    }
}

fn s(x: &str) -> String {
    x.to_string()
}

#[tool_router]
impl TixServer {
    pub fn new(default_dir: String) -> Self {
        TixServer { default_dir }
    }

    #[tool(
        description = "Documentation for tix or one command: argument formats, output and JSON shapes, exit codes, examples. Call with no command first, then per command before first use; for \"new\" pass workspace to see that board's fields.",
        annotations(read_only_hint = true)
    )]
    async fn tix_help(&self, Parameters(a): Parameters<HelpArgs>) -> CallToolResult {
        let mut args = Vec::new();
        if let Some(c) = &a.command {
            args.push(c.clone());
        }
        args.push(s("--help"));
        self.exec(self.dir(&a.workspace), args, false)
    }

    #[tool(
        description = "Create tix.yaml (default schema) and tickets/ in the workspace folder. Fails if a board already exists there or in a parent. Only run when the user asked for a new board.",
        annotations(destructive_hint = false, idempotent_hint = false)
    )]
    async fn tix_init(&self, Parameters(a): Parameters<WorkspaceArgs>) -> CallToolResult {
        self.exec(self.dir(&a.workspace), vec![s("init")], true)
    }

    #[tool(
        description = "Validate tix.yaml and every ticket. Returns {\"ok\": bool, \"problems\": [{path, message}]}; problems are also reported as an error result. Only the first broken rule per ticket is listed, so re-run after fixing.",
        annotations(read_only_hint = true)
    )]
    async fn tix_check(&self, Parameters(a): Parameters<WorkspaceArgs>) -> CallToolResult {
        self.exec(self.dir(&a.workspace), vec![s("check")], true)
    }

    #[tool(
        description = "Create a ticket. Pass schema fields in `fields` (learn them with tix_help command \"new\"). Returns the new ticket object including its id. Nothing is written if a required field is missing or a value is not allowed.",
        annotations(destructive_hint = false)
    )]
    async fn tix_new(&self, Parameters(a): Parameters<NewArgs>) -> CallToolResult {
        let mut args = vec![s("new"), s("--title"), a.title.clone()];
        if let Some(st) = &a.status {
            args.extend([s("--status"), st.clone()]);
        }
        for (k, v) in &a.fields {
            args.extend([format!("--{k}"), v.cli()]);
        }
        self.exec(self.dir(&a.workspace), args, true)
    }

    #[tool(
        description = "List tickets (optionally filtered) in board order: status order, then oldest first. Returns an array of ticket objects with \"valid\" and \"problem\".",
        annotations(read_only_hint = true)
    )]
    async fn tix_ls(&self, Parameters(a): Parameters<FilterArgs>) -> CallToolResult {
        let mut args = vec![s("ls")];
        args.extend(a.filters.iter().cloned());
        self.exec(self.dir(&a.workspace), args, true)
    }

    #[tool(
        description = "Get one ticket: all fields, deliverables, \"body\" (the Markdown brief), \"valid\" and \"problem\".",
        annotations(read_only_hint = true)
    )]
    async fn tix_show(&self, Parameters(a): Parameters<IdArgs>) -> CallToolResult {
        self.exec(self.dir(&a.workspace), vec![s("show"), a.id.clone()], true)
    }

    #[tool(
        description = "Change a ticket's status (any status to any other). Returns the updated ticket. To fix status and fields together use tix_set with \"status\" in values.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn tix_mv(&self, Parameters(a): Parameters<MoveArgs>) -> CallToolResult {
        self.exec(
            self.dir(&a.workspace),
            vec![s("mv"), a.id.clone(), a.status.clone()],
            true,
        )
    }

    #[tool(
        description = "Change title, status and/or fields in one all-or-nothing write; fields not named stay unchanged. An empty string removes a field. Returns the updated ticket.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn tix_set(&self, Parameters(a): Parameters<SetArgs>) -> CallToolResult {
        let mut args = vec![s("set"), a.id.clone()];
        for (k, v) in &a.values {
            args.push(format!("{k}={}", v.cli()));
        }
        self.exec(self.dir(&a.workspace), args, true)
    }

    #[tool(
        description = "Attach a deliverable reference (URL, path, or external id) to a ticket. Never copies or downloads anything. Fails if the ref is already attached. Returns the updated ticket.",
        annotations(destructive_hint = false)
    )]
    async fn tix_attach(&self, Parameters(a): Parameters<AttachArgs>) -> CallToolResult {
        let mut args = vec![s("attach"), a.id.clone(), a.reference.clone()];
        if let Some(l) = &a.label {
            args.extend([s("--label"), l.clone()]);
        }
        self.exec(self.dir(&a.workspace), args, true)
    }

    #[tool(
        description = "Remove the single deliverable whose ref or label equals target. Fails if none or several match. Returns the updated ticket.",
        annotations(destructive_hint = true)
    )]
    async fn tix_detach(&self, Parameters(a): Parameters<DetachArgs>) -> CallToolResult {
        self.exec(
            self.dir(&a.workspace),
            vec![s("detach"), a.id.clone(), a.target.clone()],
            true,
        )
    }

    #[tool(
        description = "Board view as columns: {\"columns\": [{\"name\", \"tickets\": [...]}]}, one per status (or per group with group=true), plus \"?\" for unknown statuses. For plain data prefer tix_ls.",
        annotations(read_only_hint = true)
    )]
    async fn tix_board(&self, Parameters(a): Parameters<BoardArgs>) -> CallToolResult {
        let mut args = vec![s("board")];
        args.extend(a.filters.iter().cloned());
        if a.group {
            args.push(s("--group"));
        }
        self.exec(self.dir(&a.workspace), args, true)
    }

    #[tool(
        description = "Path of a ticket's folder relative to the board root, e.g. {\"path\": \"tickets/01K5...\"}. Edit the brief in <board>/<path>/ticket.md below the frontmatter, then run tix_check.",
        annotations(read_only_hint = true)
    )]
    async fn tix_path(&self, Parameters(a): Parameters<IdArgs>) -> CallToolResult {
        self.exec(self.dir(&a.workspace), vec![s("path"), a.id.clone()], true)
    }
}

#[tool_handler(
    name = "tix",
    instructions = "tix is a ticket board stored as files: tix.yaml (schema) plus tickets/<ID>/ticket.md. Every tool accepts an optional `workspace` (absolute path of the board folder or any folder inside it); without it the server's working directory is used. Start with tix_help (no command) for the rules, JSON shapes and exit codes, and call tix_help with a command name before its first use. Call tix_help with command \"new\" inside a board to learn its fields and allowed values before tix_new. Results are the command's JSON output; failures come back as errors whose text names the broken rule (e.g. rule 3: required field 'client' is missing or empty). Nothing is written when a write fails. Use full ids from results; prefixes of 4+ characters also work."
)]
impl ServerHandler for TixServer {}

pub const HELP: &str = "\
Run a local MCP server over stdio exposing tix commands as tools.

Usage: tix mcp [--workspace DIR]

  --workspace DIR  default board folder for tools that omit `workspace`
                   [default: the current directory]

Tools: tix_help, tix_init, tix_check, tix_new, tix_ls, tix_show, tix_mv, tix_set,
       tix_attach, tix_detach, tix_board, tix_path. Each runs the matching command
       with --json --no-prompt and returns its output, or an error naming the rule.

Setup:
  Claude Code:     claude mcp add tix -- tix mcp
  Claude Desktop / other clients (JSON config):
    {\"mcpServers\": {\"tix\": {\"command\": \"tix\", \"args\": [\"mcp\", \"--workspace\", \"/path/to/board\"]}}}
";

/// Entry point for `tix mcp`; `args` are the arguments after `mcp`.
pub fn main(args: &[String], cwd: String) -> i32 {
    let mut dir = cwd;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return 0;
            }
            "--workspace" => match it.next() {
                Some(d) => dir = normalize_dir(d),
                None => {
                    eprintln!("--workspace needs a directory");
                    return 2;
                }
            },
            other => {
                eprintln!("unexpected argument '{other}'\n\n{HELP}");
                return 2;
            }
        }
    }
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let service = match TixServer::new(dir).serve(rmcp::transport::stdio()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("tix mcp: {e}");
                return 3;
            }
        };
        match service.waiting().await {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("tix mcp: {e}");
                3
            }
        }
    })
}
