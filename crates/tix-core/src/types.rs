//! Plain data types shared by the verified core and the io layer.
//! Timestamps are unix seconds; `tix-io` renders them as RFC 3339 (TIX-12).

use vstd::prelude::*;

verus! {

/// Field `type` from TIX-8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldType {
    Str,
    Enum,
    Date,
    List,
}

/// A field value: `string`, `enum` and `date` use `Str`; `list` uses `List`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Str(String),
    List(Vec<String>),
}

/// One entry of `fields:` in `tix.yaml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    pub required: bool,
    /// Declared `values`; meaningful for `enum` fields only.
    pub values: Vec<String>,
    pub default: Option<Value>,
}

/// One entry of `statuses:` in `tix.yaml`. `group` stays a string so that
/// TIX-9 rule 2 is checked (and proved) in the core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusDef {
    pub name: String,
    pub group: String,
}

/// Parsed `tix.yaml` (TIX-8). `version` is checked by `tix-io`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    pub statuses: Vec<StatusDef>,
    pub fields: Vec<Field>,
}

/// The three fixed status groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Backlog,
    InProgress,
    Completed,
}

/// A schema field present on a ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldEntry {
    pub name: String,
    pub value: Value,
}

/// A deliverable reference (TIX-13). `reference` is the `ref` key on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deliverable {
    pub label: String,
    pub reference: String,
}

/// A ticket as loaded from `ticket.md`. It may be invalid (TIX-10); `fields`
/// keeps every schema-field key found in the frontmatter, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created: u64,
    pub updated: u64,
    pub deliverables: Vec<Deliverable>,
    pub fields: Vec<FieldEntry>,
    pub body: String,
}

/// A TIX-9 violation: `rule` is 1..=6, `index` points at the offending
/// status (rules 1-3) or field (rules 4-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaError {
    pub rule: u8,
    pub index: usize,
}

/// A TIX-11 violation: `rule` is 1..=5. `index` points at the offending
/// schema field (rule 3), ticket field (rule 4) or deliverable (rule 5); 0 otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TicketError {
    pub rule: u8,
    pub index: usize,
}

} // verus!
