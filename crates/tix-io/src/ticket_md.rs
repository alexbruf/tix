//! `ticket.md` parse and render (TIX-12, TIX-13).
//!
//! Frontmatter is split on the `---` delimiter lines by hand (that is not a
//! YAML parser, it is just finding two lines), then the frontmatter text is
//! parsed with `serde_yaml`. The body is whatever bytes follow the closing
//! delimiter's newline, untouched.

use serde_yaml::{Mapping, Value as YamlValue};
use tix_core::{Deliverable, FieldEntry, FieldType, Schema, Ticket, Value};

const BUILTIN_KEYS: [&str; 6] = [
    "id",
    "title",
    "status",
    "created",
    "updated",
    "deliverables",
];

/// A `ticket.md` that could not be loaded at all (TIX-12). Everything else
/// (missing/invalid schema fields, unknown status, ...) is TIX-11 territory
/// and still loads (TIX-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketParseError {
    InvalidUtf8,
    MissingFrontmatter,
    Yaml(String),
    BadTimestamp { key: String, value: String },
}

impl std::fmt::Display for TicketParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketParseError::InvalidUtf8 => write!(f, "ticket.md is not valid UTF-8"),
            TicketParseError::MissingFrontmatter => {
                write!(f, "ticket.md has no --- frontmatter block")
            }
            TicketParseError::Yaml(msg) => write!(f, "invalid frontmatter YAML: {msg}"),
            TicketParseError::BadTimestamp { key, value } => {
                write!(
                    f,
                    "field '{key}' is not an RFC 3339 UTC timestamp: '{value}'"
                )
            }
        }
    }
}

/// Splits `---\n<yaml>---\n<body>` into `(yaml, body)`. The body is
/// everything after the closing delimiter line's newline, including any
/// `---` lines of its own.
fn split_frontmatter(s: &str) -> Result<(&str, &str), TicketParseError> {
    let after_open = s
        .strip_prefix("---\n")
        .or_else(|| s.strip_prefix("---\r\n"))
        .ok_or(TicketParseError::MissingFrontmatter)?;

    let mut idx = 0;
    while idx < after_open.len() {
        let line_end = after_open[idx..]
            .find('\n')
            .map(|i| idx + i + 1)
            .unwrap_or(after_open.len());
        let line = &after_open[idx..line_end];
        let trimmed = line
            .strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line);
        if trimmed == "---" {
            return Ok((&after_open[..idx], &after_open[line_end..]));
        }
        if line_end == after_open.len() {
            break;
        }
        idx = line_end;
    }
    Err(TicketParseError::MissingFrontmatter)
}

fn value_to_string(v: &YamlValue) -> String {
    match v {
        YamlValue::String(s) => s.clone(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::Null => String::new(),
        YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => String::new(),
    }
}

/// Shapes a raw YAML node into a ticket field `Value`. A sequence always
/// becomes `List` (its items stringified); a scalar becomes `Str`, unless
/// the schema declares this field `list`, in which case it is lifted into a
/// single-element list so the shape at least matches the declared type
/// (leniency, TIX-10 -- still just data for `Ticket::validate` to judge).
fn resolve_field_value(v: &YamlValue, declared: Option<FieldType>) -> Value {
    match v {
        YamlValue::Sequence(items) => Value::List(items.iter().map(value_to_string).collect()),
        scalar => {
            let s = value_to_string(scalar);
            if declared == Some(FieldType::List) {
                if s.is_empty() {
                    Value::List(Vec::new())
                } else {
                    Value::List(vec![s])
                }
            } else {
                Value::Str(s)
            }
        }
    }
}

fn parse_deliverables(map: &Mapping) -> Vec<Deliverable> {
    let Some(YamlValue::Sequence(items)) = map.get("deliverables") else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| match item {
            YamlValue::Mapping(m) => Deliverable {
                label: m.get("label").map(value_to_string).unwrap_or_default(),
                reference: m.get("ref").map(value_to_string).unwrap_or_default(),
            },
            other => Deliverable {
                label: String::new(),
                reference: value_to_string(other),
            },
        })
        .collect()
}

