mod common;
use common::*;
use tix_io::schema_yaml::DEFAULT_TIX_YAML;
use tix_io::testkit::MemHost;

/// A workspace with only the three valid fixture tickets (TIX-22 `ok` case);
/// built fresh since `common::fixture()` may not be edited.
fn valid_workspace() -> MemHost {
    let mut h = MemHost::new();
    h.put(&format!("{WS}/tix.yaml"), DEFAULT_TIX_YAML);
    h.put(
        &format!("{WS}/tickets/{T_BACKLOG}/ticket.md"),
        &format!(
            "---\nid: {T_BACKLOG}\ntitle: Nova product comparison article\nstatus: backlog\ncreated: 2026-09-10T09:00:00Z\nupdated: 2026-09-10T09:00:00Z\ndeliverables: []\nclient: nova\ntype: article\nowner: charlie\ndue: 2026-09-30\n---\nBrief goes here.\n"
        ),
    );
    h.put(
        &format!("{WS}/tickets/{T_PROGRESS}/ticket.md"),
        &format!(
            "---\nid: {T_PROGRESS}\ntitle: Landing page refresh\nstatus: in_progress\ncreated: 2026-09-11T09:00:00Z\nupdated: 2026-09-11T09:00:00Z\ndeliverables:\n- label: draft\n  ref: https://docs.google.com/document/d/abc\n- label: outline\n  ref: ./outline.md\nclient: acme\ntype: landing_page\n---\n"
        ),
    );
    h.put(
        &format!("{WS}/tickets/{T_DONE}/ticket.md"),
        &format!(
            "---\nid: {T_DONE}\ntitle: LinkedIn post\nstatus: done\ncreated: 2026-09-09T09:00:00Z\nupdated: 2026-09-09T09:00:00Z\ndeliverables: []\nclient: nova\ntype: linkedin\n---\nPosted.\n"
        ),
    );
    h
}

#[test]
fn tix_22_check_reports_each_problem() {
    let mut h = fixture();
    let o = golden("check_problems", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_31_4_check_writes_nothing() {
    let mut h = fixture();
    h.run(WS, &["check"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_22_check_ok_workspace_prints_ok() {
    let mut h = valid_workspace();
    let o = golden("check_ok", &mut h, WS, &["check"]);
    assert_eq!(o.code, 0);
}

#[test]
fn tix_22_check_tix_yaml_unparseable() {
    let mut h = MemHost::new();
    h.put(&format!("{WS}/tix.yaml"), "- just\n- a\n- list\n");
    let o = golden("check_yaml_unparseable", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_9_check_rule_1_duplicate_status_name() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\n  - name: backlog\n    group: in_progress\nfields: []\n",
    );
    let o = golden("check_rule1", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_9_check_rule_2_bad_group() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\n  - name: weird\n    group: nope\nfields: []\n",
    );
    let o = golden("check_rule2", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_9_check_rule_3_no_backlog_status() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: in_progress\n    group: in_progress\n  - name: done\n    group: completed\nfields: []\n",
    );
    let o = golden("check_rule3", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_9_check_rule_4_builtin_field_name() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: id\n    type: string\n",
    );
    let o = golden("check_rule4", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_9_check_rule_5_enum_duplicate_values() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: type\n    type: enum\n    values: [a, a]\n",
    );
    let o = golden("check_rule5", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_9_check_rule_6_bad_default() {
    let mut h = MemHost::new();
    h.put(
        &format!("{WS}/tix.yaml"),
        "version: 1\nstatuses:\n  - name: backlog\n    group: backlog\nfields:\n  - name: due\n    type: date\n    default: not-a-date\n",
    );
    let o = golden("check_rule6", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_7_check_reports_folder_id_mismatch() {
    let mut h = valid_workspace();
    h.put(
        &format!("{WS}/tickets/BADFOLDER/ticket.md"),
        "---\nid: 01K5ZZZZZZZZZZZZZZZZZZZZZZ\ntitle: Mismatched folder\nstatus: backlog\ncreated: 2026-09-10T09:00:00Z\nupdated: 2026-09-10T09:00:00Z\ndeliverables: []\nclient: acme\ntype: article\n---\n",
    );
    let o = golden("check_folder_id_mismatch", &mut h, WS, &["check"]);
    assert_eq!(o.code, 1);
}

#[test]
fn tix_22_check_json() {
    let mut h = fixture();
    let o = golden("check_json", &mut h, WS, &["check", "--json"]);
    assert_eq!(o.code, 1);
}
