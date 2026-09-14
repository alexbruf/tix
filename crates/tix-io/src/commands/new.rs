//! `tix new` (TIX-15).

use crate::app::{cli_value, emit_ticket, load_valid_schema, save_ticket};
use crate::host::{CmdResult, Ctx, Failure, Host, PromptKind};
use crate::messages::ticket_error;
use crate::ulid::new_ulid;
use clap::ArgMatches;
use tix_core::ops::new_ticket;
use tix_core::{FieldEntry, FieldType, Ticket};

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;

    let title = match m.get_one::<String>("title") {
        Some(t) => t.clone(),
        None => ctx
            .prompt("title", PromptKind::String, &[])
            .unwrap_or_default(),
    };

    let status = m.get_one::<String>("status").cloned();

    // Fields in schema order (TIX-15): flag, else default, else prompt if required.
    let mut fields: Vec<FieldEntry> = Vec::new();
    for field in &schema.fields {
        if let Some(raw) = m.get_one::<String>(field.name.as_str()) {
            fields.push(FieldEntry {
                name: field.name.clone(),
                value: cli_value(field, raw),
            });
            continue;
        }
        if let Some(default) = &field.default {
            fields.push(FieldEntry {
                name: field.name.clone(),
                value: default.clone(),
            });
            continue;
        }
        if field.required {
            let kind = match field.ty {
                FieldType::Str => PromptKind::String,
                FieldType::Enum => PromptKind::Enum,
                FieldType::Date => PromptKind::Date,
                FieldType::List => PromptKind::List,
            };
            let options: Vec<String> = if field.ty == FieldType::Enum {
                field.values.clone()
            } else {
                Vec::new()
            };
            if let Some(answer) = ctx.prompt(&field.name, kind, &options) {
                if !answer.is_empty() {
                    fields.push(FieldEntry {
                        name: field.name.clone(),
                        value: cli_value(field, &answer),
                    });
                }
            }
        }
        // Optional fields with no flag, no default, and no prompt answer are absent.
    }

    let now = ctx.host.now_unix();
    let random = ctx.host.random_bytes(10);
    let mut entropy = [0u8; 10];
    entropy.copy_from_slice(&random);
    let id = new_ulid(now, &entropy);

    // The candidate `new_ticket` would build, for an accurate error message on
    // failure (there is no "original ticket" for `new`).
    let resolved_status = status.clone().unwrap_or_else(|| {
        schema
            .statuses
            .iter()
            .find(|s| s.group == "backlog")
            .map(|s| s.name.clone())
            .unwrap_or_default()
    });
    let candidate = Ticket {
        id: id.clone(),
        title: title.clone(),
        status: resolved_status,
        created: now,
        updated: now,
        deliverables: Vec::new(),
        fields: fields.clone(),
        body: String::new(),
    };

    match new_ticket(&schema, id, title, status, fields, now) {
        Ok(t) => {
            save_ticket(ctx, &schema, &t)?;
            emit_ticket(ctx, &schema, &t);
            Ok(())
        }
        Err(e) => Err(Failure::validation(ticket_error(&candidate, &schema, &e))),
    }
}
