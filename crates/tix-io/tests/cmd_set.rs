mod common;
use common::*;

#[test]
fn tix_19_set_title_and_field_together() {
    let mut h = fixture();
    let o = golden(
        "set_title_and_field",
        &mut h,
        WS,
        &["set", "01K5AQ", "title=New title", "owner=alex"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(after.contains("title: New title"));
    assert!(after.contains("owner: alex"));
    assert!(after.contains("updated: 2026-09-14T15:02:11Z"));
    assert!(after.contains("client: nova"));
    assert!(after.ends_with("Brief goes here.\n"));
    assert_eq!(
        h.writes(),
        vec![
            format!("{WS}/tickets/{T_BACKLOG}"),
            format!("{WS}/tickets/{T_BACKLOG}/ticket.md"),
        ]
    );
}

#[test]
fn tix_19_set_empty_value_removes_optional_field() {
    let mut h = fixture();
    let o = golden(
        "set_remove_optional",
        &mut h,
        WS,
        &["set", "01K5AQ", "owner="],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(!after.contains("owner:"));
}

#[test]
fn tix_19_set_empty_required_field_exits_1() {
    let mut h = fixture();
    let o = golden(
        "set_remove_required",
        &mut h,
        WS,
        &["set", "01K5AQ", "client="],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_19_set_unknown_key_exits_1() {
    let mut h = fixture();
    let o = golden(
        "set_unknown_key",
        &mut h,
        WS,
        &["set", "01K5AQ", "bogus=foo"],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_19_set_duplicate_key_exits_2() {
    let mut h = fixture();
    let o = golden(
        "set_duplicate_key",
        &mut h,
        WS,
        &["set", "01K5AQ", "owner=a", "owner=b"],
    );
    assert_eq!(o.code, 2);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_10_set_status_and_field_repairs_ticket_in_one_write() {
    // Unknown status and missing required field: mv alone fails rule 3, a
    // field-only set fails rule 2, so set accepts status= (PLAN.md decision).
    let mut h = fixture();
    let path = format!("{WS}/tickets/{T_UNKNOWN_STATUS}/ticket.md");
    let broken = h.file(&path).unwrap().replace("client: acme\n", "");
    h.put(&path, &broken);
    assert_eq!(h.run(WS, &["mv", "01K5E9", "backlog"]).code, 1);
    assert_eq!(h.run(WS, &["set", "01K5E9", "client=acme"]).code, 1);
    let o = golden(
        "set_status_repair",
        &mut h,
        WS,
        &["set", "01K5E9", "status=backlog", "client=acme"],
    );
    assert_eq!(o.code, 0);
    let text = h.file(&path).unwrap();
    assert!(text.contains("status: backlog\n") && text.contains("client: acme\n"));
}

#[test]
fn tix_19_set_bad_assignment_shape_exits_2() {
    let mut h = fixture();
    let o = golden(
        "set_bad_assignment_shape",
        &mut h,
        WS,
        &["set", "01K5AQ", "owner"],
    );
    assert_eq!(o.code, 2);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_12_set_preserves_body_byte_for_byte() {
    let mut h = fixture();
    let o = golden(
        "set_body_preserved",
        &mut h,
        WS,
        &["set", "01K5AQ", "owner=charlie"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(after.ends_with("Brief goes here.\n"));
}

#[test]
fn tix_10_set_repairs_ticket_missing_required_field() {
    let mut h = fixture();
    let o = golden(
        "set_repair_missing_client",
        &mut h,
        WS,
        &["set", "01K5D1", "client=acme"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_NO_CLIENT}/ticket.md"))
        .unwrap();
    assert!(after.contains("client: acme"));
}
