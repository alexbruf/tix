//! `tix ls` (TIX-16).

use crate::app::{load_parsed, load_valid_schema, parse_filters, short_id, ticket_json};
use crate::host::{CmdResult, Ctx, Host};
use crate::messages::{display_value, ticket_error};
use crate::table::{plain_table, render};
use clap::ArgMatches;
use serde_json::Value as J;
use tix_core::board::{sort_indices, status_ranks};
use tix_core::query::filter;

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;
    let raw: Vec<String> = m
        .get_many::<String>("filters")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let tokens = parse_filters(&schema, &raw)?;
    let tickets = load_parsed(ctx, &schema)?;
    let matched = filter(&tickets, &schema, &tokens);
    let ranks = status_ranks(&tickets, &schema);
    let order = sort_indices(&tickets, &ranks, &matched);

    if ctx.json {
        let arr: Vec<J> = order
            .iter()
            .map(|&i| J::Object(ticket_json_with_validity(&tickets[i], &schema)))
            .collect();
        ctx.out(&format!("{}\n", J::Array(arr)));
        return Ok(());
    }

    let required: Vec<&str> = schema
        .fields
        .iter()
        .filter(|f| f.required)
        .map(|f| f.name.as_str())
        .collect();

    let mut table = plain_table();
    let mut header = vec!["ID".to_string(), "STATUS".to_string(), "TITLE".to_string()];
    header.extend(required.iter().map(|n| n.to_string()));
    table.set_header(header);

    for &i in &order {
        let t = &tickets[i];
        let mark = if t.validate(&schema).is_err() {
            "!"
        } else {
            ""
        };
        let mut row = vec![
            format!("{mark}{}", short_id(&t.id)),
            t.status.clone(),
            t.title.clone(),
        ];
        for name in &required {
            let cell = t
                .fields
                .iter()
                .find(|e| e.name == *name)
                .map(|e| display_value(&e.value))
                .unwrap_or_default();
            row.push(cell);
        }
        table.add_row(row);
    }

    ctx.out(&render(&table));
    Ok(())
}

/// `app::ticket_json` extended with `valid`/`problem` (TIX-10).
pub(crate) fn ticket_json_with_validity(
    t: &tix_core::Ticket,
    schema: &tix_core::Schema,
) -> serde_json::Map<String, J> {
    let mut obj = ticket_json(t, schema);
    match t.validate(schema) {
        Ok(()) => {
            obj.insert("valid".into(), J::Bool(true));
            obj.insert("problem".into(), J::Null);
        }
        Err(e) => {
            obj.insert("valid".into(), J::Bool(false));
            obj.insert("problem".into(), J::String(ticket_error(t, schema, &e)));
        }
    }
    obj
}
