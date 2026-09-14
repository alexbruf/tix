//! argv parsing (clap) and command dispatch. `run` is what the wasm export calls.

use crate::app::SCHEMA_FILE;
use crate::commands;
use crate::help;
use crate::host::{CmdResult, Ctx, Failure, Host, EXIT_OK, EXIT_USAGE};
use crate::messages::display_value;
use crate::schema_yaml::parse_schema;
use crate::storage::join_path;
use crate::workspace::find_workspace;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tix_core::{Field, FieldType, Schema};

/// Builds the command tree. With a schema, `tix new` gets one `--<field>` flag per field (TIX-15).
pub fn command(schema: Option<&Schema>) -> Command {
    let id = || {
        Arg::new("id")
            .required(true)
            .value_name("ID")
            .help("Ticket id or a unique prefix of 4+ characters")
    };
    let filters = || {
        Arg::new("filters")
            .num_args(0..)
            .value_name("KEY:VALUE")
            .help("Filters, all must match: status:<s>, group:<g>, <field>:<v>")
    };
    let mut new = Command::new("new")
        .about("Create a ticket and print its id")
        .long_about(help::NEW_LONG)
        .after_long_help(help::NEW_AFTER)
        .arg(
            Arg::new("title")
                .long("title")
                .value_name("TEXT")
                .help("Title (required; prompted if omitted)"),
        )
        .arg(
            Arg::new("status")
                .long("status")
                .value_name("STATUS")
                .help(match schema {
                    Some(s) => format!(
                        "Initial status, one of: {} [default: first backlog status]",
                        s.statuses
                            .iter()
                            .map(|x| x.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    None => "Initial status [default: first backlog status]".to_string(),
                }),
        );
    match schema {
        Some(s) => {
            for f in &s.fields {
                if f.name != "title" && f.name != "status" {
                    new = new.arg(
                        Arg::new(f.name.clone())
                            .long(f.name.clone())
                            .value_name(field_value_name(f))
                            .help(field_help(f)),
                    );
                }
            }
        }
        None => {
            new = new.before_long_help("(Run inside a workspace to see its --<field> flags.)");
        }
    }
    Command::new("tix")
        .about("File-based ticket board: tickets are Markdown files, the schema is tix.yaml")
        .long_about(help::ROOT_LONG)
        .after_help(
            "Every command has its own help with formats, output and examples: `tix <command> --help`.\n\
             Run `tix --help` for the agent quickstart, JSON shapes and exit codes.",
        )
        .after_long_help(help::ROOT_AFTER)
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print exactly one JSON document on stdout"),
        )
        .arg(
            Arg::new("no-prompt")
                .long("no-prompt")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Never read stdin; a missing required value is an error (use in scripts and agents)"),
        )
        .subcommand(
            Command::new("init")
                .about("Create tix.yaml and tickets/ in the current directory")
                .long_about(help::INIT_LONG)
                .after_long_help(help::INIT_AFTER),
        )
        .subcommand(
            Command::new("check")
                .about("Validate tix.yaml and every ticket; prints `ok` or one line per problem")
                .long_about(help::CHECK_LONG)
                .after_long_help(help::CHECK_AFTER),
        )
        .subcommand(new)
        .subcommand(
            Command::new("ls")
                .about("List tickets as a table, optionally filtered")
                .long_about(help::LS_LONG)
                .after_long_help(format!("{}\n\n{}", help::FILTERS, help::LS_AFTER))
                .arg(filters()),
        )
        .subcommand(
            Command::new("show")
                .about("Print one ticket: fields, deliverables and brief")
                .long_about(help::SHOW_LONG)
                .after_long_help(help::SHOW_AFTER)
                .arg(id()),
        )
        .subcommand(
            Command::new("mv")
                .about("Change a ticket's status")
                .long_about(help::MV_LONG)
                .after_long_help(help::MV_AFTER)
                .arg(id())
                .arg(
                    Arg::new("status")
                        .required(true)
                        .value_name("STATUS")
                        .help("A status name from tix.yaml"),
                ),
        )
        .subcommand(
            Command::new("set")
                .about("Set title, status or fields in one all-or-nothing write")
                .long_about(help::SET_LONG)
                .after_long_help(help::SET_AFTER)
                .arg(id())
                .arg(
                    Arg::new("assignments")
                        .required(true)
                        .num_args(1..)
                        .value_name("KEY=VALUE")
                        .help("title=..., status=..., <field>=... ; <field>= removes the field"),
                ),
        )
        .subcommand(
            Command::new("attach")
                .about("Attach a deliverable reference (URL, path or id; never copied)")
                .long_about(help::ATTACH_LONG)
                .after_long_help(help::ATTACH_AFTER)
                .arg(id())
                .arg(
                    Arg::new("ref")
                        .required(true)
                        .value_name("REF")
                        .help("Reference stored verbatim"),
                )
                .arg(
                    Arg::new("label")
                        .long("label")
                        .value_name("LABEL")
                        .help("Label [default: last /-segment of REF]"),
                ),
        )
        .subcommand(
            Command::new("detach")
                .about("Remove the one deliverable whose ref or label matches")
                .long_about(help::DETACH_LONG)
                .after_long_help(help::DETACH_AFTER)
                .arg(id())
                .arg(
                    Arg::new("target")
                        .required(true)
                        .value_name("REF_OR_LABEL")
                        .help("Exact ref or label of the deliverable to remove"),
                ),
        )
        .subcommand(
            Command::new("board")
                .about("Show the board: one column per status (or per group)")
                .long_about(help::BOARD_LONG)
                .after_long_help(format!("{}\n\n{}", help::FILTERS, help::BOARD_AFTER))
                .arg(filters())
                .arg(
                    Arg::new("group")
                        .long("group")
                        .action(ArgAction::SetTrue)
                        .help("Columns backlog, in_progress, completed instead of statuses"),
                ),
        )
        .subcommand(
            Command::new("path")
                .about("Print tickets/<ID>, the ticket folder relative to the workspace root")
                .long_about(help::PATH_LONG)
                .after_long_help(help::PATH_AFTER)
                .arg(id()),
        )
}

fn field_value_name(f: &Field) -> String {
    match f.ty {
        FieldType::Str => "TEXT".to_string(),
        FieldType::Enum => f.values.join("|"),
        FieldType::Date => "YYYY-MM-DD".to_string(),
        FieldType::List => "A,B,...".to_string(),
    }
}

/// One line per schema field for `tix new --help`: type, required, default.
fn field_help(f: &Field) -> String {
    let ty = match f.ty {
        FieldType::Str => "string".to_string(),
        FieldType::Enum => format!("enum: {}", f.values.join(", ")),
        FieldType::Date => "date".to_string(),
        FieldType::List => "list, comma-separated".to_string(),
    };
    let mut text = format!(
        "{} ({ty})",
        if f.required { "Required" } else { "Optional" }
    );
    if let Some(d) = &f.default {
        text.push_str(&format!(" [default: {}]", display_value(d)));
    } else if f.required {
        text.push_str(" [prompted if omitted]");
    }
    text
}

/// Runs one invocation. `argv` excludes the program name; `cwd` is absolute and `/`-separated.
pub fn run<H: Host>(host: &mut H, argv: &[String], cwd: &str) -> u32 {
    let root = match find_workspace(host, cwd) {
        Ok(r) => r,
        Err(e) => return fail(host, Failure::from(e)),
    };
    // Best effort: only used to add `tix new --<field>` flags.
    let schema = root.as_ref().and_then(|r| {
        let bytes = host.read(&join_path(r, SCHEMA_FILE).ok()?).ok()??;
        parse_schema(&bytes).ok()
    });
    let args = std::iter::once("tix".to_string()).chain(argv.iter().cloned());
    let matches = match command(schema.as_ref()).try_get_matches_from(args) {
        Ok(m) => m,
        Err(e) => {
            let text = e.render().to_string();
            return if e.use_stderr() {
                host.stderr(&text);
                EXIT_USAGE
            } else {
                host.stdout(&text);
                EXIT_OK
            };
        }
    };
    let (name, sub) = matches.subcommand().expect("subcommand_required");
    let json = matches.get_flag("json");
    let no_prompt = matches.get_flag("no-prompt");
    let root = match (name, root) {
        ("init", _) => cwd.to_string(),
        (_, Some(r)) => r,
        (_, None) => return fail(host, Failure::usage("no tix.yaml found")),
    };
    let mut ctx = Ctx {
        host,
        root,
        json,
        no_prompt,
    };
    match dispatch(&mut ctx, name, sub) {
        Ok(()) => EXIT_OK,
        Err(f) => fail(ctx.host, f),
    }
}

fn dispatch<H: Host>(ctx: &mut Ctx<H>, name: &str, m: &ArgMatches) -> CmdResult {
    match name {
        "init" => commands::init::run(ctx, m),
        "check" => commands::check::run(ctx, m),
        "new" => commands::new::run(ctx, m),
        "ls" => commands::ls::run(ctx, m),
        "show" => commands::show::run(ctx, m),
        "mv" => commands::mv::run(ctx, m),
        "set" => commands::set::run(ctx, m),
        "attach" => commands::attach::run(ctx, m),
        "detach" => commands::detach::run(ctx, m),
        "board" => commands::board::run(ctx, m),
        "path" => commands::path::run(ctx, m),
        _ => unreachable!("clap rejects unknown subcommands"),
    }
}

fn fail<H: Host>(host: &mut H, f: Failure) -> u32 {
    let mut msg = f.message;
    if !msg.ends_with('\n') {
        msg.push('\n');
    }
    host.stderr(&msg);
    f.code
}
