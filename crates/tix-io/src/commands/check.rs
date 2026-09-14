//! `tix check` (TIX-22).

use crate::app::{read_schema, ticket_file, ticket_folders, SCHEMA_FILE};
use crate::host::{CmdResult, Ctx, Failure, Host, EXIT_VALIDATION};
use crate::messages::{schema_error, ticket_error};
use crate::storage::Storage;
use crate::ticket_md::parse_ticket;
use clap::ArgMatches;
use serde_json::Value as J;

pub fn run<H: Host>(ctx: &mut Ctx<H>, _m: &ArgMatches) -> CmdResult {
    let mut problems: Vec<(String, String)> = Vec::new();

    let schema = match read_schema(ctx)? {
        Err(msg) => {
            problems.push((SCHEMA_FILE.to_string(), msg));
            None
        }
        Ok(s) => match s.validate() {
            Err(e) => {
                problems.push((SCHEMA_FILE.to_string(), schema_error(&s, &e)));
                None
            }
            Ok(()) => Some(s),
        },
    };

    if let Some(schema) = &schema {
        for folder in ticket_folders(ctx)? {
            let path = ticket_file(&folder);
            match ctx.read(&path)? {
                None => {
                    problems.push((format!("tickets/{folder}"), "missing ticket.md".to_string()))
                }
                Some(bytes) => match parse_ticket(&bytes, schema, &folder) {
                    Err(e) => problems.push((path, e.to_string())),
                    Ok(t) => {
                        if t.id != folder {
                            problems.push((
                                path.clone(),
                                format!("id '{}' does not match folder name", t.id),
                            ));
                        }
                        if let Err(e) = t.validate(schema) {
                            problems.push((path, ticket_error(&t, schema, &e)));
                        }
                    }
                },
            }
        }
    }

    if ctx.json {
        let arr: Vec<J> = problems
            .iter()
            .map(|(p, m)| serde_json::json!({ "path": p, "message": m }))
            .collect();
        ctx.out(&format!(
            "{}\n",
            serde_json::json!({ "ok": problems.is_empty(), "problems": arr })
        ));
    } else if problems.is_empty() {
        ctx.out("ok\n");
    } else {
        for (p, m) in &problems {
            ctx.out(&format!("{p}: {m}\n"));
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(Failure {
            code: EXIT_VALIDATION,
            message: format!("{} problem(s)", problems.len()),
        })
    }
}
