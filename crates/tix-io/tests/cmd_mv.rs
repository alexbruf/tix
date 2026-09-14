mod common;
use common::*;

#[test]
fn tix_18_mv_changes_status_and_updated_only() {
    let mut h = fixture();
    let o = golden("mv_ok", &mut h, WS, &["mv", "01K5AQ", "in_progress"]);
    assert_eq!(o.code, 0);
    let after = h
        .file(&format!("{WS}/tickets/{T_BACKLOG}/ticket.md"))
        .unwrap();
    assert!(after.contains("status: in_progress"));
    assert!(after.contains("updated: 2026-09-14T15:02:11Z"));
    assert!(after.contains("created: 2026-09-10T09:00:00Z"));
    assert!(after.contains("id: 01K5AQ9Z3R7M8N2P4Q6S8T0V1W"));
    assert!(after.contains("title: Nova product comparison article"));
    assert!(after.contains("client: nova"));
    assert!(after.contains("type: article"));
    assert!(after.contains("owner: charlie"));
    assert!(after.contains("due: 2026-09-30"));
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
fn tix_18_mv_unknown_status_lists_valid_statuses() {
    let mut h = fixture();
    let o = golden("mv_unknown_status", &mut h, WS, &["mv", "01K5AQ", "bogus"]);
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_7_mv_ambiguous_prefix_exits_2() {
    let mut h = fixture();
    let o = golden("mv_ambiguous", &mut h, WS, &["mv", "01K5", "done"]);
    assert_eq!(o.code, 2);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_18_unknown_status_message_names_it() {
    let mut h = fixture();
    let o = h.run(WS, &["mv", "01K5AQ", "bogus"]);
    assert_eq!(o.code, 1);
    assert_eq!(
        o.stderr,
        "rule 2: unknown status 'bogus'; valid statuses: backlog, in_progress, done\n"
    );
    assert!(h.writes().is_empty());
}
