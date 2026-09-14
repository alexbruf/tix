mod common;
use common::*;

#[test]
fn tix_13_attach_default_label_from_url() {
    let mut h = fixture();
    let o = golden(
        "attach_default_label_url",
        &mut h,
        WS,
        &["attach", "01K5AQ", "https://docs.google.com/document/d/xyz"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(after.contains("label: xyz"));
    assert!(after.contains("ref: https://docs.google.com/document/d/xyz"));
    assert_eq!(
        h.writes(),
        vec![
            format!("{WS}/tickets/{T_BACKLOG}"),
            format!("{WS}/tickets/{T_BACKLOG}/ticket.md"),
        ]
    );
}

#[test]
fn tix_13_attach_default_label_from_relative_path() {
    let mut h = fixture();
    let o = golden(
        "attach_default_label_relative",
        &mut h,
        WS,
        &["attach", "01K5AQ", "./outline.md"],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(after.contains("label: outline.md"));
    assert!(after.contains("ref: ./outline.md"));
}

#[test]
fn tix_20_attach_explicit_label() {
    let mut h = fixture();
    let o = golden(
        "attach_explicit_label",
        &mut h,
        WS,
        &[
            "attach",
            "01K5AQ",
            "https://example.com/doc",
            "--label",
            "draft",
        ],
    );
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(after.contains("label: draft"));
    assert!(after.contains("ref: https://example.com/doc"));
}

#[test]
fn tix_20_attach_duplicate_ref_exits_1() {
    let mut h = fixture();
    let o = golden(
        "attach_duplicate_ref",
        &mut h,
        WS,
        &["attach", "01K5B2", "https://docs.google.com/document/d/abc"],
    );
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}