fn get_timestamp(map: &Mapping, key: &str) -> Result<u64, TicketParseError> {
    let raw = || TicketParseError::BadTimestamp {
        key: key.to_string(),
        value: String::new(),
    };
    let v = map.get(key).ok_or_else(raw)?;
    let s = match v {
        YamlValue::String(s) => s.clone(),
        other => {
            return Err(TicketParseError::BadTimestamp {
                key: key.to_string(),
                value: value_to_string(other),
            })
        }
    };
    parse_rfc3339(&s).ok_or_else(|| TicketParseError::BadTimestamp {
        key: key.to_string(),
        value: s,
    })
}

/// Parses `ticket.md` bytes. Lenient per TIX-10: a missing/mistyped
/// `title`/`status`/schema field does not fail the parse, only a missing
/// `id` falls back to `folder_id`, an unparseable document does.
pub fn parse_ticket(
    bytes: &[u8],
    schema: &Schema,
    folder_id: &str,
) -> Result<Ticket, TicketParseError> {
    let text = std::str::from_utf8(bytes).map_err(|_| TicketParseError::InvalidUtf8)?;
    let (yaml_src, body) = split_frontmatter(text)?;

    let doc: YamlValue =
        serde_yaml::from_str(yaml_src).map_err(|e| TicketParseError::Yaml(e.to_string()))?;
    let map = match doc {
        YamlValue::Mapping(m) => m,
        YamlValue::Null => Mapping::new(),
        _ => {
            return Err(TicketParseError::Yaml(
                "frontmatter is not a mapping".to_string(),
            ))
        }
    };

    let id = match map.get("id") {
        Some(v) => value_to_string(v),
        None => folder_id.to_string(),
    };
    let title = map.get("title").map(value_to_string).unwrap_or_default();
    let status = map.get("status").map(value_to_string).unwrap_or_default();
    let created = get_timestamp(&map, "created")?;
    let updated = get_timestamp(&map, "updated")?;
    let deliverables = parse_deliverables(&map);

    let mut fields = Vec::new();
    for (k, v) in map.iter() {
        let key = match k {
            YamlValue::String(s) => s.clone(),
            other => value_to_string(other),
        };
        if BUILTIN_KEYS.contains(&key.as_str()) {
            continue;
        }
        let declared = schema.fields.iter().find(|f| f.name == key).map(|f| f.ty);
        fields.push(FieldEntry {
            name: key,
            value: resolve_field_value(v, declared),
        });
    }

    Ok(Ticket {
        id,
        title,
        status,
        created,
        updated,
        deliverables,
        fields,
        body: body.to_string(),
    })
}

fn y(s: &str) -> YamlValue {
    YamlValue::String(s.to_string())
}

fn field_value_to_yaml(v: &Value) -> YamlValue {
    match v {
        Value::Str(s) => y(s),
        Value::List(items) => YamlValue::Sequence(items.iter().map(|s| y(s)).collect()),
    }
}

fn deliverables_to_yaml(items: &[Deliverable]) -> YamlValue {
    YamlValue::Sequence(
        items
            .iter()
            .map(|d| {
                let mut m = Mapping::new();
                m.insert(y("label"), y(&d.label));
                m.insert(y("ref"), y(&d.reference));
                YamlValue::Mapping(m)
            })
            .collect(),
    )
}

/// Renders a ticket back to `ticket.md` bytes: builtins in fixed order,
/// then schema fields in schema order, then any leftover (unknown) keys,
/// then the body byte-for-byte (TIX-12).
pub fn render_ticket(t: &Ticket, schema: &Schema) -> String {
    let mut map = Mapping::new();
    map.insert(y("id"), y(&t.id));
    map.insert(y("title"), y(&t.title));
    map.insert(y("status"), y(&t.status));
    map.insert(y("created"), y(&format_rfc3339(t.created)));
    map.insert(y("updated"), y(&format_rfc3339(t.updated)));
    map.insert(y("deliverables"), deliverables_to_yaml(&t.deliverables));

    for field in &schema.fields {
        if let Some(entry) = t.fields.iter().find(|e| e.name == field.name) {
            map.insert(y(&entry.name), field_value_to_yaml(&entry.value));
        }
    }
    for entry in &t.fields {
        if !schema.fields.iter().any(|f| f.name == entry.name) {
            map.insert(y(&entry.name), field_value_to_yaml(&entry.value));
        }
    }

    let yaml = serde_yaml::to_string(&map).expect("ticket mapping always serializes");
    format!("---\n{yaml}---\n{}", t.body)
}

