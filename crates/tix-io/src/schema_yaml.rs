//! `tix.yaml` parse and render (TIX-8). Schema *validity* (TIX-9) is
//! `tix-core::Schema::validate`'s job; this module only owns the shape of
//! the YAML itself.

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value as YamlValue};
use tix_core::{Field, FieldType, Schema, StatusDef, Value};

/// The exact default `tix.yaml` `tix init` writes (TIX-8).
pub const DEFAULT_TIX_YAML: &str = "\
version: 1
statuses:
  - name: backlog
    group: backlog
  - name: in_progress
    group: in_progress
  - name: done
    group: completed
fields:
  - name: client
    type: string
    required: true
  - name: type
    type: enum
    values: [article, landing_page, linkedin, other]
    required: true
  - name: owner
    type: string
  - name: due
    type: date
";

#[derive(Deserialize)]
struct RawSchema {
    version: u32,
    statuses: Vec<RawStatus>,
    fields: Vec<RawField>,
}

#[derive(Deserialize)]
struct RawStatus {
    name: String,
    group: String,
}

#[derive(Deserialize)]
struct RawField {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    values: Option<Vec<String>>,
    #[serde(default)]
    default: Option<RawDefault>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum RawDefault {
    Str(String),
    List(Vec<String>),
}

/// A `tix.yaml` shape violation (TIX-8). Not to be confused with
/// `tix-core::SchemaError`, which is TIX-9 validity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaParseError {
    Yaml(String),
    UnsupportedVersion(u32),
    UnknownFieldType { field: String, ty: String },
    EnumMissingValues { field: String },
}

impl std::fmt::Display for SchemaParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaParseError::Yaml(msg) => write!(f, "invalid tix.yaml: {msg}"),
            SchemaParseError::UnsupportedVersion(v) => {
                write!(f, "unsupported tix.yaml version: {v}")
            }
            SchemaParseError::UnknownFieldType { field, ty } => {
                write!(f, "field '{field}' has unknown type '{ty}'")
            }
            SchemaParseError::EnumMissingValues { field } => {
                write!(f, "enum field '{field}' must declare values")
            }
        }
    }
}

fn parse_field_type(ty: &str) -> Option<FieldType> {
    match ty {
        "string" => Some(FieldType::Str),
        "enum" => Some(FieldType::Enum),
        "date" => Some(FieldType::Date),
        "list" => Some(FieldType::List),
        _ => None,
    }
}

fn field_type_name(ty: FieldType) -> &'static str {
    match ty {
        FieldType::Str => "string",
        FieldType::Enum => "enum",
        FieldType::Date => "date",
        FieldType::List => "list",
    }
}

/// Parses `tix.yaml` bytes into a core `Schema` (TIX-8 shape rules only).
pub fn parse_schema(bytes: &[u8]) -> Result<Schema, SchemaParseError> {
    let raw: RawSchema =
        serde_yaml::from_slice(bytes).map_err(|e| SchemaParseError::Yaml(e.to_string()))?;
    if raw.version != 1 {
        return Err(SchemaParseError::UnsupportedVersion(raw.version));
    }
    let statuses = raw
        .statuses
        .into_iter()
        .map(|s| StatusDef {
            name: s.name,
            group: s.group,
        })
        .collect();

    let mut fields = Vec::with_capacity(raw.fields.len());
    for f in raw.fields {
        let ty = parse_field_type(&f.ty).ok_or_else(|| SchemaParseError::UnknownFieldType {
            field: f.name.clone(),
            ty: f.ty.clone(),
        })?;
        if ty == FieldType::Enum && f.values.as_ref().is_none_or(|v| v.is_empty()) {
            return Err(SchemaParseError::EnumMissingValues {
                field: f.name.clone(),
            });
        }
        let default = f.default.map(|d| match d {
            RawDefault::Str(s) => Value::Str(s),
            RawDefault::List(items) => Value::List(items),
        });
        fields.push(Field {
            name: f.name,
            ty,
            required: f.required,
            values: f.values.unwrap_or_default(),
            default,
        });
    }

    Ok(Schema { statuses, fields })
}

fn y(s: &str) -> YamlValue {
    YamlValue::String(s.to_string())
}

