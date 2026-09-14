mod common;
use common::*;

#[test]
fn tix_17_show_valid_ticket_with_deliverables_and_body() {
    let mut h = fixture();
    let id = format!("01K5F{}", "0".repeat(21));
    assert_eq!(id.len(), 26);
    h.put(
        &format!("{WS}/tickets/{id}/ticket.md"),
        &format!(
            "---\nid: {id}\ntitle: Has both\nstatus: backlog\ncreated: 2026-09-10T09:00:00Z\nupdated: 2026-09-10T09:00:00Z\ndeliverables:\n- label: draft\n  ref: https://docs.google.com/document/d/xyz\nclient: nova\ntype: article\n---\nA real brief.\n"
        ),
    );
    let short = &id[..8];
    golden("show_valid", &mut h, WS, &["show", short]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_31_4_show_writes_nothing() {
    let mut h = fixture();
    h.run(WS, &["show", "01K5AQ"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_10_show_invalid_ticket_marks_with_bang() {
    let mut h = fixture();
    golden("show_invalid", &mut h, WS, &["show", "01K5D1"]);
}

#[test]
fn tix_7_show_ambiguous_prefix_exits_2() {
    let mut h = fixture();
    let o = golden("show_ambiguous", &mut h, WS, &["show", "01K5"]);
    assert_eq!(o.code, 2);
}

#[test]
fn tix_17_show_json() {
    let mut h = fixture();
    golden("show_json", &mut h, WS, &["show", "01K5AQ", "--json"]);
}
