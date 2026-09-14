//! Shared command plumbing: loading the schema and tickets, id resolution,
//! filters, and turning CLI strings into field values.

use crate::host::{Ctx, Failure, Host};
use crate::messages::schema_error;
use crate::schema_yaml::parse_schema;
use crate::storage::Storage;
use crate::ticket_md::{parse_ticket, render_ticket, TicketParseError};
use tix_core::query::{parse_token, resolve, ResolveError, Token};
use tix_core::{Field, FieldType, Schema, Ticket, Value};

pub const SCHEMA_FILE: &str = "tix.yaml";
pub const TICKETS_DIR: &str = "tickets";

pub fn ticket_dir(folder: &str) -> String {
    format!("{TICKETS_DIR}/{folder}")
}

pub fn ticket_file(folder: &str) -> String {
    format!("{TICKETS_DIR}/{folder}/ticket.md")
}

/// First 8 characters of an id (TIX-16).
pub fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// Reads and parses `tix.yaml` without judging TIX-9 validity.
pub fn read_schema<H: Host>(ctx: &Ctx<H>) -> Result<Result<Schema, String>, Failure> {
    let bytes = ctx
        .read(SCHEMA_FILE)?
        .ok_or_else(|| Failure::usage("no tix.yaml found"))?;
    Ok(parse_schema(&bytes).map_err(|e| e.to_string()))
}

/// Reads `tix.yaml` and requires it to be valid (every command except `init` and `check`).
pub fn load_valid_schema<H: Host>(ctx: &Ctx<H>) -> Result<Schema, Failure> {
    let schema =
        read_schema(ctx)?.map_err(|m| Failure::validation(format!("{SCHEMA_FILE}: {m}")))?;
    if let Err(e) = schema.validate() {
        return Err(Failure::validation(format!(
            "{SCHEMA_FILE}: {}",
            schema_error(&schema, &e)
        )));
    }
    Ok(schema)
}

/// Ticket folder names under `tickets/`, sorted.
pub fn ticket_folders<H: Host>(ctx: &Ctx<H>) -> Result<Vec<String>, Failure> {
    if !ctx.exists(TICKETS_DIR)? {
        return Ok(Vec::new());
    }
    let mut names = ctx.list_dir(TICKETS_DIR)?;
    names.sort();
    Ok(names)
}

/// One `tickets/<folder>/ticket.md`, parsed or not.
pub struct Loaded {
    pub folder: String,
    pub ticket: Result<Ticket, TicketParseError>,
}

/// Loads every ticket folder that has a `ticket.md`. Parse failures are kept
/// so `check` can report them and listing commands can warn (TIX-10).
pub fn load_all<H: Host>(ctx: &Ctx<H>, schema: &Schema) -> Result<Vec<Loaded>, Failure> {
    let mut out = Vec::new();
    for folder in ticket_folders(ctx)? {
        if let Some(bytes) = ctx.read(&ticket_file(&folder))? {
            out.push(Loaded {
                ticket: parse_ticket(&bytes, schema, &folder),
                folder,
            });
        }
    }
    Ok(out)
}

/// Loads every parseable ticket, warning on stderr about the rest.
pub fn load_parsed<H: Host>(ctx: &mut Ctx<H>, schema: &Schema) -> Result<Vec<Ticket>, Failure> {
    let mut tickets = Vec::new();
    for l in load_all(ctx, schema)? {
        match l.ticket {
            Ok(t) => tickets.push(t),
            Err(e) => ctx.err(&format!("{}: {e}\n", ticket_file(&l.folder))),
        }
    }
    Ok(tickets)
}

/// Resolves an id prefix to a ticket folder name (TIX-7, TIX-28); failures exit 2.
pub fn resolve_folder<H: Host>(ctx: &Ctx<H>, prefix: &str) -> Result<String, Failure> {
    let folders = ticket_folders(ctx)?;
    match resolve(&prefix.to_string(), &folders) {
        Ok(i) => Ok(folders[i].clone()),
        Err(ResolveError::TooShort) => Err(Failure::usage(format!(
            "id prefix '{prefix}' is too short; use at least 4 characters"
        ))),
        Err(ResolveError::NotFound) => Err(Failure::usage(format!("no ticket matches '{prefix}'"))),
        Err(ResolveError::Ambiguous(c)) => Err(Failure::usage(format!(
            "id prefix '{prefix}' is ambiguous: {}",
            c.iter()
                .map(|&i| folders[i].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Loads one ticket by folder; a parse failure is a validation error.
pub fn load_ticket<H: Host>(
    ctx: &Ctx<H>,
    schema: &Schema,
    folder: &str,
) -> Result<Ticket, Failure> {
    let path = ticket_file(folder);
    let bytes = ctx
        .read(&path)?
        .ok_or_else(|| Failure::usage(format!("{path} does not exist")))?;
    parse_ticket(&bytes, schema, folder).map_err(|e| Failure::validation(format!("{path}: {e}")))
}

/// Writes a ticket to `tickets/<id>/ticket.md`, creating the folder.
pub fn save_ticket<H: Host>(ctx: &mut Ctx<H>, schema: &Schema, t: &Ticket) -> Result<(), Failure> {
    ctx.mkdir_all(&ticket_dir(&t.id))?;
    ctx.write(&ticket_file(&t.id), render_ticket(t, schema).as_bytes())?;
    Ok(())
}

/// Parses `KEY:VALUE` filter tokens (TIX-16, TIX-27); bad tokens exit 2.
pub fn parse_filters(schema: &Schema, raw: &[String]) -> Result<Vec<Token>, Failure> {
    raw.iter()
        .map(|tok| {
            let (k, v) = tok
                .split_once(':')
                .ok_or_else(|| Failure::usage(format!("filter '{tok}' is not KEY:VALUE")))?;
            parse_token(schema, &k.to_string(), v.to_string())
                .ok_or_else(|| Failure::usage(format!("unknown filter key '{k}'")))
        })
        .collect()
}

/// A field value from a CLI string: `list` fields split on commas, others verbatim.
pub fn cli_value(field: &Field, raw: &str) -> Value {
    match field.ty {
        FieldType::List => Value::List(
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        ),
        _ => Value::Str(raw.to_string()),
    }
}

/// Serializes a ticket's frontmatter (plus `body`) as a JSON object.
pub fn ticket_json(t: &Ticket, schema: &Schema) -> serde_json::Map<String, serde_json::Value> {
    use serde_json::Value as J;
    let mut m = serde_json::Map::new();
    m.insert("id".into(), J::String(t.id.clone()));
    m.insert("title".into(), J::String(t.title.clone()));
    m.insert("status".into(), J::String(t.status.clone()));
    m.insert(
        "created".into(),
        J::String(crate::ticket_md::format_rfc3339(t.created)),
    );
    m.insert(
        "updated".into(),
        J::String(crate::ticket_md::format_rfc3339(t.updated)),
    );
    m.insert(
        "deliverables".into(),
        J::Array(
            t.deliverables
                .iter()
                .map(|d| serde_json::json!({ "label": d.label, "ref": d.reference }))
                .collect(),
        ),
    );
    let ordered = schema
        .fields
        .iter()
        .filter_map(|f| t.fields.iter().find(|e| e.name == f.name))
        .chain(
            t.fields
                .iter()
                .filter(|e| !schema.fields.iter().any(|f| f.name == e.name)),
        );
    for e in ordered {
        let v = match &e.value {
            Value::Str(s) => J::String(s.clone()),
            Value::List(xs) => J::Array(xs.iter().cloned().map(J::String).collect()),
        };
        m.insert(e.name.clone(), v);
    }
    m
}
