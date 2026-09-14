mod common;
use common::*;

#[test]
fn tix_23_path_prints_ticket_folder() {
    let mut h = fixture();
    golden("path_ok", &mut h, "/ws/tickets", &["path", "01K5AQ"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_23_path_json() {
    golden(
        "path_json",
        &mut fixture(),
        WS,
        &["path", "01K5B2", "--json"],
    );
}

#[test]
fn tix_7_prefix_too_short_ambiguous_and_missing_exit_2() {
    let mut h = fixture();
    assert_eq!(
        golden("path_too_short", &mut h, WS, &["path", "01K"]).code,
        2
    );
    assert_eq!(
        golden("path_ambiguous", &mut h, WS, &["path", "01K5"]).code,
        2
    );
    assert_eq!(
        golden("path_not_found", &mut h, WS, &["path", "ZZZZ"]).code,
        2
    );
}

#[test]
fn tix_6_no_workspace_exits_2() {
    let mut h = fixture();
    assert_eq!(
        golden(
            "path_no_workspace",
            &mut h,
            "/elsewhere",
            &["path", "01K5AQ"]
        )
        .code,
        2
    );
}
