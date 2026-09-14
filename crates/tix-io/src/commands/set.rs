//! `tix set` (TIX-19).

use crate::app::{
    cli_value, emit_ticket, load_ticket, load_valid_schema, resolve_folder, save_ticket,
};
use crate::host::{CmdResult, Ctx, Failure, Host};
use crate::messages::ticket_error;
use clap::ArgMatches;
use tix_core::ops::{set_fields, Assignment};
use tix_core::{FieldEntry, Ticket, Value};

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;
    let prefix = m.get_one::<String>("id").expect("required");
    let folder = resolve_folder(ctx, prefix)?;
    let t = load_ticket(ctx, &schema, &folder)?;

    let raw: Vec<&String> = m
        .get_many::<String>("assignments")
        .expect("required")
        .collect();

    let mut seen: Vec<&str> = Vec::new();
    let mut title: Option<String> = None;
    let mut status: Option<String> = None;
    let mut assigns: Vec<Assignment> = Vec::new();

    for arg in raw {
        let (key, value) = arg
            .split_once('=')
            .ok_or_else(|| Failure::usage(format!("assignment '{arg}' is not KEY=VALUE")))?;
        if seen.contains(&key) {
            return Err(Failure::usage(format!("key '{key}' given more than once")));
        }
        seen.push(key);

        match key {
            "title" => title = Some(value.to_string()),
            "id" => return Err(Failure::usage("cannot set 'id'")),
            "status" => status = Some(value.to_string()),
            "created" => return Err(Failure::usage("cannot set 'created'")),
            "updated" => return Err(Failure::usage("cannot set 'updated'")),
            "deliverables" => {
                return Err(Failure::usage(
                    "cannot set 'deliverables'; use tix attach/detach",
                ))
            }
            _ => {
                if value.is_empty() {
                    assigns.push(Assignment {
                        name: key.to_string(),
                        value: None,
                    });
                } else if let Some(field) = schema.fields.iter().find(|f| f.name == key) {
                    assigns.push(Assignment {
                        name: key.to_string(),
                        value: Some(cli_value(field, value)),
                    });
                } else {
                    assigns.push(Assignment {
                        name: key.to_string(),
                        value: Some(Value::Str(value.to_string())),
                    });
                }
            }
        }
    }

    let now = ctx.host.now_unix();
    let candidate = message_candidate(&t, &title, &status, &assigns);
    match set_fields(t, &schema, title, status, assigns, now) {
        Ok(t2) => {
            save_ticket(ctx, &schema, &t2)?;
            emit_ticket(ctx, &schema, &t2);
            Ok(())
        }
        Err(e) => Err(Failure::validation(ticket_error(&candidate, &schema, &e))),
    }
}

/// The ticket `set_fields` validates, rebuilt only so error messages can
/// point at the right field (the core's `apply_all`: remove, then append).
fn message_candidate(
    t: &Ticket,
    title: &Option<String>,
    status: &Option<String>,
    assigns: &[Assignment],
) -> Ticket {
    let mut c = t.clone();
    if let Some(x) = title {
        c.title = x.clone();
    }
    if let Some(x) = status {
        c.status = x.clone();
    }
    for a in assigns {
        c.fields.retain(|e| e.name != a.name);
        if let Some(v) = &a.value {
            c.fields.push(FieldEntry {
                name: a.name.clone(),
                value: v.clone(),
            });
        }
    }
    c
}