/// Renders a `Schema` back to `tix.yaml` text.
pub fn render_schema(schema: &Schema) -> String {
    let mut root = Mapping::new();
    root.insert(y("version"), YamlValue::Number(1.into()));

    let statuses = schema
        .statuses
        .iter()
        .map(|s| {
            let mut m = Mapping::new();
            m.insert(y("name"), y(&s.name));
            m.insert(y("group"), y(&s.group));
            YamlValue::Mapping(m)
        })
        .collect();
    root.insert(y("statuses"), YamlValue::Sequence(statuses));

    let fields = schema
        .fields
        .iter()
        .map(|f| {
            let mut m = Mapping::new();
            m.insert(y("name"), y(&f.name));
            m.insert(y("type"), y(field_type_name(f.ty)));
            if !f.values.is_empty() {
                m.insert(
                    y("values"),
                    YamlValue::Sequence(f.values.iter().map(|v| y(v)).collect()),
                );
            }
            m.insert(y("required"), YamlValue::Bool(f.required));
            if let Some(default) = &f.default {
                let v = match default {
                    Value::Str(s) => y(s),
                    Value::List(items) => YamlValue::Sequence(items.iter().map(|v| y(v)).collect()),
                };
                m.insert(y("default"), v);
            }
            YamlValue::Mapping(m)
        })
        .collect();
    root.insert(y("fields"), YamlValue::Sequence(fields));

    serde_yaml::to_string(&root).expect("schema mapping always serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tix_core::FieldType;

    #[test]
    fn tix_8_default_schema_parses() {
        let schema = parse_schema(DEFAULT_TIX_YAML.as_bytes()).unwrap();
        assert_eq!(schema.statuses.len(), 3);
        assert_eq!(
            schema.statuses[0],
            StatusDef {
                name: "backlog".into(),
                group: "backlog".into()
            }
        );
        assert_eq!(
            schema.statuses[1],
            StatusDef {
                name: "in_progress".into(),
                group: "in_progress".into()
            }
        );
        assert_eq!(
            schema.statuses[2],
            StatusDef {
                name: "done".into(),
                group: "completed".into()
            }
        );

        assert_eq!(schema.fields.len(), 4);

        let client = &schema.fields[0];
        assert_eq!(client.name, "client");
        assert_eq!(client.ty, FieldType::Str);
        assert!(client.required);
        assert!(client.values.is_empty());
        assert_eq!(client.default, None);

        let type_field = &schema.fields[1];
        assert_eq!(type_field.name, "type");
        assert_eq!(type_field.ty, FieldType::Enum);
        assert!(type_field.required);
        assert_eq!(
            type_field.values,
            vec![
                "article".to_string(),
                "landing_page".to_string(),
                "linkedin".to_string(),
                "other".to_string()
            ]
        );

        let owner = &schema.fields[2];
        assert_eq!(owner.name, "owner");
        assert_eq!(owner.ty, FieldType::Str);
        assert!(!owner.required);

        let due = &schema.fields[3];
        assert_eq!(due.name, "due");
        assert_eq!(due.ty, FieldType::Date);
        assert!(!due.required);
    }

    #[test]
    fn tix_8_unknown_field_type_is_parse_error() {
        let yaml = "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: x\n    type: bogus\n";
        let err = parse_schema(yaml.as_bytes()).unwrap_err();
        assert_eq!(
            err,
            SchemaParseError::UnknownFieldType {
                field: "x".into(),
                ty: "bogus".into()
            }
        );
    }

    #[test]
    fn tix_8_enum_without_values_is_parse_error() {
        let yaml = "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: x\n    type: enum\n";
        let err = parse_schema(yaml.as_bytes()).unwrap_err();
        assert_eq!(
            err,
            SchemaParseError::EnumMissingValues { field: "x".into() }
        );
    }

    #[test]
    fn tix_8_unsupported_version_is_parse_error() {
        let yaml = "version: 2\nstatuses: []\nfields: []\n";
        let err = parse_schema(yaml.as_bytes()).unwrap_err();
        assert_eq!(err, SchemaParseError::UnsupportedVersion(2));
    }

    #[test]
    fn tix_8_default_list_and_string_forms_parse() {
        let yaml = "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: owner\n    type: string\n    default: alex\n  - name: tags\n    type: list\n    default: [a, b]\n";
        let schema = parse_schema(yaml.as_bytes()).unwrap();
        assert_eq!(schema.fields[0].default, Some(Value::Str("alex".into())));
        assert_eq!(
            schema.fields[1].default,
            Some(Value::List(vec!["a".into(), "b".into()]))
        );
    }

    #[test]
    fn tix_8_render_round_trips_through_parse() {
        let schema = parse_schema(DEFAULT_TIX_YAML.as_bytes()).unwrap();
        let rendered = render_schema(&schema);
        let reparsed = parse_schema(rendered.as_bytes()).unwrap();
        assert_eq!(schema, reparsed);
    }
}
