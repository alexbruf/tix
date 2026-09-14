//! argv parsing (clap) and command dispatch. `run` is what the wasm export calls.

use crate::app::SCHEMA_FILE;
use crate::commands;
use crate::host::{CmdResult, Ctx, Failure, Host, EXIT_OK, EXIT_USAGE};
use crate::schema_yaml::parse_schema;
use crate::storage::join_path;
use crate::workspace::find_workspace;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tix_core::Schema;

/// Builds the command tree. With a schema, `tix new` gets one `--<field>` flag per field (TIX-15).
pub fn command(schema: Option<&Schema>) -> Command {
    let id = || {
        Arg::new("id")
            .required(true)
            .help("Ticket id or unique prefix (4+ chars)")
    };
    let filters = || {
        Arg::new("filters")
            .num_args(0..)
            .help("KEY:VALUE filters, ANDed")
    };
    let mut new = Command::new("new")
        .about("Create a ticket")
        .arg(Arg::new("title").long("title").value_name("T"))
        .arg(Arg::new("status").long("status").value_name("S"));
    if let Some(s) = schema {
        for f in &s.fields {
            if f.name != "title" && f.name != "status" {
                new = new.arg(
                    Arg::new(f.name.clone())
                        .long(f.name.clone())
                        .value_name("VALUE"),
                );
            }
        }
    }
    Command::new("tix")
        .about("File-based ticket board")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Machine output"),
        )
        .arg(
            Arg::new("no-prompt")
                .long("no-prompt")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Never prompt; missing required input is an error"),
        )
        .subcommand(Command::new("init").about("Create tix.yaml and tickets/ here"))
        .subcommand(Command::new("check").about("Validate tix.yaml and every ticket"))
        .subcommand(new)
        .subcommand(Command::new("ls").about("List tickets").arg(filters()))
        .subcommand(Command::new("show").about("Show one ticket").arg(id()))
        .subcommand(
            Command::new("mv")
                .about("Change a ticket's status")
                .arg(id())
                .arg(Arg::new("status").required(true)),
        )
        .subcommand(
            Command::new("set")
                .about("Set title or fields")
                .arg(id())
                .arg(
                    Arg::new("assignments")
                        .required(true)
                        .num_args(1..)
                        .value_name("KEY=VALUE"),
                ),
        )
        .subcommand(
            Command::new("attach")
                .about("Attach a deliverable reference")
                .arg(id())
                .arg(Arg::new("ref").required(true).value_name("REF"))
                .arg(Arg::new("label").long("label").value_name("L")),
        )
        .subcommand(
            Command::new("detach")
                .about("Remove a deliverable by ref or label")
                .arg(id())
                .arg(Arg::new("target").required(true).value_name("REF_OR_LABEL")),
        )
        .subcommand(
            Command::new("board")
                .about("Show the board")
                .arg(filters())
                .arg(
                    Arg::new("group")
                        .long("group")
                        .action(ArgAction::SetTrue)
                        .help("One column per group"),
                ),
        )
        .subcommand(
            Command::new("path")
                .about("Print the ticket folder path")
                .arg(id()),
        )
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
