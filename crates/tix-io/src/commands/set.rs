//! `tix set` (TIX-19).

use crate::app::{
    cli_value, load_ticket, load_valid_schema, resolve_folder, save_ticket, ticket_json,
};
use crate::host::{CmdResult, Ctx, Failure, Host};
use crate::messages::ticket_error;
use clap::ArgMatches;
use tix_core::ops::{set_fields, Assignment};
use tix_core::{Schema, Ticket, Value};

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
            "status" => return Err(Failure::usage("cannot set 'status'; use tix mv")),
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
    let original = t.clone();
    match set_fields(t, &schema, title, assigns, now) {
        Ok(t2) => {
            save_ticket(ctx, &schema, &t2)?;
            emit(ctx, &schema, &t2);
            Ok(())
        }
        Err(e) => Err(Failure::validation(ticket_error(&original, &schema, &e))),
    }
}

fn emit<H: Host>(ctx: &mut Ctx<H>, schema: &Schema, t: &Ticket) {
    if ctx.json {
        let text = serde_json::Value::Object(ticket_json(t, schema)).to_string();
        ctx.out(&format!("{text}\n"));
    } else {
        ctx.out(&format!("{}\n", t.id));
    }
}
