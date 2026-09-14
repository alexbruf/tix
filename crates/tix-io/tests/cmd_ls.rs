mod common;
use common::*;
use tix_io::testkit::MemHost;

#[test]
fn tix_16_ls_no_filters_sorted_by_status_then_created() {
    let mut h = fixture();
    golden("ls_all", &mut h, WS, &["ls"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_31_4_ls_writes_nothing() {
    let mut h = fixture();
    h.run(WS, &["ls"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_16_ls_status_filter() {
    let mut h = fixture();
    golden("ls_status_backlog", &mut h, WS, &["ls", "status:backlog"]);
}

#[test]
fn tix_16_ls_group_filter() {
    let mut h = fixture();
    golden("ls_group_completed", &mut h, WS, &["ls", "group:completed"]);
}

#[test]
fn tix_16_ls_field_filter() {
    let mut h = fixture();
    golden("ls_type_article", &mut h, WS, &["ls", "type:article"]);
}

#[test]
fn tix_16_ls_list_field_filter() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: tags\n    type: list\n",
    );
    h.put(
        &format!("{WS}/tickets/{T_BACKLOG}/ticket.md"),
        &format!(
            "---\nid: {T_BACKLOG}\ntitle: Has the tag\nstatus: backlog\ncreated: 2026-09-10T09:00:00Z\nupdated: 2026-09-10T09:00:00Z\ndeliverables: []\ntags:\n  - urgent\n  - client-work\n---\n"
        ),
    );
    h.put(
        &format!("{WS}/tickets/{T_PROGRESS}/ticket.md"),
        &format!(
            "---\nid: {T_PROGRESS}\ntitle: No matching tag\nstatus: backlog\ncreated: 2026-09-11T09:00:00Z\nupdated: 2026-09-11T09:00:00Z\ndeliverables: []\ntags:\n  - other\n---\n"
        ),
    );
    golden("ls_list_field_filter", &mut h, WS, &["ls", "tags:urgent"]);
}

#[test]
fn tix_16_ls_unknown_filter_key_exits_2() {
    let mut h = fixture();
    let o = golden("ls_unknown_key", &mut h, WS, &["ls", "bogus:val"]);
    assert_eq!(o.code, 2);
}

#[test]
fn tix_16_ls_token_without_colon_exits_2() {
    let mut h = fixture();
    let o = golden("ls_no_colon", &mut h, WS, &["ls", "bogus"]);
    assert_eq!(o.code, 2);
}

#[test]
fn tix_16_ls_json() {
    let mut h = fixture();
    golden("ls_json", &mut h, WS, &["ls", "--json"]);
}
