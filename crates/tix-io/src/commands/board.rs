//! `tix board` (TIX-21, TIX-26).

use crate::app::{load_parsed, load_valid_schema, parse_filters, short_id};
use crate::commands::ls::ticket_json_with_validity;
use crate::host::{CmdResult, Ctx, Host};
use crate::table::{board_table, render};
use clap::ArgMatches;
use serde_json::Value as J;
use tix_core::board::{board, board_groups, Board};
use tix_core::query::filter;
use tix_core::{Schema, Ticket};

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;
    let raw: Vec<String> = m
        .get_many::<String>("filters")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let tokens = parse_filters(&schema, &raw)?;
    let tickets = load_parsed(ctx, &schema)?;
    let matched = filter(&tickets, &schema, &tokens);
    let grouped = m.get_flag("group");

    let b: Board = if grouped {
        board_groups(&tickets, &schema, &matched)
    } else {
        board(&tickets, &schema, &matched)
    };

    let names: Vec<String> = if grouped {
        vec![
            "backlog".to_string(),
            "in_progress".to_string(),
            "completed".to_string(),
        ]
    } else {
        schema.statuses.iter().map(|s| s.name.clone()).collect()
    };

    if ctx.json {
        let mut columns: Vec<J> = names
            .iter()
            .zip(b.columns.iter())
            .map(|(name, col)| column_json(name, col, &tickets, &schema))
            .collect();
        if !b.unknown.is_empty() {
            columns.push(column_json("?", &b.unknown, &tickets, &schema));
        }
        ctx.out(&format!("{}\n", serde_json::json!({ "columns": columns })));
        return Ok(());
    }

    let mut table = board_table();
    let mut header: Vec<String> = names
        .iter()
        .zip(b.columns.iter())
        .map(|(name, col)| format!("{name} ({})", col.len()))
        .collect();
    if !b.unknown.is_empty() {
        header.push(format!("? ({})", b.unknown.len()));
    }
    table.set_header(header);

    let mut all_cols: Vec<&Vec<usize>> = b.columns.iter().collect();
    if !b.unknown.is_empty() {
        all_cols.push(&b.unknown);
    }
    let rows = all_cols.iter().map(|c| c.len()).max().unwrap_or(0);
    for r in 0..rows {
        let row: Vec<String> = all_cols
            .iter()
            .map(|c| {
                c.get(r)
                    .map(|&i| cell_text(&tickets[i], &schema))
                    .unwrap_or_default()
            })
            .collect();
        table.add_row(row);
    }

    ctx.out(&render(&table));
    Ok(())
}

fn cell_text(t: &Ticket, schema: &Schema) -> String {
    let mark = if t.validate(schema).is_err() { "!" } else { "" };
    format!("{mark}{} {}", short_id(&t.id), t.title)
}

fn column_json(name: &str, idxs: &[usize], tickets: &[Ticket], schema: &Schema) -> J {
    let items: Vec<J> = idxs
        .iter()
        .map(|&i| J::Object(ticket_json_with_validity(&tickets[i], schema)))
        .collect();
    serde_json::json!({ "name": name, "tickets": items })
}