// --- civil-date <-> unix-seconds, no chrono (days-from-civil algorithm,
// Howard Hinnant: http://howardhinnant.github.io/date_algorithms.html) ---

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn parse_rfc3339(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() != 20 {
        return None;
    }
    if b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    let minute: u32 = s.get(14..16)?.parse().ok()?;
    let second: u32 = s.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month as i64, day as i64);
    let secs = days * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    u64::try_from(secs).ok()
}

fn format_rfc3339(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_yaml::{parse_schema, DEFAULT_TIX_YAML};
    use proptest::prelude::*;

    fn default_schema() -> Schema {
        parse_schema(DEFAULT_TIX_YAML.as_bytes()).unwrap()
    }

    const EXAMPLE: &str = concat!(
        "---\n",
        "id: 01K5AQ9Z3R7M8N2P4Q6S8T0V1W\n",
        "title: Nova product comparison article\n",
        "status: in_progress\n",
        "created: 2026-09-14T15:02:11Z\n",
        "updated: 2026-09-15T09:40:00Z\n",
        "deliverables:\n",
        "  - label: draft\n",
        "    ref: https://docs.google.com/document/d/abc\n",
        "  - label: outline\n",
        "    ref: ./outline.md\n",
        "client: nova\n",
        "type: article\n",
        "owner: charlie\n",
        "due: 2026-09-30\n",
        "---\n",
        "Brief goes here. Anything Markdown.\n",
    );

    #[test]
    fn tix_12_example_parses_as_expected() {
        let schema = default_schema();
        let t = parse_ticket(EXAMPLE.as_bytes(), &schema, "unused").unwrap();
        assert_eq!(t.id, "01K5AQ9Z3R7M8N2P4Q6S8T0V1W");
        assert_eq!(t.title, "Nova product comparison article");
        assert_eq!(t.status, "in_progress");
        assert_eq!(t.created, parse_rfc3339("2026-09-14T15:02:11Z").unwrap());
        assert_eq!(t.updated, parse_rfc3339("2026-09-15T09:40:00Z").unwrap());
        assert_eq!(
            t.deliverables,
            vec![
                Deliverable {
                    label: "draft".into(),
                    reference: "https://docs.google.com/document/d/abc".into()
                },
                Deliverable {
                    label: "outline".into(),
                    reference: "./outline.md".into()
                },
            ]
        );
        assert_eq!(
            t.fields,
            vec![
                FieldEntry {
                    name: "client".into(),
                    value: Value::Str("nova".into())
                },
                FieldEntry {
                    name: "type".into(),
                    value: Value::Str("article".into())
                },
                FieldEntry {
                    name: "owner".into(),
                    value: Value::Str("charlie".into())
                },
                FieldEntry {
                    name: "due".into(),
                    value: Value::Str("2026-09-30".into())
                },
            ]
        );
        assert_eq!(t.body, "Brief goes here. Anything Markdown.\n");
    }

    // serde_yaml's default serializer does not indent a sequence nested directly
    // under a mapping key, so `deliverables:` items line up under the key rather
    // than being indented two spaces the way the TIX-12 example shows it. Parsing
    // still accepts the indented spec form (see `tix_12_example_parses_as_expected`
    // above); this is the byte-exact shape `render_ticket` actually produces.
    const EXAMPLE_RENDERED: &str = concat!(
        "---\n",
        "id: 01K5AQ9Z3R7M8N2P4Q6S8T0V1W\n",
        "title: Nova product comparison article\n",
        "status: in_progress\n",
        "created: 2026-09-14T15:02:11Z\n",
        "updated: 2026-09-15T09:40:00Z\n",
        "deliverables:\n",
        "- label: draft\n",
        "  ref: https://docs.google.com/document/d/abc\n",
        "- label: outline\n",
        "  ref: ./outline.md\n",
        "client: nova\n",
        "type: article\n",
        "owner: charlie\n",
        "due: 2026-09-30\n",
        "---\n",
        "Brief goes here. Anything Markdown.\n",
    );

    #[test]
    fn tix_12_example_renders_back_identical() {
        let schema = default_schema();
        let t = parse_ticket(EXAMPLE.as_bytes(), &schema, "unused").unwrap();
        let rendered = render_ticket(&t, &schema);
        assert_eq!(rendered, EXAMPLE_RENDERED);
    }

    #[test]
    fn tix_12_body_preserved_byte_for_byte() {
        let schema = default_schema();
        let body = "line one\r\nline two\r\n\ntrailing spaces   \n";
        let text = format!(
            "---\nid: X\ntitle: t\nstatus: backlog\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n---\n{body}"
        );
        let t = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap();
        assert_eq!(t.body.as_bytes(), body.as_bytes());
        let rendered = render_ticket(&t, &schema);
        assert!(rendered.ends_with(body));
    }

    #[test]
    fn tix_12_body_starting_with_dashes_is_not_mistaken_for_a_delimiter() {
        let schema = default_schema();
        let body = "---\nnot frontmatter, just text\n";
        let text = format!(
            "---\nid: X\ntitle: t\nstatus: backlog\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n---\n{body}"
        );
        let t = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap();
        assert_eq!(t.body, body);
    }

    #[test]
    fn tix_10_missing_title_and_status_default_to_empty_string() {
        let schema = default_schema();
        let text =
            "---\nid: X\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n---\nbody";
        let t = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap();
        assert_eq!(t.title, "");
        assert_eq!(t.status, "");
    }

    #[test]
    fn tix_10_missing_id_falls_back_to_folder_id() {
        let schema = default_schema();
        let text = "---\ntitle: t\nstatus: backlog\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n---\nbody";
        let t = parse_ticket(text.as_bytes(), &schema, "01FOLDERID").unwrap();
        assert_eq!(t.id, "01FOLDERID");
    }

    #[test]
    fn tix_10_missing_deliverables_key_is_empty_vec() {
        let schema = default_schema();
        let text = "---\nid: X\ntitle: t\nstatus: backlog\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n---\nbody";
        let t = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap();
        assert_eq!(t.deliverables, Vec::new());
    }

    #[test]
    fn tix_10_non_string_scalar_field_becomes_its_string_form() {
        // serde_yaml (YAML 1.2 core schema) reads `yes`/`no` as plain strings, not
        // booleans, so this exercises the stringify path with an actual bool (`true`).
        let schema = default_schema();
        let text = "---\nid: X\ntitle: t\nstatus: backlog\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\nclient: true\n---\nbody";
        let t = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap();
        assert_eq!(
            t.fields,
            vec![FieldEntry {
                name: "client".into(),
                value: Value::Str("true".into())
            }]
        );
    }

    #[test]
    fn tix_10_unknown_keys_survive_into_fields() {
        let schema = default_schema();
        let text = "---\nid: X\ntitle: t\nstatus: backlog\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\nnot_in_schema: surprise\n---\nbody";
        let t = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap();
        assert_eq!(
            t.fields,
            vec![FieldEntry {
                name: "not_in_schema".into(),
                value: Value::Str("surprise".into())
            }]
        );
    }

    #[test]
    fn tix_12_missing_frontmatter_is_a_parse_error() {
        let schema = default_schema();
        assert_eq!(
            parse_ticket(b"no frontmatter here", &schema, "id").unwrap_err(),
            TicketParseError::MissingFrontmatter
        );
        assert_eq!(
            parse_ticket(b"---\nid: X\n", &schema, "id").unwrap_err(),
            TicketParseError::MissingFrontmatter
        );
    }

    #[test]
    fn tix_12_bad_timestamp_is_a_parse_error() {
        let schema = default_schema();
        let text = "---\nid: X\ntitle: t\nstatus: backlog\ncreated: not-a-date\nupdated: 2026-01-01T00:00:00Z\n---\nbody";
        let err = parse_ticket(text.as_bytes(), &schema, "fallback").unwrap_err();
        assert_eq!(
            err,
            TicketParseError::BadTimestamp {
                key: "created".into(),
                value: "not-a-date".into()
            }
        );
    }

    #[test]
    fn tix_12_days_from_civil_round_trips_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2026, 9, 14)), (2026, 9, 14));
        assert_eq!(civil_from_days(days_from_civil(2000, 2, 29)), (2000, 2, 29));
        assert_eq!(civil_from_days(days_from_civil(1900, 3, 1)), (1900, 3, 1));
    }

    #[test]
    fn tix_12_rfc3339_round_trips() {
        let secs = parse_rfc3339("2026-09-14T15:02:11Z").unwrap();
        assert_eq!(format_rfc3339(secs), "2026-09-14T15:02:11Z");
    }

    #[test]
    fn tix_12_rfc3339_rejects_bad_calendar_date() {
        assert_eq!(parse_rfc3339("2026-02-30T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-09-14 15:02:11Z"), None);
    }

    // --- TIX-31.1 property round trip ---

    fn tricky_string_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            5 => "[a-zA-Z0-9 ]{1,20}",
            1 => Just("yes".to_string()),
            1 => Just("no".to_string()),
            1 => Just("123".to_string()),
            1 => Just("null".to_string()),
            1 => Just(": x".to_string()),
            1 => Just("#leading hash".to_string()),
            1 => Just(" leading space".to_string()),
            1 => Just("a: b".to_string()),
            1 => Just("unicode ключ 日本語 🎉".to_string()),
            1 => Just("trailing space ".to_string()),
        ]
    }

    fn ulid_strategy() -> impl Strategy<Value = String> {
        let crockford: Vec<char> = "0123456789ABCDEFGHJKMNPQRSTVWXYZ".chars().collect();
        let firsts: Vec<char> = crockford[0..8].to_vec(); // no 128-bit overflow
        (
            prop::sample::select(firsts),
            prop::collection::vec(prop::sample::select(crockford), 25),
        )
            .prop_map(|(first, rest)| {
                let mut s = String::with_capacity(26);
                s.push(first);
                s.extend(rest);
                s
            })
    }

    fn deliverables_strategy() -> impl Strategy<Value = Vec<Deliverable>> {
        prop::collection::hash_set("[a-zA-Z0-9/._:-]{1,12}", 0..=3).prop_flat_map(|refs| {
            let refs: Vec<String> = refs.into_iter().collect();
            let n = refs.len();
            prop::collection::vec(tricky_string_strategy(), n).prop_map(move |labels| {
                labels
                    .into_iter()
                    .zip(refs.clone())
                    .map(|(label, reference)| Deliverable { label, reference })
                    .collect()
            })
        })
    }

    fn body_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            1 => Just(String::new()),
            1 => Just("---\nSomething after a dash line.\n".to_string()),
            1 => Just("Line one\r\nLine two\r\n".to_string()),
            1 => Just("Trailing whitespace.   \n\n".to_string()),
            3 => "[ -~\n]{0,80}",
        ]
    }

    fn ticket_strategy() -> impl Strategy<Value = Ticket> {
        (
            ulid_strategy(),
            tricky_string_strategy(),
            prop::sample::select(vec!["backlog", "in_progress", "done"]),
            946_684_800u64..=2_208_988_800u64,
            946_684_800u64..=2_208_988_800u64,
            deliverables_strategy(),
            tricky_string_strategy(),
            prop::sample::select(vec!["article", "landing_page", "linkedin", "other"]),
            proptest::option::of(tricky_string_strategy()),
            proptest::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}"),
            body_strategy(),
        )
            .prop_map(
                |(
                    id,
                    title,
                    status,
                    created,
                    updated,
                    deliverables,
                    client,
                    ty,
                    owner,
                    due,
                    body,
                )| {
                    let mut fields = vec![
                        FieldEntry {
                            name: "client".into(),
                            value: Value::Str(client),
                        },
                        FieldEntry {
                            name: "type".into(),
                            value: Value::Str(ty.to_string()),
                        },
                    ];
                    if let Some(o) = owner {
                        fields.push(FieldEntry {
                            name: "owner".into(),
                            value: Value::Str(o),
                        });
                    }
                    if let Some(d) = due {
                        fields.push(FieldEntry {
                            name: "due".into(),
                            value: Value::Str(d),
                        });
                    }
                    Ticket {
                        id,
                        title,
                        status: status.to_string(),
                        created,
                        updated,
                        deliverables,
                        fields,
                        body,
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn tix_31_1_round_trip(t in ticket_strategy()) {
            let schema = default_schema();
            let rendered = render_ticket(&t, &schema);
            let parsed = parse_ticket(rendered.as_bytes(), &schema, &t.id).expect("rendered ticket must parse");
            prop_assert_eq!(parsed.body.as_bytes(), t.body.as_bytes());
            prop_assert_eq!(parsed, t);
        }
    }
}
