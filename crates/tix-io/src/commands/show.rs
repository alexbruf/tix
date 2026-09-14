//! `tix show` (TIX-17).

use crate::app::{load_ticket, load_valid_schema, resolve_folder, ticket_json};
use crate::host::{CmdResult, Ctx, Host};
use crate::messages::{display_value, ticket_error};
use crate::ticket_md::format_rfc3339;
use clap::ArgMatches;
use serde_json::Value as J;

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let prefix = m.get_one::<String>("id").expect("required");
    let schema = load_valid_schema(ctx)?;
    let folder = resolve_folder(ctx, prefix)?;
    let t = load_ticket(ctx, &schema, &folder)?;
    let problem = t.validate(&schema).err();

    if ctx.json {
        let mut obj = ticket_json(&t, &schema);
        obj.insert("body".into(), J::String(t.body.clone()));
        obj.insert("valid".into(), J::Bool(problem.is_none()));
        obj.insert(
            "problem".into(),
            match &problem {
                Some(e) => J::String(ticket_error(&t, &schema, e)),
                None => J::Null,
            },
        );
        ctx.out(&format!("{}\n", J::Object(obj)));
        return Ok(());
    }

    let mut out = String::new();
    if let Some(e) = &problem {
        out.push_str(&format!("! {}\n", ticket_error(&t, &schema, e)));
    }
    out.push_str(&format!("id: {}\n", t.id));
    out.push_str(&format!("title: {}\n", t.title));
    out.push_str(&format!("status: {}\n", t.status));
    out.push_str(&format!("created: {}\n", format_rfc3339(t.created)));
    out.push_str(&format!("updated: {}\n", format_rfc3339(t.updated)));

    let ordered = schema
        .fields
        .iter()
        .filter_map(|f| t.fields.iter().find(|e| e.name == f.name))
        .chain(
            t.fields
                .iter()
                .filter(|e| !schema.fields.iter().any(|f| f.name == e.name)),
        );
    for entry in ordered {
        out.push_str(&format!(
            "{}: {}\n",
            entry.name,
            display_value(&entry.value)
        ));
    }

    if !t.deliverables.is_empty() {
        out.push_str("deliverables:\n");
        for d in &t.deliverables {
            out.push_str(&format!("  {} -> {}\n", d.label, d.reference));
        }
    }

    if !t.body.is_empty() {
        out.push('\n');
        out.push_str(&t.body);
    }

    ctx.out(&out);
    Ok(())
}
