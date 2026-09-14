mod common;
use common::*;

#[test]
fn tix_20_detach_by_label() {
    let mut h = fixture();
    let o = golden(
        "detach_by_label",
        &mut h,
        WS,
        &["detach", "01K5B2", "outline"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_PROGRESS}/ticket.md"))
        .unwrap();
    assert!(!after.contains("outline"));
    assert!(after.contains("draft"));
    assert_eq!(
        h.writes(),
        vec![
            format!("{WS}/tickets/{T_PROGRESS}"),
            format!("{WS}/tickets/{T_PROGRESS}/ticket.md"),
        ]
    );
}

#[test]
fn tix_20_detach_by_ref() {
    let mut h = fixture();
    let o = golden(
        "detach_by_ref",
        &mut h,
        WS,
        &["detach", "01K5B2", "https://docs.google.com/document/d/abc"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_PROGRESS}/ticket.md"))
        .unwrap();
    assert!(!after.contains("docs.google.com"));
    assert!(after.contains("outline"));
}

#[test]
fn tix_20_detach_no_match_exits_1() {
    let mut h = fixture();
    let o = golden("detach_no_match", &mut h, WS, &["detach", "01K5B2", "nope"]);
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_20_detach_ambiguous_exits_1() {
    let mut h = fixture();
    let id = format!("01K5F{}", "0".repeat(21));
    assert_eq!(id.len(), 26);
    h.put(
        &format!("{WS}/tickets/{id}/ticket.md"),
        &format!(
            "---\nid: {id}\ntitle: Ambiguous test\nstatus: backlog\ncreated: 2026-09-10T09:00:00Z\nupdated: 2026-09-10T09:00:00Z\ndeliverables:\n- label: alpha\n  ref: shared\n- label: shared\n  ref: beta\nclient: nova\ntype: article\n---\n"
        ),
    );
    let o = golden(
        "detach_ambiguous",
        &mut h,
        WS,
        &["detach", &id[..6], "shared"],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}
