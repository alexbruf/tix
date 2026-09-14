mod common;
use common::*;
use tix_io::testkit::{MemHost, TEST_NOW};
use tix_io::ulid::new_ulid;

/// The id `tix new` produces on a fresh [`MemHost`], deterministic because
/// the clock and entropy are fixed by the test host.
fn expected_id() -> String {
    new_ulid(TEST_NOW, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
}

#[test]
fn tix_15_new_with_all_flags_given() {
    let mut h = fixture();
    let id = expected_id();
    let o = golden(
        "new_all_flags",
        &mut h,
        WS,
        &[
            "new",
            "--title",
            "Article title",
            "--client",
            "acme",
            "--type",
            "article",
            "--owner",
            "charlie",
            "--due",
            "2026-10-01",
        ],
    );
    assert_eq!(o.code, 0);
    assert_eq!(o.stdout, format!("{id}\n"));
    assert!(h.prompts.is_empty());
    let body = h.file(&format!("{WS}/tickets/{id}/ticket.md")).unwrap();
    assert!(body.contains("title: Article title"));
    assert!(body.contains("client: acme"));
    assert!(body.contains("type: article"));
    assert!(body.contains("owner: charlie"));
    assert!(body.contains("due: 2026-10-01"));
    assert_eq!(
        h.writes(),
        vec![
            format!("{WS}/tickets/{id}"),
            format!("{WS}/tickets/{id}/ticket.md"),
        ]
    );
}

#[test]
fn tix_15_new_prompts_required_fields_not_optional() {
    let mut h = fixture();
    h.answers.push_back(Some("acme".to_string()));
    h.answers.push_back(Some("article".to_string()));
    let id = expected_id();
    let o = golden(
        "new_prompts_required",
        &mut h,
        WS,
        &["new", "--title", "Prompted ticket"],
    );
    assert_eq!(o.code, 0);
    assert_eq!(
        h.prompts,
        vec![
            (
                "client".to_string(),
                "string".to_string(),
                Vec::<String>::new()
            ),
            (
                "type".to_string(),
                "enum".to_string(),
                vec![
                    "article".to_string(),
                    "landing_page".to_string(),
                    "linkedin".to_string(),
                    "other".to_string(),
                ]
            ),
        ]
    );
    let body = h.file(&format!("{WS}/tickets/{id}/ticket.md")).unwrap();
    assert!(body.contains("client: acme"));
    assert!(body.contains("type: article"));
    assert!(!body.contains("owner:"));
    assert!(!body.contains("due:"));
}

#[test]
fn tix_15_new_no_prompt_missing_required_field_exits_1() {
    let mut h = fixture();
    let o = golden(
        "new_no_prompt_missing_required",
        &mut h,
        WS,
        &["new", "--title", "T", "--no-prompt"],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
    assert!(h.prompts.is_empty());
}

#[test]
fn tix_15_new_unknown_status_exits_1() {
    let mut h = fixture();
    let o = golden(
        "new_unknown_status",
        &mut h,
        WS,
        &[
            "new", "--title", "T", "--client", "acme", "--type", "article", "--status", "bogus",
        ],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_15_new_with_explicit_status() {
    let mut h = fixture();
    let id = expected_id();
    let o = golden(
        "new_explicit_status",
        &mut h,
        WS,
        &[
            "new",
            "--title",
            "T",
            "--client",
            "acme",
            "--type",
            "article",
            "--status",
            "in_progress",
        ],
    );
    assert_eq!(o.code, 0);
    let body = h.file(&format!("{WS}/tickets/{id}/ticket.md")).unwrap();
    assert!(body.contains("status: in_progress"));
}

#[test]
fn tix_15_new_invalid_enum_value_exits_1() {
    let mut h = fixture();
    let o = golden(
        "new_invalid_enum",
        &mut h,
        WS,
        &["new", "--title", "T", "--client", "acme", "--type", "bogus"],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_15_new_required_field_with_default_is_not_prompted() {
    let mut h = MemHost::new();
    h.put(
        "/proj/tix.yaml",
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: client\n    type: string\n    required: true\n    default: acme\n",
    );
    let id = expected_id();
    let o = golden(
        "new_default_field_not_prompted",
        &mut h,
        "/proj",
        &["new", "--title", "T"],
    );
    assert_eq!(o.code, 0);
    assert!(h.prompts.is_empty());
    let body = h.file(&format!("/proj/tickets/{id}/ticket.md")).unwrap();
    assert!(body.contains("client: acme"));
}

#[test]
fn tix_15_new_json_output() {
    let mut h = fixture();
    let o = golden(
        "new_json",
        &mut h,
        WS,
        &[
            "new", "--title", "T", "--client", "acme", "--type", "article", "--json",
        ],
    );
    assert_eq!(o.code, 0);
}
