mod common;
use common::*;

#[test]
fn tix_21_board_unknown_status_column() {
    let mut h = fixture();
    golden("board_default", &mut h, WS, &["board"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_31_4_board_writes_nothing() {
    let mut h = fixture();
    h.run(WS, &["board"]);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_21_board_group_collapses_to_three_columns() {
    let mut h = fixture();
    golden("board_group", &mut h, WS, &["board", "--group"]);
}

#[test]
fn tix_21_board_filter() {
    let mut h = fixture();
    golden("board_filtered", &mut h, WS, &["board", "status:backlog"]);
}

#[test]
fn tix_21_board_json() {
    let mut h = fixture();
    golden("board_json", &mut h, WS, &["board", "--json"]);
}
