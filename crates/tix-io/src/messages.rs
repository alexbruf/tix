//! Human-readable messages for core validation errors. Each names the rule
//! it reports (TIX-9, TIX-11).

use tix_core::{Schema, SchemaError, Ticket, TicketError, Value};

pub fn schema_error(s: &Schema, e: &SchemaError) -> String {
    let status = |i: usize| s.statuses.get(i).map(|x| x.name.as_str()).unwrap_or("");
    let field = |i: usize| s.fields.get(i).map(|x| x.name.as_str()).unwrap_or("");
    match e.rule {
        1 if s.statuses.is_empty() => "rule 1: statuses must not be empty".to_string(),
        1 => format!("rule 1: duplicate status name '{}'", status(e.index)),
        2 => format!(
            "rule 2: status '{}' has group '{}'; expected backlog, in_progress or completed",
            status(e.index),
            s.statuses
                .get(e.index)
                .map(|x| x.group.as_str())
                .unwrap_or("")
        ),
        3 => "rule 3: no status has group 'backlog'".to_string(),
        4 => format!(
            "rule 4: field name '{}' is duplicated, not [a-z][a-z0-9_]*, or a built-in name",
            field(e.index)
        ),
        5 => format!(
            "rule 5: enum field '{}' needs at least one value and unique values",
            field(e.index)
        ),
        _ => format!(
            "rule 6: default for field '{}' is not a valid value",
            field(e.index)
        ),
    }
}

pub fn ticket_error(t: &Ticket, s: &Schema, e: &TicketError) -> String {
    match e.rule {
        1 if t.title.is_empty() => "rule 1: title is empty".to_string(),
        1 => format!("rule 1: id '{}' is not a ULID", t.id),
        2 => format!(
            "rule 2: unknown status '{}'; valid statuses: {}",
            t.status,
            s.statuses
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        3 => format!(
            "rule 3: required field '{}' is missing or empty",
            s.fields.get(e.index).map(|f| f.name.as_str()).unwrap_or("")
        ),
        4 => {
            let entry = t.fields.get(e.index);
            let name = entry.map(|f| f.name.as_str()).unwrap_or("");
            match s.fields.iter().find(|f| f.name == name) {
                None => format!("rule 4: field '{name}' is not declared in tix.yaml"),
                Some(f) => format!(
                    "rule 4: field '{name}' value {} is not a valid {}",
                    entry.map(|x| display_value(&x.value)).unwrap_or_default(),
                    match f.ty {
                        tix_core::FieldType::Str => "string".to_string(),
                        tix_core::FieldType::Enum => format!("enum ({})", f.values.join(", ")),
                        tix_core::FieldType::Date => "date (YYYY-MM-DD)".to_string(),
                        tix_core::FieldType::List => "list of non-empty strings".to_string(),
                    }
                ),
            }
        }
        _ => {
            let d = t.deliverables.get(e.index);
            match d {
                Some(d) if d.label.is_empty() || d.reference.is_empty() => {
                    "rule 5: deliverable label and ref must be non-empty".to_string()
                }
                Some(d) => format!(
                    "rule 5: deliverable ref '{}' is already attached",
                    d.reference
                ),
                None => "rule 5: invalid deliverables".to_string(),
            }
        }
    }
}

/// A field value as shown in tables and messages: lists joined with `, `.
pub fn display_value(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::List(xs) => xs.join(", "),
    }
}
